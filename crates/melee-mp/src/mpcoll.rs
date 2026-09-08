//! `mpcoll.c`: ECB (environment collision box) construction and the
//! collision response that moves a `CollData` (a fighter's or item's
//! collision state) against the map.
//!
//! The C keeps its scratch in file-scope statics: the wall-id lists
//! `mpColl_80458810` with counts `mpColl_804D6488`/`8C`, the best-wall
//! accumulator `mpColl_804D6490_max_x`/`94_line_id`/`98_flags`,
//! `mpColl_IsEcbTiny`, and the platform-pass callback pair
//! `mpColl_804D64A0`/`A4`. Here the first group is [`CollScratch`] inside
//! [`CollMap`]; the callback pair is passed down explicitly as a
//! [`LineFilter`] (it is set immediately before and read only inside the
//! same call). The four collision routines `mpColl_80043754` dispatches to
//! become [`CollideMode`]. `grDynamicAttr_801CA284` is the
//! [`DynamicAttrHook`] on the map; `lb_8000B1CC` (bone world position) is a
//! [`BoneLookup`] the caller supplies.
//!
//! Debug-ROM `OSReport` paths are omitted. Every float op is in the C's
//! order; `ABS()` is the platform macro `(x) < 0 ? -(x) : (x)`, which keeps
//! `-0.0`, so it is [`c_abs`] rather than `fabsf` wherever its result feeds
//! arithmetic.

use gekko_math::fma::{fmadds, fmsubs, fnmsubs};
use gekko_math::msl::{cosf, sinf};
use hsd_types::{Vec2, Vec3};
use melee_types::mp::{
    coll_data_x130, collide, line_flag, line_kind, CollData, EcbSourceKind, EcbSourceParams,
    FtCollisionBox, FtEcb, MpLibGroundEnum, SurfaceData, NO_ID,
};

use crate::map::{CollMap, F32_MAX};
use crate::query::LineFilter;

/// `CollisionFlagAir_*` (`mpcoll.c:73-75`): the `flags` word of the airborne
/// collision routine `mpColl_80046904`.
pub mod air_flags {
    /// `CollisionFlagAir_StayAirborne`: snap to the floor line but do not
    /// land (`mpColl_80044948_Floor`).
    pub const STAY_AIRBORNE: u32 = 0x1;
    /// `CollisionFlagAir_PlatformPassCallback`: consult the floor filter
    /// callback (drop-through platforms).
    pub const PLATFORM_PASS_CALLBACK: u32 = 0x2;
    /// `CollisionFlagAir_CanGrabLedge`.
    pub const CAN_GRAB_LEDGE: u32 = 0x4;
}

/// `ABS(x)` from `Runtime/platform.h:198`: `(x) < 0 ? -(x) : (x)`.
#[inline]
pub(crate) fn c_abs(x: f32) -> f32 {
    if x < 0.0 {
        -x
    } else {
        x
    }
}

/// `grDynamicAttr_801CA284(pos, floor_id) -> material`: the stage's dynamic
/// terrain override, or 0 for none.
pub type DynamicAttrHook = fn(pos: &Vec3, floor_id: i32) -> i32;

/// `lb_8000B1CC(jobj, NULL, &vec)`: world position of an ECB source bone,
/// given the bone slot stored in [`EcbSourceParams::JObj`].
pub type BoneLookup<'a> = &'a dyn Fn(Option<u32>) -> Vec3;

/// `mpColl_80458810` and the `mpColl_804D6488..98` statics.
#[derive(Clone, Debug, Default)]
pub struct CollScratch {
    /// `mpColl_80458810.right[9]`
    right: [i32; 9],
    /// `mpColl_80458810.left[9]`
    left: [i32; 9],
    /// `mpColl_80458810.normal`
    normal: Vec3,
    /// `mpColl_804D6488`
    right_count: i32,
    /// `mpColl_804D648C`
    left_count: i32,
    /// `mpColl_804D6490_max_x`
    max_x: f32,
    /// `mpColl_804D6494_line_id`
    line_id: i32,
    /// `mpColl_804D6498_flags`
    flags: u32,
    /// `mpColl_IsEcbTiny`
    is_ecb_tiny: bool,
}

/// Which of the four collision routines `mpColl_80043754` steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CollideMode {
    /// `mpColl_80046904`
    Air,
    /// `mpColl_80046F78`
    Point,
    /// `mpColl_8004ACE4`
    Ground,
    /// `mpColl_8004C534`
    Ceiling,
}

/// A wall side, for the routines that exist in mirrored pairs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    /// Right walls (normal +x) are touched by the ECB's left side.
    Right,
    /// Left walls (normal -x) are touched by the ECB's right side.
    Left,
}

/// The `x0` column of the terrain tables `mpLib_803BD3D8..803BDBC0`
/// (`mplib.c:92-235`), indexed by material. Every stage's table in
/// `mpLib_803BF248` maps material `i` to an entry with this `x0`, so
/// `mpLib_800569EC` is stage-independent.
pub const TERRAIN_SPEED_SCALE: [f32; 20] = [
    1.0, 1.0, 1.5, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.1, 0.9, 1.0, 0.2, 1.0, 0.1, 1.0,
    1.0,
];

/// `mpLib_800569EC` (retail `0x800569EC`, `mplib.c:5155`): the terrain speed
/// scale of a line's `lo_flags`, indexed by its material byte. The C indexes
/// a 20-entry table with `(u8) flags` and no bound check; a material past 19
/// is asserted here.
pub fn terrain_speed_scale(flags: u32) -> f32 {
    let idx = (flags & line_flag::MATERIAL_MASK) as usize;
    assert!(
        idx < TERRAIN_SPEED_SCALE.len(),
        "mpLib_800569EC: material {idx} out of table"
    );
    TERRAIN_SPEED_SCALE[idx]
}

// ---------------------------------------------------------------------------
// CollData-only functions
// ---------------------------------------------------------------------------

/// `mpCollPrev` (retail `0x80041C8C`, `mpcoll.c:87`): start a collision
/// pass. The C also clears the platform-pass callback statics, which are a
/// parameter here, and reports out-of-range positions on the debug ROM.
pub fn coll_prev(cd: &mut CollData) {
    cd.x28_vec = cd.cur_pos;
}

/// The default 8-tall, 8-wide diamond written when `x34_flags.b0` is set.
fn default_ecb() -> FtEcb {
    FtEcb {
        top: Vec2::new(0.0, 8.0),
        bottom: Vec2::new(0.0, 0.0),
        right: Vec2::new(4.0, 4.0),
        left: Vec2::new(-4.0, 4.0),
    }
}

/// `mpColl_SetECBSource_JObj` (retail `0x80042078`, `mpcoll.c:220`).
/// `x108` and `joints` are the bone slots for `x108_joint` and
/// `x10C_joint[6]`; `x124` is the C's `arg9`.
pub fn set_ecb_source_jobj(
    cd: &mut CollData,
    x108: Option<u32>,
    joints: [Option<u32>; 6],
    x124: f32,
) {
    cd.ecb_source.kind = EcbSourceKind::JObj.into();
    cd.ecb_source.params = EcbSourceParams::JObj {
        x108_joint: x108,
        x10c_joint: joints,
    };
    cd.ecb_source.x124 = x124;
    cd.ecb_source.x128 = 10.0;
    cd.ecb_source.x12c = 10.0;
    if cd.x34_flags.b0 {
        cd.ecb = default_ecb();
        cd.prev_ecb = cd.ecb;
        cd.xe4_ecb = cd.ecb;
        cd.x64_ecb = cd.ecb;
        cd.desired_ecb = cd.ecb;
    }
    cd.facing_dir = -1;
    cd.x50 = 0.0;
}

/// `mpColl_SetECBSource_Fixed` (retail `0x8004220C`, `mpcoll.c:257`).
pub fn set_ecb_source_fixed(cd: &mut CollData, up: f32, down: f32, front: f32, back: f32) {
    cd.ecb_source.kind = EcbSourceKind::Fixed.into();
    cd.ecb_source.params = EcbSourceParams::Fixed {
        up,
        down,
        front,
        back,
        angle: 0.0,
    };
    if cd.x34_flags.b0 {
        cd.ecb = default_ecb();
        cd.prev_ecb = cd.ecb;
        cd.xe4_ecb = cd.ecb;
        cd.x64_ecb = cd.ecb;
        cd.desired_ecb = cd.ecb;
    }
    cd.facing_dir = -1;
}

/// `mpColl_SetLedgeSnap` (retail `0x80042374`).
pub fn set_ledge_snap(
    cd: &mut CollData,
    ledge_snap_x: f32,
    ledge_snap_y: f32,
    ledge_snap_height: f32,
) {
    cd.ledge_snap_x = ledge_snap_x;
    cd.ledge_snap_y = ledge_snap_y;
    cd.ledge_snap_height = ledge_snap_height;
}

/// `mpColl_80042384` (retail `0x80042384`, `mpcoll.c:294`): force the
/// desired ECB to a sane diamond (at least 1 tall and 2 wide, side points
/// strictly between top and bottom).
pub fn sanitize_desired_ecb(cd: &mut CollData) {
    let d = &mut cd.desired_ecb;
    if c_abs(d.top.y - d.bottom.y) < 1.0 {
        d.top.y += 1.0;
        let mid = 0.5 * (d.top.y + d.bottom.y);
        d.left.y = mid;
        d.right.y = mid;
    }
    if d.top.y < 1.0 {
        d.top.y = 1.0;
    }
    if d.left.x > -1.0 {
        d.left.x = -1.0;
    }
    if d.right.x < 1.0 {
        d.right.x = 1.0;
    }
    if d.top.y < d.bottom.y {
        d.top.y = 1.0 + d.bottom.y;
    }
    if d.right.y > d.top.y || d.right.y < d.bottom.y {
        let mid = 0.5 * (d.top.y + d.bottom.y);
        d.left.y = mid;
        d.right.y = mid;
    }
    if d.top.y - d.right.y < 0.001 || d.right.y - d.bottom.y < 0.001 {
        d.right.y = 0.5 * (d.top.y + d.bottom.y);
    }
    if d.top.y - d.left.y < 0.001 || d.left.y - d.bottom.y < 0.001 {
        d.left.y = 0.5 * (d.top.y + d.bottom.y);
    }
}

/// `update_min_max` (`mpcoll.c:336`): `if min > v {min = v} else if max < v {max = v}`.
#[inline]
fn update_min_max(min: &mut f32, max: &mut f32, val: f32) {
    if *min > val {
        *min = val;
    } else if *max < val {
        *max = val;
    }
}

/// `update_min_max_2` (`mpcoll.c:459`): the max test first.
#[inline]
fn update_min_max_2(min: &mut f32, max: &mut f32, val: f32) {
    if *max < val {
        *max = val;
    } else if *min > val {
        *min = val;
    }
}

/// The `CollData_X130_Clear` prologue shared by the three ECB loaders.
fn clear_ecb_if_flagged(cd: &mut CollData) {
    if cd.x130_flags & coll_data_x130::CLEAR != 0 {
        cd.ecb = FtEcb::default();
        cd.x130_flags &= !coll_data_x130::CLEAR;
    }
    cd.xe4_ecb = cd.ecb;
}

/// `mpColl_LoadECB_JObj` (retail `0x800424DC`, `mpcoll.c:345`): fit the
/// desired ECB around the six source bones. `flags` bits: 1 grounded
/// (bottom at 0), 4 no 2-unit padding (ledge grab), 8 width forced to
/// +-1, 0x10 height forced to 2.
pub fn load_ecb_jobj(cd: &mut CollData, flags: u32, bone: BoneLookup<'_>) {
    clear_ecb_if_flagged(cd);
    let joints = match cd.ecb_source.params {
        EcbSourceParams::JObj { x10c_joint, .. } => x10c_joint,
        EcbSourceParams::Fixed { .. } => {
            panic!("mpColl_LoadECB_JObj on a fixed ECB source")
        }
    };

    // Loop through all collision data joints, expanding the ECB to
    // contain them all
    let temp_x = cd.cur_pos.x;
    let temp_y = cd.cur_pos.y;
    let vec = bone(joints[0]);
    let mut left_x = vec.x - temp_x;
    let mut right_x = left_x;
    let mut bottom_y = vec.y - temp_y;
    let mut top_y = bottom_y;
    for &j in &joints[1..] {
        let vec = bone(j);
        let dx = vec.x - temp_x;
        let dy = vec.y - temp_y;
        update_min_max(&mut left_x, &mut right_x, dx);
        update_min_max(&mut bottom_y, &mut top_y, dy);
    }

    if flags & air_flags::CAN_GRAB_LEDGE == 0 {
        left_x -= 2.0;
        right_x += 2.0;
        bottom_y -= 2.0;
        top_y += 2.0;
    }

    let phi_f1 = if 4.0 > cd.ecb_source.x12c {
        4.0
    } else {
        cd.ecb_source.x12c
    };
    let phi_f2 = c_abs(right_x - left_x);
    if phi_f2 < phi_f1 {
        right_x = 0.5 * phi_f2;
        left_x = -right_x;
    }

    let phi_f1 = if 4.0 > cd.ecb_source.x128 {
        4.0
    } else {
        cd.ecb_source.x128
    };
    let phi_f2 = c_abs(top_y - bottom_y);
    if phi_f2 < phi_f1 {
        let tmpval = 0.5 * phi_f2;
        let mid_y = 0.5 * (top_y + bottom_y);
        top_y = mid_y + tmpval;
        bottom_y = mid_y - tmpval;
    }

    if flags & 0x8 != 0 {
        left_x = -1.0;
        right_x = 1.0;
    } else {
        right_x = if right_x < 2.0 { 2.0 } else { right_x };
        left_x = if left_x > -2.0 { -2.0 } else { left_x };
    }

    if flags & 1 != 0 {
        bottom_y = 0.0;
        if flags & 0x10 != 0 {
            top_y = 2.0;
        }
    } else {
        if bottom_y < 0.0 {
            bottom_y = 0.0;
        }
        if flags & 0x10 != 0 {
            let mid_y = 0.5 * (bottom_y + top_y);
            bottom_y = mid_y - 1.0;
            top_y = mid_y + 1.0;
            if bottom_y < 0.0 {
                bottom_y = 0.0;
                top_y = 2.0;
            }
        }
    }

    cd.desired_ecb.top = Vec2::new(0.0, top_y);
    cd.desired_ecb.bottom = Vec2::new(0.0, bottom_y);
    // retail 0x800428E4/E8/F8: fmuls + fadds, not fused (shared rounded half-sum).
    let side_y = cd.ecb_source.x124 + 0.5 * (bottom_y + top_y);
    cd.desired_ecb.right = Vec2::new(right_x, side_y);
    cd.desired_ecb.left = Vec2::new(left_x, side_y);
    cd.x34_flags.b0 = false;
}

/// `mpColl_LoadECB_Fixed` (retail `0x8004293C`, `mpcoll.c:482`): the
/// desired ECB from the fixed `up/down/front/back` extents, rotated by
/// `angle` and mirrored by `facing_dir`.
pub fn load_ecb_fixed(cd: &mut CollData) {
    let (up, down, front, back, angle) = match cd.ecb_source.params {
        EcbSourceParams::Fixed {
            up,
            down,
            front,
            back,
            angle,
        } => (up, down, front, back, angle),
        // The C reads the union as floats regardless; a JObj source here
        // would be garbage in retail too.
        EcbSourceParams::JObj { .. } => panic!("mpColl_LoadECB_Fixed on a JObj ECB source"),
    };
    clear_ecb_if_flagged(cd);

    let mut bottom_y = -down;
    let mut top_y = up;
    let (mut right_x, mut left_x) = if cd.facing_dir == 1 {
        (front, -back)
    } else {
        (back, -front)
    };

    if angle != 0.0 {
        let sin = sinf(angle);
        let cos = cosf(angle);

        let orig_top_y = top_y;
        let orig_bottom_y = bottom_y;
        let orig_right_x = right_x;
        let orig_left_x = left_x;

        // fake
        let midpoint_x = 0.5 * (orig_right_x + orig_left_x);

        top_y = 0.0;
        right_x = 0.0;
        bottom_y = 0.0;
        left_x = 0.0;

        let rot_top_x = -orig_top_y * sin;
        let rot_top_y = orig_top_y * cos;
        update_min_max_2(&mut left_x, &mut right_x, rot_top_x);
        update_min_max_2(&mut bottom_y, &mut top_y, rot_top_y);

        let rot_bot_x = -orig_bottom_y * sin;
        let rot_bot_y = orig_bottom_y * cos;
        update_min_max_2(&mut left_x, &mut right_x, rot_bot_x);
        update_min_max_2(&mut bottom_y, &mut top_y, rot_bot_y);

        // retail 0x80042AEC/F0/F4/F8: fmuls, fmuls, fmsubs, fmadds.
        let rot_right_x = fmsubs(orig_right_x, cos, midpoint_x * sin);
        let rot_right_y = fmadds(orig_right_x, sin, midpoint_x * cos);
        update_min_max_2(&mut left_x, &mut right_x, rot_right_x);
        update_min_max_2(&mut bottom_y, &mut top_y, rot_right_y);

        // retail 0x80042B34/38/3C/40: fmuls, fmuls, fmsubs, fmadds.
        let rot_left_x = fmsubs(orig_left_x, cos, midpoint_x * sin);
        let rot_left_y = fmadds(orig_left_x, sin, midpoint_x * cos);
        update_min_max_2(&mut left_x, &mut right_x, rot_left_x);
        update_min_max_2(&mut bottom_y, &mut top_y, rot_left_y);
    }

    if top_y < 0.0 {
        top_y = 0.0;
    }
    if bottom_y > -0.0 {
        bottom_y = -0.0;
    }
    if right_x < 0.0 {
        right_x = 0.0;
    }
    if left_x > -0.0 {
        left_x = -0.0;
    }

    if (top_y - bottom_y) < 3.0 {
        top_y = 1.5;
        bottom_y = -top_y;
    }
    if (right_x - left_x) < 3.0 {
        right_x = 1.5;
        left_x = -right_x;
    }

    cd.desired_ecb.top = Vec2::new(0.0, top_y);
    cd.desired_ecb.bottom = Vec2::new(0.0, bottom_y);
    let midpoint_y = 0.5 * (top_y + bottom_y);
    cd.desired_ecb.right = Vec2::new(right_x, midpoint_y);
    cd.desired_ecb.left = Vec2::new(left_x, midpoint_y);
    cd.x34_flags.b0 = false;
}

