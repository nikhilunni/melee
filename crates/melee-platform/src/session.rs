//! Shared session policy: fixed simulation ticks and logical input latching.
use melee_lib::{presentation::Presentation, *};
use melee_replay::Recording;
use std::{path::Path, time::Duration};

pub use crate::input::{Action, Gamepad, PadButton};
use crate::input::PortInputs;

#[derive(Default)]
struct TickClock {
    phase: u128,
}
impl TickClock {
    fn advance(&mut self, elapsed: Duration) -> u32 {
        // Native pause/focus changes reset the clock. A long OS stall is treated
        // as a pause, avoiding seconds of delayed input catch-up after a suspend.
        if elapsed > Duration::from_millis(250) {
            self.phase = 0;
            return 0;
        }
        self.phase += elapsed.as_nanos() * 60;
        let ticks = (self.phase / 1_000_000_000).min(8) as u32;
        self.phase -= u128::from(ticks) * 1_000_000_000;
        ticks
    }
}
pub struct Session {
    seed: Seed,
    game: Match,
    presentation: Presentation,
    inputs: PortInputs,
    clock: TickClock,
    paused: bool,
    focused: bool,
    recording: Recording,
    failure: Option<String>,
}
impl Session {
    /// The match the app starts when nothing else is chosen.
    pub fn default_config() -> MatchConfig {
        MatchConfig::versus(
            Stage::FinalDestination,
            [
                PlayerConfig::new(Port::P1, Character::Fox),
                PlayerConfig::new(Port::P2, Character::Marth),
            ],
        )
        .with_seed(Seed(42))
    }
    pub fn new(source: &dyn FileSource, config: MatchConfig) -> Result<Self, String> {
        let assets = GameAssets::load_from(source, &config).map_err(|e| e.to_string())?;
        let recording = Recording::new(&config, &assets);
        let seed = config.seed;
        let game = Match::new(&assets, config).map_err(|e| e.to_string())?;
        let presentation = Presentation::new(&game).map_err(|e| e.to_string())?;
        Ok(Self {
            seed,
            game,
            presentation,
            inputs: PortInputs::default(),
            clock: TickClock::default(),
            paused: false,
            focused: true,
            recording,
            failure: None,
        })
    }
    pub fn game(&self) -> &Match {
        &self.game
    }
    pub fn presentation(&self) -> &Presentation {
        &self.presentation
    }
    /// A logical keyboard action on controller port `port` (0..4).
    pub fn set_action(&mut self, port: usize, action: Action, down: bool) {
        if port < 4 && !self.is_paused() {
            self.inputs.set_action(port, action, down);
        }
    }
    /// The gamepad on controller port `port` (0..4), or `None` once it is
    /// disconnected.
    pub fn set_gamepad(&mut self, port: usize, pad: Option<Gamepad>) {
        if port < 4 {
            let pad = pad.filter(|_| !self.is_paused());
            self.inputs.set_gamepad(port, pad);
        }
    }
    pub fn pause(&mut self, paused: bool) {
        self.paused = paused;
        self.clock = TickClock::default();
        self.inputs = PortInputs::default();
    }
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
        self.clock = TickClock::default();
        self.inputs = PortInputs::default();
    }
    pub fn toggle_pause(&mut self) {
        self.pause(!self.paused);
    }
    pub fn is_paused(&self) -> bool {
        self.paused || !self.focused
    }
    pub fn needs_frame(&self) -> bool {
        self.failure.is_none() && !self.is_paused() && self.game.status().is_running()
    }
    pub fn recording(&self) -> &Recording {
        &self.recording
    }
    pub fn save_replay(&self, path: &Path) -> Result<(), String> {
        self.recording.save(path)
    }
    /// The replay as file bytes, for hosts that save through their own API.
    pub fn replay_bytes(&self) -> Result<Vec<u8>, String> {
        let mut bytes = Vec::new();
        self.recording.write(&mut bytes)?;
        Ok(bytes)
    }
    /// The first fault's message, once the session has stopped on one.
    pub fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }

    /// Called only after an error; disk I/O and formatting never run on a healthy tick.
    fn stop_with_replay(&mut self, error: String) -> String {
        if let Some(previous) = &self.failure {
            return previous.clone();
        }
        let message = self.save_fault_replay(error);
        self.failure = Some(message.clone());
        message
    }

    /// The web has no file system: the host offers [`Session::replay_bytes`]
    /// as a download instead.
    #[cfg(target_arch = "wasm32")]
    fn save_fault_replay(&self, error: String) -> String {
        format!("{error}\nUse Save Replay to download the replay.")
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn save_fault_replay(&self, error: String) -> String {
        let directory = std::env::temp_dir().join("melee-replays");
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = directory.join(format!("fault-{}-{stamp}.json", std::process::id()));
        let saved = std::fs::create_dir_all(&directory)
            .map_err(|e| e.to_string())
            .and_then(|()| self.recording.save_new(&path));
        match saved {
            Ok(()) => format!("{error}\nReplay saved: {}", path.display()),
            Err(save_error) => {
                format!("{error}\nReplay save failed: {save_error}. Use Save Replay to retry.")
            }
        }
    }

    pub fn stop_after_host_fault(&mut self, message: String) -> String {
        // Host/presentation faults happen outside Match::step: all recorded
        // inputs succeeded. Do not mislabel the last successful tick as a fault.
        self.stop_with_replay(format!(
            "application fault after {} successful ticks: {message}",
            self.game.tick().0
        ))
    }
    pub fn reset(&mut self) -> Result<(), String> {
        self.game.reset(self.seed).map_err(|e| e.to_string())?;
        self.recording.reset();
        self.failure = None;
        self.paused = false;
        self.clock = TickClock::default();
        self.inputs = PortInputs::default();
        self.presentation
            .capture(&self.game)
            .map_err(|e| e.to_string())
    }
    pub fn advance(&mut self, elapsed: Duration) -> Result<(), String> {
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        if !self.needs_frame() {
            return Ok(());
        }
        let ticks = self.clock.advance(elapsed);
        for _ in 0..ticks {
            if !self.game.status().is_running() {
                break;
            }
            if self.recording.is_full() {
                return Err(self.stop_with_replay(
                    "30-minute replay capacity reached; save and restart the match".into(),
                ));
            }
            let inputs = self.inputs.sample();
            self.recording.push(inputs).map_err(str::to_owned)?;
            let game = &mut self.game;
            if let Err(error) = panic_replay::stepping(&self.recording, || game.step(&inputs)) {
                let message = format!("attempted tick {}: {error}", self.recording.samples().len());
                self.recording.fail(error.to_string());
                return Err(self.stop_with_replay(message));
            }
            self.recording.note_events(self.game.consumed_events());
        }
        if ticks > 0 {
            if let Err(error) = self.presentation.capture(&self.game) {
                return Err(self.stop_after_host_fault(error.to_string()));
            }
        }
        Ok(())
    }
}

