//! Host input to controller samples. Hosts report logical keyboard actions
//! and gamepad states per port; each tick latches them into one
//! [`Inputs`]. A port without a human (a CPU, or no player) stays neutral.
use melee_lib::{Buttons, ControllerState, Inputs};

/// Native adapters map physical keys to these stable logical actions.
#[repr(u32)]
#[derive(Clone, Copy, Debug)]
pub enum Action {
    Left,
    Right,
    Up,
    Down,
    Attack,
    Special,
    Jump,
    Shield,
    Grab,
}
impl TryFrom<u32> for Action {
    type Error = ();
    fn try_from(value: u32) -> Result<Self, ()> {
        Ok(match value {
            0 => Self::Left,
            1 => Self::Right,
            2 => Self::Up,
            3 => Self::Down,
            4 => Self::Attack,
            5 => Self::Special,
            6 => Self::Jump,
            7 => Self::Shield,
            8 => Self::Grab,
            _ => return Err(()),
        })
    }
}

/// A gamepad as a GameCube controller after the host pad's mapping and
/// calibration ([`crate::pads`]): what retail's PADRead hands HSD, signed
/// origin-adjusted stick bytes and unsigned trigger bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Gamepad {
    /// [`PadButton`] bits.
    pub buttons: u32,
    /// Main stick bytes, up positive (HSD clamps them to its 80-unit circle).
    pub stick: [i8; 2],
    /// C-stick bytes, up positive.
    pub cstick: [i8; 2],
    /// Analog L and R (HSD clamps them to 140).
    pub triggers: [u8; 2],
}

/// Gamepad buttons in GameCube terms; bit `1 << value` of [`Gamepad::buttons`].
#[repr(u32)]
#[derive(Clone, Copy, Debug)]
pub enum PadButton {
    A,
    B,
    X,
    Y,
    Z,
    /// L clicked in (a full press).
    L,
    /// R clicked in.
    R,
    DpadLeft,
    DpadRight,
    DpadDown,
    DpadUp,
}
impl PadButton {
    pub const fn bit(self) -> u32 {
        1 << self as u32
    }
}

/// Retail analog trigger range (HSD's 140-unit full press).
pub const TRIGGER_UNITS: u8 = 140;

impl Gamepad {
    fn sample(&self) -> ControllerState {
        let mut buttons = 0;
        for (pad, button) in [
            (PadButton::A, Buttons::A),
            (PadButton::B, Buttons::B),
            (PadButton::X, Buttons::X),
            (PadButton::Y, Buttons::Y),
            (PadButton::Z, Buttons::Z),
            (PadButton::L, Buttons::L),
            (PadButton::R, Buttons::R),
        ] {
            if self.buttons & pad.bit() != 0 {
                buttons |= button.0;
            }
        }
        // HSD_Pad's d-pad bits: left, right, down, up (controller.h).
        for (index, pad) in [
            PadButton::DpadLeft,
            PadButton::DpadRight,
            PadButton::DpadDown,
            PadButton::DpadUp,
        ]
        .into_iter()
        .enumerate()
        {
            if self.buttons & pad.bit() != 0 {
                buttons |= 1 << index;
            }
        }
        // A clicked shoulder is a full analog press, as on the controller.
        let mut triggers = self.triggers;
        for (side, pad) in [PadButton::L, PadButton::R].into_iter().enumerate() {
            if self.buttons & pad.bit() != 0 {
                triggers[side] = TRIGGER_UNITS;
            }
        }
        ControllerState::from_origin_adjusted(Buttons(buttons), self.stick, self.cstick, triggers)
    }
}

