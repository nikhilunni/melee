//! ft_081B.c fighter wrappers around melee-mp's airborne collision.
use super::{ecb::EcbPose, ground::EnvironmentCollision};
use crate::physics::FighterPhysics;
use hsd_anim::jobj::{JObjId, JObjTree};
use melee_mp::{set_facing_dir, CollMap};
use melee_types::mp::{coll_data_x130, line_flag, FtCollisionBox};

/// Fighter_procMap (0x8006C27C): unlock ECB before the state's callback.
pub fn begin_map(
    state: &FighterPhysics,
    environment: &mut EnvironmentCollision,
    tree: &mut JObjTree,
    root: JObjId,
) {
    if environment.lock_frames != 0 {
        environment.lock_frames = environment.lock_frames.wrapping_sub(1);
        if environment.lock_frames == 0 {
            environment.data.x130_flags &= !coll_data_x130::LOCKED;
        }
    }
    environment.collision_flag = false;
    tree.set_translate(root, &state.position);
}
/// ftCo_80096CC8 (0x80096CC8, ftCo_FallSpecial.c:134): the floor filter
/// every ft_80083090-family collision passes. Any real line is floor,
/// except that a platform (mpLineGetFlags & LINE_FLAG_PLATFORM) is floor only
/// while the stick's y (fp+0x624) is above PlCo +25C (80096D00 fcmpo; ble
/// skips it).
pub fn platform_floor_filter(
    stick_y: f32,
    drop_threshold: f32,
) -> impl FnMut(&CollMap, i32) -> bool {
    let platforms_are_floor = stick_y > drop_threshold;
    move |map, line| {
        line != -1 && (map.line_get_flags(line) & line_flag::PLATFORM == 0 || platforms_are_floor)
    }
}

/// ft_800831CC -> ft_80083090_inline -> mpColl_80047E14, floor filter
/// ftCo_80096CC8 (see [`platform_floor_filter`]).
#[allow(clippy::too_many_arguments)]
pub fn collide_fall_filtered(
    state: &mut FighterPhysics,
    environment: &mut EnvironmentCollision,
    map: &mut CollMap,
    tree: &mut JObjTree,
    root: JObjId,
    can_grab_ledge: bool,
    stick_y: f32,
    drop_threshold: f32,
) -> bool {
    let cd = &mut environment.data;
    cd.last_pos = cd.cur_pos;
    cd.cur_pos = state.position;
    set_facing_dir(cd, if state.facing < 0.0 { -1 } else { 1 });
    let pose = EcbPose::read(tree, root, cd);
    let mut filter = platform_floor_filter(stick_y, drop_threshold);
    let landed = if can_grab_ledge {
        map.air_collide_platform_pass_ledge(cd, Some(&mut filter), Some(&|i| pose.position(i)))
    } else {
        map.air_collide_platform_pass(cd, Some(&mut filter), Some(&|i| pose.position(i)))
    };
    state.position = cd.cur_pos;
    tree.set_translate(root, &state.position);
    landed
}

/// [`collide_fall_filtered`] with every platform solid, as though the stick
/// were neutral.
pub fn collide_fall(
    state: &mut FighterPhysics,
    environment: &mut EnvironmentCollision,
    map: &mut CollMap,
    tree: &mut JObjTree,
    root: JObjId,
    can_grab_ledge: bool,
) -> bool {
    let (stick_y, never) = (0.0, f32::NEG_INFINITY);
    let (s, e, t) = (state, environment, tree);
    collide_fall_filtered(s, e, map, t, root, can_grab_ledge, stick_y, never)
}
/// ft_80083E64 -> mpColl_8004730C, fixed entry ECB (bottom follows trophy).
pub fn collide_entry(
    state: &mut FighterPhysics,
    environment: &mut EnvironmentCollision,
    map: &mut CollMap,
    ecb: FtCollisionBox,
) -> bool {
    let ecb = facing_box(ecb, state.facing);
    let cd = &mut environment.data;
    cd.last_pos = cd.cur_pos;
    cd.cur_pos = state.position;
    let landed = if state.ground_or_air == melee_types::GroundOrAir::Air {
        map.air_collide_box(cd, &ecb)
    } else {
        map.ground_collide_box(cd, &ecb)
    };
    state.position = cd.cur_pos;
    landed
}

/// ft_80082838 (80082838): a fixed box mirrored for a left-facing fighter.
pub fn facing_box(mut ecb: FtCollisionBox, facing: f32) -> FtCollisionBox {
    if facing < 0.0 {
        let left = ecb.left.x;
        ecb.left.x = -ecb.right.x;
        ecb.right.x = -left;
    }
    ecb
}

