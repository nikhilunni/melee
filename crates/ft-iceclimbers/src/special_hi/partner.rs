//! Nana's side of the Belay, ftnanaspecialhi.c (801230D0..801237F8) and
//! ftnana.c's checks (8012300C, 8012309C): rows 361..366 (both climbers
//! carry them; only Nana enters them).
//!
//! Popo's start pulls Nana in (ftNn_Init_801232A4): she hangs from his
//! right hand (361, ftNn_Init_801230D0) until her animation ends, then is
//! flung up (365, ftNn_Init_801237F8) and lands (362). While she belays her
//! accessory tells Popo where her left hand is (fn_80123218), where his
//! rope's tail hangs.
use super::attrs;
use crate::climber::{self, vars, Accessory};
use crate::partner::PartnerView;
use gekko_math::{
    fma::fmadds,
    msl::{cosf, sinf, sqrtf},
};
use hsd_types::Vec3;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, FighterProc, MotionRow, PhysicsPhase},
        ActionId, Fighter,
    },
};
use melee_types::mp::collide;

/// ftPp_MS_SpecialHi_0 (361): hanging from Popo's hand.
pub const HANG: ActionId = ActionId(361);
/// ftPp_MS_SpecialHi_1 (362): landed.
pub const LAND: ActionId = ActionId(362);
/// ftPp_MS_SpecialHi_4 (365): flung up the rope.
pub const FLIGHT: ActionId = ActionId(365);
/// ftPp_MS_SpecialHi_0..5: Nana's Belay (Popo's physics and the CPU
/// read it).
pub const BELAY: core::ops::RangeInclusive<u16> = 361..=366;
/// ftNn_Init_8012309C: Nana is in flight or has landed from it.
const FLOWN: core::ops::RangeInclusive<u16> = 362..=366;
/// ftNn_Init_801230D0: Popo's start and throw rows (347..352) hold her.
const HOLDING: core::ops::RangeInclusive<u16> = 347..=352;

/// ftPp_SpecialHi_0_Phys: a Nana whose Popo let go falls from 8 behind
/// (retail @275).
const LET_GO_BACKSTEP: f32 = 8.0;
/// ftNn_Init_801237F8: the launch starts 4 ahead and 7 up, in model scale
/// (retail @340, @341).
const LAUNCH_AHEAD: f32 = 4.0;
const LAUNCH_UP: f32 = 7.0;
/// ftPp_SpecialHi_4_Coll: a wall reverses her (retail @326).
const REVERSE: f32 = -1.0;

/// ftNn_Init_8012300C's switch on the x2071 nibble: these classes keep
/// Nana out of the Belay.
fn busy(nibble: u8) -> bool {
    matches!(nibble, 1 | 3..=8 | 10..=13)
}

/// ftNn_Init_MotionStateTable rows 361, 362 and 365; 363, 364 and 366
/// are never entered (their callbacks are NULL or empty) and stay
/// unported.
pub const ROWS: [MotionRow; 3] = [
    climber::row(
        HANG.0,
        hang_anim,
        climber::no_input,
        hang_physics,
        no_collision,
    ),
    climber::row(
        LAND.0,
        land_anim,
        climber::no_input,
        callbacks::physics::guard_on,
        land_collision,
    ),
    climber::row(
        FLIGHT.0,
        flight_anim,
        climber::no_input,
        flight_physics,
        flight_collision,
    ),
];

/// What a Belay proc reads of the other climber beyond the basic view.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Wants {
    /// A part of the partner's (an FtPart used as the parts index) whose
    /// world position the proc reads.
    pub part: Option<usize>,
    /// The partner's CollData (ft_800849EC copies it).
    pub collision: bool,
}

/// The Belay's reads of the player's other climber, for
/// `crate::partner::observe`: Popo's physics reads Nana's left hand while
/// she belays; Nana hanging reads Popo's right hand while he holds her,
/// and (for her launch) his CollData.
pub fn wants(f: &Fighter, proc: FighterProc, partner: ActionId) -> Wants {
    let action = f.motion_state.action.0;
    match (climber::climber(f), proc) {
        (crate::init::Climber::Popo, FighterProc::Update)
            if [
                super::START,
                super::THROW,
                super::CLIMB,
                super::AIR_START,
                super::AIR_THROW,
                super::AIR_CLIMB,
            ]
            .contains(&ActionId(action))
                && BELAY.contains(&partner.0) =>
        {
            Wants {
                part: Some(super::LEFT_HAND),
                collision: false,
            }
        }
        (crate::init::Climber::Nana, FighterProc::Animation) if action == HANG.0 => Wants {
            part: None,
            collision: true,
        },
        (crate::init::Climber::Nana, FighterProc::Update) if action == HANG.0 => Wants {
            part: HOLDING.contains(&partner.0).then_some(super::RIGHT_HAND),
            collision: false,
        },
        _ => Wants::default(),
    }
}

