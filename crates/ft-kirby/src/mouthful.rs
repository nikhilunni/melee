//! The Inhale once it has caught a fighter, ftkirbyspecialn.c
//! (800F5EA8..800F9070): the pull (SpecialNCapture0), the gulp (Eat) and
//! Kirby standing, walking, turning, jumping, landing and falling with a
//! mouthful. Swallowing and spitting are in `swallow.rs`; the caught
//! fighter's own states are common code (`melee_ft::fighter::capture_kirby`).
//!
//! The rows for a caught item (SpecialNCapture1 and its aerial twin,
//! fn_800F6178 / fn_800F6280, ftKb_SpecialN_800F5DE8) are not ported.
use crate::{common, init::Kirby};
use gekko_math::msl::{fabsf, fctiwz};
use hsd_types::Vec3;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        capture_kirby::{self, CaptorRequest, Mouth},
        grab::GrabLink,
        jump::JumpInput,
        ledge::GrabExclusions,
        state::{AnimationPhase, CollisionPhase, InputPhase, MotionRow, PhysicsPhase},
        walk::{walk_tier, WalkState},
        ActionId, Fighter, MotionData, MotionEntryFlags,
    },
    input::Buttons,
};
use melee_types::GroundOrAir;

/// ftKb_MS_SpecialNCapture0 .. ftKb_MS_EatLanding (356..366) and their
/// aerial rows.
pub const CAPTURE: ActionId = ActionId(356);
pub const EAT: ActionId = ActionId(358);
pub const EAT_WAIT: ActionId = ActionId(359);
pub const EAT_WALK_SLOW: ActionId = ActionId(360);
pub const EAT_TURN: ActionId = ActionId(363);
pub const EAT_JUMP_SQUAT: ActionId = ActionId(364);
pub const EAT_JUMP: ActionId = ActionId(365);
pub const EAT_LANDING: ActionId = ActionId(366);
pub const AIR_CAPTURE: ActionId = ActionId(374);
pub const EAT_AIR: ActionId = ActionId(376);
pub const EAT_FALL: ActionId = ActionId(377);
pub const EAT_TURN_AIR: ActionId = ActionId(382);

/// ftKb_SM_EatWalkSlow/Middle/Fast: the walks' animations, whose lengths
/// the Inhale's entry keeps (u.kb.xE8..xF0).
const WALK_ANIMATIONS: [i32; 3] = [311, 312, 313];

/// Fighter_ChangeMotionState flag words of these rows.
const SKIP_MODEL: u32 = 0x10;
/// KeepGfx | SkipModel.
const KEEP_GFX_SKIP_MODEL: u32 = 0x12;
/// SkipModel | SkipMatAnim.
const SKIP_MODEL_MAT_ANIM: u32 = 0x90;
/// KeepGfx | SkipModel | SkipMatAnim.
const KEEP_GFX_SKIP_MODEL_MAT_ANIM: u32 = 0x92;
/// The grab callbacks' change: KeepGfx, SkipModel and KeepSfx.
const CAPTURE_ENTRY: u32 = 0x212;
/// ftKb_MF_SpecialN_Capture_Coll: ftCommon_GroundAirColl_MF with KeepGfx
/// and SkipModel.
pub(crate) const CAPTURE_COUNTERPART: u32 = 0x0C4C_5092;
/// ftKb_MF_SpecialN_EatFall_Coll: ftCommon_GroundAirColl_MF with SkipModel.
const EAT_FALL_COUNTERPART: u32 = 0x0C4C_5090;

/// fp->mv.kb.specialhi.x0 / x4 as the mouthful's jump uses them
/// (mv.co.kneebend, then mv.co.jump).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MouthfulJump {
    /// x0: the jump input was let go during the squat.
    pub short_hop: bool,
    /// x4 in the squat: what asked for the jump.
    pub input: JumpInput,
    /// x4 in the air: ftCo_Jump_Phys_Inner has skipped its first tick.
    pub physics_started: bool,
}
impl Default for MouthfulJump {
    fn default() -> Self {
        Self {
            short_hop: false,
            input: JumpInput::Buttons,
            physics_started: false,
        }
    }
}

