//! Jigglypuff: ft/kinds/ftPurin. Common states live in melee-ft.
pub mod attributes;
mod common;
pub mod hat;
pub mod init;
pub mod special_hi;
pub mod special_lw;
pub mod special_n;
pub mod special_s;

/// ftPr_Init_MotionStateTable: rows 341..372, contiguous from ftCo_MS_Count.
pub const SPECIAL_ROW_COUNT: usize = 32;
/// ftPr_SM_JumpAerialF1 = ftCo_SM_Count: row `i` plays submotion 295 + i.
const FIRST_SUBMOTION: i32 = 295;

/// ftPr_Init_MotionStateTable (ftpurin.c).
pub const fn special_rows() -> [melee_ft::fighter::MotionRow; SPECIAL_ROW_COUNT] {
    use melee_ft::fighter::{
        state::{self, callbacks},
        ActionId, MotionRow,
    };
    use melee_types::CommonMotionState as S;
    let mut rows = [state::unimplemented_row(); SPECIAL_ROW_COUNT];
    let mut i = 0;
    while i < SPECIAL_ROW_COUNT {
        rows[i].action = ActionId(341 + i as u16);
        i += 1;
    }
    const fn row(
        index: usize,
        anim: melee_ft::fighter::state::AnimFn,
        iasa: melee_ft::fighter::state::InputFn,
        physics: melee_ft::fighter::state::PhysicsFn,
        collision: melee_ft::fighter::state::CollisionFn,
    ) -> MotionRow {
        MotionRow {
            action: ActionId(341 + index as u16),
            id: S::None,
            animation: FIRST_SUBMOTION + index as i32,
            anim,
            iasa,
            physics,
            collision,
            camera: callbacks::camera::follow_fighter,
            implemented: true,
        }
    }
    // JumpAerialF1..F5 (341..345): the common multijump callbacks.
    i = 0;
    while i < 5 {
        rows[i] = MotionRow {
            action: ActionId(341 + i as u16),
            animation: FIRST_SUBMOTION + i as i32,
            anim: callbacks::animation::multi_jump,
            physics: callbacks::physics::multi_jump,
            collision: callbacks::collision::fall,
            ..state::COMMON[S::JumpAerialF as usize]
        };
        i += 1;
    }
    // SpecialNStartR..SpecialNHit (346..362).
    let rollout = special_n::rows();
    i = 0;
    while i < rollout.len() {
        rows[5 + i] = rollout[i];
        i += 1;
    }
    // SpecialS / SpecialAirS (363 / 364).
    rows[22] = row(
        22,
        special_s::anim,
        special_s::input,
        special_s::ground_physics,
        special_s::ground_collision,
    );
    rows[23] = row(
        23,
        special_s::anim,
        special_s::input,
        special_s::air_physics,
        special_s::air_collision,
    );
    // SpecialHiL, SpecialAirHiL, SpecialHiR, SpecialAirHiR (365..368) and
    // the same layout for SpecialLw (369..372).
    i = 24;
    while i < 28 {
        let air = i % 2 == 1;
        rows[i] = row(
            i,
            special_hi::anim,
            special_hi::input,
            if air {
                special_hi::air_physics
            } else {
                special_hi::ground_physics
            },
            if air {
                special_hi::air_collision
            } else {
                special_hi::ground_collision
            },
        );
        rows[i + 4] = row(
            i + 4,
            special_lw::anim,
            special_lw::input,
            if air {
                special_lw::air_physics
            } else {
                special_lw::ground_physics
            },
            if air {
                special_lw::air_collision
            } else {
                special_lw::ground_collision
            },
        );
        i += 1;
    }
    rows
}

/// ftPr_Init_MotionStateTable move IDs: the multijumps are
/// FtMoveId_Default, the rest their special's.
pub const SPECIAL_MOVES: [Option<melee_types::combat::StaleMove>; SPECIAL_ROW_COUNT] = {
    use melee_types::combat::StaleMove as M;
    let mut moves = [None; SPECIAL_ROW_COUNT];
    let mut i = 5;
    while i < SPECIAL_ROW_COUNT {
        moves[i] = Some(match i {
            5..=21 => M::SpecialNeutral,
            22 | 23 => M::SpecialSide,
            24..=27 => M::SpecialUp,
            _ => M::SpecialDown,
        });
        i += 1;
    }
    moves
};
