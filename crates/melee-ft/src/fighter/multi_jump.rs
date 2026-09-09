//! Kirby/Jigglypuff shared multijumps: ftCommon/ftCo_JumpAerialF1.c.
use super::{
    assets::{FighterAssets, Result},
    ActionId, CharacterCallbacks, Fighter, MotionData,
};
use crate::input::Buttons;
use hsd_types::Vec3;
use melee_types::CommonMotionState;

/// Fighter_x2D0_t (ft/types.h): an alias into either character's ext_attr.
#[derive(Clone, Debug, PartialEq)]
pub struct MultiJumpAttributes {
    /// +00: signed turning countdown, loaded by lwz in ftCo_800D74A4.
    pub turn_frames: i32,
    /// +04: backward stick threshold (strictly less than its negation).
    pub reverse_threshold: f32,
    /// +08: horizontal launch impulse per unit stick.
    pub horizontal_impulse: f32,
    /// +0C/+10: aerial drift acceleration and maximum speed multipliers.
    pub acceleration_multiplier: f32,
    pub speed_multiplier: f32,
    /// +14..24: vertical impulses for JumpAerialF1..F5.
    pub vertical_impulses: [f32; 5],
    /// +28: number of consecutive action states in either family.
    pub state_count: i32,
    /// +2C/+30: ordinary/helmet first action IDs; -1 disables a family.
    pub first_actions: [i32; 2],
}
impl MultiJumpAttributes {
    /// ftCo_800D72A0 (800D72A0): both ordinary and helmet state ranges.
    pub fn contains_action(&self, action: i32) -> bool {
        self.first_actions
            .iter()
            .any(|&first| first != -1 && first <= action && action < first + self.state_count)
    }
}

/// Shared mv.co.jumpaerial scratch, not character-owned persistent state.
#[derive(Clone, Debug)]
pub struct MultiJumpState {
    /// +2340: remaining ticks of the backward turn.
    pub turn_remaining: i32,
    /// +2344: unchanged union word inherited by landing's drop timer.
    pub retained_drop_timer: f32,
}
impl<C: CharacterCallbacks> Fighter<C> {
    /// ft_did_jump (800CB804), ftCo_800D730C (800D730C).
    pub(super) fn aerial_jump_requested(&self, assets: &FighterAssets) -> bool {
        if i32::from(self.physics.jumps_used) >= self.attributes.jumping.max_jumps {
            return false;
        }
        let threshold = assets.input.thresholds.tap_jump_threshold;
        if let Some(attributes) = self.character.multi_jump_attributes() {
            if self.physics.jumps_used != 1 {
                // Later jumps accept held input once the current script opens
                // its jump window. Falling states do not require that command.
                return (!attributes.contains_action(i32::from(self.motion_state.action))
                    || self.commands.variables[0] != 0)
                    && (self.input.current.stick.y >= threshold
                        || self.input.current.held.intersects(Buttons::XY));
            }
        }
        self.input.pressed.intersects(Buttons::XY)
            || (self.input.current.stick.y >= threshold
                && i32::from(self.input.vertical.tilt) < assets.input.thresholds.tap_jump_window)
    }

    /// ftCo_800D74A4 (800D74A4) -> ftCo_800CBAC4 (800CBAC4).
    pub(super) fn enter_multi_jump(&mut self, assets: &FighterAssets) -> Result<()> {
        if self.physics.jumps_used == 1 {
            self.leave_ground();
        }
        let attributes = self
            .character
            .multi_jump_attributes()
            .expect("multijump attributes");
        let index = usize::from(
            self.physics
                .jumps_used
                .checked_sub(1)
                .expect("airborne jump count"),
        );
        assert!(index < attributes.state_count as usize);
        let first = attributes.first_actions[self.character.multi_jump_family()];
        assert!(first >= 0, "disabled multijump family");
        let action = ActionId(u16::try_from(first + index as i32).expect("multijump action"));
        // retail 800D74EC fmuls; vertical impulses are loaded directly.
        let velocity = Vec3::new(
            self.input.current.stick.x * attributes.horizontal_impulse,
            attributes.vertical_impulses[index],
            0.0,
        );
        let turn_frames = attributes.turn_frames;
        let threshold = attributes.reverse_threshold;
        let retained_drop_timer = self.retained_drop_timer();
        self.commands.variables[0] = 0;
        self.change_motion_state_with_rate(action, assets, 0.0, 1.0)?;
        self.physics.self_velocity = velocity;
        // arg3=false: unlike basic double jumps, keep the vertical tilt age.
        self.physics.jumps_used += 1;
        // retail 800D7554 fmuls; turn facing halfway through, not at entry.
        self.state_data = MotionData::MultiJump(MultiJumpState {
            turn_remaining: if self.input.current.stick.x * self.physics.facing < -threshold {
                turn_frames
            } else {
                0
            },
            retained_drop_timer,
        });
        self.multi_jump_turn();
        Ok(())
    }

    /// ft_800CB6EC (800CB6EC): model turning also shared with Yoshi/Ness.
    fn multi_jump_turn(&mut self) {
        let MotionData::MultiJump(jump) = &mut self.state_data else {
            panic!("multijump scratch")
        };
        if jump.turn_remaining == 0 {
            return;
        }
        jump.turn_remaining -= 1;
        let frames = self.character.multi_jump_attributes().unwrap().turn_frames;
        let root = self.animation.root;
        let old = self.skeleton.get(root).rotate.y;
        // retail 800CB768 fdivs, 800CB76C fnmsubs; @197 = float PI/180.
        let angle = gekko_math::fma::fnmsubs(0.017453292, 180.0 / frames as f32, old);
        self.skeleton.set_rotation_y(root, angle);
        if jump.turn_remaining == frames / 2 {
            self.physics.facing = -self.physics.facing;
        }
    }

    /// ftCo_JumpAerialF1_Anim (800D7590): Fall until every jump is consumed.
    pub(super) fn multi_jump_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        self.multi_jump_turn();
        if !self.animation.frames_remaining(&self.skeleton) {
            let state = if i32::from(self.physics.jumps_used) >= self.attributes.jumping.max_jumps {
                CommonMotionState::FallAerial
            } else {
                CommonMotionState::Fall
            };
            self.change_motion_state(state, assets)?;
        }
        Ok(())
    }

    /// ftCo_JumpAerialF1_Phys (800D7634), ft_80084E1C (80084E1C).
    pub(super) fn multi_jump_physics(&mut self, assets: &FighterAssets) {
        self.apply_fall_gravity(assets);
        let attributes = self.character.multi_jump_attributes().unwrap();
        let air = &self.attributes.air;
        // retail 800D765C/64 and 80084EA8/AC: separate fmuls, no fusion.
        let acceleration = air.air_drift_stick_mul * attributes.acceleration_multiplier;
        let maximum = air.air_drift_max * attributes.speed_multiplier;
        let stick = self.input.current.stick.x;
        let (acceleration, target) = if stick.abs() >= assets.jumping.multi_jump_drift_threshold {
            (stick * acceleration, stick * maximum)
        } else {
            (0.0, 0.0)
        };
        self.physics.animation_velocity.x = crate::physics::airborne::drift_acceleration(
            self.physics.self_velocity.x,
            acceleration,
            target,
            air,
        );
    }
}
