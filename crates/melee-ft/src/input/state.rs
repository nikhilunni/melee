use super::pad::{Buttons, Stick};

/// One of the three histories: current (index 0), previous (1), saved (2).
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct InputFrame {
    /// input.lstick[0..3], Fighter +620/+628/+630.
    pub stick: Stick,
    /// input.cstick[0..3], Fighter +638/+640/+648.
    pub cstick: Stick,
    /// input.triggers[0..3], Fighter +650/+654/+658.
    pub trigger: f32,
    /// input.held_buttons[0..3], Fighter +65C/+660/+664.
    pub held: Buttons,
}

/// Timers grouped by axis rather than the interleaved C layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnalogTimers {
    /// x670_timer_lstick_tilt_x / x671_timer_lstick_tilt_y /
    /// trigger_analog_timer (+670/+671/+672).
    pub tilt: u8,
    /// x673 / x674 / x675 (+673/+674/+675). The header's +674 for x675
    /// is stale; retail uses +675 (e.g. 0x8006B4D0).
    pub held: u8,
    /// x676_x / x677_y / x678 (+676/+677/+678).
    pub since_crossing: u8,
    /// x679_x / x67A_y / x67B (+679/+67A/+67B); joystick-count consumers.
    pub activity: u8,
}
impl Default for AnalogTimers {
    fn default() -> Self {
        Self {
            tilt: 0xFE,
            held: 0xFE,
            since_crossing: 0xFE,
            activity: 0xFE,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ButtonTimers {
    /// x67C (+67C), frames since A edge.
    pub attack: u8,
    /// x67D (+67D), frames since B edge.
    pub special: u8,
    /// x67E (+67E), frames since X/Y edge.
    pub jump_button: u8,
    /// x67F (+67F), frames since synthesized LR edge.
    pub shield: u8,
    /// x680 (+680), frames since digital L/R edge.
    pub digital_shield: u8,
    /// x681 (+681), frames since D-pad up edge.
    pub taunt: u8,
    /// x682 (+682), frames since D-pad down edge.
    pub down: u8,
    /// x683 (+683), old x67C on A press.
    pub previous_attack: u8,
    /// x684 (+684), old x680 on digital L/R press.
    pub previous_digital_shield: u8,
    /// x685 (+685), frames since ftCo_Jump_GetInput.
    pub jump: u8,
    /// x686 (+686), frames since up-special input.
    pub special_up: u8,
    /// x687 (+687), frames since down-special input.
    pub special_down: u8,
    /// x688 (+688), frames since side-special input.
    pub special_side: u8,
    /// x689 (+689), frames since neutral-special input.
    pub special_neutral: u8,
    /// x68A (+68A), old x685 on jump input.
    pub previous_jump: u8,
    /// x68B (+68B), old x686 on up-special input.
    pub previous_special_up: u8,
}
impl Default for ButtonTimers {
    fn default() -> Self {
        Self {
            attack: 255,
            special: 255,
            jump_button: 255,
            shield: 255,
            digital_shield: 255,
            taunt: 255,
            down: 255,
            previous_attack: 255,
            previous_digital_shield: 255,
            jump: 255,
            special_up: 255,
            special_down: 255,
            special_side: 255,
            special_neutral: 255,
            previous_jump: 255,
            previous_special_up: 255,
        }
    }
}

/// Fighter input state, initialized like Fighter_ResetInputData_80068854.
/// Saved history is explicitly zero at construction; reset preserves it,
/// matching Fighter_UnkInitLoad_80068914_Inner1.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct FighterInput {
    pub current: InputFrame,
    pub previous: InputFrame,
    pub saved: InputFrame,
    /// input.pressed_buttons (+668).
    pub pressed: Buttons,
    /// input.released_buttons (+66C).
    pub released: Buttons,
    pub horizontal: AnalogTimers,
    pub vertical: AnalogTimers,
    pub shoulder: AnalogTimers,
    pub buttons: ButtonTimers,
    /// x221D_b3: next previous sample comes from current, rather than saved.
    pub use_current_history: bool,
    /// x2228_b7: last horizontal smash direction was positive.
    pub last_horizontal_positive: bool,
    /// x2229_b0: last vertical smash direction was negative.
    pub last_vertical_negative: bool,
    /// Not Fighter state: the port's controller-fix setting and hardware
    /// pad-queue bytes, refreshed by the scene each input proc.
    pub hardware: super::controller_fix::HardwareInput,
}
impl FighterInput {
    /// fighter.c:655-681. Does not touch saved history or direction flags.
    pub fn clear_current_and_buffers(&mut self) {
        self.current = InputFrame::default();
        self.previous = InputFrame::default();
        self.pressed = Buttons::default();
        self.released = Buttons::default();
        self.horizontal = AnalogTimers::default();
        self.vertical = AnalogTimers::default();
        self.shoulder = AnalogTimers::default();
        self.buttons = ButtonTimers::default();
    }
}
