use melee_lib::{diagnostics, *};
use melee_sim::{inputs::PadScript, scenario::Scenario};
use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn config() -> MatchConfig {
    MatchConfig::versus(
        Stage::FinalDestination,
        [
            PlayerConfig::new(Port::P1, Character::Fox),
            PlayerConfig::new(Port::P2, Character::Marth),
        ],
    )
    .with_seed(Seed(42))
}
fn assets() -> Option<&'static GameAssets> {
    static ASSETS: OnceLock<GameAssets> = OnceLock::new();
    let files = root().join("harness/roms/files");
    if !melee_test_support::require_files([files.join("PlCo.dat")]) {
        return None;
    }
    Some(ASSETS.get_or_init(|| GameAssets::load(&files, &config()).unwrap()))
}
fn sample(frame: u64) -> Inputs {
    let mut inputs = Inputs::default();
    if (100..110).contains(&frame) {
        inputs[Port::P1].stick.x = 1.0;
    }
    if frame == 125 {
        inputs[Port::P1].buttons = Buttons::A;
    }
    if frame == 150 {
        inputs[Port::P2].buttons = Buttons::X;
    }
    inputs
}
fn run(game: &mut Match, frames: u64) {
    for _ in 0..frames {
        let inputs = sample(game.tick().0);
        game.step(&inputs).unwrap();
    }
}
fn fixture(name: &str) -> Option<(Scenario, Match, PadScript)> {
    let scenario = Scenario::load(&root().join(format!("harness/scenarios/{name}.toml"))).unwrap();
    if !melee_test_support::require_files(scenario.required_files()) {
        return None;
    }
    let game = diagnostics::import_match(&scenario).unwrap();
    let pads = melee_sim::trace::pad_script(&scenario).unwrap();
    Some((scenario, game, pads))
}
fn row(pads: &PadScript, frame: u64) -> Inputs {
    Inputs(std::array::from_fn(|p| pads.sample(frame, p)))
}

#[test]
fn streamed_inputs_match_scripted_inputs() {
    let scenario =
        Scenario::load(&root().join("harness/scenarios/start_fd_fox_cold.toml")).unwrap();
    if !melee_test_support::require_files(scenario.required_files()) {
        return;
    }
    let mut config = MatchConfig::versus(
        Stage::FinalDestination,
        [
            PlayerConfig::new(Port::P1, Character::Fox),
            PlayerConfig::new(Port::P2, Character::Fox),
        ],
    )
    .with_stocks(scenario.fighters[0].stocks)
    .with_seed(Seed(scenario.seed.unwrap()));
    config.rules.all_characters_unlocked = scenario.all_characters_unlocked.unwrap();
    let assets = GameAssets::load(scenario.assets_path(), &config).unwrap();
    let mut game = Match::new(&assets, config).unwrap();
    let script = PadScript::neutral(301);
    let initial = melee_sim::initial_state::InitialState::from_parameters(&scenario).unwrap();
    let mut scripted = melee_sim::frame::Simulation::with_inputs(initial, script);
    scripted.tick_without_snapshot().unwrap(); // explicit reset-boundary offset
    for _ in 0..300 {
        game.step(&Inputs::default()).unwrap();
        let expected = scripted.tick().unwrap();
        let actual = diagnostics::inspect(&game).unwrap();
        assert_eq!(actual.fighters.state, expected.state);
        assert_eq!(actual.items.state, scripted.item_snapshot(0).state);
        assert_eq!(actual.particle_rng_sites, scripted.particle_rng_sites());
        assert_eq!(actual.rng_writers, scripted.rng_writers());
    }
}

#[test]
fn shared_assets_matches_are_independent() {
    let Some(assets) = assets() else {
        return;
    };
    let mut first = Match::new(assets, config()).unwrap();
    let untouched = Match::new(assets, config()).unwrap();
    let before = diagnostics::inspect(&untouched).unwrap();
    run(&mut first, 180);
    assert_eq!(before, diagnostics::inspect(&untouched).unwrap());
    assert_eq!(untouched.tick(), Tick(0));
}

#[test]
fn parallel_matches_match_serial_execution() {
    fn send<T: Send>() {}
    fn send_sync<T: Send + Sync>() {}
    send::<Match>();
    send_sync::<GameAssets>();
    let Some(assets) = assets() else {
        return;
    };
    let mut serial = Match::new(assets, config()).unwrap();
    let workers: Vec<_> = (0..2)
        .map(|_| {
            let mut game = Match::new(assets, config()).unwrap();
            std::thread::spawn(move || {
                run(&mut game, 180);
                game
            })
        })
        .collect();
    run(&mut serial, 180);
    for worker in workers {
        assert_eq!(
            diagnostics::inspect(&serial).unwrap(),
            diagnostics::inspect(&worker.join().unwrap()).unwrap()
        );
    }
}

