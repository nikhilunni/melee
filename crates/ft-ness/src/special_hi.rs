//! PK Thunder, ftnessspecialhi.c (80117B70..80119DD8).
//!
//! The start (358 / 362) ends by sending the thunder up from Ness's hand
//! and entering the control row (359 / 363), where the thunder reads his
//! stick. Once it has left him and comes back within reach he launches
//! away from it (PK Thunder 2: 361 on the floor, 365 in the air), along the
//! floor when the contact drives him into it at a shallow angle and into a
//! knockdown when it is steep. The aerial launch slides along shallow
//! surfaces, rebounds off steep ones (366) and ends in the special fall.
use crate::{
    attributes::{PkThunder2Attributes, PkThunderAttributes},
    common::{self, row},
    init::Ness,
};
use gekko_math::{
    fma::{fmadds, fmsubs, fnmsubs},
    msl::{cosf, sinf},
};
use hsd_types::Vec3;
use melee_ef::request::EffectRequest;
use melee_ft::{
    anim::WaitChoice,
    collision::air,
    fighter::{
        assets::{FighterAssets, Result},
        part_rotation::Axis,
        state::{AnimationPhase, CollisionPhase, MotionRow, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
    physics::friction,
};
use melee_it::{ItemControl, ItemRequest, SpawnItem};
use melee_lb::trigf::atan2f;
use melee_types::{mp::collide, FtPart, ItemKind};

/// ftNs_MS_SpecialHiStart..SpecialAirHiRebound (358..366).
pub const START: ActionId = ActionId(358);
pub const HOLD: ActionId = ActionId(359);
pub const END: ActionId = ActionId(360);
pub const LAUNCH: ActionId = ActionId(361);
pub const AIR_START: ActionId = ActionId(362);
pub const AIR_HOLD: ActionId = ActionId(363);
pub const AIR_END: ActionId = ActionId(364);
pub const AIR_LAUNCH: ActionId = ActionId(365);
pub const REBOUND: ActionId = ActionId(366);

/// efSync_Spawn(1262 / 1263, gobj, parts[FtPart_HipN]): the control row's
/// glow and the launch's (models 0x2710 and 0x2711).
const CONTROL_EFFECT: u16 = 0x4EE;
const LAUNCH_EFFECT: u16 = 0x4EF;
/// FTNESS_JIBAKU_COLL_FLAG: the ground/air flags with KeepGfx and SkipHit.
const LAUNCH_COLL: MotionEntryFlags =
    MotionEntryFlags(common::GROUND_AIR.0 | common::KEEP_GFX | common::SKIP_HIT);
const KEEP_GFX: MotionEntryFlags = MotionEntryFlags(common::KEEP_GFX);
/// ftNs_SpecialHi_ItemPKThunder_CheckNessCollide: the thunder touches Ness
/// within this box around a point five units (times his Y scale) up.
const REACH_X: f32 = 8.333_333;
const REACH_Y: f32 = 12.333_333;
const CENTER_RISE: f32 = 5.0;
/// `fp->parts[0]`: the model root, turned about X along the launch.
const ROOT_PART: usize = 0;
/// (float) M_PI_2 and 0.017453292f (MTXDegToRad).
const HALF_PI: f32 = std::f32::consts::FRAC_PI_2;
const DEGREES: f32 = 0.017453292;
const TAU: f64 = std::f64::consts::TAU;
const PI: f64 = std::f64::consts::PI;
/// ftNs_SpecialHi_Phys / ftNs_SpecialAirHi_Phys: a deceleration that would
/// leave this little speed is not applied.
const SPEED_EPSILON: f32 = 0.0001;
/// A rebound keeps half the mirrored velocity.
const REBOUND_SCALE: f32 = 0.5;

/// mv.ns.specialhi.thunderColl.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Reach {
    /// 0: the thunder has left Ness and may strike him.
    #[default]
    Apart,
    /// 1: it has not left him yet.
    Leaving,
    /// 2: it struck him.
    Struck,
}

/// mv.ns.specialhi and Ness's hold on the thunder (u.ns.pkthunder_gobj,
/// u.ns.pkthunder_gfx).
#[derive(Clone, Copy, Debug, Default)]
pub struct PkThunder {
    /// +2340 thunderColl.
    pub reach: Reach,
    /// +2344 thunderTimerLoop1: counts down from the control row's start.
    pub loop_frames: i32,
    /// +2348 thunderTimerLoop2: counts down once the thunder is gone.
    pub gone_frames: i32,
    /// +234C gravityDelay: aerial frames before gravity.
    pub gravity_delay: i32,
    /// +2350 collPos1: where the thunder struck.
    pub contact: Vec3,
    /// +2368 aerialVel: the launch's direction, radians.
    pub angle: f32,
    /// +2370 facingDir: the launch's vertical sign.
    pub vertical_sign: f32,
    /// +2374 unkVector1: the velocity the launch's physics left, which its
    /// collision restores.
    pub kept_velocity: Vec3,
    /// +2380 jibakuGFX: animation callbacks of the launch so far.
    pub launch_frames: i32,
    /// +2384 fallAccel: the launch's sink, once the script starts it.
    pub sink: f32,
    /// u.ns.pkthunder_gobj: his thunder is out and still his.
    pub thunder_out: bool,
}

pub const fn rows() -> [MotionRow; 9] {
    [
        row(
            START,
            0x135,
            start_anim,
            common::no_input,
            start_ground_physics,
            ground_collision,
        ),
        row(
            HOLD,
            0x136,
            hold_anim,
            common::no_input,
            common::ground_friction,
            ground_collision,
        ),
        row(
            END,
            0x137,
            end_ground_anim,
            common::no_input,
            common::ground_friction,
            ground_collision,
        ),
        row(
            LAUNCH,
            0x138,
            launch_ground_anim,
            common::no_input,
            launch_ground_physics,
            launch_ground_collision,
        ),
        row(
            AIR_START,
            0x139,
            start_anim,
            common::no_input,
            air_physics,
            air_collision,
        ),
        row(
            AIR_HOLD,
            0x13A,
            hold_anim,
            common::no_input,
            air_physics,
            air_collision,
        ),
        row(
            AIR_END,
            0x13B,
            end_air_anim,
            common::no_input,
            air_physics,
            air_collision,
        ),
        row(
            AIR_LAUNCH,
            0x13C,
            launch_air_anim,
            common::no_input,
            launch_air_physics,
            launch_air_collision,
        ),
        row(
            REBOUND,
            0x13D,
            rebound_anim,
            common::no_input,
            rebound_physics,
            rebound_collision,
        ),
    ]
}

fn control(f: &Fighter) -> &PkThunderAttributes {
    &f.character.get::<Ness>().attributes.pk_thunder
}
fn launch(f: &Fighter) -> &PkThunder2Attributes {
    &f.character.get::<Ness>().attributes.pk_thunder_2
}
fn scratch(f: &mut Fighter) -> &mut PkThunder {
    &mut f.character.get_mut::<Ness>().pk_thunder
}

/// Whether the current row is one of the aerial ones.
fn airborne_row(f: &Fighter) -> bool {
    f.motion_state.action.0 >= AIR_START.0
}

/// The counters from the attributes, the sink and kept velocity cleared,
/// no callbacks, the model root level (the entries' and
/// ftNs_SpecialHi_Coll's reset).
fn reset(f: &mut Fighter) {
    let a = control(f).clone();
    let s = scratch(f);
    s.loop_frames = a.loop_frames;
    s.gone_frames = a.loop_frames_2;
    s.gravity_delay = a.gravity_delay;
    s.sink = 0.0;
    s.kept_velocity = Vec3::ZERO;
    f.character.get_mut::<Ness>().damage_callbacks = false;
    f.core.set_part_rotation(ROOT_PART, Axis::X, 0.0);
}

/// ftNs_SpecialHiStart_Enter (80118120) / ftNs_SpecialAirHiStart_Enter
/// (80118250): the aerial entry stops the fall.
pub fn enter(f: &mut Fighter, air_entry: bool, a: &FighterAssets) {
    f.change_motion_state(if air_entry { AIR_START } else { START }, a)
        .expect("PK Thunder assets");
    f.commands.variables = [0; 4];
    reset(f);
    let facing = f.physics.facing;
    let s = scratch(f);
    s.reach = Reach::Leaving;
    s.launch_frames = 0;
    s.contact = Vec3::ZERO;
    s.angle = if facing == 1.0 { 0.0 } else { PI as f32 };
    s.vertical_sign = 1.0;
    if air_entry {
        f.physics.self_velocity.y = 0.0;
    }
    f.step_animation(a);
}

/// ftNs_SpecialHiStopGFX (80117B70): efLib_DestroyAll.
fn stop_effects(f: &mut Fighter) {
    f.effects.push(EffectRequest::DestroyOwned);
}

/// The model root turned along the velocity: facing * atan2f(x, y) - pi/2
/// (retail 0x801184D4 and its copies: fmsubs).
fn lean_along_velocity(f: &mut Fighter) {
    let v = f.physics.self_velocity;
    let angle = fmsubs(f.physics.facing, atan2f(v.x, v.y), HALF_PI);
    f.core.set_part_rotation(ROOT_PART, Axis::X, angle);
}

/// Ness's centre for the thunder's contact: five units times his Y scale
/// above his position (retail 0x80117C34: fmadds).
fn center(f: &Fighter) -> Vec3 {
    let scale = f.player.scale * f.attributes.size.model_scaling;
    let mut position = f.physics.position;
    position.y = fmadds(CENTER_RISE, scale, position.y);
    position
}

/// ftNs_SpecialHiStart_Anim (801186B0) / ftNs_SpecialAirHiStart_Anim
/// (80118A10): at the end the control row, the thunder from
/// fp->parts[FtPart_L2ndNa] on the stage plane (it_802AB58C), every jump
/// spent, and the control glow in place of whatever Ness owned.
fn start_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if f.animation.frames_remaining(&f.skeleton) {
        return Ok(None);
    }
    let state = if airborne_row(f) { AIR_HOLD } else { HOLD };
    f.change_motion_state(state, p.assets)?;
    if !scratch(f).thunder_out {
        let c = &mut f.core;
        let mut hand = melee_ft::fighter::caches::part_position(
            &mut c.skeleton,
            &c.animation,
            common::part(FtPart::L2ndNa),
            Vec3::ZERO,
        );
        hand.z = 0.0;
        let mut spawn =
            SpawnItem::ray(ItemKind::NessPKThunder, c.player.id, hand, c.physics.facing);
        // it_8026BB68 -> ftLib_80086990: the ECB centre (fadds, fmuls, fadds).
        let midpoint = 0.5 * (c.collision.data.ecb.top.y + c.collision.data.ecb.bottom.y);
        spawn.position = Vec3::new(
            c.physics.position.x + 0.0,
            c.physics.position.y + midpoint,
            c.physics.position.z + 0.0,
        );
        c.item_requests.push(ItemRequest::Spawn(spawn));
        scratch(f).thunder_out = true;
        common::install_damage_callbacks(f);
    }
    f.physics.jumps_used = f.attributes.jumping.max_jumps as u8;
    f.core
        .effects_after_items
        .push(EffectRequest::DestroyOwned);
    f.core
        .effects_after_items
        .push(EffectRequest::SyncAttached {
            id: CONTROL_EFFECT,
            bone: common::part(FtPart::HipN),
        });
    Ok(None)
}

