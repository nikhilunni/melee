//! Controller-fix Gecko codes: the Universal Controller Fix (UCF) that
//! tournament setups and Slippi run on top of retail NTSC 1.02.
//!
//! Ported from the Slippi `slippi-ssbm-asm` repository (GPL-3; logic only,
//! no source copied). Each hook cites its source file and Gecko injection
//! address, the retail instruction the code replaces:
//!
//! | Version | File | Injection |
//! |---|---|---|
//! | 0.74, 0.8 | `External/UCF 0.74/UCF DB.asm`, `External/UCF 0.8/Logic/UCF DB.asm` | 0x800C9A44 (ftCo_Turn_IASA) |
//! | 0.74, 0.8 | `External/UCF 0.74/UCF SD.asm`, `External/UCF 0.8/Logic/UCF SD.asm` | 0x800998A4 (ftCo_80099894) |
//! | 0.8 | `External/UCF 0.8/Logic/UCF Tumble.asm` | 0x800908F4 (ftCo_DamageFall_IASA) |
//!
//! The 0.74 and 0.8 dashback and shield-drop codes are the same program
//! (the 0.8 files only add version tables and comments); 0.8 adds the
//! tumble wiggle. The CSS indicator text (0x802662D0) draws on the character
//! select screen and has no effect on a match.
//!
//! The codes read the hardware pad queue (HSD_PadLibData.queue, retail
//! 0x8046B108, read index at HSD_PadLibData+1, 0x804C1F78): the signed,
//! origin-adjusted `PADStatus.stickX` byte of the sample this tick consumed
//! and of the sample two ticks earlier, before HSD clamps the stick to the
//! 80-unit circle. [`PadQueueX`] carries exactly those two bytes.
use super::state::FighterInput;
use gekko_math::msl::{fabsf, fctiwz};
use std::cmp::Ordering;

/// A player's controller-fix Gecko code. Retail is `Off`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ControllerFix {
    #[default]
    Off,
    /// UCF 0.74 (2019): dashback and shield drop.
    Ucf074,
    /// UCF 0.8 (2021): 0.74 plus the tumble wiggle.
    Ucf080,
    /// UCF 0.84 (2024): not ported (it also rewrites HSD pad processing).
    Ucf084,
    /// Slippi's "Dween" dashback fix: not ported.
    Dween,
}

impl ControllerFix {
    /// Every setting with its scenario/CLI name, in declaration order.
    pub const ALL: [(Self, &'static str); 5] = [
        (Self::Off, "off"),
        (Self::Ucf074, "ucf-0.74"),
        (Self::Ucf080, "ucf-0.8"),
        (Self::Ucf084, "ucf-0.84"),
        (Self::Dween, "dween"),
    ];

    /// Parse a scenario name (`"off"`, `"ucf-0.74"`, `"ucf-0.8"`, ...).
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .find(|(_, n)| *n == name)
            .map(|&(fix, _)| fix)
    }

    pub fn name(self) -> &'static str {
        Self::ALL
            .iter()
            .find(|(fix, _)| *fix == self)
            .map(|&(_, n)| n)
            .expect("every setting is named")
    }

    /// Why the simulator cannot run this setting, if it cannot.
    pub fn unsupported(self) -> Option<&'static str> {
        match self {
            Self::Off | Self::Ucf074 | Self::Ucf080 => None,
            Self::Ucf084 => Some(
                "UCF 0.84 is not ported (External/UCF 0.84: pad buffer and 1.0 cardinals, \
                 SDI, shield SDI, DBOOC squat fix)",
            ),
            Self::Dween => Some("the Dween dashback fix is not ported"),
        }
    }

    /// The UCF dashback (`UCF DB.asm`) is installed.
    fn dashback(self) -> bool {
        match self {
            Self::Off => false,
            Self::Ucf074 | Self::Ucf080 => true,
            Self::Ucf084 => unimplemented!("UCF 0.84: External/UCF 0.84/UCF/UCF Dashback.asm"),
            Self::Dween => unimplemented!("Dween dashback fix"),
        }
    }

    /// The UCF shield drop (`UCF SD.asm`) is installed.
    fn shield_drop(self) -> bool {
        match self {
            Self::Off => false,
            Self::Ucf074 | Self::Ucf080 => true,
            Self::Ucf084 => unimplemented!("UCF 0.84: External/UCF 0.84/UCF/UCF Shield Drop.asm"),
            Self::Dween => unimplemented!("Dween controller fix: shield drop"),
        }
    }

    /// The UCF tumble wiggle (`UCF Tumble.asm`) is installed.
    fn tumble(self) -> bool {
        match self {
            Self::Off | Self::Ucf074 => false,
            Self::Ucf080 => true,
            Self::Ucf084 => unimplemented!("UCF 0.84: External/UCF 0.84/UCF/UCF Tumble.asm"),
            Self::Dween => unimplemented!("Dween controller fix: tumble"),
        }
    }
}