/// The other climber as observed before this proc.
fn other(f: &Fighter) -> Option<PartnerView> {
    f.character.get::<crate::init::IceClimber>().partner
}

/// checkNanaInRange (Popo's start): Nana within x7C (separate fsubs,
/// fmuls, fadds; sqrtf__Ff) and free to join (ftNn_Init_8012300C), which
/// pulls her in.
pub fn try_join(f: &mut Fighter) -> bool {
    let Some(nana) = other(f) else {
        return false;
    };
    let dx = f.physics.position.x - nana.position.x;
    let dy = f.physics.position.y - nana.position.y;
    if sqrtf(dx * dx + dy * dy) >= attrs(f).partner_reach {
        return false;
    }
    if !free_to_join(&nana) {
        return false;
    }
    let facing = f.physics.facing;
    crate::partner::work(f).belay_join = Some(facing);
    true
}

/// ftNn_Init_8012300C (8012300C): not out of play, not in hitlag, and her
/// motion's class is one that lets her go.
fn free_to_join(nana: &PartnerView) -> bool {
    if nana.disabled || nana.in_hitlag {
        return false;
    }
    let class = nana.class.unwrap_or_else(|| {
        unimplemented!(
            "ftNn_Init_8012300C: x2071 of Nana's action {} (no x4_flags column)",
            nana.action.0
        )
    });
    !busy(class)
}

/// ftNn_Init_8012309C (8012309C): Nana is flying or has landed from it.
pub fn in_flight(f: &Fighter) -> bool {
    other(f).is_some_and(|nana| !nana.disabled && FLOWN.contains(&nana.action.0))
}

/// ftNn_Init_801232A4 (801232A4): Nana turns to Popo's facing, leaves the
/// ground (every jump spent) or spends her jumps, and hangs
/// (ftNn_Init_801233F8: 361, accessory4 fn_80123218); cmd_vars[0]
/// cleared and x2222_b2 set.
pub fn join(f: &mut Fighter, assets: &FighterAssets, facing: f32) -> Result<()> {
    f.physics.facing = facing;
    if f.physics.ground_or_air == melee_types::GroundOrAir::Ground {
        f.leave_ground_with_spent_jumps();
    } else {
        f.physics.jumps_used = f.attributes.jumping.max_jumps as u8;
    }
    f.change_motion_state(HANG, assets)?;
    install_anchor(f);
    f.commands.variables[0] = 0;
    Ok(())
}

/// accessory4_cb = fn_80123218.
fn install_anchor(f: &mut Fighter) {
    vars(f).accessory = Accessory::RopeAnchor;
    f.core.arm_accessory4();
}

/// fn_80123218 (80123218): Nana's left hand (lb_8000B1CC on L4thNb) for
/// Popo's u.pp.x2240.
pub fn anchor_accessory(f: &mut Fighter) {
    let c = &mut f.core;
    let hand = melee_ft::fighter::caches::part_position(
        &mut c.skeleton,
        &c.animation,
        super::LEFT_HAND,
        Vec3::ZERO,
    );
    crate::partner::work(f).rope_anchor = Some(hand);
}

/// ftPp_SpecialHi_0_Anim (80123348): at the end, the launch.
fn hang_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        launch(f, p.assets)?;
    }
    Ok(None)
}

/// ftNn_Init_801237F8 (801237F8): Popo's CollData (ft_800849EC), the
/// launch velocity along x140 at x13C (fmuls), the launch point 4 ahead
/// and 7 up in model scale (fmuls, fmadds), then the flight (365) and the
/// anchor accessory.
fn launch(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    if let Some(popo) = climber::payload(f).partner_collision {
        melee_mp::copy_coll_data(&popo, &mut f.core.collision.data, 2);
    }
    let (speed, angle) = {
        let a = attrs(f);
        (a.partner_launch_speed, a.partner_launch_angle)
    };
    let facing = f.physics.facing;
    f.physics.self_velocity.x = facing * (speed * cosf(angle));
    f.physics.self_velocity.y = speed * sinf(angle);
    let scale = f.player.scale;
    // retail 80123890 / 801238A4: fmadds.
    f.physics.position.x = fmadds(LAUNCH_AHEAD * facing, scale, f.physics.position.x);
    f.physics.position.y = fmadds(LAUNCH_UP, scale, f.physics.position.y);
    f.change_motion_state(FLIGHT, assets)?;
    install_anchor(f);
    Ok(())
}

/// ftPp_SpecialHi_0_Phys (80123390): held by Popo (ftNn_Init_801230D0),
/// or, once he let go, 8 back and falling (fnmsubs, ftCo_Fall_Enter).
fn hang_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    if !hang_from_leader(f) {
        // retail 801233C8: fnmsubs.
        f.physics.position.x =
            gekko_math::fma::fnmsubs(LET_GO_BACKSTEP, f.physics.facing, f.physics.position.x);
        climber::finish(f, p.assets, true).expect("Fall assets");
    }
    f.core.finish_air_update(p.assets, p.wind);
}

