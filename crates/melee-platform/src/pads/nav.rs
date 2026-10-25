//! A pad in the menus: directions from the d-pad or the stick, repeating
//! while held as Melee's menus do, and button presses as edges.

use crate::input::{Gamepad, PadButton};

/// Menu events of one poll: bits of a `u32` (`melee_menu_event_e`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MenuEvents(pub u32);

impl MenuEvents {
    pub const UP: u32 = 1 << 0;
    pub const DOWN: u32 = 1 << 1;
    pub const LEFT: u32 = 1 << 2;
    pub const RIGHT: u32 = 1 << 3;
    /// GameCube A: confirm.
    pub const CONFIRM: u32 = 1 << 4;
    /// GameCube B: back out.
    pub const BACK: u32 = 1 << 5;
    pub const X: u32 = 1 << 6;
    pub const Y: u32 = 1 << 7;
    pub const L: u32 = 1 << 8;
    pub const R: u32 = 1 << 9;
    pub const Z: u32 = 1 << 10;
    pub const START: u32 = 1 << 11;
    /// The pad's Select or Home button: the app menu.
    pub const MENU: u32 = 1 << 12;
    /// This press joined the pad to a player (character select).
    pub const JOINED: u32 = 1 << 16;
    /// A rebind finished (or timed out).
    pub const BOUND: u32 = 1 << 17;
    pub const DIRECTIONS: u32 = Self::UP | Self::DOWN | Self::LEFT | Self::RIGHT;

    pub fn has(self, bits: u32) -> bool {
        self.0 & bits != 0
    }
    /// Any button press (not a direction or a notification).
    pub fn pressed_button(self) -> bool {
        self.0 & (Self::CONFIRM | Self::BACK | Self::X | Self::Y | Self::L | Self::R | Self::Z | Self::START | Self::MENU)
            != 0
    }
}

/// A mapped pad as the menus see it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MenuPad {
    pub pad: Gamepad,
    pub start: bool,
    pub menu: bool,
}

/// First repeat after this long held, then one per [`REPEAT_MS`].
pub const REPEAT_DELAY_MS: f64 = 400.0;
pub const REPEAT_MS: f64 = 90.0;
/// The stick points somewhere past this many units (of the 87-unit notch)
/// and lets go below [`STICK_RELEASE`].
const STICK_PRESS: i32 = 48;
const STICK_RELEASE: i32 = 32;
/// Analog shoulders count as pressed past half of retail's 140.
const SHOULDER_PRESS: u8 = 70;

#[derive(Clone, Copy, Debug, Default)]
pub struct Navigator {
    direction: u32,
    stick: u32,
    repeat_at: f64,
    buttons: u32,
}

impl Navigator {
    /// Forget held state (after a rebind or a screen change of ownership):
    /// whatever is held now does not fire until released.
    pub fn hold(&mut self, now: &MenuPad) {
        self.buttons = Self::buttons(now);
        self.direction = u32::MAX;
        self.stick = 0;
    }

    fn buttons(state: &MenuPad) -> u32 {
        let pad = &state.pad;
        let held = |b: PadButton| pad.buttons & b.bit() != 0;
        let mut bits = 0;
        for (on, bit) in [
            (held(PadButton::A), MenuEvents::CONFIRM),
            (held(PadButton::B), MenuEvents::BACK),
            (held(PadButton::X), MenuEvents::X),
            (held(PadButton::Y), MenuEvents::Y),
            (held(PadButton::Z), MenuEvents::Z),
            (held(PadButton::L) || pad.triggers[0] >= SHOULDER_PRESS, MenuEvents::L),
            (held(PadButton::R) || pad.triggers[1] >= SHOULDER_PRESS, MenuEvents::R),
            (state.start, MenuEvents::START),
            (state.menu, MenuEvents::MENU),
        ] {
            if on {
                bits |= bit;
            }
        }
        bits
    }

    /// The stick's direction, with hysteresis on the dominant axis.
    fn stick_direction(&mut self, [x, y]: [i8; 2]) -> u32 {
        let (x, y) = (i32::from(x), i32::from(y));
        let along = |dir: u32| match dir {
            MenuEvents::UP => y,
            MenuEvents::DOWN => -y,
            MenuEvents::LEFT => -x,
            MenuEvents::RIGHT => x,
            _ => 0,
        };
        if self.stick != 0 && along(self.stick) >= STICK_RELEASE {
            // Still held; a turn to another axis past the press point moves on.
            let dominant = if x.abs() > y.abs() {
                if x > 0 { MenuEvents::RIGHT } else { MenuEvents::LEFT }
            } else if y > 0 {
                MenuEvents::UP
            } else {
                MenuEvents::DOWN
            };
            if dominant != self.stick && along(dominant) >= STICK_PRESS {
                self.stick = dominant;
            }
            return self.stick;
        }
        self.stick = 0;
        if x.abs().max(y.abs()) >= STICK_PRESS {
            self.stick = if x.abs() > y.abs() {
                if x > 0 { MenuEvents::RIGHT } else { MenuEvents::LEFT }
            } else if y > 0 {
                MenuEvents::UP
            } else {
                MenuEvents::DOWN
            };
        }
        self.stick
    }

