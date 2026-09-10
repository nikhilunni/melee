//! Marth: ft/kinds/ftMars. Shared states live in melee-ft.
pub mod attributes;
pub mod init;

pub mod special_hi;
pub mod special_lw;
pub mod special_n;
pub mod special_s;

/// Character rows are contiguous from ftCo_MS_Count, as in ftMars_Init_MotionStateTable.
pub const fn special_rows() -> [melee_ft::fighter::MotionRow; 32] {
    use melee_ft::fighter::{
        state::{self, callbacks},
        ActionId,
    };
    let mut rows = [state::unimplemented_row(); 32];
    let mut i = 0;
    while i < 32 {
        rows[i].action = ActionId(341 + i as u16);
        i += 1;
    }
    i = 0;
    while i < 8 {
        rows[i] = melee_ft::fighter::MotionRow {
            action: ActionId(341 + i as u16),
            id: melee_types::CommonMotionState::None,
            animation: 295 + i as i32,
            anim: if i % 4 == 0 {
                special_n::start
            } else if i % 4 == 1 {
                special_n::hold
            } else {
                special_n::end
            },
            iasa: if i % 4 == 1 {
                special_n::input
            } else {
                no_input
            },
            physics: if i >= 4 {
                special_n::air_physics
            } else if i == 0 {
                special_n::startup_physics
            } else {
                callbacks::physics::guard_on
            },
            collision: if i >= 4 {
                special_n::air_collision
            } else {
                special_n::collision
            },
            camera: callbacks::camera::follow_fighter,
            implemented: true,
        };
        i += 1;
    }
    i = 8;
    while i < 26 {
        rows[i] = melee_ft::fighter::MotionRow {
            action: ActionId(341 + i as u16),
            id: melee_types::CommonMotionState::None,
            animation: 295 + i as i32,
            anim: special_s::anim,
            iasa: if (i - 8) % 9 < 6 {
                special_s::input
            } else {
                no_input
            },
            physics: if i >= 17 {
                special_s::air_physics
            } else if i < 11 {
                callbacks::physics::guard_on
            } else {
                callbacks::physics::jab
            },
            collision: if i >= 17 {
                special_s::air_collision
            } else {
                special_s::collision
            },
            camera: callbacks::camera::follow_fighter,
            implemented: true,
        };
        i += 1;
    }
    i = 26;
    while i < 28 {
        rows[i] = melee_ft::fighter::MotionRow {
            action: ActionId(341 + i as u16),
            id: melee_types::CommonMotionState::None,
            animation: 295 + i as i32,
            anim: special_hi::anim,
            iasa: special_hi::input,
            physics: special_hi::physics,
            collision: special_hi::collision,
            camera: callbacks::camera::follow_fighter,
            implemented: true,
        };
        i += 1;
    }
    i = 28;
    while i < 30 {
        rows[i] = melee_ft::fighter::MotionRow {
            action: ActionId(341 + i as u16),
            id: melee_types::CommonMotionState::None,
            animation: 295 + i as i32,
            anim: if i == 28 {
                special_lw::anim
            } else {
                special_lw::hit_anim
            },
            iasa: no_input,
            physics: if i == 28 {
                special_lw::physics
            } else {
                callbacks::physics::guard_on
            },
            collision: if i == 28 {
                special_s::collision
            } else {
                special_n::collision
            },
            camera: callbacks::camera::follow_fighter,
            implemented: true,
        };
        i += 1;
    }
    rows
}

fn no_input(_: &mut melee_ft::fighter::Fighter, _: melee_ft::fighter::state::InputPhase<'_>) {}

/// ftMars_Init_MotionStateTable's FtMoveId values, including aerial counterparts.
pub const fn special_moves() -> [Option<melee_types::combat::StaleMove>; 32] {
    use melee_types::combat::StaleMove as M;
    let mut moves = [None; 32];
    let mut i = 0;
    while i < 32 {
        moves[i] = Some(if i < 8 {
            M::SpecialNeutral
        } else if i < 26 {
            M::SpecialSide
        } else if i < 28 {
            M::SpecialUp
        } else {
            M::SpecialDown
        });
        i += 1;
    }
    moves
}
