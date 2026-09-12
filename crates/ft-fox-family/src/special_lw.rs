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
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        Fighter, MotionPreservation, MotionRow,
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
        exclude_master_ball_ownership: a.skip_ownership_change != 0,
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
        row(
            S::SpecialLwTurn,
            314,
            turn::<C>,
            no_input,
            callbacks::physics::guard_on,
            ground_collision,
        ),
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
        row(
            S::SpecialAirLwTurn,
            318,
            turn::<C>,
            no_input,
            air_physics::<C>,
            turn_air_collision,
        ),
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
    f.combat.reflector_enabled = true;
    f.shield.reflect.volume.position_cached = false;
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
/// ftFx_SpecialLw_Turn (800E8F20), also inlined in both Turn Anim callbacks.
fn advance_turn<C: FoxFamily>(f: &mut Fighter) {
    let duration = f.character.get::<C>().attributes().reflector.turn_frames;
    let remaining = {
        let scratch = f.character.get_mut::<C>().special_lw();
        scratch.turn_frames -= 1;
        scratch.turn_frames
    };
    if f.commands.variables[0] == 0 && remaining as f32 <= duration {
        f.commands.variables[0] = 1;
        f.physics.facing = -f.physics.facing;
    }
    let root = f.animation.root;
    // ftPartGetRotZ (80075F48) actually reads rotation Y in both branches.
    let previous = f.skeleton.rotation_y(root);
    // retail 800E8FB8 fdivs, 800E8FBC fnmsubs; @415 is float PI/180.
    let rotation = gekko_math::fma::fnmsubs(0.017453292, 180.0 / duration, previous);
    f.skeleton.set_rotation_y(root, rotation);
}

/// ftFx_SpecialLwTurn_Check (800E942C): KeepGfx and the existing reflector.
fn enter_turn<C: FoxFamily>(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    let owned = f.effect_state.destroy_on_state_change;
    f.effect_state.destroy_on_state_change = false;
    f.change_motion_state(
        (if f.physics.ground_or_air == GroundOrAir::Air {
            S::SpecialAirLwTurn
        } else {
            S::SpecialLwTurn
        })
        .into(),
        assets,
    )?;
    f.effect_state.destroy_on_state_change = owned;
    let duration = f.character.get::<C>().attributes().reflector.turn_frames;
    f.character.get_mut::<C>().special_lw().turn_frames = gekko_math::msl::fctiwz(duration);
    f.combat.reflector_enabled = true;
    f.commands.variables[0] = 0;
    advance_turn::<C>(f);
    f.character.get_mut::<C>().special_lw().pending_effect = Some(0x488);
    Ok(())
}

/// ftFx_SpecialLwTurn_Anim / SpecialAirLwTurn_Anim (800E8FDC / 800E90EC).
fn turn<C: FoxFamily>(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let release = released::<C>(f, true);
    advance_turn::<C>(f);
    if f.character.get_mut::<C>().special_lw().turn_frames <= 0 {
        // ftFx_SpecialLwHit_Check (800E9564): Loop retains its existing effect.
        if release {
            enter_end::<C>(f, p.assets)?;
        } else {
            enter_loop::<C>(f, p.assets)?;
        }
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
        enter_turn::<C>(f, p.assets).expect("Reflector turn");
        return;
    }
    if f.physics.ground_or_air == GroundOrAir::Ground {
        if melee_ft::input::human::jump_input(&f.input, &p.assets.input) {
            f.character.get_mut::<C>().special_lw().reflector = None;
            f.enter_knee_bend(p.assets).expect("Reflector jump cancel");
        } else {
            start_input(f, p);
        }
    } else if f
        .try_aerial_jump(p.assets)
        .expect("Reflector aerial jump cancel")
    {
        f.character.get_mut::<C>().special_lw().reflector = None;
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
/// ftFx_SpecialAirLwTurn_Coll / GroundToAir (800E92E8 / 800E93A4):
/// land with the current turn frame, effect and reflector descriptor intact.
fn turn_air_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    use melee_ft::collision::air;
    let c = &mut f.core;
    air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    if air::collide_air_dodge(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
    ) {
        f.land();
        // retail 800E93DC: 0x0C4C5082, common counterpart flags + KeepGfx.
        // Unlike Counter, this does not preserve hitboxes or hit status.
        f.change_ground_air_motion(
            S::SpecialLwTurn.into(),
            p.assets.expect("Reflector turn landing assets"),
            MotionPreservation {
                effects: true,
                ..Default::default()
            },
        )?;
        f.combat.reflector_enabled = true;
        let maximum = f.attributes.air.air_drift_max;
        f.physics.self_velocity.x = f.physics.self_velocity.x.clamp(-maximum, maximum);
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
    let hip = assets
        .parts
        .joint(melee_types::FtPart::HipN)
        .expect("Reflector HipN") as usize;
    let core = &mut f.core;
    let center = melee_ft::fighter::caches::bone_position(
        &mut core.skeleton,
        core.animation.root,
        hip,
        hsd_types::Vec3::ZERO,
    );
    core.commands
        .radial_impulses
        .push(melee_lb::radial_force::RadialImpulse {
            center,
            frames: reaction.impulse_ticks,
            strength: reaction.impulse_scale,
            decay: reaction.impulse_decay,
            phase_step: reaction.impulse_angle,
        });
    f.change_motion_state(reaction.action.into(), assets)?;
    f.combat.reflector_enabled = true;
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
        // All three retail accessory callbacks install PauseAll/ResumeAll,
        // including KeepGfx turns that do not create another model.
        f.effect_state.hitlag_callbacks = true;
        // ftFx_SpecialLw_Create*GFX: KeepGfx turns retain the live effect.
        if !f.effect_state.destroy_on_state_change {
            // Retail uses literal HipN slot 4, not ftParts_GetBoneIndex.
            f.effects.push(EffectRequest::SyncAttached { id, bone: 4 });
            f.effect_state.destroy_on_state_change = true;
        }
    }
}

/// The live callback latch, not stale move scratch, owns item contact eligibility.
pub fn reflector_contact<C: FoxFamily>(
    f: &mut Fighter,
    hit: &melee_coll::hitbox::HitCapsule,
    scale: f32,
) -> Option<ReflectDescriptor> {
    if !f.combat.reflector_enabled {
        return None;
    }
    let descriptor = f.character.get_mut::<C>().special_lw().reflector?;
    f.core
        .reflector_contact(hit, scale, &descriptor)
        .then_some(descriptor)
}
pub fn reflect_hit<C: FoxFamily>(
    f: &mut Fighter,
    direction: f32,
    assets: &FighterAssets,
) -> Result<()> {
    on_reflect::<C>(f, direction, assets).map(|_| ())
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
        assert!(!defense.exclude_master_ball_ownership);
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
