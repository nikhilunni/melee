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
    let mut seen = [false; 4];
    for name in [
        "start_fd_items_kusudama",
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
                // Capsule, Box, Barrel and Party Ball all reach real captures.
                let index = match item.kind().0 {
                    0..=2 => Some(item.kind().0 as usize),
                    4 => Some(3),
                    _ => None,
                };
                if let Some(index) = index {
                    seen[index] = true;
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
fn moving_kongo_barrel_catch_hold_launch_and_capture_allocate_nothing() {
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        std::path::PathBuf::from,
    );
    let scenario = melee_sim::scenario::Scenario::load(
        &root.join("harness/scenarios/corpus_v3_kj_fox_marth4_ef5e3e043_p0.toml"),
    )
    .unwrap();
    if !melee_test_support::require_files(scenario.required_files()) {
        return;
    }
    let mut game = diagnostics::import_match(&scenario).unwrap();
    let mut presentation = presentation::Presentation::new(&game).unwrap();
    let pads = melee_sim::trace::pad_script(&scenario).unwrap();
    let mut previous = [ActionId(0); 2];
    let mut held_ticks = 0;
    let mut launches = 0;
    for frame in 0..scenario.frames {
        let inputs = Inputs(std::array::from_fn(|p| pads.sample(frame, p)));
        assert_eq!(measure(&mut game, &inputs), 0, "barrel tick {frame}");
        COUNT.with(|v| v.set(0));
        ENABLED.with(|v| v.set(true));
        let result = presentation.capture(&game);
        ENABLED.with(|v| v.set(false));
        result.unwrap();
        assert_eq!(COUNT.with(Cell::get), 0, "barrel capture {frame}");
        for (index, fighter) in game.observe().unwrap().fighters().enumerate() {
            let action = fighter.action();
            held_ticks += usize::from(action == ActionId(293));
            launches += usize::from(previous[index] == ActionId(293) && action == ActionId(91));
            previous[index] = action;
        }
    }
    assert_eq!(
        (held_ticks, launches),
        (18, 3),
        "exercise all recorded holds and launches"
    );
}

/// The last retail tick exercises the new floor callback; no later tick is invented.
#[test]
fn revival_floor_full_capture_step_observe_and_presentation_allocate_nothing() {
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        std::path::PathBuf::from,
    );
    let scenario = melee_sim::scenario::Scenario::load(
        &root.join("harness/scenarios/corpus_v3_kj_fox_marth4_e0c8363c7_p1.toml"),
    )
    .unwrap();
    if !melee_test_support::require_files(scenario.required_files()) {
        return;
    }
    assert_eq!(scenario.frames, 1679);
    let mut game = diagnostics::import_match(&scenario).unwrap();
    let mut presentation = presentation::Presentation::new(&game).unwrap();
    let pads = melee_sim::trace::pad_script(&scenario).unwrap();
    for frame in 0..scenario.frames {
        let inputs = Inputs(std::array::from_fn(|p| pads.sample(frame, p)));
        assert_eq!(
            measure(&mut game, &inputs),
            0,
            "revival tick/observe {frame}"
        );
        COUNT.with(|v| v.set(0));
        ENABLED.with(|v| v.set(true));
        let result = presentation.capture(&game);
        ENABLED.with(|v| v.set(false));
        result.unwrap();
        assert_eq!(COUNT.with(Cell::get), 0, "revival capture {frame}");
        if frame >= 1677 {
            let action = game.observe().unwrap().fighters().nth(1).unwrap().action();
            assert_eq!(action, ActionId(if frame == 1677 { 12 } else { 14 }));
        }
    }
}

#[test]
fn kongo_log_contact_and_capture_allocate_nothing() {
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        std::path::PathBuf::from,
    );
    let scenario = melee_sim::scenario::Scenario::load(
        &root.join("harness/scenarios/corpus_v3_kj_fox_marth4_e8be4d273_p2.toml"),
    )
    .unwrap();
    if !melee_test_support::require_files(scenario.required_files()) {
        return;
    }
    let mut game = diagnostics::import_match(&scenario).unwrap();
    let mut presentation = presentation::Presentation::new(&game).unwrap();
    let pads = melee_sim::trace::pad_script(&scenario).unwrap();
    let mut contact_ticks = 0;
    for frame in 0..scenario.frames {
        let inputs = Inputs(std::array::from_fn(|p| pads.sample(frame, p)));
        assert_eq!(measure(&mut game, &inputs), 0, "log tick {frame}");
        COUNT.with(|v| v.set(0));
        ENABLED.with(|v| v.set(true));
        let result = presentation.capture(&game);
        ENABLED.with(|v| v.set(false));
        result.unwrap();
        assert_eq!(COUNT.with(Cell::get), 0, "log capture {frame}");
        let fox = game.observe().unwrap().fighter(Port::P1).unwrap();
        if (2638..=2669).contains(&frame) {
            assert!(fox.grounded(), "recorded contact {frame}");
            contact_ticks += 1;
        }
        if frame == 2638 {
            assert_eq!(fox.action(), ActionId(257));
            let position = fox.position();
            assert_eq!(position.x.to_bits(), 109.094_55_f32.to_bits());
            assert_eq!(position.y.to_bits(), (-14.925_915_f32).to_bits());
        }
        if frame == 2670 {
            assert!(!fox.grounded(), "recorded departure");
        }
    }
    assert_eq!(contact_ticks, 32, "exercise the actual log contact");
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

