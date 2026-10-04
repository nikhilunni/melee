use super::{
    common::InputCommonData,
    geometry::crosses_stick_circle,
    pad::{Buttons, PadSample, Stick},
    state::{AnalogTimers, FighterInput, InputFrame},
};
use gekko_math::msl::fabsf;
use melee_types::PlayerKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputSource {
    Pad,
    /// The CPU (melee-cpu) supplies the sample (CpuState::pad_sample).
    Cpu,
}

/// ftCo_800A2040 (0x800A2040, ftCommon/ftCo_0A01.c:1080-1089).
/// Pass Player_8003248C's resolved kind, not just the slot's stored kind.
/// CPU mode numbers remain owned by melee-cpu; 5 selects controller override.
pub fn input_source(resolved_player_kind: PlayerKind, cpu_mode: i32) -> InputSource {
    if resolved_player_kind == PlayerKind::Cpu && cpu_mode != 5 {
        InputSource::Cpu
    } else {
        InputSource::Pad
    }
}

/// Player_8003248C (0x8003248C, pl/player.c:446-470). A secondary fighter
/// (x221F_b4) whose PdPmdat entry.z is zero is CPU-controlled even in a human slot.
pub fn resolve_player_kind(
    kind: PlayerKind,
    secondary: bool,
    character_entry_z_is_zero: bool,
) -> PlayerKind {
    if secondary && character_entry_z_is_zero && matches!(kind, PlayerKind::Human | PlayerKind::Cpu)
    {
        PlayerKind::Cpu
    } else {
        kind
    }
}

/// Fighter_8006ABA0 (s_link 2, 0x8006ABA0). Human slots never call the AI
/// or consume RNG, regardless of their initialized CpuFighter.xC value.
pub fn run_cpu_input_proc(disabled: bool, source: InputSource) {
    if disabled {
        return;
    }
    match source {
        InputSource::Pad => {}
        InputSource::Cpu => unreachable!("ftCo_800B3900 runs in melee-cpu, which the scene calls"),
    }
}

