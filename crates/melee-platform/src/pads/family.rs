//! Which kind of controller a host pad is, from what the host can see:
//! USB vendor and product ids where it has them (the web's `Gamepad.id`),
//! otherwise the names (macOS GameController's `vendorName` and
//! `productCategory`).

use super::mapping::Input;

/// A controller family: its default mapping and how its buttons are labelled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Family {
    /// A GameCube controller: through a Wii U adapter (WUP-028 or a clone
    /// in Wii U mode), a PC-mode adapter, or Nintendo's Switch GameCube
    /// controller. Mapped one to one; its stick is already gated.
    GameCube,
    SwitchPro,
    /// A Joy-Con pair held as one controller.
    JoyCon,
    DualSense,
    DualShock4,
    Xbox,
    EightBitDo,
    /// Anything else with a standard layout.
    Generic,
}

impl Family {
    pub const ALL: [Self; 8] = [
        Self::GameCube,
        Self::SwitchPro,
        Self::JoyCon,
        Self::DualSense,
        Self::DualShock4,
        Self::Xbox,
        Self::EightBitDo,
        Self::Generic,
    ];
    /// Stable key, used in saved preferences.
    pub fn key(self) -> &'static str {
        match self {
            Self::GameCube => "gamecube",
            Self::SwitchPro => "switch-pro",
            Self::JoyCon => "joy-con",
            Self::DualSense => "dualsense",
            Self::DualShock4 => "dualshock4",
            Self::Xbox => "xbox",
            Self::EightBitDo => "8bitdo",
            Self::Generic => "generic",
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::GameCube => "GameCube controller",
            Self::SwitchPro => "Switch Pro Controller",
            Self::JoyCon => "Joy-Con pair",
            Self::DualSense => "DualSense",
            Self::DualShock4 => "DualShock 4",
            Self::Xbox => "Xbox controller",
            Self::EightBitDo => "8BitDo controller",
            Self::Generic => "Gamepad",
        }
    }
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.key() == key)
    }
    /// The stick arrives already gated and origin-calibrated by GameCube
    /// hardware, so it maps one to one instead of through the modern-stick
    /// gate ([`super::calibration`]).
    pub fn gamecube_stick(self) -> bool {
        self == Self::GameCube
    }
    /// The stick deadzone a fresh pad of this family starts with: worn or
    /// noisy sticks (Joy-Con, Xbox) rest further from the centre.
    pub fn default_deadzone(self) -> f32 {
        match self {
            Self::GameCube => 0.0,
            Self::JoyCon => 0.2,
            Self::Xbox | Self::Generic => 0.12,
            Self::SwitchPro | Self::EightBitDo | Self::DualShock4 => 0.1,
            Self::DualSense => 0.08,
        }
    }
    /// What the family prints on a physical control (positions are the
    /// W3C standard layout's: the bottom face button is `South`).
    pub fn label(self, input: Input) -> &'static str {
        use Input::*;
        let nintendo = matches!(self, Self::SwitchPro | Self::JoyCon);
        let sony = matches!(self, Self::DualSense | Self::DualShock4);
        match input {
            South if self == Self::GameCube => "A",
            East if self == Self::GameCube => "X",
            West if self == Self::GameCube => "B",
            North if self == Self::GameCube => "Y",
            South if nintendo => "B",
            East if nintendo => "A",
            West if nintendo => "Y",
            North if nintendo => "X",
            South if sony => "Cross",
            East if sony => "Circle",
            West if sony => "Square",
            North if sony => "Triangle",
            South => "A",
            East => "B",
            West => "X",
            North => "Y",
            LeftBumper if self == Self::GameCube => "-",
            RightBumper if self == Self::GameCube => "Z",
            LeftTrigger if self == Self::GameCube => "L",
            RightTrigger if self == Self::GameCube => "R",
            LeftBumper if sony => "L1",
            RightBumper if sony => "R1",
            LeftTrigger if sony => "L2",
            RightTrigger if sony => "R2",
            LeftBumper if nintendo => "L",
            RightBumper if nintendo => "R",
            LeftTrigger if nintendo => "ZL",
            RightTrigger if nintendo => "ZR",
            LeftBumper => "LB",
            RightBumper => "RB",
            LeftTrigger => "LT",
            RightTrigger => "RT",
            Select if self == Self::DualSense => "Create",
            Select if self == Self::DualShock4 => "Share",
            Select if nintendo => "Minus",
            Select if self == Self::Xbox => "View",
            Select => "Select",
            Start if sony => "Options",
            Start if nintendo => "Plus",
            Start if self == Self::Xbox => "Menu",
            Start => "Start",
            LeftStick if sony => "L3",
            RightStick if sony => "R3",
            LeftStick => "Left stick press",
            RightStick => "Right stick press",
            DpadUp => "D-pad up",
            DpadDown => "D-pad down",
            DpadLeft => "D-pad left",
            DpadRight => "D-pad right",
            Home if sony => "PS",
            Home if self == Self::Xbox => "Xbox",
            Home => "Home",
        }
    }
}