/// The public API clones an actual suspended Barrel program after the drop,
/// then steps both independent copies through its delayed command and End.
#[test]
fn retained_item_program_step_capture_and_clone_allocate_nothing() {
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        std::path::PathBuf::from,
    );
    for (name, drop_tick) in [
        ("corpus_v3_bf_items_sword_e5993e1bc_p2", 375),
        ("corpus_v3_fd_items_taru_e17844051_p1", 353),
    ] {
        let scenario = melee_sim::scenario::Scenario::load(
            &root.join(format!("harness/scenarios/{name}.toml")),
        )
        .unwrap();
        if !melee_test_support::require_files(scenario.required_files()) {
            return;
        }
        let mut game = diagnostics::import_match(&scenario).unwrap();
        let mut view = presentation::Presentation::new(&game).unwrap();
        let pads = melee_sim::trace::pad_script(&scenario).unwrap();
        let mut branch = None;
        let mut continued = 0;
        for frame in 0..scenario.frames {
            let inputs = Inputs(std::array::from_fn(|p| pads.sample(frame, p)));
            assert_eq!(measure(&mut game, &inputs), 0, "{name} tick {frame}");
            if let Some(cloned) = branch.as_mut() {
                assert_eq!(measure(cloned, &inputs), 0, "{name} cloned tick {frame}");
                assert_eq!(
                    diagnostics::inspect(&game).unwrap(),
                    diagnostics::inspect(cloned).unwrap()
                );
                continued += 1;
            }
            COUNT.with(|v| v.set(0));
            ENABLED.with(|v| v.set(true));
            let result = view.capture(&game);
            ENABLED.with(|v| v.set(false));
            result.unwrap();
            assert_eq!(COUNT.with(Cell::get), 0, "{name} capture {frame}");
            if frame == drop_tick {
                assert!(game
                    .observe()
                    .unwrap()
                    .items()
                    .any(|item| item.kind().0 == 2));
                branch = Some(game.clone());
            }
        }
        assert_eq!(continued, scenario.frames - 1 - drop_tick);
        assert!(branch.is_some());
    }
}

#[test]
fn pinned_mouth_death_live_clone_step_and_capture_allocate_nothing() {
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        std::path::PathBuf::from,
    );
    let scenario = melee_sim::scenario::Scenario::load(
        &root.join("harness/scenarios/kirby_inhale_randall_pit_ys_fox4.toml"),
    )
    .unwrap();
    if !melee_test_support::require_files(scenario.required_files()) {
        return;
    }
    assert_eq!(scenario.frames, 1500);
    let mut game = diagnostics::import_match(&scenario).unwrap();
    let mut presentation = presentation::Presentation::new(&game).unwrap();
    let pads = melee_sim::trace::pad_script(&scenario).unwrap();
    let mut branch = None;
    for frame in 0..1500 {
        if frame == 1354 {
            let fox = game.observe().unwrap().fighter(Port::P2).unwrap();
            assert_eq!(fox.action(), ActionId(289));
            assert!(fox.grabbed());
            let cloned = game.clone();
            assert_eq!(
                diagnostics::inspect(&game).unwrap(),
                diagnostics::inspect(&cloned).unwrap()
            );
            let capture = presentation::Presentation::new(&cloned).unwrap();
            branch = Some((cloned, capture));
        }
        let inputs = Inputs(std::array::from_fn(|p| pads.sample(frame, p)));
        assert_eq!(measure(&mut game, &inputs), 0, "original step{frame}");
        COUNT.with(|v| v.set(0));
        ENABLED.with(|v| v.set(true));
        let result = presentation.capture(&game);
        ENABLED.with(|v| v.set(false));
        result.unwrap();
        assert_eq!(COUNT.with(Cell::get), 0, "original capture{frame}");
        if let Some((cloned, capture)) = &mut branch {
            assert_eq!(measure(cloned, &inputs), 0, "live clone step{frame}");
            COUNT.with(|v| v.set(0));
            ENABLED.with(|v| v.set(true));
            let result = capture.capture(cloned);
            ENABLED.with(|v| v.set(false));
            result.unwrap();
            assert_eq!(COUNT.with(Cell::get), 0, "clone capture{frame}");
            assert_eq!(
                diagnostics::inspect(&game).unwrap(),
                diagnostics::inspect(cloned).unwrap(),
                "live clone tick{frame}"
            );
        }
        let expected = match frame {
            1358 => Some([ActionId(29), ActionId(0)]),
            1359 => Some([ActionId(0); 2]),
            1418 => Some([ActionId(0), ActionId(12)]),
            1419 => Some([ActionId(12); 2]),
            1478 => Some([ActionId(12), ActionId(13)]),
            1479 => Some([ActionId(13); 2]),
            _ => None,
        };
        if let Some(expected) = expected {
            let view = game.observe().unwrap();
            assert_eq!(
                view.fighters().map(|f| f.action()).collect::<Vec<_>>(),
                expected
            );
        }
    }
    assert!(branch.is_some());
}