/// `mpColl_80042C58` (retail `0x80042C58`, `mpcoll.c:599`): the desired ECB
/// straight from a fighter-supplied box.
pub fn load_ecb_box(cd: &mut CollData, b: &FtCollisionBox) {
    clear_ecb_if_flagged(cd);
    cd.desired_ecb.top = Vec2::new(0.0, b.top);
    cd.desired_ecb.bottom = Vec2::new(0.0, b.bottom);
    cd.desired_ecb.right = b.right;
    cd.desired_ecb.left = b.left;
    cd.x34_flags.b0 = false;
}

/// `mpColl_LoadECB_inline` / `mpColl_LoadECB` (retail `0x80042D24`,
/// `mpcoll.c:624-669`): load the desired ECB from its source, preserving the
/// bottom point when `CollData_X130_Locked`, then sanitize. `bones` is
/// needed only for a JObj source.
pub fn load_ecb_with_flags(cd: &mut CollData, flags: u32, bones: Option<BoneLookup<'_>>) {
    let saved = if cd.x130_flags & coll_data_x130::LOCKED != 0 {
        Some(cd.desired_ecb.bottom)
    } else {
        None
    };
    if cd.ecb_source.kind == i32::from(EcbSourceKind::JObj) {
        let bones = bones.expect("JObj ECB source needs a bone lookup");
        load_ecb_jobj(cd, flags, bones);
    } else {
        load_ecb_fixed(cd);
    }
    if let Some(b) = saved {
        cd.desired_ecb.bottom = b;
    }
    sanitize_desired_ecb(cd);
}

/// `mpColl_LoadECB` (retail `0x80042D24`): [`load_ecb_with_flags`] with 6.
pub fn load_ecb(cd: &mut CollData, bones: Option<BoneLookup<'_>>) {
    load_ecb_with_flags(cd, 6, bones);
}

/// The `0x12` variant several entry points use (`mpcoll.c:2824-2829`):
/// load without the `Locked` bottom preservation, then sanitize.
fn load_ecb_0x12_unlocked(cd: &mut CollData, bones: Option<BoneLookup<'_>>) {
    if cd.ecb_source.kind == i32::from(EcbSourceKind::JObj) {
        let bones = bones.expect("JObj ECB source needs a bone lookup");
        load_ecb_jobj(cd, 0x12, bones);
    } else {
        load_ecb_fixed(cd);
    }
    sanitize_desired_ecb(cd);
}

/// `Vec2_Interpolate` (`mpcoll.c:675`).
#[inline]
fn vec2_interpolate(time: f32, dest: &mut Vec2, src: &Vec2) {
    // retail 0x80042E68..0x80042EF4 (stride 0x14): fmadds (each x/y component).
    dest.x = fmadds(time, src.x - dest.x, dest.x);
    dest.y = fmadds(time, src.y - dest.y, dest.y);
}

/// `mpCollInterpolateECB` (retail `0x80042DB0`, `mpcoll.c:681`): move the
/// ECB `time` of the way to the desired ECB. Panics on NaN like the C's
/// `HSD_ASSERTREPORT(1193)`.
pub fn interpolate_ecb(cd: &mut CollData, time: f32) {
    cd.prev_ecb = cd.ecb;
    if cd.x34_flags.b6 {
        cd.ecb = cd.x64_ecb;
        cd.x34_flags.b6 = false;
    }
    vec2_interpolate(time, &mut cd.ecb.top, &cd.desired_ecb.top);
    vec2_interpolate(time, &mut cd.ecb.bottom, &cd.desired_ecb.bottom);
    vec2_interpolate(time, &mut cd.ecb.left, &cd.desired_ecb.left);
    vec2_interpolate(time, &mut cd.ecb.right, &cd.desired_ecb.right);
    let e = &cd.ecb;
    assert!(
        !(e.top.x.is_nan()
            || e.top.y.is_nan()
            || e.bottom.x.is_nan()
            || e.bottom.y.is_nan()
            || e.left.x.is_nan()
            || e.left.y.is_nan()
            || e.right.x.is_nan()
            || e.right.y.is_nan()),
        "mpcoll.c:1193: ECB is NaN"
    );
}

/// `mpColl_80043670` (retail `0x80043670`): zero the ECB on the next load.
pub fn mark_ecb_clear(cd: &mut CollData) {
    cd.x130_flags |= coll_data_x130::CLEAR;
}

/// `mpColl_80043680` (retail `0x80043680`): teleport, resetting history.
pub fn set_position(cd: &mut CollData, pos: &Vec3) {
    cd.cur_pos = *pos;
    cd.prev_pos = cd.cur_pos;
    cd.last_pos = cd.prev_pos;
    cd.x130_flags |= coll_data_x130::CLEAR;
}

/// `mpCollSetFacingDir` (retail `0x800436D8`).
pub fn set_facing_dir(cd: &mut CollData, facing_dir: i32) {
    cd.facing_dir = facing_dir as i16;
}

/// `mpColl_800436E4` (retail `0x800436E4`, `mpcoll.c:900`): set a fixed
/// ECB's rotation, wrapped once into `[-tau, tau]`. Halts on a JObj source
/// ("not support rotate at JObj type coll").
pub fn set_ecb_angle(cd: &mut CollData, angle: f32) {
    // The decomp's `M_TAU` is the double literal 6.283185307179586, which
    // rounds to the same f64 as the std constant (asserted in tests).
    const M_TAU: f64 = core::f64::consts::TAU;
    let mut var_f1 = angle;
    match &mut cd.ecb_source.params {
        EcbSourceParams::Fixed { angle, .. }
            if cd.ecb_source.kind == i32::from(EcbSourceKind::Fixed) =>
        {
            if f64::from(var_f1) > M_TAU {
                var_f1 = (f64::from(var_f1) - M_TAU) as f32;
            } else if f64::from(var_f1) < -M_TAU {
                var_f1 = (f64::from(var_f1) + M_TAU) as f32;
            }
            *angle = var_f1;
        }
        _ => panic!("mpcoll.c:913: not support rotate at JObj type coll"),
    }
}

/// `mpCollSqueezeHorizontal` (retail `0x8004C864`, `mpcoll.c:4400`): the ECB
/// was pushed by walls on both sides; shrink it to fit between `left` and
/// `right` (the x positions after each push) and centre it.
pub fn squeeze_horizontal(cd: &mut CollData, _airborne: bool, left: f32, right: f32) {
    let half_width = 0.5 * (right - left + cd.ecb.right.x - cd.ecb.left.x);
    if !cd.x34_flags.b6 {
        cd.x64_ecb = cd.ecb;
    }
    cd.x34_flags.b6 = true;
    cd.cur_pos.x = (right + cd.ecb.right.x) - half_width;
    cd.ecb.right.x = half_width;
    cd.ecb.left.x = -half_width;
    cd.desired_ecb.right.x = cd.ecb.right.x;
    cd.desired_ecb.left.x = cd.ecb.left.x;
    cd.x34_flags.b5 = false;
}

/// `mpCollSqueezeVertical` (retail `0x8004C91C`, `mpcoll.c:4417`): the ECB
/// was pushed by a ceiling above and a floor below; shrink it to fit.
pub fn squeeze_vertical(cd: &mut CollData, airborne: bool, top: f32, bottom: f32) {
    let height = top - bottom + cd.ecb.top.y - cd.ecb.bottom.y;
    if !cd.x34_flags.b6 {
        cd.x64_ecb = cd.ecb;
    }
    cd.x34_flags.b6 = true;

    if height < 3.0 {
        let old_height = cd.ecb.top.y - cd.ecb.bottom.y;
        let new_height = cd.ecb.top.y + top - bottom;
        // MIN(a, b): (a < b) ? a : b
        cd.ecb.top.y = if old_height < new_height {
            old_height
        } else {
            new_height
        };
        cd.ecb.bottom.y = 0.0;
        cd.cur_pos.y = bottom;
    } else if !airborne {
        cd.cur_pos.y = bottom;
        cd.ecb.top.y = height + cd.ecb.bottom.y;
    } else {
        cd.cur_pos.y = 0.5 * (top + bottom);
        cd.ecb.top.y = 0.5 * (cd.ecb.top.y + cd.ecb.bottom.y + height);
        cd.ecb.bottom.y = cd.ecb.top.y - height;
    }
    let mid_y = 0.5 * (cd.ecb.top.y + cd.ecb.bottom.y);
    cd.ecb.right.y = mid_y;
    cd.ecb.left.y = mid_y;
    cd.desired_ecb.top.y = cd.ecb.top.y;
    cd.desired_ecb.bottom.y = cd.ecb.bottom.y;
    cd.desired_ecb.left.y = cd.ecb.left.y;
    cd.desired_ecb.right.y = cd.ecb.right.y;
    cd.x34_flags.b5 = false;
}

/// `mpUpdateFloorSkip` (retail `0x8004CBE8`).
pub fn update_floor_skip(cd: &mut CollData) {
    cd.floor_skip = cd.floor.index;
}

/// `mpClearFloorSkip` (retail `0x8004CBF4`).
pub fn clear_floor_skip(cd: &mut CollData) {
    cd.floor_skip = NO_ID;
}

/// `mpCopyCollData` (retail `0x8004CC00`, `mpcoll.c:4517`): copy the
/// collision state field by field (not `x0_gobj`, `x28_vec`, `x35_flags`,
/// `x34_flags.b7`, `joint_id_only`, `x50`, the ledge snap, or the ECB
/// source). `arg2 == 1` is a no-op here: the ECBs it copies are copied again
/// unconditionally below.
pub fn copy_coll_data(src: &CollData, dst: &mut CollData, arg2: i32) {
    if arg2 == 1 {
        dst.x64_ecb = src.x64_ecb;
        dst.desired_ecb = src.desired_ecb;
        dst.ecb = src.ecb;
        dst.prev_ecb = src.prev_ecb;
        dst.xe4_ecb = src.xe4_ecb;
    }
    dst.cur_pos = src.cur_pos;
    dst.prev_pos = src.prev_pos;
    dst.last_pos = src.last_pos;

    dst.x34_flags.b0 = src.x34_flags.b0;
    dst.x34_flags.b1234 = src.x34_flags.b1234;
    dst.x34_flags.b5 = src.x34_flags.b5;
    dst.x34_flags.b6 = src.x34_flags.b6;

    dst.facing_dir = src.facing_dir;
    dst.x38 = src.x38;
    dst.floor_skip = src.floor_skip;
    dst.ledge_id_left = src.ledge_id_left;
    dst.ledge_id_right = src.ledge_id_right;
    dst.joint_id_skip = src.joint_id_skip;
    dst.lstick_x = src.lstick_x;

    dst.x64_ecb = src.x64_ecb;
    dst.desired_ecb = src.desired_ecb;
    dst.ecb = src.ecb;
    dst.prev_ecb = src.prev_ecb;
    dst.xe4_ecb = src.xe4_ecb;

    dst.x130_flags = src.x130_flags;
    dst.env_flags = src.env_flags;
    dst.prev_env_flags = src.prev_env_flags;
    dst.x13c = src.x13c;
    dst.contact = src.contact;

    dst.floor = src.floor;
    dst.left_facing_wall = src.left_facing_wall;
    dst.right_facing_wall = src.right_facing_wall;
    dst.ceiling = src.ceiling;
}

// ---------------------------------------------------------------------------
// Helpers over CollData used by the map methods
// ---------------------------------------------------------------------------

/// `cur_pos + ecb.<point>` as `(x, y)`.
#[inline]
fn at(pos: &Vec3, p: &Vec2) -> (f32, f32) {
    (pos.x + p.x, pos.y + p.y)
}

/// Store a probe result into a `SurfaceData`'s flags and normal (the C's
/// `flags_out`/`normal_out` pointing into the surface).
#[inline]
fn store_surface(sd: &mut SurfaceData, flags: u32, normal: Vec3) {
    sd.flags = flags;
    sd.normal = normal;
}

impl CollMap {
    /// `coll->x38 != mpColl_804D64AC`: a joint transform ran since this
    /// `CollData` last stepped, so the `*Remap` sweeps are used.
    #[inline]
    fn moved_since(&self, cd: &CollData) -> bool {
        cd.x38 != self.coll_804d64ac
    }

    // -----------------------------------------------------------------------
    // Setup / teardown
    // -----------------------------------------------------------------------

    /// `mpCollCheckBounding` (retail `0x80041DD0`, `mpcoll.c:131`): cull
    /// joints outside the box swept by the ECB this step (widened by the
    /// ledge snap box when `CAN_GRAB_LEDGE`).
    pub fn coll_check_bounding(&mut self, cd: &CollData, flags: u32) {
        let mut left = cd.ecb.left.x + cd.cur_pos.x;
        let v = cd.prev_ecb.left.x + cd.prev_pos.x;
        if left > v {
            left = v;
        }
        let mut right = cd.ecb.right.x + cd.cur_pos.x;
        let v = cd.prev_ecb.right.x + cd.prev_pos.x;
        if right < v {
            right = v;
        }
        let mut bottom = cd.ecb.bottom.y + cd.cur_pos.y;
        let v = cd.prev_ecb.bottom.y + cd.prev_pos.y;
        if bottom > v {
            bottom = v;
        }
        let mut top = cd.ecb.top.y + cd.cur_pos.y;
        let v = cd.prev_ecb.top.y + cd.prev_pos.y;
        if top < v {
            top = v;
        }

        if flags & air_flags::CAN_GRAB_LEDGE != 0 {
            let ledge_snap_x = cd.ledge_snap_x;
            let half_height = 0.5 * cd.ledge_snap_height;
            right += ledge_snap_x;
            left -= ledge_snap_x;
            let v = cd.ledge_snap_y - half_height + cd.cur_pos.y;
            if bottom > v {
                bottom = v;
            }
            let v = cd.ledge_snap_y - half_height + cd.prev_pos.y;
            if bottom > v {
                bottom = v;
            }
            let offset = cd.ledge_snap_y + half_height;
            let v = cd.cur_pos.y + offset;
            if top < v {
                top = v;
            }
            let v = cd.prev_pos.y + offset;
            if top < v {
                top = v;
            }
        }
        self.bounding_check(left, bottom, right, top);
    }

    /// `mpColl_80041EE4` (retail `0x80041EE4`, `mpcoll.c:168`), "CollDataInit":
    /// reset every field but `cur_pos`.
    pub fn coll_data_init(&self, cd: &mut CollData) {
        cd.x34_flags.b0 = true;
        cd.x34_flags.b6 = false;
        cd.x34_flags.b7 = false;
        cd.x35_flags.b0 = true;
        cd.x34_flags.b1234 = 0;
        cd.env_flags = 0;
        cd.x130_flags = 0;
        cd.prev_pos = cd.cur_pos;
        cd.last_pos = cd.cur_pos;
        cd.x28_vec = cd.cur_pos;
        cd.floor_skip = NO_ID;
        cd.ledge_id_right = NO_ID;
        cd.ledge_id_left = NO_ID;
        cd.floor = SurfaceData {
            index: NO_ID,
            flags: 0,
            normal: Vec3::new(0.0, 1.0, 0.0),
        };
        cd.ceiling = SurfaceData {
            index: NO_ID,
            flags: 0,
            normal: Vec3::new(0.0, -1.0, 0.0),
        };
        cd.right_facing_wall = SurfaceData {
            index: NO_ID,
            flags: 0,
            normal: Vec3::new(0.0, 1.0, 0.0),
        };
        cd.left_facing_wall = SurfaceData {
            index: NO_ID,
            flags: 0,
            normal: Vec3::new(0.0, -1.0, 0.0),
        };
        cd.x38 = self.coll_804d64ac;
        cd.x50 = 0.0;
        cd.joint_id_skip = NO_ID;
        cd.joint_id_only = NO_ID;
        cd.ledge_snap_x = 0.0;
        cd.ledge_snap_y = 0.0;
        cd.ledge_snap_height = 0.0;
        cd.ecb = FtEcb::default();
        cd.prev_ecb = FtEcb::default();
        cd.xe4_ecb = FtEcb::default();
        cd.ecb_source = Default::default();
        cd.desired_ecb = FtEcb::default();
        cd.x64_ecb = FtEcb::default();
    }

    /// `mpColl_80043268` (retail `0x80043268`, `mpcoll.c:746`): fire the
    /// floor's joint `cb_0` with `ground_kind` 1 (`arg2`) or 2.
    pub fn notify_floor_joint(&mut self, cd: &mut CollData, line_id: i32, arg2: bool, dy: f32) {
        let joint_id = self.joint_from_line(line_id);
        if joint_id != NO_ID {
            let (cb, user_data) = self.joint_get_cb1(joint_id);
            if let Some(cb) = cb {
                let thing = if !arg2 { 2 } else { 1 };
                cb(user_data, joint_id, cd, cd.x50 as i32, thing, dy);
            }
        }
    }

    /// `mpCollEnd_inline2` (`mpcoll.c:767`): fire the ceiling's joint `cb_1`
    /// with `ground_kind` 0.
    fn notify_ceiling_joint(&mut self, cd: &mut CollData, line_id: i32, dy: f32) {
        let joint_id = self.joint_from_line(line_id);
        if joint_id != NO_ID {
            let (cb, user_data) = self.joint_get_cb2(joint_id);
            if let Some(cb) = cb {
                cb(user_data, joint_id, cd, cd.x50 as i32, 0, dy);
            }
        }
    }