/// ftNs_SpecialHi_ItemPKThunder_CheckNessCollide (80117BBC): the thunder's
/// newest kept position against Ness's centre. It must leave his reach
/// before it can strike him.
fn struck(f: &mut Fighter, thunder: Vec3) -> bool {
    let center = center(f);
    let within =
        (center.x - thunder.x).abs() < REACH_X && (center.y - thunder.y).abs() < REACH_Y;
    let s = scratch(f);
    match s.reach {
        Reach::Apart if within => {
            s.reach = Reach::Struck;
            s.contact = thunder;
            true
        }
        Reach::Leaving if !within => {
            s.reach = Reach::Apart;
            false
        }
        _ => false,
    }
}

/// ftNs_SpecialHiHold_Anim (801187A4) / ftNs_SpecialAirHiHold_Anim
/// (80118B04): the counters; with no thunder the end once both run out; a
/// thunder that struck him launches Ness; one no longer his ends the move.
fn hold_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let s = scratch(f);
    if s.loop_frames > 0 {
        s.loop_frames -= 1;
    }
    if !s.thunder_out && s.gone_frames > 0 {
        s.gone_frames -= 1;
    }
    let s = *s;
    let end = if airborne_row(f) { AIR_END } else { END };
    if !s.thunder_out {
        if s.loop_frames <= 0 && s.gone_frames <= 0 {
            f.change_motion_state(end, p.assets)?;
            stop_effects(f);
        }
        return Ok(None);
    }
    match f.core.owned_article {
        Some(report) => {
            if struck(f, report.point) {
                if airborne_row(f) {
                    launch_into_air(f, p.assets)?;
                } else {
                    launch_from_ground(f, p.assets)?;
                }
            }
        }
        // it_802AB568(pkthunder_gobj) != gobj.
        None => {
            scratch(f).thunder_out = false;
            f.change_motion_state(end, p.assets)?;
            stop_effects(f);
        }
    }
    Ok(None)
}

