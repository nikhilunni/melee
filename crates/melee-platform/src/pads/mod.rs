//! Gamepads that just work: every host reads its pads in one layout (the
//! W3C standard gamepad's, [`RawPad`]) and the core does the rest, so the
//! macOS and web apps behave the same:
//!
//! - [`family`]: which controller it is (Switch Pro, Joy-Con, DualSense,
//!   DualShock 4, Xbox, 8BitDo, GameCube through an adapter, or generic);
//! - [`mapping`]: its controls onto the GameCube controller's by position,
//!   with per-pad bindings, deadzone and trigger click point that the host
//!   saves ([`Pads::prefs`]) keyed by the controller's identity;
//! - [`calibration`]: modern sticks through the GameCube's octagonal gate,
//!   so the bytes reach the simulation as a GameCube stick's would;
//! - [`nav`]: menu events with Melee-like repeat;
//! - [`adapter`]: the Wii U GameCube adapter's reports, one to one.
//!
//! Which player a pad drives is its port, set by pressing a button on
//! character select ("press a button to join", [`crate::app::App::pad_update`]).

pub mod adapter;
pub mod calibration;
pub mod family;
pub mod mapping;
pub mod nav;

use crate::input::{Gamepad, PadButton};
use std::collections::BTreeMap;

pub use family::Family;
pub use mapping::{Input, Settings, Target, INPUT_COUNT};
pub use nav::{MenuEvents, MenuPad};

/// A pad's controls as the host reads them, in the W3C standard layout.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RawPad {
    /// Each [`Input`]'s value, 0..1 (triggers analog, the rest 0 or 1).
    pub buttons: [f32; INPUT_COUNT],
    /// Left stick x, y and right stick x, y; -1..1, up positive.
    pub axes: [f32; 4],
    /// GameCube hardware bytes, when the host reads them directly (the
    /// adapter): the sticks and triggers then map one to one.
    pub direct: Option<DirectAnalog>,
}

/// A GameCube controller's analog wire bytes (sticks centred near 128).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DirectAnalog {
    pub stick: [u8; 2],
    pub cstick: [u8; 2],
    pub triggers: [u8; 2],
}

impl RawPad {
    /// One adapter port's controller in the standard layout: the GameCube's
    /// A at the bottom, B left, X right, Y top, Z the right bumper.
    pub fn from_adapter(pad: &adapter::AdapterPad) -> Self {
        use adapter::buttons as b;
        let mut raw = Self::default();
        for (bit, input) in [
            (b::A, Input::South),
            (b::B, Input::West),
            (b::X, Input::East),
            (b::Y, Input::North),
            (b::Z, Input::RightBumper),
            (b::L, Input::LeftTrigger),
            (b::R, Input::RightTrigger),
            (b::START, Input::Start),
            (b::DPAD_UP, Input::DpadUp),
            (b::DPAD_DOWN, Input::DpadDown),
            (b::DPAD_LEFT, Input::DpadLeft),
            (b::DPAD_RIGHT, Input::DpadRight),
        ] {
            if pad.buttons & bit != 0 {
                raw.buttons[input.index()] = 1.0;
            }
        }
        raw.direct = Some(DirectAnalog {
            stick: pad.stick,
            cstick: pad.cstick,
            triggers: pad.triggers,
        });
        raw
    }
}

/// A connected pad as a settings screen shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct PadInfo {
    pub handle: u32,
    pub family: Family,
    pub name: String,
    /// The key its saved settings live under.
    pub identity: String,
    /// The controller port it drives, once joined.
    pub port: Option<usize>,
    /// Waiting for a control to press for this GameCube button.
    pub listening: Option<Target>,
    pub settings: Settings,
    /// The last mapped state (for a live stick and trigger readout).
    pub state: Gamepad,
}

/// One poll of a pad.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PadRead {
    pub state: Gamepad,
    pub events: MenuEvents,
    pub port: Option<usize>,
}

/// A press waits this long for the control to bind before giving up.
const LISTEN_MS: f64 = 5000.0;
/// A control counts as pressed (menus, rebinding) past half its travel.
const PRESSED: f32 = 0.5;

/// Origin-calibration of a GameCube stick, taken at its first report as
/// the console does at plug-in.
#[derive(Clone, Copy, Debug)]
struct Origin {
    stick: [i16; 2],
    cstick: [i16; 2],
    triggers: [i16; 2],
}