    /// `mpCollEnd` (retail `0x80043324`, `mpcoll.c:793`): finish a collision
    /// pass. Applies the stage's dynamic material to the floor flags, then
    /// fires the floor joint callback when standing or on an edge and the
    /// ceiling callback when touching a ceiling.
    pub fn coll_end(&mut self, cd: &mut CollData, arg1: bool, arg2: bool) {
        if cd.floor.index != NO_ID {
            let temp = match self.dynamic_attr_hook {
                Some(hook) => hook(&cd.cur_pos, cd.floor.index),
                None => 0,
            };
            if temp != 0 {
                cd.floor.flags = (cd.floor.flags & !0xFF) | (temp as u32 & 0xFF);
            }
        }
        let env = cd.env_flags as u32;
        if arg1
            || env & collide::EDGE != 0
            || env & collide::LEFT_EDGE != 0
            || env & collide::RIGHT_EDGE != 0
        {
            let dy = cd.cur_pos.y - cd.last_pos.y;
            let floor = cd.floor.index;
            self.notify_floor_joint(cd, floor, arg2, dy);
        }
        if env & (collide::CEILING_HUG | collide::CEILING_PUSH) != 0 {
            let dy = cd.cur_pos.y - cd.last_pos.y;
            let ceiling = cd.ceiling.index;
            self.notify_ceiling_joint(cd, ceiling, dy);
        }
    }

