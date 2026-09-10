//! Capture timer, mash input, pummel entry and paired escape.
use super::{
    assets::{FighterAssets, Result},
    state::{AnimationPhase, PhysicsPhase},
    Fighter, MotionData,
};
use crate::{
    anim::WaitChoice,
    input::{Buttons, FighterInput},
};
use hsd_archive::Archive;
use melee_types::{CommonMotionState as S, GroundOrAir};

pub struct Parameters {
    pub stick_threshold: f32,
    pub base_timer: f32,
    pub handicap_scale: f32,
    pub handicap_origin: f32,
    pub rank_scale: f32,
    pub rank_origin: f32,
    pub percent_scale: f32,
    pub decrement: f32,
    pub mash_decrement: f32,
    pub fast_frames: f32,
    pub fast_rate: f32,
    pub escape_speed: f32,
    pub escape_friction: f32,
}
impl Parameters {
    /// ftCommonData: ftCo_CapturePulled/Wait/Cut and ftCommon_GrabMash.
    pub fn read(archive: &Archive, base: u32) -> Result<Self> {
        let r = archive.reader();
        Ok(Self {
            stick_threshold: r.f32(base + 0x308)?,
            base_timer: r.f32(base + 0x354)?,
            handicap_scale: r.f32(base + 0x358)?,
            handicap_origin: r.f32(base + 0x35C)?,
            rank_scale: r.f32(base + 0x360)?,
            rank_origin: r.f32(base + 0x364)?,
            percent_scale: r.f32(base + 0x368)?,
            escape_friction: r.f32(base + 0x36C)?,
            escape_speed: r.f32(base + 0x370)?,
            decrement: r.f32(base + 0x3A4)?,
            mash_decrement: r.f32(base + 0x3A8)?,
            fast_frames: r.f32(base + 0x3B0)?,
            fast_rate: r.f32(base + 0x3B4)?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct CaptureState {
    pub timer: f32,
    pub elapsed: f32,
    fast_remaining: f32,
    stick_directions: [i8; 2],
}
impl CaptureState {
    /// fn_800DA8E4 (800DA8E4): handicap/rank-adjusted initial grab timer.
    pub fn new(percent: f32, handicap: u8, rank: u8, p: &Parameters) -> Self {
        let rank_term = p.rank_scale * (p.rank_origin - (f32::from(rank) + 1.0));
        // Retail 800DA9CC/800DA9D4: fmadds, with separate intervening fadds.
        let base = gekko_math::fma::fmadds(
            p.handicap_scale,
            p.handicap_origin - f32::from(handicap),
            p.base_timer,
        );
        Self {
            timer: gekko_math::fma::fmadds(percent, p.percent_scale, base + rank_term),
            elapsed: 0.0,
            fast_remaining: 0.0,
            stick_directions: [0; 2],
        }
    }

    /// ftCommon_GrabMash (8007DC08): one button decrement and one stick decrement.
    /// Neutral retains each axis's last non-neutral direction.
    fn mash(&mut self, input: &FighterInput, p: &Parameters) -> bool {
        let buttons = input
            .pressed
            .intersects(Buttons::A | Buttons::B | Buttons::XY | Buttons::SHIELD);
        if buttons {
            self.timer -= p.mash_decrement;
        }
        let previous = self.stick_directions;
        for (direction, value) in self
            .stick_directions
            .iter_mut()
            .zip([input.current.stick.x, input.current.stick.y])
        {
            if value < -p.stick_threshold {
                *direction = -1;
            }
            if value > p.stick_threshold {
                *direction = 1;
            }
        }
        let stick = previous != self.stick_directions;
        if stick {
            self.timer -= p.mash_decrement;
        }
        buttons || stick
    }
}

/// ftCo_CaptureWaitLw_Anim -> ftCo_CaptureWaitHi_Anim (800DBE24 / 800DB908).
/// The scene performs the reciprocal escape immediately after this animation.
pub fn capture_animation(f: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(phase.assets);
    let p = &phase.assets.grab_escape;
    let input = f.input.clone();
    let MotionData::Capture(capture) = &mut f.core.state_data else {
        panic!("capture scratch missing")
    };
    // Retail increments elapsed in double precision, then rounds to single.
    capture.elapsed = (f64::from(capture.elapsed) + 1.0) as f32;
    capture.timer -= p.decrement;
    let mashed = capture.mash(&input, p);
    let mut rate = None;
    if capture.fast_remaining != 0.0 {
        capture.fast_remaining -= 1.0;
        if capture.fast_remaining <= 0.0 && !mashed {
            capture.fast_remaining = 0.0;
            rate = Some(1.0);
        }
    }
    if capture.fast_remaining <= 0.0 && mashed {
        capture.fast_remaining = p.fast_frames;
        rate = Some(p.fast_rate);
    }
    if let Some(rate) = rate {
        f.core.animation.set_rate(&mut f.core.skeleton, rate, false);
    }
    Ok(None)
}

/// ftCo_8008EC90 / ftCo_800DC3A4: damage without releasing or launching the victim.
pub(super) fn capture_damage(
    f: &mut Fighter,
    hit: &melee_coll::damage::ReceivedHit,
    assets: &FighterAssets,
) -> Result<i32> {
    if !matches!(f.motion_state.id, S::CaptureWaitLw | S::CaptureDamageLw) {
        unimplemented!("ftCo_8008EC90: captured damage outside low capture");
    }
    f.core.physics.percent += hit.descriptor.damage;
    f.core.input.pressed = Buttons::default();
    f.core.input.released = Buttons::default();
    f.change_motion_state(S::CaptureDamageLw.into(), assets)?;
    let MotionData::Capture(capture) = &mut f.core.state_data else {
        panic!("capture scratch missing")
    };
    capture.fast_remaining = 0.0;
    f.core.status.grab_exclusions = super::ledge::GrabExclusions::ALL;
    Ok(gekko_math::msl::fctiwz(hit.descriptor.damage).max(1))
}

/// ftCo_CaptureDamageLw_Anim (800DC470): count down without the wait state's escape check.
pub fn capture_damage_animation(
    f: &mut Fighter,
    phase: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(phase.assets);
    let MotionData::Capture(capture) = &mut f.core.state_data else {
        panic!("capture scratch missing")
    };
    // fn_800DB8A4: increment in f64, then subtract the f32 timer decrement and mash.
    capture.elapsed = (f64::from(capture.elapsed) + 1.0) as f32;
    capture.timer -= phase.assets.grab_escape.decrement;
    capture.mash(&f.core.input, &phase.assets.grab_escape);
    if !f.animation.frames_remaining(&f.skeleton) {
        super::grab::capture_wait(f, phase.assets)?;
    }
    Ok(None)
}

/// fn_800DA4C0 / fn_800DA4FC: pummel has priority over all four throws.
pub fn pummel_input(f: &mut Fighter, phase: super::state::InputPhase<'_>) {
    if f.input.pressed.intersects(Buttons::A) {
        f.core.physics.ground_velocity = 0.0;
        f.change_motion_state(S::CatchAttack.into(), phase.assets)
            .expect("pummel entry");
        f.core.status.grab_exclusions = super::ledge::GrabExclusions::ALL;
    }
}

/// ftCo_CatchAttack_Anim (800DA56C), fn_800DA2B0: return without a new grab flash.
pub fn pummel_animation(f: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(phase.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.change_motion_state(S::CatchWait.into(), phase.assets)?;
        f.core.status.grab_exclusions = super::ledge::GrabExclusions::ALL;
    }
    Ok(None)
}

/// ftCo_800DA698 / ftCo_CaptureCut_Enter (800DA698 / 800DC750).
pub fn release(
    victim: &mut Fighter,
    attacker: &mut Fighter,
    va: &FighterAssets,
    aa: &FighterAssets,
) -> Result<()> {
    if victim.physics.ground_or_air != GroundOrAir::Ground
        || attacker.physics.ground_or_air != GroundOrAir::Ground
    {
        unimplemented!("ftCo_CatchCut / CaptureCut: airborne escape");
    }
    attacker.core.physics.ground_velocity = -attacker.physics.facing * aa.grab_escape.escape_speed;
    attacker.change_motion_state(S::CatchCut.into(), aa)?;
    victim.core.combat.grab = None;
    attacker.core.combat.grab = None;
    victim.core.physics.ground_velocity = -victim.physics.facing * va.grab_escape.escape_speed;
    victim.change_motion_state(S::CaptureCut.into(), va)?;
    Ok(())
}

/// ftCo_CatchCut_Anim / ftCo_CaptureCut_Anim: ground -> Wait, air -> Fall.
pub fn cut_animation(f: &mut Fighter, phase: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(phase.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.change_motion_state(
            (if f.physics.ground_or_air == GroundOrAir::Ground {
                S::Wait
            } else {
                S::Fall
            })
            .into(),
            phase.assets,
        )?;
    }
    Ok(None)
}

/// ftCo_CaptureCut_Phys: captured fighter's independent common friction multiplier.
pub fn cut_physics(f: &mut Fighter, phase: PhysicsPhase<'_>) {
    use crate::physics::{friction::friction_acceleration, grounded};
    if f.physics.ground_or_air != GroundOrAir::Ground {
        unimplemented!("ftCo_CaptureCut_Phys: airborne escape");
    }
    f.core.physics.ground_acceleration = friction_acceleration(
        f.physics.ground_velocity,
        phase.assets.grab_escape.escape_friction * f.attributes.ground.ground_friction,
    );
    let core = &mut f.core;
    grounded::apply_ground_movement(
        &mut core.physics,
        core.collision.data.floor.normal,
        phase.map.floor_speed_scale(&core.collision.data),
    );
    grounded::finish_ground_update(
        &mut core.physics,
        &core.collision.data,
        &grounded::GroundedParameters::from_attributes(&core.attributes, &phase.assets.common),
        phase.map,
        phase.wind,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Stick;

    fn parameters() -> Parameters {
        Parameters {
            stick_threshold: 0.5,
            base_timer: 30.0,
            handicap_scale: 8.0,
            handicap_origin: 9.0,
            rank_scale: 15.0,
            rank_origin: 4.0,
            percent_scale: 1.6,
            decrement: 1.0,
            mash_decrement: 6.0,
            fast_frames: 10.0,
            fast_rate: 2.0,
            escape_speed: 1.0,
            escape_friction: 1.0,
        }
    }

    #[test]
    fn grab_timer_uses_percent_handicap_and_tied_rank() {
        let p = parameters();
        assert_eq!(CaptureState::new(0.0, 9, 0, &p).timer, 75.0);
        assert_eq!(CaptureState::new(10.0, 8, 1, &p).timer, 84.0);
    }

    #[test]
    fn mash_counts_buttons_and_stick_separately_and_retains_direction_through_neutral() {
        let p = parameters();
        let mut capture = CaptureState::new(0.0, 9, 0, &p);
        let mut input = FighterInput {
            pressed: Buttons::A | Buttons::B,
            ..FighterInput::default()
        };
        input.current.stick = Stick { x: 0.8, y: -0.8 };
        assert!(capture.mash(&input, &p));
        assert_eq!(capture.timer, 63.0); // Two buttons and two axes still cost 6 + 6.
        input.pressed = Buttons::default();
        input.current.stick = Stick::default();
        assert!(!capture.mash(&input, &p));
        input.current.stick = Stick { x: 0.8, y: -0.8 };
        assert!(!capture.mash(&input, &p));
        input.current.stick.x = -0.5; // Strict crossing: equality does not count.
        assert!(!capture.mash(&input, &p));
        input.current.stick.x = -0.8;
        assert!(capture.mash(&input, &p));
        assert_eq!(capture.timer, 57.0);
    }
}
