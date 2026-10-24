//! The fighter Kirby inhales: ftCo_CaptureKirby.c, ftCo_CaptureWaitKirby.c
//! and ftCo_ThrownKirby.c (800BD19C..800BE7C0).
//!
//! The Inhale's wind pulls the fighter toward Kirby's mouth, shrinking it
//! as it nears (CaptureKirby, placed each tick by accessory1 ftCo_800BD39C).
//! Once Kirby's own state finds it close enough it is pinned to his TransN2,
//! hidden and intangible (CaptureWaitKirby); a hold timer runs down, faster
//! for mashing, and its stick moves Kirby from inside. Kirby then spits it
//! out as a star that flies along and hits (ThrownKirbyStar) or swallows it
//! and lets it out as the copy star (ThrownCopyStar); either ends in
//! ThrownKirby, where the fighter grows back to its size.
//!
//! Kirby's own states live in ft-kirby; he hands the victim his attributes
//! through [`Mouth`] and [`StarParameters`] (the ftKb_SpecialN_800F58AC ..
//! 800F5B5C accessors).
use super::{
    assets::{FighterAssets, Result},
    capture_yoshi::grab_mash,
    grab::GrabLink,
    grab_throw,
    ledge::GrabExclusions,
    state::{AnimationPhase, CollisionPhase, PhysicsPhase},
    Fighter, FighterCore, MotionData,
};
use crate::anim::WaitChoice;
use hsd_types::{Vec2, Vec3};
use melee_types::{combat::HurtStatus, mp::FtCollisionBox, CommonMotionState as S, GroundOrAir};

/// ftCo_SM_ThrownCopyStar / ftCo_SM_ThrownKirbyStar: Kirby's animations and
/// scripts the star plays.
pub const COPY_STAR_MOTION: i32 = 284;
pub const SPIT_STAR_MOTION: i32 = 285;

/// x670 / x671_timer_lstick_tilt: a tilt the struggle has used.
const STICK_TIMER_SPENT: u8 = 254;

/// What the inhaled fighter reads from its captor's ftKb_DatAttrs while it
/// is pulled in and held (ftKb_SpecialN_800F5AF0..800F5B5C, 800F598C,
/// 800F5A04): constant for the hold, so Kirby hands them over at the catch.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Mouth {
    /// +38, +3C: the mouth from Kirby's position, ahead and up.
    pub offset: Vec2,
    /// +40: inside this distance from the mouth the fighter shrinks.
    pub shrink_radius: f32,
    /// +44: the share of its size it has lost at the mouth.
    pub shrink: f32,
    /// +48, +4C: how far the pull moves it each tick, across and up.
    pub pull_step: Vec2,
    /// +5C: the hold timer's start (ftCommon_InitGrab).
    pub hold_time: f32,
    /// +58: taken from the timer each tick.
    pub hold_decay: f32,
    /// +54: taken per mash input (ftCommon_GrabMash).
    pub mash_decrement: f32,
    /// +84: the speed a tap of the held fighter's stick gives Kirby.
    pub steer_speed: f32,
}

/// mv.co.capturekirby (Fighter +2340) with the grab timer and mash state.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CaptureKirbyState {
    pub mouth: Mouth,
    /// pos_offset: the fighter from the mouth.
    pub offset: Vec2,
    /// scale (+2C): the root joint's scale at the catch.
    pub root_scale: Vec3,
    /// Fighter.grab_timer.
    pub timer: f32,
    /// x1A50 / x1A51: the last stick direction on each axis.
    stick_directions: [i8; 2],
    /// x18: this tick's ftCommon_GrabMash result.
    pub mashed: bool,
    /// accessory1_cb == ftCo_800BD39C: still being pulled in.
    pulling: bool,
    /// CaptureWaitKirby's IASA callback ran this tick; the scene runs its
    /// body on the pair (`hold_input`).
    input_pending: bool,
}

/// What Kirby's callback asks of the fighter in his mouth; the scene runs
/// it on the pair once the callback returns.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CaptorRequest {
    /// ftCo_800BD620: the fighter has reached the mouth.
    Close,
    /// ftCo_800DE2CC, ftCo_800BDB58, ftColl_8007B8CC: spat out as a star.
    Spit(StarParameters),
    /// ftCo_800DE2CC, ftCo_800BE000: swallowed, out as the copy star.
    Swallow(StarParameters),
}

