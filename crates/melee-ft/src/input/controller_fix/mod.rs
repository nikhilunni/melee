//! Controller-fix Gecko codes: the Universal Controller Fix (UCF) that
//! tournament setups and Slippi run on top of retail NTSC 1.02.
//!
//! Ported from the Slippi `slippi-ssbm-asm` repository (GPL-3; logic only,
//! no source copied). Each hook cites its source file and Gecko injection
//! address, the retail instruction the code replaces:
//!
//! | Version | File | Injection |
//! |---|---|---|
//! | 0.73 | `Binary/UCF/Ucf0.73Beta.bin` (machine code) | 0x800C9A44, 0x800998A4 |
//! | 0.74, 0.8 | `External/UCF 0.74/UCF DB.asm`, `External/UCF 0.8/Logic/UCF DB.asm` | 0x800C9A44 (ftCo_Turn_IASA) |
//! | 0.74, 0.8 | `External/UCF 0.74/UCF SD.asm`, `External/UCF 0.8/Logic/UCF SD.asm` | 0x800998A4 (ftCo_80099894) |
//! | 0.8 | `External/UCF 0.8/Logic/UCF Tumble.asm` | 0x800908F4 (ftCo_DamageFall_IASA) |
//! | 0.84 | `External/UCF 0.84/UCF/UCF Pad Buffer + 1.0 Cardinals.asm` | 0x8006B460 (Fighter_Spaghetti_8006AD10) |
//! | 0.84 | `.../UCF Dashback.asm` | 0x800C9A44 |
//! | 0.84 | `.../UCF Shield Drop.asm`, `UCF Shield Drop Extended.asm` | 0x800998A4, 0x8009A0B8 (ftCo_8009A080) |
//! | 0.84 | `.../UCF Tumble.asm` | 0x800908F4 |
//! | 0.84 | `.../UCF SDI.asm`, `UCF Shield SDI.asm` | 0x8008E54C (ftCo_Damage_OnEveryHitlag), 0x80093294 (ftCo_80093240) |
//! | 0.84 | `.../UCF DBOOC SquatRv Fix.asm` | 0x800D65EC (ftCo_SquatRv_CheckInput) |
//!
//! The 0.74 and 0.8 dashback and shield-drop codes are the same program
//! (the 0.8 files only add version tables and comments); 0.8 adds the
//! tumble wiggle. 0.73 and 0.84 are distributed as machine code only; their
//! hooks here follow the disassembled instructions. 0.73's dashback wants the
//! stick toward the turn (0.74: a full stick either way); its shield drop
//! computes what 0.74's does. The CSS indicator text (0x802662D0)
//! draws on the character select screen and has no effect on a match.
//!
//! The 0.73/0.74/0.8 codes read the hardware pad queue (HSD_PadLibData.queue,
//! retail 0x8046B108, read index at HSD_PadLibData+1, 0x804C1F78): the
//! signed, origin-adjusted `PADStatus.stickX` byte of the sample this tick
//! consumed and of the sample two ticks earlier, before HSD clamps the stick
//! to the 80-unit circle ([`PadQueueX`]). 0.84 keeps its own four-sample ring
//! per port instead ([`PadBuffer`]), fed from the consumed entry's sticks
//! ([`RawSticks`]).
use super::state::FighterInput;
use melee_types::mp::SurfaceData;

mod ucf073;
mod ucf08;
mod ucf084;

pub use ucf084::PadBuffer;

/// A player's controller-fix Gecko code. Retail is `Off`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ControllerFix {
    #[default]
    Off,
    /// UCF 0.73 beta (2018): dashback (stick toward the turn) and shield
    /// drop.
    Ucf073,
    /// UCF 0.74 (2019): dashback and shield drop.
    Ucf074,
    /// UCF 0.8 (2021): 0.74 plus the tumble wiggle.
    Ucf080,
    /// UCF 0.84 (2024): its own pad buffer, 1.0 cardinals, dashback, shield
    /// drop, tumble, SDI, shield SDI and the squat-release fix.
    Ucf084,
    /// Slippi's "Dween" dashback fix: not ported.
    Dween,
}

impl ControllerFix {
    /// Every setting with its scenario/CLI name, in declaration order.
    pub const ALL: [(Self, &'static str); 6] = [
        (Self::Off, "off"),
        (Self::Ucf073, "ucf-0.73"),
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
            Self::Off | Self::Ucf073 | Self::Ucf074 | Self::Ucf080 | Self::Ucf084 => None,
            Self::Dween => Some("the Dween dashback fix is not ported"),
        }
    }

