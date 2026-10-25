//! A pad's physical controls onto the GameCube controller's, by position:
//! the bottom face button is the GameCube's big A wherever it sits on the
//! label (a Nintendo pad's A is on the right, so its B becomes A). Each pad
//! keeps its bindings, stick deadzone and trigger click point.

use super::family::Family;

/// Physical controls in the W3C standard gamepad layout (the web's
/// `mapping == "standard"` button order), named by position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Input {
    South,
    East,
    West,
    North,
    LeftBumper,
    RightBumper,
    LeftTrigger,
    RightTrigger,
    Select,
    Start,
    LeftStick,
    RightStick,
    DpadUp,
    DpadDown,
    DpadLeft,
    DpadRight,
    Home,
}
pub const INPUT_COUNT: usize = 17;

impl Input {
    pub const ALL: [Self; INPUT_COUNT] = [
        Self::South,
        Self::East,
        Self::West,
        Self::North,
        Self::LeftBumper,
        Self::RightBumper,
        Self::LeftTrigger,
        Self::RightTrigger,
        Self::Select,
        Self::Start,
        Self::LeftStick,
        Self::RightStick,
        Self::DpadUp,
        Self::DpadDown,
        Self::DpadLeft,
        Self::DpadRight,
        Self::Home,
    ];
    pub fn index(self) -> usize {
        self as usize
    }
    pub fn from_index(index: usize) -> Option<Self> {
        Self::ALL.get(index).copied()
    }
    /// The analog trigger behind this control, if it is one (0 left, 1 right).
    pub fn trigger_side(self) -> Option<usize> {
        match self {
            Self::LeftTrigger => Some(0),
            Self::RightTrigger => Some(1),
            _ => None,
        }
    }
    /// Where the control sits, for a settings screen.
    pub fn position(self) -> &'static str {
        match self {
            Self::South => "bottom face button",
            Self::East => "right face button",
            Self::West => "left face button",
            Self::North => "top face button",
            Self::LeftBumper => "left bumper",
            Self::RightBumper => "right bumper",
            Self::LeftTrigger => "left trigger",
            Self::RightTrigger => "right trigger",
            Self::Select => "select",
            Self::Start => "start",
            Self::LeftStick => "left stick press",
            Self::RightStick => "right stick press",
            Self::DpadUp => "d-pad up",
            Self::DpadDown => "d-pad down",
            Self::DpadLeft => "d-pad left",
            Self::DpadRight => "d-pad right",
            Self::Home => "home",
        }
    }
}

/// What a physical control does: a GameCube controller button.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Target {
    #[default]
    None,
    A,
    B,
    X,
    Y,
    Z,
    /// L: analog from a trigger, clicked past the click point.
    L,
    R,
    Start,
    DpadUp,
    DpadDown,
    DpadLeft,
    DpadRight,
}

impl Target {
    /// The GameCube controls a settings screen lists, in order.
    pub const ALL: [Self; 12] = [
        Self::A,
        Self::B,
        Self::X,
        Self::Y,
        Self::Z,
        Self::L,
        Self::R,
        Self::Start,
        Self::DpadUp,
        Self::DpadDown,
        Self::DpadLeft,
        Self::DpadRight,
    ];
    pub fn from_index(index: u32) -> Option<Self> {
        match index {
            0 => Some(Self::None),
            n => Self::ALL.get(n as usize - 1).copied(),
        }
    }
    pub fn index(self) -> u32 {
        self as u32
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::None => "Nothing",
            Self::A => "A",
            Self::B => "B",
            Self::X => "X",
            Self::Y => "Y",
            Self::Z => "Z",
            Self::L => "L",
            Self::R => "R",
            Self::Start => "Start",
            Self::DpadUp => "D-pad up",
            Self::DpadDown => "D-pad down",
            Self::DpadLeft => "D-pad left",
            Self::DpadRight => "D-pad right",
        }
    }
    /// Short code in saved preferences.
    fn code(self) -> &'static str {
        match self {
            Self::None => "-",
            Self::A => "A",
            Self::B => "B",
            Self::X => "X",
            Self::Y => "Y",
            Self::Z => "Z",
            Self::L => "L",
            Self::R => "R",
            Self::Start => "S",
            Self::DpadUp => "U",
            Self::DpadDown => "D",
            Self::DpadLeft => "<",
            Self::DpadRight => ">",
        }
    }
    fn from_code(code: &str) -> Option<Self> {
        std::iter::once(Self::None)
            .chain(Self::ALL)
            .find(|t| t.code() == code)
    }
}

/// Stick deadzone bounds (fraction of full tilt).
pub const DEADZONE_MAX: f32 = 0.5;
/// Trigger click point bounds (fraction of full travel).
pub const TRIGGER_CLICK_MIN: f32 = 0.3;
/// Default click point: near the end of travel, like the GameCube's click.
const TRIGGER_CLICK: f32 = 0.95;

/// One pad's mapping.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    /// What each [`Input`] does, by index.
    pub bindings: [Target; INPUT_COUNT],
    /// Radial stick deadzone, 0..=[`DEADZONE_MAX`].
    pub deadzone: f32,
    /// Trigger travel at which L/R click in, [`TRIGGER_CLICK_MIN`]..=1.
    pub trigger_click: f32,
}

