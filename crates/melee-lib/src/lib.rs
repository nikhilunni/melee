//! Deterministic Melee matches, independent of any application or renderer.
mod article_pose;
mod assets;
mod banner;
pub mod diagnostics;
mod frame;
mod initial_state;
mod match_clock;
mod quake;
mod scene_fighter;
mod scene_items;
mod scene_stage;
mod setup;
pub mod slippi;
#[cfg(test)]
use melee_sim::{scenario, trace};

mod config;
mod cpu;
mod error;
mod events;
mod game;
mod input;
mod observation;
pub use config::{Character, Costume, MatchConfig, MatchRules, PlayerConfig, Port, Seed, Stage};
pub use error::{StartError, StateError, StepError};
pub use events::{ConsumedEvents, ExternalEvents, StageRead};
pub use game::{GameAssets, Match, MatchOutcome, MatchStatus, Tick};
pub use input::{Buttons, ControllerState, Inputs, Stick};
pub use observation::{
    ActionId, BlastZones, FighterObservation, ItemId, ItemKind, ItemObservation, Observation, Vec2,
    Vec3,
};
pub use observation::{StageSurface, SurfaceId};

/// Opt-in visual resources and read-only presentation capture. No GPU dependency.
pub mod presentation;