    pub fn step(&mut self, state: &MenuPad, now_ms: f64) -> MenuEvents {
        let buttons = Self::buttons(state);
        let mut events = buttons & !self.buttons;
        self.buttons = buttons;

        let pad = &state.pad;
        let dpad = [
            (PadButton::DpadUp, MenuEvents::UP),
            (PadButton::DpadDown, MenuEvents::DOWN),
            (PadButton::DpadLeft, MenuEvents::LEFT),
            (PadButton::DpadRight, MenuEvents::RIGHT),
        ]
        .into_iter()
        .find(|(b, _)| pad.buttons & b.bit() != 0)
        .map_or(0, |(_, dir)| dir);
        let stick = self.stick_direction(pad.stick);
        let direction = if dpad != 0 { dpad } else { stick };
        if direction == 0 {
            self.direction = 0;
        } else if self.direction == u32::MAX {
            // Held through a hold(): wait for a release.
        } else if direction != self.direction {
            self.direction = direction;
            self.repeat_at = now_ms + REPEAT_DELAY_MS;
            events |= direction;
        } else if now_ms >= self.repeat_at {
            self.repeat_at += REPEAT_MS;
            if self.repeat_at < now_ms {
                // A slow poll fires once, not a burst.
                self.repeat_at = now_ms + REPEAT_MS;
            }
            events |= direction;
        }
        MenuEvents(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pad(buttons: u32, stick: [i8; 2]) -> MenuPad {
        MenuPad {
            pad: Gamepad {
                buttons,
                stick,
                ..Gamepad::default()
            },
            ..MenuPad::default()
        }
    }

    #[test]
    fn a_held_direction_repeats_after_a_delay() {
        let mut nav = Navigator::default();
        let right = pad(PadButton::DpadRight.bit(), [0, 0]);
        let mut fired = vec![];
        for frame in 0..60 {
            let now = f64::from(frame) * 1000.0 / 60.0;
            if nav.step(&right, now).has(MenuEvents::RIGHT) {
                fired.push(frame);
            }
        }
        // Immediately, then at 400 ms (frame 24), then every 90 ms.
        assert_eq!(fired, [0, 24, 30, 35, 41, 46, 51, 57]);
        assert_eq!(nav.step(&pad(0, [0, 0]), 1000.0), MenuEvents(0));
    }

    #[test]
    fn the_stick_navigates_with_hysteresis() {
        let mut nav = Navigator::default();
        assert_eq!(nav.step(&pad(0, [40, 0]), 0.0), MenuEvents(0));
        assert_eq!(nav.step(&pad(0, [60, 10]), 16.0), MenuEvents(MenuEvents::RIGHT));
        // Easing back to 35 still holds right: no second press.
        assert_eq!(nav.step(&pad(0, [35, 0]), 32.0), MenuEvents(0));
        assert_eq!(nav.step(&pad(0, [20, 0]), 48.0), MenuEvents(0));
        assert_eq!(nav.step(&pad(0, [-10, -70]), 64.0), MenuEvents(MenuEvents::DOWN));
    }

    #[test]
    fn buttons_fire_once_per_press() {
        let mut nav = Navigator::default();
        let a = pad(PadButton::A.bit(), [0, 0]);
        assert_eq!(nav.step(&a, 0.0), MenuEvents(MenuEvents::CONFIRM));
        assert_eq!(nav.step(&a, 16.0), MenuEvents(0));
        let mut start = pad(0, [0, 0]);
        start.start = true;
        start.pad.triggers = [100, 0];
        assert_eq!(nav.step(&start, 32.0), MenuEvents(MenuEvents::START | MenuEvents::L));
        // After hold(), the held buttons and direction wait for a release.
        let both = pad(PadButton::A.bit() | PadButton::DpadUp.bit(), [0, 0]);
        nav.hold(&both);
        assert_eq!(nav.step(&both, 1000.0), MenuEvents(0));
        assert_eq!(nav.step(&pad(0, [0, 0]), 1016.0), MenuEvents(0));
        assert_eq!(nav.step(&both, 1032.0), MenuEvents(MenuEvents::CONFIRM | MenuEvents::UP));
    }
}