/// Every port's keyboard actions and gamepad, latched once per tick.
#[derive(Default)]
pub struct PortInputs {
    held: [u32; 4],
    /// Actions pressed since the last tick, so a tap shorter than a tick
    /// still lands.
    pressed: [u32; 4],
    gamepads: [Option<Gamepad>; 4],
    /// Gamepad buttons pressed since the last tick.
    pad_pressed: [u32; 4],
}
impl PortInputs {
    pub fn set_action(&mut self, port: usize, action: Action, down: bool) {
        let bit = 1 << (action as u32);
        if down {
            if self.held[port] & bit == 0 {
                self.pressed[port] |= bit;
            }
            self.held[port] |= bit;
        } else {
            self.held[port] &= !bit;
        }
    }
    /// The port's gamepad state, or `None` when it is disconnected.
    pub fn set_gamepad(&mut self, port: usize, pad: Option<Gamepad>) {
        if let Some(pad) = pad {
            let before = self.gamepads[port].map_or(0, |p| p.buttons);
            self.pad_pressed[port] |= pad.buttons & !before;
        }
        self.gamepads[port] = pad;
    }
    pub fn sample(&mut self) -> Inputs {
        let mut inputs = Inputs::default();
        for (port, pad) in inputs.0.iter_mut().enumerate() {
            *pad = self.sample_port(port);
        }
        inputs
    }
    fn sample_port(&mut self, port: usize) -> ControllerState {
        let current = self.held[port];
        let tapped = current | std::mem::take(&mut self.pressed[port]);
        let active = |action: Action| current & (1 << (action as u32)) != 0;
        let x = (i8::from(active(Action::Right)) - i8::from(active(Action::Left))) * 80;
        let y = (i8::from(active(Action::Up)) - i8::from(active(Action::Down))) * 80;
        let mut buttons = 0;
        for (action, button) in [
            (Action::Attack, Buttons::A),
            (Action::Special, Buttons::B),
            (Action::Jump, Buttons::X),
            (Action::Shield, Buttons::L),
            (Action::Grab, Buttons::Z),
        ] {
            if tapped & (1 << (action as u32)) != 0 {
                buttons |= button.0;
            }
        }
        let keys_neutral = x == 0 && y == 0 && buttons == 0;
        let pad_taps = std::mem::take(&mut self.pad_pressed[port]);
        match self.gamepads[port] {
            // The keyboard leads while it is in use; otherwise the gamepad.
            Some(gamepad) if keys_neutral => Gamepad {
                buttons: gamepad.buttons | pad_taps,
                ..gamepad
            }
            .sample(),
            _ => ControllerState::from_origin_adjusted(Buttons(buttons), [x, y], [0, 0], [0, 0]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quick_button_tap_survives_until_one_simulation_tick() {
        let mut keys = PortInputs::default();
        keys.set_action(0, Action::Jump, true);
        keys.set_action(0, Action::Jump, false);
        assert_eq!(keys.sample().0[0].buttons, Buttons::X);
        assert_eq!(keys.sample().0[0].buttons, Buttons::default());
    }
    #[test]
    fn a_gamepad_drives_its_port_with_retail_stick_and_trigger_ranges() {
        let mut inputs = PortInputs::default();
        let pad = Gamepad {
            buttons: PadButton::A.bit() | PadButton::DpadUp.bit(),
            stick: [-40, 20],
            cstick: [0, 80],
            triggers: [70, 0],
        };
        inputs.set_gamepad(2, Some(pad));
        let sample = inputs.sample();
        let state = sample.0[2];
        assert_eq!(state.buttons, Buttons(Buttons::A.0 | 1 << 3));
        assert_eq!((state.stick.x, state.stick.y), (-0.5, 0.25));
        assert_eq!(state.cstick.y, 1.0);
        assert_eq!(state.left_trigger, 70.0 / 140.0);
        assert_eq!(
            sample.0[0],
            ControllerState::default(),
            "other ports neutral"
        );
        // A tap between ticks still lands; a held R is a full press.
        inputs.set_gamepad(2, Some(Gamepad::default()));
        inputs.set_gamepad(
            2,
            Some(Gamepad {
                buttons: PadButton::B.bit(),
                ..Gamepad::default()
            }),
        );
        inputs.set_gamepad(2, Some(Gamepad::default()));
        assert_eq!(inputs.sample().0[2].buttons, Buttons::B);
        inputs.set_gamepad(
            2,
            Some(Gamepad {
                buttons: PadButton::R.bit(),
                ..Gamepad::default()
            }),
        );
        let state = inputs.sample().0[2];
        assert_eq!((state.buttons, state.right_trigger), (Buttons::R, 1.0));
        // Unplugged: neutral again.
        inputs.set_gamepad(2, None);
        assert_eq!(inputs.sample().0[2], ControllerState::default());
    }
    #[test]
    fn sticks_past_the_rim_clamp_to_hsds_circle() {
        let mut inputs = PortInputs::default();
        inputs.set_gamepad(
            0,
            Some(Gamepad {
                stick: [127, 0],
                triggers: [255, 0],
                ..Gamepad::default()
            }),
        );
        let state = inputs.sample().0[0];
        assert_eq!((state.stick.x, state.left_trigger), (1.0, 1.0));
    }
}
