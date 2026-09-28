//! s_link 7 ground pose: floor targets, two-joint leg IK, foot alignment
//! and the body's tilt to the floor's slope.
use super::{ecb::world_position, ground::EnvironmentCollision};
use crate::{desc::bones::GroundPoseBones, physics::FighterPhysics};
use gekko_math::{fma::fmadds, msl::sqrtf};
use hsd_anim::jobj::{JObjId, JObjTree};
use hsd_types::Vec3;
use melee_lb::ik::{normalize, TwoJointIk};
use melee_mp::CollMap;
use melee_types::{mp::line_kind, GroundOrAir};

/// No neighbouring line (`-1`).
const NO_LINE: i32 = -1;

#[derive(Clone, Copy, Debug, Default)]
pub struct GroundPoseFlags(pub u8);
impl GroundPoseFlags {
    pub const LEFT_LEG: u8 = 1;
    pub const RIGHT_LEG: u8 = 2;
    pub const BODY_TILT: u8 = 4;
}

pub struct GroundPose<'a> {
    pub bones: &'a GroundPoseBones,
    /// Fighter.x34_scale.y, +0x38; distinct from model scale.
    pub player_scale: f32,
    pub flags: GroundPoseFlags,
    /// p_ftCommonData->x804: the body tilt's limit in degrees.
    pub max_tilt_degrees: f32,
}

impl GroundPose<'_> {
    /// `Fighter_8006C5F4` -> `ft_80089B08` (0x80089B08), ft_0899.c:73-236.
    /// Solved matrices survive raw local-rotation restoration, as in retail.
    /// `flags` is the decoded three-bit `x221C_u16_y` (+221C, bits 6..8).
    pub fn update(
        &self,
        state: &FighterPhysics,
        environment: &EnvironmentCollision,
        map: &CollMap,
        tree: &mut JObjTree,
        root: JObjId,
    ) {
        let bones = self.bones;
        let flags = self.flags;
        if state.ground_or_air != GroundOrAir::Ground {
            return;
        }
        for (mask, leg) in [
            (GroundPoseFlags::RIGHT_LEG, bones.right_leg),
            (GroundPoseFlags::LEFT_LEG, bones.left_leg),
        ] {
            if flags.0 & mask == 0 {
                continue;
            }
            let ids = tree
                .bones(root, leg.map(|index| Some(index as usize)))
                .map(|id| id.expect("ground pose bone outside skeleton"));
            let [hip, knee, foot] = ids.map(|id| world_position(tree, id));
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
            let saved = ids.map(|id| tree.rotation(id));
            let mut ik = TwoJointIk {
                hip,
                knee,
                foot,
                extended_foot: target,
                target,
                upper_length: bones.upper_length * player_scale,
                lower_length: length,
            };
            let (correct, normal) = floor_target(
                map,
                environment.data.floor.index,
                tree.translation(root),
                &mut ik.target,
            );
            if correct {
                ik.solve(tree, ids[0], ids[1]);
            }
            align_foot(tree, ids[2], normal);
            // ft_0899.c:139-141/174-176: raw restores do NOT mark matrices dirty.
            for (id, rotation) in ids.into_iter().zip(saved) {
                tree.get_mut(id).rotate = rotation;
            }
        }
        if flags.0 & GroundPoseFlags::BODY_TILT != 0 {
            let angle = body_tilt(
                map,
                environment.data.floor.index,
                environment.data.floor.normal,
                state.facing,
                self.max_tilt_degrees,
            );
            // ftPartSetRotX(fp, 0, angle): part 0 is the Euler root.
            tree.set_rotation_x(root, angle);
        }
    }
}

