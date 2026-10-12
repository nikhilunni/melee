//! Scripted adapter over the same one-tick engine used by melee-lib::Match.
use crate::{initial_state::InitialState, inputs::PadScript};
pub use melee_lib::diagnostics::RenderedPose;
#[derive(Clone)]
pub struct Simulation {
    engine: melee_lib::diagnostics::Simulation,
    /// Shared, so a copy of a long match's simulation does not copy its
    /// whole input script.
    pads: std::sync::Arc<PadScript>,
    /// [`Self::skip_display_pass`], taken by the next tick.
    display_override: Option<bool>,
}
impl Simulation {
    pub fn new(state: InitialState) -> Self {
        Self::with_inputs(state, PadScript::default())
    }
    pub fn with_inputs(state: InitialState, pads: PadScript) -> Self {
        Self::with_shared_inputs(state, std::sync::Arc::new(pads))
    }
    /// [`Self::with_inputs`] for simulations that run the same script.
    pub fn with_shared_inputs(state: InitialState, pads: std::sync::Arc<PadScript>) -> Self {
        Self {
            engine: melee_lib::diagnostics::Simulation::new(state),
            pads,
            display_override: None,
        }
    }
    /// Replace the pad script; ticks already run keep the pads they used.
    pub fn set_pads(&mut self, pads: PadScript) {
        self.pads = std::sync::Arc::new(pads);
    }
    /// The pads, raw stick bytes and display pass the next tick consumes.
    fn set_tick_inputs(&mut self) {
        let tick = self.engine.frame();
        self.engine.set_inputs(self.pads.samples(tick));
        self.engine.set_raw_sticks(self.pads.raw_sticks(tick));
        self.engine
            .set_recorded_pad_queue_x(self.pads.recorded_pad_queue_x(tick));
        self.engine.set_display_pass(
            self.display_override
                .take()
                .unwrap_or_else(|| self.pads.display_pass(tick)),
        );
    }
    /// The next tick runs with no display pass before it (a console's lag
    /// frame), whatever the script's display clock says.
    pub fn skip_display_pass(&mut self) {
        self.display_override = Some(false);
    }
    pub fn tick_without_snapshot(&mut self) -> anyhow::Result<()> {
        self.engine
            .set_external_events(self.pads.events(self.engine.frame()));
        self.set_tick_inputs();
        self.engine.tick_without_snapshot()
    }
    pub fn tick(&mut self) -> anyhow::Result<melee_diff::Record> {
        self.engine
            .set_external_events(self.pads.events(self.engine.frame()));
        self.set_tick_inputs();
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
