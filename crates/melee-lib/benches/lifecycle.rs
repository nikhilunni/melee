//! Separate lifecycle latency and retained heap measurements, excluding disk load.
use melee_lib::*;
use std::{
    alloc::{GlobalAlloc, Layout, System},
    hint::black_box,
    sync::atomic::{AtomicUsize, Ordering::Relaxed},
    time::Instant,
};
static LIVE: AtomicUsize = AtomicUsize::new(0);
struct Heap;
unsafe impl GlobalAlloc for Heap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            LIVE.fetch_add(layout.size(), Relaxed);
        }
        ptr
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            LIVE.fetch_add(layout.size(), Relaxed);
        }
        ptr
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let result = unsafe { System.realloc(ptr, layout, size) };
        if !result.is_null() {
            LIVE.fetch_add(size, Relaxed);
            LIVE.fetch_sub(layout.size(), Relaxed);
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
fn main() {
    let directory =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files");
    let config = MatchConfig::versus(
        Stage::FinalDestination,
        [
            PlayerConfig::new(Port::P1, Character::Fox),
            PlayerConfig::new(Port::P2, Character::Marth),
        ],
    )
    .with_seed(Seed(42));
    let assets = GameAssets::load(directory, &config).expect("local game assets required");
    let mut game = Match::new(&assets, config.clone()).unwrap();
    for _ in 0..180 {
        game.step(&Inputs::default()).unwrap();
    }
    const SAMPLES: u32 = 50;
    let start = Instant::now();
    for _ in 0..SAMPLES {
        black_box(Match::new(&assets, config.clone()).unwrap());
    }
    println!("creation+drop: {:?}/match", start.elapsed() / SAMPLES);
    let mut destination = game.clone();
    let start = Instant::now();
    for seed in 0..SAMPLES {
        destination.reset(Seed(seed)).unwrap();
        black_box(&destination);
    }
    println!("reset: {:?}/match", start.elapsed() / SAMPLES);
    let start = Instant::now();
    for _ in 0..SAMPLES {
        black_box(game.clone());
    }
    println!("clone+drop: {:?}/match", start.elapsed() / SAMPLES);
    let start = Instant::now();
    for _ in 0..SAMPLES {
        destination.clone_from(&game);
        black_box(&destination);
    }
    println!("clone_from: {:?}/match", start.elapsed() / SAMPLES);
    for count in [1, 8, 32] {
        let before = LIVE.load(Relaxed);
        let matches: Vec<_> = (0..count).map(|_| game.clone()).collect();
        let bytes = LIVE.load(Relaxed) - before;
        println!(
            "{count} shared-asset matches: {bytes} retained heap bytes ({} each)",
            bytes / count
        );
        black_box(matches);
    }
}