/// The launch's direction from the contact (NessFloatMath_PKThunder2 and
/// ftNs_SpecialHi_Enter's first half): away from where the thunder struck.
fn away_from_contact(f: &mut Fighter) -> Vec3 {
    let center = center(f);
    let contact = scratch(f).contact;
    Vec3::new(center.x - contact.x, center.y - contact.y, 0.0)
}

/// ftNs_SpecialAirHi_Enter (80118570), also inlined in the aerial control
/// row: Ness faces away from the contact and flies along it at x54.
fn launch_into_air(f: &mut Fighter, a: &FighterAssets) -> Result<()> {
    let away = away_from_contact(f);
    f.physics.facing = if away.x >= 0.0 { 1.0 } else { -1.0 };
    let angle = atan2f(away.y, away.x);
    let s = scratch(f);
    s.vertical_sign = if away.y >= 0.0 { 1.0 } else { -1.0 };
    s.angle = angle;
    let speed = launch(f).speed;
    f.physics.self_velocity.x = speed * cosf(angle);
    f.physics.self_velocity.y = speed * sinf(angle);
    f.change_motion_state(AIR_LAUNCH, a)?;
    lean_along_velocity(f);
    f.character.get_mut::<Ness>().damage_callbacks = false;
    f.physics.jumps_used = f.attributes.jumping.max_jumps as u8;
    Ok(())
}

