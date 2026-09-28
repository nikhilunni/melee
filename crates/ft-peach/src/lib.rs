//! Peach: ft/kinds/ftPeach. Shared states live in melee-ft.
pub mod articles;
pub mod attack_s4;
pub mod attributes;
pub mod float;
pub mod float_attack;
pub mod init;
pub mod special_hi;
pub mod special_n;

/// ftPe_Init_MotionStateTable: rows 341..370, contiguous from ftCo_MS_Count.
pub const SPECIAL_ROW_COUNT: usize = 30;

/// ftPe_Init_MotionStateTable (ftpeach.c). The forward smashes, side and
/// down specials stay unported rows.
pub const fn special_rows() -> [melee_ft::fighter::MotionRow; SPECIAL_ROW_COUNT] {
    use melee_ft::fighter::{
        parasol,
        state::{self, callbacks},
        ActionId, MotionRow,
    };
    let mut rows = [state::unimplemented_row(); SPECIAL_ROW_COUNT];
    let mut i = 0;
    while i < SPECIAL_ROW_COUNT {
        rows[i].action = ActionId(341 + i as u16);
        i += 1;
    }
    const fn row(
        action: u16,
        animation: i32,
        anim: melee_ft::fighter::state::AnimFn,
        iasa: melee_ft::fighter::state::InputFn,
        physics: melee_ft::fighter::state::PhysicsFn,
        collision: melee_ft::fighter::state::CollisionFn,
    ) -> MotionRow {
        MotionRow {
            action: ActionId(action),
            id: melee_types::CommonMotionState::None,
            animation,
            anim,
            iasa,
            physics,
            collision,
            camera: callbacks::camera::follow_fighter,
            implemented: true,
        }
    }
    // ftPe_SM_Float = ftCo_SM_Count (295), then FloatFallF/B.
    rows[0] = row(
        341,
        295,
        float::float_anim,
        float::float_input,
        float::float_physics,
        float::collision,
    );
    i = 1;
    while i < 3 {
        rows[i] = row(
            341 + i as u16,
            295 + i as i32,
            float::float_fall_anim,
            float::float_fall_input,
            float::float_fall_physics,
            float::collision,
        );
        i += 1;
    }
    // FloatAttackAirN..Lw reuse the common aerial animations (ftCo_SM_AttackAirN = 68).
    while i < 8 {
        rows[i] = row(
            341 + i as u16,
            68 + (i as i32 - 3),
            float_attack::anim,
            float_attack::input,
            float_attack::physics,
            float_attack::collision,
        );
        i += 1;
    }
    // ftPe_MS_AttackS4Club..Racket (349..351), ftPe_SM 298..300: the common
    // forward smash callbacks (ftPe_AttackS4_Anim/IASA/Phys/Coll match them).
    i = 8;
    while i < 11 {
        rows[i] = row(
            341 + i as u16,
            290 + i as i32,
            callbacks::animation::jab,
            callbacks::input::tilt,
            callbacks::physics::jab,
            callbacks::collision::escape,
        );
        i += 1;
    }
    // ftPe_MS_SpecialHiStart..SpecialAirHiEnd (361..364), ftPe_SM 308..311.
    rows[20] = row(
        361,
        308,
        special_hi::start_anim,
        special_hi::start_input,
        special_hi::start_physics,
        special_hi::start_collision,
    );
    rows[22] = row(
        363,
        310,
        special_hi::start_anim,
        special_hi::start_input,
        special_hi::air_start_physics,
        special_hi::start_collision,
    );
    rows[21].anim = special_hi::unreachable_end;
    rows[23].anim = special_hi::unreachable_end;
    // ftPe_MS_SpecialN..SpecialAirNHit (365..368), ftPe_SM 312..315.
    rows[24] = row(
        365,
        312,
        special_n::anim,
        special_n::input,
        special_n::physics,
        special_n::collision,
    );
    rows[25] = row(
        366,
        313,
        special_n::hit_anim,
        special_n::input,
        special_n::physics,
        special_n::collision,
    );
    rows[26] = row(
        367,
        314,
        special_n::air_anim,
        special_n::input,
        special_n::air_physics,
        special_n::air_collision,
    );
    rows[27] = row(
        368,
        315,
        special_n::air_hit_anim,
        special_n::input,
        special_n::air_hit_physics,
        special_n::air_collision,
    );
    // ftPe_MS_ItemParasolOpen / ItemParasolFall (369 / 370): the common
    // ItemParasolOpen and ItemParasolFallSpecial callbacks, ftPe_SM 316/317.
    rows[28] = row(
        369,
        316,
        parasol::open_anim,
        parasol::open_input,
        parasol::physics,
        parasol::open_collision,
    );
    rows[29] = row(
        370,
        317,
        parasol::fall_special_anim,
        parasol::fall_special_input,
        parasol::physics,
        parasol::fall_special_collision,
    );
    rows
}

/// ftPe_Init_MotionStateTable move IDs: Float and FloatFall are
/// FtMoveId_Default; the float aerials keep their common aerial's move.
pub const SPECIAL_MOVES: [Option<melee_types::combat::StaleMove>; SPECIAL_ROW_COUNT] = {
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
        Some(M::SideSmash),
        Some(M::SideSmash),
        Some(M::SideSmash),
        Some(M::SpecialDown),
        Some(M::SpecialDown),
        Some(M::SpecialSide),
        Some(M::SpecialSide),
        Some(M::SpecialSide),
        Some(M::SpecialSide),
        Some(M::SpecialSide),
        Some(M::SpecialSide),
        Some(M::SpecialSide),
        Some(M::SpecialUp),
        Some(M::SpecialUp),
        Some(M::SpecialUp),
        Some(M::SpecialUp),
        Some(M::SpecialNeutral),
        Some(M::SpecialNeutral),
        Some(M::SpecialNeutral),
        Some(M::SpecialNeutral),
        Some(M::Parasol),
        Some(M::Parasol),
    ]
};