struct Pad {
    handle: u32,
    family: Family,
    name: String,
    identity: String,
    settings: Settings,
    port: Option<usize>,
    origin: Option<Origin>,
    nav: nav::Navigator,
    /// [`Input`] bits held at the last poll.
    held: u32,
    listening: Option<(Target, Option<f64>)>,
    last: MenuPad,
}

impl Pad {
    /// The GameCube sticks and triggers: one to one for GameCube hardware
    /// (origin-adjusted), through the octagonal gate otherwise.
    fn analog(&mut self, raw: &RawPad) -> ([i8; 2], [i8; 2], Option<[u8; 2]>) {
        let wire = match raw.direct {
            Some(d) => Some((
                d.stick.map(|v| i16::from(v) - 128),
                d.cstick.map(|v| i16::from(v) - 128),
                d.triggers.map(i16::from),
            )),
            None if self.family.gamecube_stick() => {
                let [x, y] = calibration::direct_stick(raw.axes[0], raw.axes[1]);
                let [cx, cy] = calibration::direct_stick(raw.axes[2], raw.axes[3]);
                let trigger = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as i16;
                Some((
                    [i16::from(x), i16::from(y)],
                    [i16::from(cx), i16::from(cy)],
                    [
                        trigger(raw.buttons[Input::LeftTrigger.index()]),
                        trigger(raw.buttons[Input::RightTrigger.index()]),
                    ],
                ))
            }
            None => None,
        };
        let Some((stick, cstick, triggers)) = wire else {
            let dz = self.settings.deadzone;
            return (
                calibration::gate_stick(raw.axes[0], raw.axes[1], dz, calibration::MAIN_GATE),
                calibration::gate_stick(raw.axes[2], raw.axes[3], dz, calibration::C_GATE),
                None,
            );
        };
        let origin = *self.origin.get_or_insert(Origin {
            stick,
            cstick,
            triggers,
        });
        let signed = |v: i16, o: i16| (v - o).clamp(-128, 127) as i8;
        let unsigned = |v: i16, o: i16| (v - o).clamp(0, 255) as u8;
        let dz = (self.settings.deadzone * 128.0) as i16;
        let rest = |s: [i8; 2]| {
            if i16::from(s[0]).abs() <= dz && i16::from(s[1]).abs() <= dz {
                [0, 0]
            } else {
                s
            }
        };
        (
            rest([signed(stick[0], origin.stick[0]), signed(stick[1], origin.stick[1])]),
            rest([signed(cstick[0], origin.cstick[0]), signed(cstick[1], origin.cstick[1])]),
            Some([
                unsigned(triggers[0], origin.triggers[0]),
                unsigned(triggers[1], origin.triggers[1]),
            ]),
        )
    }

    /// The pad as a GameCube controller, and which [`Input`]s are held.
    fn map(&mut self, raw: &RawPad) -> (MenuPad, u32) {
        let (stick, cstick, direct_triggers) = self.analog(raw);
        let mut state = MenuPad {
            pad: Gamepad {
                stick,
                cstick,
                ..Gamepad::default()
            },
            ..MenuPad::default()
        };
        let mut held = 0;
        for input in Input::ALL {
            let value = raw.buttons[input.index()];
            let value = if value.is_finite() { value.clamp(0.0, 1.0) } else { 0.0 };
            let pressed = value >= PRESSED;
            if pressed {
                held |= 1 << input.index();
            }
            let bit = |b: PadButton| b.bit();
            let target = self.settings.target(input);
            match target {
                Target::L | Target::R => {
                    let side = usize::from(target == Target::R);
                    let analog = match (input.trigger_side(), direct_triggers) {
                        (Some(trigger), Some(bytes)) => bytes[trigger],
                        (Some(_), None) => calibration::trigger_byte(value),
                        // A digital control clicks in fully.
                        (None, _) => calibration::trigger_byte(if pressed { 1.0 } else { 0.0 }),
                    };
                    state.pad.triggers[side] = state.pad.triggers[side].max(analog);
                    let clicked = if input.trigger_side().is_some() {
                        value >= self.settings.trigger_click
                    } else {
                        pressed
                    };
                    if clicked {
                        state.pad.buttons |= bit(if side == 0 { PadButton::L } else { PadButton::R });
                    }
                }
                _ if !pressed => {}
                Target::None => {}
                Target::Start => state.start = true,
                Target::A => state.pad.buttons |= bit(PadButton::A),
                Target::B => state.pad.buttons |= bit(PadButton::B),
                Target::X => state.pad.buttons |= bit(PadButton::X),
                Target::Y => state.pad.buttons |= bit(PadButton::Y),
                Target::Z => state.pad.buttons |= bit(PadButton::Z),
                Target::DpadUp => state.pad.buttons |= bit(PadButton::DpadUp),
                Target::DpadDown => state.pad.buttons |= bit(PadButton::DpadDown),
                Target::DpadLeft => state.pad.buttons |= bit(PadButton::DpadLeft),
                Target::DpadRight => state.pad.buttons |= bit(PadButton::DpadRight),
            }
        }
        // Select and Home open the app menu unless bound to something.
        state.menu = [Input::Select, Input::Home]
            .into_iter()
            .any(|i| held & 1 << i.index() != 0 && self.settings.target(i) == Target::None);
        (state, held)
    }
}