/// What the star reads from Kirby's ftKb_DatAttrs (ftKb_SpecialN_800F58AC /
/// 800F58D8, 800F5A88 / 800F5AB0, 800F5820) and from ftData[Kirby].ext_attr,
/// the same block for every Kirby (800F5874, 800F5A70, 800F5A98, 800F5AC0,
/// 800F5AD8).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StarParameters {
    /// The star's velocity for a fighter facing right: mirrored by its
    /// facing at the release.
    pub velocity: Vec2,
    /// +8C (spat) or +98 (swallowed): taken off the star's speed each tick.
    pub deceleration: f32,
    /// +68 (spat) or +6C (swallowed): the star's timer.
    pub duration: f32,
    /// +64: taken from the timer each tick, in and out of the star.
    pub decay: f32,
    /// +60: taken per mash input.
    pub mash_decrement: f32,
    /// +B4..+C8: the star's environment box before the fighter's own
    /// scale (co_attrs.xDC).
    pub collision_box: FtCollisionBox,
    /// +9C, +A0: the velocity out of a spat star.
    pub exit_velocity: Vec2,
    /// +70: the frames over which the fighter grows back.
    pub growth_frames: f32,
    /// +44: the share of its size that grows back.
    pub growth: f32,
}

/// mv.co.thrownkirby (Fighter +2340) with the grab timer and mash state.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StarState {
    pub parameters: StarParameters,
    /// thrower_gobj (x0): the Kirby that let the star go (spawn number).
    pub thrower: u32,
    /// x8: the side a wall sent it back to (-1 or 1), 0 with none.
    pub wall_side: f32,
    /// x14: this tick's ftCommon_GrabMash result.
    pub mashed: bool,
    /// x18_b0: the copy star of a swallow, not a spat star.
    pub swallowed: bool,
    /// scale (+1C): the root joint's scale at the release.
    pub root_scale: Vec3,
    /// coll_box (+28).
    pub collision_box: FtCollisionBox,
    /// Fighter.grab_timer.
    pub timer: f32,
    stick_directions: [i8; 2],
    /// accessory4_cb == ftCo_800BE6AC: out of the star and growing back.
    growing: bool,
}

fn capture(f: &mut FighterCore) -> &mut CaptureKirbyState {
    match &mut f.state_data {
        MotionData::CaptureKirby(state) => state,
        _ => panic!("Inhale capture scratch missing"),
    }
}

fn star(f: &mut FighterCore) -> &mut StarState {
    match &mut f.state_data {
        MotionData::KirbyStar(state) => state,
        _ => panic!("Kirby star scratch missing"),
    }
}

/// ftKb_SpecialN_800F5B5C (800F5B5C): the mouth in the world.
fn mouth_position(captor: &FighterCore, mouth: &Mouth) -> Vec3 {
    let p = captor.physics.position;
    // retail 0x800F5B88: fmadds; 0x800F5B98: fadds.
    Vec3::new(
        gekko_math::fma::fmadds(mouth.offset.x, captor.physics.facing, p.x),
        p.y + mouth.offset.y,
        p.z,
    )
}

/// ftCo_800BD19C (800BD19C): the squared distance from `point` (separate
/// products and sums).
pub fn distance_squared(position: Vec3, point: Vec3) -> f32 {
    let (dx, dy, dz) = (
        position.x - point.x,
        position.y - point.y,
        position.z - point.z,
    );
    dx * dx + dy * dy + dz * dz
}

/// ftCo_800BD1DC (800BD1DC), the Inhale's grabbed_cb: the fighter turns to
/// face Kirby and starts toward his mouth from where it stands.
pub fn enter(
    victim: &mut Fighter,
    captor: &mut Fighter,
    victim_assets: &FighterAssets,
    mouth: Mouth,
) -> Result<()> {
    // ftCommon_8007DB58.
    victim.interrupt_actions(victim_assets);
    // ftCo_8009750C drops a heavy item; ftCo_800DD168 releases the victim's
    // own victim.
    if victim
        .core
        .held_item
        .as_ref()
        .is_some_and(|held| held.heavy)
    {
        unimplemented!("ftCo_8009750C (ftCo_CaptureKirby.c:47): inhaled with a heavy item");
    }
    if victim.core.combat.grab.is_some() {
        unimplemented!("ftCo_800DD168 (ftCo_CaptureKirby.c:48): inhaled while holding another");
    }
    victim.core.combat.grab = Some(GrabLink::Captured {
        captor: captor.core.spawn_number,
    });
    victim.core.physics.facing = -captor.core.physics.facing;
    victim.change_motion_state(S::CaptureKirby.into(), victim_assets)?;
    victim.core.leave_ground();
    // inlineA0: the offset from the mouth, the root's scale, x2226_b2.
    let from = mouth_position(&captor.core, &mouth);
    let position = victim.core.physics.position;
    let root = victim.core.animation.root;
    victim.core.state_data = MotionData::CaptureKirby(CaptureKirbyState {
        mouth,
        offset: Vec2::new(position.x - from.x, position.y - from.y),
        root_scale: victim.core.skeleton.scale(root),
        // ftCommon_InitGrab(fp, 0, +5C).
        timer: mouth.hold_time,
        stick_directions: [0; 2],
        mashed: false,
        pulling: true,
        input_pending: false,
    });
    victim.core.set_pinned(true);
    // ftCommon_8007E2F4(fp, 0x1FF).
    victim.core.status.grab_exclusions = GrabExclusions::ALL;
    victim.step_animation(victim_assets);
    // ftCommon_8007EBAC(fp, 1, 0) rumbles; ftCommon_8007E2FC.
    victim.core.clear_movement();
    Ok(())
}