pub const fn rows() -> [MotionRow; 14] {
    use melee_ft::fighter::state::callbacks::physics;
    [
        common::row(
            CAPTURE,
            308,
            capture_anim,
            common::no_input,
            physics::guard_on,
            capture_collision,
        ),
        common::row(
            EAT,
            309,
            eat_anim,
            common::no_input,
            physics::guard_on,
            eat_collision,
        ),
        common::row(
            EAT_WAIT,
            310,
            idle_anim,
            wait_input,
            physics::guard_on,
            wait_collision,
        ),
        walk_row(0),
        walk_row(1),
        walk_row(2),
        common::row(
            EAT_TURN,
            317,
            turn_anim,
            common::no_input,
            physics::guard_on,
            turn_collision,
        ),
        common::row(
            EAT_JUMP_SQUAT,
            314,
            jump_squat_anim,
            jump_squat_input,
            physics::guard_on,
            walk_collision,
        ),
        common::row(
            EAT_JUMP,
            315,
            idle_anim,
            common::no_input,
            jump_physics,
            fall_collision,
        ),
        common::row(
            EAT_LANDING,
            316,
            landing_anim,
            common::no_input,
            physics::guard_on,
            walk_collision,
        ),
        common::row(
            AIR_CAPTURE,
            308,
            capture_anim,
            common::no_input,
            physics::air_friction,
            air_capture_collision,
        ),
        common::row(
            EAT_AIR,
            309,
            eat_air_anim,
            common::no_input,
            physics::air_friction,
            eat_air_collision,
        ),
        common::row(
            EAT_FALL,
            310,
            idle_anim,
            fall_input,
            physics::air_friction,
            fall_collision,
        ),
        common::row(
            EAT_TURN_AIR,
            317,
            turn_air_anim,
            common::no_input,
            physics::air_friction,
            turn_air_collision,
        ),
    ]
}

/// EatWalkSlow, EatWalkMiddle and EatWalkFast (360..362).
const fn walk_row(tier: u16) -> MotionRow {
    common::row(
        ActionId(EAT_WALK_SLOW.0 + tier),
        WALK_ANIMATIONS[tier as usize],
        walk_anim,
        walk_input,
        melee_ft::fighter::state::callbacks::physics::walk,
        walk_collision,
    )
}

/// ftKb_SpecialN_800F9070 (800F9070): the take-damage and death callbacks.
pub(crate) fn install_callbacks(f: &mut Fighter) {
    f.character.get_mut::<Kirby>().damage_callbacks = true;
}

/// ftCommon_8007E2F4(fp, 0x1FF).
pub(crate) fn ungrabbable(f: &mut Fighter) {
    f.status.grab_exclusions = GrabExclusions::ALL;
}

/// `fp->victim_gobj != NULL`.
pub(crate) fn holding(f: &Fighter) -> bool {
    matches!(f.combat.grab, Some(GrabLink::Holding { .. }))
}

pub(crate) fn change(
    f: &mut Fighter,
    state: ActionId,
    flags: u32,
    a: &FighterAssets,
) -> Result<()> {
    f.change_motion_state_with_flags(state, a, MotionEntryFlags(flags), 0.0, 1.0)
}

/// A ground/air counterpart at the current frame, rate 1, with the
/// callbacks again.
pub(crate) fn counterpart(
    f: &mut Fighter,
    state: ActionId,
    flags: u32,
    assets: &FighterAssets,
) -> Result<()> {
    let frame = f.animation.frame;
    f.change_motion_state_with_flags(state, assets, MotionEntryFlags(flags), frame, 1.0)?;
    install_callbacks(f);
    Ok(())
}

/// What the caught fighter reads of Kirby's attributes while it is held.
fn mouth(f: &Fighter) -> Mouth {
    let a = &f.character.get::<Kirby>().attributes.inhale;
    Mouth {
        offset: a.mouth_offset,
        shrink_radius: a.shrink_radius,
        shrink: a.shrink,
        pull_step: a.pull_step,
        hold_time: a.hold_time,
        hold_decay: a.hold_decay,
        mash_decrement: a.hold_mash_decrement,
        steer_speed: a.steer_speed,
    }
}

