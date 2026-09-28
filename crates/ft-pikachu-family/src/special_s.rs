//! Skull Bash, ftpikachuspecials.c (801253D4..80125D78), with the charge
//! set-up ftPk_SpecialN_80124DC8 (80124DC8) and the sparks
//! ftPk_SpecialN_SpawnEffect0/1 (80124C90 / 80124D2C).
//!
//! The start (343 / 348) runs into the charging hold (344 / 349). Letting
//! go of B, or charging past the attribute's maximum, winds up (347 / 352);
//! the script's cmd_vars[0] launches Pikachu (350), whose landing or wall
//! contact ends the move (346 / 351).
use crate::{
    common::{self, change, no_input},
    flags::{GROUND_AIR, KEEP_COL_ANIM_HIT_STATUS, KEEP_GFX, KEEP_SFX, SKIP_HIT},
    row, Accessory, FamilyState as S, PikachuFamily,
};
use melee_ef::request::EffectRequest;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        Fighter, MotionRow,
    },
    input::Buttons,
};
use melee_types::{mp::collide, FtPart};

/// transition_flags0 (ftpikachuspecials.c:120): the start's counterparts.
const START_FLAGS: u32 = GROUND_AIR | KEEP_COL_ANIM_HIT_STATUS | KEEP_SFX;
/// transition_flags1: the hold's counterparts also keep the sparks.
const HOLD_FLAGS: u32 = START_FLAGS | KEEP_GFX;
/// transition_flags2: the wind-up's counterparts.
const WIND_UP_FLAGS: u32 = GROUND_AIR | KEEP_GFX | KEEP_COL_ANIM_HIT_STATUS;
/// transition_flags3: the launch keeps the sparks and the hitbox.
const LAUNCH_FLAGS: u32 = KEEP_GFX | SKIP_HIT;
/// efSync_Spawn ids of the sparks at HipN.
const CHARGE_SPARKS: u16 = 1214;
const LAUNCH_SPARKS: u16 = 1215;

pub const fn rows<C: PikachuFamily>() -> [MotionRow; 9] {
    [
        row(
            S::SpecialSStart,
            start_anim::<C, false>,
            no_input,
            start_ground_physics::<C>,
            start_ground_collision,
        ),
        row(
            S::SpecialSHold,
            hold_anim::<C, false>,
            hold_input::<C, false>,
            callbacks::physics::guard_on,
            hold_ground_collision,
        ),
        row(
            S::SpecialSEnd,
            end_anim::<false>,
            no_input,
            end_ground_physics::<C>,
            end_ground_collision,
        ),
        row(
            S::SpecialS0,
            wind_up_anim::<C, false>,
            no_input,
            callbacks::physics::guard_on,
            wind_up_ground_collision,
        ),
        row(
            S::SpecialAirSStart,
            start_anim::<C, true>,
            no_input,
            start_air_physics::<C>,
            start_air_collision,
        ),
        row(
            S::SpecialAirSHold,
            hold_anim::<C, true>,
            hold_input::<C, true>,
            callbacks::physics::air_friction,
            hold_air_collision,
        ),
        row(
            S::SpecialAirS1,
            flight_anim::<C>,
            no_input,
            flight_physics::<C>,
            flight_collision::<C>,
        ),
        row(
            S::SpecialAirSEnd,
            end_anim::<true>,
            no_input,
            end_air_physics::<C>,
            end_air_collision::<C>,
        ),
        row(
            S::SpecialAirS0,
            wind_up_anim::<C, true>,
            no_input,
            callbacks::physics::air_friction,
            wind_up_air_collision,
        ),
    ]
}

fn attributes<C: PikachuFamily>(f: &Fighter) -> &crate::attributes::SkullBashAttributes {
    &f.character.get::<C>().attributes().skull_bash
}

