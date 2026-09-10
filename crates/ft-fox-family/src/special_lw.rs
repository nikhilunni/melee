//! Reflector, ftfoxspeciallw.c (800E83E0..800E9DF8).
use crate::{
    special_s::{air_friction, finish_air, row},
    FamilyState as S, FoxFamily,
};
use melee_coll::defense::ReflectDescriptor;
use melee_ef::request::EffectRequest;
use melee_ft::{
    anim::WaitChoice,
    desc::fox_attributes::ReflectionAttributes,
    fighter::{
        assets::{FighterAssets, Result},
        state::{self, callbacks, AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        Fighter, MotionRow,
    },
    input::{Buttons, WaitContext, WaitPredicate, WaitTransition},
    physics::airborne,
};
use melee_types::{CommonMotionState, GroundOrAir};

#[derive(Clone, Debug, Default)]
pub struct SpecialLw {
    /// Fighter +2340: minimum hold countdown.
    pub release_lag: i32,
    /// Fighter +2344: turn countdown.
    pub turn_frames: i32,
    /// Fighter +2348: B has been released; this latch never re-arms in the loop.
    pub released: bool,
    /// Fighter +234C: gravity countdown.
    pub gravity_delay: i32,
    /// ftColl_CreateReflectHit's owned defense data; no item-kind selection.
    pub reflector: Option<ReflectDescriptor>,
    /// accessory4_cb, cleared after the synchronous effect is requested.
    pub pending_effect: Option<u16>,
}

/// ftColl_CreateReflectHit (8007B240), literal descriptor transfer.
pub fn descriptor(a: &ReflectionAttributes) -> ReflectDescriptor {
    ReflectDescriptor {
        bone: a.joint as usize,
        maximum_damage: a.max_damage,
        offset: a.offset,
        radius: a.size,
        damage_multiplier: a.damage_multiplier,
        speed_multiplier: a.speed_multiplier,
        preserve_owner: a.skip_ownership_change != 0,
    }
}

/// ftFx_SpecialLwHit_Enter (800E9A68), including lb_800119DC's impulse data.
#[derive(Clone, Copy, Debug)]
pub struct ReflectReaction {
    pub action: S,
    pub facing: f32,
    pub impulse_ticks: i32,
    pub impulse_scale: f32,
    pub impulse_decay: f32,
    pub impulse_angle: f32,
}
pub fn reflection_reaction(airborne: bool, direction: f32) -> ReflectReaction {
    ReflectReaction {
        action: if airborne {
            S::SpecialAirLwHit
        } else {
            S::SpecialLwHit
        },
        facing: direction,
        impulse_ticks: 120,
        impulse_scale: 3.0,
        impulse_decay: 0.1,
        impulse_angle: (std::f64::consts::PI / 3.0) as f32,
    }
}

pub const fn rows<C: FoxFamily>() -> [MotionRow; 10] {
    let mut turn = state::unimplemented_row();
    turn.action = melee_ft::fighter::ActionId(S::SpecialLwTurn as u16);
    let mut air_turn = state::unimplemented_row();
    air_turn.action = melee_ft::fighter::ActionId(S::SpecialAirLwTurn as u16);
    [
        row(
            S::SpecialLwStart,
            313,
            start::<C>,
            start_input,
            callbacks::physics::guard_on,
            ground_collision,
        ),
        row(
            S::SpecialLwLoop,
            314,
            hold::<C>,
            loop_input::<C>,
            callbacks::physics::guard_on,
            ground_collision,
        ),
        row(
            S::SpecialLwHit,
            315,
            hit::<C>,
            no_input,
            callbacks::physics::guard_on,
            ground_collision,
        ),
        row(
            S::SpecialLwEnd,
            316,
            end::<C>,
            no_input,
            callbacks::physics::guard_on,
            ground_collision,
        ),
        turn,
        row(
            S::SpecialAirLwStart,
            317,
            start::<C>,
            no_input,
            air_physics::<C>,
            air_collision::<C>,
        ),
        row(
            S::SpecialAirLwLoop,
            318,
            hold::<C>,
            loop_input::<C>,
            air_physics::<C>,
            air_collision::<C>,
        ),
        row(
            S::SpecialAirLwHit,
            319,
            hit::<C>,
            no_input,
            air_physics::<C>,
            air_collision::<C>,
        ),
        row(
            S::SpecialAirLwEnd,
            320,
            end::<C>,
            no_input,
            air_physics::<C>,
            air_collision::<C>,
        ),
        air_turn,
    ]
}
fn no_input(_: &mut Fighter, _: InputPhase<'_>) {}
/// ftFx_SpecialLw_Enter / SpecialAirLw_Enter (800E8560 / 800E85EC).
pub fn enter<C: FoxFamily>(f: &mut Fighter, air: bool, assets: &FighterAssets) {
    let a = &f.character.get::<C>().attributes().reflector;
    let (lag, delay, divisor) = (
        gekko_math::msl::fctiwz(a.release_lag),
        a.gravity_delay,
        a.momentum_preserve_x,
    );
    if air {
        // ftFx_SpecialAirLw_Enter: fdivs, no multiply-add.
        f.physics.self_velocity.y = 0.0;
        f.physics.self_velocity.x /= divisor;
    }
    f.change_motion_state(
        (if air {
            S::SpecialAirLwStart
        } else {
            S::SpecialLwStart
        })
        .into(),
        assets,
    )
    .expect("Reflector start assets");
    f.step_animation(assets);
    *f.character.get_mut::<C>().special_lw() = SpecialLw {
        release_lag: lag,
        gravity_delay: delay,
        pending_effect: Some(0x489),
        ..Default::default()
    };
    f.commands.variables[1] = 4;
}
fn released<C: FoxFamily>(f: &mut Fighter, count_down: bool) -> bool {
    let held = f.input.current.held.intersects(Buttons::B);
    let s = f.character.get_mut::<C>().special_lw();
    s.released |= !held;
    if count_down && s.release_lag > 0 {
        s.release_lag -= 1;
    }
    s.released && s.release_lag <= 0
}
fn create_bubble<C: FoxFamily>(f: &mut Fighter) {
    let bubble = descriptor(&f.character.get::<C>().attributes().reflector.reflection);
    f.character.get_mut::<C>().special_lw().reflector = Some(bubble);
}
fn destroy_effect(f: &mut Fighter) {
    f.effect_state.destroy_on_state_change = false;
    f.effects.push(EffectRequest::DestroyOwned);
}
fn enter_loop<C: FoxFamily>(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    // Ft_MF_KeepGfx: the startup caller explicitly destroys after this transition.
    let owned = f.effect_state.destroy_on_state_change;
    f.effect_state.destroy_on_state_change = false;
    f.change_motion_state(
        (if f.physics.ground_or_air == GroundOrAir::Air {
            S::SpecialAirLwLoop
        } else {
            S::SpecialLwLoop
        })
        .into(),
        assets,
    )?;
    f.effect_state.destroy_on_state_change = owned;
    create_bubble::<C>(f);
    Ok(())
}
fn enter_end<C: FoxFamily>(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    f.character.get_mut::<C>().special_lw().reflector = None;
    f.change_motion_state(
        (if f.physics.ground_or_air == GroundOrAir::Air {
            S::SpecialAirLwEnd
        } else {
            S::SpecialLwEnd
        })
        .into(),
        assets,
    )
}
fn start<C: FoxFamily>(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    released::<C>(f, false);
    if !f.animation.frames_remaining(&f.skeleton) {
        enter_loop::<C>(f, p.assets)?;
        destroy_effect(f);
        f.character.get_mut::<C>().special_lw().pending_effect = Some(0x488);
    }
    Ok(None)
}
fn hold<C: FoxFamily>(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if released::<C>(f, true) {
        enter_end::<C>(f, p.assets)?;
    }
    Ok(None)
}
fn hit<C: FoxFamily>(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let release = released::<C>(f, true);
    if !f.animation.frames_remaining(&f.skeleton) {
        if release {
            enter_end::<C>(f, p.assets)?;
        } else {
            enter_loop::<C>(f, p.assets)?;
            destroy_effect(f);
            f.character.get_mut::<C>().special_lw().pending_effect = Some(0x488);
        }
    }
    Ok(None)
}
fn end<C: FoxFamily>(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        destroy_effect(f);
        f.character.get_mut::<C>().special_lw().reflector = None;
        f.change_motion_state(
            (if f.physics.ground_or_air == GroundOrAir::Ground {
                CommonMotionState::Wait
            } else {
                CommonMotionState::Fall
            })
            .into(),
            p.assets,
        )?;
    }
    Ok(None)
}
fn start_input(f: &mut Fighter, p: InputPhase<'_>) {
    if f.input.current.stick.y <= -p.assets.movement.platform_drop_threshold
        && i32::from(f.input.vertical.tilt) < p.assets.movement.platform_drop_window
        && f.collision.data.floor.flags & melee_types::mp::line_flag::PLATFORM != 0
    {
        unimplemented!("ftFx_SpecialLwStart_CheckPass: preserved platform-drop transition");
    }
}
fn loop_input<C: FoxFamily>(f: &mut Fighter, p: InputPhase<'_>) {
    let context = WaitContext {
        facing: f.physics.facing,
        ..Default::default()
    };
    if melee_ft::input::iasa::evaluate(WaitPredicate::Turn, &f.input, &p.assets.input, &context)
        == WaitTransition::Turn
    {
        unimplemented!("ftFx_SpecialLwTurn_Check: reflector turn");
    }
    if f.physics.ground_or_air == GroundOrAir::Ground {
        if melee_ft::input::human::jump_input(&f.input, &p.assets.input) {
            f.character.get_mut::<C>().special_lw().reflector = None;
            f.enter_knee_bend(p.assets).expect("Reflector jump cancel");
        } else {
            start_input(f, p);
        }
    } else if f.input.pressed.intersects(Buttons::XY)
        && i32::from(f.physics.jumps_used) < f.attributes.jumping.max_jumps
    {
        unimplemented!("ftFx_SpecialAirLwLoop_IASA: aerial jump cancel");
    }
}
fn air_physics<C: FoxFamily>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let s = f.character.get_mut::<C>().special_lw();
    let falling = s.gravity_delay == 0;
    if !falling {
        s.gravity_delay -= 1;
    }
    if falling {
        let gravity = f.character.get::<C>().attributes().reflector.fall_accel;
        f.physics.self_velocity.y = airborne::gravity(
            f.physics.self_velocity.y,
            gravity,
            f.attributes.air.terminal_velocity,
        );
    }
    air_friction(f, f.attributes.air.aerial_friction);
    finish_air(f, p);
}
fn ground_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    if !crate::special_hi::grounded_support(f, p) {
        unimplemented!("ftFx_SpecialLw ground-to-air preserved transition");
    }
    Ok(())
}
fn air_collision<C: FoxFamily>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    let c = &mut f.core;
    melee_ft::collision::air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    if melee_ft::collision::air::collide_fall(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        false,
    ) {
        let state = match f.motion_state.action.0 {
            x if x == S::SpecialAirLwStart as u16 => S::SpecialLwStart,
            x if x == S::SpecialAirLwLoop as u16 => S::SpecialLwLoop,
            x if x == S::SpecialAirLwHit as u16 => S::SpecialLwHit,
            x if x == S::SpecialAirLwEnd as u16 => S::SpecialLwEnd,
            _ => unreachable!("installed Reflector collision row"),
        };
        let frame = f.animation.frame;
        let keep_effect = state != S::SpecialLwEnd;
        let owned = f.effect_state.destroy_on_state_change;
        if keep_effect {
            f.effect_state.destroy_on_state_change = false;
        }
        f.land();
        f.change_motion_state_at(
            state.into(),
            p.assets.expect("Reflector landing assets"),
            frame,
        )?;
        if keep_effect {
            f.effect_state.destroy_on_state_change = owned;
        }
        // ftCommon_ClampAirDrift, separate clamp of self_vel.x after motion entry.
        let max = f.attributes.air.air_drift_max;
        f.physics.self_velocity.x = f.physics.self_velocity.x.clamp(-max, max);
        if matches!(state, S::SpecialLwLoop | S::SpecialLwHit) {
            create_bubble::<C>(f);
        }
    }
    Ok(())
}
/// Reflect callback; impulse data is returned to the caller's lb effect boundary.
pub fn on_reflect<C: FoxFamily>(
    f: &mut Fighter,
    direction: f32,
    assets: &FighterAssets,
) -> Result<ReflectReaction> {
    let reaction = reflection_reaction(f.physics.ground_or_air == GroundOrAir::Air, direction);
    f.physics.facing = reaction.facing;
    f.change_motion_state(reaction.action.into(), assets)?;
    create_bubble::<C>(f);
    f.character.get_mut::<C>().special_lw().pending_effect = Some(0x48A);
    Ok(reaction)
}
pub fn accessory<C: FoxFamily>(f: &mut Fighter, _: &FighterAssets) {
    if let Some(id) = f
        .character
        .get_mut::<C>()
        .special_lw()
        .pending_effect
        .take()
    {
        // Retail uses literal joint slot HipN (4), not ftParts_GetBoneIndex here.
        f.effects.push(EffectRequest::SyncAttached { id, bone: 4 });
        f.effect_state.destroy_on_state_change = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reflection_preserves_descriptor_scalars_and_uses_the_contact_direction() {
        let source = ReflectionAttributes {
            joint: 4,
            max_damage: 50,
            offset: hsd_types::Vec3::new(0.0, 2.0, 0.0),
            size: 8.0,
            damage_multiplier: 1.5,
            speed_multiplier: 1.0,
            skip_ownership_change: 0,
        };
        let defense = descriptor(&source);
        assert_eq!(defense.bone, 4);
        assert_eq!(defense.maximum_damage, 50);
        assert_eq!(defense.offset, source.offset);
        assert_eq!(defense.radius.to_bits(), 8.0f32.to_bits());
        assert_eq!(defense.damage_multiplier.to_bits(), 1.5f32.to_bits());
        assert_eq!(defense.speed_multiplier.to_bits(), 1.0f32.to_bits());
        assert!(!defense.preserve_owner);
        for (air, direction, state) in [
            (false, -1.0, S::SpecialLwHit),
            (true, 1.0, S::SpecialAirLwHit),
        ] {
            let response = reflection_reaction(air, direction);
            assert_eq!(response.action, state);
            assert_eq!(response.facing.to_bits(), direction.to_bits());
            assert_eq!(response.impulse_ticks, 120);
            assert_eq!(response.impulse_scale, 3.0);
            assert_eq!(response.impulse_decay, 0.1);
            assert_eq!(response.impulse_angle.to_bits(), 0x3F86_0A92);
        }
    }
}