/// Real mixed contact, followed by nudge or the complete Stone timeout. A
/// clone made before contact must independently continue through both.
/// Clone creation is outside the measured step/capture allocation windows.
#[test]
fn stone_contact_step_and_capture_allocate_nothing_after_live_clone() {
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        std::path::PathBuf::from,
    );
    for (name, contact, ticks) in [
        ("kirby_stone_phantom_fd_fox4", 276, 360),
        ("corpus_v3_ys_kirby_fox4_e45211ed5_p0", 513, 699),
    ] {
        let scenario = melee_sim::scenario::Scenario::load(
            &root.join(format!("harness/scenarios/{name}.toml")),
        )
        .unwrap();
        if !melee_test_support::require_files(scenario.required_files()) {
            return;
        }
        assert_eq!(scenario.frames, ticks);
        let mut game = diagnostics::import_match(&scenario).unwrap();
        let mut capture = presentation::Presentation::new(&game).unwrap();
        let pads = melee_sim::trace::pad_script(&scenario).unwrap();
        let mut branch = None;
        let mut continued = 0;
        for frame in 0..ticks {
            if frame == contact {
                let cloned = game.clone();
                let view = presentation::Presentation::new(&cloned).unwrap();
                branch = Some((cloned, view));
            }
            let inputs = Inputs(std::array::from_fn(|p| pads.sample(frame, p)));
            assert_eq!(measure(&mut game, &inputs), 0, "{name} tick{frame}");
            COUNT.with(|v| v.set(0));
            ENABLED.with(|v| v.set(true));
            let result = capture.capture(&game);
            ENABLED.with(|v| v.set(false));
            result.unwrap();
            assert_eq!(COUNT.with(Cell::get), 0);
            if let Some((cloned, view)) = &mut branch {
                assert_eq!(measure(cloned, &inputs), 0, "{name} clone tick{frame}");
                COUNT.with(|v| v.set(0));
                ENABLED.with(|v| v.set(true));
                let result = view.capture(cloned);
                ENABLED.with(|v| v.set(false));
                result.unwrap();
                assert_eq!(COUNT.with(Cell::get), 0);
                assert_eq!(
                    diagnostics::inspect(&game).unwrap(),
                    diagnostics::inspect(cloned).unwrap()
                );
                continued += 1;
            }
            if name.contains("e45211ed5") && frame == 642 {
                assert_eq!(
                    game.observe().unwrap().fighter(Port::P1).unwrap().action(),
                    ActionId(395)
                );
            }
        }
        assert_eq!(continued, ticks - contact);
    }
}