/// Every connected pad, and the settings saved for pads seen before.
#[derive(Default)]
pub struct Pads {
    pads: Vec<Pad>,
    saved: BTreeMap<String, Settings>,
}

fn identity(family: Family, vendor: u16, product: u16, name: &str) -> String {
    let name: String = name
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    format!("{}|{vendor:04x}:{product:04x}|{}", family.key(), name.trim())
}

impl Pads {
    /// A pad appeared. `name` and `category` are whatever names the host
    /// has (both are searched for the family); USB ids are 0 when unknown.
    /// Saved settings for the same controller apply.
    pub fn connect(&mut self, handle: u32, name: &str, category: &str, vendor: u16, product: u16) -> Family {
        self.pads.retain(|p| p.handle != handle);
        let family = family::detect(vendor, product, &[name, category]);
        let identity = identity(family, vendor, product, name);
        let settings = self
            .saved
            .get(&identity)
            .copied()
            .unwrap_or_else(|| Settings::defaults(family));
        let display = if name.trim().is_empty() { family.name() } else { name.trim() };
        self.pads.push(Pad {
            handle,
            family,
            name: display.to_owned(),
            identity,
            settings,
            port: None,
            origin: None,
            nav: nav::Navigator::default(),
            held: 0,
            listening: None,
            last: MenuPad::default(),
        });
        family
    }
    /// A pad went away; its port, if it had one.
    pub fn disconnect(&mut self, handle: u32) -> Option<usize> {
        let index = self.pads.iter().position(|p| p.handle == handle)?;
        self.pads.remove(index).port
    }
    pub fn is_connected(&self, handle: u32) -> bool {
        self.pads.iter().any(|p| p.handle == handle)
    }
    /// Connected pads' handles, oldest first.
    pub fn handles(&self) -> Vec<u32> {
        self.pads.iter().map(|p| p.handle).collect()
    }
    fn pad(&self, handle: u32) -> Option<&Pad> {
        self.pads.iter().find(|p| p.handle == handle)
    }
    fn pad_mut(&mut self, handle: u32) -> Option<&mut Pad> {
        self.pads.iter_mut().find(|p| p.handle == handle)
    }
    pub fn info(&self, handle: u32) -> Option<PadInfo> {
        self.pad(handle).map(|p| PadInfo {
            handle: p.handle,
            family: p.family,
            name: p.name.clone(),
            identity: p.identity.clone(),
            port: p.port,
            listening: p.listening.map(|(t, _)| t),
            settings: p.settings,
            state: p.last.pad,
        })
    }
    pub fn port(&self, handle: u32) -> Option<usize> {
        self.pad(handle).and_then(|p| p.port)
    }
    /// The pad driving controller port `port`.
    pub fn on_port(&self, port: usize) -> Option<u32> {
        self.pads.iter().find(|p| p.port == Some(port)).map(|p| p.handle)
    }

    /// Read one pad: its GameCube state and menu events. While it waits
    /// for a rebind, the next control pressed binds instead.
    pub fn read(&mut self, handle: u32, raw: &RawPad, now_ms: f64) -> Option<PadRead> {
        let pad = self.pad_mut(handle)?;
        let (state, held) = pad.map(raw);
        let newly = held & !pad.held;
        pad.held = held;
        pad.last = state;
        if let Some((target, deadline)) = pad.listening {
            let deadline = deadline.unwrap_or(now_ms + LISTEN_MS);
            pad.listening = Some((target, Some(deadline)));
            let mut events = MenuEvents(0);
            if let Some(input) = Input::ALL.into_iter().find(|i| newly & 1 << i.index() != 0) {
                pad.settings.bind(input, target);
                pad.listening = None;
                let (state, _) = pad.map(raw);
                pad.last = state;
                pad.nav.hold(&state);
                let (identity, settings, port) = (pad.identity.clone(), pad.settings, pad.port);
                self.saved.insert(identity, settings);
                return Some(PadRead {
                    state: state.pad,
                    events: MenuEvents(MenuEvents::BOUND),
                    port,
                });
            }
            if now_ms >= deadline {
                pad.listening = None;
                pad.nav.hold(&state);
                events = MenuEvents(MenuEvents::BOUND);
            }
            return Some(PadRead {
                state: state.pad,
                events,
                port: pad.port,
            });
        }
        let events = pad.nav.step(&state, now_ms);
        Some(PadRead {
            state: state.pad,
            events,
            port: pad.port,
        })
    }