/// ftNs_SpecialHi_Enter (80118384), from the grounded control row. Off a
/// platform, or struck from below (the launch within 90 degrees of the
/// floor's normal), Ness leaves the floor and launches into the air.
/// Driven into the floor at up to 90 + x60 degrees he slides along it;
/// steeper, he is knocked down (ftCo_80097D40).
fn launch_from_ground(f: &mut Fighter, a: &FighterAssets) -> Result<()> {
    let platform = f.collision.data.floor.flags & melee_types::mp::line_flag::PLATFORM != 0;
    if !platform {
        let away = away_from_contact(f);
        let incidence = melee_lb::vector::angle(f.collision.data.floor.normal, away);
        if incidence >= HALF_PI {
            // retail 0x80118428..30: fadds, then fmuls.
            let limit = DEGREES * (90.0 + launch(f).knockdown_angle);
            if incidence > limit {
                stop_effects(f);
                f.core.set_part_rotation(ROOT_PART, Axis::X, 0.0);
                return f.enter_down_bound(a);
            }
            f.physics.facing = if away.x >= 0.0 { 1.0 } else { -1.0 };
            let s = scratch(f);
            s.vertical_sign = if away.y >= 0.0 { 1.0 } else { -1.0 };
            s.angle = atan2f(away.y, away.x);
            f.change_motion_state(LAUNCH, a)?;
            f.physics.ground_velocity = launch(f).speed * f.physics.facing;
            lean_along_velocity(f);
            f.character.get_mut::<Ness>().damage_callbacks = false;
            f.physics.jumps_used = f.attributes.jumping.max_jumps as u8;
            return Ok(());
        }
    }
    // fp->x1968_jumpsUsed = max_jumps; ftCommon_8007D60C.
    f.core.leave_ground_with_spent_jumps();
    launch_into_air(f, a)
}

