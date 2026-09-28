//! Pound, ftpurinspecials.c (8013D234..8013D658). The aerial script sets
//! cmd_vars[0] once to aim the lunge and walks cmd_vars[1] through the
//! three aerial physics phases, as in Falcon Punch.
use crate::{common, init::Jigglypuff};
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        ActionId, Fighter, MotionPreservation,
    },
};

/// ftPr_MS_SpecialS (363) and ftPr_MS_SpecialAirS (364).
pub const GROUND: ActionId = ActionId(363);
pub const AIR: ActionId = ActionId(364);

/// `MTXDegToRad(1)` as MWCC rounds it (retail @244).
const DEGREES_TO_RADIANS: f32 = 0.017453292;

/// ftPr_MF_SpecialS_Coll: ftCommon_GroundAirColl_MF | KeepGfx | SkipHit.
const GROUND_AIR_PRESERVATION: MotionPreservation = MotionPreservation {
    hit_status: false,
    hitboxes: true,
    effects: true,
    fast_fall: false,
};

/// ftPr_SpecialS_Enter (8013D234) / ftPr_SpecialAirS_Enter (8013D2A0).
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    let retained = f.inherited_scratch_word();
    f.change_motion_state(if air { AIR } else { GROUND }, a)
        .expect("Pound assets");
    f.character.get_mut::<Jigglypuff>().retained_word = retained;
    f.step_animation(a);
    f.commands.variables[..4].fill(0);
}

/// ftPr_SpecialS_Anim / ftPr_SpecialAirS_Anim: Wait or Fall at the end.
pub fn anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        let air = f.motion_state.action == AIR;
        common::finish(f, p.assets, air)?;
    }
    Ok(None)
}

/// ftPr_SpecialS_IASA / ftPr_SpecialAirS_IASA are empty.
pub fn input(_: &mut Fighter, _: InputPhase<'_>) {}

/// ftPr_SpecialS_Phys: ft_80084FA8's root motion.
pub fn ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::jab(f, p);
}

/// calcAngleRadians (inlined): the stick's vertical magnitude, clamped to
/// attribute +E0, less +DC, scaled to the maximum angle and signed like
/// the stick. 8013D430..64: fsubs, fmuls, fdivs, fmuls, each rounded.
fn lunge_angle(stick_y: f32, pound: &crate::attributes::PoundAttributes) -> f32 {
    let maximum = pound.angle_stick_maximum;
    let minimum = pound.angle_stick_minimum;
    let mut magnitude = if stick_y < 0.0 { -stick_y } else { stick_y };
    if magnitude > maximum {
        magnitude = maximum;
    }
    magnitude -= minimum;
    if magnitude < 0.0 {
        magnitude = 0.0;
    }
    if stick_y < 0.0 {
        magnitude = -magnitude;
    }
    DEGREES_TO_RADIANS * (magnitude * pound.maximum_angle_degrees / (maximum - minimum))
}

/// ftPr_SpecialAirS_Phys (8013D3AC): the script's cue aims the lunge
/// (counting it in Fighter +222C); then the phase in cmd_vars[1] selects
/// ft_80084EEC, a velocity decay, or ft_80084DB0.
pub fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    if f.commands.variables[0] != 0 {
        f.commands.variables[0] = 0;
        let puff = f.character.get_mut::<Jigglypuff>();
        puff.pound_count = puff.pound_count.wrapping_add(1);
        let pound = &puff.attributes.pound;
        let angle = lunge_angle(f.core.input.current.stick.y, pound);
        let speed = pound.air_speed;
        // 8013D474 / 8013D48C..90: separate fmuls.
        f.core.physics.self_velocity.y = speed * gekko_math::msl::sinf(angle);
        f.core.physics.self_velocity.x =
            speed * (f.core.physics.facing * gekko_math::msl::cosf(angle));
    }
    match f.commands.variables[1] {
        0 => callbacks::physics::air_friction(f, p),
        1 => {
            // 8013D4CC..E4: separate fmuls, y before x.
            let decay = f
                .character
                .get::<Jigglypuff>()
                .attributes
                .pound
                .air_speed_decay;
            f.physics.self_velocity.y *= decay;
            f.physics.self_velocity.x *= decay;
            f.core.finish_air_update(p.assets, p.wind);
        }
        2 => callbacks::physics::fall(f, p),
        _ => f.core.finish_air_update(p.assets, p.wind),
    }
}

/// ftPr_SpecialS_Coll: off the edge, ftPr_SpecialS_8013D590 continues in
/// the air (ftCommon_GroundToAirStateChange).
pub fn ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::stays_grounded(f, &mut p) {
        return Ok(());
    }
    f.leave_ground();
    f.change_ground_air_motion(
        AIR,
        p.assets.expect("Pound collision assets"),
        GROUND_AIR_PRESERVATION,
    )
}

/// ftPr_SpecialAirS_Coll: landing continues on the ground
/// (ftPr_SpecialS_8013D5F0: ftCommon_AirToGroundStateChange, then
/// ftCommon_ClampAirDrift).
pub fn air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    f.land();
    f.change_ground_air_motion(
        GROUND,
        p.assets.expect("Pound landing assets"),
        GROUND_AIR_PRESERVATION,
    )?;
    // ftCommon_ClampAirDrift (8007D468): clamp only X.
    let maximum = f.attributes.air.air_drift_max;
    f.physics.self_velocity.x = f.physics.self_velocity.x.clamp(-maximum, maximum);
    Ok(())
}