/// fn_800F6210 (800F6210) / fn_800F6318 (800F6318), the Inhale's grab_cb on
/// a fighter, then ftCo_800BD1DC, the grabbed_cb it installed.
pub fn grab(
    kirby: &mut Fighter,
    victim: &mut Fighter,
    kirby_assets: &FighterAssets,
    victim_assets: &FighterAssets,
) -> Result<()> {
    let air = matches!(
        kirby.motion_state.action,
        crate::special_n::AIR_START | crate::special_n::AIR_LOOP
    );
    change(
        kirby,
        if air { AIR_CAPTURE } else { CAPTURE },
        CAPTURE_ENTRY,
        kirby_assets,
    )?;
    // x2222_b2; u.kb.xF4_b0 = false (a fighter, not an item).
    let state = kirby.character.get_mut::<Kirby>();
    state.cape_turn_blocked = true;
    state.mouthful_ability = capture_kirby::ability(victim);
    ungrabbable(kirby);
    // ftColl_80078A2C chose the victim (fp->victim_gobj).
    kirby.combat.grab = Some(GrabLink::Holding {
        victim: victim.spawn_number,
        vertical_offset: 0.0,
    });
    let mouth = mouth(kirby);
    capture_kirby::enter(victim, kirby, victim_assets, mouth)
}

/// ftKb_SpecialNCapture_Anim (800F7498) / ftKb_SpecialN_800F5EA8 (800F5EA8):
/// once the caught fighter is within reach of the mouth it is taken in
/// (ftCo_800BD620) and Kirby gulps.
fn capture_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let a = &f.character.get::<Kirby>().attributes.inhale;
    let position = f.physics.position;
    // retail 0x800F5EF4: fmadds; 0x800F5F04: fadds.
    let mouth = Vec3::new(
        gekko_math::fma::fmadds(a.mouth_offset.x, f.physics.facing, position.x),
        position.y + a.mouth_offset.y,
        position.z,
    );
    let reach = a.close_distance * a.close_distance;
    let Some(victim) = f.core.partner_position else {
        unimplemented!("ftKb_SpecialN_800F5EA8 (ftkirbyspecialn.c:323): the pull with no victim");
    };
    if capture_kirby::distance_squared(victim, mouth) < reach {
        f.combat.mouth_request = Some(CaptorRequest::Close);
        common::seal_graphics(f, p.assets, p.rng);
        let state = if f.physics.ground_or_air == GroundOrAir::Air {
            EAT_AIR
        } else {
            EAT
        };
        change(f, state, SKIP_MODEL, p.assets)?;
        install_callbacks(f);
        ungrabbable(f);
    }
    Ok(None)
}

/// The rows whose Anim callback is empty.
fn idle_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    Ok(None)
}

/// EatWait, from Eat, EatTurn, EatLanding and the walk.
fn enter_wait(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    change(f, EAT_WAIT, SKIP_MODEL, assets)?;
    install_callbacks(f);
    f.step_animation(assets);
    ungrabbable(f);
    Ok(())
}

/// fn_800F6450 (800F6450) and the aerial rows' ends: EatFall.
fn enter_fall(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    f.leave_ground();
    change(f, EAT_FALL, SKIP_MODEL_MAT_ANIM, assets)?;
    install_callbacks(f);
    f.step_animation(assets);
    ungrabbable(f);
    Ok(())
}

/// ftKb_Eat_Anim (800F75F8): the gulp ends in EatWait.
fn eat_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::seal_graphics(f, p.assets, p.rng);
        enter_wait(f, p.assets)?;
    }
    Ok(None)
}

/// ftKb_SpecialAirNCaptured_Anim (800F7674): the aerial gulp ends in
/// EatFall.
fn eat_air_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::seal_graphics(f, p.assets, p.rng);
        enter_fall(f, p.assets)?;
    }
    Ok(None)
}

