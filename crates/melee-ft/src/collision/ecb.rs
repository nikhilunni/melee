use crate::desc::EcbBones;
use hsd_anim::jobj::{JObjId, JObjTree};
use hsd_types::Vec3;
use melee_mp::{set_ecb_source_jobj, set_ledge_snap, CollMap};
use melee_types::mp::{CollData, EcbSourceParams};

/// `ft_80081B38` (0x80081B38, ft_081B.c:33-68).
/// Player scale is `Fighter.x34_scale.y`, not the model's attribute scale.
pub fn initialize(
    map: &CollMap,
    position: Vec3,
    bones: &EcbBones,
    player_scale: f32,
    weight: f32,
) -> CollData {
    let mut collision = CollData::default();
    reinitialize(map, &mut collision, position, bones, player_scale, weight);
    collision
}

/// Reuse the existing CollData storage, as ft_80081B38 does on stock reset.
/// mpColl_80041EE4 resets named fields and retains other flag bits.
pub fn reinitialize(
    map: &CollMap,
    collision: &mut CollData,
    position: Vec3,
    bones: &EcbBones,
    player_scale: f32,
    weight: f32,
) {
    collision.cur_pos = position;
    map.coll_data_init(collision);
    collision.x34_flags.b1234 = 1;
    let joints = bones
        .joints
        .map(|i| Some(u32::try_from(i).expect("negative ECB bone index")));
    // retail ft_80081B38: fmuls only; asm.py --fused has no sites.
    set_ecb_source_jobj(collision, Some(0), joints, bones.center_y * player_scale);
    set_ledge_snap(
        collision,
        bones.ledge_snap_x * player_scale,
        bones.ledge_snap_y * player_scale,
        bones.ledge_snap_height * player_scale,
    );
    collision.x50 = weight;
    let minimum_size = 10.0 * player_scale;
    collision.ecb_source.x128 = minimum_size;
    collision.ecb_source.x12c = minimum_size;
}

/// `lb_8000B1CC` (0x8000B1CC, lb_00B0.c:105-130), NULL local offset.
/// Roots use their translation directly; child bones force matrix setup.
pub fn world_position(tree: &mut JObjTree, joint: JObjId) -> Vec3 {
    if tree.parent(joint).is_none() {
        return tree.translation(joint);
    }
    let matrix = tree.get_mtx(joint);
    Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3])
}

/// Snapshot only the referenced joints so `melee-mp`'s immutable BoneLookup
/// can consume positions after JObjTree has lazily rebuilt their matrices.
pub struct EcbPose {
    // CollData's JObj source has six extent joints and one center joint
    // (lb/types.h:186-187, x10C_joint[6] and x108_joint).
    positions: [(u32, Vec3); 7],
    len: usize,
}
impl EcbPose {
    pub fn read(tree: &mut JObjTree, root: JObjId, collision: &CollData) -> Self {
        let mut pose = Self {
            positions: [(0, Vec3::ZERO); 7],
            len: 0,
        };
        if let EcbSourceParams::JObj {
            x108_joint,
            x10c_joint,
        } = collision.ecb_source.params
        {
            // Each distinct bone once, in first-use order.
            for index in x10c_joint.into_iter().chain([x108_joint]).flatten() {
                if pose.positions[..pose.len].iter().any(|&(i, _)| i == index) {
                    continue;
                }
                pose.positions[pose.len].0 = index;
                pose.len += 1;
            }
            // One skeleton walk resolves every sampled bone.
            let indices: [Option<usize>; 7] = std::array::from_fn(|slot| {
                (slot < pose.len).then(|| pose.positions[slot].0 as usize)
            });
            let joints = tree.bones(root, indices);
            for (slot, joint) in joints[..pose.len].iter().enumerate() {
                let joint = joint.expect("ECB bone outside skeleton");
                pose.positions[slot].1 = world_position(tree, joint);
            }
        }
        pose
    }
    pub fn position(&self, index: Option<u32>) -> Vec3 {
        let index = index.expect("NULL ECB bone");
        self.positions[..self.len]
            .iter()
            .find(|&&(i, _)| i == index)
            .expect("unsampled ECB bone")
            .1
    }
}

/// Build the grounded desired diamond without stepping collision, useful at
/// initial-state reconstruction. Same flags (5) as mpColl_8004B4B0; mp owns
/// JObj padding/minimums, fixed sources and locked-bottom preservation.
pub fn load_grounded(collision: &mut CollData, tree: &mut JObjTree, root: JObjId) {
    let pose = EcbPose::read(tree, root, collision);
    melee_mp::load_ecb_with_flags(collision, 5, Some(&|i| pose.position(i)));
}