/// Clone construction is outside the counter; both independent terminal
/// entries and their public step/observe/presentation capture allocate nothing.
#[test]
fn heavy_pickup_terminal_entry_step_and_capture_allocate_nothing_after_live_clone() {
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        std::path::PathBuf::from,
    );
    for (name, ticks, actor) in [
        ("corpus_v3_bf_items_sword_e5993e1bc_p0", 259, Port::P2),
        ("corpus_v3_fd_items_sword_e5de865a8_p1", 257, Port::P1),
        ("corpus_v3_fd_items_sword_e6af4a7bb_p2", 240, Port::P1),
        ("corpus_v3_fd_items_sword_efc6a328f_p2", 241, Port::P1),
        ("corpus_v3_fd_items_taru_e1895a33d_p0", 295, Port::P2),
    ] {
        let scenario = melee_sim::scenario::Scenario::load(
            &root.join(format!("harness/scenarios/{name}.toml")),
        )
        .unwrap();
        assert!(melee_test_support::require_files(scenario.required_files()));
        assert_eq!(scenario.frames, ticks);
        let mut game = diagnostics::import_match(&scenario).unwrap();
        let mut capture = presentation::Presentation::new(&game).unwrap();
        let pads = melee_sim::trace::pad_script(&scenario).unwrap();
        let mut branch = None;
        for tick in 0..ticks {
            if tick + 1 == ticks {
                let cloned = game.clone();
                let capture = presentation::Presentation::new(&cloned).unwrap();
                assert_eq!(
                    diagnostics::inspect(&game).unwrap(),
                    diagnostics::inspect(&cloned).unwrap()
                );
                branch = Some((cloned, capture));
            }
            let inputs = Inputs(std::array::from_fn(|p| pads.sample(tick, p)));
            assert_eq!(measure(&mut game, &inputs), 0, "{name} step{tick}");
            COUNT.with(|v| v.set(0));
            ENABLED.with(|v| v.set(true));
            let observed = game.observe().map(|v| v.fighters().count());
            let result = capture.capture(&game);
            ENABLED.with(|v| v.set(false));
            assert_eq!(observed.unwrap(), 2);
            result.unwrap();
            assert_eq!(COUNT.with(Cell::get), 0, "{name} capture{tick}");
            if let Some((cloned, capture)) = &mut branch {
                assert_eq!(measure(cloned, &inputs), 0, "{name} clone entry");
                COUNT.with(|v| v.set(0));
                ENABLED.with(|v| v.set(true));
                let result = capture.capture(cloned);
                ENABLED.with(|v| v.set(false));
                result.unwrap();
                assert_eq!(COUNT.with(Cell::get), 0, "{name} clone capture");
                assert_eq!(
                    diagnostics::inspect(&game).unwrap(),
                    diagnostics::inspect(cloned).unwrap()
                );
            }
        }
        let view = game.observe().unwrap();
        assert_eq!(view.fighter(actor).unwrap().action(), ActionId(93));
        assert_eq!(view.items().count(), 1);
        assert!(view.items().all(|item| item.owner().is_none()));
        assert!(branch.is_some());
        // No later retail continuation is claimed: the actual public next
        // step must report the deliberately unported animation callback.
        let error = game.step(&Inputs::default()).unwrap_err();
        assert!(
            error.to_string().contains("HeavyGet continuation"),
            "{error}"
        );
    }
}

/// Clone creation is unmeasured. Both independent full public continuations
/// reach the terminal entry with allocation-free step/observe/capture.
#[test]
fn held_back_air_entry_step_and_capture_allocate_nothing_after_live_clone() {
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        std::path::PathBuf::from,
    );
    let scenario = melee_sim::scenario::Scenario::load(
        &root.join("harness/scenarios/corpus_v3_fd_items_sword_nocont_e8f0de8c6_p2.toml"),
    )
    .unwrap();
    assert!(melee_test_support::require_files(scenario.required_files()));
    assert_eq!(scenario.frames, 621);
    let mut game = diagnostics::import_match(&scenario).unwrap();
    let mut capture = presentation::Presentation::new(&game).unwrap();
    let pads = melee_sim::trace::pad_script(&scenario).unwrap();
    let mut branch = None;
    for tick in 0..621 {
        if tick == 620 {
            let cloned = game.clone();
            let capture = presentation::Presentation::new(&cloned).unwrap();
            assert_eq!(
                diagnostics::inspect(&game).unwrap(),
                diagnostics::inspect(&cloned).unwrap()
            );
            branch = Some((cloned, capture));
        }
        let inputs = Inputs(std::array::from_fn(|p| pads.sample(tick, p)));
        assert_eq!(measure(&mut game, &inputs), 0, "step{tick}");
        COUNT.with(|v| v.set(0));
        ENABLED.with(|v| v.set(true));
        let observed = game.observe().map(|view| view.fighters().count());
        let result = capture.capture(&game);
        ENABLED.with(|v| v.set(false));
        assert_eq!(observed.unwrap(), 2);
        result.unwrap();
        assert_eq!(COUNT.with(Cell::get), 0, "capture{tick}");
        if let Some((cloned, capture)) = &mut branch {
            assert_eq!(measure(cloned, &inputs), 0, "cloned entry");
            COUNT.with(|v| v.set(0));
            ENABLED.with(|v| v.set(true));
            let result = capture.capture(cloned);
            ENABLED.with(|v| v.set(false));
            result.unwrap();
            assert_eq!(COUNT.with(Cell::get), 0, "cloned entry capture");
            assert_eq!(
                diagnostics::inspect(&game).unwrap(),
                diagnostics::inspect(cloned).unwrap()
            );
        }
    }
    let view = game.observe().unwrap();
    assert_eq!(view.fighter(Port::P2).unwrap().action(), ActionId(67));
    assert_eq!(view.items().count(), 2);
    assert_eq!(
        view.items().filter(|item| item.owner().is_some()).count(),
        1
    );
    assert!(branch.is_some());
    // The callback refusal is intentionally outside allocation measurement.
    let error = game.step(&Inputs::default()).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("held Sword aerial entry continuation"),
        "{error}"
    );
    let error = branch
        .as_mut()
        .unwrap()
        .0
        .step(&Inputs::default())
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("held Sword aerial entry continuation"),
        "{error}"
    );
}

