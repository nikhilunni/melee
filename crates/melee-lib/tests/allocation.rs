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

#[test]
fn presentation_capture_allocates_nothing_and_does_not_change_simulation() {
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
    let mut view = presentation::Presentation::new(&game).unwrap();
    let groups: std::collections::BTreeSet<_> = view
        .meshes()
        .iter()
        .map(|m| m.instance_group)
        .filter(|g| *g != 0)
        .collect();
    let mut saw_article = false;
    let mut saw_shield = false;
    let mut saw_particle = false;
    for tick in 0..600 {
        let mut inputs = Inputs::default();
        if tick >= 220 && tick % 60 < 30 {
            inputs.0[0].buttons = Buttons::B;
        }
        if (180..210).contains(&tick) {
            inputs.0[1].buttons = Buttons::L;
        }
        game.step(&inputs).unwrap();
        let before = if tick % 60 == 0 {
            Some(diagnostics::inspect(&game).unwrap())
        } else {
            None
        };
        COUNT.with(|v| v.set(0));
        ENABLED.with(|v| v.set(true));
        let result = view.capture(&game);
        ENABLED.with(|v| v.set(false));
        result.unwrap();
        assert_eq!(COUNT.with(Cell::get), 0, "presentation tick {tick}");
        let article_visible = view
            .meshes()
            .iter()
            .enumerate()
            .any(|(i, m)| m.instance_group != 0 && view.visibility()[i]);
        saw_article |= article_visible;
        saw_shield |= view
            .sprites()
            .iter()
            .any(|s| matches!(s.shape, presentation::SpriteShape::Shield { .. }));
        for shield in view
            .sprites()
            .iter()
            .filter(|s| matches!(s.shape, presentation::SpriteShape::Shield { .. }))
        {
            assert!(shield.color[3] > 0);
            assert!(shield.half_size.iter().all(|&radius| radius > 1.0));
        }
        saw_particle |= view
            .sprites()
            .iter()
            .any(|s| matches!(s.shape, presentation::SpriteShape::Texture));
        assert!(view.sprites().iter().all(|s| s
            .position
            .iter()
            .chain(&s.half_size)
            .all(|v| v.is_finite())));
        let instances: usize = groups.iter().map(|g| view.instance_range(*g).len()).sum();
        let projectiles = game
            .observe()
            .unwrap()
            .items()
            .filter(|item| item.velocity().x != 0.0)
            .count();
        assert_eq!(
            instances, projectiles,
            "projectile population at tick {tick}"
        );

        assert!(view
            .instances()
            .iter()
            .flatten()
            .flatten()
            .all(|v| v.is_finite()));

        if let Some(before) = before {
            assert_eq!(before, diagnostics::inspect(&game).unwrap());
        }
    }
    assert!(saw_article, "script must exercise live article instances");
    assert!(
        saw_shield && saw_particle,
        "script must exercise shields and particles"
    );
    game.reset(Seed(42)).unwrap();
    view.capture(&game).unwrap();
    assert!(groups.iter().all(|g| view.instance_range(*g).is_empty()));
}

#[test]
fn presentation_captures_both_complete_matches_without_allocating() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for name in ["match_fd_foxmarth", "match2_fd_foxmarth"] {
        let scenario = melee_sim::scenario::Scenario::load(
            &root.join(format!("harness/scenarios/{name}.toml")),
        )
        .unwrap();
        if !melee_test_support::require_files(scenario.required_files()) {
            return;
        }
        let mut game = diagnostics::import_match(&scenario).unwrap();
        let mut headless = game.clone();
        let pads = melee_sim::trace::pad_script(&scenario).unwrap();
        let mut view = presentation::Presentation::new(&game).unwrap();
        for frame in 0..scenario.frames {
            if !game.status().is_running() {
                break;
            }
            let inputs = Inputs(std::array::from_fn(|p| pads.sample(frame, p)));
            game.step(&inputs).unwrap();
            headless.step(&inputs).unwrap();
            COUNT.with(|v| v.set(0));
            ENABLED.with(|v| v.set(true));
            let result = view.capture(&game);
            ENABLED.with(|v| v.set(false));
            result.unwrap_or_else(|e| panic!("{name} frame {frame}: {e}"));
            assert_eq!(COUNT.with(Cell::get), 0, "{name} frame {frame}");
        }
        assert!(matches!(game.status(), MatchStatus::Finished(_)));
        assert_eq!(
            diagnostics::inspect(&game).unwrap(),
            diagnostics::inspect(&headless).unwrap()
        );
    }
}