/// ftKb_EatWait_IASA's and ftKb_SpecialAirNCaptureWait_IASA's first two
/// inlines: B, or the stick down past +78, swallows; A spits.
fn swallow_or_spit(f: &mut Fighter, air: bool, assets: &FighterAssets) -> Result<bool> {
    if !holding(f) {
        // `fp->victim_gobj != NULL` fails: neither. Kirby leaves these rows
        // whenever the pair parts (CatchCut, or Fall when the victim dies).
        unimplemented!(
            "ftKb_EatWait_IASA (ftkirbyspecialn.c:1211-1243): a mouthful's wait with no victim"
        );
    }
    let range = f
        .character
        .get::<Kirby>()
        .attributes
        .inhale
        .swallow_stick_range;
    let pressed = f.input.pressed;
    if pressed.intersects(Buttons::B) || f.input.current.stick.y < -range {
        crate::swallow::enter_swallow(f, air, assets)?;
        return Ok(true);
    }
    if pressed.intersects(Buttons::A) {
        crate::swallow::enter_spit(f, air, assets)?;
        return Ok(true);
    }
    Ok(false)
}

/// The IASAs' turn inline: a stick past +74 away from the facing.
fn turn_requested(f: &Fighter) -> bool {
    let range = f
        .character
        .get::<Kirby>()
        .attributes
        .inhale
        .turn_stick_range;
    let mut stick = f.input.current.stick.x;
    if fabsf(stick) < range {
        stick = 0.0;
    }
    (stick < 0.0 && f.physics.facing == 1.0) || (stick > 0.0 && f.physics.facing == -1.0)
}

/// ftCo_Jump_GetInput (800CAE80): the stick takes priority over X and Y.
fn jump_input(f: &Fighter, assets: &FighterAssets) -> Option<JumpInput> {
    let t = &assets.input.thresholds;
    if f.input.current.stick.y >= t.tap_jump_threshold
        && i32::from(f.input.vertical.tilt) < t.tap_jump_window
    {
        Some(JumpInput::Stick)
    } else if f.input.pressed.intersects(Buttons::XY) {
        Some(JumpInput::Buttons)
    } else {
        None
    }
}

/// The jump squat with a mouthful, from EatWait and the walk.
fn enter_jump_squat(f: &mut Fighter, input: JumpInput, assets: &FighterAssets) -> Result<()> {
    f.character.get_mut::<Kirby>().mouthful_jump = MouthfulJump {
        short_hop: false,
        input,
        physics_started: false,
    };
    change(f, EAT_JUMP_SQUAT, KEEP_GFX_SKIP_MODEL_MAT_ANIM, assets)?;
    install_callbacks(f);
    f.step_animation(assets);
    ungrabbable(f);
    Ok(())
}

/// fn_800F64C8 (800F64C8) -> ftWalkCommon_800DFCA4 on the three walks with
/// the multiplier +7C, from `frame`.
fn enter_walk(f: &mut Fighter, frame: f32, assets: &FighterAssets) -> Result<()> {
    let multiplier = f.character.get::<Kirby>().attributes.inhale.walk_multiplier;
    let tier = walk_tier(
        f.physics.ground_velocity,
        f.attributes.walking.walk_max_vel,
        multiplier,
        &assets.movement,
    );
    f.change_motion_state_with_flags(
        ActionId(EAT_WALK_SLOW.0 + tier),
        assets,
        MotionEntryFlags(SKIP_MODEL),
        frame,
        1.0,
    )?;
    f.step_animation(assets);
    f.core.state_data = MotionData::Walk(WalkState {
        slippery_animation_velocity: f.physics.ground_velocity,
        base_motion: i32::from(EAT_WALK_SLOW.0),
        acceleration_multiplier: multiplier,
    });
    ungrabbable(f);
    Ok(())
}

/// ftKb_EatWait_IASA (800F8060): swallow or spit, the turn, the jump, the
/// walk (ftWalkCommon_800DFC70).
fn wait_input(f: &mut Fighter, p: InputPhase<'_>) {
    wait_actions(f, p.assets).expect("mouthful assets");
}