/// Whether accessory1 is ftCo_800BD39C: the fighter is still on its way in.
pub fn pulling(victim: &FighterCore) -> bool {
    matches!(&victim.state_data, MotionData::CaptureKirby(state) if state.pulling)
}

/// ftCo_800DC920's FTKIND_KIRBY arm (ftCo_CaptureCut.c:115-122): a pair
/// parted while the Inhale still pulls its catch (a hit or a death in the
/// few ticks of SpecialNCapture0) returns the fighter to its size and sets
/// it down from where it is, not from its XRotN. No scene reaches it with
/// two fighters, so it is not ported.
pub(super) fn refuse_parting_in_the_pull(victim: &FighterCore) {
    if pulling(victim) {
        unimplemented!(
            "ftCo_800DC920 (ftCo_CaptureCut.c:115-122): a pair parted while the Inhale pulls its \
             catch in"
        );
    }
}

/// ftCo_800BD39C (800BD39C), CaptureKirby's accessory1: the fighter shrinks
/// with its distance from the mouth, then moves a step closer and is placed
/// from the mouth as it now stands. No fused operations.
pub fn pull(victim: &mut FighterCore, captor: &FighterCore) {
    let state = *capture(victim);
    let mouth = state.mouth;
    let mut step = Vec3::new(state.offset.x, state.offset.y, 0.0);
    // lbVector_Len: x*x + y*y, then z*z added to it; sqrtf__Ff.
    let distance = gekko_math::msl::sqrtf(step.z * step.z + (step.x * step.x + step.y * step.y));
    if distance < mouth.shrink_radius {
        let s = (1.0 - mouth.shrink) + distance / mouth.shrink_radius * mouth.shrink;
        let scale = Vec3::new(
            s * state.root_scale.x,
            s * state.root_scale.y,
            s * state.root_scale.z,
        );
        let root = victim.animation.root;
        victim.skeleton.set_scale(root, &scale);
    }
    step.x = clamp_step(step.x, mouth.pull_step.x);
    step.y = clamp_step(step.y, mouth.pull_step.y);
    let offset = Vec2::new(state.offset.x - step.x, state.offset.y - step.y);
    capture(victim).offset = offset;
    let from = mouth_position(captor, &mouth);
    victim.physics.position = Vec3::new(from.x + offset.x, from.y + offset.y, from.z);
    // Fighter_CallAcessoryCallbacks_8006C624 (fighter.c:2547): the root
    // follows cur_pos once accessory1 returns.
    let (root, position) = (victim.animation.root, victim.physics.position);
    victim.skeleton.set_translate(root, &position);
}

/// inlineB1: a component past `limit` either way becomes the limit with
/// its sign (the sign as a float, then fmuls).
fn clamp_step(value: f32, limit: f32) -> f32 {
    let magnitude = if value < 0.0 { -value } else { value };
    if magnitude > limit {
        limit * if value < 0.0 { -1.0 } else { 1.0 }
    } else {
        value
    }
}

/// ftCo_800BD620 (800BD620): the fighter is in the mouth. It is pinned to
/// Kirby's TransN2 (ftCo_800DB368, placed by accessory1 ftCo_800DB464),
/// hidden and intangible at its own size, with no animation.
pub fn close(
    victim: &mut Fighter,
    captor: &mut Fighter,
    victim_assets: &FighterAssets,
    captor_assets: &FighterAssets,
) -> Result<()> {
    let scratch = CaptureKirbyState {
        pulling: false,
        ..*capture(&mut victim.core)
    };
    victim.core.set_pinned(false);
    grab_throw::constrain_to_captor(
        &mut victim.core,
        &mut captor.core,
        victim_assets,
        captor_assets,
    );
    // Ft_MF_SkipModel | Ft_MF_SkipMatAnim.
    victim.change_motion_state_with_flags(
        S::CaptureWaitKirby.into(),
        victim_assets,
        super::MotionEntryFlags(
            super::MotionEntryFlags::SKIP_MODEL.0 | super::MotionEntryFlags::SKIP_MAT_ANIM.0,
        ),
        0.0,
        1.0,
    )?;
    victim.core.state_data = MotionData::CaptureKirby(scratch);
    victim.core.update_model_scale();
    victim.core.effect_state.invisible = true;
    victim.core.set_body_hurt_status(HurtStatus::Intangible);
    victim.core.leave_ground();
    victim.core.status.grab_exclusions = GrabExclusions::ALL;
    victim.step_animation(victim_assets);
    victim.core.clear_movement();
    Ok(())
}

/// CaptureKirby's and CaptureWaitKirby's Anim callbacks are empty: the
/// animation still advances.
pub fn animation(victim: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    victim.step_animation(phase.assets);
    Ok(None)
}