    /// `mpColl_80043558` (retail `0x80043558`, `mpcoll.c:842`): fire the
    /// joint callback of a floor (`cb_0`, kind 2) or ceiling (`cb_1`, kind 0)
    /// line with no motion.
    pub fn notify_line_joint(&mut self, cd: &mut CollData, line_id: i32) {
        let kind = self.line_get_kind(line_id);
        if kind == line_kind::FLOOR {
            let joint_id = self.joint_from_line(line_id);
            if joint_id != NO_ID {
                let (cb, user_data) = self.joint_get_cb1(joint_id);
                if let Some(cb) = cb {
                    cb(user_data, joint_id, cd, cd.x50 as i32, 2, 0.0);
                }
            }
        } else if kind == line_kind::CEILING {
            let joint_id = self.joint_from_line(line_id);
            if joint_id != NO_ID {
                let (cb, user_data) = self.joint_get_cb2(joint_id);
                if let Some(cb) = cb {
                    cb(user_data, joint_id, cd, cd.x50 as i32, 0, 0.0);
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // The step loop (mpColl_80043754)
    // -----------------------------------------------------------------------

    /// `mpColl_80043754` (retail `0x80043754`, `mpcoll.c:926`): split the
    /// move from `last_pos` to `cur_pos` (and the ECB change) into steps of
    /// at most 6 units and run the collision routine on each until one
    /// reports a landing (`x34_flags.b5`).
    fn collide_steps(
        &mut self,
        mode: CollideMode,
        cd: &mut CollData,
        flags: u32,
        mut floor_cb: LineFilter<'_, '_>,
    ) -> bool {
        let mut vel = Vec3::new(
            cd.cur_pos.x - cd.last_pos.x,
            cd.cur_pos.y - cd.last_pos.y,
            cd.cur_pos.z - cd.last_pos.z,
        );
        let mut x = c_abs(vel.x);
        let mut y = c_abs(vel.y);

        let mut dist_left_x = cd.desired_ecb.left.x - cd.ecb.left.x;
        dist_left_x = c_abs(dist_left_x);
        let mut dist_right_x = cd.desired_ecb.right.x - cd.ecb.right.x;
        if dist_right_x < 0.0 {
            dist_right_x = -dist_right_x;
        }
        if dist_left_x < dist_right_x {
            dist_left_x = dist_right_x;
        }

        let mut dist_top_y = cd.desired_ecb.top.y - cd.ecb.top.y;
        dist_top_y = c_abs(dist_top_y);
        let mut dist_right_y = cd.desired_ecb.right.y - cd.ecb.right.y;
        if dist_right_y < 0.0 {
            dist_right_y = -dist_right_y;
        }
        if dist_top_y < dist_right_y {
            dist_top_y = dist_right_y;
        }

        // max_inline(a, b): (a > b) ? a : b
        x = if x > dist_left_x { x } else { dist_left_x };
        y = if y > dist_top_y { y } else { dist_top_y };
        x = if x > y { x } else { y };

        let steps: i32;
        if x > 6.0 {
            steps = (x / 6.0) as i32 + 1;
            let s = steps as f32;
            vel.x /= s;
            vel.y /= s;
            vel.z /= s;
        } else {
            steps = 1;
        }
        let mut step = 0;
        cd.cur_pos = cd.last_pos;
        cd.x34_flags.b5 = false;
        let mut ret = false;
        while step < steps && !cd.x34_flags.b5 {
            interpolate_ecb(cd, 1.0 / (steps - step) as f32);
            cd.prev_pos = cd.cur_pos;
            cd.cur_pos.x += vel.x;
            cd.cur_pos.y += vel.y;
            cd.cur_pos.z += vel.z;
            self.coll_check_bounding(cd, flags);
            ret = match mode {
                CollideMode::Air => self.air_collide(cd, flags, floor_cb.as_deref_mut()),
                CollideMode::Point => self.point_collide(cd),
                CollideMode::Ground => self.ground_collide(cd, flags),
                CollideMode::Ceiling => self.ceiling_collide(cd, flags),
            };
            self.uncheck_bounding();
            step += 1;
            cd.x38 = self.coll_804d64ac;
        }
        ret
    }

    /// The `inline0..inline4` prologue/epilogue (`mpcoll.c:2651-2738`):
    /// roll `env_flags`, classify the ECB as tiny, run the step loop, and
    /// call `mpCollEnd`.
    fn run(
        &mut self,
        mode: CollideMode,
        cd: &mut CollData,
        flags: u32,
        end_arg2: bool,
        floor_cb: LineFilter<'_, '_>,
    ) -> bool {
        cd.prev_env_flags = cd.env_flags;
        cd.env_flags = 0;
        self.coll.is_ecb_tiny =
            cd.ecb.top.y - cd.ecb.bottom.y < 6.0 && cd.ecb.right.y - cd.ecb.left.y < 6.0;
        let result = self.collide_steps(mode, cd, flags, floor_cb);
        self.coll_end(cd, result, end_arg2);
        result
    }

    // -----------------------------------------------------------------------
    // Wall bookkeeping (mpColl_RightWall_inline, mpColl_LeftWall_inline)
    // -----------------------------------------------------------------------

    /// `mpColl_RightWall_inline` / `mpColl_LeftWall_inline` (`mpcoll.c:705`,
    /// `719`): record a wall hit unless it is connected to one already
    /// recorded. Asserts on more than `MPCOLL_WALLID_MAX` walls.
    fn push_wall(&mut self, side: Side, line_id: i32) {
        let count = match side {
            Side::Right => self.coll.right_count,
            Side::Left => self.coll.left_count,
        } as usize;
        for i in 0..count {
            let start_id = match side {
                Side::Right => self.coll.right[i],
                Side::Left => self.coll.left[i],
            };
            if line_id == start_id || self.lines_connected(start_id, line_id) {
                return;
            }
        }
        assert!(count < 9, "mpcoll.c: more than MPCOLL_WALLID_MAX walls");
        match side {
            Side::Right => {
                self.coll.right[count] = line_id;
                self.coll.right_count += 1;
            }
            Side::Left => {
                self.coll.left[count] = line_id;
                self.coll.left_count += 1;
            }
        }
    }

    /// `mpColl_RightWall_inline2` / `mpColl_LeftWall_inline2`
    /// (`mpcoll.c:1658`, `1977`): the wall sweep, remapped if a joint moved.
    fn wall_sweep(
        &mut self,
        cd: &CollData,
        side: Side,
        ax: f32,
        ay: f32,
        bx: f32,
        by: f32,
    ) -> Option<i32> {
        let (js, jo) = (cd.joint_id_skip, cd.joint_id_only);
        let hit = match (side, self.moved_since(cd)) {
            (Side::Right, true) => self.check_right_wall_remap(ax, ay, bx, by, js, jo),
            (Side::Right, false) => self.check_right_wall(ax, ay, bx, by, js, jo),
            (Side::Left, true) => self.check_left_wall_remap(ax, ay, bx, by, js, jo),
            (Side::Left, false) => self.check_left_wall(ax, ay, bx, by, js, jo),
        };
        hit.map(|h| h.line_id)
    }

    /// `mpCheckRightWall` / `mpCheckLeftWall` with only the line id wanted.
    fn wall_static(
        &mut self,
        cd: &CollData,
        side: Side,
        ax: f32,
        ay: f32,
        bx: f32,
        by: f32,
    ) -> Option<i32> {
        let (js, jo) = (cd.joint_id_skip, cd.joint_id_only);
        let hit = match side {
            Side::Right => self.check_right_wall(ax, ay, bx, by, js, jo),
            Side::Left => self.check_left_wall(ax, ay, bx, by, js, jo),
        };
        hit.map(|h| h.line_id)
    }

    /// `mpLib_800511A4_RightWall` / `mpLib_800515A0_LeftWall` by side.
    #[allow(clippy::too_many_arguments)]
    fn wall_vertex(
        &mut self,
        cd: &CollData,
        side: Side,
        a0x: f32,
        a0y: f32,
        a1x: f32,
        a1y: f32,
        b0x: f32,
        b0y: f32,
        b1x: f32,
        b1y: f32,
    ) -> Option<i32> {
        let (js, jo) = (cd.joint_id_skip, cd.joint_id_only);
        match side {
            Side::Right => {
                self.right_wall_vertex_sweep(a0x, a0y, a1x, a1y, b0x, b0y, b1x, b1y, js, jo)
            }
            Side::Left => {
                self.left_wall_vertex_sweep(a0x, a0y, a1x, a1y, b0x, b0y, b1x, b1y, js, jo)
            }
        }
    }

    // -----------------------------------------------------------------------
    // Ceiling / floor corner handling (mpColl_800439FC .. mpColl_80043F40)
    // -----------------------------------------------------------------------

    /// `mpColl_800439FC` (retail `0x800439FC`, `mpcoll.c:1002`): against a
    /// ceiling, a left wall touching the ECB's right point pushes the body
    /// left and re-snaps to the ceiling.
    pub fn ceiling_left_wall_multi_collide(&mut self, cd: &mut CollData) {
        let (right_x, right_y) = at(&cd.cur_pos, &cd.ecb.right);
        let right_dx = c_abs(cd.ecb.right.x);
        // recalculate ceiling direction from its normal
        // retail 0x80043A4C/50: fmadds, fnmsubs.
        let f1 = fmadds(cd.ceiling.normal.y, right_dx, right_x);
        let f2 = fnmsubs(cd.ceiling.normal.x, right_dx, right_y);
        let Some(hit) =
            self.check_left_wall(f1, f2, right_x, right_y, cd.joint_id_skip, cd.joint_id_only)
        else {
            return;
        };
        cd.contact = hit.pos;
        let sp10 = Vec3::new(cd.contact.x - right_dx, cd.cur_pos.y + cd.ecb.top.y, 0.0);
        let Some(p) = self.ceiling_probe(cd.ceiling.index, &sp10) else {
            return;
        };
        store_surface(&mut cd.ceiling, p.flags, p.normal);
        cd.cur_pos.y += p.delta;
        cd.cur_pos.x = sp10.x;
    }

    /// `mpColl_80043ADC` (retail `0x80043ADC`, `mpcoll.c:1038`): mirror of
    /// [`Self::ceiling_left_wall_multi_collide`] for a right wall.
    pub fn ceiling_right_wall_multi_collide(&mut self, cd: &mut CollData) {
        let (left_x, left_y) = at(&cd.cur_pos, &cd.ecb.left);
        let left_dx = c_abs(cd.ecb.left.x);
        // retail 0x80043B2C/30: fnmsubs, fmadds.
        let f1 = fnmsubs(cd.ceiling.normal.y, left_dx, left_x);
        let f2 = fmadds(cd.ceiling.normal.x, left_dx, left_y);
        let Some(hit) =
            self.check_right_wall(f1, f2, left_x, left_y, cd.joint_id_skip, cd.joint_id_only)
        else {
            return;
        };
        cd.contact = hit.pos;
        let sp10 = Vec3::new(cd.contact.x + left_dx, cd.cur_pos.y + cd.ecb.top.y, 0.0);
        let Some(p) = self.ceiling_probe(cd.ceiling.index, &sp10) else {
            return;
        };
        store_surface(&mut cd.ceiling, p.flags, p.normal);
        cd.cur_pos.y += p.delta;
        cd.cur_pos.x = sp10.x;
    }

    /// `mpColl_80043BBC` (retail `0x80043BBC`, `mpcoll.c:1078`): standing on
    /// a floor, is a left wall (other than the one the floor chain connects
    /// to) between the ECB bottom and right points?
    pub fn floor_connected_left_wall(&mut self, cd: &CollData) -> Option<i32> {
        let line_id = self.line_prev_non_floor(cd.floor.index);
        let (bottom_x, bottom_y) = at(&cd.cur_pos, &cd.ecb.bottom);
        let (right_x, right_y) = at(&cd.cur_pos, &cd.ecb.right);
        match self.wall_static(cd, Side::Left, bottom_x, bottom_y, right_x, right_y) {
            Some(wall_id) if wall_id != line_id => Some(wall_id),
            _ => None,
        }
    }

    /// `mpColl_80043C6C` (retail `0x80043C6C`, `mpcoll.c:1102`): push out of
    /// a left wall met while on the floor and re-snap to the floor.
    pub fn floor_left_wall_multi_collide(
        &mut self,
        cd: &mut CollData,
        line_id: i32,
        ignore_bottom: bool,
    ) {
        let right_dx = c_abs(cd.ecb.right.x);
        let mut pos = Vec3::new(
            cd.cur_pos.x + cd.ecb.right.x,
            cd.cur_pos.y + cd.ecb.right.y,
            0.0,
        );
        if self.left_wall_probe(line_id, &pos).is_some() {
            // recalculate floor direction from its normal
            // retail 0x80043D04/10: fnmsubs, fmadds.
            let floor_x = fnmsubs(cd.floor.normal.y, right_dx, pos.x);
            let floor_y = fmadds(cd.floor.normal.x, right_dx, pos.y);
            if let Some(hit) = self.check_left_wall(
                floor_x,
                floor_y,
                pos.x,
                pos.y,
                cd.joint_id_skip,
                cd.joint_id_only,
            ) {
                cd.contact = hit.pos;
                pos.x = cd.contact.x - right_dx;
                pos.y = if ignore_bottom {
                    cd.cur_pos.y
                } else {
                    cd.cur_pos.y + cd.ecb.bottom.y
                };
                if let Some(p) = self.floor_probe(cd.floor.index, &pos) {
                    store_surface(&mut cd.floor, p.flags, p.normal);
                    cd.cur_pos.y += p.delta;
                    cd.cur_pos.x = pos.x;
                }
            }
        } else {
            pos = self.left_wall_get_top(line_id);
            let f1 = pos.x - 2.0;
            let f2 = pos.y;
            // retail 0x80043DC4/DC: fnmsubs for both corner-extension coordinates.
            pos.x = fnmsubs(2.0, right_dx, f1);
            pos.y = fnmsubs(2.0, cd.ecb.right.y - cd.ecb.bottom.y, f2);
            if let Some(hit) = self.check_floor(
                f1,
                f2,
                pos.x,
                pos.y,
                0.0,
                cd.floor_skip,
                cd.joint_id_skip,
                cd.joint_id_only,
                None,
            ) {
                cd.contact = hit.pos;
                pos.x = cd.contact.x;
                pos.y = if ignore_bottom {
                    cd.cur_pos.y
                } else {
                    cd.cur_pos.y + cd.ecb.bottom.y
                };
                if let Some(p) = self.floor_probe(cd.floor.index, &pos) {
                    store_surface(&mut cd.floor, p.flags, p.normal);
                    cd.cur_pos.y += p.delta;
                    cd.cur_pos.x = pos.x;
                }
            }
        }
    }

    /// `mpColl_80043E90` (retail `0x80043E90`, `mpcoll.c:1167`): mirror of
    /// [`Self::floor_connected_left_wall`] for right walls.
    pub fn floor_connected_right_wall(&mut self, cd: &CollData) -> Option<i32> {
        let line_id = self.line_next_non_floor(cd.floor.index);
        let (bottom_x, bottom_y) = at(&cd.cur_pos, &cd.ecb.bottom);
        let (left_x, left_y) = at(&cd.cur_pos, &cd.ecb.left);
        match self.wall_static(cd, Side::Right, bottom_x, bottom_y, left_x, left_y) {
            Some(wall_id) if wall_id != line_id => Some(wall_id),
            _ => None,
        }
    }

    /// `mpColl_80043F40` (retail `0x80043F40`, `mpcoll.c:1191`): mirror of
    /// [`Self::floor_left_wall_multi_collide`].
    pub fn floor_right_wall_multi_collide(
        &mut self,
        cd: &mut CollData,
        line_id: i32,
        ignore_bottom: bool,
    ) {
        let left_dx = c_abs(cd.ecb.left.x);
        let mut pos = Vec3::new(
            cd.cur_pos.x + cd.ecb.left.x,
            cd.cur_pos.y + cd.ecb.left.y,
            0.0,
        );
        if self.right_wall_probe(line_id, &pos).is_some() {
            // retail 0x80043FD8/E4: fmadds, fnmsubs.
            let floor_x = fmadds(cd.floor.normal.y, left_dx, pos.x);
            let floor_y = fnmsubs(cd.floor.normal.x, left_dx, pos.y);
            if let Some(hit) = self.check_right_wall(
                floor_x,
                floor_y,
                pos.x,
                pos.y,
                cd.joint_id_skip,
                cd.joint_id_only,
            ) {
                cd.contact = hit.pos;
                pos.x = cd.contact.x + left_dx;
                pos.y = if ignore_bottom {
                    cd.cur_pos.y
                } else {
                    cd.cur_pos.y + cd.ecb.bottom.y
                };
                if let Some(p) = self.floor_probe(cd.floor.index, &pos) {
                    store_surface(&mut cd.floor, p.flags, p.normal);
                    cd.cur_pos.y += p.delta;
                    cd.cur_pos.x = pos.x;
                }
            }
        } else {
            pos = self.right_wall_get_top(line_id);
            let f1 = 2.0 + pos.x;
            let f2 = pos.y;
            // 2.0 * (ecb bottom -> ecb left).normal() + ecb left
            // retail 0x80044098/B0: fmadds, fnmsubs.
            pos.x = fmadds(2.0, left_dx, f1);
            pos.y = fnmsubs(2.0, cd.ecb.left.y - cd.ecb.bottom.y, f2);
            if let Some(hit) = self.check_floor(
                f1,
                f2,
                pos.x,
                pos.y,
                0.0,
                cd.floor_skip,
                cd.joint_id_skip,
                cd.joint_id_only,
                None,
            ) {
                cd.contact = hit.pos;
                pos.x = cd.contact.x;
                pos.y = if ignore_bottom {
                    cd.cur_pos.y
                } else {
                    cd.cur_pos.y + cd.ecb.bottom.y
                };
                if let Some(p) = self.floor_probe(cd.floor.index, &pos) {
                    store_surface(&mut cd.floor, p.flags, p.normal);
                    cd.cur_pos.x = pos.x;
                    cd.cur_pos.y += p.delta;
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Ledge grabs (mpColl_80044164, mpColl_800443C4)
    // -----------------------------------------------------------------------

    /// Shared body of `mpColl_80044164` (left ledge, `dir = 1`,
    /// `checks = 6`) and `mpColl_800443C4` (right ledge, `dir = -1`,
    /// `checks = 10`) (`mpcoll.c:1253-1394`). Returns the grabbed ledge's
    /// line id; `cd.contact` is set to the ledge point when one is found.
    fn check_for_ledge(&mut self, cd: &mut CollData, side: Side) -> Option<i32> {
        let half_height = 0.5 * cd.ledge_snap_height;
        let mut snap_x = cd.ledge_snap_x;
        let snap_y = cd.ledge_snap_y;
        let (left, right) = match side {
            Side::Left => {
                if cd.prev_pos.x < cd.cur_pos.x {
                    (cd.prev_pos.x, snap_x + (cd.cur_pos.x + cd.ecb.right.x))
                } else {
                    (cd.cur_pos.x, snap_x + (cd.prev_pos.x + cd.ecb.right.x))
                }
            }
            Side::Right => {
                snap_x = -snap_x;
                if cd.prev_pos.x > cd.cur_pos.x {
                    (snap_x + (cd.cur_pos.x + cd.ecb.left.x), cd.prev_pos.x)
                } else {
                    (snap_x + (cd.prev_pos.x + cd.ecb.left.x), cd.cur_pos.x)
                }
            }
        };
        let (bottom, top) = if cd.prev_pos.y < cd.cur_pos.y {
            (
                (cd.prev_pos.y + snap_y) - half_height,
                half_height + (cd.cur_pos.y + snap_y),
            )
        } else {
            (
                (cd.cur_pos.y + snap_y) - half_height,
                half_height + (cd.prev_pos.y + snap_y),
            )
        };

        let already_checked = self.checked_bounding();
        if !already_checked {
            self.bounding_check(left, bottom, right, top);
        }
        let (dir, checks) = match side {
            Side::Left => (1, 6),
            Side::Right => (-1, 10),
        };
        let ledge = self.find_ledge(
            cd.floor_skip,
            cd.joint_id_skip,
            cd.joint_id_only,
            dir,
            left,
            bottom,
            right,
            top,
        );
        let mut grabbed: Option<i32> = None;
        if let Some(l) = ledge {
            cd.contact = l.pos;
            let ledge_id = l.line_id;
            let (bx, by) = at(&cd.cur_pos, &cd.ecb.bottom);
            let (tx, ty) = at(&cd.cur_pos, &cd.ecb.top);
            let near_edge = match side {
                Side::Left => {
                    let edge = self.floor_get_left(ledge_id);
                    cd.contact.x - edge.x < 5.0 && bx < edge.x && by < edge.y
                }
                Side::Right => {
                    let edge = self.floor_get_right(ledge_id);
                    edge.x - cd.contact.x < 5.0 && bx > edge.x && by < edge.y
                }
            };
            if near_edge {
                let clear = by > cd.contact.y || {
                    let (js, jo) = (cd.joint_id_skip, cd.joint_id_only);
                    let ledge_joint = self.joint_from_line(ledge_id);
                    let first = match self.check_multiple(
                        tx,
                        ty,
                        cd.contact.x,
                        cd.contact.y,
                        checks,
                        js,
                        jo,
                    ) {
                        None => true,
                        Some(h) => ledge_joint == self.joint_from_line(h.line_id),
                    };
                    first && {
                        match self.check_multiple(
                            bx,
                            -2.0 + by,
                            cd.contact.x,
                            cd.contact.y,
                            checks,
                            js,
                            jo,
                        ) {
                            None => true,
                            Some(h) => ledge_joint == self.joint_from_line(h.line_id),
                        }
                    }
                };
                if clear {
                    grabbed = Some(ledge_id);
                }
            }
        }
        if !already_checked {
            self.uncheck_bounding();
        }
        grabbed
    }

    /// `mpColl_80044164` (retail `0x80044164`, `mpcoll.c:1253`): look for a
    /// ledge to the fighter's right whose left end is the grab point (the
    /// "left ledge" of the stage). Returns the ledge line id.
    pub fn check_for_left_ledge(&mut self, cd: &mut CollData) -> Option<i32> {
        self.check_for_ledge(cd, Side::Left)
    }

    /// `mpColl_800443C4` (retail `0x800443C4`, `mpcoll.c:1326`): mirror of
    /// [`Self::check_for_left_ledge`].
    pub fn check_for_right_ledge(&mut self, cd: &mut CollData) -> Option<i32> {
        self.check_for_ledge(cd, Side::Right)
    }

    // -----------------------------------------------------------------------
    // Floor (mpColl_80044628_Floor .. mpColl_80044948_Floor)
    // -----------------------------------------------------------------------

    /// `mpColl_80044628_Floor` (retail `0x80044628`, `mpcoll.c:1396`),
    /// "FloorCheckAir": sweep the ECB bottom against floors; failing that,
    /// try the floor the touched wall chain connects to. Sets
    /// `Collide_FloorPush` (and `FloorHug` for a direct hit).
    pub fn floor_check_air(
        &mut self,
        cd: &mut CollData,
        cb: LineFilter<'_, '_>,
        left_right: i32,
    ) -> bool {
        let (pbx, pby) = at(&cd.prev_pos, &cd.prev_ecb.bottom);
        let bottom = Vec3::new(
            cd.cur_pos.x + cd.ecb.bottom.x,
            cd.cur_pos.y + cd.ecb.bottom.y,
            0.0,
        );
        let mut cb = cb;
        let hit = if self.moved_since(cd) {
            self.check_floor_remap(
                pbx,
                pby,
                bottom.x,
                bottom.y,
                0.0,
                cd.floor_skip,
                cd.joint_id_skip,
                cd.joint_id_only,
                cb.as_deref_mut(),
            )
        } else {
            self.check_floor(
                pbx,
                pby,
                bottom.x,
                bottom.y,
                0.0,
                cd.floor_skip,
                cd.joint_id_skip,
                cd.joint_id_only,
                cb.as_deref_mut(),
            )
        };
        if let Some(h) = hit {
            cd.contact = h.pos;
            cd.floor.index = h.line_id;
            store_surface(&mut cd.floor, h.flags, h.normal);
            if cd.floor.flags & line_flag::PLATFORM == 0 || cd.floor.index != cd.floor_skip {
                cd.env_flags |= collide::FLOOR_PUSH as i32;
                cd.env_flags |= collide::FLOOR_HUG as i32;
                return true;
            }
        }

        let mut line_id = NO_ID;
        let found = (left_right & 1 != 0 && {
            line_id = self.line_prev_non_left_wall(cd.left_facing_wall.index);
            line_id != NO_ID
        }) || (left_right & 2 != 0 && {
            line_id = self.line_next_non_right_wall(cd.right_facing_wall.index);
            line_id != NO_ID
        });
        if found && self.line_is_active(line_id) && self.line_get_kind(line_id) == line_kind::FLOOR
        {
            if let Some(p) = self.floor_probe(line_id, &bottom) {
                store_surface(&mut cd.floor, p.flags, p.normal);
                if p.delta > 0.0 {
                    cd.floor.index = p.line_id;
                    if cd.floor.flags & line_flag::PLATFORM == 0 || cd.floor.index != cd.floor_skip
                    {
                        let pass = match cb {
                            None => true,
                            Some(f) => f(cd.floor.index),
                        };
                        if pass {
                            cd.env_flags |= collide::FLOOR_PUSH as i32;
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    /// `mpColl_80044838_Floor` (retail `0x80044838`, `mpcoll.c:1464`),
    /// "SnapToFloorNoEdgePass": snap onto the current floor chain, or onto
    /// its nearer end if the body overhangs. Always returns true.
    pub fn snap_to_floor(&mut self, cd: &mut CollData, ignore_bottom: bool) -> bool {
        let bottom = if ignore_bottom {
            Vec3::new(cd.cur_pos.x, cd.cur_pos.y, 0.0)
        } else {
            Vec3::new(
                cd.cur_pos.x + cd.ecb.bottom.x,
                cd.cur_pos.y + cd.ecb.bottom.y,
                0.0,
            )
        };
        if let Some(p) = self.floor_probe(cd.floor.index, &bottom) {
            store_surface(&mut cd.floor, p.flags, p.normal);
            cd.floor.index = p.line_id;
            cd.cur_pos.y += p.delta;
        } else {
            let mut edge = self.floor_get_left(cd.floor.index);
            if edge.x <= bottom.x {
                edge = self.floor_get_right(cd.floor.index);
            }
            cd.cur_pos.x = edge.x - cd.ecb.bottom.x;
            cd.cur_pos.y = edge.y - cd.ecb.bottom.y;
            if let Some(p) = self.floor_probe(cd.floor.index, &edge) {
                store_surface(&mut cd.floor, p.flags, p.normal);
                cd.floor.index = p.line_id;
            }
        }
        true
    }

    /// `mpColl_80044948_Floor` (retail `0x80044948`, `mpcoll.c:1502`): the
    /// `STAY_AIRBORNE` floor snap: snap to the floor if over it; at an end,
    /// stop only if a wall continues the chain there.
    pub fn snap_to_floor_stay_airborne(&mut self, cd: &mut CollData) -> bool {
        let mut hit_wall = false;
        let bottom = if cd.ecb.bottom.y <= 0.0 {
            Vec3::new(
                cd.cur_pos.x + cd.ecb.bottom.x,
                cd.cur_pos.y + cd.ecb.bottom.y,
                0.0,
            )
        } else {
            Vec3::new(cd.cur_pos.x, cd.cur_pos.y, 0.0)
        };
        if let Some(p) = self.floor_probe(cd.floor.index, &bottom) {
            store_surface(&mut cd.floor, p.flags, p.normal);
            cd.floor.index = p.line_id;
            hit_wall = true;
            cd.cur_pos.y += p.delta;
        } else {
            let mut edge = self.floor_get_left(cd.floor.index);
            if bottom.x < edge.x {
                let line_id = self.line_prev_non_floor(cd.floor.index);
                if line_id != NO_ID && self.line_get_kind(line_id) == line_kind::RIGHT_WALL {
                    hit_wall = true;
                }
            } else {
                edge = self.floor_get_right(cd.floor.index);
                let line_id = self.line_next_non_floor(cd.floor.index);
                if line_id != NO_ID && self.line_get_kind(line_id) == line_kind::LEFT_WALL {
                    hit_wall = true;
                }
            }
            cd.cur_pos.y = edge.y - cd.ecb.bottom.y;
            if hit_wall {
                cd.cur_pos.x = edge.x;
                if let Some(p) = self.floor_probe(cd.floor.index, &edge) {
                    store_surface(&mut cd.floor, p.flags, p.normal);
                    cd.floor.index = p.line_id;
                }
                // else: OSReport("Error:oioi... id=%d")
            }
        }
        hit_wall
    }

    // -----------------------------------------------------------------------
    // Ceiling (mpColl_80044AD8_Ceiling, mpColl_80044C74_Ceiling)
    // -----------------------------------------------------------------------

    /// `mpColl_80044AD8_Ceiling` (retail `0x80044AD8`, `mpcoll.c:1557`),
    /// "CeilingCheck": sweep the ECB top against ceilings; failing that, try
    /// the ceiling the touched wall chain connects to.
    pub fn ceiling_check(&mut self, cd: &mut CollData, left_right: i32) -> bool {
        let (ptx, pty) = at(&cd.prev_pos, &cd.prev_ecb.top);
        let top = Vec3::new(
            cd.cur_pos.x + cd.ecb.top.x,
            cd.cur_pos.y + cd.ecb.top.y,
            0.0,
        );
        let (js, jo) = (cd.joint_id_skip, cd.joint_id_only);
        let hit = if self.moved_since(cd) {
            self.check_ceiling_remap(ptx, pty, top.x, top.y, js, jo)
        } else {
            self.check_ceiling(ptx, pty, top.x, top.y, js, jo)
        };
        if let Some(h) = hit {
            cd.contact = h.pos;
            cd.ceiling.index = h.line_id;
            store_surface(&mut cd.ceiling, h.flags, h.normal);
            cd.env_flags |= collide::CEILING_PUSH as i32;
            cd.env_flags |= collide::CEILING_HUG as i32;
            return true;
        }

        let mut line_id = NO_ID;
        let found = (left_right & 1 != 0 && {
            line_id = self.line_next_non_left_wall(cd.left_facing_wall.index);
            line_id != NO_ID
        }) || (left_right & 2 != 0 && {
            line_id = self.line_prev_non_right_wall(cd.right_facing_wall.index);
            line_id != NO_ID
        });
        if found
            && self.line_is_active(line_id)
            && self.line_get_kind(line_id) == line_kind::CEILING
        {
            if let Some(p) = self.ceiling_probe(line_id, &top) {
                store_surface(&mut cd.ceiling, p.flags, p.normal);
                if p.delta < 0.0 {
                    cd.ceiling.index = p.line_id;
                    cd.env_flags |= collide::CEILING_PUSH as i32;
                    return true;
                }
            }
        }
        false
    }

    /// Shared body of `mpColl_80044C74_Ceiling` (`mpcoll.c:1605`, air,
    /// `check_active = true`) and `mpColl_8004AB80` (`mpcoll.c:3799`,
    /// ground, `check_active = false`): snap the ECB top to the ceiling
    /// chain, or to its nearer end when a wall continues there.
    fn snap_to_ceiling_impl(&mut self, cd: &mut CollData, check_active: bool) -> bool {
        let top = Vec3::new(
            cd.cur_pos.x + cd.ecb.top.x,
            cd.cur_pos.y + cd.ecb.top.y,
            0.0,
        );
        if let Some(p) = self.ceiling_probe(cd.ceiling.index, &top) {
            store_surface(&mut cd.ceiling, p.flags, p.normal);
            cd.ceiling.index = p.line_id;
            cd.cur_pos.y += p.delta;
        } else {
            let mut hit_wall = false;
            let mut ceiling_end = self.ceiling_get_left(cd.ceiling.index);
            if top.x <= ceiling_end.x {
                let line_id = self.line_next_non_ceiling(cd.ceiling.index);
                if line_id != NO_ID
                    && (!check_active || self.line_is_active(line_id))
                    && self.line_get_kind(line_id) == line_kind::RIGHT_WALL
                {
                    hit_wall = true;
                }
            } else {
                ceiling_end = self.ceiling_get_right(cd.ceiling.index);
                let line_id = self.line_prev_non_ceiling(cd.ceiling.index);
                if line_id != NO_ID
                    && (!check_active || self.line_is_active(line_id))
                    && self.line_get_kind(line_id) == line_kind::LEFT_WALL
                {
                    hit_wall = true;
                }
            }
            cd.cur_pos.y = ceiling_end.y - cd.ecb.top.y;
            if hit_wall {
                cd.cur_pos.x = ceiling_end.x;
                if let Some(p) = self.ceiling_probe(cd.ceiling.index, &ceiling_end) {
                    store_surface(&mut cd.ceiling, p.flags, p.normal);
                    cd.ceiling.index = p.line_id;
                }
                // else: OSReport("oioi...")
            }
        }
        true
    }

    /// `mpColl_80044C74_Ceiling` (retail `0x80044C74`), "CeilingCollideAir".
    pub fn snap_to_ceiling_air(&mut self, cd: &mut CollData) -> bool {
        self.snap_to_ceiling_impl(cd, true)
    }

    /// `mpColl_8004AB80` (retail `0x8004AB80`): the grounded ceiling snap;
    /// identical but for not testing the continuing wall's enabled bit.
    pub fn snap_to_ceiling_ground(&mut self, cd: &mut CollData) -> bool {
        self.snap_to_ceiling_impl(cd, false)
    }

    // -----------------------------------------------------------------------
    // Wall checks (mpColl_80044E10 .. mpColl_80046224 and ground/ceiling kin)
    // -----------------------------------------------------------------------

    /// `mpColl_80044E10_RightWall` (`mpcoll.c:1671`) and
    /// `mpColl_80045B74_LeftWall` (`mpcoll.c:1990`), "WallCheckAir": sweep
    /// the ECB's wall-side, bottom, and top points, the two ECB edges, and
    /// (unless the ECB is tiny) the wall vertices against the edges. Records
    /// every distinct wall hit and sets `*WallHug` / `*WallPush`.
    fn wall_check_air(&mut self, cd: &mut CollData, side: Side) -> bool {
        let mut hit_wall = false;
        match side {
            Side::Right => self.coll.right_count = 0,
            Side::Left => self.coll.left_count = 0,
        }
        // The ECB point that touches this wall side.
        let (side_pt, prev_side_pt, hug) = match side {
            Side::Right => (cd.ecb.left, cd.prev_ecb.left, collide::RIGHT_WALL_HUG),
            Side::Left => (cd.ecb.right, cd.prev_ecb.right, collide::LEFT_WALL_HUG),
        };
        let (sx, sy) = at(&cd.cur_pos, &side_pt);
        let (px, py) = at(&cd.prev_pos, &prev_side_pt);
        if let Some(id) = self.wall_sweep(cd, side, px, py, sx, sy) {
            self.push_wall(side, id);
            hit_wall = true;
            cd.env_flags |= hug as i32;
        }

        let (px, py) = at(&cd.prev_pos, &cd.prev_ecb.bottom);
        let (bx, by) = at(&cd.cur_pos, &cd.ecb.bottom);
        if let Some(id) = self.wall_sweep(cd, side, px, py, bx, by) {
            self.push_wall(side, id);
            hit_wall = true;
        }

        let (px, py) = at(&cd.prev_pos, &cd.prev_ecb.top);
        let (tx, ty) = at(&cd.cur_pos, &cd.ecb.top);
        if let Some(id) = self.wall_sweep(cd, side, px, py, tx, ty) {
            self.push_wall(side, id);
            hit_wall = true;
        }

        let (mut vx, mut vy) = at(&cd.cur_pos, &cd.ecb.bottom);
        if let Some(id) = self.wall_static(cd, side, vx, vy, sx, sy) {
            self.push_wall(side, id);
            hit_wall = true;
        }

        let (psx, psy) = at(&cd.prev_pos, &prev_side_pt);
        if !self.coll.is_ecb_tiny {
            let (pbx, pby) = at(&cd.prev_pos, &cd.prev_ecb.bottom);
            let found = match side {
                Side::Right => self.wall_vertex(cd, side, pbx, pby, psx, psy, vx, vy, sx, sy),
                Side::Left => self.wall_vertex(cd, side, psx, psy, pbx, pby, sx, sy, vx, vy),
            };
            if let Some(id) = found {
                self.push_wall(side, id);
                hit_wall = true;
            }
        }

        (vx, vy) = at(&cd.cur_pos, &cd.ecb.top);
        if let Some(id) = self.wall_static(cd, side, vx, vy, sx, sy) {
            self.push_wall(side, id);
            hit_wall = true;
        }

        if !self.coll.is_ecb_tiny {
            let (ptx, pty) = at(&cd.prev_pos, &cd.prev_ecb.top);
            let found = match side {
                Side::Right => self.wall_vertex(cd, side, psx, psy, ptx, pty, sx, sy, vx, vy),
                Side::Left => self.wall_vertex(cd, side, ptx, pty, psx, psy, vx, vy, sx, sy),
            };
            if let Some(id) = found {
                self.push_wall(side, id);
                hit_wall = true;
            }
        }

        if hit_wall {
            let push = match side {
                Side::Right => collide::RIGHT_WALL_PUSH,
                Side::Left => collide::LEFT_WALL_PUSH,
            };
            cd.env_flags |= push as i32;
        }
        hit_wall
    }

    /// `mpColl_80044E10_RightWall` (retail `0x80044E10`).
    pub fn right_wall_check_air(&mut self, cd: &mut CollData) -> bool {
        self.wall_check_air(cd, Side::Right)
    }

    /// `mpColl_80045B74_LeftWall` (retail `0x80045B74`).
    pub fn left_wall_check_air(&mut self, cd: &mut CollData) -> bool {
        self.wall_check_air(cd, Side::Left)
    }

    /// `mpColl_80048AB0_RightWall` (`mpcoll.c:2992`) and
    /// `mpColl_80049778_LeftWall` (`mpcoll.c:3287`): the grounded wall
    /// check. As the airborne one, but ignoring the two walls the current
    /// floor chain runs into (`line_id1`, `line_id2`) for the bottom-point
    /// sweeps.
    fn wall_check_ground(&mut self, cd: &mut CollData, side: Side) -> bool {
        let mut hit_wall = false;
        match side {
            Side::Right => self.coll.right_count = 0,
            Side::Left => self.coll.left_count = 0,
        }
        let (line_id1, line_id2) = if self.line_is_active(cd.floor.index) {
            match side {
                Side::Right => {
                    let l1 = self.line_next_non_floor_id0(cd.floor.index);
                    let temp = self.floor_prev_across_joint(cd.floor.index);
                    let l2 = if temp != NO_ID {
                        self.line_next_non_floor_id0(temp)
                    } else {
                        NO_ID
                    };
                    (l1, l2)
                }
                Side::Left => {
                    let l1 = self.line_prev_non_floor_id0(cd.floor.index);
                    let temp = self.floor_next_across_joint(cd.floor.index);
                    let l2 = if temp != NO_ID {
                        self.line_prev_non_floor_id0(temp)
                    } else {
                        NO_ID
                    };
                    (l1, l2)
                }
            }
        } else {
            (NO_ID, NO_ID)
        };
        let excluded = |id: i32| id != line_id1 && id != line_id2;

        let (side_pt, prev_side_pt, hug) = match side {
            Side::Right => (cd.ecb.left, cd.prev_ecb.left, collide::RIGHT_WALL_HUG),
            Side::Left => (cd.ecb.right, cd.prev_ecb.right, collide::LEFT_WALL_HUG),
        };
        let (sx, sy) = at(&cd.cur_pos, &side_pt);
        let (px, py) = at(&cd.prev_pos, &prev_side_pt);
        if let Some(id) = self.wall_sweep(cd, side, px, py, sx, sy) {
            self.push_wall(side, id);
            hit_wall = true;
            cd.env_flags |= hug as i32;
        }

        let (px, py) = at(&cd.prev_pos, &cd.prev_ecb.bottom);
        let (bx, by) = at(&cd.cur_pos, &cd.ecb.bottom);
        if let Some(id) = self.wall_sweep(cd, side, px, py, bx, by) {
            if excluded(id) {
                self.push_wall(side, id);
                hit_wall = true;
            }
        }

        let (px, py) = at(&cd.prev_pos, &cd.prev_ecb.top);
        let (tx, ty) = at(&cd.cur_pos, &cd.ecb.top);
        if let Some(id) = self.wall_sweep(cd, side, px, py, tx, ty) {
            self.push_wall(side, id);
            hit_wall = true;
        }

        let (mut vx, mut vy) = at(&cd.cur_pos, &cd.ecb.bottom);
        if let Some(id) = self.wall_static(cd, side, vx, vy, sx, sy) {
            if excluded(id) {
                self.push_wall(side, id);
                hit_wall = true;
            }
        }

        let (psx, psy) = at(&cd.prev_pos, &prev_side_pt);
        if !self.coll.is_ecb_tiny {
            let (pbx, pby) = at(&cd.prev_pos, &cd.prev_ecb.bottom);
            let found = match side {
                Side::Right => self.wall_vertex(cd, side, pbx, pby, psx, psy, vx, vy, sx, sy),
                Side::Left => self.wall_vertex(cd, side, psx, psy, pbx, pby, sx, sy, vx, vy),
            };
            if let Some(id) = found {
                if excluded(id) {
                    self.push_wall(side, id);
                    hit_wall = true;
                }
            }
        }

        (vx, vy) = at(&cd.cur_pos, &cd.ecb.top);
        if let Some(id) = self.wall_static(cd, side, vx, vy, sx, sy) {
            self.push_wall(side, id);
            hit_wall = true;
        }

        if !self.coll.is_ecb_tiny {
            let (ptx, pty) = at(&cd.prev_pos, &cd.prev_ecb.top);
            let found = match side {
                Side::Right => self.wall_vertex(cd, side, psx, psy, ptx, pty, sx, sy, vx, vy),
                Side::Left => self.wall_vertex(cd, side, ptx, pty, psx, psy, vx, vy, sx, sy),
            };
            if let Some(id) = found {
                self.push_wall(side, id);
                hit_wall = true;
            }
        }

        if hit_wall {
            let push = match side {
                Side::Right => collide::RIGHT_WALL_PUSH,
                Side::Left => collide::LEFT_WALL_PUSH,
            };
            cd.env_flags |= push as i32;
        }
        hit_wall
    }

    /// `mpColl_80048AB0_RightWall` (retail `0x80048AB0`).
    pub fn right_wall_check_ground(&mut self, cd: &mut CollData) -> bool {
        self.wall_check_ground(cd, Side::Right)
    }

    /// `mpColl_80049778_LeftWall` (retail `0x80049778`).
    pub fn left_wall_check_ground(&mut self, cd: &mut CollData) -> bool {
        self.wall_check_ground(cd, Side::Left)
    }

    /// `mpColl_8004B894_RightWall` (`mpcoll.c:4099`) and
    /// `mpColl_8004BDD4_LeftWall` (`mpcoll.c:4194`): the ceiling-hanging
    /// wall check: no vertex sweeps, and the walls the ceiling chain runs
    /// into are ignored for the top-point sweeps.
    fn wall_check_ceiling(&mut self, cd: &mut CollData, side: Side) -> bool {
        let mut hit_wall = false;
        match side {
            Side::Right => self.coll.right_count = 0,
            Side::Left => self.coll.left_count = 0,
        }
        let (line_id1, line_id2) = if self.line_is_active(cd.ceiling.index) {
            match side {
                Side::Right => {
                    let l1 = self.line_prev_non_ceiling_id0(cd.ceiling.index);
                    let temp = self.ceiling_next_across_joint(cd.ceiling.index);
                    let l2 = if temp != NO_ID {
                        self.line_prev_non_ceiling_id0(temp)
                    } else {
                        NO_ID
                    };
                    (l1, l2)
                }
                Side::Left => {
                    let l1 = self.line_next_non_ceiling_id0(cd.ceiling.index);
                    let temp = self.ceiling_prev_across_joint(cd.ceiling.index);
                    let l2 = if temp != NO_ID {
                        self.line_next_non_ceiling_id0(temp)
                    } else {
                        NO_ID
                    };
                    (l1, l2)
                }
            }
        } else {
            (NO_ID, NO_ID)
        };
        let excluded = |id: i32| id != line_id1 && id != line_id2;

        let (side_pt, prev_side_pt, hug) = match side {
            Side::Right => (cd.ecb.left, cd.prev_ecb.left, collide::RIGHT_WALL_HUG),
            Side::Left => (cd.ecb.right, cd.prev_ecb.right, collide::LEFT_WALL_HUG),
        };
        let (sx, sy) = at(&cd.cur_pos, &side_pt);
        let (px, py) = at(&cd.prev_pos, &prev_side_pt);
        if let Some(id) = self.wall_sweep(cd, side, px, py, sx, sy) {
            self.push_wall(side, id);
            hit_wall = true;
            cd.env_flags |= hug as i32;
        }

        let (px, py) = at(&cd.prev_pos, &cd.prev_ecb.bottom);
        let (bx, by) = at(&cd.cur_pos, &cd.ecb.bottom);
        if let Some(id) = self.wall_sweep(cd, side, px, py, bx, by) {
            self.push_wall(side, id);
            hit_wall = true;
        }

        let (px, py) = at(&cd.prev_pos, &cd.prev_ecb.top);
        let (tx, ty) = at(&cd.cur_pos, &cd.ecb.top);
        if let Some(id) = self.wall_sweep(cd, side, px, py, tx, ty) {
            if excluded(id) {
                self.push_wall(side, id);
                hit_wall = true;
            }
        }

        let (vx, vy) = at(&cd.cur_pos, &cd.ecb.bottom);
        if let Some(id) = self.wall_static(cd, side, vx, vy, sx, sy) {
            self.push_wall(side, id);
            hit_wall = true;
        }

        let (vx, vy) = at(&cd.cur_pos, &cd.ecb.top);
        if let Some(id) = self.wall_static(cd, side, vx, vy, sx, sy) {
            if excluded(id) {
                self.push_wall(side, id);
                hit_wall = true;
            }
        }

        if hit_wall {
            let push = match side {
                Side::Right => collide::RIGHT_WALL_PUSH,
                Side::Left => collide::LEFT_WALL_PUSH,
            };
            cd.env_flags |= push as i32;
        }
        hit_wall
    }

    /// `mpColl_8004B894_RightWall` (retail `0x8004B894`).
    pub fn right_wall_check_ceiling(&mut self, cd: &mut CollData) -> bool {
        self.wall_check_ceiling(cd, Side::Right)
    }

    /// `mpColl_8004BDD4_LeftWall` (retail `0x8004BDD4`).
    pub fn left_wall_check_ceiling(&mut self, cd: &mut CollData) -> bool {
        self.wall_check_ceiling(cd, Side::Left)
    }

    // -----------------------------------------------------------------------
    // Wall collide (mpColl_800454A4, mpColl_80046224, mpColl_800491C8,
    // mpColl_80049EAC)
    // -----------------------------------------------------------------------

    /// Record a better wall candidate in the scratch accumulator.
    #[inline]
    fn wall_candidate(
        &mut self,
        side: Side,
        x: f32,
        line_id: i32,
        flags: u32,
        normal: Vec3,
    ) -> bool {
        let better = match side {
            Side::Right => self.coll.max_x < x,
            Side::Left => self.coll.max_x > x,
        };
        if better {
            self.coll.max_x = x;
            self.coll.line_id = line_id;
            self.coll.flags = flags;
            self.coll.normal = normal;
        }
        better
    }

    /// Shared body of the four "WallCollide" routines: `mpColl_800454A4`
    /// (`mpcoll.c:1780`, right, air), `mpColl_80046224` (`2098`, left, air),
    /// `mpColl_800491C8` (`3116`, right, ground), `mpColl_80049EAC` (`3411`,
    /// left, ground). For every recorded wall, find the x the body must be
    /// pushed to so the ECB clears it (probing the ECB's three points, the
    /// wall's end vertices against the ECB edges, and, in the air
    /// variants, the ceiling corner above the wall), then apply the largest
    /// push.
    ///
    /// The air-left variant's ECB edge slopes subtract `ecb.bottom.x` /
    /// `ecb.top.x` where the other three do not; `sub_ecb_x` keeps that.
    fn wall_collide(
        &mut self,
        cd: &mut CollData,
        side: Side,
        with_ceiling_corner: bool,
        sub_ecb_x: bool,
    ) -> bool {
        self.coll.max_x = match side {
            Side::Right => -F32_MAX,
            Side::Left => F32_MAX,
        };
        let count = match side {
            Side::Right => self.coll.right_count,
            Side::Left => self.coll.left_count,
        } as usize;
        let (bx, by) = at(&cd.cur_pos, &cd.ecb.bottom);
        let (tx, ty) = at(&cd.cur_pos, &cd.ecb.top);
        // The ECB point on this wall's side.
        let side_pt = match side {
            Side::Right => cd.ecb.left,
            Side::Left => cd.ecb.right,
        };

        for i in 0..count {
            let mut wall_id = match side {
                Side::Right => self.coll.right[i],
                Side::Left => self.coll.left[i],
            };
            let get_top = |m: &Self, id: i32| match side {
                Side::Right => m.right_wall_get_top(id),
                Side::Left => m.left_wall_get_top(id),
            };
            let get_bottom = |m: &Self, id: i32| match side {
                Side::Right => m.right_wall_get_bottom(id),
                Side::Left => m.left_wall_get_bottom(id),
            };
            let probe = |m: &Self, id: i32, p: &Vec3| match side {
                Side::Right => m.right_wall_probe(id, p),
                Side::Left => m.left_wall_probe(id, p),
            };
            let better = |m: &Self, x: f32| match side {
                Side::Right => m.coll.max_x < x,
                Side::Left => m.coll.max_x > x,
            };

            let mut pos = get_top(self, wall_id);
            if pos.y < by {
                if better(self, pos.x) {
                    if let Some(p) = probe(self, wall_id, &pos) {
                        self.wall_candidate(side, pos.x, p.line_id, p.flags, p.normal);
                    }
                }
                continue;
            }

            pos = get_bottom(self, wall_id);
            if pos.y > ty {
                if better(self, pos.x) {
                    if let Some(p) = probe(self, wall_id, &pos) {
                        self.wall_candidate(side, pos.x, p.line_id, p.flags, p.normal);
                    }
                }
                continue;
            }

            pos = Vec3::new(bx, by, 0.0);
            if let Some(p) = probe(self, wall_id, &pos) {
                self.wall_candidate(side, cd.cur_pos.x + p.delta, p.line_id, p.flags, p.normal);
            }
            pos = Vec3::new(cd.cur_pos.x + side_pt.x, cd.cur_pos.y + side_pt.y, 0.0);
            if let Some(p) = probe(self, wall_id, &pos) {
                self.wall_candidate(side, cd.cur_pos.x + p.delta, p.line_id, p.flags, p.normal);
            }
            pos = Vec3::new(tx, ty, 0.0);
            if let Some(p) = probe(self, wall_id, &pos) {
                self.wall_candidate(side, cd.cur_pos.x + p.delta, p.line_id, p.flags, p.normal);
            }

            if with_ceiling_corner {
                match side {
                    Side::Right => {
                        let line_id = self.line_prev_non_right_wall(wall_id);
                        if line_id != NO_ID
                            && self.line_is_active(line_id)
                            && self.line_get_kind(line_id) & line_kind::CEILING != 0
                        {
                            let top = self.right_wall_get_top(wall_id);
                            if pos.y > top.y {
                                let line_id = self.line_next_non_ceiling(line_id);
                                if line_id != NO_ID
                                    && self.line_is_active(line_id)
                                    && self.line_get_kind(line_id) & line_kind::RIGHT_WALL != 0
                                {
                                    let nrm = self.line_get_normal(line_id);
                                    // retail 0x80045820/2C/34: fneg, fdivs, fmadds.
                                    let x = fmadds(-nrm.y, (pos.y - top.y) / nrm.x, top.x) - pos.x
                                        + 0.5;
                                    if self.coll.max_x < cd.cur_pos.x + x {
                                        let temp = self.line_get_flags(line_id);
                                        self.wall_candidate(
                                            side,
                                            cd.cur_pos.x + x,
                                            line_id,
                                            temp,
                                            nrm,
                                        );
                                    }
                                }
                            }
                        }
                    }
                    Side::Left => {
                        let line_id = self.line_next_non_left_wall(wall_id);
                        if line_id != NO_ID
                            && self.line_is_active(line_id)
                            && self.line_get_kind(line_id) & line_kind::CEILING != 0
                        {
                            let vec = self.left_wall_get_top(wall_id);
                            if pos.y > vec.y {
                                let line_id2 = self.line_prev_non_ceiling(line_id);
                                // The C tests `line_id` (the ceiling) here,
                                // not `line_id2`, so this branch is dead in
                                // retail; transcribed as is.
                                if line_id2 != NO_ID
                                    && self.line_is_active(line_id)
                                    && self.line_get_kind(line_id) & line_kind::LEFT_WALL != 0
                                {
                                    let nrm = self.line_get_normal(line_id2);
                                    // retail 0x80046598/B0/B4: fneg, fdivs, fmadds.
                                    let x = fmadds(nrm.y, (pos.y - vec.y) / -nrm.x, vec.x)
                                        - pos.x
                                        - 0.5;
                                    if self.coll.max_x > cd.cur_pos.x + x {
                                        let temp = self.line_get_flags(line_id2);
                                        self.wall_candidate(
                                            side,
                                            cd.cur_pos.x + x,
                                            line_id2,
                                            temp,
                                            nrm,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // The ECB's two edges on this side, as x-per-y slopes.
            let top_y = cd.ecb.top.y;
            let side_y = side_pt.y;
            let bottom_y = cd.ecb.bottom.y;
            let side_x = side_pt.x;
            let (f27, f26) = if sub_ecb_x {
                (
                    (side_x - cd.ecb.bottom.x) / (side_y - bottom_y),
                    (side_x - cd.ecb.top.x) / (side_y - top_y),
                )
            } else {
                (side_x / (side_y - bottom_y), side_x / (side_y - top_y))
            };
            let top = cd.cur_pos.y + top_y;
            let mid = cd.cur_pos.y + side_y;
            let bot = cd.cur_pos.y + bottom_y;
            let kind = match side {
                Side::Right => line_kind::RIGHT_WALL,
                Side::Left => line_kind::LEFT_WALL,
            };

            // Walk the wall chain downward from `wall_id`, testing the lower
            // vertex of each line against the ECB's lower and upper edges.
            let mut j = wall_id;
            while j != NO_ID && (self.line_get_kind(j) & line_kind::KIND_MASK) == kind {
                let pos = match side {
                    Side::Right => self.line_get_v1_pos(j),
                    Side::Left => self.line_get_v0_pos(j),
                };
                let x;
                if bot <= pos.y && pos.y <= mid {
                    // retail 0x800458F8/0x80046688/0x80049500/0x8004A1E4: fmadds.
                    x = fmadds(f27, pos.y - bot, cd.ecb.bottom.x);
                } else if mid <= pos.y && pos.y <= top {
                    // retail 0x80045924/0x800466B4/0x8004952C/0x8004A210: fmadds.
                    x = fmadds(f26, pos.y - top, cd.ecb.top.x);
                } else if pos.y < bot {
                    break;
                } else {
                    j = match side {
                        Side::Right => self.line_next(j),
                        Side::Left => self.line_prev(j),
                    };
                    continue;
                }
                let x = pos.x - x;
                if better(self, x) {
                    let flags = self.line_get_flags(j);
                    let normal = self.line_get_normal(j);
                    self.wall_candidate(side, x, j, flags, normal);
                }
                j = match side {
                    Side::Right => self.line_next(j),
                    Side::Left => self.line_prev(j),
                };
            }

            // And upward, testing the upper vertex.
            while wall_id != NO_ID && (self.line_get_kind(wall_id) & line_kind::KIND_MASK) == kind {
                let pos = match side {
                    Side::Right => self.line_get_v0_pos(wall_id),
                    Side::Left => self.line_get_v1_pos(wall_id),
                };
                let x;
                if bot <= pos.y && pos.y <= mid {
                    // retail 0x800459FC/0x8004678C/0x80049604/0x8004A2E8: fmadds.
                    x = fmadds(f27, pos.y - bot, cd.ecb.bottom.x);
                } else if mid <= pos.y && pos.y <= top {
                    // retail 0x80045A28/0x800467B8/0x80049630/0x8004A314: fmadds.
                    x = fmadds(f26, pos.y - top, cd.ecb.top.x);
                } else if pos.y > top {
                    break;
                } else {
                    wall_id = match side {
                        Side::Right => self.line_prev(wall_id),
                        Side::Left => self.line_next(wall_id),
                    };
                    continue;
                }
                let x = pos.x - x;
                if better(self, x) {
                    let flags = self.line_get_flags(wall_id);
                    let normal = self.line_get_normal(wall_id);
                    self.wall_candidate(side, x, wall_id, flags, normal);
                }
                wall_id = match side {
                    Side::Right => self.line_prev(wall_id),
                    Side::Left => self.line_next(wall_id),
                };
            }
        }

        let line_id = self.coll.line_id;
        let flags = self.coll.flags;
        let normal = self.coll.normal;
        let pushed = match side {
            Side::Right => cd.cur_pos.x < self.coll.max_x,
            Side::Left => cd.cur_pos.x > self.coll.max_x,
        };
        if pushed {
            cd.cur_pos.x = self.coll.max_x;
            let sd = match side {
                Side::Right => &mut cd.right_facing_wall,
                Side::Left => &mut cd.left_facing_wall,
            };
            sd.index = line_id;
            sd.flags = flags;
            sd.normal = normal;
            return true;
        }
        false
    }

    /// `mpColl_800454A4_RightWall` (retail `0x800454A4`), "RightWallCollideAir".
    pub fn right_wall_collide_air(&mut self, cd: &mut CollData) -> bool {
        self.wall_collide(cd, Side::Right, true, false)
    }

    /// `mpColl_80046224_LeftWall` (retail `0x80046224`), "LeftWallCollideAir".
    pub fn left_wall_collide_air(&mut self, cd: &mut CollData) -> bool {
        self.wall_collide(cd, Side::Left, true, true)
    }

    /// `mpColl_800491C8_RightWall` (retail `0x800491C8`): grounded.
    pub fn right_wall_collide_ground(&mut self, cd: &mut CollData) -> bool {
        self.wall_collide(cd, Side::Right, false, false)
    }

    /// `mpColl_80049EAC_LeftWall` (retail `0x80049EAC`): grounded.
    pub fn left_wall_collide_ground(&mut self, cd: &mut CollData) -> bool {
        self.wall_collide(cd, Side::Left, false, false)
    }

    // -----------------------------------------------------------------------
    // Composite helpers (mpCollCeilingInline, mpCollFloorInline)
    // -----------------------------------------------------------------------

    /// `mpCollCeilingInline` (`mpcoll.c:2299`): after a ceiling snap, push
    /// out of any wall the ECB's side points now touch.
    fn ceiling_wall_corners(&mut self, cd: &mut CollData) {
        let non_ceiling_id = self.line_next_non_ceiling(cd.ceiling.index);
        let (tx, ty) = at(&cd.cur_pos, &cd.ecb.top);
        let (sx, sy) = at(&cd.cur_pos, &cd.ecb.right);
        let hit_wall = matches!(
            self.wall_static(cd, Side::Left, tx, ty, sx, sy),
            Some(id) if id != non_ceiling_id
        );
        if hit_wall {
            self.ceiling_left_wall_multi_collide(cd);
        }

        let non_ceiling_id = self.line_prev_non_ceiling(cd.ceiling.index);
        let (tx, ty) = at(&cd.cur_pos, &cd.ecb.top);
        let (sx, sy) = at(&cd.cur_pos, &cd.ecb.left);
        let hit_wall = matches!(
            self.wall_static(cd, Side::Right, tx, ty, sx, sy),
            Some(id) if id != non_ceiling_id
        );
        if hit_wall {
            self.ceiling_right_wall_multi_collide(cd);
        }
    }

    /// `mpCollFloorInline` (`mpcoll.c:2343`): after a floor snap, push out
    /// of any wall the ECB now touches.
    fn floor_wall_corners(&mut self, cd: &mut CollData, ecb_unlocked: bool, squeeze_flags: i32) {
        if let Some(wall_id) = self.floor_connected_left_wall(cd) {
            self.floor_left_wall_multi_collide(cd, wall_id, ecb_unlocked && squeeze_flags & 1 == 0);
        }
        if let Some(wall_id) = self.floor_connected_right_wall(cd) {
            self.floor_right_wall_multi_collide(
                cd,
                wall_id,
                ecb_unlocked && squeeze_flags & 1 == 0,
            );
        }
    }

    // -----------------------------------------------------------------------
    // The collision routines
    // -----------------------------------------------------------------------

    /// `mpColl_80046904` (retail `0x80046904`, `mpcoll.c:2360`): one step of
    /// airborne collision. Walls (twice each side), squeeze, ceiling, floor,
    /// squeeze, repeated until the ECB and squeeze state settle; then ledge
    /// grabs. Returns whether the body landed.
    pub fn air_collide(
        &mut self,
        cd: &mut CollData,
        flags: u32,
        floor_cb: LineFilter<'_, '_>,
    ) -> bool {
        let platform_pass = flags & air_flags::PLATFORM_PASS_CALLBACK != 0;
        let stay_airborne = flags & air_flags::STAY_AIRBORNE != 0;
        let mut floor_cb = floor_cb;

        let mut left_right_flags = 0i32;
        let mut touched_floor = false;
        let mut squeeze_flags_all = 0i32;
        let mut squeeze_flags = 0i32;

        loop {
            let mut x_after_collide_right = 0.0f32;
            let old_squeeze_flags = squeeze_flags;
            let mut x_after_collide_left = 0.0f32;
            let prev_b6 = cd.x34_flags.b6;
            squeeze_flags = 0;
            for _ in 0..2 {
                if self.left_wall_check_air(cd) {
                    if self.left_wall_collide_air(cd) {
                        left_right_flags |= 1;
                        squeeze_flags |= 8;
                    }
                    x_after_collide_left = cd.cur_pos.x;
                }
                if self.right_wall_check_air(cd) {
                    if self.right_wall_collide_air(cd) {
                        left_right_flags |= 2;
                        squeeze_flags |= 4;
                    }
                    x_after_collide_right = cd.cur_pos.x;
                }
            }

            if (squeeze_flags & 0xC) == 0xC {
                squeeze_horizontal(cd, true, x_after_collide_right, x_after_collide_left);
            }

            let mut y_after_collide_ceiling = 0.0f32;
            let mut y_after_collide_floor = 0.0f32;

            if self.ceiling_check(cd, left_right_flags) && self.snap_to_ceiling_air(cd) {
                self.ceiling_wall_corners(cd);
                squeeze_flags |= 1;
                y_after_collide_ceiling = cd.cur_pos.y;
            }

            let r3 = if platform_pass {
                self.floor_check_air(cd, floor_cb.as_deref_mut(), left_right_flags)
            } else {
                self.floor_check_air(cd, None, left_right_flags)
            };

            if r3 {
                if stay_airborne {
                    if self.snap_to_floor_stay_airborne(cd) {
                        if let Some(wid) = self.floor_connected_left_wall(cd) {
                            self.floor_left_wall_multi_collide(cd, wid, false);
                        }
                        if let Some(wid) = self.floor_connected_right_wall(cd) {
                            self.floor_right_wall_multi_collide(cd, wid, false);
                        }
                    }
                } else {
                    let ecb_unlocked = cd.ecb.bottom.y > 0.0;
                    if self.snap_to_floor(cd, ecb_unlocked && squeeze_flags & 1 == 0) {
                        if let Some(wid) = self.floor_connected_left_wall(cd) {
                            self.floor_left_wall_multi_collide(
                                cd,
                                wid,
                                ecb_unlocked && squeeze_flags & 1 == 0,
                            );
                        }
                        if let Some(wid) = self.floor_connected_right_wall(cd) {
                            self.floor_right_wall_multi_collide(
                                cd,
                                wid,
                                ecb_unlocked && squeeze_flags & 1 == 0,
                            );
                        }
                        cd.x34_flags.b5 = true;
                        touched_floor = true;
                    }
                }

                y_after_collide_floor = cd.cur_pos.y;
                squeeze_flags |= 2;
                if self.ceiling_check(cd, left_right_flags) && self.snap_to_ceiling_air(cd) {
                    self.ceiling_wall_corners(cd);
                    squeeze_flags |= 1;
                    y_after_collide_ceiling = cd.cur_pos.y;
                }
            }
            if (squeeze_flags & 3) == 3 {
                let airborne = !touched_floor;
                squeeze_vertical(cd, airborne, y_after_collide_ceiling, y_after_collide_floor);
            }
            squeeze_flags_all |= squeeze_flags;
            if !(prev_b6 != cd.x34_flags.b6 || squeeze_flags != old_squeeze_flags) {
                break;
            }
        }

        if !touched_floor && flags & air_flags::CAN_GRAB_LEDGE != 0 {
            let env = cd.env_flags as u32;
            let mut on_edge = env & collide::LEFT_EDGE != 0 || env & collide::RIGHT_EDGE != 0;
            if !on_edge && cd.cur_pos.y < cd.prev_pos.y {
                if cd.facing_dir == 1 || cd.facing_dir == 0 {
                    if let Some(id) = self.check_for_left_ledge(cd) {
                        cd.ledge_id_left = id;
                        on_edge = true;
                        cd.env_flags |= collide::LEFT_LEDGE_GRAB as i32;
                    } else {
                        on_edge = false;
                    }
                    if on_edge {
                        cd.env_flags |= collide::LEFT_LEDGE_GRAB as i32;
                    }
                }
                if cd.facing_dir == -1 || cd.facing_dir == 0 {
                    if let Some(id) = self.check_for_right_ledge(cd) {
                        cd.ledge_id_right = id;
                        on_edge = true;
                        cd.env_flags |= collide::RIGHT_LEDGE_GRAB as i32;
                    } else {
                        on_edge = false;
                    }
                    if on_edge {
                        cd.env_flags |= collide::RIGHT_LEDGE_GRAB as i32;
                    }
                }
            }
        }

        if squeeze_flags_all & 8 == 0 {
            cd.env_flags &= !(collide::LEFT_WALL_MASK as i32);
        }
        if squeeze_flags_all & 4 == 0 {
            cd.env_flags &= !(collide::RIGHT_WALL_MASK as i32);
        }
        touched_floor
    }

    /// `mpColl_80046F78` (retail `0x80046F78`, `mpcoll.c:2582`): one step of
    /// point collision (no ECB): sweep the position against every surface
    /// and snap to whatever is hit first.
    pub fn point_collide(&mut self, cd: &mut CollData) -> bool {
        let (px, py) = (cd.prev_pos.x, cd.prev_pos.y);
        let (x, y) = (cd.cur_pos.x, cd.cur_pos.y);
        let (js, jo) = (cd.joint_id_skip, cd.joint_id_only);
        let hit = if self.moved_since(cd) {
            self.check_all_remap(js, jo, px, py, x, y)
        } else {
            self.check_all(js, jo, px, py, x, y)
        };
        let Some(h) = hit else {
            return false;
        };
        cd.contact = h.pos;
        let line_id = h.line_id;
        let kind = self.line_get_kind(line_id);
        if kind == line_kind::FLOOR {
            if let Some(p) = self.floor_probe(line_id, &cd.contact) {
                store_surface(&mut cd.floor, p.flags, p.normal);
                cd.floor.index = p.line_id;
                cd.cur_pos.x = cd.contact.x;
                cd.cur_pos.y = cd.contact.y + p.delta;
                cd.cur_pos.z = cd.contact.z;
                cd.env_flags |= collide::FLOOR_PUSH as i32;
                return true;
            }
            false
        } else if kind == line_kind::CEILING {
            if let Some(p) = self.ceiling_probe(line_id, &cd.contact) {
                store_surface(&mut cd.ceiling, p.flags, p.normal);
                cd.ceiling.index = p.line_id;
                cd.cur_pos.x = cd.contact.x;
                cd.cur_pos.y = cd.contact.y + p.delta;
                cd.cur_pos.z = cd.contact.z;
                cd.env_flags |= collide::CEILING_PUSH as i32;
                return true;
            }
            false
        } else if kind == line_kind::LEFT_WALL {
            if let Some(p) = self.left_wall_probe(line_id, &cd.contact) {
                store_surface(&mut cd.left_facing_wall, p.flags, p.normal);
                cd.left_facing_wall.index = p.line_id;
                cd.cur_pos.x = cd.contact.x + p.delta;
                cd.cur_pos.y = cd.contact.y;
                cd.cur_pos.z = cd.contact.z;
                cd.env_flags |= collide::LEFT_WALL_PUSH as i32;
                return true;
            }
            false
        } else if kind == line_kind::RIGHT_WALL {
            if let Some(p) = self.right_wall_probe(line_id, &cd.contact) {
                store_surface(&mut cd.right_facing_wall, p.flags, p.normal);
                cd.right_facing_wall.index = p.line_id;
                cd.cur_pos.x = cd.contact.x + p.delta;
                cd.cur_pos.y = cd.contact.y;
                cd.cur_pos.z = cd.contact.z;
                cd.env_flags |= collide::RIGHT_WALL_PUSH as i32;
                return true;
            }
            false
        } else {
            panic!("mpcoll.c:3685: line {line_id} has no kind");
        }
    }

    /// `mpColl_800488F4` (retail `0x800488F4`, `mpcoll.c:2932`): the grounded
    /// floor follow. Snap onto the current floor chain; past its end, stop
    /// at the end if a wall continues there, else flag a ledge slip.
    pub fn follow_floor_ground(&mut self, cd: &mut CollData) -> bool {
        let floor_id = cd.floor.index;
        let bottom = Vec3::new(
            cd.cur_pos.x + cd.ecb.bottom.x,
            cd.cur_pos.y + cd.ecb.bottom.y,
            0.0,
        );
        if !self.line_is_active(cd.floor.index)
            || self.line_get_kind(cd.floor.index) != line_kind::FLOOR
        {
            return false;
        }
        if let Some(p) = self.floor_probe(floor_id, &bottom) {
            store_surface(&mut cd.floor, p.flags, p.normal);
            cd.cur_pos.y += p.delta;
            cd.floor.index = p.line_id;
            return true;
        }
        let mut hit_wall = false;
        let mut edge = self.floor_get_left(floor_id);
        if cd.cur_pos.x < edge.x {
            let non_floor_id = self.line_prev_non_floor(floor_id);
            if non_floor_id != NO_ID
                && self.line_is_active(non_floor_id)
                && self.line_get_kind(non_floor_id) & line_kind::RIGHT_WALL != 0
            {
                hit_wall = true;
            } else {
                cd.env_flags |= collide::LEFT_LEDGE_SLIP as i32;
            }
        } else {
            edge = self.floor_get_right(floor_id);
            if cd.cur_pos.x > edge.x {
                let non_floor_id = self.line_next_non_floor(floor_id);
                if non_floor_id != NO_ID
                    && self.line_is_active(non_floor_id)
                    && self.line_get_kind(non_floor_id) & line_kind::LEFT_WALL != 0
                {
                    hit_wall = true;
                } else {
                    cd.env_flags |= collide::RIGHT_LEDGE_SLIP as i32;
                }
            }
        }
        if hit_wall {
            cd.cur_pos = edge;
            return true;
        }
        false
    }

    /// `mpColl_8004A45C_Floor` (retail `0x8004A45C`, `mpcoll.c:3586`): stop
    /// at the floor's end instead of walking off it (sets `Collide_*Edge`),
    /// unless a wall there has already stopped the body.
    pub fn stop_at_floor_edge(&mut self, cd: &mut CollData, line_id: i32) -> bool {
        if !self.line_is_active(line_id) || self.line_get_kind(line_id) != line_kind::FLOOR {
            return false;
        }
        let mut on_edge = false;
        let mut floor: Option<(i32, u32, Vec3)> = None;
        let mut edge = self.floor_get_left(line_id);
        if cd.cur_pos.x <= edge.x {
            let p = self.floor_probe(line_id, &edge);
            floor = p.map(|p| (p.line_id, p.flags, p.normal));
            let edge_x = edge.x + 1.0;
            let edge_y = edge.y + 1.0;
            let right_x = edge.x + cd.ecb.right.x - cd.ecb.bottom.x;
            let right_y = edge.y + cd.ecb.right.y - cd.ecb.bottom.y;
            // make sure a wall hasn't stopped us
            if self
                .check_left_wall(
                    edge_x,
                    edge_y,
                    right_x,
                    right_y,
                    cd.joint_id_skip,
                    cd.joint_id_only,
                )
                .is_none()
            {
                on_edge = true;
                cd.env_flags |= collide::RIGHT_EDGE as i32;
            }
        } else {
            edge = self.floor_get_right(line_id);
            if cd.cur_pos.x >= edge.x {
                let p = self.floor_probe(line_id, &edge);
                floor = p.map(|p| (p.line_id, p.flags, p.normal));
                let edge_x = edge.x - 1.0;
                let edge_y = edge.y + 1.0;
                let left_x = edge.x + cd.ecb.left.x - cd.ecb.bottom.x;
                let left_y = edge.y + cd.ecb.left.y - cd.ecb.bottom.y;
                if self
                    .check_right_wall(
                        edge_x,
                        edge_y,
                        left_x,
                        left_y,
                        cd.joint_id_skip,
                        cd.joint_id_only,
                    )
                    .is_none()
                {
                    on_edge = true;
                    cd.env_flags |= collide::LEFT_EDGE as i32;
                }
            }
        }
        if on_edge {
            cd.cur_pos.x = edge.x - cd.ecb.bottom.x;
            cd.cur_pos.y = edge.y - cd.ecb.bottom.y;
            cd.cur_pos.z = edge.z;
            // The C copies its locals whether or not the probe succeeded;
            // on the (unreachable for a point on the line's own end) failure
            // the index is -1 and flags/normal are uninitialised. Leave them
            // unchanged in that case.
            let (idx, flags, normal) = floor.unwrap_or((NO_ID, cd.floor.flags, cd.floor.normal));
            cd.floor.index = idx;
            cd.floor.flags = flags;
            cd.floor.normal = normal;
            return true;
        }
        false
    }

    /// `mpColl_8004A678_Floor` (retail `0x8004A678`, `mpcoll.c:3655`): the
    /// teeter: stop at the floor's end when facing off it with the stick
    /// held less than 0.75 outward and no wall touching (sets
    /// `Collide_Edge`).
    pub fn teeter_at_floor_edge(&mut self, cd: &mut CollData, line_id: i32) -> bool {
        if !self.line_is_active(line_id) || self.line_get_kind(line_id) != line_kind::FLOOR {
            return false;
        }
        let mut on_edge = false;
        let mut floor: Option<(i32, u32, Vec3)> = None;
        let env = cd.env_flags as u32;
        let mut edge = self.floor_get_left(line_id);
        if cd.cur_pos.x <= edge.x {
            if env & collide::LEFT_WALL_MASK == 0
                && cd.facing_dir == -1
                && f64::from(cd.lstick_x) > -0.75
            {
                if let Some(p) = self.floor_probe(line_id, &edge) {
                    floor = Some((p.line_id, p.flags, p.normal));
                    edge.y += p.delta;
                }
                let edge_x = edge.x + 1.0;
                let edge_y = edge.y + 1.0;
                let right_x = edge.x + cd.ecb.right.x - cd.ecb.bottom.x;
                let right_y = edge.y + cd.ecb.right.y - cd.ecb.bottom.y;
                // make sure a wall hasn't stopped us
                if self
                    .check_left_wall(
                        edge_x,
                        edge_y,
                        right_x,
                        right_y,
                        cd.joint_id_skip,
                        cd.joint_id_only,
                    )
                    .is_none()
                {
                    on_edge = true;
                }
            }
        } else {
            edge = self.floor_get_right(line_id);
            if cd.cur_pos.x >= edge.x
                && env & collide::RIGHT_WALL_MASK == 0
                && cd.facing_dir == 1
                && f64::from(cd.lstick_x) < 0.75
            {
                if let Some(p) = self.floor_probe(line_id, &edge) {
                    floor = Some((p.line_id, p.flags, p.normal));
                    edge.y += p.delta;
                }
                let edge_x = edge.x - 1.0;
                let edge_y = edge.y + 1.0;
                let left_x = edge.x + cd.ecb.left.x - cd.ecb.bottom.x;
                let left_y = edge.y + cd.ecb.left.y - cd.ecb.bottom.y;
                if self
                    .check_right_wall(
                        edge_x,
                        edge_y,
                        left_x,
                        left_y,
                        cd.joint_id_skip,
                        cd.joint_id_only,
                    )
                    .is_none()
                {
                    on_edge = true;
                }
            }
        }
        if on_edge {
            cd.cur_pos.x = edge.x - cd.ecb.bottom.x;
            cd.cur_pos.y = edge.y - cd.ecb.bottom.y;
            cd.cur_pos.z = edge.z;
            // See stop_at_floor_edge for the failed-probe case.
            let (idx, flags, normal) = floor.unwrap_or((NO_ID, cd.floor.flags, cd.floor.normal));
            cd.floor.index = idx;
            cd.floor.flags = flags;
            cd.floor.normal = normal;
            cd.env_flags |= collide::EDGE as i32;
            return true;
        }
        false
    }

    /// `mpColl_8004A908_Floor` (retail `0x8004A908`, `mpcoll.c:3739`): after
    /// walking off floor `line_id`, look for a different, unconnected floor
    /// under the ECB bottom (first from the previous bottom, then from the
    /// previous ECB midpoint).
    pub fn find_new_floor_below(&mut self, cd: &mut CollData, line_id: i32) -> bool {
        let mut pbx = cd.prev_pos.x + cd.prev_ecb.bottom.x;
        let mut pby = cd.prev_pos.y + cd.prev_ecb.bottom.y;
        let (bx, by) = at(&cd.cur_pos, &cd.ecb.bottom);
        for pass in 0..2 {
            if pass == 1 {
                // retail 0x8004AA5C/6C: fadds + fmadds.
                pby = fmadds(0.5, cd.prev_ecb.top.y + cd.prev_ecb.bottom.y, cd.prev_pos.y);
                pbx = cd.prev_pos.x + cd.prev_ecb.bottom.x;
            }
            let hit = if self.moved_since(cd) {
                self.check_floor_remap(
                    pbx,
                    pby,
                    bx,
                    by,
                    0.0,
                    cd.floor_skip,
                    cd.joint_id_skip,
                    cd.joint_id_only,
                    None,
                )
            } else {
                self.check_floor(
                    pbx,
                    pby,
                    bx,
                    by,
                    0.0,
                    cd.floor_skip,
                    cd.joint_id_skip,
                    cd.joint_id_only,
                    None,
                )
            };
            if let Some(h) = hit {
                if h.line_id != NO_ID
                    && h.line_id != line_id
                    && (line_id == NO_ID || !self.lines_connected(h.line_id, line_id))
                {
                    cd.floor.index = h.line_id;
                    cd.floor.flags = h.flags;
                    cd.floor.normal = h.normal;
                    return true;
                }
            }
        }
        false
    }

    /// `mpColl_8004ACE4` (retail `0x8004ACE4`, `mpcoll.c:3850`): one step of
    /// grounded collision. `flags` bit 0 teeters at edges
    /// ([`Self::teeter_at_floor_edge`]), bit 1 stops at edges
    /// ([`Self::stop_at_floor_edge`]); otherwise walking off a floor falls
    /// through to [`Self::air_collide`].
    pub fn ground_collide(&mut self, cd: &mut CollData, flags: u32) -> bool {
        let mut touching_floor = false;
        let mut left_right = 0i32;
        loop {
            let mut x_after_right = 0.0f32;
            let mut x_after_left = 0.0f32;
            let mut y_after_ceiling = 0.0f32;
            let mut y_after_floor = 0.0f32;
            let mut hit_right = false;
            let prev_b6 = cd.x34_flags.b6;
            let mut hit_left = false;
            if self.left_wall_check_ground(cd) {
                hit_left = self.left_wall_collide_ground(cd);
                if hit_left {
                    left_right |= 1;
                } else {
                    cd.env_flags &= !(collide::LEFT_WALL_MASK as i32);
                }
                x_after_left = cd.cur_pos.x;
                cd.x34_flags.b5 = true;
            }
            if self.right_wall_check_ground(cd) {
                hit_right = self.right_wall_collide_ground(cd);
                if hit_right {
                    left_right |= 2;
                } else {
                    cd.env_flags &= !(collide::RIGHT_WALL_MASK as i32);
                }
                x_after_right = cd.cur_pos.x;
                cd.x34_flags.b5 = true;
            }
            if self.left_wall_check_ground(cd) {
                hit_left |= self.left_wall_collide_ground(cd);
                if hit_left {
                    left_right |= 1;
                }
                x_after_left = cd.cur_pos.x;
                cd.x34_flags.b5 = true;
            }
            if self.right_wall_check_ground(cd) {
                hit_right |= self.right_wall_collide_ground(cd);
                if hit_right {
                    left_right |= 2;
                }
                x_after_right = cd.cur_pos.x;
                cd.x34_flags.b5 = true;
            }

            if hit_left && hit_right {
                squeeze_horizontal(cd, false, x_after_right, x_after_left);
            }

            let mut hit_ceiling = false;
            let mut hit_floor = false;
            if self.ceiling_check(cd, left_right) && self.snap_to_ceiling_ground(cd) {
                hit_ceiling = true;
                y_after_ceiling = cd.cur_pos.y;
            }
            if self.follow_floor_ground(cd) {
                self.floor_wall_corners(cd, false, 0);
                y_after_floor = cd.cur_pos.y;
                touching_floor = true;
                hit_floor = true;
                if self.ceiling_check(cd, left_right) && self.snap_to_ceiling_ground(cd) {
                    y_after_ceiling = cd.cur_pos.y;
                    hit_ceiling = true;
                }
            } else {
                let mut var_r23 = false;
                let floor_id = cd.floor.index;
                if self.line_is_active(floor_id) && self.line_get_kind(floor_id) == line_kind::FLOOR
                {
                    if flags & 1 != 0 {
                        if self.teeter_at_floor_edge(cd, floor_id) {
                            cd.x34_flags.b5 = true;
                            touching_floor = false;
                            hit_floor = true;
                            y_after_floor = cd.cur_pos.y;
                        } else {
                            var_r23 = true;
                        }
                    } else if flags & 2 != 0 {
                        if self.stop_at_floor_edge(cd, floor_id) {
                            cd.x34_flags.b5 = true;
                            touching_floor = true;
                            hit_floor = true;
                            y_after_floor = cd.cur_pos.y;
                        } else {
                            var_r23 = true;
                        }
                    } else {
                        var_r23 = true;
                    }

                    if var_r23 {
                        let old_skip = cd.floor_skip;
                        cd.floor_skip = floor_id;
                        if self.air_collide(cd, 0, None) {
                            touching_floor = true;
                        }
                        cd.floor_skip = old_skip;
                        cd.x34_flags.b5 = true;
                    }
                }
            }

            if hit_floor && hit_ceiling {
                let airborne = !touching_floor;
                squeeze_vertical(cd, airborne, y_after_ceiling, y_after_floor);
            }
            if prev_b6 == cd.x34_flags.b6 {
                break;
            }
        }

        let floor_id = cd.floor.index;
        if self.find_new_floor_below(cd, floor_id) {
            if self.snap_to_floor(cd, false) {
                self.floor_wall_corners(cd, false, 0);
            }
            cd.x34_flags.b5 = false;
            touching_floor = true;
        }

        if touching_floor {
            cd.env_flags |= collide::FLOOR_PUSH as i32;
        }
        touching_floor
    }

    /// `mpColl_8004B6D8` (retail `0x8004B6D8`, `mpcoll.c:4040`): the
    /// ceiling-hanging follow, mirror of [`Self::follow_floor_ground`].
    pub fn follow_ceiling(&mut self, cd: &mut CollData) -> bool {
        let ceiling_id = cd.ceiling.index;
        let top = Vec3::new(
            cd.cur_pos.x + cd.ecb.top.x,
            cd.cur_pos.y + cd.ecb.top.y,
            0.0,
        );
        if !self.line_is_active(cd.ceiling.index)
            || self.line_get_kind(cd.ceiling.index) != line_kind::CEILING
        {
            return false;
        }
        if let Some(p) = self.ceiling_probe(ceiling_id, &top) {
            store_surface(&mut cd.ceiling, p.flags, p.normal);
            cd.cur_pos.y += p.delta;
            cd.ceiling.index = p.line_id;
            return true;
        }
        let mut hit_wall = false;
        let mut ceiling_end = self.ceiling_get_left(ceiling_id);
        if cd.cur_pos.x < ceiling_end.x {
            let non_ceiling_id = self.line_next_non_ceiling(ceiling_id);
            if non_ceiling_id != NO_ID
                && self.line_is_active(non_ceiling_id)
                && self.line_get_kind(non_ceiling_id) & line_kind::RIGHT_WALL != 0
            {
                hit_wall = true;
            } else {
                cd.env_flags |= collide::LEFT_LEDGE_SLIP as i32;
            }
        } else {
            ceiling_end = self.ceiling_get_right(ceiling_id);
            if cd.cur_pos.x > ceiling_end.x {
                let non_ceiling_id = self.line_prev_non_ceiling(ceiling_id);
                if non_ceiling_id != NO_ID
                    && self.line_is_active(non_ceiling_id)
                    && self.line_get_kind(non_ceiling_id) & line_kind::LEFT_WALL != 0
                {
                    hit_wall = true;
                } else {
                    cd.env_flags |= collide::RIGHT_LEDGE_SLIP as i32;
                }
            }
        }
        if hit_wall {
            cd.cur_pos = ceiling_end;
            return true;
        }
        false
    }

    /// `mpColl_8004C328_Ceiling` (retail `0x8004C328`, `mpcoll.c:4289`):
    /// stop at the ceiling's end (sets `Collide_*Edge`) unless a wall there
    /// has already stopped the body.
    pub fn stop_at_ceiling_edge(&mut self, cd: &mut CollData, line_id: i32) -> bool {
        if !self.line_is_active(line_id) || self.line_get_kind(line_id) != line_kind::CEILING {
            return false;
        }
        let mut result = false;
        let mut ceiling: Option<(i32, u32, Vec3)> = None;
        let mut edge = self.ceiling_get_left(line_id);
        if cd.cur_pos.x <= edge.x {
            let p = self.ceiling_probe(line_id, &edge);
            ceiling = p.map(|p| (p.line_id, p.flags, p.normal));
            let edge_x = edge.x + 1.0;
            let edge_y = edge.y - 1.0;
            let sx = edge.x + cd.ecb.right.x - cd.ecb.top.x;
            let sy = edge.y + cd.ecb.right.y - cd.ecb.top.y;
            // make sure a wall hasn't stopped us
            if self
                .check_left_wall(edge_x, edge_y, sx, sy, cd.joint_id_skip, cd.joint_id_only)
                .is_none()
            {
                result = true;
                cd.env_flags |= collide::RIGHT_EDGE as i32;
            }
        } else {
            edge = self.ceiling_get_right(line_id);
            if cd.cur_pos.x >= edge.x {
                let p = self.ceiling_probe(line_id, &edge);
                ceiling = p.map(|p| (p.line_id, p.flags, p.normal));
                let edge_x = edge.x - 1.0;
                let edge_y = edge.y - 1.0;
                let sx = edge.x + cd.ecb.left.x - cd.ecb.top.x;
                let sy = edge.y + cd.ecb.left.y - cd.ecb.top.y;
                if self
                    .check_right_wall(edge_x, edge_y, sx, sy, cd.joint_id_skip, cd.joint_id_only)
                    .is_none()
                {
                    result = true;
                    cd.env_flags |= collide::LEFT_EDGE as i32;
                }
            }
        }
        if result {
            cd.cur_pos = edge;
            // See stop_at_floor_edge for the failed-probe case.
            let (idx, flags, normal) =
                ceiling.unwrap_or((NO_ID, cd.ceiling.flags, cd.ceiling.normal));
            cd.ceiling.index = idx;
            cd.ceiling.flags = flags;
            cd.ceiling.normal = normal;
            return true;
        }
        false
    }

    /// `mpColl_8004C534` (retail `0x8004C534`, `mpcoll.c:4354`): one step of
    /// ceiling-hanging collision. `flags` bit 1 stops at the ceiling's ends.
    pub fn ceiling_collide(&mut self, cd: &mut CollData, flags: u32) -> bool {
        let mut hit_ceiling = false;
        if self.left_wall_check_ceiling(cd) {
            self.left_wall_collide_ground(cd);
            cd.x34_flags.b5 = true;
        }
        if self.right_wall_check_ceiling(cd) {
            self.right_wall_collide_ground(cd);
            cd.x34_flags.b5 = true;
        }
        if self.follow_ceiling(cd) {
            self.ceiling_wall_corners(cd);
            hit_ceiling = true;
        } else if self.line_is_active(cd.ceiling.index) {
            let ceiling_id = cd.ceiling.index;
            if flags & 1 == 0 {
                if flags & 2 != 0 {
                    if self.stop_at_ceiling_edge(cd, ceiling_id) {
                        hit_ceiling = true;
                    } else {
                        cd.x34_flags.b5 = true;
                    }
                } else {
                    cd.x34_flags.b5 = true;
                }
            }
        }
        if hit_ceiling {
            cd.env_flags |= collide::CEILING_PUSH as i32;
        }
        hit_ceiling
    }

    // -----------------------------------------------------------------------
    // Entry points (mpColl_800471F8 .. mpColl_8004C750)
    // -----------------------------------------------------------------------

    /// `mpColl_800471F8` (retail `0x800471F8`): airborne collision with the
    /// default ECB load (6), no flags.
    pub fn air_collide_pass(&mut self, cd: &mut CollData, bones: Option<BoneLookup<'_>>) -> bool {
        coll_prev(cd);
        load_ecb_with_flags(cd, 6, bones);
        self.run(CollideMode::Air, cd, 0, true, None)
    }

    /// `mpColl_8004730C` (retail `0x8004730C`): airborne, fixed box ECB.
    pub fn air_collide_box(&mut self, cd: &mut CollData, b: &FtCollisionBox) -> bool {
        coll_prev(cd);
        load_ecb_box(cd, b);
        self.run(CollideMode::Air, cd, 0, true, None)
    }

    /// `mpColl_800473CC` (retail `0x800473CC`): airborne, can grab ledges.
    pub fn air_collide_ledge(&mut self, cd: &mut CollData, bones: Option<BoneLookup<'_>>) -> bool {
        coll_prev(cd);
        load_ecb_with_flags(cd, 6, bones);
        self.run(CollideMode::Air, cd, air_flags::CAN_GRAB_LEDGE, true, None)
    }

    /// `mpColl_800474E0` (retail `0x800474E0`): airborne, ECB load 5, can
    /// grab ledges.
    pub fn air_collide_ledge_ecb5(
        &mut self,
        cd: &mut CollData,
        bones: Option<BoneLookup<'_>>,
    ) -> bool {
        coll_prev(cd);
        load_ecb_with_flags(cd, 5, bones);
        self.run(CollideMode::Air, cd, air_flags::CAN_GRAB_LEDGE, true, None)
    }

    /// `mpColl_800475F4` (retail `0x800475F4`): airborne, fixed box, can grab
    /// ledges.
    pub fn air_collide_ledge_box(&mut self, cd: &mut CollData, b: &FtCollisionBox) -> bool {
        coll_prev(cd);
        load_ecb_box(cd, b);
        self.run(CollideMode::Air, cd, air_flags::CAN_GRAB_LEDGE, true, None)
    }

    /// `mpColl_800476B4` (retail `0x800476B4`): airborne, stay airborne, with
    /// the platform-pass filter.
    pub fn air_collide_stay_pass(
        &mut self,
        cd: &mut CollData,
        floor_cb: LineFilter<'_, '_>,
        bones: Option<BoneLookup<'_>>,
    ) -> bool {
        coll_prev(cd);
        load_ecb_with_flags(cd, 6, bones);
        self.run(
            CollideMode::Air,
            cd,
            air_flags::STAY_AIRBORNE | air_flags::PLATFORM_PASS_CALLBACK,
            true,
            floor_cb,
        )
    }

    /// `mpColl_800477E0` (retail `0x800477E0`): airborne, stay airborne.
    pub fn air_collide_stay(&mut self, cd: &mut CollData, bones: Option<BoneLookup<'_>>) -> bool {
        coll_prev(cd);
        load_ecb_with_flags(cd, 6, bones);
        self.run(CollideMode::Air, cd, air_flags::STAY_AIRBORNE, true, None)
    }

    /// `mpColl_800478F4` (retail `0x800478F4`): stay airborne, ECB load 5.
    pub fn air_collide_stay_ecb5(
        &mut self,
        cd: &mut CollData,
        bones: Option<BoneLookup<'_>>,
    ) -> bool {
        coll_prev(cd);
        load_ecb_with_flags(cd, 5, bones);
        self.run(CollideMode::Air, cd, air_flags::STAY_AIRBORNE, true, None)
    }

    /// `mpColl_80047A08` (retail `0x80047A08`): stay airborne, fixed box.
    pub fn air_collide_stay_box(&mut self, cd: &mut CollData, b: &FtCollisionBox) -> bool {
        coll_prev(cd);
        load_ecb_box(cd, b);
        self.run(CollideMode::Air, cd, air_flags::STAY_AIRBORNE, true, None)
    }

    /// `mpColl_80047AC8` (retail `0x80047AC8`): airborne with the
    /// platform-pass filter (the usual fall).
    pub fn air_collide_platform_pass(
        &mut self,
        cd: &mut CollData,
        floor_cb: LineFilter<'_, '_>,
        bones: Option<BoneLookup<'_>>,
    ) -> bool {
        coll_prev(cd);
        load_ecb_with_flags(cd, 6, bones);
        self.run(
            CollideMode::Air,
            cd,
            air_flags::PLATFORM_PASS_CALLBACK,
            true,
            floor_cb,
        )
    }

    /// `mpColl_80047BF4` (retail `0x80047BF4`): platform pass, ECB load 0xA.
    pub fn air_collide_platform_pass_ecb10(
        &mut self,
        cd: &mut CollData,
        floor_cb: LineFilter<'_, '_>,
        bones: Option<BoneLookup<'_>>,
    ) -> bool {
        coll_prev(cd);
        load_ecb_with_flags(cd, 0xA, bones);
        self.run(
            CollideMode::Air,
            cd,
            air_flags::PLATFORM_PASS_CALLBACK,
            true,
            floor_cb,
        )
    }

    /// `mpColl_80047D20` (retail `0x80047D20`): platform pass, ECB load 0x12
    /// without the locked-bottom preservation.
    pub fn air_collide_platform_pass_ecb18(
        &mut self,
        cd: &mut CollData,
        floor_cb: LineFilter<'_, '_>,
        bones: Option<BoneLookup<'_>>,
    ) -> bool {
        coll_prev(cd);
        load_ecb_0x12_unlocked(cd, bones);
        self.run(
            CollideMode::Air,
            cd,
            air_flags::PLATFORM_PASS_CALLBACK,
            true,
            floor_cb,
        )
    }

    /// `mpColl_80047E14` (retail `0x80047E14`): platform pass, can grab
    /// ledges.
    pub fn air_collide_platform_pass_ledge(
        &mut self,
        cd: &mut CollData,
        floor_cb: LineFilter<'_, '_>,
        bones: Option<BoneLookup<'_>>,
    ) -> bool {
        coll_prev(cd);
        load_ecb_with_flags(cd, 6, bones);
        self.run(
            CollideMode::Air,
            cd,
            air_flags::PLATFORM_PASS_CALLBACK | air_flags::CAN_GRAB_LEDGE,
            true,
            floor_cb,
        )
    }

    /// `mpColl_80047F40` (retail `0x80047F40`): platform pass, can grab
    /// ledges, ECB load 0xA.
    pub fn air_collide_platform_pass_ledge_ecb10(
        &mut self,
        cd: &mut CollData,
        floor_cb: LineFilter<'_, '_>,
        bones: Option<BoneLookup<'_>>,
    ) -> bool {
        coll_prev(cd);
        load_ecb_with_flags(cd, 0xA, bones);
        self.run(
            CollideMode::Air,
            cd,
            air_flags::PLATFORM_PASS_CALLBACK | air_flags::CAN_GRAB_LEDGE,
            true,
            floor_cb,
        )
    }

    /// `mpColl_8004806C` (retail `0x8004806C`): platform pass, can grab
    /// ledges, ECB load 0x12 unlocked.
    pub fn air_collide_platform_pass_ledge_ecb18(
        &mut self,
        cd: &mut CollData,
        floor_cb: LineFilter<'_, '_>,
        bones: Option<BoneLookup<'_>>,
    ) -> bool {
        coll_prev(cd);
        load_ecb_0x12_unlocked(cd, bones);
        self.run(
            CollideMode::Air,
            cd,
            air_flags::PLATFORM_PASS_CALLBACK | air_flags::CAN_GRAB_LEDGE,
            true,
            floor_cb,
        )
    }

    /// `mpColl_80048160` (retail `0x80048160`): airborne, ECB load 0xA.
    pub fn air_collide_ecb10(&mut self, cd: &mut CollData, bones: Option<BoneLookup<'_>>) -> bool {
        coll_prev(cd);
        load_ecb_with_flags(cd, 0xA, bones);
        self.run(CollideMode::Air, cd, 0, true, None)
    }

    /// `mpColl_80048274` (retail `0x80048274`): stay airborne, ECB load 0xA.
    pub fn air_collide_stay_ecb10(
        &mut self,
        cd: &mut CollData,
        bones: Option<BoneLookup<'_>>,
    ) -> bool {
        coll_prev(cd);
        load_ecb_with_flags(cd, 0xA, bones);
        self.run(CollideMode::Air, cd, air_flags::STAY_AIRBORNE, true, None)
    }

    /// `mpColl_80048388` (retail `0x80048388`): stay airborne, ECB load 0x12
    /// unlocked.
    pub fn air_collide_stay_ecb18(
        &mut self,
        cd: &mut CollData,
        bones: Option<BoneLookup<'_>>,
    ) -> bool {
        coll_prev(cd);
        load_ecb_0x12_unlocked(cd, bones);
        self.run(CollideMode::Air, cd, air_flags::STAY_AIRBORNE, true, None)
    }

    /// `mpColl_80048464` (retail `0x80048464`): can grab ledges, ECB load 0xA.
    pub fn air_collide_ledge_ecb10(
        &mut self,
        cd: &mut CollData,
        bones: Option<BoneLookup<'_>>,
    ) -> bool {
        coll_prev(cd);
        load_ecb_with_flags(cd, 0xA, bones);
        self.run(CollideMode::Air, cd, air_flags::CAN_GRAB_LEDGE, true, None)
    }

    /// `mpColl_80048578` (retail `0x80048578`): can grab ledges, ECB load
    /// 0x12 unlocked.
    pub fn air_collide_ledge_ecb18(
        &mut self,
        cd: &mut CollData,
        bones: Option<BoneLookup<'_>>,
    ) -> bool {
        coll_prev(cd);
        load_ecb_0x12_unlocked(cd, bones);
        self.run(CollideMode::Air, cd, air_flags::CAN_GRAB_LEDGE, true, None)
    }

    /// `mpColl_80048654` (retail `0x80048654`): airborne, ECB load 5.
    pub fn air_collide_ecb5(&mut self, cd: &mut CollData, bones: Option<BoneLookup<'_>>) -> bool {
        coll_prev(cd);
        load_ecb_with_flags(cd, 5, bones);
        self.run(CollideMode::Air, cd, 0, true, None)
    }

    /// `mpColl_80048768` (retail `0x80048768`): airborne, ECB load 0x12
    /// unlocked.
    pub fn air_collide_ecb18(&mut self, cd: &mut CollData, bones: Option<BoneLookup<'_>>) -> bool {
        coll_prev(cd);
        load_ecb_0x12_unlocked(cd, bones);
        self.run(CollideMode::Air, cd, 0, true, None)
    }

    /// `mpColl_80048844` (retail `0x80048844`): point collision (no ECB
    /// load).
    pub fn point_collide_pass(&mut self, cd: &mut CollData) -> bool {
        coll_prev(cd);
        self.run(CollideMode::Point, cd, 0, true, None)
    }

    /// `mpColl_8004B108` (retail `0x8004B108`): grounded collision, ECB load
    /// 5, falling off edges.
    pub fn ground_collide_pass(
        &mut self,
        cd: &mut CollData,
        bones: Option<BoneLookup<'_>>,
    ) -> bool {
        coll_prev(cd);
        load_ecb_with_flags(cd, 5, bones);
        self.run(CollideMode::Ground, cd, 0, false, None)
    }

    /// `mpColl_8004B21C` (retail `0x8004B21C`): grounded, fixed box.
    pub fn ground_collide_box(&mut self, cd: &mut CollData, b: &FtCollisionBox) -> bool {
        coll_prev(cd);
        load_ecb_box(cd, b);
        self.run(CollideMode::Ground, cd, 0, false, None)
    }

    /// `mpColl_8004B2DC` (retail `0x8004B2DC`): grounded, stopping at edges.
    pub fn ground_collide_stop_at_edge(
        &mut self,
        cd: &mut CollData,
        bones: Option<BoneLookup<'_>>,
    ) -> bool {
        coll_prev(cd);
        load_ecb_with_flags(cd, 5, bones);
        self.run(CollideMode::Ground, cd, 2, false, None)
    }

    /// `mpColl_8004B3F0` (retail `0x8004B3F0`): grounded, stopping at edges,
    /// fixed box.
    pub fn ground_collide_stop_at_edge_box(
        &mut self,
        cd: &mut CollData,
        b: &FtCollisionBox,
    ) -> bool {
        coll_prev(cd);
        load_ecb_box(cd, b);
        self.run(CollideMode::Ground, cd, 2, false, None)
    }

    /// `mpColl_8004B4B0` (retail `0x8004B4B0`): grounded, teetering at edges.
    pub fn ground_collide_teeter(
        &mut self,
        cd: &mut CollData,
        bones: Option<BoneLookup<'_>>,
    ) -> bool {
        coll_prev(cd);
        load_ecb_with_flags(cd, 5, bones);
        self.run(CollideMode::Ground, cd, 1, false, None)
    }

    /// `mpColl_8004B5C4` (retail `0x8004B5C4`): grounded, teetering, ECB
    /// load 9.
    pub fn ground_collide_teeter_ecb9(
        &mut self,
        cd: &mut CollData,
        bones: Option<BoneLookup<'_>>,
    ) -> bool {
        coll_prev(cd);
        load_ecb_with_flags(cd, 9, bones);
        self.run(CollideMode::Ground, cd, 1, false, None)
    }

    /// `mpColl_8004C750` (retail `0x8004C750`): ceiling-hanging collision,
    /// stopping at the ceiling's ends.
    pub fn ceiling_collide_pass(
        &mut self,
        cd: &mut CollData,
        bones: Option<BoneLookup<'_>>,
    ) -> bool {
        coll_prev(cd);
        load_ecb_with_flags(cd, 5, bones);
        self.run(CollideMode::Ceiling, cd, 2, false, None)
    }

    // -----------------------------------------------------------------------
    // Queries over a CollData
    // -----------------------------------------------------------------------

    /// `mpColl_8004CA6C` (retail `0x8004CA6C`): the floor's terrain speed
    /// scale, 1.0 when not on a floor.
    pub fn floor_speed_scale(&self, cd: &CollData) -> f32 {
        if cd.floor.index != NO_ID {
            terrain_speed_scale(cd.floor.flags)
        } else {
            1.0
        }
    }

    /// `mpCollGetSpeedCeiling` (retail `0x8004CAA0`).
    pub fn ceiling_speed(&self, cd: &CollData) -> Option<Vec3> {
        let top = Vec3::new(cd.ecb.top.x, cd.ecb.top.y, 0.0);
        self.line_speed(cd.ceiling.index, &top)
    }

    /// `mpCollGetSpeedLeftWall` (retail `0x8004CAE8`).
    pub fn left_wall_speed(&self, cd: &CollData) -> Option<Vec3> {
        let top = Vec3::new(cd.ecb.top.x, cd.ecb.top.y, 0.0);
        self.line_speed(cd.left_facing_wall.index, &top)
    }

    /// `mpCollGetSpeedRightWall` (retail `0x8004CB30`).
    pub fn right_wall_speed(&self, cd: &CollData) -> Option<Vec3> {
        let top = Vec3::new(cd.ecb.top.x, cd.ecb.top.y, 0.0);
        self.line_speed(cd.right_facing_wall.index, &top)
    }

    /// `mpCollGetSpeedFloor` (retail `0x8004CB78`). Note the C probes with
    /// the ECB *top* offset, as the others do.
    pub fn floor_speed(&self, cd: &CollData) -> Option<Vec3> {
        let top = Vec3::new(cd.ecb.top.x, cd.ecb.top.y, 0.0);
        self.line_speed(cd.floor.index, &top)
    }

    /// `mpColl_IsOnPlatform` (retail `0x8004CBC0`).
    pub fn is_on_platform(&self, cd: &CollData) -> bool {
        self.line_get_flags(cd.floor.index) & line_flag::PLATFORM != 0
    }

    /// `mpColl_8004D024` (retail `0x8004D024`, `mpcoll.c:4573`): is `pos`
    /// inside solid stage? Runs an airborne pass with a 10-unit fixed ECB
    /// from 3 units below and reports whether it was squeezed.
    pub fn is_point_inside_solid(&mut self, pos: &Vec3) -> bool {
        let mut cd = CollData::default();
        self.coll_data_init(&mut cd);
        cd.x34_flags.b1234 = 0;
        set_ecb_source_fixed(&mut cd, 10.0, 10.0, 10.0, 10.0);
        cd.last_pos = Vec3::new(pos.x, -3.0 + pos.y, pos.z);
        cd.cur_pos = *pos;
        cd.x130_flags |= coll_data_x130::CLEAR;
        coll_prev(&mut cd);
        load_ecb(&mut cd, None);
        self.run(CollideMode::Air, &mut cd, 0, true, None);
        cd.x34_flags.b6
    }
}

/// `MpLibGroundEnum` is re-exported for callers matching callback kinds;
/// the kinds mpcoll passes are 0 (ceiling), 1/2 (floor), and
/// [`MpLibGroundEnum::LEDGE`] (3).
#[allow(dead_code)]
const _GROUND_KIND_DOC: i32 = MpLibGroundEnum::LEDGE;
