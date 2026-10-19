//! Jigglypuff's Rollout (ftpurinspecialn.c) and Kirby's copy of it
//! (ftkirbyspecialpurin.c), one set of states generic over [`RolloutKind`].
//!
//! The two files compile to the same code (the same fused operations at the
//! same places) over different rows, attribute blocks and callbacks: Kirby's
//! take-damage and death callbacks are his own (ftKb_Init_800EE7B8 /
//! 800EE74C) rather than the pose's restore, his saved scale is u.kb.x8C,
//! and his aerial roll's landing dust takes the roll's direction and angle
//! where Jigglypuff's takes her facing and the floor's angle.
mod attributes;
pub mod rollout;

pub use attributes::{RolloutAttributes, ROLLOUT_ATTRIBUTES_SIZE};
pub use rollout::{LandingDust, Rollout, RolloutKind, RolloutRow, RolloutRows, RolloutState};
