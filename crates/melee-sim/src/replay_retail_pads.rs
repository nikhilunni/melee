//! The raw pad retail must be fed for its fighter to read what a replay's
//! Pre Frame recorded (`replay --retail-inputs`, docs/SLIPPI.md "A replay
//! played back on retail").
//!
//! Pre Frame holds the fighter's side of the pad: the dead-zoned sticks
//! (Fighter +0x620.. +0x63C), the held word with HSD's stick-direction bits
//! (+0x65C), the physical buttons and analog triggers, and of the raw
//! `PADStatus` only the main stick's X before Slippi 3.15 (what UCF reads from
//! the pad queue). The raw bytes are found, not derived: a candidate is taken
//! through HSD's own clamp and direction test (HSD_PadClampCheck3 0x80376E90,
//! HSD_PadADConvertCheck1 0x803771D4) and the fighter's dead zone, and must
//! give every recorded field. A dead-zoned axis reads zero for 45 raw values;
//! the direction bits say which side of HSD's 30-unit threshold the stick was
//! on, so a search that ignored them fed retail a pad whose held word was not
//! the console's.
use anyhow::{bail, ensure, Result};
use melee_ft::input::pad::{normalize_stick, PadSample, STICK_DIRECTIONS};
use slp::cold::ControllerFrame;

/// PlCo's stick dead zone (ftCommonData x0/x4): Pre Frame's joystick is
/// Fighter.input's, zero at or inside it.
const STICK_DEAD_ZONE: f32 = 0.275;
/// HSD's analog trigger scale (HSD_PadScale): raw 140 is 1.0.
const TRIGGER_MAX: f32 = 140.0;
/// HSD_PadADConvert's bits for the main stick and for the C-stick.
const MAIN_DIRECTIONS: u32 = 0x000F_0000;
const C_DIRECTIONS: u32 = 0x00F0_0000;
/// The first harness frame whose pad the fighters read: the countdown's GO
/// releases input at Slippi frame -39 (docs/SLIPPI.md, "Cold input release").
pub const INPUT_RELEASE_FRAME: u64 = 84;
/// `PAD_ERR_TRANSFER`, the one error HSD_PadRenewMasterStatus answers by
/// keeping the last status (controller.c:386, 0x803778B0).
pub const PAD_ERR_TRANSFER: i8 = -3;

/// How a tick's raw pad stands to its Pre Frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reading {
    /// The pad reads as every recorded field.
    Read,
    /// The recorded raw X is not the recorded stick's, and the stick,
    /// buttons and triggers are the tick before's: the console's poll
    /// failed with `PAD_ERR_TRANSFER`. The SDK zeroes such an entry and HSD
    /// keeps its last status, so the entry is fed as it was (raw X as
    /// recorded, the rest zero, `err` -3) and retail keeps the pad too.
    TransferError,
    /// Input is frozen (before GO) and no pad reads as the recorded stick:
    /// there Pre Frame's stick need not be a pad's (-127/128 and 1.0 without
    /// a direction bit occur). The pad keeps what HSD's side records, the
    /// raw X and the direction bits; nothing reads it.
    Frozen,
}

/// One tick's raw `PADStatus` values for the tick input clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetailPad {
    pub button: u16,
    pub stick: [i8; 2],
    pub cstick: [i8; 2],
    pub triggers: [u8; 2],
    pub reading: Reading,
}

#[derive(Clone, Copy)]
enum Which {
    Main,
    C,
}

/// What the fighter and HSD read from one stick's raw bytes: the dead-zoned
/// pair and the stick's direction bits.
fn reads(raw: [i8; 2], which: Which) -> ([f32; 2], u32) {
    let dead = |v: f32| if v.abs() <= STICK_DEAD_ZONE { 0.0 } else { v };
    let clamped = normalize_stick(raw[0], raw[1]);
    let (pad, mask) = match which {
        Which::Main => (
            PadSample {
                stick: clamped,
                ..PadSample::default()
            },
            MAIN_DIRECTIONS,
        ),
        Which::C => (
            PadSample {
                cstick: clamped,
                ..PadSample::default()
            },
            C_DIRECTIONS,
        ),
    };
    debug_assert_eq!(STICK_DIRECTIONS, MAIN_DIRECTIONS | C_DIRECTIONS);
    (
        [dead(clamped.x), dead(clamped.y)],
        pad.with_stick_directions().buttons.0 & mask,
    )
}

