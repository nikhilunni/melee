//! Shared fall animation families and air-dodge special fall, ftCommon/ftCo_Fall*.c.
use super::FighterCore;
use super::{Fighter, MotionData};
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
impl Fighter {
    /// ftCo_80096900 (80096900), called by EscapeAir_Anim (80099BD0).
    /// ftCo_FallSpecial.c:34-57, EscapeAir.c:78-79: ordinary gravity,
    /// forced landing lag and no landing interrupt; consumes all air jumps.
    pub(super) fn enter_air_dodge_fall(
        &mut self,
        assets: &super::assets::FighterAssets,
    ) -> super::assets::Result<()> {
        self.enter_special_fall(
            assets,
            true,
            true,
            false,
            assets.air_dodge.special_fall_mobility,
            assets.air_dodge.landing_lag,
        )
    }

    /// ftCo_80096900, ordinary-gravity path shared by air dodge and SpecialN.
    pub fn enter_special_fall(
        &mut self,
        assets: &super::assets::FighterAssets,
        ordinary_gravity: bool,
        force_landing_lag: bool,
        allow_interrupt: bool,
        mobility: f32,
        landing_lag: f32,
    ) -> super::assets::Result<()> {
        self.change_motion_state(melee_types::CommonMotionState::FallSpecial.into(), assets)?;
        self.core.state_data = MotionData::FallSpecial(SpecialFallState {
            animation: FallState::new(FallFamily::Special),
            // retail 8009696C fmuls; no multiply-add.
            mobility: self.core.attributes.air.air_drift_max * mobility,
            ordinary_gravity,
            force_landing_lag,
            allow_interrupt,
            landing_lag,
        });
        if self.core.physics.ground_or_air == melee_types::GroundOrAir::Ground {
            // ftCo_FallSpecial.c:52-53: entered on the ground, the fighter
            // leaves it with its jumps spent (ftCommon_8007D60C).
            self.core.leave_ground_with_spent_jumps();
        } else {
            // ftCommon_UseAllJumps (ftCo_80096900's path).
            self.core.physics.jumps_used = self.core.attributes.jumping.max_jumps as u8;
        }
        Ok(())
    }

    /// ftCo_FallSpecial_Phys (80096B44), ftCo_FallSpecial.c:104-125.
    pub(super) fn special_fall_physics(&mut self, assets: &super::assets::FighterAssets) {
        let MotionData::FallSpecial(fall) = &self.core.state_data else {
            panic!("special fall scratch missing")
        };
        if fall.ordinary_gravity {
            self.airborne_physics(assets);
            return;
        }
        let mobility = fall.mobility;
        let previous_y = self.physics.self_velocity.y;
        self.apply_fall_gravity(assets);
        let core = &mut self.core;
        let air = &core.attributes.air;
        if !core.physics.fast_fall {
            core.physics.self_velocity.y =
                crate::physics::airborne::gravity(previous_y, air.gravity, air.fast_fall_velocity);
        }
        let x = core.input.current.stick.x;
        let acceleration = x * air.air_drift_stick_mul
            + if x > 0.0 {
                air.aerial_drift_base
            } else {
                -air.aerial_drift_base
            };
        let target = (x * air.air_drift_max).clamp(-mobility, mobility);
        core.physics.animation_velocity.x = crate::physics::airborne::drift_acceleration(
            core.physics.self_velocity.x,
            acceleration,
            target,
            air,
        );
    }

    /// ftCo_80096D28 (80096D28), ftCo_FallSpecial.c:170-180.
    pub(super) fn land_from_special_fall(
        &mut self,
        assets: &super::assets::FighterAssets,
    ) -> super::assets::Result<()> {
        let MotionData::FallSpecial(fall) = &self.core.state_data else {
            panic!("special fall scratch missing")
        };
        if fall.force_landing_lag || self.core.physics.self_velocity.y < assets.soft_landing_speed {
            self.enter_special_landing(assets, fall.allow_interrupt, fall.landing_lag)
        } else {
            self.land();
            self.change_motion_state(melee_types::CommonMotionState::Wait.into(), assets)
        }
    }
}
impl FighterCore {
    /// ftCommon_8007D60C (8007D60C): specials leaving support spend every
    /// jump and lock the ECB for five frames. Unlike an ordinary fall, this
    /// preserves position Z and shield knockback. The player statistic has no
    /// simulation observer.
    pub fn leave_ground_with_spent_jumps(&mut self) {
        self.physics.ground_or_air = melee_types::GroundOrAir::Air;
        self.physics.ground_velocity = 0.0;
        self.physics.animation_velocity.y = 0.0;
        self.physics.jumps_used = self.attributes.jumping.max_jumps as u8;
        self.collision.lock_frames = 5;
        self.collision.data.x130_flags |= melee_types::mp::coll_data_x130::LOCKED;
    }
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
}

/// Item-free predicates from ftCo_Fall_IASA_Inner (0x800CCAAC).
/// DamageFall (80090828) omits air dodge; ordinary aerial states permit it.
/// Jump/JumpAerial callers gate their character float hooks by motion state.
#[inline(always)]
pub fn iasa(
    input: &crate::input::FighterInput,
    common: &crate::input::InputCommonData,
    jumps_used: u8,
    max_jumps: i32,
    allow_air_dodge: bool,
    check_float: impl FnMut(super::FloatInputPhase) -> bool,
) -> crate::input::WaitTransition {
    let jump = i32::from(jumps_used) < max_jumps
        && (input.pressed.intersects(crate::input::Buttons::XY)
            || (input.current.stick.y >= common.thresholds.tap_jump_threshold
                && i32::from(input.vertical.tilt) < common.thresholds.tap_jump_window));
    let special = input.pressed.intersects(crate::input::Buttons::B);
    iasa_with_jump(input, common, special, jump, allow_air_dodge, check_float)
}

/// The multijump path changes only the jump predicate; aerial action priority
/// and the float hooks retain the same order as ftCo_Fall_IASA_Inner.
pub(super) fn iasa_with_jump(
    input: &crate::input::FighterInput,
    common: &crate::input::InputCommonData,
    // ftCo_SpecialAir_CheckInput succeeds (Fighter::air_special_pressed).
    special: bool,
    jump: bool,
    allow_air_dodge: bool,
    mut check_float: impl FnMut(super::FloatInputPhase) -> bool,
) -> crate::input::WaitTransition {
    use crate::input::{Buttons, WaitTransition as T};
    if special {
        return T::AirSpecial;
    }
    if allow_air_dodge && input.pressed.intersects(Buttons::DIGITAL_SHOULDERS) {
        return T::Escape;
    }
    if super::attack::aerial::requested(input, common) {
        return T::Attack;
    }
    if check_float(super::FloatInputPhase::BeforeAerialJump) {
        return T::Float;
    }
    if jump {
        return T::Jump;
    }
    if check_float(super::FloatInputPhase::AfterAerialJump) {
        return T::Float;
    }
    T::None
}