/// The two raw `PADStatus.stickX` bytes UCF's FETCH_INPUT reads for a port:
/// queue entry `qread - 1` (the sample this tick consumed) and `qread - 3`
/// (two samples earlier).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PadQueueX {
    pub current: i8,
    pub two_ticks_ago: i8,
}

impl PadQueueX {
    /// `sub r3,r3,prev; mullw r3,r3,r3`: the squared change over two ticks.
    fn squared_change(self) -> i32 {
        let change = i32::from(self.current) - i32::from(self.two_ticks_ago);
        change * change
    }
}

/// `cmpwi r3, 0x15F9`: 75 squared. A change must exceed it.
const RAW_CHANGE_SQUARED: i32 = 0x15F9;

/// What the controller-fix codes read for one fighter this tick: its port's
/// setting and hardware queue bytes. The scene refreshes it at the start of
/// every input proc (fp+0x618 names the port; Nana shares her player's).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HardwareInput {
    pub fix: ControllerFix,
    pub stick_x: PadQueueX,
    /// UCF DB's Ice Climbers branch: Popo's new facing, for the scene to
    /// write into Nana's newest follow sample (see [`PartnerTurn`]).
    pub partner_turn: Option<PartnerTurn>,
}

/// UCF DB, Popo only: Nana's newest recorded follow sample (her
/// `cpu.x444` write cursor, Nana fp+0x1ECC) takes Popo's facing (+0x18)
/// and a full stick toward it (+0x6: 0x7F right, 0x80 left).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartnerTurn {
    pub facing: f32,
}

impl PartnerTurn {
    /// `fcmpo f1, 0.0; bgt-`: 0x7F when facing right, 0x80 otherwise.
    pub fn stick_x(self) -> i8 {
        if self.facing > 0.0 {
            0x7F
        } else {
            i8::MIN
        }
    }
}

/// The fighter facts UCF DB reads beside the pad.
pub struct TurnFacts {
    /// cur_anim_frame (+0x894).
    pub animation_frame: f32,
    /// x221F_b4: the player's second fighter (Nana).
    pub secondary: bool,
}

/// `UCF DB.asm` at 0x800C9A44, after ftCo_Turn_IASA's first facing flip
/// (only while mv.co.turn.x2340 is clear): on the turn's second animation
/// frame, a full horizontal stick whose raw X moved more than 75 units in
/// two ticks turns the slow turn into a smash turn. Returns whether it
/// sets x2358 and x2340 (just turned, has turned).
pub fn smash_turn(input: &FighterInput, dash_threshold: f32, facts: &TurnFacts) -> bool {
    /// `.float 2`: the second frame of the turn.
    const SLOW_TURN_FRAME: f32 = 2.0;
    if !input.hardware.fix.dashback() {
        return false;
    }
    // fcmpo cr0,f1,f2; bne Injection_Exit.
    if facts.animation_frame != SLOW_TURN_FRAME {
        return false;
    }
    // fabs f1; lfs f2,0x3C(PlCo) (0.8); fcmpo; blt Injection_Exit.
    if fabsf(input.current.stick.x) < dash_threshold {
        return false;
    }
    // lbz r3,0x670; cmpwi r3,2; bge- Injection_Exit.
    if i32::from(input.horizontal.tilt) >= 2 {
        return false;
    }
    // lbz 0x221F; rlwinm. 0x08: Nana never smash turns here.
    if facts.secondary {
        return false;
    }
    // mullw; cmpwi 0x15F9; ble- Injection_Exit.
    input.hardware.stick_x.squared_change() > RAW_CHANGE_SQUARED
}

