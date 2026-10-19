//! What a `Match` costs to hold and to copy: retained heap per clone, the
//! largest allocations a clone makes, and new/clone/reset/600-tick timings.
//!
//!   cargo run --release -p melee-lib --example match_memory -- <files>
//!
//! `MATCH_MEMORY_TRACE=<bytes>` prints a backtrace for every allocation a
//! clone makes at or above that size (symbol names need an unstripped build).
use melee_lib::*;
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    hint::black_box,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed},
    time::Instant,
};

static LIVE: AtomicUsize = AtomicUsize::new(0);
static COUNT: AtomicUsize = AtomicUsize::new(0);
static TRACING: AtomicBool = AtomicBool::new(false);
static TRACE_AT: AtomicUsize = AtomicUsize::new(usize::MAX);
/// Every allocation size a clone makes, preallocated so recording never allocates.
static SIZES: std::sync::Mutex<Vec<usize>> = std::sync::Mutex::new(Vec::new());
thread_local! {
    static IN_TRACE: Cell<bool> = const { Cell::new(false) };
}

struct Heap;
impl Heap {
    fn record(size: usize) {
        LIVE.fetch_add(size, Relaxed);
        COUNT.fetch_add(1, Relaxed);
        if TRACING.load(Relaxed) {
            let mut sizes = SIZES.lock().unwrap();
            if sizes.len() < sizes.capacity() {
                sizes.push(size);
            }
        }
        if TRACING.load(Relaxed) && size >= TRACE_AT.load(Relaxed) {
            IN_TRACE.with(|busy| {
                if !busy.replace(true) {
                    let trace = std::backtrace::Backtrace::force_capture().to_string();
                    // Only frames in this workspace's crates, as file:line.
                    let frames: Vec<&str> = trace
                        .lines()
                        .map(str::trim)
                        .filter_map(|l| l.strip_prefix("at "))
                        .filter_map(|l| l.split_once("/crates/").map(|(_, path)| path))
                        .filter(|l| !l.contains("examples/match_memory"))
                        .take(12)
                        .collect();
                    eprintln!("{size}\t{}", frames.join(" | "));
                    busy.set(false);
                }
            });
        }
    }
}
unsafe impl GlobalAlloc for Heap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            Self::record(layout.size());
        }
        ptr
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            Self::record(layout.size());
        }
        ptr
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let result = unsafe { System.realloc(ptr, layout, size) };
        if !result.is_null() {
            LIVE.fetch_add(size, Relaxed);
            LIVE.fetch_sub(layout.size(), Relaxed);
            COUNT.fetch_add(1, Relaxed);
        }
        result
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Relaxed);
        unsafe { System.dealloc(ptr, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: Heap = Heap;

/// Mean wall time of `n` runs of `f`, in milliseconds.
fn time_ms<T>(n: u32, mut f: impl FnMut() -> T) -> f64 {
    let start = Instant::now();
    for _ in 0..n {
        black_box(f());
    }
    start.elapsed().as_secs_f64() * 1e3 / f64::from(n)
}

fn main() {
    let files = std::env::args()
        .nth(1)
        .expect("usage: match_memory <extracted disc files>");
    let config = MatchConfig::versus(
        Stage::FinalDestination,
        [
            PlayerConfig::new(Port::P1, Character::Fox),
            PlayerConfig::new(Port::P2, Character::Marth),
        ],
    )
    .with_seed(Seed(42));
    let before_assets = LIVE.load(Relaxed);
    let assets = GameAssets::load(&files, &config).expect("game assets");
    let asset_bytes = LIVE.load(Relaxed) - before_assets;

    let before_match = LIVE.load(Relaxed);
    let mut game = Match::new(&assets, config.clone()).unwrap();
    let match_bytes = LIVE.load(Relaxed) - before_match;
    let ticks_ms = {
        let start = Instant::now();
        for _ in 0..600 {
            game.step(&Inputs::default()).unwrap();
        }
        start.elapsed().as_secs_f64() * 1e3
    };

    if let Some(at) = std::env::var("MATCH_MEMORY_TRACE")
        .ok()
        .and_then(|v| v.parse().ok())
    {
        TRACE_AT.store(at, Relaxed);
    }
    SIZES.lock().unwrap().reserve(1 << 20);
    let (before, count) = (LIVE.load(Relaxed), COUNT.load(Relaxed));
    TRACING.store(true, Relaxed);
    let copy = game.clone();
    TRACING.store(false, Relaxed);
    let clone_bytes = LIVE.load(Relaxed) - before;
    let clone_allocations = COUNT.load(Relaxed) - count;
    drop(copy);
    let mut by_size = std::collections::BTreeMap::<usize, usize>::new();
    for &size in SIZES.lock().unwrap().iter() {
        *by_size.entry(size).or_default() += 1;
    }
    let mut rows: Vec<_> = by_size.into_iter().collect();
    rows.sort_by_key(|&(size, n)| std::cmp::Reverse(size * n));
    println!("clone's allocations by size (top 25 by total bytes):");
    for (size, n) in rows.iter().take(25) {
        println!("  {size:>9} bytes x {n:>6} = {:>10}", size * n);
    }

    println!(
        "size_of::<Match>():     {} bytes",
        std::mem::size_of::<Match>()
    );
    println!("GameAssets::load heap:  {asset_bytes} bytes (shared by every match)");
    println!("Match::new heap:        {match_bytes} bytes");
    println!("Match::clone heap:      {clone_bytes} bytes in {clone_allocations} allocations");
    println!("600 ticks:              {ticks_ms:.1} ms");
    let new_ms = time_ms(20, || Match::new(&assets, config.clone()).unwrap());
    let clone_ms = time_ms(50, || game.clone());
    let mut destination = game.clone();
    let clone_from_ms = time_ms(50, || destination.clone_from(&game));
    let mut seed = 0;
    let reset_ms = time_ms(20, || {
        seed += 1;
        destination.reset(Seed(seed)).unwrap();
    });
    println!("Match::new:             {new_ms:.3} ms");
    println!("Match::clone (+drop):   {clone_ms:.3} ms");
    println!("Match::clone_from:      {clone_from_ms:.3} ms");
    println!("Match::reset:           {reset_ms:.3} ms");
}
