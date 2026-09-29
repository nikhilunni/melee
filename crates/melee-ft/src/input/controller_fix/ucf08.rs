//! UCF 0.74 and 0.8: `External/UCF 0.74/UCF {DB,SD}.asm` (2019) and
//! `External/UCF 0.8/Logic/UCF {DB,SD,Tumble}.asm` (2021).
use super::{squared_change, PartnerTurn, TurnFacts, RAW_CHANGE_SQUARED};
use crate::input::state::FighterInput;
use gekko_math::msl::{fabsf, fctiwz};
use std::cmp::Ordering;

/// `UCF DB.asm` at 0x800C9A44: on the turn's second animation frame, a full
/// horizontal stick whose raw X moved more than 75 units in two ticks turns
/// the slow turn into a smash turn.
pub(super) fn smash_turn(
    input: &FighterInput,
    dash_threshold: f32,
    facts: &TurnFacts,
) -> Option<Option<PartnerTurn>> {
    /// `.float 2`: the second frame of the turn.
    const SLOW_TURN_FRAME: f32 = 2.0;
    // fcmpo cr0,f1,f2; bne Injection_Exit.
    if facts.animation_frame != SLOW_TURN_FRAME {
        return None;
    }
    // fabs f1; lfs f2,0x3C(PlCo) (0.8); fcmpo; blt Injection_Exit.
    if fabsf(input.current.stick.x) < dash_threshold {
        return None;
    }
    // lbz r3,0x670; cmpwi r3,2; bge- Injection_Exit.
    if i32::from(input.horizontal.tilt) >= 2 {
        return None;
    }
    // lbz 0x221F; rlwinm. 0x08: Nana never smash turns here.
    if facts.secondary {
        return None;
    }
    // mullw; cmpwi 0x15F9; ble- Injection_Exit.
    let queue = input.hardware.queue_x;
    if squared_change(queue.current, queue.two_ticks_ago) <= RAW_CHANGE_SQUARED {
        return None;
    }
    // lwz r4,0x4; cmpwi r4,0xA: only Popo writes Nana's sample, a full
    // stick toward the new facing (fcmpo 0.0; bgt-: 0x7F, else 0x80).
    Some(facts.popo.then_some(PartnerTurn {
        facing: facts.facing,
        stick_x: if facts.facing > 0.0 { 0x7F } else { i8::MIN },
    }))
}

/// `UCF SD.asm` at 0x800998A4: a stick held sideways on the rim, not yet
/// past -0.8, refuses the spot dodge.
pub(super) fn blocks_spot_dodge(input: &FighterInput, escape_threshold: f32) -> bool {
    /// `.float 80`: HSD's stick scale.
    const STICK_UNITS: f32 = 80.0;
    /// `.long 0x37270000` (about 9.97e-6), subtracted before truncation.
    const TRUNCATION_BIAS: u32 = 0x3727_0000;
    /// `.float -0.8`.
    const DOWN_LIMIT: f32 = -0.8;
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

/// 0.8 `UCF Tumble.asm` at 0x800908F4, replacing `cmpw r3, r0` (tilt age
/// against PlCo+0x214, which is 1).
pub(super) fn tumble_wiggle(input: &FighterInput, wiggle_threshold: f32) -> bool {
    let tilt = i32::from(input.horizontal.tilt);
    // cmpwi r3, 1; bne- END: the compare's own result decides (0 wiggles as
    // in retail, 2 or more fails).
    if tilt != 1 {
        return tilt < 1;
    }
    // lfs 0x628 (last tick's stick x); fabs; lfs PlCo+0x210; fcmpo; bge END.
    if fabsf(input.previous.stick.x).partial_cmp(&wiggle_threshold) != Some(Ordering::Less) {
        return false;
    }
    // li r4, 0x15F9; cmpw r4, r3: less-than wiggles.
    let queue = input.hardware.queue_x;
    RAW_CHANGE_SQUARED < squared_change(queue.current, queue.two_ticks_ago)
}