/// The launch's glow at its first animation callback (jibakuGFX == 1).
fn launch_effect(f: &mut Fighter) {
    let s = scratch(f);
    s.launch_frames += 1;
    if s.launch_frames == 1 {
        stop_effects(f);
        f.effects.push(EffectRequest::SyncAttached {
            id: LAUNCH_EFFECT,
            bone: common::part(FtPart::HipN),
        });
    }
}

/// ftNs_SpecialHiEnd_Anim (80118900).
fn end_ground_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::wait(f, p.assets)?;
    }
    Ok(None)
}

/// ftNs_SpecialHi_Anim (8011893C): the glow; at the end the grounded end
/// row, with the effects gone.
fn launch_ground_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    launch_effect(f);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.change_motion_state(END, p.assets)?;
        stop_effects(f);
    }
    Ok(None)
}

/// The special fall every aerial row ends in (ftCo_800969D8(gobj, 1, 0, 1,
/// 1.0, x70, x6C)), or a plain fall when x70 is zero; ftCommon_8007D60C
/// first.
fn fall(f: &mut Fighter, a: &FighterAssets) -> Result<()> {
    f.core.leave_ground_with_spent_jumps();
    let (lag, blend) = {
        let l = launch(f);
        (l.landing_lag, l.fall_blend)
    };
    if lag == 0.0 {
        common::fall(f, a)
    } else {
        f.enter_special_fall_blended(a, true, false, true, 1.0, lag, blend)
    }
}

/// ftNs_SpecialAirHiEnd_Anim (80118D60).
fn end_air_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.physics.jumps_used = f.attributes.jumping.max_jumps as u8;
        fall(f, p.assets)?;
    }
    Ok(None)
}

/// ftNs_SpecialAirHi_Anim (80118DF8): the glow; at the end Ness falls at
/// the sink's speed.
fn launch_air_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    launch_effect(f);
    if !f.animation.frames_remaining(&f.skeleton) {
        let sink = scratch(f).sink;
        f.physics.self_velocity.y = -sink.abs();
        fall(f, p.assets)?;
    }
    Ok(None)
}

/// ftNs_SpecialAirHiRebound_Anim (80118EF0).
fn rebound_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        fall(f, p.assets)?;
    }
    Ok(None)
}

/// ftNs_SpecialHiStart_Phys (80118FA4): the gravity delay counts down,
/// then ft_80084F3C.
fn start_ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let s = scratch(f);
    if s.gravity_delay != 0 {
        s.gravity_delay -= 1;
    }
    common::ground_friction(f, p);
}

/// ftNs_SpecialAirHi{Start,Hold,End}_Phys (80119134, 80119194, 801191F4):
/// the gravity delay, then ftCommon_Fall at x50 to the fighter's terminal
/// velocity; ftCommon_ApplyFrictionAir with the aerial friction.
fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let s = scratch(f);
    if s.gravity_delay != 0 {
        s.gravity_delay -= 1;
    } else {
        let gravity = control(f).fall_acceleration;
        let terminal = f.attributes.air.terminal_velocity;
        common::fall_at(f, gravity, terminal);
    }
    let aerial = f.attributes.air.aerial_friction;
    f.physics.animation_velocity.x =
        friction::air_friction_acceleration(f.physics.self_velocity.x, aerial);
    f.core.finish_air_update(p.assets, p.wind);
}

/// ftNs_SpecialHi_Phys (8011901C): the slide loses x5C a frame toward the
/// facing (retail 0x80119048: fnmsubs) unless that would stop it; the
/// velocity is kept; ftCommon_ApplyGroundMovement; the model leans along
/// it.
fn launch_ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let deceleration = launch(f).deceleration;
    let facing = f.physics.facing;
    let speed = f.physics.ground_velocity;
    let slowed = fnmsubs(deceleration, facing, speed);
    let stops = if facing == 1.0 {
        slowed <= SPEED_EPSILON
    } else {
        slowed >= -SPEED_EPSILON
    };
    f.physics.ground_velocity = if stops { speed } else { slowed };
    // The vertical test only writes self_vel.y back to itself.
    scratch(f).kept_velocity = f.physics.self_velocity;
    common::apply_ground_movement(f, &p);
    lean_along_velocity(f);
    common::finish_ground_update(f, &p);
}

