//! UCF 0.84 (2024), `External/UCF 0.84/UCF/*.asm`: machine code only, ported
//! from its disassembly. Every 0.84 hook reads the pad buffer this module's
//! [`update_pad_buffer`] keeps, not HSD's queue.
use super::{squared_change, PartnerTurn, SpotDodgeFacts, TurnFacts, RAW_CHANGE_SQUARED};
use crate::input::{state::FighterInput, Stick};
use gekko_math::{
    fma::{fmadds, fmsubs},
    msl::{fabsf, fctiwz},
};
use melee_types::mp::line_flag;
use std::cmp::Ordering;

/// One port's 12-byte slot in the pad buffer code's data (the code's own
/// `bl` skips 48 bytes: four ports). Zero when the code is installed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PadBuffer {
    /// +0..+7: the last four main-stick samples, raw X and Y.
    samples: [[i8; 2]; 4],
    /// +8: the newest sample's slot (`addi 1; clrlwi 30` before the store).
    cursor: u8,
    /// +9: ticks the stick has sat on the lower rim since a fast downward
    /// flick; read by the shield drop extension (0x8009A0B8).
    rim_ticks: u8,
}

impl PadBuffer {
    fn newest(&self) -> [i8; 2] {
        self.samples[usize::from(self.cursor)]
    }
    /// `addi -2; clrlwi 30`: two samples before the newest.
    fn two_back(&self) -> [i8; 2] {
        self.samples[((i32::from(self.cursor) - 2) & 3) as usize]
    }
    /// The squared change of one axis (0: X, 1: Y) over two samples.
    fn axis_change(&self, axis: usize) -> i32 {
        squared_change(self.newest()[axis], self.two_back()[axis])
    }
}

/// `.float 80`: HSD's stick scale.
const STICK_UNITS: f32 = 80.0;
/// `.long 0x38D1B717` (1e-4), subtracted by `fmsubs` before truncation.
const TRUNCATION_BIAS: u32 = 0x38D1_B717;
/// `cmpwi 6400`: 80 squared, the rim in stick units.
const RIM_UNITS_SQUARED: i32 = 6400;
/// `cmpwi 3844`: 62 squared, the SDI raw-change bound.
const SDI_CHANGE_SQUARED: i32 = 0xF04;
/// `cmpwi 1936`: 44 squared, the downward flick that starts the rim count.
const FLICK_CHANGE_SQUARED: i32 = 0x790;

/// The shared helper (`fabs; fmsubs 80, 1e-4; fctiwz; addi 2`): an axis in
/// stick units rounded up two.
fn rim_units(axis: f32) -> i32 {
    fctiwz(fmsubs(
        fabsf(axis),
        STICK_UNITS,
        f32::from_bits(TRUNCATION_BIAS),
    )) + 2
}

/// Whether the stick lies past the 80-unit rim, x and y each rounded up.
fn on_rim(stick: Stick) -> bool {
    let (x, y) = (rim_units(stick.x), rim_units(stick.y));
    x * x + y * y > RIM_UNITS_SQUARED
}

/// `UCF Pad Buffer + 1.0 Cardinals.asm` at 0x8006B460 (its first
/// instruction, `lbz r3, 0x678(r31)`, is the retail one it replaces).
pub(super) fn update_pad_buffer(input: &mut FighterInput, cardinal_exempt: bool) {
    let raw = input.hardware.raw;
    let buffer = &mut input.hardware.buffer;
    buffer.cursor = (buffer.cursor + 1) & 3;
    buffer.samples[usize::from(buffer.cursor)] = raw.stick;
    // lwz 4 == 19 && lwz 0x10 == 349 skips both sticks.
    if !cardinal_exempt {
        snap_cardinal(raw.stick, &mut input.current.stick);
        snap_cardinal(raw.cstick, &mut input.current.cstick);
    }
    input.hardware.buffer.rim_ticks = next_rim_ticks(input);
}

/// The code's cardinal helper (+0x40): a raw axis at 80 or beyond with the
/// other within 6 becomes exactly +/-1.0 and 0.0.
fn snap_cardinal([x, y]: [i8; 2], stick: &mut Stick) {
    // addi 79; clrlwi 24; cmplwi 158: |axis| >= 80.
    let full = |axis: i8| (i32::from(axis) + 79) & 0xFF > 158;
    // addi 6; clrlwi 24; cmplwi 12: |axis| <= 6.
    let centered = |axis: i8| (i32::from(axis) + 6) & 0xFF <= 12;
    // rlwinm 0,0,0 (the sign bit); xoris 0x3F80.
    let unit = |axis: i8| f32::from_bits((i32::from(axis) as u32 & 0x8000_0000) ^ 0x3F80_0000);
    if full(x) {
        if centered(y) {
            *stick = Stick { x: unit(x), y: 0.0 };
        }
    } else if full(y) && centered(x) {
        *stick = Stick { x: 0.0, y: unit(y) };
    }
}

/// The rim counter's next value (+0x1A0..+0x274).
fn next_rim_ticks(input: &FighterInput) -> u8 {
    /// `.long 0xBF1C0000` (-0.609375).
    const LOWER_RIM_Y: f32 = -0.609_375;
    let stick = input.current.stick;
    let buffer = &input.hardware.buffer;
    // fcmpu y, -0.609375; bgt: reset.
    if stick.y > LOWER_RIM_Y || !on_rim(stick) {
        return 0;
    }
    if buffer.rim_ticks != 0 {
        // A running count grows (a byte: it wraps).
        return buffer.rim_ticks.wrapping_add(1);
    }
    // lbz 0x671; cmplwi 1; bgt: a slow vertical tilt never starts it.
    if input.vertical.tilt > 1 {
        return 0;
    }
    u8::from(buffer.axis_change(1) > FLICK_CHANGE_SQUARED)
}

