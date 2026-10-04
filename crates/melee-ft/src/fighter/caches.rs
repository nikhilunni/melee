//! Idle collision caches: ftColl_8007AF28/8007AF60 and ftCo_800A0DA4.
use hsd_anim::{
    jobj::{JObjId, JObjTree},
    mtx,
};
use hsd_types::Vec3;

use crate::anim::playback::FighterAnimation;
use melee_coll::hurtbox::HurtCapsule;
#[derive(Clone, Debug)]
pub struct DynamicCollider {
    pub bone: usize,
    pub offset: Vec3,
    pub radius: f32,
    pub position: Vec3,
}

/// ft_8007C17C/8007C224 (0x8007C17C/0x8007C224), ft_07C1.c:35-73.
/// This capsule follows the model even when owner is NULL and no throw hits.
#[derive(Clone, Debug, Default)]
pub struct ThrownHitbox {
    pub bone: usize,
    pub radius: f32,
    /// HitCapsuleState (+1064); enabled=1, first update=2, following=3.
    pub state: u32,
    pub offset: Vec3,
    /// x4C/x58 (+10B0/+10BC).
    pub position: Vec3,
    pub previous_position: Vec3,
}
impl ThrownHitbox {
    pub fn update(&mut self, tree: &mut JObjTree, animation: &FighterAnimation) {
        match self.state {
            0 => {}
            1 => {
                self.position = part_position(tree, animation, self.bone, self.offset);
                self.previous_position = self.position;
                self.state = 2;
            }
            2 | 3 => {
                self.state = 3;
                self.previous_position = self.position;
                self.position = part_position(tree, animation, self.bone, self.offset);
            }
            _ => unimplemented!("ft_07C1.c:71: unsupported HitCapsule state {}", self.state),
        }
    }
}
/// lb_8000B1CC (0x8000B1CC), lb_00B0.c:105-140. A zero local offset
/// copies the matrix translation directly (preserving signed zero).
pub fn bone_position(tree: &mut JObjTree, root: JObjId, bone: usize, offset: Vec3) -> Vec3 {
    let joint = tree.bone(root, bone).expect("collision bone");
    joint_position(tree, joint, offset)
}
/// `bone_position` for one of a fighter's own bones. Retail's capsules hold
/// the bone's JObj (Fighter.parts, filled by the same depth-first walk at
/// load), so the per-tick caches read the parts table instead of walking the
/// skeleton again for every capsule.
pub fn part_position(
    tree: &mut JObjTree,
    animation: &FighterAnimation,
    bone: usize,
    offset: Vec3,
) -> Vec3 {
    let joint = animation.parts[bone].joint;
    debug_assert_eq!(
        tree.bone(animation.root, bone),
        Some(joint),
        "fighter parts follow the skeleton's depth-first order"
    );
    joint_position(tree, joint, offset)
}
/// lb_8000B1CC's transform of `offset` by a joint's world matrix.
fn joint_position(tree: &mut JObjTree, joint: JObjId, offset: Vec3) -> Vec3 {
    if offset.x == 0.0 && offset.y == 0.0 && offset.z == 0.0 {
        return crate::collision::ecb::world_position(tree, joint);
    }
    let matrix = *tree.get_mtx(joint);
    let mut position = Vec3::ZERO;
    mtx::mtx_mult_vec(&matrix, &offset, &mut position);
    position
}
/// ftCo_800A0DA4 (0x800A0DA4), ftCo_0A01.c:553-609. The caller clears
/// caches at s_link 4; collision consumers can evaluate them before s_link 14.
pub fn hurtbox_extents(
    boxes: &mut [HurtCapsule],
    tree: &mut JObjTree,
    animation: &FighterAnimation,
    position: Vec3,
    facing: f32,
    player_scale: f32,
) -> [f32; 4] {
    let (mut left, mut right, mut top) = (0.0, 0.0, 0.0);
    for hurt in boxes {
        if !hurt.cached {
            hurt.positions = hurt
                .offsets
                .map(|offset| part_position(tree, animation, hurt.bone, offset));
            hurt.cached = true;
        }
        // asm.py ftCo_800A0DA4 --fused: none. Each bound is a separate add/sub.
        let radius = hurt.radius * player_scale;
        for point in hurt.positions {
            let x = point.x - position.x;
            let y = point.y - position.y;
            if left > x - radius {
                left = x - radius;
            }
            if right < x + radius {
                right = x + radius;
            }
            if top < y + radius {
                top = y + radius;
            }
        }
    }
    let (front, back) = if facing > 0.0 {
        (right, -left)
    } else {
        (-left, right)
    };
    // C uses double 0.5 after the single-precision addition, then rounds.
    [front, back, (0.5_f64 * f64::from(front + back)) as f32, top]
}

/// The matrix a hurt, shield, reflect or absorb test takes for a bone: its
/// world matrix, behind the fighter's x44_mtx when it is flat
/// (PSMTXConcat(ftCommon_8007F804(fp), bone->mtx)).
pub(super) fn unflattened(flat: Option<&hsd_types::Mtx>, bone: &hsd_types::Mtx) -> hsd_types::Mtx {
    match flat {
        Some(flat) => {
            let mut matrix = hsd_types::Mtx::default();
            mtx::mtx_concat(flat, bone, &mut matrix);
            matrix
        }
        None => *bone,
    }
}

