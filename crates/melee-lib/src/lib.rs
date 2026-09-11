//! Deterministic Melee matches, independent of any application or renderer.
mod assets;
mod countdown;
pub mod diagnostics;
mod frame;
mod initial_state;
mod scene_fighter;
mod scene_items;
mod scene_stage;
mod setup;
#[cfg(test)]
use melee_sim::{scenario, trace};

mod config;
mod error;
mod game;
mod input;
mod observation;
pub use config::{Character, Costume, MatchConfig, MatchRules, PlayerConfig, Port, Seed, Stage};
pub use error::{StartError, StateError, StepError};
pub use game::{GameAssets, Match, MatchOutcome, MatchStatus, Tick};
pub use input::{Buttons, ControllerState, Inputs, Stick};
pub use observation::{
    ActionId, BlastZones, FighterObservation, ItemId, ItemKind, ItemObservation, Observation, Vec2,
    Vec3,
};
pub use observation::{StageSurface, SurfaceId};
