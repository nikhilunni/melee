//! Spin Attack, ftlinkspecialhi.c (800EBA4C..800EBF2C).
//!
//! The grounded spin stays grounded unless it leaves the floor, when it
//! continues as the aerial spin at the same frame with every jump spent.
//! The aerial spin rises on the attribute speed under reduced gravity and
//! drift, then falls special. Both install the swirl effect once
//! (onAccessory4) and the effect hitlag callbacks.
use crate::{
    common::{self, flags},
    row, Accessory, FamilyState, LinkFamily,
};
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, PhysicsPhase},
        Fighter, MotionRow,
    },
};
use melee_types::{FtPart, GroundOrAir};

/// efSync_Spawn ids of onAccessory4 (efsync.c 0x4BB / 0x4BC).
const GROUND_SWIRL: u16 = 1211;
const AIR_SWIRL: u16 = 1212;

fn attributes<C: LinkFamily>(f: &Fighter) -> &crate::attributes::SpinAttackAttributes {
    &f.character.get::<C>().attributes().spin_attack
}

/// ftLk_SpecialHi_Enter (800EBB08) / ftLk_SpecialAirHi_Enter (800EBB5C).
pub fn enter<C: LinkFamily>(f: &mut Fighter, air: bool, a: &FighterAssets) {
    let state = if air {
        FamilyState::SpecialAirHi
    } else {
        FamilyState::SpecialHi
    };
    common::change(f, state.action(), 0, 0.0, a).expect("Spin Attack assets");
    // ftAnim_8006EBA4.
    f.step_animation(a);
    if air {
        let (scale, speed) = {
            let s = attributes::<C>(f);
            (s.air_horizontal_scale, s.air_vertical_speed)
        };
        // 800EBBA0: fmuls.
        f.physics.self_velocity.x *= scale;
        f.physics.self_velocity.y = speed;
        f.physics.jumps_used = f.core.attributes.jumping.max_jumps as u8;
    }
    crate::arm_accessory::<C>(f, Accessory::SpinSwirl);
}

/// onAccessory4 (800EBA4C): the swirl on TransN and the kind's sword tip
/// (parts[FtPart_L2ndNa] for Link, parts[FtPart_L3rdNa] otherwise, raw
/// indices), once per spin (x2219_b0); then the effect hitlag callbacks.
pub(crate) fn swirl<C: LinkFamily>(f: &mut Fighter, assets: &FighterAssets) {
    if !f.effect_state.destroy_on_state_change {
        let trans = usize::from(
            assets.parts.part_to_joint[FtPart::TransN as usize].expect("Spin Attack TransN"),
        );
        let tip = C::SPIN_TIP_PART as usize;
        let id = if f.physics.ground_or_air == GroundOrAir::Ground {
            GROUND_SWIRL
        } else {
            AIR_SWIRL
        };
        f.effects
            .push(melee_ef::request::EffectRequest::SyncAttachedPair {
                id,
                bones: [trans, tip],
            });
        f.effect_state.destroy_on_state_change = true;
    }
    f.effect_state.hitlag_callbacks = true;
}

/// ftLk_SpecialHi_Anim (800EBBC8): Wait at the animation's end.
fn ground_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::finish(f, false, p.assets)?;
    }
    Ok(None)
}

/// ftLk_SpecialAirHi_Anim (800EBC0C): ftCo_80096900(gobj, 1, 1, false,
/// drift scale, landing lag) at the animation's end.
fn air_anim<C: LinkFamily>(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        let (mobility, lag) = {
            let s = attributes::<C>(f);
            (s.air_drift_stick_scale, s.landing_lag)
        };
        f.enter_special_fall(p.assets, true, true, false, mobility, lag)?;
    }
    Ok(None)
}

/// ftLk_SpecialAirHi_Phys (800EBCB4): gravity scaled by the attribute,
/// then ftCommon_8007D344 with the scaled drift (four separate fmuls).
fn air_physics<C: LinkFamily>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let (gravity_scale, stick_scale, max_scale) = {
        let s = attributes::<C>(f);
        (
            s.gravity_scale,
            s.air_drift_stick_scale,
            s.air_drift_max_scale,
        )
    };
    let air = &f.core.attributes.air;
    let (gravity, terminal) = (air.gravity * gravity_scale, air.terminal_velocity);
    let (acceleration, maximum) = (
        air.air_drift_stick_mul * stick_scale,
        air.air_drift_max * max_scale,
    );
    common::fall(f, gravity, terminal);
    common::drift(f, 0.0, acceleration, maximum);
    f.core.finish_air_update(p.assets, p.wind);
}

/// ftLk_SpecialHi_Phys (800EBD30): the aerial physics once airborne, else
/// ft_80084F3C.
fn ground_physics<C: LinkFamily>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    if f.physics.ground_or_air == GroundOrAir::Air {
        air_physics::<C>(f, p);
    } else {
        callbacks::physics::guard_on(f, p);
    }
}

/// ftLk_SpecialHi_Coll (800EBD80): off the floor, doColl (800EBE64):
/// every jump spent (ftCommon_8007D60C), then the aerial spin at the
/// current frame, keeping its effects and hitboxes.
fn ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::grounded(f, &mut p) {
        let assets = p.assets.expect("Spin Attack assets");
        f.core.leave_ground_with_spent_jumps();
        let frame = f.animation.frame;
        common::change(
            f,
            FamilyState::SpecialAirHi.action(),
            flags::GROUND_AIR | flags::KEEP_GFX | flags::SKIP_HIT,
            frame,
            assets,
        )?;
    }
    Ok(())
}

/// ftLk_SpecialAirHi_Coll (800EBDC0): ft_CheckGroundAndLedge, then the
/// special landing with the spin's lag (the decomp reads it through
/// dat_attrs as `facing_dir1`, +0x30), else ftCliffCommon_80081298.
fn air_collision<C: LinkFamily>(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    let assets = p.assets.expect("Spin Attack assets");
    if common::lands_or_catches_ledge(f, &mut p) {
        let lag = attributes::<C>(f).landing_lag;
        f.enter_special_landing(assets, false, lag)?;
    } else {
        f.try_grab_ledge(assets, p.map)?;
    }
    Ok(())
}

pub(crate) const fn rows<C: LinkFamily>() -> [MotionRow; 2] {
    [
        row(
            FamilyState::SpecialHi,
            ground_anim,
            common::no_input,
            ground_physics::<C>,
            ground_collision,
        ),
        row(
            FamilyState::SpecialAirHi,
            air_anim::<C>,
            common::no_input,
            air_physics::<C>,
            air_collision::<C>,
        ),
    ]
}