/// wasm32-unknown-unknown aborts on panic, so `Match::step`'s `catch_unwind`
/// never returns a fault there. While a tick runs, the session's recording
/// (already holding that tick's input) is reachable from a panic hook, which
/// can offer it for download before the module aborts. Natively a panic
/// unwinds into a normal fault and this does nothing.
pub mod panic_replay {
    use melee_replay::Recording;

    #[cfg(target_arch = "wasm32")]
    thread_local! {
        static STEPPING: std::cell::Cell<*const Recording> =
            const { std::cell::Cell::new(std::ptr::null()) };
    }

    /// Run one tick with `recording` published to [`bytes`]. The caller
    /// borrows only the match mutably; the recording is not touched until
    /// `step` returns, so the hook's shared read cannot race a write.
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn stepping<T>(recording: &Recording, step: impl FnOnce() -> T) -> T {
        STEPPING.with(|current| current.set(recording));
        let result = step();
        STEPPING.with(|current| current.set(std::ptr::null()));
        result
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn stepping<T>(_recording: &Recording, step: impl FnOnce() -> T) -> T {
        step()
    }

    /// From a panic hook: the replay of the match whose tick is panicking.
    pub fn bytes() -> Option<Vec<u8>> {
        #[cfg(target_arch = "wasm32")]
        {
            let recording = STEPPING.with(|current| current.get());
            // SAFETY: non-null only inside `stepping`, whose caller holds a
            // shared borrow of the recording for the whole tick; wasm32 is
            // single-threaded, so the hook runs on that stack.
            let recording = unsafe { recording.as_ref() }?;
            let mut out = Vec::new();
            recording.write(&mut out).ok()?;
            Some(out)
        }
        #[cfg(not(target_arch = "wasm32"))]
        None
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn display_refresh_rate_does_not_set_simulation_rate() {
        for refresh in [30u64, 60, 120, 144] {
            let mut clock = TickClock::default();
            let mut ticks = 0;
            let mut previous = 0;
            for frame in 1..=refresh {
                let now = frame * 1_000_000_000 / refresh;
                ticks += clock.advance(Duration::from_nanos(now - previous));
                previous = now;
            }
            assert_eq!(ticks, 60);
        }
    }
}
