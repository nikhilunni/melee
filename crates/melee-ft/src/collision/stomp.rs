//! ft_80081A00 (80081A00): an airborne map pass that came down onto a
//! stompable item (a shell, a Goomba) does not land: the fighter bounces
//! off it (ftCo_800C703C) and the item learns who stomped it (xCFC).
use super::ground::EnvironmentCollision;
use crate::physics::FighterPhysics;
use hsd_types::Vec3;
use melee_it::stomp::{lands_on, StompBounds, StompTarget};
use melee_types::fixed::FixedVec;

/// Stompable items one map proc may test (the scene fails closed above).
pub const STOMP_TARGETS: usize = 16;

/// What the stomp test reads and writes beside the fighter's CollData.
#[derive(Clone, Debug, Default)]
pub struct StompContext {
    /// x2223_b4: the current state never stomps (an up or down special's
    /// travel); cleared by a motion change without Ft_MF_SkipMetalB.
    pub disabled: bool,
    /// x2219_b1: out of play (dead or waiting to revive).
    pub out_of_play: bool,
    /// dmg.x1930.x0: the bounds sampled at s_link 0 (ft_800819A8).
    pub previous: StompBounds,
    /// PlCo x17C (co_attrs): the foot height above the position.
    pub foot_offset: f32,
    /// PlCo x6CC / x6D0 (ftCo_800C703C): the bounce's blend frames and
    /// upward speed.
    pub bounce_frames: i32,
    pub bounce_speed: f32,
    /// The stompable items, in the item list's order, offered by the scene
    /// for this proc.
    pub targets: FixedVec<StompTarget, STOMP_TARGETS>,
    /// Items stomped this proc and the side each was on (xCD0).
    pub stomped: FixedVec<(u32, f32), STOMP_TARGETS>,
    /// The test ran this proc (x2223_b5, set by ft_80081A00).
    pub tested: bool,
    /// The fighter was airborne when its map proc began.
    pub airborne_before: bool,
}

/// dmg.x1930.xC: the bounds after the pass.
pub fn current_bounds(environment: &EnvironmentCollision, position_y: f32) -> StompBounds {
    let cd = &environment.data;
    StompBounds {
        left: cd.ecb.left.x + cd.cur_pos.x,
        right: cd.ecb.right.x + cd.cur_pos.x,
        bottom: position_y + environment.stomp.foot_offset,
    }
}

/// ft_80081A00 (80081A00) after an airborne pass: unless the fighter is
/// out of play or its state forbids it, every offered item whose box the
/// fighter's feet came down onto (lb_80014638) is stomped, from the side
/// its position is on; any stomp sets the bounce (ftCo_800C703C) and the
/// pass does not land. Returns whether it stomped.
pub fn land_on_items(state: &mut FighterPhysics, environment: &mut EnvironmentCollision) -> bool {
    let stomp = &environment.stomp;
    if stomp.out_of_play || stomp.disabled {
        return false;
    }
    environment.collision_flag = true;
    let current = current_bounds(environment, state.position.y);
    let previous = stomp.previous;
    let position_x = state.position.x;
    let stomp = &mut environment.stomp;
    stomp.tested = true;
    let mut any = false;
    for target in stomp.targets.iter() {
        if lands_on(previous, current, target.stomp_box) {
            let side = if target.position_x > position_x {
                -1.0
            } else {
                1.0
            };
            stomp.stomped.push((target.item, side));
            any = true;
        }
    }
    if any {
        // ftCo_800C703C (800C703C).
        state.velocity_blend.origin = Vec3::new(state.self_velocity.x, stomp.bounce_speed, 0.0);
        state.velocity_blend.duration = stomp.bounce_frames;
        state.velocity_blend.remaining = stomp.bounce_frames;
    }
    any
}