/// ftPk_SpecialS_Enter (801253D4) / ftPk_SpecialAirS_Enter (80125434):
/// x21EC = ftPk_SpecialN_80124DC8 runs inside the motion change, before
/// its first script frame; nothing between reads what it writes.
pub fn enter<C: PikachuFamily>(f: &mut Fighter, air: bool, assets: &FighterAssets) {
    let divisor = attributes::<C>(f).entry_velocity_divisor;
    set_up_charge::<C>(f);
    // Retail fdivs.
    if air {
        f.physics.self_velocity.x /= divisor;
        f.physics.self_velocity.y = 0.0;
    } else {
        f.physics.ground_velocity /= divisor;
    }
    let state = if air {
        S::SpecialAirSStart
    } else {
        S::SpecialSStart
    };
    change(f, state.action(), 0, 0.0, 1.0, assets).expect("Skull Bash assets");
    f.step_animation(assets);
}

/// ftPk_SpecialN_80124DC8 (80124DC8): a side-B within the smash window of
/// the stick's tilt (the u8 x673 against the float window, retail
/// 80124DFC fcmpo) starts charged (fctiwz of the charge attribute). The
/// thrown-item statistic is not modelled.
fn set_up_charge<C: PikachuFamily>(f: &mut Fighter) {
    f.commands.variables[0] = 0;
    let a = attributes::<C>(f);
    let charge = if f32::from(f.input.horizontal.held) < a.smash_input_window {
        gekko_math::msl::fctiwz(a.smash_input_charge)
    } else {
        0
    };
    f.character.get_mut::<C>().specials().skull_bash_charge = charge;
}

fn charge<C: PikachuFamily>(f: &mut Fighter) -> &mut i32 {
    &mut f.character.get_mut::<C>().specials().skull_bash_charge
}

/// Install the sparks as the one-shot accessory4.
fn arm_sparks<C: PikachuFamily>(f: &mut Fighter, sparks: Accessory) {
    f.character.get_mut::<C>().specials().accessory = sparks;
    f.arm_accessory4();
}

/// ftCommon_8007DB24: x2219_b0 clears and every owned effect goes.
fn remove_effects(f: &mut Fighter) {
    f.effect_state.destroy_on_state_change = false;
    f.effects.push(EffectRequest::DestroyOwned);
}

/// ftPk_SpecialSStart_Anim / ftPk_SpecialAirSStart_Anim: at the end, the
/// hold (ftPk_SpecialS_ChangeMotion_Unk04 / Unk05) with the charge sparks.
fn start_anim<C: PikachuFamily, const AIR: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        let state = if AIR {
            S::SpecialAirSHold
        } else {
            S::SpecialSHold
        };
        change(f, state.action(), KEEP_SFX, 0.0, 1.0, p.assets)?;
        arm_sparks::<C>(f, Accessory::ChargeSparks);
    }
    Ok(None)
}

/// ftPk_SpecialSStart_Phys (801254DC): the start's own friction.
fn start_ground_physics<C: PikachuFamily>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let friction = attributes::<C>(f).start_friction;
    common::ground_friction(f, friction);
    common::move_on_ground(f, &p);
}

/// ftPk_SpecialAirSStart_Phys (80125518): the start's gravity once the
/// script sets cmd_vars[0], and its friction.
fn start_air_physics<C: PikachuFamily>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let a = attributes::<C>(f);
    let (gravity, friction) = (a.start_gravity, a.start_friction);
    if f.commands.variables[0] != 0 {
        let terminal = f.attributes.air.terminal_velocity;
        common::fall(f, gravity, terminal);
    }
    common::air_friction(f, friction);
    common::finish_air(f, &p);
}

/// ftPk_SpecialSStart_Coll: off the floor, ftPk_SpecialS_ChangeMotion_Unk00.
fn start_ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::grounded(f, &mut p) {
        let assets = p.assets.expect("Skull Bash collision assets");
        common::ground_to_air(f, S::SpecialAirSStart.action(), START_FLAGS, assets)?;
    }
    Ok(())
}

/// ftPk_SpecialAirSStart_Coll: landing, ftPk_SpecialS_ChangeMotion_Unk01.
fn start_air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::lands(f, &mut p) {
        let assets = p.assets.expect("Skull Bash landing assets");
        common::air_to_ground(f, S::SpecialSStart.action(), START_FLAGS, assets)?;
    }
    Ok(())
}

