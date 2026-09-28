//! Green Missile, ftluigispecials.c (80142A24..80143F70).
//!
//! The start (343 / 349) runs into the charging hold (344 / 350). Letting
//! go of B, or charging past the attribute's maximum, launches (347 / 353),
//! unless the entry's draw chose a misfire (348 / 354). The script's
//! cmd_vars[0] sends Luigi flying (351), whose landing or wall contact ends
//! the move (346 / 352).
use crate::{
    common::{self, change, flags, no_input, row},
    init::{Accessory, Luigi},
};
use melee_ef::request::EffectRequest;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, MotionRow, PhysicsPhase},
        ActionId, Fighter,
    },
    input::Buttons,
};
use melee_types::{mp::collide, FtPart};

/// ftLg_MS_* (forward.h), 343..354.
pub const START: ActionId = ActionId(343);
pub const HOLD: ActionId = ActionId(344);
/// ftLg_MS_SpecialS2: the grounded flight, which nothing enters.
pub const FLIGHT_GROUND: ActionId = ActionId(345);
pub const END: ActionId = ActionId(346);
pub const LAUNCH: ActionId = ActionId(347);
pub const MISFIRE: ActionId = ActionId(348);
pub const AIR_START: ActionId = ActionId(349);
pub const AIR_HOLD: ActionId = ActionId(350);
pub const FLIGHT: ActionId = ActionId(351);
pub const AIR_END: ActionId = ActionId(352);
pub const AIR_LAUNCH: ActionId = ActionId(353);
pub const AIR_MISFIRE: ActionId = ActionId(354);

/// transition_flags0 (ftluigispecials.c:172): the start's counterparts.
const START_FLAGS: u32 = flags::GROUND_AIR | flags::KEEP_COL_ANIM_HIT_STATUS | flags::KEEP_SFX;
/// transition_flags1: the hold's counterparts also keep the effects.
const HOLD_FLAGS: u32 = START_FLAGS | flags::KEEP_GFX;
/// transition_flags2: the launch's and the misfire's counterparts.
const LAUNCH_FLAGS: u32 = flags::GROUND_AIR | flags::KEEP_GFX | flags::KEEP_COL_ANIM_HIT_STATUS;
/// transition_flags3: the flight keeps the effects and the hitbox.
const FLIGHT_FLAGS: u32 = flags::GROUND_AIR | flags::KEEP_GFX | flags::SKIP_HIT;
/// efSync_Spawn(0x50A, gobj, HipN): the launch's sparks (common generator
/// 0x5F on the hip).
const LAUNCH_SPARKS: u16 = 0x50A;

/// fp->mv.lg.SpecialS.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GreenMissile {
    /// +2340 chargeFrames.
    pub charge: i32,
    /// +2344 isMisfire.
    pub misfire: bool,
    /// ftLg_SpecialS_SetVars's HSD_Randi, which the port draws once the
    /// entering IASA returns.
    pub misfire_draw_pending: bool,
}

pub const fn rows() -> [MotionRow; 11] {
    [
        row(
            START,
            2,
            start_anim::<false>,
            no_input,
            start_ground_physics,
            start_ground_collision,
        ),
        row(
            HOLD,
            3,
            hold_anim::<false>,
            hold_input::<false>,
            hold_ground_physics,
            hold_ground_collision,
        ),
        row(
            END,
            7,
            end_anim::<false>,
            no_input,
            end_ground_physics,
            end_ground_collision,
        ),
        row(
            LAUNCH,
            4,
            launch_anim::<false>,
            no_input,
            launch_ground_physics,
            launch_ground_collision::<false>,
        ),
        row(
            MISFIRE,
            5,
            launch_anim::<false>,
            no_input,
            launch_ground_physics,
            launch_ground_collision::<true>,
        ),
        row(
            AIR_START,
            8,
            start_anim::<true>,
            no_input,
            start_air_physics,
            start_air_collision,
        ),
        row(
            AIR_HOLD,
            9,
            hold_anim::<true>,
            hold_input::<true>,
            callbacks::physics::air_friction,
            hold_air_collision,
        ),
        // ftLg_Init_MotionStateTable plays ftLg_SM_SpecialS2 for the aerial flight.
        row(
            FLIGHT,
            6,
            flight_anim,
            no_input,
            flight_physics,
            flight_collision,
        ),
        row(
            AIR_END,
            12,
            end_anim::<true>,
            no_input,
            end_air_physics,
            end_air_collision,
        ),
        row(
            AIR_LAUNCH,
            10,
            launch_anim::<true>,
            no_input,
            callbacks::physics::air_friction,
            launch_air_collision::<false>,
        ),
        row(
            AIR_MISFIRE,
            11,
            launch_anim::<true>,
            no_input,
            misfire_air_physics,
            launch_air_collision::<true>,
        ),
    ]
}

