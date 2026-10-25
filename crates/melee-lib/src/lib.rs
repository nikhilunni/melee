//! Deterministic Melee matches, independent of any application or renderer.
mod article_pose;
mod assets;
mod banner;
mod score_popup_timelines;
mod frame_timer;
mod special_melee;
mod stamina;
mod standings;
pub use special_melee::SpecialMelee;
pub mod diagnostics;
mod frame;
mod initial_state;
mod ko_counts;
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
pub use assets::FileSource;
pub use melee_it::random_spawn::ItemRules;
pub use melee_types::MatchKind;
pub use config::{
    ported_item_switch, Character, ControllerFix, Costume, MatchConfig, MatchRules, PlayerConfig,
    Port, Seed, Stage, PORTED_RANDOM_ITEMS,
};
pub use error::{unported_stage, StartError, StateError, StepError};
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
