use melee_lib::{diagnostics, GameAssets};
use melee_platform::session::{Action, Session};
use melee_replay::Recording;
use std::{path::PathBuf, time::Duration};

fn directory() -> Option<PathBuf> {
    let files = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files");
    melee_test_support::require_files([files.join("PlCo.dat")]).then_some(files)
}
fn tick(session: &mut Session) -> Result<(), String> {
    session.advance(Duration::from_nanos(16_666_667))
}
#[test]
fn ui_consumed_inputs_replay_after_pause_focus_and_reset() {
    let Some(files) = directory() else {
        return;
    };
    let mut session = Session::new(&files, Session::default_config()).unwrap();
    for _ in 0..110 {
        tick(&mut session).unwrap();
    }
    session.set_action(0, Action::Jump, true);
    session.set_action(0, Action::Jump, false);
    tick(&mut session).unwrap();
    let count = session.recording().samples().len();
    session.pause(true);
    tick(&mut session).unwrap();
    session.set_focused(false);
    session.set_focused(true);
    tick(&mut session).unwrap();
    assert_eq!(session.recording().samples().len(), count);
    session.pause(false);
    for _ in 0..100 {
        tick(&mut session).unwrap();
    }
    let path = std::env::temp_dir().join(format!("melee-ui-replay-{}.json", std::process::id()));
    session.save_replay(&path).unwrap();
    let replay = Recording::load(&path).unwrap();
    std::fs::remove_file(path).unwrap();
    let assets = GameAssets::load(&files, &replay.config.decode().unwrap()).unwrap();
    let reproduced = replay.replay(&assets).unwrap();
    assert_eq!(
        diagnostics::inspect(session.game()).unwrap(),
        diagnostics::inspect(&reproduced).unwrap()
    );
    session.reset().unwrap();
    assert!(session.recording().samples().is_empty());
    tick(&mut session).unwrap();
    assert_eq!(session.recording().samples().len(), 1);
}

#[test]
fn ui_playtest_exports_first_fault_and_stops_until_reset() {
    let Some(files) = directory() else {
        return;
    };
    let mut session = Session::new(&files, Session::default_config()).unwrap();
    for _ in 0..110 {
        tick(&mut session).unwrap();
    }
    session.set_action(1, Action::Jump, true);
    tick(&mut session).unwrap();
    session.set_action(1, Action::Jump, false);
    for _ in 0..10 {
        tick(&mut session).unwrap();
    }
    session.set_action(1, Action::Down, true);
    session.set_action(1, Action::Special, true);
    let result = tick(&mut session);
    // This playtest currently reaches aerial Counter. Once ported, the same
    // input schedule must still agree with headless replay; do not require a bug.
    let replay = session.recording();
    let assets = GameAssets::load(&files, &replay.config.decode().unwrap()).unwrap();
    match result {
        Err(message) => {
            let fault = replay.fault.as_ref().expect("simulation fault recorded");
            let reproduced = replay.replay(&assets).err().expect("same fault headlessly");
            assert_eq!(fault.attempt, reproduced.attempt);
            assert_eq!(fault.message, reproduced.message);
            assert!(message.contains(&fault.message));
            let path = message
                .split("Replay saved: ")
                .nth(1)
                .expect("automatic export");
            let saved = Recording::load(std::path::Path::new(path)).unwrap();
            assert_eq!(saved.samples(), replay.samples());
            std::fs::remove_file(path).unwrap();
            let count = replay.samples().len();
            assert!(!session.needs_frame());
            assert_eq!(tick(&mut session).unwrap_err(), message);
            assert_eq!(session.recording().samples().len(), count);
        }
        Ok(()) => assert_eq!(
            diagnostics::inspect(session.game()).unwrap(),
            diagnostics::inspect(&replay.replay(&assets).unwrap()).unwrap()
        ),
    }
    session.reset().unwrap();
    assert!(session.needs_frame());
    assert!(session.recording().fault.is_none());
    tick(&mut session).unwrap();
}
