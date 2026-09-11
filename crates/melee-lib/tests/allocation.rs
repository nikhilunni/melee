use melee_lib::*;
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    path::Path,
};
thread_local! {
    static ENABLED: Cell<bool> = const { Cell::new(false) };
    static COUNT: Cell<usize> = const { Cell::new(0) };
    static TRACE: Cell<bool> = const { Cell::new(false) };
    static TRACK_HEAP: Cell<bool> = const { Cell::new(false) };
    static HEAP_DELTA: Cell<isize> = const { Cell::new(0) };
}
fn account(bytes: isize) {
    if TRACK_HEAP.try_with(Cell::get).unwrap_or(false) {
        HEAP_DELTA.with(|value| value.set(value.get() + bytes));
    }
}
struct Allocator;
fn count() {
    if ENABLED.try_with(Cell::get).unwrap_or(false) {
        COUNT.with(|c| c.set(c.get() + 1));
        if TRACE.with(Cell::get) {
            ENABLED.with(|v| v.set(false));
            eprintln!("{}", std::backtrace::Backtrace::force_capture());
            ENABLED.with(|v| v.set(true));
        }
    }
}
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count();
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            account(layout.size() as isize);
        }
        ptr
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count();
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            account(layout.size() as isize);
        }
        ptr
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count();
        let ptr = unsafe { System.realloc(ptr, layout, size) };
        if !ptr.is_null() {
            account(size as isize - layout.size() as isize);
        }
        ptr
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        account(-(layout.size() as isize));
        unsafe { System.dealloc(ptr, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: Allocator = Allocator;
fn measure(game: &mut Match, inputs: &Inputs) -> usize {
    COUNT.with(|v| v.set(0));
    ENABLED.with(|v| v.set(true));
    let result = game.step(inputs);
    if result.is_ok() {
        let view = game.observe().unwrap();
        for fighter in view.fighters() {
            std::hint::black_box(fighter.position());
        }
        std::hint::black_box(view.items().count());
    }
    ENABLED.with(|v| v.set(false));
    result.unwrap();
    COUNT.with(Cell::get)
}
#[test]
fn step_after_clone_and_clone_from_allocates_nothing() {
    TRACE.with(|v| v.set(std::env::var_os("MELEE_ALLOC_TRACE").is_some()));
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for name in ["match_fd_foxmarth", "match2_fd_foxmarth"] {
        let scenario = melee_sim::scenario::Scenario::load(
            &root.join(format!("harness/scenarios/{name}.toml")),
        )
        .unwrap();
        if !melee_test_support::require_files(scenario.required_files()) {
            return;
        }
        let mut original = diagnostics::import_match(&scenario).unwrap();
        let pads = melee_sim::trace::pad_script(&scenario).unwrap();
        let mut cloned = original.clone();
        let mut restored = original.clone();
        for frame in 0..scenario.frames {
            if !original.status().is_running() {
                break;
            }
            if frame % 509 == 0 {
                cloned = original.clone();
                restored.clone_from(&original);
            }
            let inputs = Inputs(std::array::from_fn(|p| pads.sample(frame, p)));
            for (label, game) in [
                ("original", &mut original),
                ("clone", &mut cloned),
                ("clone_from", &mut restored),
            ] {
                let allocations = measure(game, &inputs);
                assert_eq!(allocations, 0, "{name} {label} tick {frame}");
            }
        }
    }
}

#[test]
fn cold_match_step_and_observe_allocate_nothing() {
    let files = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files");
    if !melee_test_support::require_files([files.join("PlCo.dat")]) {
        return;
    }
    let config = MatchConfig::versus(
        Stage::FinalDestination,
        [
            PlayerConfig::new(Port::P1, Character::Fox),
            PlayerConfig::new(Port::P2, Character::Marth),
        ],
    )
    .with_seed(Seed(42));
    let assets = GameAssets::load(files, &config).unwrap();
    let mut game = Match::new(&assets, config).unwrap();
    let mut branch = game.clone();
    for tick in 0..600 {
        for game in [&mut game, &mut branch] {
            assert_eq!(measure(game, &Inputs::default()), 0, "cold tick {tick}");
        }
    }
}

#[test]
fn dropping_matches_and_assets_releases_every_owned_allocation() {
    let files = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files");
    if !melee_test_support::require_files([files.join("PlCo.dat")]) {
        return;
    }
    let config = MatchConfig::versus(
        Stage::FinalDestination,
        [
            PlayerConfig::new(Port::P1, Character::Fox),
            PlayerConfig::new(Port::P2, Character::Marth),
        ],
    )
    .with_seed(Seed(42));
    // Initialize existing immutable singleton data before measuring ownership.
    {
        let assets = GameAssets::load(&files, &config).unwrap();
        drop(Match::new(&assets, config.clone()).unwrap());
    }
    struct Tracking;
    impl Drop for Tracking {
        fn drop(&mut self) {
            TRACK_HEAP.with(|flag| flag.set(false));
        }
    }
    HEAP_DELTA.with(|bytes| bytes.set(0));
    TRACK_HEAP.with(|flag| flag.set(true));
    let tracking = Tracking;
    {
        let assets = GameAssets::load(&files, &config).unwrap();
        let mut game = Match::new(&assets, config).unwrap();
        let mut branch = game.clone();
        branch.reset(Seed(67)).unwrap();
        branch.clone_from(&game);
        game.step(&Inputs::default()).unwrap();
    }
    drop(tracking);
    assert_eq!(
        HEAP_DELTA.with(Cell::get),
        0,
        "owned match/resource allocation leaked"
    );
}
