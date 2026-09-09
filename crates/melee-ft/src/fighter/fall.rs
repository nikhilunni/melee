//! Shared fall animation families and air-dodge special fall, ftCommon/ftCo_Fall*.c.
use super::{CharacterCallbacks, Fighter, MotionData};
/// PlCo ftCommonData +444/+448: normalized speed deadzone and blend smoothing.
#[derive(Clone, Copy, Debug)]
pub struct FallParameters {
    pub deadzone: f32,
    pub smoothing: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FallFamily {
    Ordinary,
    Aerial,
    Special,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FallPose {
    Neutral,
    Forward,
    Backward,
}
impl FallFamily {
    /// ftCommon/forward.h: submotion IDs, distinct from action states 29..37.
    pub fn motion(self, pose: FallPose) -> i32 {
        let neutral = match self {
            Self::Ordinary => 20,
            Self::Aerial => 23,
            Self::Special => 26,
        };
        neutral
            + match pose {
                FallPose::Neutral => 0,
                FallPose::Forward => 1,
                FallPose::Backward => 2,
            }
    }
}
/// mv.co.fall/fallaerial: selected secondary submotion and smoothed weight.
#[derive(Clone, Debug)]
pub struct FallState {
    pub family: FallFamily,
    pub pose: FallPose,
    pub blend: f32,
}
impl FallState {
    pub fn new(family: FallFamily) -> Self {
        Self {
            family,
            pose: FallPose::Neutral,
            blend: 0.0,
        }
    }
    /// ftCo_Fall_Anim_Inner (800CCBE0), ftCo_Fall.c:157-201.
    pub fn advance(
        &mut self,
        velocity: f32,
        maximum: f32,
        facing: f32,
        p: FallParameters,
    ) -> FallPose {
        let fraction = (velocity / maximum).clamp(-1.0, 1.0);
        let magnitude = gekko_math::msl::fabsf(fraction);
        let (pose, target) = if magnitude > p.deadzone {
            let pose = if fraction * facing > 0.0 {
                FallPose::Forward
            } else {
                FallPose::Backward
            };
            (pose, (magnitude - p.deadzone) / (1.0 - p.deadzone))
        } else {
            (FallPose::Neutral, 0.0)
        };
        // retail 800CCCA0 fsubs, 800CCCA4 fmadds: round the difference first.
        self.blend = gekko_math::fma::fmadds(p.smoothing, target - self.blend, self.blend);
        pose
    }
}
/// mv.co.fallspecial; caller-selected physics and landing behavior.
#[derive(Clone, Debug)]
pub struct SpecialFallState {
    pub animation: FallState,
    pub mobility: f32,
    pub ordinary_gravity: bool,
    pub force_landing_lag: bool,
    pub allow_interrupt: bool,
    pub landing_lag: f32,
}
impl<C: CharacterCallbacks> Fighter<C> {
    /// ftCommon_8007D5D4 (0x8007D5D4), ftcommon.c:515-525.
    pub fn leave_ground(&mut self) {
        self.physics.ground_or_air = melee_types::GroundOrAir::Air;
        self.physics.ground_velocity = 0.0;
        self.physics.shield_knockback_velocity.z = 0.0;
        self.physics.position.z = 0.0;
        self.physics.animation_velocity.y = 0.0;
        self.physics.jumps_used = 1;
        self.collision.lock_frames = 10;
        self.collision.data.x130_flags |= melee_types::mp::coll_data_x130::LOCKED;
    }

    /// ftCo_Fall_Anim (800CCA00), FallAerial_Anim (800CCDFC),
    /// FallSpecial_Anim (80096AA0): secondary animation, never a state change.
    pub(super) fn fall_animation(
        &mut self,
        assets: &super::assets::FighterAssets,
    ) -> super::assets::Result<()> {
        let fall = match &mut self.state_data {
            MotionData::Fall(fall) => fall,
            MotionData::FallSpecial(fall) => &mut fall.animation,
            _ => panic!("fall scratch missing"),
        };
        let pose = fall.advance(
            self.physics.self_velocity.x,
            self.attributes.air.air_drift_max,
            self.physics.facing,
            assets.falling,
        );
        if fall.blend != 0.0 && pose != fall.pose {
            self.animation.set_secondary_animation(
                &assets.motions[&fall.family.motion(pose)],
                self.animation.frame,
                1.0,
            )?;
            // ftCo_Fall_Anim_Inner evaluates and blends immediately on a switch.
            self.animation
                .apply_fall_pose::<super::RetailTrig>(&mut self.skeleton, fall.blend);
            fall.pose = pose;
        }
        // ftCo_800CC988 (800CC988) evaluates again, even on the switch tick.
        if fall.blend != 0.0 {
            self.animation
                .apply_fall_pose::<super::RetailTrig>(&mut self.skeleton, fall.blend);
        }
        Ok(())
    }

    /// ftCo_80096900 (80096900), called by EscapeAir_Anim (80099BD0).
    /// ftCo_FallSpecial.c:34-57, EscapeAir.c:78-79: ordinary gravity,
    /// forced landing lag and no landing interrupt; consumes all air jumps.
    pub(super) fn enter_air_dodge_fall(
        &mut self,
        assets: &super::assets::FighterAssets,
    ) -> super::assets::Result<()> {
        self.change_motion_state(melee_types::CommonMotionState::FallSpecial, assets)?;
        self.state_data = MotionData::FallSpecial(SpecialFallState {
            animation: FallState::new(FallFamily::Special),
            // retail 8009696C fmuls; no multiply-add.
            mobility: self.attributes.air.air_drift_max * assets.air_dodge.special_fall_mobility,
            ordinary_gravity: true,
            force_landing_lag: true,
            allow_interrupt: false,
            landing_lag: assets.air_dodge.landing_lag,
        });
        if self.physics.ground_or_air == melee_types::GroundOrAir::Ground {
            unimplemented!("ftCo_FallSpecial.c:52-53: grounded special-fall entry");
        }
        self.physics.jumps_used = self.attributes.jumping.max_jumps as u8;
        Ok(())
    }

    /// ftCo_FallSpecial_Phys (80096B44), ftCo_FallSpecial.c:104-125.
    pub(super) fn special_fall_physics(&mut self, assets: &super::assets::FighterAssets) {
        let MotionData::FallSpecial(fall) = &self.state_data else {
            panic!("special fall scratch missing")
        };
        if !fall.ordinary_gravity {
            unimplemented!("ftCo_FallSpecial.c:126-149: special-move gravity and mobility cap");
        }
        // 80096BAC fmuls, 80096BC8 fadds, 80096BD4 fmuls are separate;
        // 8007D140 forwards to the same drift clamp used by ordinary air physics.
        // Retail xC != 0 does not apply the saved mobility cap.
        self.airborne_physics(assets);
    }

    /// ftCo_80096D28 (80096D28), ftCo_FallSpecial.c:170-180.
    pub(super) fn land_from_special_fall(
        &mut self,
        assets: &super::assets::FighterAssets,
    ) -> super::assets::Result<()> {
        let MotionData::FallSpecial(fall) = &self.state_data else {
            panic!("special fall scratch missing")
        };
        if fall.force_landing_lag || self.physics.self_velocity.y < assets.soft_landing_speed {
            self.enter_special_landing(assets, fall.allow_interrupt, fall.landing_lag)
        } else {
            self.land();
            self.change_motion_state(melee_types::CommonMotionState::Wait, assets)
        }
    }
}

/// Item-free predicates from ftCo_Fall_IASA_Inner (0x800CCAAC).
/// Jump/JumpAerial callers gate their character float hooks by motion state.
pub fn iasa(
    input: &crate::input::FighterInput,
    common: &crate::input::InputCommonData,
    jumps_used: u8,
    max_jumps: i32,
    check_float: impl FnMut(super::FloatInputPhase),
) -> crate::input::WaitTransition {
    let jump = i32::from(jumps_used) < max_jumps
        && (input.pressed.intersects(crate::input::Buttons::XY)
            || (input.current.stick.y >= common.thresholds.tap_jump_threshold
                && i32::from(input.vertical.tilt) < common.thresholds.tap_jump_window));
    iasa_with_jump(input, jump, check_float)
}

/// The multijump path changes only the jump predicate; aerial action priority
/// and the float hooks retain the same order as ftCo_Fall_IASA_Inner.
pub(super) fn iasa_with_jump(
    input: &crate::input::FighterInput,
    jump: bool,
    mut check_float: impl FnMut(super::FloatInputPhase),
) -> crate::input::WaitTransition {
    use crate::input::{Buttons, WaitTransition as T};
    if input.pressed.intersects(Buttons::B) {
        return T::Special;
    }
    if input.pressed.intersects(Buttons::DIGITAL_SHOULDERS) {
        return T::Escape;
    }
    if input.pressed.intersects(Buttons::A) {
        return T::Attack;
    }
    assert_eq!(
        input.current.cstick,
        crate::input::Stick::default(),
        "C-stick aerial selection needs ftCo_800DF478"
    );
    check_float(super::FloatInputPhase::BeforeAerialJump);
    if jump {
        return T::Jump;
    }
    check_float(super::FloatInputPhase::AfterAerialJump);
    T::None
}