/// ftCo_CaptureWaitKirby_IASA (800BD7C0) reaches into Kirby: the row's
/// callback only marks that it ran (CaptureKirby's is empty).
pub fn input(victim: &mut Fighter, _phase: super::state::InputPhase<'_>) {
    if let MotionData::CaptureKirby(state) = &mut victim.core.state_data {
        state.input_pending = !state.pulling;
    }
}

/// Whether CaptureWaitKirby's IASA ran this tick (taken).
pub fn take_input(victim: &mut FighterCore) -> bool {
    match &mut victim.state_data {
        MotionData::CaptureKirby(state) => std::mem::take(&mut state.input_pending),
        _ => false,
    }
}

/// ftData's floor flag for a platform a fighter can drop through
/// (ftKb_SpecialN_800F597C: `coll_data.floor.flags & 0x100`).
const PASSABLE_FLOOR: u32 = 0x100;

/// ftCo_800BD6EC (800BD6EC): a tap up hops Kirby off the floor with the
/// fighter inside (ftKb_SpecialN_800F598C); a tap down on a floor he can
/// pass drops him through it. The tap is spent (its tilt timer
/// becomes 254).
fn steer_vertically(
    victim: &mut FighterCore,
    captor: &mut Fighter,
    hold: &super::MouthHold,
    victim_assets: &FighterAssets,
    captor_assets: &FighterAssets,
) -> Result<bool> {
    let t = &victim_assets.input.thresholds;
    let stick_y = victim.input.current.stick.y;
    let timer = victim.input.vertical.tilt;
    // retail 0x800BD704..0x800BD724: fcmpo/cror (>=), then cmpw on the byte.
    if stick_y >= t.tap_jump_threshold && i32::from(timer) < t.tap_jump_window {
        victim.input.vertical.tilt = STICK_TIMER_SPENT;
        let speed = capture(victim).mouth.steer_speed;
        if captor.physics.ground_or_air == GroundOrAir::Ground {
            (hold.leave_floor)(captor, captor_assets)?;
            captor.physics.self_velocity.y = speed * 1.0;
            return Ok(true);
        }
        return Ok(false);
    }
    // 0x800BD744..0x800BD778: the stick against PlCo +464 as it stands (no
    // negation), the timer as a float against +468, then the floor's flag.
    let m = &victim_assets.movement;
    if stick_y <= m.platform_drop_threshold
        && f32::from(timer) < m.platform_drop_window
        && captor.collision.data.floor.flags & PASSABLE_FLOOR != 0
    {
        // ftCo_CaptureWaitKirby.c:56-62: inlineA0(-1), then
        // ftKb_SpecialN_800F5954's mpUpdateFloorSkip on Kirby's floor, which
        // runs whether or not ftKb_SpecialN_800F598C found him grounded.
        victim.input.vertical.tilt = STICK_TIMER_SPENT;
        let speed = capture(victim).mouth.steer_speed;
        let left_floor = captor.physics.ground_or_air == GroundOrAir::Ground;
        if left_floor {
            (hold.leave_floor)(captor, captor_assets)?;
            // `specialn_stop_momentum * (f32)-1`: a sign flip, bit for bit.
            captor.physics.self_velocity.y = -speed;
        }
        melee_mp::update_floor_skip(&mut captor.collision.data);
        return Ok(left_floor);
    }
    Ok(false)
}

/// ftCo_CaptureWaitKirby_IASA's inlineB0: a tap sideways pushes Kirby that
/// way (ftKb_SpecialN_800F5A04), along the floor or through the air.
fn steer_sideways(victim: &mut FighterCore, captor: &mut FighterCore, assets: &FighterAssets) {
    let t = &assets.input.thresholds;
    let stick_x = victim.input.current.stick.x;
    let magnitude = if stick_x < 0.0 { -stick_x } else { stick_x };
    // retail 0x800BD834..0x800BD870: the window as a float plus PlCo +44.
    let window = t.dash_smash_window as f32 + assets.input.item_smash_window_extension;
    if magnitude >= t.dash_smash_stick_threshold && f32::from(victim.input.horizontal.tilt) < window
    {
        victim.input.horizontal.tilt = STICK_TIMER_SPENT;
        let direction = if stick_x < 0.0 { -1.0 } else { 1.0 };
        let speed = direction * capture(victim).mouth.steer_speed;
        if captor.physics.ground_or_air == GroundOrAir::Ground {
            captor.physics.ground_velocity = speed;
        } else {
            captor.physics.self_velocity.x = speed;
        }
    }
}

/// Which of the pair a tick of the hold moved to a new motion.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HoldChanges {
    pub victim: bool,
    pub captor: bool,
}

