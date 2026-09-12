use melee_lib::{diagnostics, *};
use melee_replay::{Recording, Sample, MAX_TICKS};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    path::Path,
    sync::OnceLock,
};

thread_local! {
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}
struct Allocator;
fn count() {
    if COUNTING.try_with(Cell::get).unwrap_or(false) {
        ALLOCATIONS.with(|n| n.set(n.get() + 1));
    }
}
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count();
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count();
        unsafe { System.realloc(ptr, layout, size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: Allocator = Allocator;
fn config() -> MatchConfig {
    MatchConfig::versus(
        Stage::FinalDestination,
        [
            PlayerConfig::new(Port::P1, Character::Fox),
            PlayerConfig::new(Port::P2, Character::Marth),
        ],
    )
    .with_seed(Seed(42))
}
fn assets() -> Option<&'static GameAssets> {
    static ASSETS: OnceLock<GameAssets> = OnceLock::new();
    let files = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files");
    if !melee_test_support::require_files([files.join("PlCo.dat")]) {
        return None;
    }
    Some(ASSETS.get_or_init(|| GameAssets::load(files, &config()).unwrap()))
}
fn roundtrip(recording: &Recording) -> Recording {
    let mut bytes = Vec::new();
    recording.write(&mut bytes).unwrap();
    Recording::read(bytes.as_slice()).unwrap()
}
#[test]
fn replay_preserves_all_controller_bits() {
    let words = [[
        0x80000040, 0x80000000, 0x3eaaaaab, 0xbf800000, 0x00000001, 0x3ecccccd, 0x3f800000,
    ]; 4];
    let sample = Sample(words);
    let encoded = serde_json::to_vec(&sample).unwrap();
    let decoded: Sample = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(Sample::from(decoded.inputs()), sample);
}
#[test]
fn replay_matches_direct_continuation_and_rejects_asset_drift() {
    let Some(assets) = assets() else {
        return;
    };
    let mut recording = Recording::new(&config(), assets);
    let mut direct = Match::new(assets, config()).unwrap();
    for frame in 0..300 {
        let mut inputs = Inputs::default();
        if (100..110).contains(&frame) {
            inputs[Port::P1].stick.x = 1.0;
        }
        if frame == 150 {
            inputs[Port::P2].buttons = Buttons::X;
        }
        recording.push(inputs).unwrap();
        direct.step(&inputs).unwrap();
    }
    let decoded = roundtrip(&recording);
    assert_eq!(decoded.config.decode().unwrap(), config());
    let replayed = decoded.replay(assets).unwrap();
    assert_eq!(
        diagnostics::inspect(&replayed).unwrap(),
        diagnostics::inspect(&direct).unwrap()
    );
    let mut json = serde_json::to_value(&recording).unwrap();
    json["asset_fingerprint"] = serde_json::json!(0);
    let altered = Recording::read(serde_json::to_vec(&json).unwrap().as_slice()).unwrap();
    assert!(altered
        .replay(assets)
        .err()
        .unwrap()
        .message
        .contains("fingerprint"));
    json["version"] = serde_json::json!(999);
    assert!(Recording::read(serde_json::to_vec(&json).unwrap().as_slice()).is_err());
}
#[test]
fn failing_input_and_diagnostic_survive_export() {
    let Some(assets) = assets() else {
        return;
    };
    let mut game = Match::new(assets, config()).unwrap();
    let mut recording = Recording::new(&config(), assets);
    for _ in 0..100 {
        recording.push(Inputs::default()).unwrap();
        game.step(&Inputs::default()).unwrap();
    }
    let mut invalid = Inputs::default();
    invalid[Port::P2].left_trigger = 2.0;
    recording.push(invalid).unwrap();
    let message = game.step(&invalid).unwrap_err().to_string();
    recording.fail(message.clone());
    let decoded = roundtrip(&recording);
    let fault = decoded.replay(assets).err().unwrap();
    assert_eq!(fault.attempt, 101);
    assert_eq!(fault.message, message);
    assert_eq!(decoded.fault.unwrap().message, message);
    assert!(recording.push(Inputs::default()).is_err());
}
#[test]
fn full_recording_and_reset_never_allocate_or_drop_the_prefix() {
    let Some(assets) = assets() else {
        return;
    };
    let mut recording = Recording::new(&config(), assets);
    let mut first = Inputs::default();
    first[Port::P1].buttons = Buttons::A;
    ALLOCATIONS.with(|n| n.set(0));
    COUNTING.with(|c| c.set(true));
    recording.push(first).unwrap();
    for _ in 1..MAX_TICKS {
        recording.push(Inputs::default()).unwrap();
    }
    assert!(recording.push(Inputs::default()).is_err());
    assert_eq!(recording.samples()[0], Sample::from(first));
    recording.reset();
    recording.push(first).unwrap();
    COUNTING.with(|c| c.set(false));
    assert_eq!(ALLOCATIONS.with(Cell::get), 0);
    assert_eq!(recording.samples().len(), 1);
}

#[test]
fn manual_export_replaces_chosen_file_and_automatic_export_preserves_it() {
    let Some(assets) = assets() else {
        return;
    };
    let mut recording = Recording::new(&config(), assets);
    let path =
        std::env::temp_dir().join(format!("melee-export-replace-{}.json", std::process::id()));
    std::fs::write(&path, "old recording").unwrap();
    assert!(recording.save_new(&path).is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "old recording");
    recording.push(Inputs::default()).unwrap();
    recording.save(&path).unwrap();
    assert_eq!(Recording::load(&path).unwrap().samples().len(), 1);
    std::fs::remove_file(path).unwrap();
}
#[test]
fn recording_and_stepping_together_allocate_nothing() {
    let Some(assets) = assets() else {
        return;
    };
    let mut game = Match::new(assets, config()).unwrap();
    let mut recording = Recording::new(&config(), assets);
    ALLOCATIONS.with(|n| n.set(0));
    COUNTING.with(|c| c.set(true));
    for _ in 0..600 {
        let input = Inputs::default();
        recording.push(input).unwrap();
        game.step(&input).unwrap();
    }
    COUNTING.with(|c| c.set(false));
    assert_eq!(ALLOCATIONS.with(Cell::get), 0);
}