fn attributes(f: &Fighter) -> &crate::attributes::GreenMissileAttributes {
    &f.character.get::<Luigi>().attributes.green_missile
}

fn scratch(f: &mut Fighter) -> &mut GreenMissile {
    &mut f.character.get_mut::<Luigi>().green_missile
}

/// ftLg_SpecialS_Enter (80142B14) / ftLg_SpecialAirS_Enter (80142B88):
/// x21EC = ftLg_SpecialS_SetVars runs inside the motion change, before its
/// first script frame.
pub fn enter(f: &mut Fighter, air: bool, assets: &FighterAssets) {
    let divisor = attributes(f).entry_speed_divisor;
    set_up(f);
    // Retail fdivs.
    if air {
        f.physics.self_velocity.x /= divisor;
        f.physics.self_velocity.y = 0.0;
    } else {
        f.physics.ground_velocity /= divisor;
    }
    let state = if air { AIR_START } else { START };
    change(f, state, 0, 0.0, 1.0, assets).expect("Green Missile assets");
    f.step_animation(assets);
}

/// ftLg_SpecialS_SetVars (80142A5C): a side-B within the smash window of
/// the stick's tilt (the u8 x673 as float against the window, retail
/// 80142A9C fcmpo) starts charged (fctiwz of the charge attribute). The
/// thrown-item statistic is not modelled. Its misfire draw waits for
/// `draw_misfire`.
fn set_up(f: &mut Fighter) {
    f.commands.variables[0] = 0;
    let a = attributes(f);
    let charge = if f32::from(f.input.horizontal.held) < a.smash_window {
        gekko_math::msl::fctiwz(a.smash_charge)
    } else {
        0
    };
    *scratch(f) = GreenMissile {
        charge,
        misfire: false,
        misfire_draw_pending: true,
    };
}

/// ftLg_SpecialS_SetVars's tail (80142AD0..AFC): HSD_Randi(fctiwz(odds));
/// zero misfires. Nothing between the motion change and the end of the
/// entering IASA draws from the RNG.
pub fn draw_misfire(f: &mut Fighter, rng: &mut gekko_math::HsdRng) {
    if !std::mem::take(&mut scratch(f).misfire_draw_pending) {
        return;
    }
    let odds = gekko_math::msl::fctiwz(attributes(f).misfire_odds);
    scratch(f).misfire = rng.randi(odds) == 0;
}

fn charge(f: &mut Fighter) -> &mut i32 {
    &mut scratch(f).charge
}

/// accessory4_cb = ftLg_SpecialS_SetGFX until the next motion change.
fn install_accessory(f: &mut Fighter) {
    f.character.get_mut::<Luigi>().accessory = Accessory::ChargeEffects;
    f.arm_accessory4();
}

/// ftLg_SpecialS_SetGFX (80142A24): x2219_b0 set (no effect of its own),
/// then Fighter_SetEffectHitlagCallbacks. It stays installed.
pub fn charge_effects(f: &mut Fighter) {
    f.effect_state.destroy_on_state_change = true;
    f.effect_state.hitlag_callbacks = true;
}

/// ftLg_SpecialSStart_Anim / ftLg_SpecialAirSStart_Anim: at the end, the
/// hold (ftLg_SpecialSHold_Enter, KeepSfx) with SetGFX installed.
fn start_anim<const AIR: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        let state = if AIR { AIR_HOLD } else { HOLD };
        change(f, state, flags::KEEP_SFX, 0.0, 1.0, p.assets)?;
        install_accessory(f);
    }
    Ok(None)
}

/// ftLg_SpecialSStart_Phys (80142CC0): the wind-up friction.
fn start_ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let friction = attributes(f).windup_friction;
    common::ground_friction(f, friction);
    common::apply_ground_movement(f, &p);
    common::finish_update(f, &p);
}

/// ftLg_SpecialAirSStart_Phys: the wind-up's gravity once the script sets
/// cmd_vars[0], and its friction.
fn start_air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let a = attributes(f);
    let (gravity, friction) = (a.windup_gravity, a.windup_friction);
    if f.commands.variables[0] != 0 {
        let terminal = f.attributes.air.terminal_velocity;
        common::fall(f, gravity, terminal);
    }
    common::air_friction(f, friction);
    common::finish_update(f, &p);
}

