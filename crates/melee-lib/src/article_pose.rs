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
    tree: JObjTree,
    root: JObjId,
    /// ItemModelDesc x8: the depth-first index of the constrained bone.
    attach_bone: usize,
}
impl ArticleSkeleton {
    /// Item_80268D34's model with article state 0's joint animation.
    pub fn load(
        kind: ItemKind,
        archive: &hsd_archive::Archive,
        visual: &hsd_archive::desc::item_visual::ItemVisual,
    ) -> anyhow::Result<Self> {
        let (mut tree, root) = hsd_anim::load::load_joint_tree(archive, &visual.model)
            .map_err(|e| anyhow::anyhow!("article model: {e}"))?;
        if let Some(anim) = visual.states[0].joint.as_ref() {
            hsd_anim::load::attach_anim_joint(&mut tree, root, anim, archive)
                .map_err(|e| anyhow::anyhow!("article animation: {e}"))?;
        }
        Ok(Self {
            kind,
            tree,
            root,
            attach_bone: visual.attachment_bone,
        })
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

impl ArticlePoses {
    pub fn new(skeletons: &[ArticleSkeleton]) -> Self {
        let mut slots = Vec::with_capacity(skeletons.len() * SLOTS_PER_KIND);
        for skeleton in skeletons {
            for _ in 0..SLOTS_PER_KIND {
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
        hand: &Mtx,
        hand_orientation: hsd_anim::orientation::OrientationTarget,
    ) -> Mtx {
        let skeleton = self.pose(item, templates, tick, hand, hand_orientation);
        let joint = skeleton
            .tree
            .bone(skeleton.root, bone)
            .expect("article bone");
        skeleton.tree.setup_matrix(joint);
        skeleton.tree.get(joint).mtx
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
        hand: &Mtx,
        hand_orientation: hsd_anim::orientation::OrientationTarget,
    ) {
        let skeleton = self.pose(item, templates, after_tick, hand, hand_orientation);
        for index in 0..skeleton.tree.len() {
            skeleton.tree.setup_matrix(JObjId(index));
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
        hand: &Mtx,
        hand_orientation: hsd_anim::orientation::OrientationTarget,
    ) -> &mut ArticleSkeleton {
        assert_eq!(item.article_state, 0, "posed article outside state 0");
        let index = match self.slots.iter().position(|s| s.item == Some(item.id)) {
            Some(index) => index,
            None => {
                let index = self
                    .slots
                    .iter()
                    .position(|s| s.item.is_none() && s.skeleton.kind == item.kind)
                    .expect("a free posed-article slot");
                let template = templates
                    .iter()
                    .find(|t| t.kind == item.kind)
                    .expect("posed article template");
                let slot = &mut self.slots[index];
                reset(&mut slot.skeleton.tree, &template.tree);
                slot.item = Some(item.id);
                slot.steps = None;
                slot.placed = None;
                index
            }
        };
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
            _ => {
                skeleton.tree.req_anim_all(root, 0.0);
                skeleton
                    .tree
                    .anim_all::<melee_ft::fighter::RetailTrig>(root);
                0
            }
        };
        while applied < target {
            skeleton
                .tree
                .anim_all::<melee_ft::fighter::RetailTrig>(root);
            applied += 1;
        }
        slot.steps = Some(applied);
        skeleton.tree.events.clear();
        if slot.placed == Some(tick) {
            return skeleton;
        }
        slot.placed = Some(tick);
        // The item's root: Item_80268E5C's translation, its rotation and scale.
        skeleton.tree.set_translate(root, &item.root_translation);
        skeleton.tree.set_rotation_x(root, item.rotation.x);
        skeleton.tree.set_rotation_y(root, item.rotation.y);
        skeleton.tree.set_rotation_z(root, item.rotation.z);
        skeleton.tree.set_scale(root, &item.model_scale);
        let attach = skeleton
            .tree
            .bone(root, skeleton.attach_bone)
            .expect("article attach bone");
        skeleton.tree.set_position_constraint(
            attach,
            Some(hsd_types::Vec3::new(
                hand.0[0][3],
                hand.0[1][3],
                hand.0[2][3],
            )),
        );
        skeleton.tree.set_orientation_constraint(
            attach,
            Some((
                hand_orientation,
                hsd_anim::mtx::hsd_mtx_get_rotation::<melee_ft::fighter::RetailTrig>,
            )),
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
        let tree = &slot.skeleton.tree;
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