#[test]
fn cloned_matches_replay_identically() {
    let mut covered = [false; 5]; // startup, hitlag, item, stock loss, grab
    for name in [
        "match_fd_foxmarth",
        "match2_fd_foxmarth",
        "grabmash_fd_marth",
        "shieldhit_fd_marth",
        "ko_fd_marth",
    ] {
        let Some((scenario, mut game, pads)) = fixture(name) else {
            return;
        };
        let mut last_stocks = [255; 2];
        let mut frame = 0;
        while frame < scenario.frames && game.status().is_running() {
            let view = game.observe().unwrap();
            let stocks = std::array::from_fn(|p| view.fighters().nth(p).unwrap().stocks());
            let conditions = [
                game.status() == MatchStatus::Startup,
                view.fighters().any(|f| f.hitlag_remaining() > 0.0),
                view.items().next().is_some(),
                last_stocks != [255; 2] && stocks != last_stocks,
                view.fighters().any(|f| f.grabbed()),
            ];
            let first_event = conditions
                .iter()
                .zip(covered)
                .any(|(condition, seen)| *condition && !seen);
            for (seen, condition) in covered.iter_mut().zip(conditions) {
                *seen |= condition;
            }
            last_stocks = stocks;
            if frame % 509 == 0 || first_event {
                let mut branch = game.clone();
                assert_eq!(
                    diagnostics::inspect(&game).unwrap(),
                    diagnostics::inspect(&branch).unwrap(),
                    "{name} clone at {frame}"
                );
                for _ in 0..24.min(scenario.frames - frame) {
                    if !game.status().is_running() {
                        break;
                    }
                    let inputs = row(&pads, frame);
                    game.step(&inputs).unwrap();
                    branch.step(&inputs).unwrap();
                    assert_eq!(
                        diagnostics::inspect(&game).unwrap(),
                        diagnostics::inspect(&branch).unwrap(),
                        "{name} continuation {frame}"
                    );
                    frame += 1;
                }
            } else {
                game.step(&row(&pads, frame)).unwrap();
                frame += 1;
            }
        }
        let clone = game.clone();
        assert_eq!(
            diagnostics::inspect(&game).unwrap(),
            diagnostics::inspect(&clone).unwrap()
        );
    }
    assert!(
        covered.into_iter().all(|covered| covered),
        "corpus must cover all continuation states: {covered:?}"
    );
}

#[test]
fn cloned_matches_do_not_share_mutable_state() {
    let Some(assets) = assets() else {
        return;
    };
    let mut game = Match::new(assets, config()).unwrap();
    run(&mut game, 100);
    let before = diagnostics::inspect(&game).unwrap();
    let mut branch = game.clone();
    run(&mut branch, 80);
    assert_eq!(before, diagnostics::inspect(&game).unwrap());
    run(&mut game, 80);
    assert_eq!(
        diagnostics::inspect(&game).unwrap(),
        diagnostics::inspect(&branch).unwrap()
    );
}

#[test]
fn clone_from_replaces_complete_match() {
    let Some(assets) = assets() else {
        return;
    };
    let mut source = Match::new(assets, config()).unwrap();
    run(&mut source, 125);
    let other = MatchConfig::versus(
        Stage::Battlefield,
        [
            PlayerConfig::new(Port::P2, Character::Marth),
            PlayerConfig::new(Port::P4, Character::Fox),
        ],
    )
    .with_stocks(2)
    .with_seed(Seed(9));
    let other_assets = GameAssets::load(root().join("harness/roms/files"), &other).unwrap();
    let mut target = Match::new(&other_assets, other).unwrap();
    target.clone_from(&source);
    assert_eq!(
        diagnostics::inspect(&target).unwrap(),
        diagnostics::inspect(&source).unwrap()
    );
    run(&mut source, 80);
    run(&mut target, 80);
    assert_eq!(
        diagnostics::inspect(&target).unwrap(),
        diagnostics::inspect(&source).unwrap()
    );
    target.reset(Seed(42)).unwrap();
    assert_eq!(target.observe().unwrap().stage, Stage::FinalDestination);
    assert_eq!(
        target
            .observe()
            .unwrap()
            .fighter(Port::P1)
            .unwrap()
            .character(),
        Character::Fox
    );
}

#[test]
fn reset_matches_fresh_construction() {
    let Some(assets) = assets() else {
        return;
    };
    let mut reset = Match::new(assets, config()).unwrap();
    run(&mut reset, 180);
    reset.reset(Seed(93)).unwrap();
    let mut fresh = Match::new(assets, config().with_seed(Seed(93))).unwrap();
    assert_eq!(reset.tick(), Tick(0));
    assert_eq!(
        diagnostics::inspect(&reset).unwrap(),
        diagnostics::inspect(&fresh).unwrap()
    );
    run(&mut reset, 180);
    run(&mut fresh, 180);
    assert_eq!(
        diagnostics::inspect(&reset).unwrap(),
        diagnostics::inspect(&fresh).unwrap()
    );
}