fn wait_actions(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    if swallow_or_spit(f, false, assets)? {
        return Ok(());
    }
    if turn_requested(f) {
        change(f, EAT_TURN, KEEP_GFX_SKIP_MODEL_MAT_ANIM, assets)?;
        install_callbacks(f);
        f.step_animation(assets);
        ungrabbable(f);
        return Ok(());
    }
    if let Some(input) = jump_input(f, assets) {
        return enter_jump_squat(f, input, assets);
    }
    if f.input.current.stick.x * f.physics.facing >= assets.input.thresholds.walk_stick_threshold {
        enter_walk(f, 0.0, assets)?;
    }
    Ok(())
}

/// ftKb_SpecialAirNCaptureWait_IASA (800F844C): swallow or spit, the turn.
fn fall_input(f: &mut Fighter, p: InputPhase<'_>) {
    fall_actions(f, p.assets).expect("mouthful assets");
}

fn fall_actions(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    if swallow_or_spit(f, true, assets)? {
        return Ok(());
    }
    if turn_requested(f) {
        change(f, EAT_TURN_AIR, KEEP_GFX_SKIP_MODEL_MAT_ANIM, assets)?;
        install_callbacks(f);
        f.step_animation(assets);
        ungrabbable(f);
    }
    Ok(())
}

/// ftKb_EatTurn_Anim (800F7D58): at the animation's end Kirby faces the
/// other way in EatWait.
fn turn_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.physics.facing = -f.physics.facing;
        common::seal_graphics(f, p.assets, p.rng);
        enter_wait(f, p.assets)?;
    }
    Ok(None)
}

/// ftKb_SpecialAirNCaptureTurn_Anim (800F7DE4): the same into EatFall.
fn turn_air_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.physics.facing = -f.physics.facing;
        common::seal_graphics(f, p.assets, p.rng);
        enter_fall(f, p.assets)?;
    }
    Ok(None)
}

/// ftKb_EatWalk_Anim (800F7E78) -> ftWalkCommon_800DFDDC: the walk's rate
/// from its speed over the tier's own (co_attrs slow_walk_max,
/// mid_walk_point, fast_walk_min).
fn walk_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let c = &mut f.core;
    let MotionData::Walk(walk) = &c.state_data else {
        panic!("mouthful walk data missing")
    };
    let slippery =
        melee_ft::physics::grounded::ground_friction_multiplier(&c.physics, &c.collision.data)
            < 1.0;
    if slippery {
        unimplemented!(
            "ftWalkCommon_800DFDDC (ftwalkcommon.c:106-107): a mouthful walked over a slippery floor"
        );
    }
    let speed = c.physics.ground_velocity;
    let rate = if speed * c.physics.facing <= 0.0 {
        unimplemented!(
            "ftWalkCommon_800DFDDC (ftwalkcommon.c:111-112): a mouthful's walk with no forward speed"
        );
    } else {
        let w = &c.attributes.walking;
        let rates = [w.slow_walk_max, w.mid_walk_point, w.fast_walk_min];
        let tier = i32::from(c.motion_state.action.0) - walk.base_motion;
        fabsf(speed) / rates[tier as usize]
    };
    c.animation.set_rate(&mut c.skeleton, rate, false);
    Ok(None)
}

/// ftKb_EatWalk_IASA (800F8770): the jump, EatWait once the stick lets go
/// (ft_8008A1FC), else ftWalkCommon_800DFEC8: a new speed tier re-enters
/// the walk at the same phase of the new animation.
fn walk_input(f: &mut Fighter, p: InputPhase<'_>) {
    walk_actions(f, p.assets).expect("mouthful assets");
}

