//! External events: what the game logic observes from the platform besides
//! the controllers, supplied per tick like [`Inputs`](crate::Inputs).
//!
//! Retail reads a few things no game state determines. Pokémon Stadium's
//! transformation reads the next form's archive from the disc and polls
//! for the read's completion once per tick (`grStadium_801D42B8`); when
//! the poll succeeds depends on the disc (in Dolphin, its emulated DVD
//! timing), not on the match. A recorded match supplies the completion it
//! observed, so a replay reproduces every transformation; a standalone
//! match lets the port decide by a documented default policy.
//!
//! Every field defaults to "the port decides" ([`StageRead::Default`]), so
//! [`Match::step`](crate::Match::step) keeps its meaning.

/// One tick's external events. `Default` lets the port decide each one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExternalEvents {
    /// The asynchronous stage archive read, if the stage polls one this tick.
    pub stage_read: StageRead,
}

/// Whether the stage's asynchronous archive read has completed when the
/// stage polls it this tick. Only Pokémon Stadium's transformation polls
/// (from the form's choice until its announcement); on other ticks and
/// stages the value is not read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StageRead {
    /// The port's default latency policy: each form archive completes on a
    /// fixed poll per file (`melee_gr::stadium::transform::DEFAULT_READ_POLLS`,
    /// measured from retail's first read of a match). A plausible sample of
    /// retail's disc timing, not a prediction of any particular match.
    #[default]
    Default,
    /// Recorded: this tick's poll found the read complete.
    Completed,
    /// Recorded: this tick's poll found the read still in flight.
    InFlight,
    /// A replay whose recording did not capture the read. A poll fails the
    /// tick (the port never guesses on behalf of a recording).
    Unrecorded,
}

/// What the tick's game logic consumed from its [`ExternalEvents`]: the
/// outcome of each poll it made, whatever decided it. Recording these
/// reproduces a standalone match exactly on replay.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ConsumedEvents {
    /// `Some(completed)` when the stage polled its archive read this tick.
    pub stage_read: Option<bool>,
}
