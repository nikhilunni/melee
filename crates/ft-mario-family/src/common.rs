//! Motion helpers shared by Mario's specials.
use melee_ft::{
    collision::{air, ground},
    fighter::{
        assets::{FighterAssets, Result},
        state::{CollisionPhase, MotionRow},
        ActionId, Fighter, MotionEntryFlags,
    },
};
use melee_types::CommonMotionState;

/// ftMr_SM_SpecialN = ftCo_SM_Count: row 343 plays submotion 295.
const FIRST_SPECIAL_ACTION: u16 = 343;
const FIRST_SPECIAL_ANIMATION: i32 = 295;

/// A ported row of ftMr_Init_MotionStateTable (343..350).
pub const fn row(
    action: ActionId,
    anim: melee_ft::fighter::state::AnimFn,
    iasa: melee_ft::fighter::state::InputFn,
    physics: melee_ft::fighter::state::PhysicsFn,
    collision: melee_ft::fighter::state::CollisionFn,
) -> MotionRow {
    MotionRow {
        action,
        id: CommonMotionState::None,
        animation: FIRST_SPECIAL_ANIMATION + (action.0 - FIRST_SPECIAL_ACTION) as i32,
        anim,
        iasa,
        physics,
        collision,
        camera: melee_ft::fighter::state::callbacks::camera::follow_fighter,
        implemented: true,
    }
}

/// ft_8008A2BC on the ground, ftCo_Fall_Enter in the air.
pub fn finish(f: &mut Fighter, assets: &FighterAssets, air: bool) -> Result<()> {
    let state = if air {
        CommonMotionState::Fall
    } else {
        CommonMotionState::Wait
    };
    f.change_motion_state(state.into(), assets)
}

/// ft_80082708 (80082708): ordinary ground collision that lets the fighter
/// walk off the floor's edge. True while it is still supported.
pub fn stays_grounded(f: &mut Fighter, p: &mut CollisionPhase<'_>) -> bool {
    let c = &mut f.core;
    matches!(
        ground::map_ground_action(
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

/// ft_800827A0 (800827A0): ground collision that stops at the floor's
/// edge. True while the fighter is still supported.
pub fn stays_on_edge(f: &mut Fighter, p: &mut CollisionPhase<'_>) -> bool {
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

/// ftCommon_GroundToAirStateChange: ftCommon_8007D5D4, then the airborne
/// row at the current frame with `flags`.
pub fn ground_to_air(
    f: &mut Fighter,
    state: ActionId,
    assets: &FighterAssets,
    flags: MotionEntryFlags,
) -> Result<()> {
    f.leave_ground();
    let frame = f.animation.frame;
    f.change_motion_state_with_flags(state, assets, flags, frame, 1.0)
}

/// ftCommon_AirToGroundStateChange: ftCommon_8007D7FC, then the grounded
/// row at the current frame with `flags`.
pub fn air_to_ground(
    f: &mut Fighter,
    state: ActionId,
    assets: &FighterAssets,
    flags: MotionEntryFlags,
) -> Result<()> {
    f.land();
    let frame = f.animation.frame;
    f.change_motion_state_with_flags(state, assets, flags, frame, 1.0)
}
