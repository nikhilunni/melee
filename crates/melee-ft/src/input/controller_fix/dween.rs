//! Dween's controller fix ("Arduino" in Slippi's toggle set):
//! `External/UCF + Arduino Toggle UI/Arduino/Arduino - Check for Toggle.asm`,
//! one injection at 0x8006B028 in the human arm of the input proc
//! (Fighter_Spaghetti_8006AD10), the `stw r0, 0x65C(r31)` that stores the
//! pad's buttons. It runs for a port whose toggle byte is 2 and rewrites the
//! fighter's dead-zoned main stick (fp+0x620/+0x624) before anything reads
//! it: a first tilt out of neutral is held at zero for one tick (so a tap
//! that passes through the tilt range still dashes back), and a shielding
//! stick on the lower rim snaps to the shield-drop notch.
use crate::input::{
    common::InputCommonData,
    pad::{Buttons, PadSample, Stick},
    state::FighterInput,
};
use gekko_math::{fma::fmadds, msl::fabsf};

/// The code's own data after its four per-port slots (`.long 0x3F39999A`,
/// `0xBF300000`, `0x3C4CCCCD`): the notch it writes and the rim margin.
const NOTCH_X: f32 = 0.725;
const NOTCH_Y: f32 = -0.6875;
const RIM_MARGIN: f32 = 0.0125;
/// `cmpwi r4, 3` on x670: ticks the stick has been tilted sideways.
const SHIELD_TILT_TICKS: u8 = 3;
/// `li r4, 0x70`: the pad's Z, R and L.
const SHIELD_BUTTONS: Buttons = Buttons(Buttons::Z.0 | Buttons::R.0 | Buttons::L.0);
/// `li r4, 0x0E00`: B, X and Y.
const ACTION_BUTTONS: Buttons = Buttons(Buttons::B.0 | Buttons::XY.0);

/// What the code reads at 0x8006B028: `stick` and `trigger` are this tick's
/// fp+0x620/+0x624 and fp+0x650 (dead-zoned, before the digital shoulders
/// and Z rewrite the trigger); `input.previous` and the tilt timer are still
/// the last tick's. Returns the stick it leaves and stores the pad's
/// normalized stick x (`lfs f0, 0x20(r3)`) in the port's slot for next tick.
pub(super) fn adjust_stick(
    input: &mut FighterInput,
    pad: &PadSample,
    stick: Stick,
    trigger: f32,
    common: &InputCommonData,
) -> Stick {
    let before = input.hardware.dween_previous_x;
    input.hardware.dween_previous_x = pad.stick.x;
    // fcmpo 0x650, 0.0; bgt: an analog shield, or Z/R/L on the pad.
    if trigger > 0.0 || pad.buttons.intersects(SHIELD_BUTTONS) {
        return shield_drop(input, pad, stick, trigger, common);
    }
    if pad.buttons.intersects(ACTION_BUTTONS) {
        return stick;
    }
    // A pressed this tick (held now, clear in fp+0x660) leaves the stick.
    if pad.buttons.intersects(Buttons::A) && !input.previous.held.intersects(Buttons::A) {
        return stick;
    }
    dashback(stick, before, common)
}

/// `DashBack`: a horizontal stick below the dash threshold whose last pad
/// sample lay inside the dead zone, at least the smash dead zone away.
fn dashback(stick: Stick, before: f32, common: &InputCommonData) -> Stick {
    let t = &common.thresholds;
    // bne END / beq END: y must be zero and x not.
    if stick.y != 0.0 || stick.x == 0.0 {
        return stick;
    }
    // bge END against PlCo+0x3C, then PlCo+0x0 for the stored sample.
    if fabsf(stick.x) >= t.dash_smash_stick_threshold
        || fabsf(before) >= t.horizontal_stick_deadzone
    {
        return stick;
    }
    // fsubs; fabs; blt END against PlCo+0x8.
    if fabsf(stick.x - before) < t.horizontal_stick_smash_deadzone {
        return stick;
    }
    Stick { x: 0.0, y: stick.y }
}

/// `ShieldDrop` and `SDAPPLY`: the shield held over two ticks, the stick on
/// the same side for three, down past the spot dodge line but short of the
/// dash threshold, and on the rim.
fn shield_drop(
    input: &FighterInput,
    pad: &PadSample,
    stick: Stick,
    trigger: f32,
    common: &InputCommonData,
) -> Stick {
    // fmuls 0x654, 0x650; bgt: analog on both ticks. Otherwise the pad's
    // Z/R/L against last tick's held word (fp+0x660).
    let analog_held = input.previous.trigger * trigger > 0.0;
    let digital_held = pad.buttons.0 & SHIELD_BUTTONS.0 & input.previous.held.0 != 0;
    if !analog_held && !digital_held {
        return stick;
    }
    // fmuls 0x628, 0x620; ble END.
    if input.previous.stick.x * stick.x <= 0.0 {
        return stick;
    }
    if input.horizontal.tilt < SHIELD_TILT_TICKS {
        return stick;
    }
    // bgt END against PlCo+0x314; fneg; bge END against PlCo+0x3C.
    if stick.y > common.escape_threshold || -stick.y >= common.thresholds.dash_smash_stick_threshold
    {
        return stick;
    }
    // fabs; fadds; fadds; fmuls; retail fmadds: (m - y)^2 + (|x| + m)^2.
    let across = fabsf(stick.x) + RIM_MARGIN;
    let down = -stick.y + RIM_MARGIN;
    if fmadds(down, down, across * across) <= 1.0 {
        return stick;
    }
    Stick {
        // bge skips `lfs -1.0; fmuls`, an exact negation.
        x: if stick.x >= 0.0 { NOTCH_X } else { -NOTCH_X },
        y: NOTCH_Y,
    }
}