#[test]
fn monster_constructor_step_and_capture_allocate_nothing_after_live_clone() {
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        std::path::PathBuf::from,
    );
    for (name, ticks, kind) in [
        ("corpus_v3_fd_items_sword_e266e1150_p1", 1874, ItemKind(43)),
        ("corpus_v3_fd_items_sword_e4abe72ec_p2", 297, ItemKind(44)),
        ("corpus_v3_fd_items_sword_e5ca12c3b_p0", 595, ItemKind(43)),
        ("corpus_v3_fd_items_taru_eae02b66b_p2", 285, ItemKind(43)),
    ] {
        let scenario = melee_sim::scenario::Scenario::load(
            &root.join(format!("harness/scenarios/{name}.toml")),
        )
        .unwrap();
        assert!(melee_test_support::require_files(scenario.required_files()));
        assert_eq!(scenario.frames, ticks);
        let mut game = diagnostics::import_match(&scenario).unwrap();
        let mut capture = presentation::Presentation::new(&game).unwrap();
        let pads = melee_sim::trace::pad_script(&scenario).unwrap();
        let mut branch = None;
        let mut empty_before_birth = Vec::new();
        for tick in 0..ticks {
            if tick + 1 == ticks {
                empty_before_birth = capture
                    .meshes()
                    .iter()
                    .map(|mesh| mesh.instance_group)
                    .filter(|group| capture.instance_range(*group).is_empty())
                    .collect();
                empty_before_birth.sort_unstable();
                empty_before_birth.dedup();
                let cloned = game.clone();
                let capture = presentation::Presentation::new(&cloned).unwrap();
                assert_eq!(
                    diagnostics::inspect(&game).unwrap(),
                    diagnostics::inspect(&cloned).unwrap()
                );
                branch = Some((cloned, capture));
            }
            let inputs = Inputs(std::array::from_fn(|p| pads.sample(tick, p)));
            assert_eq!(measure(&mut game, &inputs), 0, "{name} step{tick}");
            COUNT.with(|v| v.set(0));
            ENABLED.with(|v| v.set(true));
            let observed = game.observe().map(|v| v.fighters().count());
            let result = capture.capture(&game);
            ENABLED.with(|v| v.set(false));
            assert_eq!(observed.unwrap(), 2);
            result.unwrap();
            assert_eq!(COUNT.with(Cell::get), 0, "{name} capture{tick}");
            if let Some((cloned, capture)) = &mut branch {
                assert_eq!(measure(cloned, &inputs), 0, "{name} clone entry");
                COUNT.with(|v| v.set(0));
                ENABLED.with(|v| v.set(true));
                let result = capture.capture(cloned);
                ENABLED.with(|v| v.set(false));
                result.unwrap();
                assert_eq!(COUNT.with(Cell::get), 0, "{name} clone capture");
                assert_eq!(
                    diagnostics::inspect(&game).unwrap(),
                    diagnostics::inspect(cloned).unwrap()
                );
            }
        }
        let view = game.observe().unwrap();
        assert_eq!(view.items().filter(|item| item.kind() == kind).count(), 1);
        assert!(view
            .items()
            .find(|item| item.kind() == kind)
            .unwrap()
            .owner()
            .is_none());
        // Identify the newly born free model through public draw data, rather
        // than treating a successful capture as evidence that it was rendered.
        let position = view
            .items()
            .find(|item| item.kind() == kind)
            .unwrap()
            .position();
        let assert_born_draw = |capture: &presentation::Presentation| {
            let groups: Vec<_> = empty_before_birth
                .iter()
                .copied()
                .filter(|group| {
                    let range = capture.instance_range(*group);
                    if range.len() != 1 {
                        return false;
                    }
                    let matrix = capture.instances()[range.start as usize];
                    matrix[3][..3] == [position.x, position.y, position.z]
                        && capture.meshes().iter().zip(capture.visibility()).any(
                            |(mesh, visible)| {
                                mesh.instance_group == *group
                                    && *visible
                                    && !mesh.vertices.is_empty()
                                    && !mesh.indices.is_empty()
                            },
                        )
                })
                .collect();
            assert_eq!(
                groups.len(),
                1,
                "{name}: born monster has one visible model group"
            );
            groups[0]
        };
        let group = assert_born_draw(&capture);
        let (_, cloned_capture) = branch.as_ref().unwrap();
        assert_eq!(group, assert_born_draw(cloned_capture));
        assert_eq!(
            capture.instance_range(group),
            cloned_capture.instance_range(group)
        );
        let range = capture.instance_range(group);
        assert_eq!(
            &capture.instances()[range.start as usize..range.end as usize],
            &cloned_capture.instances()[range.start as usize..range.end as usize]
        );
        // No later retail continuation is claimed: the actual public next
        // step must report the deliberately unported animation callback.
        let error = game.step(&Inputs::default()).unwrap_err();
        assert!(error.to_string().contains("monster Animation"), "{error}");
    }
}

