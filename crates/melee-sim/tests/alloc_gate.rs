//! Allocation census and strict simulate-only budgets across five scene types.
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

#[derive(Debug)]
struct Measurement {
    total: usize,
    peak: usize,
    allocating_ticks: usize,
}

fn measure(scenario: &Scenario, snapshot: bool) -> Measurement {
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
    let mut peak = 0;
    let mut allocating_ticks = 0;
    for _ in 1..scenario.frames {
        let before = ALLOCATIONS.with(Cell::get);
        COUNTING.with(|enabled| enabled.set(true));
        if snapshot {
            simulation.tick().unwrap();
        } else {
            simulation.tick_without_snapshot().unwrap();
        }
        COUNTING.with(|enabled| enabled.set(false));
        let count = ALLOCATIONS.with(Cell::get) - before;
        peak = peak.max(count);
        allocating_ticks += usize::from(count != 0);
    }
    Measurement {
        total: ALLOCATIONS.with(Cell::get),
        peak,
        allocating_ticks,
    }
}

fn allocation_budget(name: &str, ceiling: usize) {
    let scenario = Scenario::load(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../harness/scenarios/{name}.toml")),
    )
    .unwrap();
    if !melee_test_support::require_files(scenario.required_files()) {
        return;
    }
    let ticks = scenario.frames - 1;
    let simulate = measure(&scenario, false);
    let snapshot = measure(&scenario, true);
    use std::io::Write;
    writeln!(std::io::stdout().lock(),
        "{name}: {ticks} measured ticks; simulate-only {} ({:.6}/tick), peak {}, allocating ticks {}; with snapshot {} ({:.6}/tick), peak {}; snapshot overhead {} ({:.6}/tick)",
        simulate.total, simulate.total as f64 / ticks as f64, simulate.peak,
        simulate.allocating_ticks, snapshot.total, snapshot.total as f64 / ticks as f64,
        snapshot.peak, snapshot.total as i64 - simulate.total as i64,
        (snapshot.total as f64 - simulate.total as f64) / ticks as f64,
    ).unwrap();
    assert!(
        simulate.total <= ceiling,
        "{name}: simulate-only allocations increased: {} > {ceiling}",
        simulate.total,
    );
    // Owned diagnostic snapshots are reported above, without an asserted budget.
}

#[test]
fn start_fd_fox_allocation_budget() {
    // Remaining: entry-effect queues/models and command part attachment; C4/C8 bring this to zero.
    allocation_budget("start_fd_fox", 2_621);
}
#[test]
fn idle_fd_fox_allocation_budget() {
    // Remaining: effect queues and CommandState::step_inner/apply_part loaders; C4/C8 bring this to zero.
    allocation_budget("idle_fd_fox", 2_116);
}
#[test]
fn jab_fd_marth_allocation_budget() {
    // Remaining: effect queues/models and command part attachment; C4/C8 bring this to zero.
    allocation_budget("jab_fd_marth", 1_018);
}
#[test]
fn ko_fd_marth_allocation_budget() {
    // Remaining: death/respawn effect storage and command part attachment; C4/C8 bring this to zero.
    allocation_budget("ko_fd_marth", 2_386);
}
#[test]
fn start_bf_fox_allocation_budget() {
    // Remaining: entry-effect storage and command part attachment; C4/C8 bring this to zero.
    allocation_budget("start_bf_fox", 2_358);
}
