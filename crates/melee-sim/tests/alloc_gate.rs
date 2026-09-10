//! Allocation budget for the same 600-tick scene as the throughput benchmark.
use melee_sim::{frame::Simulation, initial_state::InitialState, scenario::Scenario, trace};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    path::Path,
};

struct CountingAllocator;
thread_local! {
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}
fn count() {
    if COUNTING.try_with(Cell::get).unwrap_or(false) {
        ALLOCATIONS.with(|count| count.set(count.get() + 1));
    }
}
unsafe impl GlobalAlloc for CountingAllocator {
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
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn measure(scenario: &Scenario, snapshot: bool) -> usize {
    let mut simulation = Simulation::with_inputs(
        InitialState::from_savestate_traces(scenario).unwrap(),
        trace::pad_script(scenario).unwrap(),
    );
    if snapshot {
        simulation.tick().unwrap();
    } else {
        simulation.tick_without_snapshot().unwrap();
    }
    ALLOCATIONS.with(|count| count.set(0));
    COUNTING.with(|enabled| enabled.set(true));
    for _ in 1..600 {
        if snapshot {
            simulation.tick().unwrap();
        } else {
            simulation.tick_without_snapshot().unwrap();
        }
    }
    COUNTING.with(|enabled| enabled.set(false));
    ALLOCATIONS.with(Cell::get)
}

#[test]
fn start_fd_fox_allocation_budget() {
    let scenario = Scenario::load(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/scenarios/start_fd_fox.toml"),
    )
    .unwrap();
    if !melee_test_support::require_files(scenario.required_files()) {
        return;
    }
    assert_eq!(scenario.frames, 600);
    let simulate = measure(&scenario, false);
    let snapshot = measure(&scenario, true);
    use std::io::Write;
    writeln!(std::io::stdout().lock(), "599 measured ticks: simulate-only {simulate} allocations ({:.6}/tick); with snapshot {snapshot} allocations ({:.6}/tick); snapshot overhead {} ({:.6}/tick)",
        simulate as f64 / 599.0, snapshot as f64 / 599.0, snapshot - simulate, (snapshot - simulate) as f64 / 599.0).unwrap();
    // Provisional C5 budget, not a zero-allocation claim. Remaining sites include
    // melee-ft fighter/effects.rs (deferred C4/C5-ft), anim/playback.rs and
    // collision/ecb.rs; melee-gr last/animation.rs; particle storage/draw logs;
    // and sim Effect::load. See PORT_NOTES/C5_C6_C10_PERF_LANE.md for the census.
    // A budget is an upper bound: recording changes may alter particle counts,
    // but must not raise either pre-existing allocation ceiling.
    assert!(
        simulate <= 27_301,
        "simulate-only allocation budget exceeded: {simulate}"
    );
    assert!(
        snapshot <= 102_175,
        "record-producing allocation budget exceeded: {snapshot}"
    );
}
