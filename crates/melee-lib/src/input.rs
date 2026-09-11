//! Controller samples, never a keyboard mapping or an action policy.
use crate::{Port, StepError};
pub use melee_ft::input::{Buttons, PadSample as ControllerState, Stick};

/// All four physical ports, neutral by default. Samples are held for one tick.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Inputs(pub [ControllerState; 4]);
impl std::ops::Index<Port> for Inputs {
    type Output = ControllerState;
    fn index(&self, port: Port) -> &Self::Output {
        &self.0[port.index()]
    }
}
impl std::ops::IndexMut<Port> for Inputs {
    fn index_mut(&mut self, port: Port) -> &mut Self::Output {
        &mut self.0[port.index()]
    }
}
impl Inputs {
    pub(crate) fn validate(&self) -> Result<(), StepError> {
        for port in Port::ALL {
            let pad = self[port];
            let sticks = [pad.stick.x, pad.stick.y, pad.cstick.x, pad.cstick.y];
            if sticks
                .iter()
                .any(|v| !v.is_finite() || !(-1.0..=1.0).contains(v))
            {
                return Err(StepError::InvalidInput {
                    port,
                    reason: "stick axes must be finite and in -1..=1",
                });
            }
            if [pad.left_trigger, pad.right_trigger]
                .iter()
                .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
            {
                return Err(StepError::InvalidInput {
                    port,
                    reason: "triggers must be finite and in 0..=1",
                });
            }
            // Normalized HSD GameStatus includes virtual stick directions (controller.c:285-288)
            // and the shared shield bit, in addition to the physical SDK buttons.
            if pad.buttons.0 & !0x80ff1f7f != 0 {
                return Err(StepError::InvalidInput {
                    port,
                    reason: "unknown controller button bits",
                });
            }
        }
        Ok(())
    }
}