/// ftLg_SpecialSStart_Coll (80142D60): off the floor,
/// ftLg_SpecialSStart_GroundToAir.
fn start_ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::grounded(f, &mut p) {
        let assets = p.assets.expect("Green Missile collision assets");
        common::ground_to_air(f, AIR_START, START_FLAGS, assets)?;
    }
    Ok(())
}

/// ftLg_SpecialAirSStart_Coll: landing, ftLg_SpecialAirSStart_AirToGround.
fn start_air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::lands(f, &mut p) {
        let assets = p.assets.expect("Green Missile landing assets");
        common::air_to_ground(f, START, START_FLAGS, assets)?;
    }
    Ok(())
}

/// ftLg_SpecialSHold_Anim (80142E98) / ftLg_SpecialAirSHold_Anim: each
/// loop of the animation removes the effects and reinstalls SetGFX; one
/// charge frame; past the maximum (retail 80142F0C fcmpo of the int as
/// float), the launch.
fn hold_anim<const AIR: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::remove_effects(f);
        install_accessory(f);
    }
    *charge(f) += 1;
    let maximum = attributes(f).max_charge_frames;
    if *charge(f) as f32 > maximum {
        // Fighter_ChangeMotionState's efAsync_QueueFlush takes what this
        // frame's script already queued.
        f.seal_issued_graphics(p.assets, p.rng);
        launch(f, AIR, p.assets)?;
    }
    Ok(None)
}

/// ftLg_SpecialSHold_IASA / ftLg_SpecialAirSHold_IASA: B let go launches.
fn hold_input<const AIR: bool>(f: &mut Fighter, p: InputPhase<'_>) {
    if !f.input.current.held.intersects(Buttons::B) {
        launch(f, AIR, p.assets).expect("Green Missile launch assets");
    }
}

/// ftLg_SpecialSHold_Phys: ft_80084F3C.
fn hold_ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::guard_on(f, p);
}

/// ftLg_SpecialSHold_Coll: off the floor, ftLg_SpecialSHold_GroundToAir.
fn hold_ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::grounded(f, &mut p) {
        let assets = p.assets.expect("Green Missile collision assets");
        common::ground_to_air(f, AIR_HOLD, HOLD_FLAGS, assets)?;
    }
    Ok(())
}

/// ftLg_SpecialAirSHold_Coll: landing, ftLg_SpecialAirSHold_AirToGround.
fn hold_air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::lands(f, &mut p) {
        let assets = p.assets.expect("Green Missile landing assets");
        common::air_to_ground(f, HOLD, HOLD_FLAGS, assets)?;
    }
    Ok(())
}

/// ftLg_SpecialSLaunch_Enter (80143528) / ftLg_SpecialAirSLaunch_Enter,
/// and the misfire entries they defer to (ftLg_SpecialSMisfire_Enter,
/// 801439A8): the motion, then cmd_vars[0] and ftCommon_8007DB24, then the
/// launch sparks (x2219_b0 was just cleared), the hitlag callbacks and no
/// accessory.
fn launch(f: &mut Fighter, air: bool, assets: &FighterAssets) -> Result<()> {
    let state = match (air, scratch(f).misfire) {
        (false, false) => LAUNCH,
        (false, true) => MISFIRE,
        (true, false) => AIR_LAUNCH,
        (true, true) => AIR_MISFIRE,
    };
    change(f, state, 0, 0.0, 1.0, assets)?;
    f.commands.variables[0] = 0;
    common::remove_effects(f);
    if !f.effect_state.destroy_on_state_change {
        let bone = usize::from(
            assets.parts.part_to_joint[FtPart::HipN as usize].expect("Green Missile hip bone"),
        );
        f.effects.push(EffectRequest::SyncAttached {
            id: LAUNCH_SPARKS,
            bone,
        });
        f.effect_state.destroy_on_state_change = true;
    }
    f.effect_state.hitlag_callbacks = true;
    f.character.get_mut::<Luigi>().accessory = Accessory::None;
    Ok(())
}