    pub fn settings(&self, handle: u32) -> Option<Settings> {
        self.pad(handle).map(|p| p.settings)
    }
    /// Change a pad's settings; they are saved for this controller.
    pub fn set_settings(&mut self, handle: u32, settings: Settings) {
        if let Some(pad) = self.pad_mut(handle) {
            pad.settings = settings;
            let identity = pad.identity.clone();
            self.saved.insert(identity, settings);
        }
    }
    /// Back to the family's defaults (and forget the saved settings).
    pub fn reset(&mut self, handle: u32) {
        if let Some(pad) = self.pad_mut(handle) {
            pad.settings = Settings::defaults(pad.family);
            let identity = pad.identity.clone();
            self.saved.remove(&identity);
        }
    }
    /// Wait for the next control pressed on this pad and bind it to
    /// `target` (`None` stops waiting).
    pub fn listen(&mut self, handle: u32, target: Option<Target>) {
        if let Some(pad) = self.pad_mut(handle) {
            // Controls already held (the A that chose the row) bind only
            // once released and pressed again: the binding takes new presses.
            pad.listening = target.map(|t| (t, None));
        }
    }
    /// Set the pad's port. A pad already there takes this pad's old port.
    pub fn assign(&mut self, handle: u32, port: Option<usize>) {
        let Some(old) = self.pad(handle).map(|p| p.port) else {
            return;
        };
        if let Some(port) = port {
            if let Some(other) = self.pads.iter_mut().find(|p| p.port == Some(port) && p.handle != handle) {
                other.port = old;
            }
        }
        if let Some(pad) = self.pad_mut(handle) {
            pad.port = port;
        }
    }

