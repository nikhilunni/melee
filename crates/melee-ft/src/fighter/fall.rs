//! Neutral Fall callbacks, ftCommon/ftCo_Fall.c:106-211.
use super::{CharacterCallbacks, Fighter, MotionData};
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

    /// ftCo_Fall_Anim (0x800CCA00), 800CCBE0 blend selection.
    /// With neutral horizontal velocity the target blend is zero.
    pub(super) fn fall_animation(&mut self) {
        assert_eq!(
            self.physics.self_velocity.x, 0.0,
            "directional Fall animation needs FallF/B"
        );
        let MotionData::Fall { blend } = self.state_data else {
            panic!("Fall data missing")
        };
        assert_eq!(blend, 0.0, "directional Fall blend not implemented");
        // The checked fmadds at 0x800CCCA4 stays +0 for this neutral path.
    }
}

/// Item-free Fox predicates reached from ftCo_Fall_IASA_Inner (0x800CCAAC).
/// Transition bodies remain owned by the future aerial action states.
pub fn iasa(
    input: &crate::input::FighterInput,
    common: &crate::input::InputCommonData,
    jumps_used: u8,
    max_jumps: i32,
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
    if i32::from(jumps_used) < max_jumps
        && (input.pressed.intersects(Buttons::XY)
            || (input.current.stick.y >= common.thresholds.tap_jump_threshold
                && i32::from(input.vertical.tilt) < common.thresholds.tap_jump_window))
    {
        return T::Jump;
    }
    T::None
}
