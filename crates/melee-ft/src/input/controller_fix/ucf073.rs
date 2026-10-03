//! UCF 0.73 beta (2018): the machine code Slippi's console builds installed
//! until 2019-10-09 (`Binary/UCF/Ucf0.73Beta.bin`, built into
//! `Output/Console/g_ucf.bin` by `console_UCF.json`). Ported from the
//! binary's disassembly; `External/UCF + Arduino Toggle UI/UCF/UCF 0.73
//! {Dashback,Shield Drop} - Check for Toggle.asm` is the same program behind
//! a per-port toggle (the Dolphin builds of the time).
//!
//! The shield drop computes what 0.74's does (its -0.8 is read from PlCo
//! here). The dashback differs: the stick must point the way the turn is
//! going, where 0.74 accepts a full stick either way.
use super::{squared_change, PartnerTurn, TurnFacts, RAW_CHANGE_SQUARED};
use crate::input::state::FighterInput;
use gekko_math::msl::{fabsf, fctiwz};
use std::cmp::Ordering;

/// `fcmpo a, b; cror eq, gt, eq; bne`: the branch is taken unless a >= b
/// (an unordered compare takes it too).
fn at_least(a: f32, b: f32) -> bool {
    matches!(a.partial_cmp(&b), Some(Ordering::Greater | Ordering::Equal))
}

/// The dashback at 0x800C9A44, run before the flip's `stfs` (f0 holds the
/// new facing throughout).
pub(super) fn smash_turn(
    input: &FighterInput,
    dash_threshold: f32,
    facts: &TurnFacts,
) -> Option<Option<PartnerTurn>> {
    /// `cmpwi r4, 0x4000`: the high half of 2.0f. The code reads only the
    /// script frame's upper 16 bits, so [2.0, 2.0078125) passes.
    const SLOW_TURN_FRAME_HIGH: u32 = 0x4000;
    // lhz r4,0x3E8(fp): the subaction script's frame (CommandInfo
    // frame_count, cur_anim_frame + x898 as ftAction_80073240 sampled it).
    // cmpwi; bne END.
    if facts.script_frame.to_bits() >> 16 != SLOW_TURN_FRAME_HIGH {
        return None;
    }
    // lfs 0x620 (stick.x); lfs 0x2344 (mv.co.turn.facing_after); fmuls;
    // lfs PlCo+0x3C; fcmpo; cror eq,gt,eq; bne END.
    if !at_least(input.current.stick.x * facts.facing_after, dash_threshold) {
        return None;
    }
    // lbz 0x670; cmpwi 2; bge- END.
    if i32::from(input.horizontal.tilt) >= 2 {
        return None;
    }
    // lbz 0x221F; rlwinm. 0x08; beq+ on; b END: Nana never smash turns.
    if facts.secondary {
        return None;
    }
    // FETCH_INPUT indexes the pad queue by fp+0xC (the player slot) where
    // 0.74 uses fp+0x618; a human's slot is its port. sub; mullw;
    // cmpwi 0x15F9; ble END.
    let queue = input.hardware.queue_x;
    if squared_change(queue.current, queue.two_ticks_ago) <= RAW_CHANGE_SQUARED {
        return None;
    }
    // lbz 0x7 (kind's low byte); cmpwi 0xA; bne+ END. Popo's partner is
    // taken as gobj->next_gx (+0x10): Nana, created right after him on the
    // fighters' GX link (Player_80031AD0; fighters are never relinked). Her
    // newest follow sample takes f0 and, by its bits against 1.0f, a full
    // stick: 0x7F right, else 0x80.
    Some(facts.popo.then_some(PartnerTurn {
        facing: facts.facing,
        stick_x: if facts.facing.to_bits() == 1.0_f32.to_bits() {
            0x7F
        } else {
            i8::MIN
        },
    }))
}

/// The shield drop at 0x800998A4. r5 is still the caller's PlCo pointer
/// (ftCo_80099794 / ftCo_8009980C load it; ftCo_800DF8E8 leaves it).
pub(super) fn blocks_spot_dodge(
    input: &FighterInput,
    escape_threshold: f32,
    walk_fast_threshold: f32,
) -> bool {
    /// `lis r4, 0x42A0`: 80.0, HSD's stick scale.
    const STICK_UNITS: f32 = 80.0;
    /// `lis r4, 0x3727` (about 9.97e-6), subtracted before truncation.
    const TRUNCATION_BIAS: u32 = 0x3727_0000;
    // The loop body (loc_0x34), once per axis: rounded up two stick units.
    let outer = |axis: f32| -> f32 {
        // fabs; fmuls 80; fsubs bias; fctiwz; addi 2.
        let units = fctiwz(fabsf(axis) * STICK_UNITS - f32::from_bits(TRUNCATION_BIAS)) + 2;
        // xoris/lfd against 0x804D8570's magic double; fsubs is exact for
        // these small integers; fdivs by 80.
        units as f32 / STICK_UNITS
    };
    // lfs 0x63C (cstick.y); lfs PlCo+0x314; fcmpo; ble- exit.
    if matches!(
        input.current.cstick.y.partial_cmp(&escape_threshold),
        Some(Ordering::Less | Ordering::Equal)
    ) {
        return false;
    }
    let x = outer(input.current.stick.x);
    let y = outer(input.current.stick.y);
    // fmuls f1,f1,f1 (x); fmuls f0,f0,f0 (y); fadds f0,f0,f1.
    let radius_squared = y * y + x * x;
    // lfs 0x804D8334 (1.0); fcmpo; cror eq,gt,eq; bne- exit.
    if !at_least(radius_squared, 1.0) {
        return false;
    }
    // lbz 0x670; cmpwi 3; ble- exit.
    if i32::from(input.horizontal.tilt) <= 3 {
        return false;
    }
    // lfs PlCo+0x2C (0.8); fneg; lfs 0x624 (stick.y); fcmpo; bge- exit.
    !at_least(-walk_fast_threshold, input.current.stick.y)
}
