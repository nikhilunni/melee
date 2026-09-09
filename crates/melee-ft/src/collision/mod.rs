//! Fighter-to-map glue. `melee-mp` owns ECB fitting, subdivision, probes,
//! floor snapping and edge tests; these modules supply the fighter state.
pub mod ecb;
pub mod ground;
pub mod pose;
#[cfg(test)]
mod tests;