/// `UCF Dashback.asm` at 0x800C9A44.
pub(super) fn smash_turn(
    input: &FighterInput,
    dash_threshold: f32,
    facts: &TurnFacts,
) -> Option<Option<PartnerTurn>> {
    /// `lis 0x4000`: 2.0's bits, compared as an integer.
    const SLOW_TURN_FRAME_BITS: u32 = 0x4000_0000;
    // andi. 0x221F, 8: never Nana.
    if facts.secondary || facts.animation_frame.to_bits() != SLOW_TURN_FRAME_BITS {
        return None;
    }
    // fmuls facing, stick.x; fcmpu PlCo+0x3C; blt.
    if facts.facing * input.current.stick.x < dash_threshold {
        return None;
    }
    // lbz 0x670; cmplwi 1; bgt.
    if input.horizontal.tilt > 1 {
        return None;
    }
    if input.hardware.buffer.axis_change(0) <= RAW_CHANGE_SQUARED {
        return None;
    }
    // Player_GetEntityAtIndex(slot, 1) for any kind; its newest follow
    // sample takes the facing's bits and (bits >> 31) + 127.
    let sign = (facts.facing.to_bits() >> 31) as u8;
    Some(Some(PartnerTurn {
        facing: facts.facing,
        stick_x: (sign + 0x7F) as i8,
    }))
}

/// `UCF Shield Drop.asm` at 0x800998A4.
pub(super) fn blocks_spot_dodge(input: &FighterInput, facts: &SpotDodgeFacts) -> bool {
    /// `.long 0xBF4CCCCD` (-0.8).
    const DOWN_LIMIT: f32 = -0.8;
    let current = &input.current;
    // fcmpo cstick.y, PlCo+0x314; ble.
    if current.cstick.y.partial_cmp(&facts.escape_threshold) != Some(Ordering::Greater) {
        return false;
    }
    // lbz 0x670; lwz PlCo+0x320; cmpw; blt: a roll's fresh tap.
    if i32::from(input.horizontal.tilt) < facts.roll_window {
        return false;
    }
    // fcmpo stick.y, -0.8; ble.
    if current.stick.y.partial_cmp(&DOWN_LIMIT) != Some(Ordering::Greater) {
        return false;
    }
    // lwz 0x83C == -1; lwz 0x840 & 0x100: on a platform.
    if facts.floor.index == -1 || facts.floor.flags & line_flag::PLATFORM == 0 {
        return false;
    }
    on_rim(current.stick)
}

/// `UCF Tumble.asm` at 0x800908F4.
pub(super) fn tumble_wiggle(input: &FighterInput, wiggle_threshold: f32) -> bool {
    let tilt = i32::from(input.horizontal.tilt);
    // cmpwi r3, 1; bne: the compare decides.
    if tilt != 1 {
        return tilt < 1;
    }
    // fabs 0x628; fcmpu PlCo+0x210; bge: fail.
    if fabsf(input.previous.stick.x).partial_cmp(&wiggle_threshold) != Some(Ordering::Less) {
        return false;
    }
    input.hardware.buffer.axis_change(0) > RAW_CHANGE_SQUARED
}

/// `UCF SDI.asm` at 0x8008E54C, once the vertical tap failed: a stick that
/// jumped from inside the SDI radius by more than 62 raw units.
pub(super) fn sdi_tap(input: &FighterInput, minimum_stick: f32) -> bool {
    // lbz 0x673; cmplwi 1; ble (continue); else lbz 0x674 > 1 fails.
    if input.horizontal.held > 1 && input.vertical.held > 1 {
        return false;
    }
    let previous = input.previous.stick;
    // fmuls y,y; fmadds x,x,y2; fmuls min,min; fcmpu; ble: fail.
    let previous_squared = fmadds(previous.x, previous.x, previous.y * previous.y);
    if (minimum_stick * minimum_stick).partial_cmp(&previous_squared) != Some(Ordering::Greater) {
        return false;
    }
    let buffer = &input.hardware.buffer;
    buffer.axis_change(0) + buffer.axis_change(1) > SDI_CHANGE_SQUARED
}

/// `UCF Shield SDI.asm` at 0x80093294, once the horizontal tap failed.
pub(super) fn shield_sdi_tap(input: &FighterInput, minimum_stick: f32) -> bool {
    // lbz 0x673; cmplwi 1; bgt: fail.
    if input.horizontal.held > 1 {
        return false;
    }
    // lfs 0x628 (signed); fcmpu PlCo+0x4B0; bge: fail.
    if input.previous.stick.x.partial_cmp(&minimum_stick) != Some(Ordering::Less) {
        return false;
    }
    input.hardware.buffer.axis_change(0) > SDI_CHANGE_SQUARED
}

/// `UCF Shield Drop Extended.asm` at 0x8009A0B8: the rim count above one
/// passes ftCo_8009A080's stick test.
pub(super) fn platform_drop_rim(input: &FighterInput) -> bool {
    input.hardware.buffer.rim_ticks > 1
}

/// `UCF DBOOC SquatRv Fix.asm` at 0x800D65EC: a fresh horizontal tap on the
/// rim releases the squat at 0.59 instead.
pub(super) fn squat_release_threshold(input: &FighterInput, threshold: f32) -> f32 {
    /// `.long 0x3F170A3D` (0.59).
    const RIM_RELEASE: u32 = 0x3F17_0A3D;
    // lbz 0x670; cmpwi 1; bge: unchanged.
    if input.horizontal.tilt >= 1 || !on_rim(input.current.stick) {
        return threshold;
    }
    f32::from_bits(RIM_RELEASE)
}