/// USB ids by family (the first match wins).
const IDS: &[(u16, Option<u16>, Family)] = &[
    // Nintendo: Wii U GameCube adapter (WUP-028), Switch GameCube controller.
    (0x057e, Some(0x0337), Family::GameCube),
    (0x057e, Some(0x2073), Family::GameCube),
    // Mayflash and other PC-mode GameCube adapters.
    (0x0079, Some(0x1843), Family::GameCube),
    (0x0079, Some(0x1844), Family::GameCube),
    (0x0079, Some(0x1846), Family::GameCube),
    (0x057e, Some(0x2009), Family::SwitchPro),
    // Joy-Con (L), (R) and the charging grip pair.
    (0x057e, Some(0x2006), Family::JoyCon),
    (0x057e, Some(0x2007), Family::JoyCon),
    (0x057e, Some(0x200e), Family::JoyCon),
    // DualSense and DualSense Edge.
    (0x054c, Some(0x0ce6), Family::DualSense),
    (0x054c, Some(0x0df2), Family::DualSense),
    // DualShock 4 (both revisions) and its wireless adapter.
    (0x054c, Some(0x05c4), Family::DualShock4),
    (0x054c, Some(0x09cc), Family::DualShock4),
    (0x054c, Some(0x0ba0), Family::DualShock4),
    (0x045e, None, Family::Xbox),
    (0x2dc8, None, Family::EightBitDo),
];

/// Name fragments by family, lower case, for hosts without USB ids.
const NAMES: &[(&str, Family)] = &[
    ("gamecube", Family::GameCube),
    ("wup-028", Family::GameCube),
    ("joy-con", Family::JoyCon),
    ("pro controller", Family::SwitchPro),
    ("dualsense", Family::DualSense),
    ("dualshock", Family::DualShock4),
    ("xbox", Family::Xbox),
    ("8bitdo", Family::EightBitDo),
];

/// The family of a pad from its USB ids (0 when unknown) and names.
pub fn detect(vendor: u16, product: u16, names: &[&str]) -> Family {
    if vendor != 0 {
        let by_id = IDS
            .iter()
            .find(|(v, p, _)| *v == vendor && p.is_none_or(|p| p == product));
        if let Some(&(_, _, family)) = by_id {
            return family;
        }
    }
    for name in names {
        let name = name.to_lowercase();
        if let Some(&(_, family)) = NAMES.iter().find(|(part, _)| name.contains(part)) {
            return family;
        }
    }
    Family::Generic
}