/// ftNs_SpecialAirHi_Phys (80119254): the speed (lbVector_Len_xy) loses
/// x5C a frame unless that would stop it, along the launch's angle; the
/// model leans along it and the velocity is kept; once the script sets
/// cmd_vars[0] Ness sinks, x50 faster each frame up to x54.
fn launch_air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let (deceleration, fall, limit) = {
        let l = launch(f);
        (l.deceleration, control(f).fall_acceleration, l.speed)
    };
    let speed = melee_lb::vector::length_xy(f.physics.self_velocity).abs();
    let slowed = speed - deceleration;
    let speed = if slowed <= SPEED_EPSILON { speed } else { slowed };
    let angle = scratch(f).angle;
    f.physics.self_velocity.x = speed * cosf(angle);
    f.physics.self_velocity.y = speed * sinf(angle);
    lean_along_velocity(f);
    scratch(f).kept_velocity = f.physics.self_velocity;
    if f.commands.variables[0] == 1 {
        let s = scratch(f);
        s.sink -= fall;
        if s.sink < -limit {
            s.sink = -limit;
        }
        let sink = s.sink;
        f.physics.position.y += sink;
    }
    f.core.finish_air_update(p.assets, p.wind);
}

/// ftNs_SpecialAirHiRebound_Phys (80119410): the fighter's own gravity
/// and aerial friction.
fn rebound_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let air = &f.attributes.air;
    let (gravity, terminal, aerial) = (air.gravity, air.terminal_velocity, air.aerial_friction);
    common::fall_at(f, gravity, terminal);
    f.physics.animation_velocity.x =
        friction::air_friction_acceleration(f.physics.self_velocity.x, aerial);
    f.core.finish_air_update(p.assets, p.wind);
}

/// ftNs_SpecialHi{Start,Hold,End}_Coll (80119460, 801194CC, 80119538):
/// ft_80082708; off the floor every jump is spent (ftCommon_8007D60C) and
/// the aerial counterpart continues at the same frame.
fn ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::stays_grounded(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("PK Thunder collision assets");
    let state = match f.motion_state.action {
        START => AIR_START,
        HOLD => AIR_HOLD,
        _ => AIR_END,
    };
    f.core.leave_ground_with_spent_jumps();
    let frame = f.animation.frame;
    common::change(f, state, common::GROUND_AIR, frame, 1.0, assets)
}

/// ftNs_SpecialAirHi{Start,Hold,End}_Coll (80119798, 80119804, 80119870):
/// ft_80081D0C; a landing continues in the grounded counterpart
/// (ftCommon_AirToGroundStateChange).
fn air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("PK Thunder landing assets");
    let state = match f.motion_state.action {
        AIR_START => START,
        AIR_HOLD => HOLD,
        _ => END,
    };
    common::air_to_ground(f, state, common::GROUND_AIR, assets)
}

/// The knockdown the launch ends in against a steep surface: the effects
/// go, the model root levels and ftCo_80097D40 enters DownBound.
fn knock_down(f: &mut Fighter, a: &FighterAssets) -> Result<()> {
    stop_effects(f);
    f.core.set_part_rotation(ROOT_PART, Axis::X, 0.0);
    f.enter_down_bound(a)
}

