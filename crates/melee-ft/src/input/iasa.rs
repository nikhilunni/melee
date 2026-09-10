//! ftCo_Wait_IASA (0x8008A4D4, ftCommon/ftCo_Wait.c:44-67).
//! Decision-only port for an item-free fighter in Wait. Transition bodies,
//! item interactions, tether restrictions and jab continuation belong to later
//! tasks; their preconditions are asserted, never silently treated as false.
use super::{common::InputCommonData, human::jump_input, pad::Buttons, state::FighterInput};
use gekko_math::msl::fabsf;
use melee_lb::trigf::atan2f;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitTransition {
    None,
    Special,
    Grab,
    Attack,
    Escape,
    Shield,
    Taunt,
    Jump,
    Dash,
    Squat,
    Turn,
    Walk,
}

/// Individual retail predicates, including the misleading Attack100 symbol:
/// that predicate actually checks *up special*, not rapid jab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitPredicate {
    SpecialSide,
    SpecialUp,
    SpecialNeutral,
    SpecialDown,
    Grab,
    SmashSide,
    SmashUp,
    SmashDown,
    TiltSide,
    TiltUp,
    TiltDown,
    Jab,
    Escape,
    Shield,
    FoxTaunt,
    Taunt,
    Jump,
    Dash,
    Squat,
    Turn,
    Walk,
}
/// Retail call sites 0x8008A4E8..0x8008A628, audited with asm.py --calls.
pub const WAIT_PREDICATES: [WaitPredicate; 21] = [
    WaitPredicate::SpecialSide,
    WaitPredicate::SpecialUp,
    WaitPredicate::SpecialNeutral,
    WaitPredicate::SpecialDown,
    WaitPredicate::Grab,
    WaitPredicate::SmashSide,
    WaitPredicate::SmashUp,
    WaitPredicate::SmashDown,
    WaitPredicate::TiltSide,
    WaitPredicate::TiltUp,
    WaitPredicate::TiltDown,
    WaitPredicate::Jab,
    WaitPredicate::Escape,
    WaitPredicate::Shield,
    WaitPredicate::FoxTaunt,
    WaitPredicate::Taunt,
    WaitPredicate::Jump,
    WaitPredicate::Dash,
    WaitPredicate::Squat,
    WaitPredicate::Turn,
    WaitPredicate::Walk,
];

/// State owned by the fighter/match, supplied to the pure decision path.
#[derive(Debug, Clone)]
pub struct WaitContext {
    /// Fighter.x2C; retail always +/-1.
    pub facing: f32,
    /// ftData_SpecialS/Hi/N/Lw[kind] != NULL, in that order.
    pub specials_available: [bool; 4],
    /// shield_health (+1998), tested as attack_pressed C float boolean.
    pub shield_health: f32,
    /// Kind is Fox/Falco AND grCorneria_801E2CE8() AND appeal count is zero.
    pub fox_taunt_available: bool,
    /// item_gobj (+1974). Item decisions are outside this item-free port.
    pub holding_item: bool,
    /// Link/Young Link u.lk.xC or Samus u.ss.x223C != NULL.
    pub tether_active: bool,
    /// hitlag_mul (+196C), the jab continuation countdown checked even at rest.
    pub jab_countdown: f32,
}
impl Default for WaitContext {
    fn default() -> Self {
        Self {
            facing: 1.0,
            specials_available: [true; 4],
            shield_health: 60.0,
            fox_taunt_available: false,
            holding_item: false,
            tether_active: false,
            jab_countdown: 0.0,
        }
    }
}

pub fn wait_iasa(
    input: &FighterInput,
    common: &InputCommonData,
    context: &WaitContext,
) -> WaitTransition {
    wait_iasa_observe(input, common, context, |_| {})
}

