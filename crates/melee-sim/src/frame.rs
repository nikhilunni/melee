//! Scripted adapter over the same one-tick engine used by melee-lib::Match.
use crate::{initial_state::InitialState, inputs::PadScript};
pub use melee_lib::diagnostics::RenderedPose;
pub struct Simulation {
    engine: melee_lib::diagnostics::Simulation,
    pads: PadScript,
}
impl Simulation {
    pub fn new(state: InitialState) -> Self {
        Self::with_inputs(state, PadScript::default())
    }
    pub fn with_inputs(state: InitialState, pads: PadScript) -> Self {
        Self {
            engine: melee_lib::diagnostics::Simulation::new(state),
            pads,
        }
    }
    pub fn tick_without_snapshot(&mut self) -> anyhow::Result<()> {
        self.engine
            .set_inputs(self.pads.samples(self.engine.frame()));
        self.engine
            .set_display_pass(self.pads.display_pass(self.engine.frame()));
        self.engine.tick_without_snapshot()
    }
    pub fn tick(&mut self) -> anyhow::Result<melee_diff::Record> {
        self.engine
            .set_inputs(self.pads.samples(self.engine.frame()));
        self.engine
            .set_display_pass(self.pads.display_pass(self.engine.frame()));
        self.engine.tick()
    }
}
impl std::ops::Deref for Simulation {
    type Target = melee_lib::diagnostics::Simulation;
    fn deref(&self) -> &Self::Target {
        &self.engine
    }
}
impl std::ops::DerefMut for Simulation {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.engine
    }
}
pub mod rendered_pose {
    pub use melee_lib::diagnostics::RenderedPose;
}
