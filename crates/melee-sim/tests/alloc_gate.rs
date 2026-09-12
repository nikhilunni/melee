//! Allocation census and strict simulate-only budgets across combat, movement and item scenes.
use melee_sim::{frame::Simulation, initial_state::InitialState, scenario::Scenario, trace};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    path::Path,
};

#[path = "alloc_gate/revival.rs"]
mod revival;

struct CountingAllocator;
thread_local! {
    static BACKTRACES: Cell<bool> = const { Cell::new(false) };
    static TICK: Cell<u64> = const { Cell::new(0) };
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}
fn count() {
    if COUNTING.try_with(Cell::get).unwrap_or(false) {
        ALLOCATIONS.with(|count| count.set(count.get() + 1));
        if BACKTRACES.with(Cell::get) {
            COUNTING.with(|enabled| enabled.set(false));
            eprintln!(
                "allocation at tick {}:\n{}",
                TICK.with(Cell::get),
                std::backtrace::Backtrace::force_capture()
            );
            COUNTING.with(|enabled| enabled.set(true));
        }
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
    // Optional diagnostic pass; backtrace storage itself is excluded from counting.
    BACKTRACES.with(|enabled| {
        enabled.set(!snapshot && std::env::var_os("MELEE_ALLOC_BACKTRACES").is_some())
    });
    for tick in 1..scenario.frames {
        TICK.with(|current| current.set(tick));
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
    // Remaining: particle/generator/AppSRT storage; shared commands allocate nothing.
    allocation_budget("start_fd_fox", 17);
}
#[test]
fn idle_fd_fox_allocation_budget() {
    // Remaining: one particle-storage growth allocation.
    allocation_budget("idle_fd_fox", 1);
}
#[test]
fn jab_fd_marth_allocation_budget() {
    // Remaining: particle/generator/AppSRT storage.
    allocation_budget("jab_fd_marth", 12);
}
#[test]
fn ko_fd_marth_allocation_budget() {
    // Revival reuses its owners; remaining allocations belong to particle storage.
    allocation_budget("ko_fd_marth", 22);
}
#[test]
fn start_bf_fox_allocation_budget() {
    // Remaining: particle/generator/AppSRT storage.
    allocation_budget("start_bf_fox", 17);
}

#[test]
fn jabcombo_fd_fox_allocation_budget() {
    allocation_budget("jabcombo_fd_fox", 0);
}
#[test]
fn fsmashcharge_fd_fox_allocation_budget() {
    allocation_budget("fsmashcharge_fd_fox", 0);
}
#[test]
fn laser_fd_fox_allocation_budget() {
    // Measured after the exact 300-tick port; existing scene ceilings stay unchanged.
    allocation_budget("laser_fd_fox", 32);
}

#[test]
fn laser_shield_fd_marth_allocation_budget() {
    allocation_budget("laser_shield_fd_marth", 0);
    allocation_budget("laser_shield_deflect_fd_marth", 0);
}

// S3: item and trail creation remain inside the prepared pools.
#[test]
fn airillusion_fd_fox_allocation_budget() {
    allocation_budget("airillusion_fd_fox", 0);
}

// S5: input-driven launch correction and a getup attack that hits the opponent.
#[test]
fn sdi_fsmash_fd_marth_allocation_budget() {
    allocation_budget("sdi_fsmash_fd_marth", 0);
}
#[test]
fn getupattack_fd_fox_allocation_budget() {
    allocation_budget("getupattack_fd_fox", 0);
}

// S2: first-use aerial and landing effect allocations remain zero.
#[test]
fn nairlc_fd_fox_allocation_budget() {
    allocation_budget("nairlc_fd_fox", 0);
}
#[test]
fn dair_fd_marth_allocation_budget() {
    allocation_budget("dair_fd_marth", 0);
}
// S3 part 2: charge flames and launch model use the prepared effect pools.
#[test]
fn firefox_fd_fox_allocation_budget() {
    allocation_budget("firefox_fd_fox", 0);
}

// S3: complete trigger, flash, counterattack and target knockdown.
#[test]
fn counter_fd_marth_allocation_budget() {
    allocation_budget("counter_fd_marth", 0);
}

// S10: four stocks of respawn, teeter and KO; ceiling measured on the exact tree.
#[test]
fn match_fd_marth_scripted_allocation_budget() {
    allocation_budget("match_fd_marth_scripted", 0);
}

#[test]
fn topko_usmash_fd_fox_allocation_budget() {
    allocation_budget("topko_usmash_fd_fox", 0);
}

#[test]
fn hi200_utilt_fd_marth_allocation_budget() {
    allocation_budget("hi200_utilt_fd_marth", 0);
}

#[test]
fn hi200_dolphinslash_fd_marth_allocation_budget() {
    allocation_budget("hi200_dolphinslash_fd_marth", 0);
}

// S6: tilted and analog shields, including contact and shieldstun recovery.
#[test]
fn shieldtilt_ftilt_fd_marth_allocation_budget() {
    allocation_budget("shieldtilt_ftilt_fd_marth", 0);
}

#[test]
fn lightshield_ftilt_fd_marth_allocation_budget() {
    allocation_budget("lightshield_ftilt_fd_marth", 0);
}

#[test]
fn powershield_ftilt_fd_marth_allocation_budget() {
    allocation_budget("powershield_ftilt_fd_marth", 0);
}

#[test]
fn shieldbreak_fd_marth_allocation_budget() {
    allocation_budget("shieldbreak_fd_marth", 0);
}

#[test]
fn topko_usmash_long_fd_fox_allocation_budget() {
    allocation_budget("topko_usmash_long_fd_fox", 0);
}

#[test]
fn human_smoke_fd_marth_allocation_budget() {
    allocation_budget("human_smoke_fd_marth", 0);
}

// S7/S8: capture mash and the first-use ledge attack.
#[test]
fn grabmash_fd_marth_allocation_budget() {
    allocation_budget("grabmash_fd_marth", 0);
}
#[test]
fn ledgeattack_fd_fox_allocation_budget() {
    allocation_budget("ledgeattack_fd_fox", 0);
}

#[test]
fn s7_back_throw_borrows_motion_and_commands_without_allocating() {
    allocation_budget("grab_fd_marth", 0);
}

#[test]
fn uthrow_fd_marth_allocation_budget() {
    allocation_budget("uthrow_fd_marth", 0);
}

#[test]
fn pummel_fd_marth_allocation_budget() {
    allocation_budget("pummel_fd_marth", 0);
}

#[test]
fn match_fd_foxmarth_allocation_budget() {
    allocation_budget("match_fd_foxmarth", 0);
}

#[test]
fn match2_fd_foxmarth_allocation_budget() {
    allocation_budget("match2_fd_foxmarth", 0);
}

#[test]
fn aerial_counter_allocation_budget() {
    allocation_budget("aircounter_fd_marth", 0);
    allocation_budget("aircounter_hit_fd_marth", 0);
    allocation_budget("aircounter_fall_fd_marth", 0);
}

#[test]
fn diagonal_smash_allocation_budget() {
    allocation_budget("fsmash_diagonal_fd_fox", 0);
    allocation_budget("fsmash_diagonal_fd_marth", 0);
    allocation_budget("fsmash_dash_diagonal_fd_fox", 0);
    allocation_budget("fsmash_dash_diagonal_fd_marth", 0);
}

#[test]
fn post_hitstun_allocation_budget() {
    allocation_budget("hitstun_exit_fair_fd_fox", 0);
    allocation_budget("hitstun_exit_fair_fd_marth", 0);
    allocation_budget("hitstun_exit_nair_fd_fox", 0);
    allocation_budget("hitstun_exit_nair_fd_marth", 0);
    allocation_budget("hitstun_shield_priority_fd_fox", 0);
    allocation_budget("hitstun_shield_priority_fd_marth", 0);
    allocation_budget("hitstun_unbuffered_attack_fd_marth", 0);
}

#[test]
fn reflector_input_allocation_budget() {
    allocation_budget("airreflectorturn_landing_fd_fox", 0);
    allocation_budget("reflectorturn_fd_fox", 0);
    allocation_budget("reflectorturn_release_fd_fox", 0);
    allocation_budget("airreflectorturn_fd_fox", 0);
    allocation_budget("airreflectorturn_priority_fd_fox", 0);
    allocation_budget("airreflectorjc_fd_fox", 0);
    allocation_budget("airreflectortapjc_fd_fox", 0);
}

#[test]
fn jumpcancel_upb_allocation_budget() {
    allocation_budget("jumpcancel_upb_fd_fox", 0);
    allocation_budget("jumpcancel_upb_fd_marth", 0);
    allocation_budget("jumpcancel_upb_priority_fd_fox", 0);
    allocation_budget("jumpcancel_upb_priority_fd_marth", 0);
    allocation_budget("jumpcancel_upb_diagonal_fd_fox", 0);
    allocation_budget("jumpcancel_upb_diagonal_fd_marth", 0);
}

#[test]
fn cstick_throw_allocation_budget() {
    allocation_budget("cstick_throw_back_fd_fox", 0);
    allocation_budget("cstick_throw_back_fd_marth", 0);
    allocation_budget("cstick_throw_down_fd_fox", 0);
    allocation_budget("cstick_throw_down_fd_marth", 0);
    allocation_budget("cstick_throw_down_pulse_fd_fox", 0);
    allocation_budget("cstick_throw_down_pulse_fd_marth", 0);
    allocation_budget("cstick_throw_forward_fd_fox", 0);
    allocation_budget("cstick_throw_forward_fd_marth", 0);
    allocation_budget("cstick_throw_horizontal_priority_fd_fox", 0);
    allocation_budget("cstick_throw_horizontal_priority_fd_marth", 0);
    allocation_budget("cstick_throw_main_priority_fd_fox", 0);
    allocation_budget("cstick_throw_main_priority_fd_marth", 0);
    allocation_budget("cstick_throw_up_fd_fox", 0);
    allocation_budget("cstick_throw_up_fd_marth", 0);
}
