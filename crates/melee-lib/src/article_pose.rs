//! Articles whose bones effects follow in the hand (Mario's cape): the
//! model's joint tree posed as retail poses it, so a generator on one of its
//! bones (xBBC_dynamicBoneTable) sits where retail's does.
//!
//! Item_8026AB54 -> it_80274F48 constrains the article's attach bone
//! (ItemModelDesc x8) to the holder's part: lb_8000C2F8 adds a position and
//! an orientation RObj, which HSD_JObjSetupMatrix resolves each time the
//! bone's matrix is set up. The attach bone has no joint animation, so the
//! local SRT that resolution recovers (JObjUpdateFunc 0x37/0x38) carries
//! into the next setup: each posed article keeps its own tree.
//!
//! The item's procs place its root once per tick, which dirties the tree;
//! the first setup after that (a generator on a bone in ParticlesMain, or
//! the display pass after the tick) resolves the constraints, and later
//! setups in the same tick find the tree clean. Moving the hand alone does
//! not dirty it.
use hsd_anim::jobj::{JObjId, JObjTree};
use hsd_types::Mtx;
use melee_types::ItemKind;

/// A prepared article skeleton with its first article state's animation
/// (none for a model that state does not animate: Mr. Game & Watch's
/// Manhole).
#[derive(Clone)]
pub(crate) struct ArticleSkeleton {
    pub kind: ItemKind,
    /// The model with each article state's joint animation: one tree for
    /// an article posed only in state 0 (Mario's cape), one per state for
    /// a held item whose states each animate it (the Fire Flower).
    trees: Vec<JObjTree>,
    /// The article state `trees` is posing now.
    current: usize,
    root: JObjId,
    /// ItemModelDesc x8: the depth-first index of the constrained bone.
    attach_bone: usize,
    /// Joints whose authored RObj constraints were left out
    /// (hsd_anim::load::load_joint_tree_without_constraints): no bone read
    /// may lie below one.
    unconstrained: Vec<JObjId>,
}
impl ArticleSkeleton {
    /// Item_80268D34's model with article state 0's joint animation.
    pub fn load(
        kind: ItemKind,
        archive: &hsd_archive::Archive,
        visual: &hsd_archive::desc::item_visual::ItemVisual,
    ) -> anyhow::Result<Self> {
        Self::load_states(kind, archive, visual, 1)
    }

    /// The model once per article state, `states` of them, each with that
    /// state's joint animation (Item_80268D34 of each state).
    pub fn load_states(
        kind: ItemKind,
        archive: &hsd_archive::Archive,
        visual: &hsd_archive::desc::item_visual::ItemVisual,
        states: usize,
    ) -> anyhow::Result<Self> {
        let mut trees = Vec::with_capacity(states);
        let mut root = None;
        let mut unconstrained = Vec::new();
        for state in 0..states {
            let (mut tree, tree_root, skipped) =
                hsd_anim::load::load_joint_tree_without_constraints(archive, &visual.model)
                    .map_err(|e| anyhow::anyhow!("article model: {e}"))?;
            unconstrained = skipped;
            if let Some(anim) = visual.states[state].joint.as_ref() {
                hsd_anim::load::attach_anim_joint_without_constraints(&mut tree, tree_root, anim, archive)
                    .map_err(|e| anyhow::anyhow!("article animation: {e}"))?;
            }
            // Posing constrains the attach bone each tick; its slots exist
            // from load so the tick never grows the constraint maps.
            if let Some(attach) = tree.bone(tree_root, visual.attachment_bone) {
                tree.reserve_position_constraint(attach);
                tree.reserve_orientation_constraint(attach);
            }
            trees.push(tree);
            root = Some(tree_root);
        }
        Ok(Self {
            kind,
            trees,
            current: 0,
            root: root.expect("an article with a state"),
            attach_bone: visual.attachment_bone,
            unconstrained,
        })
    }

    fn tree_mut(&mut self) -> &mut JObjTree {
        &mut self.trees[self.current]
    }
}

#[derive(Clone)]
struct Slot {
    item: Option<u32>,
    skeleton: ArticleSkeleton,
    /// HSD_JObjAnimAll steps applied since the state's request.
    steps: Option<u32>,
    /// The tick whose root placement (and dirtying) the tree holds.
    placed: Option<u64>,
}

/// Posed articles, one slot per article that can exist at once. Slots are
/// prepared at setup and reset in place, so posing allocates nothing.
#[derive(Clone, Default)]
pub(crate) struct ArticlePoses {
    slots: Vec<Slot>,
}

/// Articles of one kind that may be posed at once (one per fighter).
const SLOTS_PER_KIND: usize = 2;
/// Common items posed per kind (the Fire Flower): each keeps its tree
/// from its first pickup until it is gone, held or not.
const SLOTS_PER_ITEM_KIND: usize = 16;

