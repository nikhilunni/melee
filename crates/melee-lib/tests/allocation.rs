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
fn random_item_spawns_and_container_capture_allocate_nothing() {
    TRACE.with(|v| v.set(std::env::var_os("MELEE_ALLOC_TRACE").is_some()));
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        std::path::PathBuf::from,
    );
    let mut seen = [false; 3];
    for name in [
        "corpus_v3_fd_items_sword_e9af22990_p0",
        "corpus_v3_bf_items_sword_e49783a03_p1",
        "corpus_v3_fd_items_taru_ece1e43b7_p2",
        // Complete recorded lifecycles include three Box and two Barrel
        // blasts, including their deferred script/effect continuation.
        "corpus_v3_fd_items_sword_e7254fba4_p2",
        "corpus_v3_fd_items_taru_ee6ff560c_p0",
        // Article contacts transfer the primary fighter owner and create
        // the rolling hitbox in the same callback.
        "corpus_v3_bf_items_sword_e44c79d87_p2",
        "corpus_v3_fd_items_sword_e3b85eba6_p1",
        // Ground Sword throw, down-tilt and up-smash entries retain their
        // ordinary item synchronization and presentation capture paths.
        "corpus_v3_bf_items_sword_edb13a9c3_p2",
        "corpus_v3_fd_items_sword_e1611c835_p0",
        "corpus_v3_fd_items_sword_ece1e43b7_p0",
    ] {
        let scenario = melee_sim::scenario::Scenario::load(
            &root.join(format!("harness/scenarios/{name}.toml")),
        )
        .unwrap();
        if !melee_test_support::require_files(scenario.required_files()) {
            return;
        }
        let mut game = diagnostics::import_match(&scenario).unwrap();
        let mut presentation = presentation::Presentation::new(&game).unwrap();
        let pads = melee_sim::trace::pad_script(&scenario).unwrap();
        for frame in 0..scenario.frames {
            let inputs = Inputs(std::array::from_fn(|p| pads.sample(frame, p)));
            assert_eq!(measure(&mut game, &inputs), 0, "{name} tick {frame}");
            COUNT.with(|v| v.set(0));
            ENABLED.with(|v| v.set(true));
            let result = presentation.capture(&game);
            ENABLED.with(|v| v.set(false));
            result.unwrap_or_else(|error| panic!("{name} capture {frame}: {error}"));
            assert_eq!(COUNT.with(Cell::get), 0, "{name} capture {frame}");
            for item in game.observe().unwrap().items() {
                // Retail item kinds 0/1/2: Capsule, Box and Barrel.
                if let Ok(index) = usize::try_from(item.kind().0) {
                    if let Some(present) = seen.get_mut(index) {
                        *present = true;
                    }
                }
            }
        }
    }
    assert!(
        seen.into_iter().all(|present| present),
        "exercise every container"
    );
}

#[test]
fn imported_item_rules_survive_clone_and_reset_when_the_scenario_omits_them() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut scenario = melee_sim::scenario::Scenario::load(
        &root.join("harness/scenarios/corpus_v3_fd_items_sword_e9af22990_p0.toml"),
    )
    .unwrap();
    if !melee_test_support::require_files(scenario.required_files()) {
        return;
    }
    let rules = scenario.item_rules().unwrap();
    assert!(rules.enabled());
    assert!(!scenario.is_cold());
    scenario.item_frequency = None;
    scenario.item_mask = None;
    let imported = diagnostics::import_match(&scenario).unwrap();
    assert_eq!(imported.config().rules.items, rules);
    let mut cloned = imported.clone();
    cloned.reset(Seed(42)).unwrap();
    assert_eq!(cloned.config().rules.items, rules);
}

#[test]
fn imported_warm_team_rules_survive_clone_and_reset_without_metadata() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let scenario = melee_sim::scenario::Scenario::load(
        &root.join("harness/scenarios/start_fd_doubles_ff_fox_falco_marth_captainfalcon2.toml"),
    )
    .unwrap();
    if !melee_test_support::require_files(scenario.required_files()) {
        return;
    }
    assert!(!scenario.is_cold());
    assert!(!scenario.friendly_fire);
    let imported = diagnostics::import_match(&scenario).unwrap();
    assert!(imported.config().rules.friendly_fire);
    let teams: Vec<_> = imported
        .config()
        .players
        .iter()
        .map(|player| player.team)
        .collect();
    assert!(teams.iter().all(Option::is_some));
    let mut cloned = imported.clone();
    cloned.reset(Seed(42)).unwrap();
    assert!(cloned.config().rules.friendly_fire);
    assert_eq!(
        cloned
            .config()
            .players
            .iter()
            .map(|player| player.team)
            .collect::<Vec<_>>(),
        teams
    );
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

