//! Errors distinguish rejected requests from a fault during simulation.
use crate::Port;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StartError {
    InvalidConfig(&'static str),
    IncompatibleAssets,
    Load(String),
    Initialization(String),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StepError {
    InvalidInput { port: Port, reason: &'static str },
    Finished,
    Faulted,
    Simulation(String),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateError {
    Faulted,
}

impl std::fmt::Display for StartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidConfig(reason) => write!(f, "invalid match configuration: {reason}"),
            Self::IncompatibleAssets => {
                f.write_str("assets do not match the selected stage and characters")
            }
            Self::Load(reason) => write!(f, "loading game assets: {reason}"),
            Self::Initialization(reason) => write!(f, "initializing match: {reason}"),
        }
    }
}
impl std::fmt::Display for StepError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput { port, reason } => write!(f, "invalid {port:?} input: {reason}"),
            Self::Finished => f.write_str("match has finished"),
            Self::Faulted => f.write_str("match is faulted; reset or replace it before stepping"),
            Self::Simulation(reason) => write!(f, "simulation failed: {reason}"),
        }
    }
}
impl std::fmt::Display for StateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("match is faulted; reset or replace it before observing")
    }
}
impl std::error::Error for StartError {}
impl std::error::Error for StepError {}
impl std::error::Error for StateError {}