/// The raw pair nearest `near` with these direction bits that reads as
/// `processed` (any stick when `None`), its X held to `x` when the replay
/// recorded it.
fn solve(
    processed: Option<[f32; 2]>,
    directions: u32,
    which: Which,
    x: Option<i8>,
    near: [i8; 2],
) -> Option<[i8; 2]> {
    let fits = |raw: [i8; 2]| {
        let (stick, bits) = reads(raw, which);
        bits == directions && processed.is_none_or(|processed| processed == stick)
    };
    if x.is_none_or(|x| x == near[0]) && fits(near) {
        return Some(near);
    }
    let bytes = || (-128i16..=127).map(|v| v as i8);
    let xs: Vec<i8> = x.map_or_else(|| bytes().collect(), |x| vec![x]);
    xs.into_iter()
        .flat_map(|x| bytes().map(move |y| [x, y]))
        .filter(|&raw| fits(raw))
        .min_by_key(|raw| {
            (i16::from(raw[0]) - i16::from(near[0])).abs()
                + (i16::from(raw[1]) - i16::from(near[1])).abs()
        })
}

/// Whether two Pre Frames of a port hold one pad status: what HSD keeps
/// through a failed poll.
fn same_status(a: &ControllerFrame, b: &ControllerFrame) -> bool {
    let hsd = |frame: &ControllerFrame| {
        (
            frame.buttons_physical,
            frame.buttons_processed & STICK_DIRECTIONS,
            frame.stick,
            frame.cstick,
            frame.triggers,
        )
    };
    hsd(a) == hsd(b)
}

/// The raw pad for one recorded Pre Frame; `previous` is the same port's
/// Pre Frame of the tick before. A pad that cannot be made to read as the
/// recording is an error unless the recording itself says why
/// ([`Reading`]): retail would not be playing the replay's inputs.
pub fn reconstruct(
    input: &ControllerFrame,
    previous: Option<&ControllerFrame>,
) -> Result<RetailPad> {
    let tick = input.frame + 1;
    let frozen = input.frame < INPUT_RELEASE_FRAME;
    let derived = crate::inputs::replay_pad(input)?.raw_sticks();
    let main_directions = input.buttons_processed & MAIN_DIRECTIONS;
    let c_directions = input.buttons_processed & C_DIRECTIONS;
    let x = input.raw_stick.map(|raw| raw[0]).or(input.raw_stick_x);
    let near = input
        .raw_stick
        .unwrap_or([x.unwrap_or(derived.stick[0]), derived.stick[1]]);
    let read = match input.raw_stick {
        Some(raw) => (reads(raw, Which::Main) == (input.stick, main_directions)).then_some(raw),
        None => solve(Some(input.stick), main_directions, Which::Main, x, near),
    };
    let mut reading = Reading::Read;
    let stick = match read {
        Some(raw) => raw,
        None if previous.is_some_and(|previous| same_status(previous, input)) => {
            return Ok(RetailPad {
                button: 0,
                stick: [near[0], input.raw_stick.map_or(0, |raw| raw[1])],
                cstick: input.raw_cstick.unwrap_or([0, 0]),
                triggers: [0, 0],
                reading: Reading::TransferError,
            });
        }
        None if frozen => {
            reading = Reading::Frozen;
            solve(None, main_directions, Which::Main, x, near).unwrap_or(near)
        }
        None => bail!(
            "tick {tick} port {}: no raw stick reads as {:?} with direction bits \
             {main_directions:#x} and raw X {x:?}, and the pad is not the tick before's \
             (a failed poll): retail cannot be fed this replay's pad (a port running Dween's \
             fix records the code's output, not the pad)",
            input.port,
            input.stick
        ),
    };
    let cstick = match input.raw_cstick {
        Some(raw) => raw,
        None => match solve(
            Some(input.cstick),
            c_directions,
            Which::C,
            None,
            derived.cstick,
        ) {
            Some(raw) => raw,
            None if frozen => {
                reading = Reading::Frozen;
                solve(None, c_directions, Which::C, None, derived.cstick).unwrap_or(derived.cstick)
            }
            None => bail!(
                "tick {tick} port {}: no raw C-stick reads as {:?} with direction bits \
                 {c_directions:#x}",
                input.port,
                input.cstick
            ),
        },
    };
    let trigger = |value: f32| -> Result<u8> {
        let raw = (value * TRIGGER_MAX).round();
        ensure!(
            raw / TRIGGER_MAX == value,
            "tick {tick} port {}: trigger {value} is not a multiple of 1/140",
            input.port
        );
        Ok(raw as u8)
    };
    Ok(RetailPad {
        button: input.buttons_physical,
        stick,
        cstick,
        triggers: [trigger(input.triggers[0])?, trigger(input.triggers[1])?],
        reading,
    })
}