/// ftNn_Init_801230D0 (801230D0): with Popo in his start or throw, Nana
/// takes his facing (turning her model), moves so her XRotN sits on his
/// right hand (R4thNb), and animates at his rate (stopped while he is in
/// hitlag). False once he let go; true without a Popo.
fn hang_from_leader(f: &mut Fighter) -> bool {
    let Some(popo) = other(f) else {
        return true;
    };
    if !HOLDING.contains(&popo.action.0) {
        return false;
    }
    if f.physics.facing != popo.facing {
        f.physics.facing = popo.facing;
        super::face_model(f);
    }
    let hand = popo.part_position.expect("Popo's right hand");
    let c = &mut f.core;
    let own = melee_ft::fighter::caches::part_position(
        &mut c.skeleton,
        &c.animation,
        super::HIPS,
        Vec3::ZERO,
    );
    let p = &mut c.physics.position;
    let offset = Vec3::new(p.x - own.x, p.y - own.y, p.z - own.z);
    *p = Vec3::new(hand.x + offset.x, hand.y + offset.y, hand.z + offset.z);
    let rate = if popo.in_hitlag {
        Some(0.0)
    } else {
        (c.animation.speed != popo.animation_rate).then_some(popo.animation_rate)
    };
    if let Some(rate) = rate {
        c.animation.set_rate(&mut c.skeleton, rate, false);
    }
    true
}

/// ftPp_SpecialHi_0_Coll: nothing (the hand carries her).
fn no_collision(_: &mut Fighter, _: CollisionPhase<'_>) -> Result<()> {
    Ok(())
}

/// ftPp_SpecialHi_1_Anim (80123448): Wait at the end.
fn land_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        climber::finish(f, p.assets, false)?;
    }
    Ok(None)
}

/// ftPp_SpecialHi_1_Coll (801235BC): off the floor, the flight again at
/// the current frame (ftNn_Init_80123720: every jump spent).
fn land_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if super::stays_on_edge(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Belay collision assets");
    f.leave_ground_with_spent_jumps();
    let frame = f.animation.frame;
    f.change_motion_state_with_flags(FLIGHT, assets, super::GROUND_AIR_FLAGS, frame, 1.0)?;
    install_anchor(f);
    Ok(())
}

/// ftPp_SpecialHi_4_Anim (80123484): at the end the special fall
/// (ftCo_80096900(gobj, 0, 1, false, x130, x134)).
fn flight_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        let (mobility, lag) = {
            let a = attrs(f);
            (a.partner_fall_mobility, a.partner_landing_lag)
        };
        f.enter_special_fall(p.assets, false, true, false, mobility, lag)?;
    }
    Ok(None)
}

/// ftPp_SpecialHi_4_Phys (8012350C): gravity x144 (terminal x148); past
/// x138 the stick drifts her (the common drift scaled by xB0 and xB4,
/// fmuls), else falling she slows.
fn flight_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let a = *attrs(f);
    super::fall(f, a.partner_gravity, a.partner_terminal_velocity);
    let x = f.input.current.stick.x;
    let magnitude = if x < 0.0 { -x } else { x };
    if magnitude > a.partner_stick_threshold {
        let air = &f.attributes.air;
        let (acceleration, maximum) = (
            air.air_drift_stick_mul * a.climb_drift_scale,
            air.air_drift_max * a.climb_drift_max_scale,
        );
        super::drift(f, acceleration, maximum);
    } else if f.physics.self_velocity.y < 0.0 {
        super::aerial_friction(f);
    }
    f.core.finish_air_update(p.assets, p.wind);
}

/// ftPp_SpecialHi_4_Coll (801235F8): landing (ft_80081D0C), the landed
/// row (ftNn_Init_8012378C); else a wall she flies into turns her back
/// at x14C of her speed (fmuls), and a ceiling stops her rise.
fn flight_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if climber::lands(f, &mut p) {
        let assets = p.assets.expect("Belay collision assets");
        f.land();
        let frame = f.animation.frame;
        f.change_motion_state_with_flags(LAND, assets, super::GROUND_AIR_FLAGS, frame, 1.0)?;
        install_anchor(f);
        return Ok(());
    }
    let env = f.collision.data.env_flags as u32;
    let rebound = attrs(f).partner_wall_rebound;
    let vx = f.physics.self_velocity.x;
    if (env & collide::LEFT_WALL_MASK != 0 && vx > 0.0)
        || (env & collide::RIGHT_WALL_MASK != 0 && vx < 0.0)
    {
        f.physics.self_velocity.x = vx * (REVERSE * rebound);
        f.physics.facing *= REVERSE;
        super::face_model(f);
    } else if env & collide::CEILING_MASK != 0 {
        f.physics.self_velocity.y = 0.0;
    }
    Ok(())
}
