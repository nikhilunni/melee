//! Motion helpers shared by Jigglypuff's specials.
use melee_ft::{
    collision::{air, ecb::EcbPose, ground},
    fighter::{
        assets::{FighterAssets, Result},
        state::CollisionPhase,
        ActionId, Fighter,
    },
};
use melee_types::CommonMotionState;

/// ft_8008A2BC on the ground, ftCo_Fall_Enter in the air.
pub fn finish(f: &mut Fighter, assets: &FighterAssets, air: bool) -> Result<()> {
    let state = if air {
        CommonMotionState::Fall
    } else {
        CommonMotionState::Wait
    };
    f.change_motion_state(state.into(), assets)
}

/// The `-1 == fp->facing_dir` choice between a special's left and right rows.
pub fn by_facing(f: &Fighter, left: ActionId, right: ActionId) -> ActionId {
    if f.physics.facing == -1.0 {
        left
    } else {
        right
    }
}

/// ft_800827A0 (800827A0): ground collision that stops at the floor's
/// edge. True while the fighter is still supported.
pub fn stays_grounded(f: &mut Fighter, p: &mut CollisionPhase<'_>) -> bool {
    let c = &mut f.core;
    matches!(
        ground::map_escape(
            &mut c.physics,
            &mut c.collision,
            p.map,
            &mut c.skeleton,
            c.animation.root,
            c.input.current.stick.x,
        ),
        ground::WaitGroundResult::Supported
    )
}

/// ft_80081D0C (80081D0C): ordinary airborne collision; true on landing.
pub fn lands(f: &mut Fighter, p: &mut CollisionPhase<'_>) -> bool {
    let c = &mut f.core;
    air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    air::collide_air_dodge(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
    )
}

/// ft_CheckGroundAndLedge (800822A4) with CLIFFCATCH_BOTH (0): true on
/// landing; with no ledge cooldown the ledges on either side are tested.
pub fn lands_or_catches_ledge(f: &mut Fighter, p: &mut CollisionPhase<'_>) -> bool {
    let c = &mut f.core;
    air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    let cd = &mut c.collision.data;
    cd.last_pos = cd.cur_pos;
    cd.cur_pos = c.physics.position;
    let pose = EcbPose::read(&mut c.skeleton, c.animation.root, cd);
    let landed = if c.status.ledge_cooldown == 0 {
        melee_mp::set_facing_dir(cd, 0);
        p.map.air_collide_ledge(cd, Some(&|i| pose.position(i)))
    } else {
        p.map.air_collide_pass(cd, Some(&|i| pose.position(i)))
    };
    c.physics.position = cd.cur_pos;
    c.skeleton
        .set_translate(c.animation.root, &c.physics.position);
    landed
}