/// Where the holder's part holds the article: its world matrix and its
/// orientation target. `None` for a common item out of the hand, whose
/// attach bone is no longer constrained.
pub(crate) type Hand<'a> = Option<(&'a Mtx, hsd_anim::orientation::OrientationTarget)>;

impl ArticlePoses {
    pub fn new(skeletons: &[ArticleSkeleton]) -> Self {
        let mut slots = Vec::with_capacity(skeletons.len() * SLOTS_PER_ITEM_KIND);
        for skeleton in skeletons {
            let count = if skeleton.trees.len() > 1 {
                SLOTS_PER_ITEM_KIND
            } else {
                SLOTS_PER_KIND
            };
            for _ in 0..count {
                slots.push(Slot {
                    item: None,
                    skeleton: skeleton.clone(),
                    steps: None,
                    placed: None,
                });
            }
        }
        Self { slots }
    }

    /// Forget articles that no longer exist.
    pub fn retain(&mut self, live: impl Fn(u32) -> bool) {
        for slot in &mut self.slots {
            if slot.item.is_some_and(|item| !live(item)) {
                slot.item = None;
            }
        }
    }

    /// The world matrix of `item`'s depth-first bone `bone` during tick
    /// `tick`, its attach bone constrained to the holder's part (world
    /// matrix `hand`, orientation target `hand_orientation`):
    /// HSD_JObjSetupMatrix on the posed tree.
    pub fn bone_matrix(
        &mut self,
        item: &melee_it::ItemCore,
        templates: &[ArticleSkeleton],
        bone: usize,
        tick: u64,
        hand: Hand<'_>,
    ) -> Mtx {
        let skeleton = self.pose(item, templates, tick, hand);
        let root = skeleton.root;
        let (kind, current) = (skeleton.kind, skeleton.current);
        let ArticleSkeleton {
            trees,
            unconstrained,
            ..
        } = skeleton;
        let tree = &mut trees[current];
        let joint = tree.bone(root, bone).expect("article bone");
        let mut above = Some(joint);
        while let Some(id) = above {
            assert!(
                !unconstrained.contains(&id),
                "{kind:?} bone {bone} lies below an RObj constraint the port leaves out"
            );
            above = tree.parent(id);
        }
        tree.setup_matrix(joint);
        tree.get(joint).mtx
    }

    /// A display pass (HSD_JObjDispAll through the item's render callback)
    /// sets up every joint of the held article, resolving its attach bone's
    /// constraints with the holder as it stands then; the recovered local
    /// rotation carries into the next resolution.
    /// `after_tick` is the tick the display pass follows.
    pub fn display(
        &mut self,
        item: &melee_it::ItemCore,
        templates: &[ArticleSkeleton],
        after_tick: u64,
        hand: Hand<'_>,
    ) {
        let skeleton = self.pose(item, templates, after_tick, hand);
        let tree = skeleton.tree_mut();
        for index in 0..tree.len() {
            tree.setup_matrix(JObjId(index));
        }
    }