/// ftLg_SpecialS_Anim (80143258) and the aerial and misfire rows: while
/// hitbox 0 is freshly enabled (and no misfire), its damage follows the
/// charge; the script's cmd_vars[0] sends Luigi flying (the grounded rows
/// leave the floor first, ftCommon_8007D5D4).
fn launch_anim<const AIR: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let enabled = f.commands.hitboxes[0]
        .as_ref()
        .is_some_and(|hit| hit.phase == melee_coll::hitbox::CapsulePhase::Enabled);
    if !scratch(f).misfire && enabled {
        let a = attributes(f);
        let (base, per_charge) = (a.base_damage, a.damage_per_charge);
        let charge = *charge(f) as f32;
        // Retail 801432B8 fmadds, then __cvt_fp2unsigned.
        let damage = gekko_math::fma::fmadds(charge, per_charge, base);
        set_hitbox_damage(f, damage as u32);
    }
    if f.commands.variables[0] != 0 {
        // The script frame that set cmd_vars[0] also queued the launch's
        // effects (0x40E, the fire); ftLg_SpecialSFly_Enter's motion change
        // flushes them (fighter.c:951) before the fire colour program's
        // step queues its own.
        f.seal_issued_graphics(p.assets, p.rng);
        if !AIR {
            f.leave_ground();
        }
        fly(f, p.assets)?;
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
        "ftCo_800DEEB8: charged Green Missile"
    );
    let damage = damage as f32;
    let staled = f.commands.stale_damage(damage);
    let hit = f.commands.hitboxes[0]
        .as_mut()
        .expect("Green Missile hitbox");
    hit.knockback_damage = gekko_math::msl::fctiwz(damage) as u32;
    hit.descriptor.damage = staled;
}

/// ftLg_SpecialS_Phys: ft_80084F3C.
fn launch_ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::guard_on(f, p);
}

/// ftLg_SpecialAirSMisfire_Phys (80143830): ft_80084F3C, the grounded
/// friction, even in the air.
fn misfire_air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    common::ground_friction_movement(f, &p);
    common::finish_update(f, &p);
}

/// ftLg_SpecialS_Coll / ftLg_SpecialSMisfire_Coll: off the floor, the
/// aerial counterpart (ftLg_SpecialSLaunch_GroundToAir /
/// ftLg_SpecialSMisfire_GroundToAir).
fn launch_ground_collision<const IS_MISFIRE: bool>(
    f: &mut Fighter,
    mut p: CollisionPhase<'_>,
) -> Result<()> {
    if !common::grounded(f, &mut p) {
        let assets = p.assets.expect("Green Missile collision assets");
        let state = if IS_MISFIRE { AIR_MISFIRE } else { AIR_LAUNCH };
        common::ground_to_air(f, state, LAUNCH_FLAGS, assets)?;
    }
    Ok(())
}

/// ftLg_SpecialAirS_Coll / ftLg_SpecialAirSMisfire_Coll: landing, the
/// grounded counterpart.
fn launch_air_collision<const IS_MISFIRE: bool>(
    f: &mut Fighter,
    mut p: CollisionPhase<'_>,
) -> Result<()> {
    if common::lands(f, &mut p) {
        let assets = p.assets.expect("Green Missile landing assets");
        let state = if IS_MISFIRE { MISFIRE } else { LAUNCH };
        common::air_to_ground(f, state, LAUNCH_FLAGS, assets)?;
    }
    Ok(())
}

/// ftLg_SpecialSFly_Enter (80143C60): the misfire's fixed speeds, else the
/// launch speed grown with the charge (retail 80143CC0 fmadds, then fmuls
/// by the facing) and the rise half the vertical attribute plus its share
/// of the charge (80143D1C..28: fmuls, fdivs, fmuls, fmadds). The flight
/// continues the current frame; deal_dmg_cb = ftLg_SpecialS_OnGiveDamage
/// while it lasts. x21F8 (ftCommon_8007F76C) only runs after a cape
/// turnaround, not modelled.
fn fly(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    f.commands.variables[0] = 0;
    let a = attributes(f);
    let (speed, per_charge, rise, maximum, misfire_speed, misfire_rise) = (
        a.launch_speed,
        a.launch_speed_per_charge,
        a.launch_vertical_speed,
        a.max_charge_frames,
        a.misfire_speed,
        a.misfire_vertical_speed,
    );
    let GreenMissile {
        charge, misfire, ..
    } = *scratch(f);
    let charge = charge as f32;
    f.physics.self_velocity.x = if misfire {
        misfire_speed
    } else {
        gekko_math::fma::fmadds(per_charge, charge, speed)
    };
    f.physics.self_velocity.x *= f.physics.facing;
    f.physics.self_velocity.y = if misfire {
        misfire_rise
    } else {
        let share = rise * ((0.5 * charge) / maximum);
        gekko_math::fma::fmadds(0.5, rise, share)
    };
    let frame = f.animation.frame;
    change(f, FLIGHT, FLIGHT_FLAGS, frame, 1.0, assets)
}

