//! The CLI fixture is a reproducible gate product; a disabled recorder does no
//! serialization or allocation, even when handed unsupported fixture inputs.
#[path = "../src/fixture_spawns.rs"]
mod recorder;

use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    path::Path,
    process::Command,
};

struct Allocator;
thread_local! {
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}
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

#[test]
fn absent_sink_does_not_allocate_or_validate_event_payloads() {
    let mut sink = recorder::EventSink::default();
    let mut request = hsd_particle::system::SpawnRequest::new(0, 9, 0);
    // Enabled recording rejects velocity overrides. Disabled recording must not
    // inspect or serialize them, since they are ordinary simulation inputs.
    request.velocity = Some([1.0; 3]);
    let matrix = hsd_types::Mtx::IDENTITY;
    ALLOCATIONS.with(|n| n.set(0));
    COUNTING.with(|flag| flag.set(true));
    for tick in 0..600 {
        sink.begin_tick(tick);
        sink.spawn(&request, false, false);
        sink.update_joint(1, matrix);
        sink.expire_joint(1);
        sink.flags(1, 0x600, 0x800);
        sink.external_randf(0x8006_3b70);
    }
    COUNTING.with(|flag| flag.set(false));
    assert_eq!(ALLOCATIONS.with(Cell::get), 0);
    sink.enable();
    assert!(sink.finish().is_empty());
}

#[test]
fn ledge_cli_output_equals_checked_in_fixture() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let scenario_path = root.join("harness/scenarios/ledge_fd_fox.toml");
    let scenario = melee_sim::scenario::Scenario::load(&scenario_path).unwrap();
    if let Some(missing) = scenario.required_files().iter().find(|p| !p.is_file()) {
        eprintln!("skipping fixture CLI: {} absent", missing.display());
        return;
    }
    let out = std::env::temp_dir().join(format!("c13-cli-fixture-{}.json", std::process::id()));
    let result = Command::new(env!("CARGO_BIN_EXE_melee-sim"))
        .arg("fixture-spawns")
        .arg(&scenario_path)
        .arg("--out")
        .arg(&out)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let actual: serde_json::Value = serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
    let expected: serde_json::Value = serde_json::from_str(include_str!(
        "../../hsd-particle/tests/data/ledge_fd_spawns.json"
    ))
    .unwrap();
    assert_eq!(actual, expected);
    let prefix_ticks = scenario.frames / 2;
    let result = Command::new(env!("CARGO_BIN_EXE_melee-sim"))
        .arg("fixture-spawns")
        .arg(&scenario_path)
        .arg("--out")
        .arg(&out)
        .arg("--ticks")
        .arg(prefix_ticks.to_string())
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let mut expected_prefix = expected.clone();
    expected_prefix
        .as_object_mut()
        .unwrap()
        .retain(|frame, _| frame.parse::<u64>().unwrap() < prefix_ticks);
    let prefix: serde_json::Value = serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
    assert_eq!(prefix, expected_prefix);
    // Failure must leave an existing output intact.
    let result = Command::new(env!("CARGO_BIN_EXE_melee-sim"))
        .arg("fixture-spawns")
        .arg(out.with_extension("missing.toml"))
        .arg("--out")
        .arg(&out)
        .output()
        .unwrap();
    assert!(!result.status.success());
    let preserved: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
    assert_eq!(preserved, expected_prefix);
    std::fs::remove_file(out).unwrap();
}
