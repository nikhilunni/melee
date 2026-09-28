//! Peach: ft/kinds/ftPeach. Shared states live in melee-ft.
pub mod attributes;
pub mod float;
pub mod float_attack;
pub mod init;

/// ftPe_Init_MotionStateTable[0..8]: Float, FloatFallF/B and the five float
/// aerials, contiguous from ftCo_MS_Count.
pub const fn special_rows() -> [melee_ft::fighter::MotionRow; 8] {
    use melee_ft::fighter::{
        state::{self, callbacks},
        ActionId, MotionRow,
    };
    let mut rows = [state::unimplemented_row(); 8];
    let mut i = 0;
    while i < 8 {
        rows[i].action = ActionId(341 + i as u16);
        i += 1;
    }
    // ftPe_SM_Float = ftCo_SM_Count (295), then FloatFallF/B.
    rows[0] = MotionRow {
        action: float::FLOAT,
        id: melee_types::CommonMotionState::None,
        animation: 295,
        anim: float::float_anim,
        iasa: float::float_input,
        physics: float::float_physics,
        collision: float::collision,
        camera: callbacks::camera::follow_fighter,
        implemented: true,
    };
    i = 1;
    while i < 3 {
        rows[i] = MotionRow {
            action: ActionId(341 + i as u16),
            id: melee_types::CommonMotionState::None,
            animation: 295 + i as i32,
            anim: float::float_fall_anim,
            iasa: float::float_fall_input,
            physics: float::float_fall_physics,
            collision: float::collision,
            camera: callbacks::camera::follow_fighter,
            implemented: true,
        };
        i += 1;
    }
    // FloatAttackAirN..Lw reuse the common aerial animations (ftCo_SM_AttackAirN = 68).
    while i < 8 {
        rows[i] = MotionRow {
            action: ActionId(341 + i as u16),
            id: melee_types::CommonMotionState::None,
            animation: 68 + (i as i32 - 3),
            anim: float_attack::anim,
            iasa: float_attack::input,
            physics: float_attack::physics,
            collision: float_attack::collision,
            camera: callbacks::camera::follow_fighter,
            implemented: true,
        };
        i += 1;
    }
    rows
}

/// ftPe_Init_MotionStateTable move IDs: Float and FloatFall are
/// FtMoveId_Default; the float aerials keep their common aerial's move.
pub const SPECIAL_MOVES: [Option<melee_types::combat::StaleMove>; 8] = {
    use melee_types::combat::StaleMove as M;
    [
        None,
        None,
        None,
        Some(M::NeutralAir),
        Some(M::ForwardAir),
        Some(M::BackAir),
        Some(M::UpAir),
        Some(M::DownAir),
    ]
};