/// ftCo_CaptureWaitKirby_IASA (800BD7C0), run on the pair: while Kirby
/// stands or falls with his mouth full (ftKb_SpecialN_800F5A38) the held
/// fighter's stick moves him; then the hold timer's decay and the mash, and
/// at its end the release.
pub fn hold_input(
    victim: &mut Fighter,
    captor: &mut Fighter,
    victim_assets: &FighterAssets,
    captor_assets: &FighterAssets,
    map: &mut melee_mp::CollMap,
) -> Result<HoldChanges> {
    let mut changes = HoldChanges::default();
    let hold = captor
        .character
        .table()
        .mouth_hold
        .expect("the captor's mouth hooks");
    if (hold.steerable)(captor) {
        changes.captor = steer_vertically(
            &mut victim.core,
            captor,
            &hold,
            victim_assets,
            captor_assets,
        )?;
        steer_sideways(&mut victim.core, &mut captor.core, victim_assets);
    }
    let input = victim.core.input.clone();
    let state = capture(&mut victim.core);
    state.timer -= state.mouth.hold_decay;
    state.mashed = grab_mash(
        &mut state.timer,
        &mut state.stick_directions,
        &input,
        victim_assets.grab_escape.stick_threshold,
        state.mouth.mash_decrement,
    );
    if state.timer <= 0.0 {
        escape(victim, captor, victim_assets, captor_assets, map)?;
        changes = HoldChanges {
            victim: true,
            captor: true,
        };
    }
    Ok(changes)
}

/// ftCo_800BD9E0 (800BD9E0): the ability a Kirby gets by swallowing
/// `victim`.
pub fn ability(victim: &Fighter) -> melee_types::FighterKind {
    let table = victim.character.table();
    match table.swallowed_ability {
        Some(ability) => ability(&victim.character),
        None => (table.kind)(&victim.character),
    }
}

/// CaptureKirby and CaptureWaitKirby: rows whose Phys and Coll callbacks
/// are empty, placed by their accessory1.
pub fn captured(victim: &FighterCore) -> bool {
    matches!(victim.state_data, MotionData::CaptureKirby(_))
}

/// ftCo_CaptureWaitKirby_IASA's release once the timer is spent: the
/// fighter faces away from Kirby at its own size and is set down
/// (ftCo_CaptureCut_Enter -> ftCo_800DC920), then Kirby lets go
/// (ftCo_800DA698(captor, false)).
fn escape(
    victim: &mut Fighter,
    captor: &mut Fighter,
    victim_assets: &FighterAssets,
    captor_assets: &FighterAssets,
    map: &mut melee_mp::CollMap,
) -> Result<()> {
    // ftKb_SpecialN_800F5A60.
    victim.physics.facing = -captor.physics.facing;
    let scale = capture(&mut victim.core).root_scale;
    let root = victim.animation.root;
    victim.skeleton.set_scale(root, &scale);
    // ftCo_800DC920(captor, victim).
    captor.combat.grab = None;
    super::grab_damage::release_thrown(captor, victim, victim_assets, map);
    victim.combat.grab = None;
    let position = victim.physics.position;
    victim.skeleton.set_translate(root, &position);
    // The fighter left the floor at the catch; only ftCo_800DC920's sweep
    // (no floor under it joined to Kirby's) can land it again.
    if victim.physics.ground_or_air == GroundOrAir::Ground {
        unimplemented!(
            "ftCo_CaptureCut_Enter (ftCo_CaptureCut.c:29-30): a fighter out of Kirby's mouth \
             landed by its release"
        );
    }
    victim.physics.self_velocity.x =
        -victim.physics.facing * victim_assets.grab_escape.escape_speed;
    victim.change_motion_state(S::CaptureCut.into(), victim_assets)?;
    let p = &captor_assets.grab_escape;
    if captor.physics.ground_or_air == GroundOrAir::Air {
        captor.physics.self_velocity.x = -captor.physics.facing * p.air_escape_speed;
        captor.physics.self_velocity.y = p.air_escape_vertical_speed;
    } else {
        captor.physics.ground_velocity = -captor.physics.facing * p.escape_speed;
    }
    captor.change_motion_state(S::CatchCut.into(), captor_assets)
}

/// ftKb_SpecialNSpit_Anim's inline (800F7808): Kirby lets the fighter go at
/// his TransN2 (ftCo_800DE2CC) as a star that hits for the fighter's own
/// co_attrs +E0 (ftCo_800BDB58) and answers to him (ftColl_8007B8CC).
pub fn spit(
    victim: &mut Fighter,
    captor: &mut Fighter,
    victim_assets: &FighterAssets,
    captor_assets: &FighterAssets,
    map: &mut melee_mp::CollMap,
    parameters: StarParameters,
) -> Result<()> {
    release(victim, captor, victim_assets, captor_assets, map);
    enter_star(
        victim,
        captor,
        victim_assets,
        captor_assets,
        parameters,
        false,
    )?;
    // ftColl_8007ABD0(fp->x914, co_attrs.kirby_b_star_damage, gobj).
    let damage = victim.core.attributes.combat.kirby_b_star_damage;
    set_star_damage(&mut victim.core, gekko_math::msl::fctiwz(damage) as u32);
    // ftColl_8007B8CC: x1064_thrownHitbox.owner, the team and the player.
    victim.core.commands.thrown_by = Some(captor.core.spawn_number);
    Ok(())
}