/// ft_80089B08 (0x80089B08), ft_0899.c:180-232: the body's pitch to match the
/// floor. On a floor segment shorter than five units the neighbours' slopes
/// decide instead, when they differ from this floor's by over ten degrees.
/// That short-segment branch is transcribed from the asm but unwitnessed:
/// 180 explorer matches on Yoshi's Story, Battlefield and Dream Land never
/// tilted on a floor under five units long.
fn body_tilt(map: &CollMap, floor: i32, normal: Vec3, facing: f32, max_degrees: f32) -> f32 {
    // Retail @325: segments shorter than this defer to their neighbours.
    const SHORT_SEGMENT: f32 = 5.0;
    // Retail @327: ten degrees; a smaller neighbour disagreement is ignored.
    const NEIGHBOUR_THRESHOLD: f32 = 0.17453292;
    // Retail @328: degrees to radians.
    const DEGREES_TO_RADIANS: f32 = 0.017453292;
    let mut angle = facing * melee_lb::trigf::atan2f(normal.x, normal.y);
    let end = map.line_get_v1_pos(floor);
    let start = map.line_get_v0_pos(floor);
    let dy = end.y - start.y;
    let dx = end.x - start.x;
    // retail 0x8008A028 fmadds, then the inline three-step sqrtf (only for a
    // positive square; zero stays zero).
    let length = sqrtf(fmadds(dx, dx, dy * dy));
    if length < SHORT_SEGMENT {
        let neighbour_slope = |line: i32| {
            if line != NO_LINE && map.line_get_kind(line) & line_kind::FLOOR != 0 {
                let n = map.line_get_normal(line);
                Some(melee_lb::trigf::atan2f(n.x, n.y))
            } else {
                None
            }
        };
        let mut neighbours = 0.0;
        if let Some(next) = neighbour_slope(map.line_next(floor)) {
            // retail 0x8008A0D0: fmuls.
            neighbours = facing * next;
        }
        if let Some(prev) = neighbour_slope(map.line_prev(floor)) {
            // retail 0x8008A118/1C: fmadds, then fmuls by 0.5.
            neighbours = 0.5 * fmadds(facing, prev, neighbours);
        }
        if (neighbours - angle).abs() > NEIGHBOUR_THRESHOLD {
            angle = neighbours;
        }
    }
    // retail 0x8008A15C: fmuls.
    let max_angle = DEGREES_TO_RADIANS * max_degrees;
    if angle > max_angle {
        max_angle
    } else if angle < -max_angle {
        -max_angle
    } else {
        angle
    }
}

/// fn_8008998C (8008998C): a missed floor probe uses the connected floor's
/// endpoint height and leaves the normal zero, suppressing foot alignment.
fn floor_target(map: &CollMap, floor: i32, root: Vec3, target: &mut Vec3) -> (bool, Vec3) {
    let mut normal = Vec3::ZERO;
    let mut delta = if let Some(probe) = map.floor_probe(floor, target) {
        normal = probe.normal;
        // retail 80089A28/2C: fadds then fsubs.
        (target.y + probe.delta) - root.y
    } else {
        let left = map.floor_get_left(floor);
        let endpoint = if target.x > left.x {
            map.floor_get_right(floor)
        } else {
            left
        };
        endpoint.y - root.y
    };
    // Retail contact epsilon (@255), and maximum target slope (@256/257).
    const CONTACT_EPSILON: f32 = 0.0001;
    const MAX_SLOPE: f32 = 0.45;
    if delta.abs() < CONTACT_EPSILON {
        return (false, normal);
    }
    if target.x != root.x {
        let dx = target.x - root.x;
        let slope = delta / dx;
        // retail 80089AAC..CC: separate division and multiplication.
        if slope > MAX_SLOPE {
            delta = MAX_SLOPE * dx;
        } else if slope < -MAX_SLOPE {
            delta = -MAX_SLOPE * dx;
        }
    } else {
        delta = 0.0;
    }
    target.y += delta;
    (true, normal)
}

/// Euler-foot branch of lbBgFlash_80020E38 (0x80020E38, lb_020A.c:127-194).
fn align_foot(tree: &mut JObjTree, foot: JObjId, normal: Vec3) {
    // retail 80020E44..60: separate products and sums; zero normal is a no-op.
    if (normal.x * normal.x + normal.y * normal.y) + normal.z * normal.z == 0.0 {
        return;
    }
    let saved = tree.rotation(foot);
    let matrix = *tree.get_mtx(foot);
    let x = matrix.0[0][2];
    let y = matrix.0[1][2];
    let z = matrix.0[2][2];
    // retail 0x80020EF0/EF4: fmadds, after the y*y fmuls.
    let magnitude = sqrtf(fmadds(z, z, fmadds(x, x, y * y)));
    if magnitude != 0.0 {
        // Retail maximum foot tilt: twenty degrees, lb_020A.c:169-174.
        const MAX_FOOT_ANGLE: f32 = 0.34906584;
        let angle = melee_lb::trigf::atan2f(-normal.x * (z / magnitude), normal.y)
            .clamp(-MAX_FOOT_ANGLE, MAX_FOOT_ANGLE);
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
}