/// ft_800824A0 (800824A0): an airborne pass with a fixed box
/// (mpColl_8004730C). The item landing (ft_80081A00) is not modelled.
pub fn collide_box(
    state: &mut FighterPhysics,
    environment: &mut EnvironmentCollision,
    map: &mut CollMap,
    tree: &mut JObjTree,
    root: JObjId,
    ecb: FtCollisionBox,
) -> bool {
    let ecb = facing_box(ecb, state.facing);
    let cd = &mut environment.data;
    cd.last_pos = cd.cur_pos;
    cd.cur_pos = state.position;
    let landed = map.air_collide_box(cd, &ecb);
    state.position = cd.cur_pos;
    tree.set_translate(root, &state.position);
    landed
}

/// ft_80082638 (80082638): an airborne pass with a fixed box that stays
/// airborne (mpColl_80047A08); true when it touches a floor
/// (Collide_FloorMask). The item landing (ft_80081A00) is not modelled.
pub fn collide_stay_box(
    state: &mut FighterPhysics,
    environment: &mut EnvironmentCollision,
    map: &mut CollMap,
    tree: &mut JObjTree,
    root: JObjId,
    ecb: FtCollisionBox,
) -> bool {
    let ecb = facing_box(ecb, state.facing);
    let cd = &mut environment.data;
    cd.last_pos = cd.cur_pos;
    cd.cur_pos = state.position;
    map.air_collide_stay_box(cd, &ecb);
    state.position = cd.cur_pos;
    tree.set_translate(root, &state.position);
    cd.env_flags as u32 & melee_types::mp::collide::FLOOR_MASK != 0
}

/// ft_8008239C (8008239C): an airborne pass with a fixed box that, when
/// `can_grab_ledge` (no ledge cooldown), catches ledges on the `direction`
/// side (mpColl_800475F4); otherwise mpColl_8004730C. The item landing
/// (ft_80081A00) is not modelled.
#[allow(clippy::too_many_arguments)]
pub fn collide_box_catching_ledges(
    state: &mut FighterPhysics,
    environment: &mut EnvironmentCollision,
    map: &mut CollMap,
    tree: &mut JObjTree,
    root: JObjId,
    ecb: FtCollisionBox,
    direction: i32,
    can_grab_ledge: bool,
) -> bool {
    let ecb = facing_box(ecb, state.facing);
    let cd = &mut environment.data;
    cd.last_pos = cd.cur_pos;
    cd.cur_pos = state.position;
    let landed = if can_grab_ledge {
        set_facing_dir(cd, direction);
        map.air_collide_ledge_box(cd, &ecb)
    } else {
        map.air_collide_box(cd, &ecb)
    };
    state.position = cd.cur_pos;
    tree.set_translate(root, &state.position);
    landed
}

/// ft_80082C74 -> ft_80081D0C (80081D0C): air dodge uses ordinary airborne
/// collision, without ledge grabs or platform-drop filtering.
pub fn collide_air_dodge(
    state: &mut FighterPhysics,
    environment: &mut EnvironmentCollision,
    map: &mut CollMap,
    tree: &mut JObjTree,
    root: JObjId,
) -> bool {
    let cd = &mut environment.data;
    cd.last_pos = cd.cur_pos;
    cd.cur_pos = state.position;
    let pose = EcbPose::read(tree, root, cd);
    let landed = map.air_collide_pass(cd, Some(&|i| pose.position(i)));
    state.position = cd.cur_pos;
    tree.set_translate(root, &state.position);
    landed
}

/// ftCo_Pass_Coll -> ft_80082F28 -> ft_CheckGroundAndLedge (0x800822A4).
/// The floor-skip line is retained; this path has no stick-down filter.
pub fn collide_pass(
    state: &mut FighterPhysics,
    environment: &mut EnvironmentCollision,
    map: &mut CollMap,
    tree: &mut JObjTree,
    root: JObjId,
    can_grab_ledge: bool,
) -> bool {
    let cd = &mut environment.data;
    cd.last_pos = cd.cur_pos;
    cd.cur_pos = state.position;
    set_facing_dir(cd, if state.facing < 0.0 { -1 } else { 1 });
    let pose = EcbPose::read(tree, root, cd);
    let landed = if can_grab_ledge {
        map.air_collide_ledge(cd, Some(&|i| pose.position(i)))
    } else {
        map.air_collide_pass(cd, Some(&|i| pose.position(i)))
    };
    state.position = cd.cur_pos;
    tree.set_translate(root, &state.position);
    landed
}
