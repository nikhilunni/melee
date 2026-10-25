//! Fighter-to-map glue. `melee-mp` owns ECB fitting, subdivision, probes,
//! floor snapping and edge tests; these modules supply the fighter state.
pub mod air;
pub mod ecb;
pub mod ground;
pub mod pose;
pub mod stomp;
#[cfg(test)]
mod tests;