/// A web `Gamepad.id` split into USB ids and the name. Chromium writes
/// `Name (STANDARD GAMEPAD Vendor: 057e Product: 2009)`, Firefox
/// `057e-2009-Name`; Safari and others only the name.
pub fn parse_web_id(id: &str) -> (u16, u16, String) {
    let hex = |s: &str| u16::from_str_radix(s.trim(), 16).ok();
    if let (Some(v), Some(p)) = (id.find("Vendor: "), id.find("Product: ")) {
        let vendor = id[v + 8..].get(..4).and_then(hex);
        let product = id[p + 9..].get(..4).and_then(hex);
        if let (Some(vendor), Some(product)) = (vendor, product) {
            let name = id[..id.rfind(" (").unwrap_or(v)].trim().to_owned();
            return (vendor, product, name);
        }
    }
    let mut parts = id.splitn(3, '-');
    if let (Some(v), Some(p), Some(name)) = (parts.next(), parts.next(), parts.next()) {
        if v.len() == 4 && p.len() == 4 {
            if let (Some(vendor), Some(product)) = (hex(v), hex(p)) {
                return (vendor, product, name.trim().to_owned());
            }
        }
    }
    (0, 0, id.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_ids_from_each_browser_name_the_family() {
        let cases = [
            (
                "Pro Controller (STANDARD GAMEPAD Vendor: 057e Product: 2009)",
                Family::SwitchPro,
                "Pro Controller",
            ),
            (
                "DualSense Wireless Controller (STANDARD GAMEPAD Vendor: 054c Product: 0ce6)",
                Family::DualSense,
                "DualSense Wireless Controller",
            ),
            (
                "Wireless Controller (STANDARD GAMEPAD Vendor: 054c Product: 09cc)",
                Family::DualShock4,
                "Wireless Controller",
            ),
            (
                "Xbox Wireless Controller (STANDARD GAMEPAD Vendor: 045e Product: 0b13)",
                Family::Xbox,
                "Xbox Wireless Controller",
            ),
            ("054c-05c4-Wireless Controller", Family::DualShock4, "Wireless Controller"),
            ("2dc8-6101-8BitDo SN30 Pro", Family::EightBitDo, "8BitDo SN30 Pro"),
            (
                "Joy-Con L+R (STANDARD GAMEPAD Vendor: 057e Product: 200e)",
                Family::JoyCon,
                "Joy-Con L+R",
            ),
            (
                "MAYFLASH GameCube Controller Adapter (Vendor: 0079 Product: 1846)",
                Family::GameCube,
                "MAYFLASH GameCube Controller Adapter",
            ),
            ("Unknown Gamepad", Family::Generic, "Unknown Gamepad"),
        ];
        for (id, family, name) in cases {
            let (vendor, product, parsed) = parse_web_id(id);
            assert_eq!(parsed, name, "{id}");
            assert_eq!(detect(vendor, product, &[&parsed]), family, "{id}");
        }
    }

    #[test]
    fn macos_names_name_the_family() {
        // GameController's vendorName and productCategory.
        let cases = [
            (["Pro Controller", "Switch Pro Controller"], Family::SwitchPro),
            (["Joy-Con (L/R)", "Nintendo Switch Joy-Con (L/R)"], Family::JoyCon),
            (["DualSense Wireless Controller", "DualSense"], Family::DualSense),
            (["DUALSHOCK 4 Wireless Controller", "DualShock 4"], Family::DualShock4),
            (["Xbox Wireless Controller", "Xbox One"], Family::Xbox),
            (["8BitDo Ultimate", "MFi"], Family::EightBitDo),
            (["GameCube Controller (WUP-028 port 1)", "GameCube"], Family::GameCube),
            (["Some Pad", "MFi"], Family::Generic),
        ];
        for (names, family) in cases {
            assert_eq!(detect(0, 0, &names), family, "{names:?}");
        }
    }

    #[test]
    fn labels_follow_the_family_not_the_position() {
        assert_eq!(Family::SwitchPro.label(Input::South), "B");
        assert_eq!(Family::Xbox.label(Input::South), "A");
        assert_eq!(Family::DualSense.label(Input::South), "Cross");
        assert_eq!(Family::GameCube.label(Input::West), "B");
        for family in Family::ALL {
            assert_eq!(Family::from_key(family.key()), Some(family));
        }
    }
}