/// ftPk_SpecialSHold_Anim / ftPk_SpecialAirSHold_Anim (801251BC /
/// 8012525C): each loop of the animation renews the sparks; one charge
/// frame; past the maximum (retail 80125230 fcmpo of the int as float),
/// the wind-up.
fn hold_anim<C: PikachuFamily, const AIR: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        remove_effects(f);
        arm_sparks::<C>(f, Accessory::ChargeSparks);
    }
    *charge::<C>(f) += 1;
    let maximum = attributes::<C>(f).maximum_charge;
    if *charge::<C>(f) as f32 > maximum {
        wind_up::<C>(f, AIR, p.assets)?;
    }
    Ok(None)
}

/// ftPk_SpecialSHold_IASA / ftPk_SpecialAirSHold_IASA: B let go winds up.
fn hold_input<C: PikachuFamily, const AIR: bool>(f: &mut Fighter, p: InputPhase<'_>) {
    if !f.input.current.held.intersects(Buttons::B) {
        wind_up::<C>(f, AIR, p.assets).expect("Skull Bash wind-up assets");
    }
}

/// ftPk_SpecialS_ChangeMotion_Unk08 / Unk09 (80125834 / 8012589C): the
/// wind-up with the launch sparks in place of the charge's.
fn wind_up<C: PikachuFamily>(f: &mut Fighter, air: bool, assets: &FighterAssets) -> Result<()> {
    let state = if air { S::SpecialAirS0 } else { S::SpecialS0 };
    change(f, state.action(), 0, 0.0, 1.0, assets)?;
    f.commands.variables[0] = 0;
    remove_effects(f);
    arm_sparks::<C>(f, Accessory::LaunchSparks);
    Ok(())
}

/// ftPk_SpecialSHold_Coll: off the floor, ftPk_SpecialS_ChangeMotion_Unk02.
fn hold_ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::grounded(f, &mut p) {
        let assets = p.assets.expect("Skull Bash collision assets");
        common::ground_to_air(f, S::SpecialAirSHold.action(), HOLD_FLAGS, assets)?;
    }
    Ok(())
}

/// ftPk_SpecialAirSHold_Coll: landing, ftPk_SpecialS_ChangeMotion_Unk03.
fn hold_air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::lands(f, &mut p) {
        let assets = p.assets.expect("Skull Bash landing assets");
        common::air_to_ground(f, S::SpecialSHold.action(), HOLD_FLAGS, assets)?;
    }
    Ok(())
}

/// ftPk_SpecialS0_Anim / ftPk_SpecialAirS0_Anim (8012557C / 8012561C):
/// while hitbox 0 is freshly enabled its damage follows the charge; the
/// script's cmd_vars[0] launches (the grounded wind-up leaves the floor
/// first, ftCommon_8007D5D4).
fn wind_up_anim<C: PikachuFamily, const AIR: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let enabled = f.commands.hitboxes[0]
        .as_ref()
        .is_some_and(|hit| hit.phase == melee_coll::hitbox::CapsulePhase::Enabled);
    if enabled {
        let a = attributes::<C>(f);
        let (base, per_charge) = (a.base_damage, a.damage_per_charge);
        let charge = *charge::<C>(f) as f32;
        // Retail 801255D0 fmadds, then __cvt_fp2unsigned.
        let damage = gekko_math::fma::fmadds(charge, per_charge, base);
        set_hitbox_damage(f, damage as u32);
    }
    if f.commands.variables[0] != 0 {
        if !AIR {
            f.leave_ground();
        }
        launch::<C>(f, p.assets)?;
    }
    Ok(None)
}

/// ftColl_8007ABD0 (8007ABD0): hitbox 0's damage from `damage`, staled.
fn set_hitbox_damage(f: &mut Fighter, damage: u32) {
    if f.player.scale != 1.0 {
        unimplemented!("ftColl_8007ABD0: ftCo_CalcYScaledKnockback for a scaled fighter");
    }
    // ftCo_800DEEB8 scales only a released smash charge.
    assert!(
        f.commands.smash_charge.is_none(),
        "ftCo_800DEEB8: charged Skull Bash"
    );
    let damage = damage as f32;
    let staled = f.commands.stale_damage(damage);
    let hit = f.commands.hitboxes[0].as_mut().expect("Skull Bash hitbox");
    hit.knockback_damage = gekko_math::msl::fctiwz(damage) as u32;
    hit.descriptor.damage = staled;
}

