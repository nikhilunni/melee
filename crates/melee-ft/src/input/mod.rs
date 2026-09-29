//! Fighter input: human pad sampling and buffers (Fighter_Spaghetti_8006AD10,
//! s_link 3), CPU routing (ftCo_800A2040 / Fighter_8006ABA0, s_link 2), and
//! decision-only Wait IASA. See README.md for the ownership and oracle contract.
pub mod common;
pub mod controller_fix;
mod geometry;
pub mod human;
pub mod iasa;
pub mod pad;
pub mod state;

pub use common::InputCommonData;
pub use controller_fix::{ControllerFix, HardwareInput, PadQueueX};
pub use human::{
    input_source, resolve_player_kind, run_cpu_input_proc, update_human_input, update_input,
    InputContext, InputEffects, InputSource,
};
pub use iasa::{
    iasa_with_predicates, wait_iasa, wait_iasa_observe, WaitContext, WaitPredicate, WaitTransition,
    FORWARD_SMASH_PREDICATES, OTTOTTO_PREDICATES, WAIT_PREDICATES,
};
pub use pad::{Buttons, PadSample, Stick};
pub use state::{AnalogTimers, ButtonTimers, FighterInput, InputFrame};

pub use geometry::crosses_stick_circle;