    /// Every pad's saved settings, one `identity\tsettings` line each, for
    /// the host to store (UserDefaults, localStorage).
    pub fn prefs(&self) -> String {
        self.saved
            .iter()
            .map(|(identity, settings)| format!("{identity}\t{}\n", settings.encode()))
            .collect()
    }
    /// Restore [`Self::prefs`]' text; lines that do not parse are dropped.
    /// Connected pads take their saved settings.
    pub fn load_prefs(&mut self, text: &str) {
        for line in text.lines() {
            if let Some((identity, encoded)) = line.split_once('\t') {
                if let Some(settings) = Settings::decode(encoded) {
                    self.saved.insert(identity.to_owned(), settings);
                }
            }
        }
        for pad in &mut self.pads {
            if let Some(settings) = self.saved.get(&pad.identity) {
                pad.settings = *settings;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(inputs: &[Input]) -> RawPad {
        let mut raw = RawPad::default();
        for input in inputs {
            raw.buttons[input.index()] = 1.0;
        }
        raw
    }

    #[test]
    fn a_switch_pro_controller_plays_by_position() {
        let mut pads = Pads::default();
        assert_eq!(pads.connect(7, "Pro Controller", "Switch Pro Controller", 0, 0), Family::SwitchPro);
        // Bottom (labelled B), the right bumper, ZL fully in (digital), the
        // stick full right and the right stick up.
        let mut raw = press(&[Input::South, Input::RightBumper, Input::LeftTrigger]);
        raw.axes = [1.0, 0.0, 0.0, 1.0];
        let read = pads.read(7, &raw, 0.0).unwrap();
        let s = read.state;
        assert_eq!(s.buttons, PadButton::A.bit() | PadButton::Z.bit() | PadButton::L.bit());
        assert_eq!((s.stick, s.cstick, s.triggers), ([87, 0], [0, 74], [140, 0]));
        assert!(read.events.has(MenuEvents::CONFIRM));
        assert!(read.events.has(MenuEvents::RIGHT));
        assert!(read.events.has(MenuEvents::Z));
        assert_eq!(read.port, None);
    }

    #[test]
    fn an_analog_trigger_shields_lightly_and_clicks_near_the_end() {
        let mut pads = Pads::default();
        pads.connect(1, "DualSense Wireless Controller", "DualSense", 0x054c, 0x0ce6);
        let mut raw = RawPad::default();
        raw.buttons[Input::RightTrigger.index()] = 0.5;
        let state = pads.read(1, &raw, 0.0).unwrap().state;
        assert_eq!((state.triggers, state.buttons), ([0, 70], 0));
        raw.buttons[Input::RightTrigger.index()] = 0.97;
        let state = pads.read(1, &raw, 16.0).unwrap().state;
        assert_eq!((state.triggers[1], state.buttons), (136, PadButton::R.bit()));
    }

    #[test]
    fn an_adapter_port_is_origin_calibrated_one_to_one() {
        let mut pads = Pads::default();
        pads.connect(100, "GameCube controller", "", adapter::VENDOR, adapter::PRODUCT);
        let pad = |stick: [u8; 2], buttons: u16| adapter::AdapterPad {
            buttons,
            stick,
            cstick: [128, 128],
            triggers: [30, 30],
        };
        // The first report is the origin, as at plug-in.
        let rest = pads.read(100, &RawPad::from_adapter(&pad([131, 126], 0)), 0.0).unwrap();
        assert_eq!((rest.state.stick, rest.state.triggers), ([0, 0], [0, 0]));
        let held = adapter::buttons::B | adapter::buttons::L;
        let tilt = RawPad::from_adapter(&pad([131 + 70, 126 - 20], held));
        let state = pads.read(100, &tilt, 16.0).unwrap().state;
        assert_eq!(state.stick, [70, -20]);
        assert_eq!(state.buttons, PadButton::B.bit() | PadButton::L.bit());
    }

    #[test]
    fn rebinding_takes_the_next_new_press_and_is_saved() {
        let mut pads = Pads::default();
        pads.connect(3, "Xbox Wireless Controller", "Xbox One", 0, 0);
        // The A that chose the row is still held when listening starts.
        let first = pads.read(3, &press(&[Input::South]), 0.0).unwrap();
        assert!(first.events.has(MenuEvents::CONFIRM));
        pads.listen(3, Some(Target::Z));
        assert_eq!(pads.read(3, &press(&[Input::South]), 16.0).unwrap().events, MenuEvents(0));
        assert_eq!(pads.info(3).unwrap().listening, Some(Target::Z));
        let read = pads.read(3, &press(&[Input::South, Input::North]), 32.0).unwrap();
        assert_eq!(read.events, MenuEvents(MenuEvents::BOUND));
        let settings = pads.settings(3).unwrap();
        assert_eq!(settings.target(Input::North), Target::Z);
        // A control that had Z (the first bumper) takes Y.
        assert_eq!(settings.target(Input::LeftBumper), Target::Y);
        assert_eq!(settings.target(Input::RightBumper), Target::Z);
        // Saved under the controller's identity: a reconnect keeps it.
        let prefs = pads.prefs();
        let mut fresh = Pads::default();
        fresh.load_prefs(&prefs);
        fresh.connect(9, "Xbox Wireless Controller", "Xbox One", 0, 0);
        assert_eq!(fresh.settings(9), Some(settings));
        fresh.reset(9);
        assert_eq!(fresh.settings(9), Some(Settings::defaults(Family::Xbox)));
        assert_eq!(fresh.prefs(), "");
        // Listening gives up after five seconds.
        pads.listen(3, Some(Target::A));
        pads.read(3, &RawPad::default(), 100.0);
        let late = pads.read(3, &RawPad::default(), 5200.0).unwrap();
        assert_eq!(late.events, MenuEvents(MenuEvents::BOUND));
        assert_eq!(pads.info(3).unwrap().listening, None);
    }

    #[test]
    fn assigning_a_taken_port_swaps() {
        let mut pads = Pads::default();
        pads.connect(1, "a", "", 0, 0);
        pads.connect(2, "b", "", 0, 0);
        pads.assign(1, Some(0));
        pads.assign(2, Some(1));
        pads.assign(2, Some(0));
        assert_eq!((pads.port(1), pads.port(2)), (Some(1), Some(0)));
        assert_eq!(pads.on_port(0), Some(2));
        assert_eq!(pads.disconnect(2), Some(0));
        assert_eq!(pads.handles(), [1]);
    }
}