/// ftKb_SpecialNDrink_Anim (800F7B20): the swallowed fighter leaves at
/// Kirby's TransN2 as the copy star, which hits nothing (ftCo_800BE000).
pub fn swallow(
    victim: &mut Fighter,
    captor: &mut Fighter,
    victim_assets: &FighterAssets,
    captor_assets: &FighterAssets,
    map: &mut melee_mp::CollMap,
    parameters: StarParameters,
) -> Result<()> {
    release(victim, captor, victim_assets, captor_assets, map);
    enter_star(
        victim,
        captor,
        victim_assets,
        captor_assets,
        parameters,
        true,
    )?;
    // ftColl_8007AFF8: every hitbox off.
    for hit in victim.core.commands.hitboxes.iter_mut() {
        *hit = None;
    }
    Ok(())
}

/// ftCo_800DE2CC (ftCo_800DDDE4 without the release offset): the links go,
/// the fighter is set free at Kirby's TransN2 and the script's throw record
/// is logged on it. The star's motion entry clears the knockback, so the
/// record lands in the fighter's ProcessHit as damage alone.
fn release(
    victim: &mut Fighter,
    captor: &mut Fighter,
    victim_assets: &FighterAssets,
    captor_assets: &FighterAssets,
    map: &mut melee_mp::CollMap,
) {
    // ftCo_800DDDE4: a hidden body is normal again before the damage test.
    if victim.commands.hurt_status != HurtStatus::Normal {
        victim.core.set_body_hurt_status(HurtStatus::Normal);
    }
    if victim.status.ledge_intangibility != 0 || victim.status.revival_invincibility != 0 {
        unimplemented!(
            "ftCo_800DDDE4 (ftCo_Throw.c:534-538): an invincible fighter leaving the mouth"
        );
    }
    let hit = grab_throw::release_captured(
        &mut victim.core,
        &mut captor.core,
        victim_assets,
        captor_assets,
        map,
        false,
    );
    // ftColl_80076640: x1838_percentTemp and x183C_applied.
    victim.combat.frame_damage += hit.percent_damage;
    victim.combat.frame_max_damage = victim
        .combat
        .frame_max_damage
        .max(super::hit_log::damage_count(hit.percent_damage));
}

/// ftCo_800BDB58 / ftCo_800BE000's inlineB2: the fighter faces away from
/// Kirby, takes the star's velocity and plays Kirby's star row, hidden and
/// intangible, with the star's timer.
fn enter_star(
    victim: &mut Fighter,
    captor: &Fighter,
    victim_assets: &FighterAssets,
    captor_assets: &FighterAssets,
    parameters: StarParameters,
    swallowed: bool,
) -> Result<()> {
    victim.core.physics.facing = -captor.core.physics.facing;
    let facing = victim.core.physics.facing;
    // ftKb_SpecialN_800F58AC / 800F58D8 already fixed the direction for a
    // victim facing right.
    victim.core.physics.self_velocity = Vec3::new(
        if swallowed {
            facing * parameters.velocity.x
        } else {
            -facing * parameters.velocity.x
        },
        parameters.velocity.y,
        0.0,
    );
    victim.core.update_model_scale();
    let root = victim.core.animation.root;
    let root_scale = victim.core.skeleton.scale(root);
    let (row, motion) = if swallowed {
        (S::ThrownCopyStar, COPY_STAR_MOTION)
    } else {
        (S::ThrownKirbyStar, SPIT_STAR_MOTION)
    };
    // Ft_MF_SkipThrowException with Kirby as the animation's source.
    let source = grab_throw::throw_source(captor_assets, victim_assets, motion);
    victim.change_motion_state_with_source(row.into(), victim_assets, 0.0, 1.0, Some(source))?;
    victim.core.status.grab_exclusions = GrabExclusions::ALL;
    victim.core.effect_state.invisible = true;
    victim.core.set_body_hurt_status(HurtStatus::Intangible);
    // ftCommon_8007EFC0(fp, 1).
    victim.core.status.name_tag_timer = 1;
    // ftKb_SpecialN_800F5820: Kirby's box scaled by the fighter's co_attrs
    // +DC (the star accessory's scale; the accessory itself only draws).
    let scale = victim.core.attributes.kirby_throw.star_scale;
    let b = parameters.collision_box;
    victim.core.state_data = MotionData::KirbyStar(StarState {
        parameters,
        thrower: captor.core.spawn_number,
        wall_side: 0.0,
        mashed: false,
        swallowed,
        root_scale,
        collision_box: FtCollisionBox {
            top: b.top * scale,
            bottom: b.bottom * scale,
            left: Vec2::new(b.left.x * scale, b.left.y * scale),
            right: Vec2::new(b.right.x * scale, b.right.y * scale),
        },
        // ftCommon_InitGrab(fp, 0, +68 / +6C).
        timer: parameters.duration,
        stick_directions: [0; 2],
        growing: false,
    });
    victim.step_animation(victim_assets);
    victim.core.leave_ground();
    Ok(())
}