#[test]
fn held_forward_and_up_entry_step_and_capture_allocate_nothing_after_live_clone() {
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        std::path::PathBuf::from,
    );
    for (name, frames, actor, action) in [
        ("corpus_v3_bf_items_sword_ea3ec9208_p1", 314, 1, 66),
        ("corpus_v3_fd_items_sword_e00f31913_p2", 1502, 1, 68),
        ("corpus_v3_fd_items_sword_e1502cb40_p0", 1116, 1, 66),
        ("corpus_v3_fd_items_sword_edb13a9c3_p2", 568, 1, 66),
        ("corpus_v3_fd_items_sword_edb4b01fd_p1", 1073, 1, 68),
        ("corpus_v3_fd_items_sword_ee92d4a97_p0", 834, 0, 66),
        ("corpus_v3_fd_items_sword_eecfcf1ea_p2", 776, 1, 68),
        ("corpus_v3_fd_items_sword_nocont_e05625e18_p2", 1384, 0, 68),
        ("corpus_v3_fd_items_sword_nocont_e89a89d0e_p2", 727, 0, 66),
        ("corpus_v3_fd_items_sword_nocont_ea4d5d9ec_p2", 880, 1, 66),
        ("corpus_v3_fd_items_sword_nocont_ee62c6106_p1", 1052, 0, 68),
        ("corpus_v3_fd_items_taru_e8d30e8ed_p1", 1160, 1, 68),
        ("corpus_v3_fd_items_taru_ea9c3e366_p1", 1247, 1, 68),
    ] {
        let scenario = melee_sim::scenario::Scenario::load(
            &root.join(format!("harness/scenarios/{name}.toml")),
        )
        .unwrap();
        assert!(melee_test_support::require_files(scenario.required_files()));
        assert_eq!(scenario.frames, frames);
        let mut game = diagnostics::import_match(&scenario).unwrap();
        let mut capture = presentation::Presentation::new(&game).unwrap();
        let pads = melee_sim::trace::pad_script(&scenario).unwrap();
        let mut branch = None;
        for tick in 0..frames {
            if tick + 1 == frames {
                // Clone and presentation construction are intentionally unmeasured.
                let cloned = game.clone();
                let capture = presentation::Presentation::new(&cloned).unwrap();
                assert_eq!(
                    diagnostics::inspect(&game).unwrap(),
                    diagnostics::inspect(&cloned).unwrap()
                );
                branch = Some((cloned, capture));
            }
            let inputs = Inputs(std::array::from_fn(|p| pads.sample(tick, p)));
            assert_eq!(measure(&mut game, &inputs), 0, "step{tick}");
            COUNT.with(|v| v.set(0));
            ENABLED.with(|v| v.set(true));
            let observed = game.observe().map(|view| view.fighters().count());
            let result = capture.capture(&game);
            ENABLED.with(|v| v.set(false));
            assert_eq!(observed.unwrap(), 2);
            result.unwrap();
            assert_eq!(COUNT.with(Cell::get), 0, "capture{tick}");
            if let Some((cloned, capture)) = &mut branch {
                assert_eq!(measure(cloned, &inputs), 0, "cloned entry");
                COUNT.with(|v| v.set(0));
                ENABLED.with(|v| v.set(true));
                let result = capture.capture(cloned);
                ENABLED.with(|v| v.set(false));
                result.unwrap();
                assert_eq!(COUNT.with(Cell::get), 0, "cloned entry capture");
                assert_eq!(
                    diagnostics::inspect(&game).unwrap(),
                    diagnostics::inspect(cloned).unwrap()
                );
            }
        }
        let view = game.observe().unwrap();
        assert_eq!(
            view.fighter([Port::P1, Port::P2][actor]).unwrap().action(),
            ActionId(action)
        );
        assert!(view.items().count() > 0);
        assert!(branch.is_some());
        // The callback refusal is intentionally outside allocation measurement.
        let error = game.step(&Inputs::default()).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("held Sword aerial entry continuation"),
            "{error}"
        );
        let error = branch
            .as_mut()
            .unwrap()
            .0
            .step(&Inputs::default())
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("held Sword aerial entry continuation"),
            "{error}"
        );
    }
}