/// ftPk_SpecialS_ChangeMotion_Unk10 (80125A54): the launch speed grows
/// with the charge (retail 80125AA8 fmadds, then fmuls by the facing); the
/// rise is half the vertical attribute plus its share of the charge
/// (80125AE4..AF0: fmuls, fdivs, fmuls, fmadds). deal_dmg_cb =
/// ftPk_SpecialS_ZeroVelocity while the launch lasts; x21F8
/// (ftCommon_8007F76C) only runs after a cape turnaround, not modelled.
fn launch<C: PikachuFamily>(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    f.commands.variables[0] = 0;
    let a = attributes::<C>(f);
    let (speed, per_charge, rise, maximum) = (
        a.launch_speed,
        a.launch_speed_per_charge,
        a.launch_vertical_speed,
        a.maximum_charge,
    );
    let charge = *charge::<C>(f) as f32;
    f.physics.self_velocity.x = gekko_math::fma::fmadds(per_charge, charge, speed);
    f.physics.self_velocity.x *= f.physics.facing;
    let share = rise * ((0.5 * charge) / maximum);
    f.physics.self_velocity.y = gekko_math::fma::fmadds(0.5, rise, share);
    let frame = f.animation.frame;
    change(
        f,
        S::SpecialAirS1.action(),
        LAUNCH_FLAGS,
        frame,
        1.0,
        assets,
    )
}

/// ftPk_SpecialS0_Coll: off the floor, ftPk_SpecialS_ChangeMotion_Unk06.
fn wind_up_ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::grounded(f, &mut p) {
        let assets = p.assets.expect("Skull Bash collision assets");
        common::ground_to_air(f, S::SpecialAirS0.action(), WIND_UP_FLAGS, assets)?;
    }
    Ok(())
}

/// ftPk_SpecialAirS0_Coll: landing, ftPk_SpecialS_ChangeMotion_Unk07.
fn wind_up_air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::lands(f, &mut p) {
        let assets = p.assets.expect("Skull Bash landing assets");
        common::air_to_ground(f, S::SpecialS0.action(), WIND_UP_FLAGS, assets)?;
    }
    Ok(())
}

/// ftPk_SpecialAirS1_Anim: at the animation's end, the aerial ending.
fn flight_anim<C: PikachuFamily>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        enter_air_end::<C>(f, p.assets)?;
    }
    Ok(None)
}

/// ftPk_SpecialAirS1_Phys (80125924): the flight's gravity until the
/// script's cmd_vars[0], the ending's after it along with its friction.
fn flight_physics<C: PikachuFamily>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let a = attributes::<C>(f);
    let (flight, end, terminal, friction) = (
        a.flight_gravity,
        a.end_gravity,
        a.flight_terminal_velocity,
        a.end_friction,
    );
    let flag = f.commands.variables[0] != 0;
    common::fall(f, if flag { end } else { flight }, terminal);
    if flag {
        common::air_friction(f, friction);
    }
    common::finish_air(f, &p);
}

/// ftPk_SpecialAirS1_Coll (801259A4): landing ends on the ground
/// (ftCommon_8007D7FC, Unk11); a wall, touched in the same pass, ends in
/// the air (Unk12) even after that landing.
fn flight_collision<C: PikachuFamily>(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    let assets = p.assets.expect("Skull Bash flight collision assets");
    if common::lands(f, &mut p) {
        f.land();
        enter_ground_end::<C>(f, assets)?;
    }
    let env = f.collision.data.env_flags as u32;
    if env & (collide::LEFT_WALL_MASK | collide::RIGHT_WALL_MASK) != 0 {
        enter_air_end::<C>(f, assets)?;
    }
    Ok(())
}

