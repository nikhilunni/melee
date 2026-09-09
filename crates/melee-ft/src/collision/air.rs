//! ft_081B.c fighter wrappers around melee-mp's airborne collision.
use super::{ecb::EcbPose, ground::EnvironmentCollision};
use crate::physics::FighterPhysics;
use hsd_anim::jobj::{JObjId, JObjTree};
use melee_mp::{set_facing_dir, CollMap};
use melee_types::mp::{coll_data_x130, FtCollisionBox};

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
/// ft_800831CC -> ft_80083090_inline -> mpColl_80047E14.
/// Neutral stick accepts solid floors and platforms (ftCo_80096CC8).
pub fn collide_fall(
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
        map.air_collide_platform_pass_ledge(
            cd,
            Some(&mut |line| line != -1),
            Some(&|i| pose.position(i)),
        )
    } else {
        map.air_collide_platform_pass(
            cd,
            Some(&mut |line| line != -1),
            Some(&|i| pose.position(i)),
        )
    };
    state.position = cd.cur_pos;
    tree.set_translate(root, &state.position);
    landed
}
/// ft_80083E64 -> mpColl_8004730C, fixed entry ECB (bottom follows trophy).
pub fn collide_entry(
    state: &mut FighterPhysics,
    environment: &mut EnvironmentCollision,
    map: &mut CollMap,
    mut ecb: FtCollisionBox,
) -> bool {
    if state.facing < 0.0 {
        let left = ecb.left.x;
        ecb.left.x = -ecb.right.x;
        ecb.right.x = -left;
    }
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