    /// The dashback and shield-drop program this setting installs.
    fn program(self) -> Option<Program> {
        match self {
            Self::Off => None,
            Self::Ucf073 => Some(Program::Ucf073),
            Self::Ucf074 | Self::Ucf080 => Some(Program::Ucf08),
            Self::Ucf084 => Some(Program::Ucf084),
            Self::Dween => unimplemented!("Dween controller fix"),
        }
    }

    fn ucf084(self) -> bool {
        self.program() == Some(Program::Ucf084)
    }
}

/// The three dashback and shield-drop programs.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Program {
    /// `Ucf0.73Beta.bin`.
    Ucf073,
    /// `UCF DB.asm`, `UCF SD.asm` (0.74 and 0.8).
    Ucf08,
    Ucf084,
}

/// The two raw `PADStatus.stickX` bytes UCF 0.73/0.74/0.8's FETCH_INPUT reads for
/// a port: queue entry `qread - 1` (the sample this tick consumed) and
/// `qread - 3` (two samples earlier).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PadQueueX {
    pub current: i8,
    pub two_ticks_ago: i8,
}

/// The consumed queue entry's signed `PADStatus` sticks (main X/Y at +2/+3,
/// C-stick X/Y at +4/+5), before HSD clamps them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RawSticks {
    pub stick: [i8; 2],
    pub cstick: [i8; 2],
}

/// The squared change of a signed stick byte (`sub; mullw`).
fn squared_change(now: i8, before: i8) -> i32 {
    let change = i32::from(now) - i32::from(before);
    change * change
}

/// `cmpwi 0x15F9`: 75 squared, the dashback and tumble raw-change bound.
const RAW_CHANGE_SQUARED: i32 = 0x15F9;

/// What the controller-fix codes read for one fighter: its port's setting
/// and hardware bytes, and 0.84's ring for the port. The scene refreshes it
/// at the start of every input proc (fp+0x618 names the port; Nana shares
/// her player's) and stores the ring back after it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HardwareInput {
    pub fix: ControllerFix,
    pub queue_x: PadQueueX,
    pub raw: RawSticks,
    pub buffer: PadBuffer,
    /// A smash turn's write into the player's second fighter's newest follow
    /// sample, for the scene to apply (see [`PartnerTurn`]).
    pub partner_turn: Option<PartnerTurn>,
}

/// UCF DB's second-fighter branch: Player_GetEntityAtIndex(slot, 1)'s
/// newest follow sample (its `cpu.x444` write cursor, fp+0x1ECC) takes the
/// turner's facing (+0x18) and a full stick toward it (+0x6).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartnerTurn {
    pub facing: f32,
    pub stick_x: i8,
}

/// The fighter facts UCF DB reads beside the pad.
pub struct TurnFacts {
    /// cur_anim_frame (+0x894).
    pub animation_frame: f32,
    /// x3E4_fighterCmdScript.frame_count (+0x3E8), which 0.73 reads.
    pub script_frame: f32,
    /// mv.co.turn.facing_after (+0x2344), which 0.73 reads.
    pub facing_after: f32,
    /// facing_dir (+0x2C) after ftCo_Turn_IASA's flip.
    pub facing: f32,
    /// x221F_b4: the player's second fighter (Nana).
    pub secondary: bool,
    /// fp->kind == FTKIND_POPO (0.74/0.8 only write Nana for Popo).
    pub popo: bool,
}

/// UCF DB at 0x800C9A44, after ftCo_Turn_IASA's first facing flip (only
/// while mv.co.turn.x2340 is clear): `None` leaves the slow turn; `Some`
/// sets x2358 and x2340 (just turned, has turned), with the partner write
/// the code makes, if any.
pub fn smash_turn(
    input: &FighterInput,
    dash_threshold: f32,
    facts: &TurnFacts,
) -> Option<Option<PartnerTurn>> {
    match input.hardware.fix.program()? {
        Program::Ucf073 => ucf073::smash_turn(input, dash_threshold, facts),
        Program::Ucf08 => ucf08::smash_turn(input, dash_threshold, facts),
        Program::Ucf084 => ucf084::smash_turn(input, dash_threshold, facts),
    }
}