fn walk_actions(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    if let Some(input) = jump_input(f, assets) {
        return enter_jump_squat(f, input, assets);
    }
    let stick = f.input.current.stick.x;
    if stick * f.physics.facing < 0.0 || fabsf(stick) < assets.input.thresholds.walk_stick_threshold
    {
        return enter_wait(f, assets);
    }
    let MotionData::Walk(walk) = &f.core.state_data else {
        panic!("mouthful walk data missing")
    };
    let tier = walk_tier(
        f.physics.ground_velocity,
        f.attributes.walking.walk_max_vel,
        walk.acceleration_multiplier,
        &assets.movement,
    );
    if walk.base_motion + i32::from(tier) == i32::from(f.motion_state.action.0) {
        return Ok(());
    }
    let duration = assets.motions[&f.animation.motion_id].animation.frames;
    let target = assets.motions[&WALK_ANIMATIONS[usize::from(tier)]]
        .animation
        .frames;
    let quotient = fctiwz(f.animation.frame / duration) as f32;
    // retail 0x800E0010: fnmsubs.
    let phase = gekko_math::fma::fnmsubs(duration, quotient, f.animation.frame);
    let frame = fctiwz(target * (phase / duration)) as f32;
    enter_walk(f, frame, assets)
}

/// ftKb_EatJump1_IASA (800F8870) -> ftCo_KneeBend_Check_ShortHop: letting
/// go of what asked for the jump makes it a short hop.
fn jump_squat_input(f: &mut Fighter, p: InputPhase<'_>) {
    let jump = f.character.get::<Kirby>().mouthful_jump;
    let released = match jump.input {
        JumpInput::Buttons => !f.input.current.held.intersects(Buttons::XY),
        JumpInput::Stick => f.input.current.stick.y < p.assets.jumping.release_threshold,
        JumpInput::CStick => unreachable!("ftCo_Jump_GetInput never names the C-stick"),
    };
    if released {
        f.character.get_mut::<Kirby>().mouthful_jump.short_hop = true;
    }
}

/// ftKb_EatJump1_Anim (800F7E98): at the squat's end Kirby leaves the floor
/// in EatJump2 with ftCo_800CB110's take-off scaled by +80.
fn jump_squat_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::seal_graphics(f, p.assets, p.rng);
        f.leave_ground();
        change(f, EAT_JUMP, KEEP_GFX_SKIP_MODEL_MAT_ANIM, p.assets)?;
        install_callbacks(f);
        f.step_animation(p.assets);
        take_off(f);
    }
    Ok(None)
}

/// ftCo_800CB110 (800CB110) with the mouthful's multiplier: retail
/// 800CB140..800CB1B8, every product and the sum rounded separately.
fn take_off(f: &mut Fighter) {
    let kirby = f.character.get_mut::<Kirby>();
    kirby.mouthful_jump.physics_started = false;
    let short_hop = kirby.mouthful_jump.short_hop;
    let multiplier = kirby.attributes.inhale.jump_multiplier;
    let attrs = &f.core.attributes.jumping;
    let momentum = f.core.physics.self_velocity.x
        * (attrs.ground_to_air_jump_momentum_multiplier * multiplier);
    let stick = multiplier * (f.core.input.current.stick.x * attrs.jump_h_initial_velocity);
    let vertical = if short_hop {
        attrs.hop_v_initial_velocity * multiplier
    } else {
        attrs.jump_v_initial_velocity * multiplier
    };
    let horizontal = momentum + stick;
    let maximum = attrs.jump_h_max_velocity * multiplier;
    if fabsf(horizontal) > maximum {
        unimplemented!(
            "ftCo_800CB110 (ftCo_Jump.c:133-139): a mouthful's take-off past the scaled speed limit"
        );
    }
    f.core.physics.self_velocity = Vec3::new(horizontal, vertical, 0.0);
    f.core.input.vertical.tilt = 0xFE;
}

/// ftKb_EatJump2_Phys (800F8BD0) -> ftCo_Jump_Phys_Inner: nothing on the
/// first tick, then ft_80084DB0.
fn jump_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let jump = &mut f.character.get_mut::<Kirby>().mouthful_jump;
    if !jump.physics_started {
        jump.physics_started = true;
        f.core.finish_air_update(p.assets, p.wind);
        return;
    }
    melee_ft::fighter::state::callbacks::physics::pass(f, p);
}

/// ftKb_EatLanding_Anim (800F7F24): the landing ends in EatWait.
fn landing_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::seal_graphics(f, p.assets, p.rng);
        enter_wait(f, p.assets)?;
    }
    Ok(None)
}

