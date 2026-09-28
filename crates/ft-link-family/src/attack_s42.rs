//! ftLk_MS_AttackS42 (341): the forward smash's second hit
//! (ftattacks4combo.c), entered from the first by ftCo_800CED30.
use crate::FamilyState;
use melee_ft::fighter::{
    state::{callbacks, InputPhase},
    Fighter, MotionRow,
};
use melee_types::CommonMotionState;

/// ftCo_AttackS42_IASA: ftCo_Wait_IASA once the script allows it.
fn input(f: &mut Fighter, phase: InputPhase<'_>) {
    if f.core.commands.allow_interrupt {
        callbacks::input::wait(f, phase);
    }
}

/// ftLk_Init_MotionStateTable[0]: ftCo_AttackS42_Anim/Phys/Coll are
/// ftCo_AttackS4_Anim/Phys/Coll (the common AttackS4S callbacks).
pub(crate) const ROW: MotionRow = MotionRow {
    action: FamilyState::AttackS42.action(),
    id: CommonMotionState::None,
    animation: FamilyState::AttackS42.animation(),
    anim: callbacks::animation::jab,
    iasa: input,
    physics: callbacks::physics::jab,
    collision: callbacks::collision::escape,
    camera: callbacks::camera::follow_fighter,
    implemented: true,
};