/// The spot-dodge entry's facts beyond the pad (0.84 reads the floor).
pub struct SpotDodgeFacts {
    /// PlCo+0x314: the spot dodge stick threshold.
    pub escape_threshold: f32,
    /// PlCo+0x2C: the fast walk's stick threshold (0.8), which 0.73 reads
    /// as its downward limit.
    pub walk_fast_threshold: f32,
    /// PlCo+0x320: the roll's horizontal tap window.
    pub roll_window: i32,
    /// fp+0x6F0 CollData floor (+0x83C index, +0x840 flags).
    pub floor: SurfaceData,
}

/// UCF SD at 0x800998A4, the head of ftCo_80099894 (the spot dodge entry that
/// ftCo_80099794 and ftCo_8009980C call once their stick test passed). True
/// when the code returns to the caller's `li r3, 0` instead: the spot dodge
/// predicate fails, and the caller's next check (a shield drop through the
/// platform) can run.
pub fn blocks_spot_dodge(input: &FighterInput, facts: &SpotDodgeFacts) -> bool {
    match input.hardware.fix.program() {
        None => false,
        Some(Program::Ucf073) => {
            ucf073::blocks_spot_dodge(input, facts.escape_threshold, facts.walk_fast_threshold)
        }
        Some(Program::Ucf08) => ucf08::blocks_spot_dodge(input, facts.escape_threshold),
        Some(Program::Ucf084) => ucf084::blocks_spot_dodge(input, facts),
    }
}

/// ftCo_DamageFall_IASA's wiggle-age test (retail 0x800908EC..0x800908F8,
/// `x670 < PlCo+0x214`), which the tumble codes replace at 0x800908F4. Only
/// called once the stick passed the 0x210 threshold.
pub fn tumble_wiggle(input: &FighterInput, wiggle_threshold: f32, window: i32) -> bool {
    let tilt = i32::from(input.horizontal.tilt);
    match input.hardware.fix {
        ControllerFix::Ucf080 => ucf08::tumble_wiggle(input, wiggle_threshold),
        ControllerFix::Ucf084 => ucf084::tumble_wiggle(input, wiggle_threshold),
        // cmpw r3, r0; bge.
        _ => tilt < window,
    }
}

/// 0.84's pad buffer, at 0x8006B460 in the input proc (after the stick
/// timers): record the port's raw sticks, snap cardinals to 1.0 and count
/// the shield-drop rim ticks. `human`: ftCo_800A2040 is false.
/// `cardinal_exempt`: fp->kind 19 (Zelda) in motion 349.
pub fn update_pad_buffer(input: &mut FighterInput, human: bool, cardinal_exempt: bool) {
    if input.hardware.fix.ucf084() && human {
        ucf084::update_pad_buffer(input, cardinal_exempt);
    }
}

/// ftCo_Damage_OnEveryHitlag's vertical tap test (0x8008E548..0x8008E550,
/// `x671 < PlCo+0x4B4`), which 0.84's SDI code replaces at 0x8008E54C.
pub fn sdi_vertical_tap(input: &FighterInput, window: i32, minimum_stick: f32) -> bool {
    let vertical = i32::from(input.vertical.tilt) < window;
    if input.hardware.fix.ucf084() {
        vertical || ucf084::sdi_tap(input, minimum_stick)
    } else {
        vertical
    }
}

/// ftCo_80093240's horizontal tap test (0x8009328C..0x80093298,
/// `x670 < PlCo+0x4B4`), which 0.84's shield SDI replaces at 0x80093294.
pub fn shield_sdi_tap(input: &FighterInput, window: i32, minimum_stick: f32) -> bool {
    let horizontal = i32::from(input.horizontal.tilt) < window;
    if input.hardware.fix.ucf084() {
        horizontal || ucf084::shield_sdi_tap(input, minimum_stick)
    } else {
        horizontal
    }
}

/// ftCo_8009A080's stick test (`stick.y <= -PlCo+0x464`, the `cror` at
/// 0x8009A0B8), which 0.84's shield drop extension widens.
pub fn platform_drop_stick(input: &FighterInput, threshold: f32) -> bool {
    let down = input.current.stick.y <= -threshold;
    if input.hardware.fix.ucf084() {
        down || ucf084::platform_drop_rim(input)
    } else {
        down
    }
}

/// ftCo_SquatRv_CheckInput's release threshold (PlCo+0x94, loaded at
/// 0x800D65EC), which 0.84 lowers for a rim stick.
pub fn squat_release_threshold(input: &FighterInput, threshold: f32) -> f32 {
    if input.hardware.fix.ucf084() {
        ucf084::squat_release_threshold(input, threshold)
    } else {
        threshold
    }
}

#[cfg(test)]
mod tests;