/// ftColl_8007ABD0 (8007ABD0): hitbox 0's damage becomes `damage`, staled
/// by the fighter's own queue.
fn set_star_damage(f: &mut FighterCore, damage: u32) {
    if f.player.scale != 1.0 {
        unimplemented!("ftColl_8007ABD0: ftCo_CalcYScaledKnockback for a scaled fighter");
    }
    let damage = damage as f32;
    let staled = f.commands.stale_damage(damage);
    if let Some(hit) = f.commands.hitboxes[0].as_mut() {
        hit.knockback_damage = gekko_math::msl::fctiwz(damage) as u32;
        hit.descriptor.damage = staled;
    }
}

/// The star rows' and ThrownKirby's Anim callbacks are empty.
pub fn star_animation(
    victim: &mut Fighter,
    phase: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    victim.step_animation(phase.assets);
    Ok(None)
}

/// ftCo_ThrownKirbyStar_Phys (800BDDCC) / ftCo_ThrownCopyStar_Phys: the
/// star slows by its deceleration along its path (and stops across once it
/// is slower than that), the timer runs down, faster for mashing, and at its
/// end the fighter comes out.
pub fn star_physics(victim: &mut Fighter, phase: PhysicsPhase<'_>) {
    let PhysicsPhase { assets, wind, .. } = phase;
    let state = *star(&mut victim.core);
    let p = state.parameters;
    let v = victim.core.physics.self_velocity;
    // retail 0x800BDDF4..0x800BDE50: fmuls, fmuls, fadds, the inline sqrtf.
    let speed = gekko_math::msl::sqrtf(v.x * v.x + v.y * v.y);
    if speed > p.deceleration {
        // 0x800BDE60..0x800BDE88: fsubs, fmuls, fdivs for each axis.
        victim.core.physics.self_velocity.x = v.x * (speed - p.deceleration) / speed;
        victim.core.physics.self_velocity.y = v.y * (speed - p.deceleration) / speed;
        // 0x800BDE8C: the facing follows the vertical speed's sign.
        victim.core.physics.facing = if victim.core.physics.self_velocity.y < 0.0 {
            -1.0
        } else {
            1.0
        };
    } else {
        // `self_vel.x = 0`. Neither star gets this slow before its timer
        // ends: 4.0 less 0.13 a tick for 30 ticks, 2.8 less 0.17 for 10
        // (PlKb.dat +88..+98, +68, +6C), and a mash only shortens it.
        unimplemented!(
            "ftCo_ThrownKirbyStar_Phys (ftCo_ThrownKirby.c:177-179): a star slower than its \
             deceleration"
        );
    }
    let input = victim.core.input.clone();
    let state = star(&mut victim.core);
    state.timer -= p.decay;
    state.mashed = grab_mash(
        &mut state.timer,
        &mut state.stick_directions,
        &input,
        assets.grab_escape.stick_threshold,
        p.mash_decrement,
    );
    if state.timer <= 0.0 {
        leave_star(victim, assets).expect("ThrownKirby assets");
    }
    victim.core.finish_air_update(assets, wind);
}

/// ftCo_800BDA74 (800BDA74): what the star's box met this tick, the floor
/// first, then a wall on either side, then the ceiling.
fn contact_normal(victim: &mut Fighter, phase: &mut CollisionPhase<'_>) -> Vec3 {
    use melee_types::mp::collide;
    let collision_box = star(&mut victim.core).collision_box;
    let c = &mut victim.core;
    crate::collision::air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    let landed = crate::collision::air::collide_box(
        &mut c.physics,
        &mut c.collision,
        phase.map,
        &mut c.skeleton,
        c.animation.root,
        collision_box,
    );
    let cd = &c.collision.data;
    let flags = cd.env_flags as u32;
    if landed {
        // A level star meets a floor that rises ahead of it (Yoshi's
        // Story's slopes); it bounces off the floor and stays airborne.
        cd.floor.normal
    } else if flags & collide::RIGHT_WALL_MASK != 0 {
        cd.right_facing_wall.normal
    } else if flags & collide::LEFT_WALL_MASK != 0 {
        cd.left_facing_wall.normal
    } else if flags & collide::CEILING_MASK != 0 {
        unimplemented!("ftCo_800BDA74 (ftCo_ThrownKirby.c:82-83): a star under a ceiling");
    } else {
        Vec3::ZERO
    }
}

/// ftCo_ThrownKirbyStar_Coll (800BDF10) / ftCo_ThrownCopyStar_Coll: the
/// star bounces off what it meets; turned back on itself, the fighter comes
/// out.
pub fn star_collision(victim: &mut Fighter, mut phase: CollisionPhase<'_>) -> Result<()> {
    let normal = contact_normal(victim, &mut phase);
    if normal.x == 0.0 && normal.y == 0.0 && normal.z == 0.0 {
        return Ok(());
    }
    let before = victim.core.physics.self_velocity;
    let after = melee_lb::vector::mirror(before, normal);
    victim.core.physics.self_velocity = after;
    // retail 0x800BDF8C / 0x800BDF98: fmuls, then fmadds.
    if gekko_math::fma::fmadds(before.x, after.x, before.y * after.y) < 0.0 {
        leave_star(victim, phase.assets.expect("ThrownKirby assets"))?;
        star(&mut victim.core).wall_side = if normal.x < 0.0 { -1.0 } else { 1.0 };
    }
    Ok(())
}