impl super::FighterCore {
    /// Fighter_UnkApplyTransformation_8006C0F0 (8006C0F0), from
    /// Fighter_8006C80C after the effect flush: for a flat fighter, the
    /// root's transform at the model scale along its x axis, times the
    /// inverse of its world matrix.
    pub fn update_flat_matrix(&mut self) {
        if self.capabilities.model_width.is_none() {
            return;
        }
        let root = self.animation.root;
        let inverse = hsd_types::Mtx(gekko_math::matrix::inverse(&self.skeleton.get_mtx(root).0));
        let mut scale = self.skeleton.scale(root);
        // ftCommon_GetModelScale: fmuls.
        scale.x = self.player.scale * self.attributes.size.model_scaling;
        let rotation = self.skeleton.rotation(root);
        let rotation = Vec3::new(rotation.x, rotation.y, rotation.z);
        let translation = self.skeleton.translation(root);
        let mut full = hsd_types::Mtx::default();
        mtx::hsd_mtx_srt(&mut full, &scale, &rotation, &translation, None);
        let mut flat = hsd_types::Mtx::default();
        mtx::mtx_concat(&full, &inverse, &mut flat);
        self.combat.flat_matrix = Some(flat);
    }
}

impl melee_coll::detection::Collider for super::FighterCore {
    fn hurt_count(&self) -> usize {
        self.hurtboxes.len()
    }
    /// ftColl_8007B128 (8007B128) sets the state of the *first* capsule on the
    /// commanded bone and returns, so a second capsule on that bone (Fox's
    /// head) keeps the fighter-wide state.
    fn hurt_status(&self, index: usize) -> melee_types::combat::HurtStatus {
        let bone = self.hurtboxes[index].bone;
        let first_on_bone = self.hurtboxes.iter().position(|h| h.bone == bone) == Some(index);
        self.commands
            .capsule_overrides
            .iter()
            .find(|entry| first_on_bone && entry.0 == bone)
            .map_or(self.commands.capsule_status, |entry| entry.1)
    }
    fn grabbable(&self, index: usize) -> bool {
        self.hurtboxes[index].grabbable
    }
    fn sample_hurt(&mut self, index: usize) -> (HurtCapsule, hsd_types::Mtx) {
        let depth = self.physics.position.z;
        let flat = self.combat.flat_matrix;
        let hurt = &mut self.hurtboxes[index];
        if !hurt.cached {
            hurt.positions = hurt.offsets.map(|offset| {
                bone_position(&mut self.skeleton, self.animation.root, hurt.bone, offset)
            });
            // lbColl_80007ECC / lbColl_8000805C: a flat fighter's capsule,
            // when the test is what places it, sits at the fighter's depth.
            if flat.is_some() {
                for position in &mut hurt.positions {
                    position.z = depth;
                }
            }
            hurt.cached = true;
        }
        let bone = self
            .skeleton
            .bone(self.animation.root, hurt.bone)
            .expect("hurt bone");
        let matrix = unflattened(flat.as_ref(), self.skeleton.get_mtx(bone));
        (hurt.clone(), matrix)
    }
    fn scale(&self) -> f32 {
        self.player.scale
    }
}

impl super::FighterCore {
    /// lbColl_80008248 (80008248) over every hurt capsule in table order,
    /// whatever its state: whether a swept hit capsule from `previous` to
    /// `position` with `radius` touches the fighter (the broad phase
    /// widened by 3 * the fighter's scale).
    pub fn hurt_capsules_touched(
        &mut self,
        previous: hsd_types::Vec3,
        position: hsd_types::Vec3,
        radius: f32,
    ) -> bool {
        use melee_coll::detection::Collider;
        if self.player.scale != 1.0 {
            unimplemented!("lbColl_80008248: a scaled fighter's hurt matrix (ftCommon_8007F804)");
        }
        (0..self.hurt_count()).any(|index| {
            let (hurt, matrix) = self.sample_hurt(index);
            melee_coll::geometry::capsule_contact(
                melee_coll::geometry::Capsule {
                    start: previous,
                    end: position,
                    radius,
                },
                melee_coll::geometry::Capsule {
                    start: hurt.positions[0],
                    end: hurt.positions[1],
                    radius: hurt.radius,
                },
                &matrix,
                3.0 * self.player.scale,
            )
            .is_some()
        })
    }
    /// ftColl_8007B0C0 (8007B0C0): every hurt capsule takes `status`.
    pub fn set_hurt_capsules(&mut self, status: melee_types::combat::HurtStatus) {
        self.commands.capsule_status = status;
        self.commands.capsule_overrides.clear();
    }
    /// ftColl_HurtboxInit (8007B5AC): overwrite one capsule, enabled, until
    /// the next motion change restores the table.
    pub fn replace_hurt_capsule(&mut self, index: usize, capsule: HurtCapsule) {
        let bone = capsule.bone;
        self.hurtboxes[index] = capsule;
        // Per-capsule states are keyed by the first capsule on a bone.
        assert!(
            self.hurtboxes.iter().position(|h| h.bone == bone) == Some(index),
            "ftColl_HurtboxInit: an earlier capsule shares the bone"
        );
        let enabled = melee_types::combat::HurtStatus::Normal;
        let overrides = &mut self.commands.capsule_overrides;
        let mut existing = false;
        for entry in overrides.iter_mut() {
            if entry.0 == bone {
                entry.1 = enabled;
                existing = true;
                break;
            }
        }
        if !existing {
            overrides.push((bone, enabled));
        }
        self.hurtboxes_replaced = true;
    }
    /// ftColl_8007B4E0 (8007B4E0): the data table's capsules, all enabled.
    pub(super) fn restore_hurt_capsules(&mut self, assets: &super::assets::FighterAssets) {
        self.hurtboxes.clone_from_slice(&assets.hurtboxes);
        self.set_hurt_capsules(melee_types::combat::HurtStatus::Normal);
        self.hurtboxes_replaced = false;
    }
}
