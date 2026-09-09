//! HSD particle simulation and AppSRT display-cache bookkeeping, without GX rendering.
//!
//! Updates accept the caller's `HsdRng`. See `docs/PARTICLES.md` for the
//! supported scripts, retail instruction audit, and integration boundaries.
pub mod appsrt;
pub mod bank;
mod color;
mod direction;
pub mod generator;
pub mod particle;
mod program;
pub mod rng_sites;
pub mod system;

/// Simulation errors are explicit: an unknown instruction cannot silently
/// change program alignment or suppress an RNG consumer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    UnsupportedOpcode { opcode: u8, pc: u16 },
    TruncatedProgram { pc: u16 },
    InstructionLimit { pc: u16 },
    UnsupportedGenerator { shape: u16 },
    UnsupportedFeature(&'static str),
    InvalidLink(u8),
    InvalidEmissionCount,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedOpcode { opcode, pc } => write!(
                f,
                "unsupported particle opcode 0x{opcode:02X} at 0x{pc:04X}"
            ),
            other => write!(f, "particle simulation: {other:?}"),
        }
    }
}
impl std::error::Error for Error {}