/// ftCo_800BE494 (800BE494): out of the star, visible and vulnerable. A
/// spat fighter pops up and away; either grows back over the timer
/// (accessory4 ftCo_800BE6AC).
fn leave_star(victim: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    let state = *star(&mut victim.core);
    let p = state.parameters;
    // Ft_MF_Unk06: the entry does not start the KO credit's countdown.
    victim.change_motion_state_with_flags(
        S::ThrownKirby.into(),
        assets,
        super::MotionEntryFlags::UNK06,
        0.0,
        1.0,
    )?;
    victim.core.status.grab_exclusions = GrabExclusions::ALL;
    victim.core.effect_state.invisible = false;
    victim.core.set_body_hurt_status(HurtStatus::Normal);
    if !state.swallowed {
        victim.core.physics.self_velocity.y = p.exit_velocity.y;
        // retail 0x800BE544..0x800BE570: the wall's side times the speed,
        // or the speed with the flight's sign (the sign as a float).
        victim.core.physics.self_velocity.x = if state.wall_side != 0.0 {
            // mv +8 is written only after this function returns
            // (ftCo_ThrownKirbyStar_Coll), and nothing calls it again.
            unimplemented!("ftCo_800BE494 (ftCo_ThrownKirby.c:278-279): a star's exit with x8 set");
        } else {
            p.exit_velocity.x
                * if victim.core.physics.self_velocity.x < 0.0 {
                    -1.0
                } else {
                    1.0
                }
        };
    }
    // The FTKIND_KIRBY arm (ftCo_ThrownKirby.c:283-289), a swallowed Kirby
    // losing his own hat, is refused at the swallow
    // (`CharacterCallbacks::swallowed_ability`).
    victim.core.state_data = MotionData::KirbyStar(StarState {
        // ftCommon_InitGrab(fp, 0, +70).
        timer: p.growth_frames,
        stick_directions: [0; 2],
        growing: true,
        ..state
    });
    victim.step_animation(assets);
    victim.core.leave_ground();
    Ok(())
}

/// ftCo_ThrownKirby_Phys (800BE5F4): a spat fighter falls once the timer is
/// spent, a swallowed one at once, at its own size
/// (Fighter_UpdateModelScale, ftCo_Fall_Enter).
pub fn thrown_physics(victim: &mut Fighter, phase: PhysicsPhase<'_>) {
    let PhysicsPhase { assets, wind, .. } = phase;
    let state = star(&mut victim.core);
    state.timer -= state.parameters.decay;
    if state.swallowed || state.timer <= 0.0 {
        victim.core.update_model_scale();
        victim
            .change_motion_state(S::Fall.into(), assets)
            .expect("Fall assets");
    }
    victim.core.finish_air_update(assets, wind);
}

/// ThrownKirby's Coll callback is empty; Fighter_procMap's head still runs.
pub fn thrown_collision(victim: &mut Fighter, _phase: CollisionPhase<'_>) -> Result<()> {
    let c = &mut victim.core;
    crate::collision::air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    Ok(())
}

impl FighterCore {
    /// ftCo_800BE6AC (800BE6AC), ThrownKirby's accessory4: the root grows
    /// back with the timer. Returns whether the accessory4 slot is the
    /// star's. Separate fsubs, fdivs, fmuls, fsubs and fadds.
    pub(super) fn star_accessory(&mut self) -> bool {
        let MotionData::KirbyStar(state) = &self.state_data else {
            return false;
        };
        if !state.growing {
            return false;
        }
        let p = state.parameters;
        let grown = (p.growth_frames - state.timer) / p.growth_frames * p.growth;
        let s = (1.0 - p.growth) + grown;
        let scale = Vec3::new(
            s * state.root_scale.x,
            s * state.root_scale.y,
            s * state.root_scale.z,
        );
        let root = self.animation.root;
        self.skeleton.set_scale(root, &scale);
        true
    }

    /// take_dmg_cb / take_dmg_2_cb = ftCo_800BE7C0 (800BE7C0) in the star
    /// and out of it: a hit returns the fighter to its size.
    pub(super) fn star_hit_taken(&mut self) {
        if matches!(self.state_data, MotionData::KirbyStar(_)) {
            // Fighter_UpdateModelScale. The star is intangible; the four
            // ticks out of a spat one are the only opening, and with two
            // fighters the other is still spitting.
            unimplemented!(
                "ftCo_800BE7C0 (ftCo_ThrownKirby.c:340-343): a hit on a fighter out of a star"
            );
        }
    }
}