#[test]
fn invalid_inputs_do_not_advance_match() {
    let Some(assets) = assets() else {
        return;
    };
    let mut game = Match::new(assets, config()).unwrap();
    let before = diagnostics::inspect(&game).unwrap();
    for value in [f32::NAN, f32::INFINITY, -1.01, 1.01] {
        let mut inputs = Inputs::default();
        inputs[Port::P4].stick.x = value;
        assert!(matches!(
            game.step(&inputs),
            Err(StepError::InvalidInput { port: Port::P4, .. })
        ));
        assert_eq!(before, diagnostics::inspect(&game).unwrap());
        assert_eq!(game.tick(), Tick(0));
    }
    for pad in [
        ControllerState {
            left_trigger: -0.01,
            ..Default::default()
        },
        ControllerState {
            right_trigger: f32::NAN,
            ..Default::default()
        },
        ControllerState {
            right_trigger: 1.01,
            ..Default::default()
        },
        ControllerState {
            cstick: Stick {
                x: 0.0,
                y: f32::NEG_INFINITY,
            },
            ..Default::default()
        },
        ControllerState {
            buttons: Buttons(1 << 30),
            ..Default::default()
        },
    ] {
        let mut inputs = Inputs::default();
        inputs[Port::P3] = pad;
        assert!(matches!(
            game.step(&inputs),
            Err(StepError::InvalidInput { port: Port::P3, .. })
        ));
        assert_eq!(before, diagnostics::inspect(&game).unwrap());
        assert_eq!(game.tick(), Tick(0));
    }
    game.step(&Inputs::default()).unwrap();
}

#[test]
fn observation_frequency_does_not_change_simulation() {
    let Some(assets) = assets() else {
        return;
    };
    let mut observed = Match::new(assets, config()).unwrap();
    let mut reference = observed.clone();
    for frame in 0..180 {
        for _ in 0..3 {
            let view = observed.observe().unwrap();
            for fighter in view.fighters() {
                std::hint::black_box((fighter.position(), fighter.action()));
            }
            std::hint::black_box(view.items().count());
            std::hint::black_box(view.vertices().count());
            std::hint::black_box(view.surfaces().count());
        }
        observed.step(&sample(frame)).unwrap();
        reference.step(&sample(frame)).unwrap();
    }
    assert_eq!(
        diagnostics::inspect(&observed).unwrap(),
        diagnostics::inspect(&reference).unwrap()
    );
}

#[test]
fn terminal_step_returns_ok_then_rejects_further_steps() {
    let Some((scenario, mut game, pads)) = fixture("match_fd_foxmarth") else {
        return;
    };
    for frame in 0..scenario.frames {
        game.step(&row(&pads, frame)).unwrap();
        if matches!(game.status(), MatchStatus::Finished(_)) {
            break;
        }
    }
    assert!(matches!(game.status(), MatchStatus::Finished(_)));
    let before = diagnostics::inspect(&game).unwrap();
    let tick = game.tick();
    assert_eq!(game.step(&Inputs::default()), Err(StepError::Finished));
    assert_eq!(game.tick(), tick);
    assert_eq!(before, diagnostics::inspect(&game).unwrap());
    let mut clone = game.clone();
    assert_eq!(clone.step(&Inputs::default()), Err(StepError::Finished));
}

#[test]
fn shadow_floors_follow_live_fighters_without_mutating_the_match() {
    let Some(assets) = assets() else {
        return;
    };
    let mut game = Match::new(assets, config()).unwrap();
    let mut view = presentation::Presentation::new(&game).unwrap();
    for _ in 0..240 {
        game.step(&Inputs::default()).unwrap();
    }
    view.capture(&game).unwrap();
    let before = game.clone();
    for floor in view.shadow_floors() {
        assert!(floor[0] < floor[2]);
        assert_eq!(floor[1], 0.0);
        assert_eq!(floor[3], 0.0);
    }
    assert!(view.meshes().iter().any(|mesh| mesh.shadow_receiver));
    for slot in 0..2 {
        assert!(view
            .meshes()
            .iter()
            .any(|mesh| mesh.shadow_owner == Some(slot)));
    }
    for _ in 0..10 {
        view.capture(&game).unwrap();
    }
    assert_eq!(
        diagnostics::inspect(&game).unwrap(),
        diagnostics::inspect(&before).unwrap()
    );
    let mut inputs = Inputs::default();
    inputs[Port::P1].stick.x = -1.0;
    for _ in 0..90 {
        game.step(&inputs).unwrap();
    }
    view.capture(&game).unwrap();
    // Running beyond the ledge must not project a shadow onto an infinite plane.
    assert!(view.shadow_floors()[0][0] > view.shadow_floors()[0][2]);
    assert!(view.shadow_floors()[1][0] < view.shadow_floors()[1][2]);
}