/// A retail-witnessed level-9 opponent exercises mode 4, attacks and aerial
/// Firefox recovery, rather than only Nana's partner mode or human inputs.
#[test]
fn versus_cpu_think_attack_and_recovery_allocate_nothing() {
    use ft_fox_family::FamilyState as FoxState;
    use melee_diff::Value;
    use melee_types::CommonMotionState as Common;
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        std::path::PathBuf::from,
    );
    let scenario = melee_sim::scenario::Scenario::load(
        &root.join("harness/scenarios/corpus_v3_fd_fox_cpu9fox4_edb2b114a_p0.toml"),
    ).unwrap();
    if !melee_test_support::require_files(scenario.required_files()) { return; }
    let mut game = diagnostics::import_match(&scenario).unwrap();
    let initial = diagnostics::inspect(&game).unwrap();
    assert_eq!(initial.fighters.state.get("p1.cpu.type"), Some(&Value::Int(4)));
    assert_eq!(initial.fighters.state.get("p1.cpu.level"), Some(&Value::Int(9)));
    let pads = melee_sim::trace::pad_script(&scenario).unwrap();
    let (mut attacked, mut recovered) = (false, false);
    for frame in 0..scenario.frames {
        let inputs = Inputs(std::array::from_fn(|port| pads.sample(frame, port)));
        assert_eq!(measure(&mut game, &inputs), 0, "mode-4 CPU tick {frame}");
        let view = game.observe().unwrap();
        let action = view.fighter(Port::P2).unwrap().action().0;
        attacked |= (Common::Attack11 as u16..=Common::AttackAirLw as u16).contains(&action);
        recovered |= action == FoxState::SpecialHiHoldAir as u16
            || action == FoxState::SpecialAirHi as u16
            || action == FoxState::SpecialHiFall as u16;
    }
    assert!(attacked, "the measured CPU never attacked");
    assert!(recovered, "the measured CPU never used aerial Firefox recovery");
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
        // Pair the owned roster's allocation and free inside tracking;
        // the original config was allocated before tracking began.
        let mut game = Match::new(&assets, config.clone()).unwrap();
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
    assert_eq!(initial.len(), 2);
    assert!(
        frequent.meshes().iter().any(
            |mesh| mesh.shadow_receiver && mesh.billboard == presentation::Billboard::ViewPlane
        ),
        "FD's central glow must face the camera"
    );
    let mut saw_motion = false;
    for tick in 0..600 {
        game.step(&Inputs::default()).unwrap();
        frequent.capture(&game).unwrap();
        saw_motion |= initial != frequent.directional_lights();
        let lights = frequent.directional_lights();
        // Authored spline JObjs place the lights at distinct heights and rotate
        // the second path by half a turn. Omitting their matrices collapses both.
        assert_eq!(lights[0].direction[1], 7.5);
        assert_eq!(lights[1].direction[1], -2.0);
        assert!((lights[0].direction[0] + lights[1].direction[0]).abs() < 0.00002);
        assert!((lights[0].direction[2] + lights[1].direction[2]).abs() < 0.00002);
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

/// Caller-owned xorshift so the input script never touches the game's RNG.
struct Choices(u32);
impl Choices {
    fn next(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }
    fn stick(&mut self) -> Stick {
        const DIRECTIONS: [(f32, f32); 9] = [
            (0.0, 0.0),
            (1.0, 0.0),
            (-1.0, 0.0),
            (0.0, 1.0),
            (0.0, -1.0),
            (0.7, 0.7),
            (-0.7, 0.7),
            (0.7, -0.7),
            (-0.7, -0.7),
        ];
        let (x, y) = DIRECTIONS[(self.next() % 9) as usize];
        Stick { x, y }
    }
    /// A held pad: attacks, specials, jumps, shields, grabs and movement.
    fn pad(&mut self) -> (ControllerState, u32) {
        let mut pad = ControllerState::default();
        pad.stick = self.stick();
        pad.buttons = match self.next() % 10 {
            0 | 1 => Buttons::A,
            2 | 3 => Buttons::B,
            4 => Buttons::X,
            5 => Buttons::L,
            6 => Buttons::Z,
            7 => {
                pad.cstick = self.stick();
                Buttons::default()
            }
            _ => Buttons::default(),
        };
        if pad.buttons.intersects(Buttons::L) {
            pad.left_trigger = 1.0;
        }
        (pad, [1, 2, 3, 5, 8, 13, 21][(self.next() % 7) as usize])
    }
}

/// Every stage, and every character at least once, steps without touching
/// the heap once the match exists: stage animations, articles, effects and
/// item pools are all prepared at load (AGENTS.md, "no heap allocation after
/// initialization"). Characters pair in select-screen order; the stages
/// cycle so each hosts at least two matches.
#[test]
fn every_stage_and_character_steps_without_allocating() {
    TRACE.with(|v| v.set(std::env::var_os("MELEE_ALLOC_TRACE").is_some()));
    // MELEE_DATA_ROOT: a checkout with harness data (for worktrees).
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        std::path::PathBuf::from,
    );
    let files = root.join("harness/roms/files");
    if !melee_test_support::require_files([files.join("PlCo.dat")]) {
        return;
    }
    const TICKS: usize = 3600;
    let mut failures = Vec::new();
    for (index, pair) in Character::ALL.chunks(2).enumerate() {
        let stage = Stage::PORTED[index % Stage::PORTED.len()];
        let config = MatchConfig::versus(
            stage,
            [
                PlayerConfig::new(Port::P1, pair[0]),
                PlayerConfig::new(Port::P2, pair[1]),
            ],
        )
        .with_seed(Seed(0x5eed + index as u32));
        let assets = GameAssets::load(&files, &config).unwrap();
        let mut game = Match::new(&assets, config).unwrap();
        let mut choices = Choices(0x9e37_79b9 ^ (index as u32 + 1));
        let mut held = [(ControllerState::default(), 0); 2];
        let mut allocating_ticks = 0;
        let mut first = None;
        for tick in 0..TICKS {
            if !game.status().is_running() {
                break;
            }
            let mut inputs = Inputs::default();
            for (port, (pad, remaining)) in held.iter_mut().enumerate() {
                if *remaining == 0 {
                    (*pad, *remaining) = choices.pad();
                }
                *remaining -= 1;
                inputs.0[port] = *pad;
            }
            let count = measure(&mut game, &inputs);
            if count != 0 {
                allocating_ticks += 1;
                first.get_or_insert((tick, count));
            }
        }
        if let Some((tick, count)) = first {
            failures.push(format!(
                "{} {:?} vs {:?}: {allocating_ticks} allocating ticks, first at tick {tick} ({count})",
                stage.display_name(),
                pair[0],
                pair[1],
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
