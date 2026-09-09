//! Flat-floor subset of the s_link 7 ground pose. Sloped leg solving and
//! body tilt are explicit unsupported results, never silently skipped.
use super::{ecb::world_position, ground::EnvironmentCollision};
use crate::{desc::bones::GroundPoseBones, physics::FighterPhysics};
use gekko_math::{fma::fmadds, msl::sqrtf};
use hsd_anim::jobj::{JObjId, JObjTree};
use hsd_types::Vec3;
use melee_mp::CollMap;
use melee_types::GroundOrAir;

#[derive(Clone, Copy, Debug, Default)]
pub struct GroundPoseFlags(pub u8);
impl GroundPoseFlags {
    pub const LEFT_LEG: u8 = 1;
    pub const RIGHT_LEG: u8 = 2;
    pub const BODY_TILT: u8 = 4;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnsupportedGroundPose {
    LegCorrection,
    BodyTilt,
}

pub struct FlatGroundPose<'a> {
    pub bones: &'a GroundPoseBones,
    /// Fighter.x34_scale.y, +0x38; distinct from model scale.
    pub player_scale: f32,
    pub flags: GroundPoseFlags,
}

impl FlatGroundPose<'_> {
    /// `Fighter_8006C5F4` -> `ft_80089B08` (0x80089B08), ft_0899.c:73-236.
    /// On flat FD the target needs no leg correction; the foot's cached matrix
    /// is refreshed and its saved local rotation restored just as in retail.
    /// `flags` is the decoded three-bit `x221C_u16_y` (+221C, bits 6..8).
    pub fn update(
        &self,
        state: &FighterPhysics,
        environment: &EnvironmentCollision,
        map: &CollMap,
        tree: &mut JObjTree,
        root: JObjId,
    ) -> Result<(), UnsupportedGroundPose> {
        let bones = self.bones;
        let flags = self.flags;
        if state.ground_or_air != GroundOrAir::Ground {
            return Ok(());
        }
        for (mask, leg) in [
            (GroundPoseFlags::RIGHT_LEG, bones.right_leg),
            (GroundPoseFlags::LEFT_LEG, bones.left_leg),
        ] {
            if flags.0 & mask == 0 {
                continue;
            }
            let ids = leg.map(|index| {
                tree.bone(root, index as usize)
                    .expect("ground pose bone outside skeleton")
            });
            let [_, knee, foot] = ids.map(|id| world_position(tree, id));
            let player_scale = self.player_scale;
            // ft_80089B08: 0x80089B80/84 fadds then fmuls; no contraction.
            let length = player_scale * (bones.lower_length + bones.foot_extension);
            let direction = normalize(Vec3::new(foot.x - knee.x, foot.y - knee.y, foot.z - knee.z));
            // lbVector_Normalize, separate scale, lbVector_Add calls: no FMA.
            let offset = Vec3::new(
                direction.x * length,
                direction.y * length,
                direction.z * length,
            );
            let target = Vec3::new(offset.x + knee.x, offset.y + knee.y, offset.z + knee.z);
            let Some(probe) = map.floor_probe(environment.data.floor.index, &target) else {
                return Err(UnsupportedGroundPose::LegCorrection);
            };
            // fn_8008998C (0x8008998C), ft_0899.c:44-45: two fadds/fsubs.
            let delta = (target.y + probe.delta) - tree.translation(root).y;
            if delta.abs() >= 0.0001 {
                return Err(UnsupportedGroundPose::LegCorrection);
            }
            align_flat_foot(tree, ids[2], probe.normal)?;
        }
        if flags.0 & GroundPoseFlags::BODY_TILT != 0 {
            // ft_80089B08 (0x80089B08), ft_0899.c:180-233. Long flat floors
            // skip the short-segment neighbor adjustment; the clamp keeps zero.
            let normal = environment.data.floor.normal;
            let end = map.line_get_v1_pos(environment.data.floor.index);
            let start = map.line_get_v0_pos(environment.data.floor.index);
            let dx = end.x - start.x;
            let dy = end.y - start.y;
            // retail 0x8008A028 fmadds, then the standard three-step sqrtf.
            let length = sqrtf(fmadds(dx, dx, dy * dy));
            if normal.x != 0.0 || normal.y <= 0.0 || length < 5.0 {
                return Err(UnsupportedGroundPose::BodyTilt);
            }
            let angle = state.facing * melee_lb::trigf::atan2f(normal.x, normal.y);
            tree.set_rotation_x(root, angle);
        }
        Ok(())
    }
}

/// lbVector_Normalize (0x8000D2EC, lbvector.c:18-31).
fn normalize(v: Vec3) -> Vec3 {
    // retail 0x8000D2F8..D310: three fmuls, two fadds; z + (x + y).
    let length = sqrtf(v.z * v.z + (v.x * v.x + v.y * v.y));
    if length == 0.0 {
        return v;
    }
    let inverse = 1.0 / length;
    Vec3::new(v.x * inverse, v.y * inverse, v.z * inverse)
}

/// Euler-foot branch of lbBgFlash_80020E38 (0x80020E38, lb_020A.c:127-194).
fn align_flat_foot(
    tree: &mut JObjTree,
    foot: JObjId,
    normal: Vec3,
) -> Result<(), UnsupportedGroundPose> {
    if normal.x != 0.0 || normal.z != 0.0 {
        return Err(UnsupportedGroundPose::LegCorrection);
    }
    let saved = tree.rotation(foot);
    let matrix = *tree.get_mtx(foot);
    let x = matrix.0[0][2];
    let y = matrix.0[1][2];
    let z = matrix.0[2][2];
    // retail 0x80020EF0/EF4: fmadds, after the y*y fmuls.
    let magnitude = sqrtf(fmadds(z, z, fmadds(x, x, y * y)));
    if magnitude != 0.0 {
        let angle = melee_lb::trigf::atan2f(-normal.x * (z / magnitude), normal.y);
        if tree.flags(foot) & hsd_anim::jobj::JOBJ_USE_QUATERNION == 0 {
            tree.set_rotation_z(foot, angle + tree.rotation_z(foot));
        } else {
            let mut quat_matrix = hsd_types::Mtx::default();
            let mut rotation = hsd_types::Mtx::default();
            let mut result = hsd_types::Mtx::default();
            hsd_anim::mtx::mtx_quat(&mut quat_matrix, &saved);
            hsd_anim::mtx::mtx_rot_rad(&mut rotation, b'z', angle);
            hsd_anim::mtx::mtx_concat(&quat_matrix, &rotation, &mut result);
            hsd_anim::quat::mat_to_quat(&result, &mut tree.get_mut(foot).rotate);
            tree.set_mtx_dirty(foot);
        }
        tree.setup_matrix(foot);
    }
    // ft_0899.c:174-176 restores the raw local rotation WITHOUT marking dirty.
    tree.get_mut(foot).rotate = saved;
    Ok(())
}