/// Inputs from the proc/match owner. All-false represents an ordinary active
/// human in retail mode. These are gates, not inferred from neutral buttons.
#[derive(Debug, Default, Clone, Copy)]
pub struct InputContext {
    /// x221F_b3: suppress the entire proc.
    pub disabled: bool,
    /// x2224_b2: do not read a new sample, then save/reset the input block.
    pub freeze_sample: bool,
    /// x2219_b5: accumulate edges and skip post-input processing / IASA.
    pub hitlag: bool,
    /// x221D_b4 or gm_801A45E8(2): save then clear current input.
    pub save_and_clear: bool,
    /// DbLevel >= DebugRom or gm_8016B41C(): zero C-stick only.
    pub suppress_cstick: bool,
    /// gm_8016B0FC(): human buttons restricted to A, analog shield zeroed.
    pub single_button_mode: bool,
    /// gm_801A45E8(0): suppress the Z -> A+LR macro.
    pub suppress_z_macro: bool,
    /// x1980 != NULL: ftCommon_8007FFD8 belongs to capture state, outside T9.
    pub captured: bool,
    /// smash_attrs.state is PreCharge or Charging: ftCo_800DF0D0 is not idle.
    pub smash_charge_active: bool,
    /// ftCo_800A2040: the CPU, not the pad, drives this fighter (UCF 0.84's
    /// pad buffer skips it).
    pub cpu_controlled: bool,
    /// fp->kind 19 (Zelda) in motion 349: UCF 0.84 leaves its sticks unsnapped.
    pub cardinal_exempt: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct InputEffects {
    /// Calls to Player_UpdateJoystickCountByIndex, in stick-then-trigger order.
    pub joystick_count_increments: u8,
    /// The owner may dispatch fp->input_cb (+219C), e.g. wait_iasa.
    pub run_input_callback: bool,
}

/// Fighter_Spaghetti_8006AD10, s_link 3 (fighter.c:1777-2140).
/// Ports all human sampling/buffering plus the post-input counters. The caller
/// owns the callback/state transitions and applies returned Player statistics.
/// Active capture and smash-charge hooks fail explicitly until their owner is ported.
pub fn update_human_input(
    input: &mut FighterInput,
    sample: &PadSample,
    common: &InputCommonData,
    context: InputContext,
) -> InputEffects {
    if context.disabled {
        return InputEffects::default();
    }
    if !context.hitlag {
        assert!(
            !context.captured,
            "ftCommon_8007FFD8 capture hook is outside T9"
        );
        assert!(
            !context.smash_charge_active,
            "ftCo_800DF0D0 charge hook is outside T9"
        );
    }
    if !context.freeze_sample {
        sample_input(input, sample, common, context);
        update_analog_timers(input, common, context);
        update_button_timers(input);
    }
    if context.save_and_clear || context.freeze_sample {
        input.saved = input.current;
        input.use_current_history = false;
        input.clear_current_and_buffers();
    }
    if context.hitlag {
        return InputEffects::default();
    }
    let joystick_count_increments = count_joystick_activity(input, common);
    update_action_timers(input, common);
    InputEffects {
        joystick_count_increments,
        run_input_callback: true,
    }
}

/// Select the human/CPU arm explicitly, as both retail input reads do.
pub fn update_input(
    input: &mut FighterInput,
    source: InputSource,
    sample: &PadSample,
    common: &InputCommonData,
    context: InputContext,
) -> InputEffects {
    if context.disabled || context.freeze_sample {
        return update_human_input(input, sample, common, context);
    }
    match source {
        InputSource::Pad => update_human_input(input, sample, common, context),
        InputSource::Cpu => unreachable!("the caller passes CpuState::pad_sample as a pad"),
    }
}

/// Fighter's <= dead zone leaves values outside it unchanged, with no rescaling.
pub fn apply_deadzone(value: f32, threshold: f32) -> f32 {
    if fabsf(value) <= threshold {
        0.0
    } else {
        value
    }
}

fn sample_input(
    input: &mut FighterInput,
    pad: &PadSample,
    common: &InputCommonData,
    context: InputContext,
) {
    input.previous = if input.use_current_history {
        input.current
    } else {
        input.saved
    };
    input.use_current_history = true;
    let t = &common.thresholds;
    let deadzone = |s: Stick| Stick {
        x: apply_deadzone(s.x, t.horizontal_stick_deadzone),
        y: apply_deadzone(s.y, t.vertical_stick_deadzone),
    };
    let mut trigger = if pad.left_trigger > pad.right_trigger {
        pad.left_trigger
    } else {
        pad.right_trigger
    };
    if trigger <= t.analog_shoulder_deadzone {
        trigger = 0.0;
    }
    // Fighter+0x620: the dead-zoned stick, which Dween's code (0x8006B028,
    // the human arm's button store) may rewrite before anything reads it.
    let mut stick = deadzone(pad.stick);
    if !context.cpu_controlled {
        stick = super::controller_fix::sampled_stick(input, pad, stick, trigger, common);
    }
    let mut held = pad.buttons;
    if context.single_button_mode {
        trigger = 0.0;
        held.0 &= Buttons::A.0;
    } else {
        if held.intersects(Buttons::DIGITAL_SHOULDERS) {
            held |= Buttons::SHIELD;
            trigger = 1.0;
        } else if trigger != 0.0 {
            held |= Buttons::SHIELD;
        }
        if !context.suppress_z_macro && held.intersects(Buttons::Z) {
            held |= Buttons::SHIELD | Buttons::A;
            trigger = t.z_press_analog_value;
        }
    }
    input.current = InputFrame {
        stick,
        cstick: deadzone(if context.suppress_cstick {
            Stick::default()
        } else {
            pad.cstick
        }),
        trigger,
        held,
    };
    let changed = held.0 ^ input.previous.held.0;
    let pressed = Buttons(held.0 & changed);
    let released = Buttons(input.previous.held.0 & changed);
    if context.hitlag {
        input.pressed |= pressed;
        input.released |= released;
    } else {
        input.pressed = pressed;
        input.released = released;
    }
}

// Retail stores the incremented byte BEFORE comparing to 0xFE (0x8006B128).
fn increment_analog(value: &mut u8) {
    *value = value.wrapping_add(1).min(0xFE);
}
fn update_axis(
    timers: &mut AnalogTimers,
    current: f32,
    previous: f32,
    threshold: f32,
) -> Option<bool> {
    increment_analog(&mut timers.since_crossing);
    let positive = if current >= threshold {
        true
    } else if current <= -threshold {
        false
    } else {
        timers.tilt = 0xFE;
        timers.held = 0xFE;
        timers.activity = 0xFE;
        return None;
    };
    if (positive && previous >= threshold) || (!positive && previous <= -threshold) {
        increment_analog(&mut timers.tilt);
        increment_analog(&mut timers.held);
        increment_analog(&mut timers.activity);
        None
    } else {
        timers.since_crossing = 0;
        timers.held = 0;
        timers.tilt = 0;
        Some(positive)
    }
}
fn update_analog_timers(input: &mut FighterInput, common: &InputCommonData, context: InputContext) {
    let t = &common.thresholds;
    if let Some(positive) = update_axis(
        &mut input.horizontal,
        input.current.stick.x,
        input.previous.stick.x,
        t.horizontal_stick_smash_deadzone,
    ) {
        input.last_horizontal_positive = positive;
    }
    if let Some(positive) = update_axis(
        &mut input.vertical,
        input.current.stick.y,
        input.previous.stick.y,
        t.vertical_stick_smash_deadzone,
    ) {
        input.last_vertical_negative = !positive;
    }
    if crosses_stick_circle(
        input.previous.stick,
        input.current.stick,
        t.horizontal_stick_smash_deadzone,
    ) {
        input.horizontal.activity = 0;
        input.vertical.activity = 0;
    }
    // 0x8006B460: UCF 0.84's pad buffer and 1.0 cardinals hook here.
    super::controller_fix::update_pad_buffer(
        input,
        !context.cpu_controlled,
        context.cardinal_exempt,
    );
    let s = &mut input.shoulder;
    increment_analog(&mut s.since_crossing);
    if input.current.trigger >= t.shield_press_threshold {
        if input.previous.trigger >= t.shield_press_threshold {
            increment_analog(&mut s.tilt);
            increment_analog(&mut s.held);
            increment_analog(&mut s.activity);
        } else {
            *s = AnalogTimers {
                tilt: 0,
                held: 0,
                since_crossing: 0,
                activity: 0,
            };
        }
    } else {
        s.tilt = 0xFE;
        s.held = 0xFE;
        s.activity = 0xFE;
    }
}
fn update_timer(timer: &mut u8, active: bool) {
    if active {
        *timer = 0;
    } else {
        *timer = timer.saturating_add(1);
    }
}
fn update_button_timers(input: &mut FighterInput) {
    let b = &mut input.buttons;
    if input.pressed.intersects(Buttons::A) {
        b.previous_attack = b.attack;
    }
    if input.pressed.intersects(Buttons::DIGITAL_SHOULDERS) {
        b.previous_digital_shield = b.digital_shield;
    }
    for (timer, mask) in [
        (&mut b.attack, Buttons::A),
        (&mut b.special, Buttons::B),
        (&mut b.jump_button, Buttons::XY),
        (&mut b.taunt, Buttons::UP),
        (&mut b.down, Buttons::DOWN),
        (&mut b.shield, Buttons::SHIELD),
        (&mut b.digital_shield, Buttons::DIGITAL_SHOULDERS),
    ] {
        update_timer(timer, input.pressed.intersects(mask));
    }
}
/// ftCommon_8008031C (0x8008031C): report Player side effects, retain the
/// timer resets in their retail order before Fighter_UnkIncrementCounters.
fn count_joystick_activity(input: &mut FighterInput, common: &InputCommonData) -> u8 {
    let mut count = 0;
    if (fabsf(input.current.stick.x) >= common.activity_stick_threshold
        && f32::from(input.horizontal.activity) < common.activity_window)
        || (fabsf(input.current.stick.y) >= common.activity_stick_threshold
            && f32::from(input.vertical.activity) < common.activity_window)
    {
        count += 1;
        input.horizontal.activity = 0xFE;
        input.vertical.activity = 0xFE;
    }
    if fabsf(input.current.trigger) >= common.activity_trigger_threshold
        && f32::from(input.shoulder.activity) < common.activity_window
    {
        count += 1;
        input.shoulder.activity = 0xFE;
    }
    count
}
pub fn jump_input(input: &FighterInput, common: &InputCommonData) -> bool {
    (input.current.stick.y >= common.thresholds.tap_jump_threshold
        && i32::from(input.vertical.tilt) < common.thresholds.tap_jump_window)
        || input.pressed.intersects(Buttons::XY)
}
/// Fighter_UnkIncrementCounters_8006ABEC; predicates at 0x800CAE80,
/// 0x800D6928, 0x800D688C, 0x800964FC, 0x800D67C4.
fn update_action_timers(input: &mut FighterInput, common: &InputCommonData) {
    let jump = jump_input(input, common);
    let special = input.pressed.intersects(Buttons::B);
    let (x, y) = (input.current.stick.x, input.current.stick.y);
    let up = special && y >= common.special_vertical_threshold;
    let b = &mut input.buttons;
    if jump {
        b.previous_jump = b.jump;
    }
    if up {
        b.previous_special_up = b.special_up;
    }
    update_timer(&mut b.jump, jump);
    update_timer(&mut b.special_up, up);
    update_timer(
        &mut b.special_down,
        special && y < -common.special_vertical_threshold,
    );
    update_timer(
        &mut b.special_side,
        special && fabsf(x) >= common.special_side_threshold,
    );
    update_timer(
        &mut b.special_neutral,
        special
            && fabsf(x) < common.special_side_threshold
            && fabsf(y) < common.special_vertical_threshold,
    );
}