#[test]
fn klaptrap_birth_step_and_capture_allocate_nothing_after_live_clone() {
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        std::path::PathBuf::from,
    );
    for (name, ticks, kind) in [
        ("corpus_v3_kj_fox_marth4_e0fcbde24_p1", 1666, ItemKind(218)),
        ("corpus_v3_kj_fox_marth4_e75fb4a9a_p0", 1666, ItemKind(218)),
        ("corpus_v3_kj_fox_marth4_e89a89d0e_p0", 1666, ItemKind(218)),
    ] {
        let scenario = melee_sim::scenario::Scenario::load(
            &root.join(format!("harness/scenarios/{name}.toml")),
        )
        .unwrap();
        assert!(melee_test_support::require_files(scenario.required_files()));
        assert_eq!(scenario.frames, ticks);
        let mut game = diagnostics::import_match(&scenario).unwrap();
        let mut capture = presentation::Presentation::new(&game).unwrap();
        let pads = melee_sim::trace::pad_script(&scenario).unwrap();
        let mut branch = None;
        let mut empty_before_birth = Vec::new();
        for tick in 0..ticks {
            if tick + 1 == ticks {
                empty_before_birth = capture
                    .meshes()
                    .iter()
                    .map(|mesh| mesh.instance_group)
                    .filter(|group| capture.instance_range(*group).is_empty())
                    .collect();
                empty_before_birth.sort_unstable();
                empty_before_birth.dedup();
                let cloned = game.clone();
                let capture = presentation::Presentation::new(&cloned).unwrap();
                assert_eq!(
                    diagnostics::inspect(&game).unwrap(),
                    diagnostics::inspect(&cloned).unwrap()
                );
                branch = Some((cloned, capture));
            }
            let inputs = Inputs(std::array::from_fn(|p| pads.sample(tick, p)));
            assert_eq!(measure(&mut game, &inputs), 0, "{name} step{tick}");
            COUNT.with(|v| v.set(0));
            ENABLED.with(|v| v.set(true));
            let observed = game.observe().map(|v| v.fighters().count());
            let result = capture.capture(&game);
            ENABLED.with(|v| v.set(false));
            assert_eq!(observed.unwrap(), 2);
            result.unwrap();
            assert_eq!(COUNT.with(Cell::get), 0, "{name} capture{tick}");
            if let Some((cloned, capture)) = &mut branch {
                assert_eq!(measure(cloned, &inputs), 0, "{name} clone entry");
                COUNT.with(|v| v.set(0));
                ENABLED.with(|v| v.set(true));
                let result = capture.capture(cloned);
                ENABLED.with(|v| v.set(false));
                result.unwrap();
                assert_eq!(COUNT.with(Cell::get), 0, "{name} clone capture");
                assert_eq!(
                    diagnostics::inspect(&game).unwrap(),
                    diagnostics::inspect(cloned).unwrap()
                );
            }
        }
        let view = game.observe().unwrap();
        assert_eq!(view.items().filter(|item| item.kind() == kind).count(), 1);
        assert!(view
            .items()
            .find(|item| item.kind() == kind)
            .unwrap()
            .owner()
            .is_none());
        // Identify the newly born free model through public draw data, rather
        // than treating a successful capture as evidence that it was rendered.
        let position = view
            .items()
            .find(|item| item.kind() == kind)
            .unwrap()
            .position();
        let assert_born_draw = |capture: &presentation::Presentation| {
            let groups: Vec<_> = empty_before_birth
                .iter()
                .copied()
                .filter(|group| {
                    let range = capture.instance_range(*group);
                    if range.len() != 1 {
                        return false;
                    }
                    let matrix = capture.instances()[range.start as usize];
                    matrix[3][..3] == [position.x, position.y, position.z]
                        && capture.meshes().iter().zip(capture.visibility()).any(
                            |(mesh, visible)| {
                                mesh.instance_group == *group
                                    && *visible
                                    && !mesh.vertices.is_empty()
                                    && !mesh.indices.is_empty()
                            },
                        )
                })
                .collect();
            assert_eq!(
                groups.len(),
                1,
                "{name}: born Klaptrap has one visible model group"
            );
            groups[0]
        };
        let group = assert_born_draw(&capture);
        let (_, cloned_capture) = branch.as_ref().unwrap();
        assert_eq!(group, assert_born_draw(cloned_capture));
        assert_eq!(
            capture.instance_range(group),
            cloned_capture.instance_range(group)
        );
        let range = capture.instance_range(group);
        assert_eq!(
            &capture.instances()[range.start as usize..range.end as usize],
            &cloned_capture.instances()[range.start as usize..range.end as usize]
        );
        // No later retail continuation is claimed: the actual public next
        // step must report the deliberately unported animation callback.
        let error = game.step(&Inputs::default()).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("Ground_801C1CD0: Klaptrap rider"),
            "{error}"
        );
    }
}