    /// `item`'s tree, claimed on first use, brought to the item's AnimAll
    /// count; the first call for `tick` places its root (dirtying the tree)
    /// with its attach bone constrained to the hand as it is now.
    fn pose(
        &mut self,
        item: &melee_it::ItemCore,
        templates: &[ArticleSkeleton],
        tick: u64,
        hand: Hand<'_>,
    ) -> &mut ArticleSkeleton {
        let state = item.article_state;
        let index = match self.slots.iter().position(|s| s.item == Some(item.id)) {
            Some(index) => index,
            None => {
                let index = self
                    .slots
                    .iter()
                    .position(|s| s.item.is_none() && s.skeleton.kind == item.kind)
                    .expect("a free posed-article slot");
                let slot = &mut self.slots[index];
                slot.item = Some(item.id);
                // Claimed: the state's tree is reset below.
                slot.skeleton.current = usize::MAX;
                index
            }
        };
        let template = templates
            .iter()
            .find(|t| t.kind == item.kind)
            .expect("posed article template");
        assert!(
            state < template.trees.len(),
            "posed article {:?} in state {state} without its tree",
            item.kind
        );
        let slot = &mut self.slots[index];
        if slot.skeleton.current != state {
            // Item_80268D34 for this article state: its tree from the rest
            // pose and fresh playback.
            slot.skeleton.current = state;
            reset(&mut slot.skeleton.trees[state], &template.trees[state]);
            slot.steps = None;
            slot.placed = None;
        }
        let slot = &mut self.slots[index];
        let skeleton = &mut slot.skeleton;
        let root = skeleton.root;
        // Item_80268D34's request, then one AnimAll per step (the state's
        // own step 0 included).
        // pose_steps counts the state's own step, index 0 here.
        let target = item
            .pose_steps
            .checked_sub(1)
            .expect("a posed article past its state change");
        let mut applied = match slot.steps {
            Some(steps) if steps <= target => steps,
            restart => {
                if restart.is_some() && template.trees.len() > 1 {
                    // A per-state article's state entered again (its
                    // animation looping, Item_80268E5C -> Item_80268D34):
                    // lb_8000B804's rest pose before the fresh playback.
                    reset(&mut skeleton.trees[state], &template.trees[state]);
                }
                let tree = skeleton.tree_mut();
                tree.req_anim_all(root, 0.0);
                tree.anim_all::<melee_ft::fighter::RetailTrig>(root);
                0
            }
        };
        while applied < target {
            skeleton
                .tree_mut()
                .anim_all::<melee_ft::fighter::RetailTrig>(root);
            applied += 1;
        }
        slot.steps = Some(applied);
        skeleton.tree_mut().events.clear();
        if slot.placed == Some(tick) {
            return skeleton;
        }
        slot.placed = Some(tick);
        // The item's root: Item_80268E5C's translation, its rotation and scale.
        let attach_bone = skeleton.attach_bone;
        let tree = skeleton.tree_mut();
        tree.set_translate(root, &item.root_translation);
        tree.set_rotation_x(root, item.rotation.x);
        tree.set_rotation_y(root, item.rotation.y);
        tree.set_rotation_z(root, item.rotation.z);
        tree.set_scale(root, &item.model_scale);
        if let Some(spin) = item.spin_joint_rotation {
            // it_802746F8: the spin joint is that many first children down,
            // which is that depth-first index.
            let depth = usize::from(item.spin_joint_depth);
            let joint = tree.bone(root, depth).expect("item spin joint");
            tree.set_rotation_x(joint, spin.x);
            tree.set_rotation_y(joint, spin.y);
            tree.set_rotation_z(joint, spin.z);
        }
        let attach = tree.bone(root, attach_bone).expect("article attach bone");
        tree.set_position_constraint(
            attach,
            hand.map(|(hand, _)| hsd_types::Vec3::new(hand.0[0][3], hand.0[1][3], hand.0[2][3])),
        );
        tree.set_orientation_constraint(
            attach,
            hand.map(|(_, orientation)| {
                (
                    orientation,
                    hsd_anim::mtx::hsd_mtx_get_rotation::<melee_ft::fighter::RetailTrig>
                        as hsd_anim::jobj::OrientationRecovery,
                )
            }),
        );
        skeleton
    }

    /// Bone `bone` of the destroyed article `item` as a generator kept for
    /// its particles still reads it: HSD_GObjPLink_80390228 tears the tree
    /// down, so a later HSD_JObjSetupMatrix finds the dirty joint without a
    /// parent and builds its local SRT alone (retail: the sparkle's position
    /// becomes the bone's own translation). `None` for an unposed item.
    pub fn detached_bone_matrix(&mut self, item: u32, bone: usize) -> Option<Mtx> {
        let slot = self.slots.iter_mut().find(|s| s.item == Some(item))?;
        let tree = slot.skeleton.trees.get(slot.skeleton.current)?;
        let joint = tree.bone(slot.skeleton.root, bone)?;
        let node = tree.get(joint);
        let mut matrix = Mtx::default();
        if node.flags & hsd_anim::jobj::JOBJ_USE_QUATERNION != 0 {
            hsd_anim::mtx::hsd_mtx_srt_quat(
                &mut matrix,
                &node.scale,
                &node.rotate,
                &node.translate,
                None,
            );
        } else {
            let euler = hsd_types::Vec3::new(node.rotate.x, node.rotate.y, node.rotate.z);
            hsd_anim::mtx::hsd_mtx_srt(&mut matrix, &node.scale, &euler, &node.translate, None);
        }
        Some(matrix)
    }

    /// Whether `kind` is posed (it has a prepared skeleton).
    pub fn poses_kind(templates: &[ArticleSkeleton], kind: ItemKind) -> bool {
        templates.iter().any(|t| t.kind == kind)
    }
}

/// Item_80268D34's lb_8000B804: the rest pose and fresh playback, storage
/// kept.
fn reset(tree: &mut JObjTree, template: &JObjTree) {
    for index in 0..template.len() {
        let id = JObjId(index);
        let source = template.get(id);
        let target = tree.get_mut(id);
        target.flags = source.flags;
        target.rotate = source.rotate;
        target.scale = source.scale;
        target.translate = source.translate;
        target.mtx = source.mtx;
        target.scl = source.scl;
        if let (Some(target), Some(source)) = (&mut target.aobj, &source.aobj) {
            target.restore_playback(source);
        }
    }
    tree.events.clear();
}