/// The direction bits HSD gives a pad's sticks: with the physical buttons,
/// the part of the held word a pad decides.
pub fn direction_bits(pad: &RetailPad) -> u32 {
    reads(pad.stick, Which::Main).1 | reads(pad.cstick, Which::C).1
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A Pre Frame well after GO.
    fn frame(
        stick: [f32; 2],
        cstick: [f32; 2],
        directions: u32,
        raw_x: Option<i8>,
    ) -> ControllerFrame {
        ControllerFrame {
            frame: 300,
            port: 1,
            buttons_physical: 0x0200,
            buttons_processed: directions | 0x0200,
            stick,
            cstick,
            trigger: 0.0,
            triggers: [0.0, 3.0 / 140.0],
            raw_stick: None,
            raw_cstick: None,
            raw_stick_x: raw_x,
        }
    }

    #[test]
    fn a_pad_already_right_is_the_derived_one() {
        // (0.7375, -0.6625): raw (59, -53), length 79.3, the right bit.
        let input = frame([0.7375, -0.6625], [0.0, 0.0], 0x0008_0000, Some(59));
        let pad = reconstruct(&input, None).unwrap();
        assert_eq!(
            pad,
            RetailPad {
                button: 0x0200,
                stick: [59, -53],
                cstick: [0, 0],
                triggers: [0, 3],
                reading: Reading::Read,
            }
        );
    }

    #[test]
    fn a_dead_zoned_axis_is_chosen_by_the_direction_bits() {
        // `PEACH/15_52_33`, tick 3011: the C-stick reads (0, 0.3375) with its
        // up bit set. Raw (0, 27) is under HSD's threshold of 30, so the
        // console's x was at least 14 inside the dead zone.
        let with_bit = reconstruct(
            &frame([0.0, 0.0], [0.0, 0.3375], 0x0010_0000, Some(0)),
            None,
        )
        .unwrap();
        assert_eq!(with_bit.cstick[1], 27);
        assert!((14..=22).contains(&with_bit.cstick[0].abs()));
        assert_eq!(direction_bits(&with_bit), 0x0010_0000);
        let without = reconstruct(&frame([0.0, 0.0], [0.0, 0.3375], 0, Some(0)), None).unwrap();
        assert_eq!(without.cstick, [0, 27]);
        // The main stick's Y, beside a recorded X of -27 with the left bit.
        let main = reconstruct(
            &frame([-0.3375, 0.0], [0.0, 0.0], 0x0004_0000, Some(-27)),
            None,
        )
        .unwrap();
        assert_eq!(main.stick[0], -27);
        assert!((14..=22).contains(&main.stick[1].abs()));
        assert_eq!(direction_bits(&main), 0x0004_0000);
        assert_eq!(main.reading, Reading::Read);
    }

    #[test]
    fn a_raw_x_beside_the_last_ticks_pad_is_a_failed_poll() {
        // `DOC/Doc vs Fox [BF]`, tick 2: raw X 0 beside (0.7625, 0.625), the
        // stick of tick 1.
        let held = frame([0.7625, 0.625], [0.0, 0.0], 0x0008_0000, Some(0));
        let before = ControllerFrame {
            raw_stick_x: Some(61),
            ..held.clone()
        };
        let pad = reconstruct(&held, Some(&before)).unwrap();
        assert_eq!(
            pad,
            RetailPad {
                button: 0,
                stick: [0, 0],
                cstick: [0, 0],
                triggers: [0, 0],
                reading: Reading::TransferError,
            }
        );
        // Another pad than the tick before's is no failed poll.
        let moved = ControllerFrame {
            stick: [0.75, 0.625],
            ..before
        };
        let refused = reconstruct(&held, Some(&moved)).unwrap_err().to_string();
        assert!(
            refused.contains("tick 301 port 1: no raw stick reads as"),
            "{refused}"
        );
        assert!(reconstruct(&held, None).is_err());
    }

    #[test]
    fn a_stick_no_pad_gives_is_fed_from_the_pads_side_while_input_is_frozen() {
        // `19_18_23 Luigi + Fox (DL)`, tick 65: -127/128, no direction bit.
        let mut input = frame([-0.992_187_5, 0.0], [0.0, 0.0], 0, Some(-4));
        assert!(reconstruct(&input, None).is_err());
        input.frame = INPUT_RELEASE_FRAME - 1;
        let pad = reconstruct(&input, None).unwrap();
        assert_eq!((pad.stick, pad.reading), ([-4, 0], Reading::Frozen));
        input.frame = INPUT_RELEASE_FRAME;
        assert!(reconstruct(&input, None).is_err());
    }

    #[test]
    fn direction_bits_no_pad_gives_are_refused() {
        // A neutral stick with the up bit beside a raw X of zero; a C-stick
        // both up and down.
        assert!(reconstruct(&frame([0.0, 0.0], [0.0, 0.0], 0x0001_0000, Some(0)), None).is_err());
        let refused = reconstruct(&frame([0.0, 0.0], [0.0, 0.0], 0x0030_0000, Some(0)), None)
            .unwrap_err()
            .to_string();
        assert!(refused.contains("no raw C-stick reads as"), "{refused}");
    }
}