/// Observer receives only predicates actually visited, for order verification.
/// The context preconditions make omitted item/jab side effects unreachable.
pub fn wait_iasa_observe(
    input: &FighterInput,
    common: &InputCommonData,
    context: &WaitContext,
    mut visited: impl FnMut(WaitPredicate),
) -> WaitTransition {
    assert!(
        !context.holding_item,
        "Wait item/hammer paths are outside T9"
    );
    assert!(
        !context.tether_active,
        "Wait tether restrictions are outside T9"
    );
    assert!(
        context.jab_countdown == 0.0,
        "ftCo_Attack1_CheckInput jab countdown needs its state owner"
    );
    assert!(context.facing == 1.0 || context.facing == -1.0);
    for predicate in WAIT_PREDICATES {
        visited(predicate);
        let transition = evaluate(predicate, input, common, context);
        if transition != WaitTransition::None {
            return transition;
        }
    }
    WaitTransition::None
}
pub fn evaluate(
    predicate: WaitPredicate,
    input: &FighterInput,
    common: &InputCommonData,
    context: &WaitContext,
) -> WaitTransition {
    use WaitPredicate as P;
    use WaitTransition as T;
    let thresholds = &common.thresholds;
    let stick = input.current.stick;
    let attack_pressed = input.pressed.intersects(Buttons::A);
    let shield_held = input.current.held.intersects(Buttons::SHIELD);
    let (matches, transition) = match predicate {
        // ftCo_SpecialS_CheckInput 80096030; ftCo_Attack100_CheckInput 800D695C;
        // ftCo_800D6824; ftCo_800D68C0. These read buffers, not raw B edges.
        P::SpecialSide => (
            context.specials_available[0] && input.buttons.special_side == 0,
            T::Special,
        ),
        P::SpecialUp => (
            context.specials_available[1] && input.buttons.special_up == 0,
            T::Special,
        ),
        P::SpecialNeutral => (
            context.specials_available[2] && input.buttons.special_neutral == 0,
            T::Special,
        ),
        P::SpecialDown => (
            context.specials_available[3] && input.buttons.special_down == 0,
            T::Special,
        ),
        // ftCo_Catch_CheckInput 800D8990; item/tether checks pass with this context.
        P::Grab => (shield_held && attack_pressed, T::Grab),
        P::SmashSide
        | P::SmashUp
        | P::SmashDown
        | P::TiltSide
        | P::TiltUp
        | P::TiltDown
        | P::Jab => (attack_matches(predicate, input, common, context), T::Attack),
        // ftCo_80099794, then ftCo_80091A4C. Escape precedes Guard.
        P::Escape => (
            shield_held
                && stick.y <= common.escape_threshold
                && i32::from(input.vertical.tilt) < common.escape_window,
            T::Escape,
        ),
        P::Shield => (
            (input.pressed.intersects(Buttons::DIGITAL_SHOULDERS)
                && i32::from(input.shoulder.tilt) < common.powershield_window)
                || (shield_held && context.shield_health != 0.0),
            T::Shield,
        ),
        // ftFx_AppealS_CheckInput 800E59BC: on release, x682 == 1.
        P::FoxTaunt => (
            context.fox_taunt_available
                && !input.current.held.intersects(Buttons::DOWN)
                && input.buttons.down == 1,
            T::Taunt,
        ),
        P::Taunt => (input.pressed.intersects(Buttons::UP), T::Taunt), // 800DE9D8
        P::Jump => (jump_input(input, common), T::Jump),               // 800CAED0, hammer absent
        // ftCo_Dash_CheckInput 800CA094: back-smash enters Turn, not Dash.
        P::Dash => (
            fabsf(stick.x) >= thresholds.dash_smash_stick_threshold
                && i32::from(input.horizontal.tilt) < thresholds.dash_smash_window,
            if stick.x * context.facing < 0.0 {
                T::Turn
            } else {
                T::Dash
            },
        ),
        P::Squat => (stick.y < -thresholds.squat_stick_threshold, T::Squat), // 800D5FB0
        P::Turn => (
            stick.x * context.facing <= thresholds.turn_stick_threshold,
            T::Turn,
        ), // 800C97DC
        P::Walk => (
            stick.x * context.facing >= thresholds.walk_stick_threshold,
            T::Walk,
        ), // 800C9468
    };
    if matches {
        transition
    } else {
        T::None
    }
}

fn attack_matches(
    predicate: WaitPredicate,
    input: &FighterInput,
    common: &InputCommonData,
    context: &WaitContext,
) -> bool {
    use WaitPredicate as P;
    let thresholds = &common.thresholds;
    let stick = input.current.stick;
    let cstick = input.current.cstick;
    let previous_cstick = input.previous.cstick;
    let attack_pressed = input.pressed.intersects(Buttons::A);
    // ftCo_GetLStickAngle (0x8007D964): atan2f(y, ABS(x)).
    let angle = || atan2f(stick.y, fabsf(stick.x));
    match predicate {
        // AttackS4 8008BFC4 / Hi4 8008C830 / Lw4 8008CB44; C-stick edge paths
        // 800DF1C8 / 800DF2D8 / 800DF3A8 run even with no A button.
        P::SmashSide => {
            (attack_pressed
                && fabsf(stick.x) >= thresholds.dash_smash_stick_threshold
                && i32::from(input.horizontal.tilt) < thresholds.dash_smash_window)
                || (fabsf(previous_cstick.x) < thresholds.dash_smash_stick_threshold
                    && fabsf(cstick.x) >= thresholds.dash_smash_stick_threshold)
        }
        P::SmashUp => {
            (attack_pressed
                && stick.y >= common.up_smash_threshold
                && f32::from(input.vertical.tilt) < common.up_smash_window)
                || (previous_cstick.y < common.up_smash_threshold
                    && cstick.y >= common.up_smash_threshold)
        }
        P::SmashDown => {
            (attack_pressed
                && stick.y <= common.down_smash_threshold
                && f32::from(input.vertical.tilt) < common.down_smash_window)
                || (previous_cstick.y > common.down_smash_threshold
                    && cstick.y <= common.down_smash_threshold)
        }
        // AttackS3 8008B658 / Hi3 8008B980 / Lw3 8008BB44 / Attack1 8008A9F8.
        P::TiltSide => {
            attack_pressed
                && stick.x * context.facing >= common.side_tilt_threshold
                && fabsf(angle()) < common.tilt_angle
        }
        P::TiltUp => {
            attack_pressed && stick.y >= common.up_tilt_threshold && angle() > common.tilt_angle
        }
        P::TiltDown => {
            attack_pressed && stick.y <= common.down_tilt_threshold && angle() < -common.tilt_angle
        }
        P::Jab => attack_pressed,
        _ => unreachable!("only attack predicates are dispatched here"),
    }
}