#[test]
fn held_down_entry_step_observe_and_capture_allocate_nothing_after_live_clone() {
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        std::path::PathBuf::from,
    );
    for (name, frames, actor, action) in [
        ("corpus_v3_fd_items_sword_e45a17231_p2", 1634, 1, 69),
        ("corpus_v3_fd_items_sword_e9fa97e1e_p0", 442, 1, 69),
        ("corpus_v3_fd_items_sword_nocont_e0ac13e8e_p1", 1344, 1, 69),
        ("corpus_v3_fd_items_sword_nocont_ecdf8887e_p1", 966, 0, 69),
        ("corpus_v3_fd_items_taru_e5935012b_p2", 1383, 1, 69),
    ] {
        let scenario = melee_sim::scenario::Scenario::load(
            &root.join(format!("harness/scenarios/{name}.toml")),
        )
        .unwrap();
        assert!(melee_test_support::require_files(scenario.required_files()));
        assert_eq!(scenario.frames, frames);
        let mut game = diagnostics::import_match(&scenario).unwrap();
        let mut capture = presentation::Presentation::new(&game).unwrap();
        let pads = melee_sim::trace::pad_script(&scenario).unwrap();
        let mut branch = None;
        for tick in 0..frames {
            if tick + 1 == frames {
                // Clone and presentation construction are intentionally unmeasured.
                let cloned = game.clone();
                let capture = presentation::Presentation::new(&cloned).unwrap();
                assert_eq!(
                    diagnostics::inspect(&game).unwrap(),
                    diagnostics::inspect(&cloned).unwrap()
                );
                branch = Some((cloned, capture));
            }
            let inputs = Inputs(std::array::from_fn(|p| pads.sample(tick, p)));
            assert_eq!(measure(&mut game, &inputs), 0, "step{tick}");
            COUNT.with(|v| v.set(0));
            ENABLED.with(|v| v.set(true));
            let observed = game.observe().map(|view| view.fighters().count());
            let result = capture.capture(&game);
            ENABLED.with(|v| v.set(false));
            assert_eq!(observed.unwrap(), 2);
            result.unwrap();
            assert_eq!(COUNT.with(Cell::get), 0, "capture{tick}");
            if let Some((cloned, capture)) = &mut branch {
                assert_eq!(measure(cloned, &inputs), 0, "cloned entry");
                COUNT.with(|v| v.set(0));
                ENABLED.with(|v| v.set(true));
                let observed = cloned.observe().map(|view| view.fighters().count());
                let result = capture.capture(cloned);
                ENABLED.with(|v| v.set(false));
                assert_eq!(observed.unwrap(), 2);
                result.unwrap();
                assert_eq!(COUNT.with(Cell::get), 0, "cloned entry capture");
                assert_eq!(
                    diagnostics::inspect(&game).unwrap(),
                    diagnostics::inspect(cloned).unwrap()
                );
            }
        }
        let view = game.observe().unwrap();
        assert_eq!(
            view.fighter([Port::P1, Port::P2][actor]).unwrap().action(),
            ActionId(action)
        );
        assert!(view.items().count() > 0);
        assert!(branch.is_some());
        // The callback refusal is intentionally outside allocation measurement.
        let error = game.step(&Inputs::default()).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("held Sword aerial entry continuation"),
            "{error}"
        );
        let error = branch
            .as_mut()
            .unwrap()
            .0
            .step(&Inputs::default())
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("held Sword aerial entry continuation"),
            "{error}"
        );
    }
}