/// ftKb_SpecialNCapture_Coll (800F8D28) -> fn_800F697C.
fn capture_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::grounded(f, &mut p) {
        return Ok(());
    }
    // SpecialAirNCapture0 at this frame with ftKb_MF_SpecialN_Capture_Coll.
    // Kirby would have to slide off a floor's end in the few ticks of the
    // pull; no scene does.
    unimplemented!("fn_800F697C (ftkirbyspecialn.c:565-571): the pull leaving the floor");
}

/// ftKb_SpecialAirNCapture_Coll (800F8D78) -> fn_800F6A5C.
fn air_capture_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    f.land();
    let frame = f.animation.frame;
    f.change_motion_state_with_flags(
        CAPTURE,
        p.assets.expect("Inhale assets"),
        MotionEntryFlags(CAPTURE_COUNTERPART),
        frame,
        1.0,
    )?;
    ungrabbable(f);
    Ok(())
}

/// ftKb_Eat_Coll (800F8DA0) -> fn_800F6AC8.
fn eat_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::grounded(f, &mut p) {
        return Ok(());
    }
    // EatAir at this frame with ftKb_MF_SpecialN_Capture_Coll and the
    // callbacks. As the pull: no scene slides Kirby off during the gulp.
    unimplemented!("fn_800F6AC8 (ftkirbyspecialn.c:591-598): the gulp leaving the floor");
}

/// ftKb_SpecialAirNCaptured_Coll (800F8DC8) -> fn_800F6B3C.
fn eat_air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    f.land();
    counterpart(
        f,
        EAT,
        CAPTURE_COUNTERPART,
        p.assets.expect("Inhale assets"),
    )?;
    ungrabbable(f);
    Ok(())
}

/// ftKb_SpecialS_800F6BB0 (800F6BB0): EatFall at EatWait's frame, off the
/// floor's end or hopped off it by the held fighter.
pub(crate) fn leave_floor(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    f.leave_ground();
    counterpart(f, EAT_FALL, EAT_FALL_COUNTERPART, assets)?;
    ungrabbable(f);
    Ok(())
}

/// ftKb_SpecialN_800F5A38 (800F5A38): the held fighter's stick moves Kirby
/// in EatWait and EatFall.
pub(crate) fn steerable(f: &Fighter) -> bool {
    matches!(f.motion_state.action, EAT_WAIT | EAT_FALL)
}

/// ftKb_EatWait_Coll (800F8DF0) -> ftKb_SpecialS_800F6BB0.
fn wait_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::grounded(f, &mut p) {
        return Ok(());
    }
    leave_floor(f, p.assets.expect("Inhale assets"))
}

/// ftKb_SpecialAirNCaptureWait_Coll (800F8E18) and ftKb_EatJump2_Coll
/// (800F9020) -> fn_800F6528: EatLanding.
fn fall_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Inhale assets");
    f.land();
    change(f, EAT_LANDING, KEEP_GFX_SKIP_MODEL, assets)?;
    install_callbacks(f);
    f.step_animation(assets);
    Ok(())
}

/// ftKb_EatWalk_Coll (800F8FD0), ftKb_EatJump1_Coll (800F8FF8) and
/// ftKb_EatLanding_Coll (800F9048) -> fn_800F6450: EatFall from its start.
fn walk_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::grounded(f, &mut p) {
        return Ok(());
    }
    enter_fall(f, p.assets.expect("Inhale assets"))
}

/// ftKb_EatTurn_Coll (800F8F80) -> fn_800F6FC4.
fn turn_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::grounded(f, &mut p) {
        return Ok(());
    }
    f.leave_ground();
    counterpart(
        f,
        EAT_TURN_AIR,
        CAPTURE_COUNTERPART,
        p.assets.expect("Inhale assets"),
    )
}

/// ftKb_SpecialAirNCaptureTurn_Coll (800F8FA8) -> fn_800F702C.
fn turn_air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    f.land();
    counterpart(
        f,
        EAT_TURN,
        CAPTURE_COUNTERPART,
        p.assets.expect("Inhale assets"),
    )?;
    ungrabbable(f);
    Ok(())
}