impl Settings {
    /// The family's defaults: GameCube buttons by position, both bumpers Z,
    /// triggers L and R, Start, the d-pad.
    pub fn defaults(family: Family) -> Self {
        let mut bindings = [Target::None; INPUT_COUNT];
        for (input, target) in [
            (Input::South, Target::A),
            (Input::West, Target::B),
            (Input::East, Target::X),
            (Input::North, Target::Y),
            (Input::RightBumper, Target::Z),
            (Input::LeftTrigger, Target::L),
            (Input::RightTrigger, Target::R),
            (Input::Start, Target::Start),
            (Input::DpadUp, Target::DpadUp),
            (Input::DpadDown, Target::DpadDown),
            (Input::DpadLeft, Target::DpadLeft),
            (Input::DpadRight, Target::DpadRight),
        ] {
            bindings[input.index()] = target;
        }
        // A GameCube pad's left "bumper" does not exist; elsewhere both
        // bumpers are Z, so either hand can grab.
        if family != Family::GameCube {
            bindings[Input::LeftBumper.index()] = Target::Z;
        }
        Self {
            bindings,
            deadzone: family.default_deadzone(),
            trigger_click: TRIGGER_CLICK,
        }
    }
    pub fn target(&self, input: Input) -> Target {
        self.bindings[input.index()]
    }
    /// The controls bound to `target`.
    pub fn inputs_for(&self, target: Target) -> impl Iterator<Item = Input> + '_ {
        Input::ALL
            .into_iter()
            .filter(move |input| self.target(*input) == target)
    }
    /// Bind `input` to `target`. The control that had `target` takes
    /// `input`'s old job, so nothing is left unreachable.
    pub fn bind(&mut self, input: Input, target: Target) {
        let previous = self.target(input);
        if previous == target {
            return;
        }
        if target != Target::None {
            let other = self.inputs_for(target).next();
            if let Some(other) = other {
                self.bindings[other.index()] = previous;
            }
        }
        self.bindings[input.index()] = target;
    }
    /// Swap whatever does A with whatever does B.
    pub fn swap_ab(&mut self) {
        for binding in &mut self.bindings {
            *binding = match *binding {
                Target::A => Target::B,
                Target::B => Target::A,
                other => other,
            };
        }
    }
    pub fn set_deadzone(&mut self, deadzone: f32) {
        if deadzone.is_finite() {
            self.deadzone = deadzone.clamp(0.0, DEADZONE_MAX);
        }
    }
    pub fn set_trigger_click(&mut self, click: f32) {
        if click.is_finite() {
            self.trigger_click = click.clamp(TRIGGER_CLICK_MIN, 1.0);
        }
    }

    /// `v1;dz=0.10;tc=0.95;b=A,X,B,...` (one code per [`Input`]).
    pub fn encode(&self) -> String {
        let bindings: Vec<_> = self.bindings.iter().map(|t| t.code()).collect();
        format!(
            "v1;dz={:.2};tc={:.2};b={}",
            self.deadzone,
            self.trigger_click,
            bindings.join(",")
        )
    }
    pub fn decode(text: &str) -> Option<Self> {
        let mut fields = text.trim().split(';');
        if fields.next()? != "v1" {
            return None;
        }
        let mut settings = Self::defaults(Family::Generic);
        for field in fields {
            let (key, value) = field.split_once('=')?;
            match key {
                "dz" => settings.set_deadzone(value.parse().ok()?),
                "tc" => settings.set_trigger_click(value.parse().ok()?),
                "b" => {
                    let codes: Vec<_> = value.split(',').collect();
                    if codes.len() != INPUT_COUNT {
                        return None;
                    }
                    for (slot, code) in settings.bindings.iter_mut().zip(codes) {
                        *slot = Target::from_code(code)?;
                    }
                }
                _ => {}
            }
        }
        Some(settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_map_by_position_onto_the_gamecube_layout() {
        let pro = Settings::defaults(Family::SwitchPro);
        // The Pro Controller's bottom button (labelled B) is the GameCube's A.
        assert_eq!(pro.target(Input::South), Target::A);
        assert_eq!(Family::SwitchPro.label(Input::South), "B");
        assert_eq!(pro.target(Input::West), Target::B);
        assert_eq!(pro.target(Input::East), Target::X);
        assert_eq!(pro.target(Input::North), Target::Y);
        assert_eq!(pro.target(Input::RightBumper), Target::Z);
        assert_eq!(pro.target(Input::LeftTrigger), Target::L);
        assert_eq!(pro.target(Input::Start), Target::Start);
        assert_eq!(pro.target(Input::Home), Target::None);
        let gc = Settings::defaults(Family::GameCube);
        assert_eq!(gc.target(Input::LeftBumper), Target::None);
        assert_eq!(gc.deadzone, 0.0);
        assert!(Settings::defaults(Family::JoyCon).deadzone > pro.deadzone);
    }

    #[test]
    fn binding_swaps_so_every_button_stays_reachable() {
        let mut settings = Settings::defaults(Family::Xbox);
        settings.bind(Input::East, Target::A);
        assert_eq!(settings.target(Input::East), Target::A);
        assert_eq!(settings.target(Input::South), Target::X);
        settings.swap_ab();
        assert_eq!(settings.target(Input::East), Target::B);
        assert_eq!(settings.target(Input::West), Target::A);
        settings.bind(Input::Home, Target::None);
        assert_eq!(settings.target(Input::Home), Target::None);
    }

    #[test]
    fn settings_round_trip_and_reject_garbage() {
        let mut settings = Settings::defaults(Family::DualSense);
        settings.bind(Input::LeftBumper, Target::L);
        settings.set_deadzone(0.23);
        settings.set_trigger_click(2.0);
        let text = settings.encode();
        assert_eq!(Settings::decode(&text), Some(settings));
        assert_eq!(settings.trigger_click, 1.0);
        for bad in ["", "v2;dz=0.1", "v1;dz=abc", "v1;b=A,B", "v1;b=?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?"] {
            assert_eq!(Settings::decode(bad), None, "{bad}");
        }
    }
}