/// ftLg_SpecialAirS2_Anim: at the animation's end, the aerial ending.
fn flight_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        enter_air_end(f, p.assets)?;
    }
    Ok(None)
}

/// ftLg_SpecialAirS2_Phys: the flight's gravity until the script's
/// cmd_vars[0], the slowdown's after it along with its friction.
fn flight_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let a = attributes(f);
    let (flight, slowdown, terminal, friction) = (
        a.flight_gravity,
        a.end_gravity,
        a.flight_terminal_velocity,
        a.end_friction,
    );
    let slowing = f.commands.variables[0] != 0;
    common::fall(f, if slowing { slowdown } else { flight }, terminal);
    if slowing {
        common::air_friction(f, friction);
    }
    common::finish_update(f, &p);
}

/// ftLg_SpecialAirS2_Coll: landing ends on the ground (ftCommon_8007D7FC,
/// ftLg_SpecialSEnd_Enter); a wall, touched in the same pass, ends in the
/// air even after that landing.
fn flight_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    let assets = p.assets.expect("Green Missile flight collision assets");
    if common::lands(f, &mut p) {
        f.land();
        enter_ground_end(f, assets)?;
    }
    let env = f.collision.data.env_flags as u32;
    if env & (collide::LEFT_WALL_MASK | collide::RIGHT_WALL_MASK) != 0 {
        enter_air_end(f, assets)?;
    }
    Ok(())
}

/// ftLg_SpecialS_OnGiveDamage (80142C00), deal_dmg_cb during the flight:
/// the hit stops Luigi (a rise too) and ends the move in the air.
pub fn deal_damage(f: &mut Fighter, assets: &FighterAssets) {
    f.physics.self_velocity.x = 0.0;
    if f.physics.self_velocity.y >= 0.0 {
        f.physics.self_velocity.y = 0.0;
    }
    enter_air_end(f, assets).expect("Green Missile hit assets");
}

/// Whether `action` installed ftLg_SpecialS_OnGiveDamage as deal_dmg_cb.
pub fn deals_damage(action: ActionId) -> bool {
    action == FLIGHT
}

/// ftLg_SpecialSEnd_Enter (80143F18): the ground speed divided (fdivs),
/// then the grounded ending.
fn enter_ground_end(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    f.commands.variables[0] = 0;
    let divisor = attributes(f).end_speed_divisor;
    f.physics.ground_velocity /= divisor;
    change(f, END, 0, 0.0, 1.0, assets)
}

/// ftLg_SpecialAirSEnd_Enter: the aerial ending with the horizontal speed
/// divided (fdivs).
fn enter_air_end(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    f.commands.variables[0] = 0;
    let divisor = attributes(f).end_speed_divisor;
    f.physics.self_velocity.x /= divisor;
    change(f, AIR_END, 0, 0.0, 1.0, assets)
}

/// ftLg_SpecialSEnd_Anim / ftLg_SpecialAirSEnd_Anim: Wait or Fall.
fn end_anim<const AIR: bool>(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::finish(f, AIR, p.assets)?;
    }
    Ok(None)
}

/// ftLg_SpecialSEnd_Phys (80143DFC): the ending's friction.
fn end_ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let friction = attributes(f).end_friction;
    common::ground_friction(f, friction);
    common::apply_ground_movement(f, &p);
    common::finish_update(f, &p);
}

/// ftLg_SpecialAirSEnd_Phys: the ending's gravity (common terminal
/// velocity) and friction.
fn end_air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let a = attributes(f);
    let (gravity, friction) = (a.end_gravity, a.end_friction);
    let terminal = f.attributes.air.terminal_velocity;
    common::fall(f, gravity, terminal);
    common::air_friction(f, friction);
    common::finish_update(f, &p);
}

/// ftLg_SpecialSEnd_Coll (80143E8C): off the floor, Fall.
fn end_ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::grounded(f, &mut p) {
        let assets = p.assets.expect("Green Missile collision assets");
        f.change_motion_state(melee_types::CommonMotionState::Fall.into(), assets)?;
    }
    Ok(())
}

/// ftLg_SpecialAirSEnd_Coll: landing (ftCommon_8007D7FC), the grounded
/// ending.
fn end_air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::lands(f, &mut p) {
        f.land();
        enter_ground_end(f, p.assets.expect("Green Missile landing assets"))?;
    }
    Ok(())
}