/// `UCF SD.asm` at 0x800998A4, the head of ftCo_80099894 (the spot dodge
/// entry that ftCo_80099794 and ftCo_8009980C call once their stick test
/// passed). True when the code returns to the caller's `li r3, 0` instead:
/// the spot dodge predicate fails, and the caller's next check (a shield
/// drop through the platform) can run.
pub fn blocks_spot_dodge(input: &FighterInput, escape_threshold: f32) -> bool {
    /// `.float 80`: HSD's stick scale.
    const STICK_UNITS: f32 = 80.0;
    /// `.long 0x37270000` (about 9.97e-6), subtracted before truncation.
    const TRUNCATION_BIAS: u32 = 0x3727_0000;
    /// `.float -0.8`.
    const DOWN_LIMIT: f32 = -0.8;
    if !input.hardware.fix.shield_drop() {
        return false;
    }
    // "DoSomething": an axis rounded up two stick units.
    let outer = |axis: f32| -> f32 {
        // fabs; fmuls 80; fsubs bias; fctiwz; addi 2.
        let units = fctiwz(fabsf(axis) * STICK_UNITS - f32::from_bits(TRUNCATION_BIAS)) + 2;
        // The 0x43300000 magic-number conversion and fsubs are exact for
        // these small integers; then fdivs by 80.
        units as f32 / STICK_UNITS
    };
    let cstick = input.current.cstick.y;
    // fcmpo cstick.y, PlCo+0x314; ble- EnterSpotdodge (unordered too).
    if cstick.partial_cmp(&escape_threshold) != Some(Ordering::Greater) {
        return false;
    }
    let x = outer(input.current.stick.x);
    let y = outer(input.current.stick.y);
    // fmuls f2,f2,f2 (x); fmuls f1,f1,f1 (y); fadds f1,f1,f2.
    let radius_squared = y * y + x * x;
    // fcmpo 1.0; blt EnterSpotdodge.
    if radius_squared < 1.0 {
        return false;
    }
    // lbz 0x670; cmpwi 3; ble- EnterSpotdodge.
    if i32::from(input.horizontal.tilt) <= 3 {
        return false;
    }
    // fcmpo -0.8, stick.y; bge- EnterSpotdodge.
    DOWN_LIMIT < input.current.stick.y
}