/// ftNs_SpecialHi_Coll (801195A4). Off the floor at a wall the move ends
/// in the aerial end row with its scratch reset; off the floor otherwise
/// the aerial launch continues. On the floor a ceiling or wall knocks Ness
/// down; else the launch's angle follows the floor.
fn launch_ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    let assets = p.assets.expect("PK Thunder 2 collision assets");
    let grounded = common::stays_grounded(f, &mut p);
    let env = f.collision.data.env_flags as u32;
    let wall = env & (collide::LEFT_WALL_MASK | collide::RIGHT_WALL_MASK) != 0;
    let frame = f.animation.frame;
    if !grounded {
        f.core.leave_ground_with_spent_jumps();
        if wall {
            reset(f);
            return common::change(f, AIR_END, common::GROUND_AIR, frame, 1.0, assets);
        }
        return common::change(f, AIR_LAUNCH, LAUNCH_COLL, frame, 1.0, assets);
    }
    if wall || env & collide::CEILING_MASK != 0 {
        f.physics.ground_velocity = 0.0;
        return knock_down(f, assets);
    }
    let normal = f.collision.data.floor.normal;
    let facing_right = f.physics.facing == 1.0;
    let quarter = if (normal.y > 0.0) == facing_right {
        -std::f64::consts::FRAC_PI_2
    } else {
        std::f64::consts::FRAC_PI_2
    };
    scratch(f).angle = (quarter + f64::from(atan2f(normal.y, normal.x))) as f32;
    Ok(())
}

/// `angle` wrapped into [0, tau] in double steps.
fn normalized(mut angle: f32) -> f32 {
    while angle < 0.0 {
        angle = (f64::from(angle) + TAU) as f32;
    }
    while f64::from(angle) > TAU {
        angle = (f64::from(angle) - TAU) as f32;
    }
    angle
}

/// ftNs_SpecialAirHi_CollisionModVel (80117F24): against a wall too
/// shallow to rebound from, the velocity turns to run along it (a quarter
/// turn from the wall's normal, toward the side the launch came from),
/// rotated about Z by lbVector_RotateAboutUnitAxis.
fn slide_along_wall(f: &mut Fighter) {
    let heading = normalized(scratch(f).angle);
    scratch(f).angle = heading;
    let env = f.collision.data.env_flags as u32;
    let quarter = std::f64::consts::FRAC_PI_2;
    let mut along = 0.0_f32;
    if env & collide::LEFT_WALL_MASK != 0 {
        let n = f.collision.data.left_facing_wall.normal;
        let wall = normalized(atan2f(n.y, n.x));
        let opposite = normalized((PI + f64::from(heading)) as f32);
        along = if opposite - wall < 0.0 {
            (f64::from(wall) + quarter) as f32
        } else {
            (f64::from(wall) - quarter) as f32
        };
    }
    if env & collide::RIGHT_WALL_MASK != 0 {
        let n = f.collision.data.right_facing_wall.normal;
        let wall = atan2f(n.y, n.x);
        let opposite = normalized((PI + f64::from(wall)) as f32);
        along = if heading - opposite < 0.0 {
            (f64::from(wall) + quarter) as f32
        } else {
            (f64::from(wall) - quarter) as f32
        };
    }
    f.physics.self_velocity = melee_lb::vector::rotate(
        f.physics.self_velocity,
        Vec3::new(0.0, 0.0, 1.0),
        along - heading,
    );
    scratch(f).angle = atan2f(f.physics.self_velocity.y, f.physics.self_velocity.x);
}

/// The rebound off a steep ceiling or wall with unit normal `normal`: the
/// velocity mirrors and halves, its X clamped to the air drift; Ness faces
/// along it; the rebound row with its dust (efSync 1030) at his position.
fn rebound(f: &mut Fighter, normal: Vec3, a: &FighterAssets) -> Result<()> {
    let mirrored = melee_lb::vector::mirror(f.physics.self_velocity, normal);
    f.physics.self_velocity.x = mirrored.x * REBOUND_SCALE;
    f.physics.self_velocity.y = mirrored.y * REBOUND_SCALE;
    let maximum = f.attributes.air.air_drift_max;
    common::clamp_self_velocity_x(f, maximum);
    f.physics.facing = if f.physics.self_velocity.x >= 0.0 {
        1.0
    } else {
        -1.0
    };
    stop_effects(f);
    common::change(f, REBOUND, KEEP_GFX, 0.0, 1.0, a)?;
    f.step_animation(a);
    let position = f.physics.position;
    f.effects.push(EffectRequest::SurfaceRebound {
        position,
        angle: atan2f(-normal.x, normal.y),
    });
    Ok(())
}