#[test]
fn light_animation_depends_on_match_tick_not_capture_frequency() {
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
    let assets = GameAssets::load(&files, &config).unwrap();
    let mut game = Match::new(&assets, config).unwrap();
    let mut frequent = presentation::Presentation::new(&game).unwrap();
    let mut sparse = presentation::Presentation::new(&game).unwrap();
    let initial = frequent.directional_lights().to_vec();
    let mut saw_motion = false;
    for tick in 0..600 {
        game.step(&Inputs::default()).unwrap();
        frequent.capture(&game).unwrap();
        saw_motion |= initial != frequent.directional_lights();
        if tick % 73 == 0 {
            sparse.capture(&game).unwrap();
            assert_eq!(frequent.directional_lights(), sparse.directional_lights());
        }
    }
    assert!(saw_motion, "authored light paths must animate");
    let fresh = presentation::Presentation::new(&game).unwrap();
    assert_eq!(frequent.directional_lights(), fresh.directional_lights());
    game.reset(Seed(42)).unwrap();
    frequent.capture(&game).unwrap();
    assert_eq!(initial, frequent.directional_lights());
}

#[test]
fn material_animation_uses_match_state_and_prepared_images() {
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
    let mut frequent = presentation::Presentation::new(&game).unwrap();
    let mut sparse = presentation::Presentation::new(&game).unwrap();
    let initial: Vec<_> = frequent.materials().to_vec();
    let equal = |a: &presentation::Material, b: &presentation::Material| {
        a.diffuse == b.diffuse
            && a.ambient == b.ambient
            && a.specular == b.specular
            && a.textures.iter().zip(&b.textures).all(|(a, b)| {
                a.scale == b.scale
                    && a.translation == b.translation
                    && a.rotation == b.rotation
                    && a.blending == b.blending
                    && a.combiner == b.combiner
                    && a.image.rgba == b.image.rgba
            })
    };
    let mut animated = false;
    for tick in 0..600 {
        game.step(&Inputs::default()).unwrap();
        frequent.capture(&game).unwrap();
        animated |= frequent
            .materials()
            .iter()
            .zip(&initial)
            .any(|(a, b)| !equal(a, b));
        for material in frequent.materials() {
            for (texture, bank) in material.textures.iter().zip(&material.texture_banks) {
                assert!(bank
                    .iter()
                    .any(|image| std::sync::Arc::ptr_eq(image, &texture.image)));
            }
        }
        if tick % 73 == 0 {
            sparse.capture(&game).unwrap();
            assert!(frequent
                .materials()
                .iter()
                .zip(sparse.materials())
                .all(|(a, b)| equal(a, b)));
        }
    }
    assert!(animated, "stage materials must change during live playback");
    let fresh = presentation::Presentation::new(&game).unwrap();
    assert!(frequent
        .materials()
        .iter()
        .zip(fresh.materials())
        .all(|(a, b)| equal(a, b)));
    game.reset(Seed(42)).unwrap();
    frequent.capture(&game).unwrap();
    assert!(frequent
        .materials()
        .iter()
        .zip(&initial)
        .all(|(a, b)| equal(a, b)));
}

#[test]
fn stage_cycles_preserve_allocation_free_step_clone_and_capture() {
    TRACE.with(|v| v.set(std::env::var_os("MELEE_ALLOC_TRACE").is_some()));
    let files = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files");
    if !melee_test_support::require_files([files.join("GrNLa.dat")]) {
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
    let mut view = presentation::Presentation::new(&game).unwrap();
    let mut saw_overlay = false;
    for tick in 0..27_000 {
        if tick % 1200 == 0 {
            branch.clone_from(&game);
        }
        assert_eq!(
            measure(&mut game, &Inputs::default()),
            0,
            "stage tick {tick}"
        );
        assert_eq!(
            measure(&mut branch, &Inputs::default()),
            0,
            "stage clone tick {tick}"
        );
        COUNT.with(|v| v.set(0));
        ENABLED.with(|v| v.set(true));
        let result = view.capture(&game);
        ENABLED.with(|v| v.set(false));
        result.unwrap();
        assert_eq!(COUNT.with(Cell::get), 0, "stage capture tick {tick}");
        saw_overlay |= view.materials().iter().any(|m| m.overlay[3] > 0.0);
    }
    assert!(saw_overlay);
    assert_eq!(
        diagnostics::inspect(&game).unwrap(),
        diagnostics::inspect(&branch).unwrap()
    );
}