/// ftCo_DamageFall_IASA's wiggle-age test (retail 0x800908EC..0x800908F8,
/// `x670 < PlCo+0x214`), with `UCF Tumble.asm` replacing the compare at
/// 0x800908F4. Only called once the stick passed the 0x210 threshold.
pub fn tumble_wiggle(input: &FighterInput, wiggle_threshold: f32, window: i32) -> bool {
    let tilt = i32::from(input.horizontal.tilt);
    if !input.hardware.fix.tumble() {
        // cmpw r3, r0; bge.
        return tilt < window;
    }
    // cmpwi r3, 1; bne- END: the compare's own result decides (0 wiggles as
    // in retail, where PlCo+0x214 is 1; 2 or more fails).
    if tilt != 1 {
        return tilt < 1;
    }
    // lfs 0x628 (last tick's stick x); fabs; lfs PlCo+0x210; fcmpo; bge END.
    if fabsf(input.previous.stick.x).partial_cmp(&wiggle_threshold) != Some(Ordering::Less) {
        return false;
    }
    // li r4, 0x15F9; cmpw r4, r3: less-than wiggles.
    RAW_CHANGE_SQUARED < input.hardware.stick_x.squared_change()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Stick;

    fn fixed(fix: ControllerFix) -> FighterInput {
        FighterInput {
            hardware: HardwareInput {
                fix,
                ..HardwareInput::default()
            },
            ..FighterInput::default()
        }
    }

    #[test]
    fn names_round_trip() {
        for (fix, name) in ControllerFix::ALL {
            assert_eq!(ControllerFix::from_name(name), Some(fix));
            assert_eq!(fix.name(), name);
        }
        assert_eq!(ControllerFix::from_name("ucf"), None);
    }

    #[test]
    fn smash_turn_needs_a_75_unit_raw_change_on_frame_two() {
        let facts = TurnFacts {
            animation_frame: 2.0,
            secondary: false,
        };
        let mut input = fixed(ControllerFix::Ucf080);
        input.current.stick = Stick { x: -1.0, y: 0.0 };
        input.horizontal.tilt = 1;
        // (current, two ticks ago, smash turn)
        for (current, earlier, expected) in [
            (-80, 0, true),
            (-76, 0, true),
            (-75, 0, false),
            (-127, -50, true),
            (-80, -10, false),
        ] {
            input.hardware.stick_x = PadQueueX {
                current,
                two_ticks_ago: earlier,
            };
            assert_eq!(
                smash_turn(&input, 0.8, &facts),
                expected,
                "{current} {earlier}"
            );
        }
        input.hardware.stick_x = PadQueueX {
            current: -80,
            two_ticks_ago: 0,
        };
        let late = TurnFacts {
            animation_frame: 3.0,
            secondary: false,
        };
        assert!(!smash_turn(&input, 0.8, &late));
        input.horizontal.tilt = 2;
        assert!(!smash_turn(&input, 0.8, &facts));
        input.horizontal.tilt = 1;
        input.hardware.fix = ControllerFix::Off;
        assert!(!smash_turn(&input, 0.8, &facts));
    }

    #[test]
    fn shield_drop_blocks_rim_stick_held_sideways() {
        let mut input = fixed(ControllerFix::Ucf074);
        input.horizontal.tilt = 4;
        // A down-right notch on the rim: 0.7 each way reaches the circle.
        input.current.stick = Stick { x: 0.7, y: -0.7 };
        assert!(blocks_spot_dodge(&input, -0.7));
        // Straight down past -0.8 spot dodges.
        input.current.stick = Stick { x: 0.3, y: -0.8125 };
        assert!(!blocks_spot_dodge(&input, -0.7));
        // Inside the circle spot dodges.
        input.current.stick = Stick { x: 0.5, y: -0.7 };
        assert!(!blocks_spot_dodge(&input, -0.7));
        // Freshly moved sideways spot dodges.
        input.current.stick = Stick { x: 0.7, y: -0.7 };
        input.horizontal.tilt = 3;
        assert!(!blocks_spot_dodge(&input, -0.7));
        // The C-stick always spot dodges.
        input.horizontal.tilt = 4;
        input.current.cstick.y = -0.7;
        assert!(!blocks_spot_dodge(&input, -0.7));
    }

    #[test]
    fn tumble_wiggle_second_tick_needs_the_raw_change() {
        let mut input = fixed(ControllerFix::Ucf080);
        // Retail PlCo: threshold 0.8 (+0x210), window 1 (+0x214).
        for tilt in [0u8, 1, 2] {
            input.horizontal.tilt = tilt;
            input.hardware.fix = ControllerFix::Off;
            assert_eq!(tumble_wiggle(&input, 0.8, 1), tilt == 0);
            input.hardware.fix = ControllerFix::Ucf074;
            assert_eq!(tumble_wiggle(&input, 0.8, 1), tilt == 0);
        }
        input.hardware.fix = ControllerFix::Ucf080;
        input.horizontal.tilt = 0;
        assert!(tumble_wiggle(&input, 0.8, 1));
        input.horizontal.tilt = 2;
        assert!(!tumble_wiggle(&input, 0.8, 1));
        input.horizontal.tilt = 1;
        input.hardware.stick_x = PadQueueX {
            current: 80,
            two_ticks_ago: 0,
        };
        assert!(tumble_wiggle(&input, 0.8, 1));
        input.previous.stick.x = 0.8;
        assert!(!tumble_wiggle(&input, 0.8, 1));
        input.previous.stick.x = 0.5;
        input.hardware.stick_x.two_ticks_ago = 10;
        assert!(!tumble_wiggle(&input, 0.8, 1));
    }
}