/// ftNs_SpecialAirHi_Coll (801198DC): the velocity the physics kept comes
/// back; a landing (ft_CheckGroundAndLedge toward the facing) knocks Ness
/// down when it is steeper than 90 + x64 degrees and otherwise continues
/// the launch along the floor; a ledge is caught; a ceiling or wall steeper
/// than that rebounds, a shallower wall turns the launch along it.
fn launch_air_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    let assets = p.assets.expect("PK Thunder 2 collision assets");
    f.physics.self_velocity = scratch(f).kept_velocity;
    // retail 0x80119950..58: fadds, then fmuls.
    let limit = DEGREES * (90.0 + launch(f).wall_hug_angle);
    let c = &mut f.core;
    air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    let landed = air::collide_pass(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        c.status.ledge_cooldown == 0,
    );
    if landed {
        let incidence =
            melee_lb::vector::angle(f.collision.data.floor.normal, f.physics.self_velocity);
        if incidence > limit {
            f.physics.self_velocity = Vec3::ZERO;
            return knock_down(f, assets);
        }
        return common::air_to_ground(f, LAUNCH, LAUNCH_COLL, assets);
    }
    if f.try_grab_ledge(assets, p.map)? {
        // ftCliffCommon_80081298 enters CliffCatch itself;
        // ftCliffCommon_80081370 then runs a second time.
        return f.enter_cliff_catch(assets, p.map);
    }
    let env = f.collision.data.env_flags as u32;
    let velocity = f.physics.self_velocity;
    if env & collide::CEILING_MASK != 0 {
        let normal = f.collision.data.ceiling.normal;
        if melee_lb::vector::angle(normal, velocity) > limit {
            rebound(f, normal, assets)?;
        }
    } else if env & collide::LEFT_WALL_MASK != 0 {
        let normal = f.collision.data.left_facing_wall.normal;
        if melee_lb::vector::angle(normal, velocity) > limit {
            rebound(f, normal, assets)?;
        } else {
            slide_along_wall(f);
        }
    } else if env & collide::RIGHT_WALL_MASK != 0 {
        let normal = f.collision.data.right_facing_wall.normal;
        if melee_lb::vector::angle(normal, velocity) > limit {
            rebound(f, normal, assets)?;
        } else {
            slide_along_wall(f);
        }
    }
    Ok(())
}

/// ftNs_SpecialAirHiRebound_Coll (80119D58): a landing
/// (ft_CheckGroundAndLedge on either side) knocks Ness down; else a ledge
/// is caught.
fn rebound_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    let assets = p.assets.expect("PK Thunder rebound collision assets");
    if common::lands_or_finds_ledge(f, &mut p) {
        f.physics.self_velocity = Vec3::ZERO;
        return knock_down(f, assets);
    }
    if f.try_grab_ledge(assets, p.map)? {
        return f.enter_cliff_catch(assets, p.map);
    }
    Ok(())
}

/// ftNs_SpecialHi_TakeDamage (80117E60), from ftNs_Init_OnDamage: the
/// thunder and its tail lose their owner (it_802AB9C0); in a PK Thunder
/// row the effects go; the callbacks clear and the model root levels.
pub fn take_damage(f: &mut Fighter) {
    if scratch(f).thunder_out {
        let owner = f.player.id;
        f.core.item_requests.push(ItemRequest::Control {
            owner,
            kind: ItemKind::NessPKThunder,
            control: ItemControl::Orphan,
        });
    }
    thunder_gone(f);
}

/// ftNs_SpecialHi_ItemPKThunderRemove (80117DD4), from the thunder's end
/// (it_802AB90C): Ness's hold on it goes; in a PK Thunder row the effects
/// go; the callbacks clear and the model root levels.
pub fn thunder_gone(f: &mut Fighter) {
    scratch(f).thunder_out = false;
    if (START.0..=REBOUND.0).contains(&f.motion_state.action.0) {
        stop_effects(f);
    }
    f.character.get_mut::<Ness>().damage_callbacks = false;
    f.core.set_part_rotation(ROOT_PART, Axis::X, 0.0);
}
