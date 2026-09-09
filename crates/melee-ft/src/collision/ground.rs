use super::ecb::EcbPose;
use crate::physics::FighterPhysics;
use hsd_anim::jobj::{JObjId, JObjTree};
use melee_mp::CollMap;
use melee_types::{
    mp::{coll_data_x130, collide, CollData},
    GroundOrAir,
};

/// Result of ft_80084280's support test and ftCo_8009A3C8. The action-state
/// owner must enter Ottotto/Fall before another Wait tick; this layer does
/// not pretend to attach the next state's animation or run its callbacks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaitGroundResult {
    Supported,
    EnterTeeter,
    EnterFall,
}

#[derive(Clone, Debug)]
pub struct EnvironmentCollision {
    /// `x6F0_collData`, Fighter +0x6F0; mp owns its ECB and surface fields.
    pub data: CollData,
    /// `ecb_lock`, Fighter +0x88C.
    pub lock_frames: i32,
    /// `x2223_b5`, Fighter +0x2223 mask 0x04, cleared by procMap.
    pub collision_flag: bool,
    /// `x2228_b2`, Fighter +0x2228 mask 0x20, suppresses Ottotto.
    pub teeter_disabled: bool,
}
impl EnvironmentCollision {
    pub fn new(data: CollData) -> Self {
        Self {
            data,
            lock_frames: 0,
            collision_flag: false,
            teeter_disabled: false,
        }
    }
}

/// `ftCo_Wait_Coll` (0x8008A678) -> `ft_80084280` (ft_081B.c:1062-1099).
/// A backward player nudge selects ft_800827A0 -> mpColl_8004B2DC.
pub fn collide_wait(
    state: &mut FighterPhysics,
    environment: &mut EnvironmentCollision,
    map: &mut CollMap,
    pose: &EcbPose,
    stick_x: f32,
) -> WaitGroundResult {
    assert_eq!(state.ground_or_air, GroundOrAir::Ground);
    let cd = &mut environment.data;
    cd.last_pos = cd.cur_pos;
    cd.cur_pos = state.position;
    // retail 0x800842A8..: isolated fmuls and comparison, no fused sites.
    let supported = if state.player_nudge.x != 0.0 && state.player_nudge.x * state.facing < 0.0 {
        map.ground_collide_stop_at_edge(cd, Some(&|i| pose.position(i)))
    } else {
        cd.lstick_x = stick_x;
        melee_mp::set_facing_dir(cd, if state.facing > 0.0 { 1 } else { -1 });
        map.ground_collide_teeter(cd, Some(&|i| pose.position(i)))
    };
    state.position = cd.cur_pos;
    if supported {
        WaitGroundResult::Supported
    } else if cd.env_flags as u32 & collide::EDGE != 0 && !environment.teeter_disabled {
        WaitGroundResult::EnterTeeter
    } else {
        WaitGroundResult::EnterFall
    }
}

/// `Fighter_procMap` (0x8006C27C, fighter.c:2476-2516), s_link 6.
/// Root translation is written both BEFORE bone sampling and AFTER correction.
pub fn map_wait(
    state: &mut FighterPhysics,
    environment: &mut EnvironmentCollision,
    map: &mut CollMap,
    tree: &mut JObjTree,
    root: JObjId,
    stick_x: f32,
) -> WaitGroundResult {
    if environment.lock_frames != 0 {
        environment.lock_frames = environment.lock_frames.wrapping_sub(1);
        if environment.lock_frames == 0 {
            environment.data.x130_flags &= !coll_data_x130::LOCKED;
        }
    }
    environment.collision_flag = false;
    tree.set_translate(root, &state.position);
    let pose = EcbPose::read(tree, root, &environment.data);
    let result = collide_wait(state, environment, map, &pose, stick_x);
    tree.set_translate(root, &state.position);
    result
}

/// ft_80083F88 (0x80083F88) -> ft_80082708 (0x80082708):
/// Squat/Turn use ordinary ground collision, without Wait's teeter predicate.
pub fn map_ground_action(
    state: &mut FighterPhysics,
    environment: &mut EnvironmentCollision,
    map: &mut CollMap,
    tree: &mut JObjTree,
    root: JObjId,
    _stick_x: f32,
) -> WaitGroundResult {
    super::air::begin_map(state, environment, tree, root);
    let pose = EcbPose::read(tree, root, &environment.data);
    let cd = &mut environment.data;
    cd.last_pos = cd.cur_pos;
    cd.cur_pos = state.position;
    let supported = map.ground_collide_pass(cd, Some(&|i| pose.position(i)));
    state.position = cd.cur_pos;
    tree.set_translate(root, &state.position);
    if supported {
        WaitGroundResult::Supported
    } else {
        WaitGroundResult::EnterFall
    }
}

/// ft_80084104 -> ft_800827A0 (0x800827A0): escape stops at the floor edge.
pub fn map_escape(
    state: &mut FighterPhysics,
    environment: &mut EnvironmentCollision,
    map: &mut CollMap,
    tree: &mut JObjTree,
    root: JObjId,
    _stick_x: f32,
) -> WaitGroundResult {
    super::air::begin_map(state, environment, tree, root);
    let pose = EcbPose::read(tree, root, &environment.data);
    let cd = &mut environment.data;
    cd.last_pos = cd.cur_pos;
    cd.cur_pos = state.position;
    let supported = map.ground_collide_stop_at_edge(cd, Some(&|i| pose.position(i)));
    state.position = cd.cur_pos;
    tree.set_translate(root, &state.position);
    if supported {
        WaitGroundResult::Supported
    } else {
        WaitGroundResult::EnterFall
    }
}