/// ftPk_SpecialS_ZeroVelocity (80124F24), deal_dmg_cb during the launch:
/// the hit stops Pikachu (a rise too) and ends the move.
pub fn deal_damage<C: PikachuFamily>(f: &mut Fighter, assets: &FighterAssets) {
    f.physics.self_velocity.x = 0.0;
    if f.physics.self_velocity.y >= 0.0 {
        f.physics.self_velocity.y = 0.0;
    }
    enter_air_end::<C>(f, assets).expect("Skull Bash hit assets");
}

/// Whether `action` installed ftPk_SpecialS_ZeroVelocity as deal_dmg_cb.
pub fn deals_damage(action: melee_ft::fighter::ActionId) -> bool {
    action == S::SpecialAirS1.action()
}

/// ftPk_SpecialS_ChangeMotion_Unk11 (80125C9C): landing speed divided
/// (fdivs), then the grounded ending.
fn enter_ground_end<C: PikachuFamily>(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    f.commands.variables[0] = 0;
    let divisor = attributes::<C>(f).end_velocity_divisor;
    f.physics.ground_velocity /= divisor;
    change(f, S::SpecialSEnd.action(), 0, 0.0, 1.0, assets)
}

/// ftPk_SpecialS_ChangeMotion_Unk12 (80125D08): the aerial ending with the
/// horizontal speed divided (fdivs).
fn enter_air_end<C: PikachuFamily>(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    f.commands.variables[0] = 0;
    let divisor = attributes::<C>(f).end_velocity_divisor;
    f.physics.self_velocity.x /= divisor;
    change(f, S::SpecialAirSEnd.action(), 0, 0.0, 1.0, assets)
}

/// ftPk_SpecialSEnd_Anim / ftPk_SpecialAirSEnd_Anim: Wait or Fall.
fn end_anim<const AIR: bool>(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::finish(f, AIR, p.assets)?;
    }
    Ok(None)
}

/// ftPk_SpecialSEnd_Phys (80125B98): the ending's friction.
fn end_ground_physics<C: PikachuFamily>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let friction = attributes::<C>(f).end_friction;
    common::ground_friction(f, friction);
    common::move_on_ground(f, &p);
}

/// ftPk_SpecialAirSEnd_Phys (80125BDC): the ending's gravity (common
/// terminal velocity) and friction.
fn end_air_physics<C: PikachuFamily>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let a = attributes::<C>(f);
    let (gravity, friction) = (a.end_gravity, a.end_friction);
    let terminal = f.attributes.air.terminal_velocity;
    common::fall(f, gravity, terminal);
    common::air_friction(f, friction);
    common::finish_air(f, &p);
}

/// ftPk_SpecialSEnd_Coll: off the floor, Fall.
fn end_ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::grounded(f, &mut p) {
        let assets = p.assets.expect("Skull Bash collision assets");
        f.change_motion_state(melee_types::CommonMotionState::Fall.into(), assets)?;
    }
    Ok(())
}

/// ftPk_SpecialAirSEnd_Coll: landing (ftCommon_8007D7FC), Unk11.
fn end_air_collision<C: PikachuFamily>(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::lands(f, &mut p) {
        f.land();
        enter_ground_end::<C>(f, p.assets.expect("Skull Bash landing assets"))?;
    }
    Ok(())
}

/// ftPk_SpecialN_SpawnEffect0 / SpawnEffect1 (80124C90 / 80124D2C): the
/// sparks at HipN unless owned effects already play (x2219_b0), then
/// Fighter_SetEffectHitlagCallbacks; the accessory removes itself.
pub fn sparks(f: &mut Fighter, assets: &FighterAssets, launch: bool) {
    if !f.effect_state.destroy_on_state_change {
        let bone = usize::from(
            assets.parts.part_to_joint[FtPart::HipN as usize].expect("Skull Bash spark bone"),
        );
        let id = if launch { LAUNCH_SPARKS } else { CHARGE_SPARKS };
        f.effects.push(EffectRequest::SyncAttached { id, bone });
        f.effect_state.destroy_on_state_change = true;
    }
    f.effect_state.hitlag_callbacks = true;
}
