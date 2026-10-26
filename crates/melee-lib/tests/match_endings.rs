//! Match endings through the public API, compared tick for tick with retail:
//! a one-minute match that times out on a stock tie continues into the
//! Sudden Death match retail plays next.
use melee_lib::{diagnostics, *};
use melee_sim::scenario::Scenario;
use std::path::PathBuf;

fn root() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn expected(scenario: &Scenario) -> Vec<melee_diff::Record> {
    melee_diff::read_trace(melee_test_support::trace::open(&scenario.expected_path()).unwrap())
        .unwrap()
}

/// Step with neutral pads while the match runs, comparing each tick with
/// the retail rows after the setup row.
fn play(game: &mut Match, rows: &[melee_diff::Record], name: &str) -> usize {
    play_holding(game, rows, name, &Inputs::default())
}

/// [`play`], every port holding `inputs` the whole match.
fn play_holding(game: &mut Match, rows: &[melee_diff::Record], name: &str, inputs: &Inputs) -> usize {
    let mut ticks = 0;
    while game.status().is_running() {
        game.step(inputs).unwrap();
        ticks += 1;
        let mut actual = diagnostics::inspect(game).unwrap().fighters;
        // Match ticks count from after the setup row; the labels differ.
        actual.frame = rows[ticks].frame;
        assert!(
            melee_diff::first_divergence([&rows[ticks]], [&actual]).is_none(),
            "{name} tick {ticks}: {}",
            melee_diff::first_divergence([&rows[ticks]], [&actual]).unwrap()
        );
    }
    ticks
}

/// timeout_tie_fd_marth(_cold) and sudden_death_start_fd_marth: idle Marth
/// (P1) and Fox (P2) on Final Destination with a one-minute timer.
#[test]
fn timed_out_tie_continues_into_retail_sudden_death() {
    let load = |name: &str| {
        Scenario::load(&root().join(format!("harness/scenarios/{name}.toml"))).unwrap()
    };
    let timeout = load("timeout_tie_fd_marth_cold");
    let sudden_death = load("sudden_death_start_fd_marth");
    if !melee_test_support::require_files(
        timeout
            .required_files()
            .into_iter()
            .chain(sudden_death.required_files()),
    ) {
        return;
    }
    let mut config = MatchConfig::versus(
        Stage::FinalDestination,
        [
            PlayerConfig::new(Port::P1, Character::Marth),
            PlayerConfig::new(Port::P2, Character::Fox),
        ],
    )
    .with_seed(Seed(timeout.seed.unwrap()));
    config.rules.time_limit_seconds = timeout.time_limit;
    let assets = GameAssets::load(timeout.assets_path(), &config).unwrap();
    let mut game = Match::new(&assets, config).unwrap();

    let ticks = play(&mut game, &expected(&timeout), "timeout");
    assert_eq!(
        game.status(),
        MatchStatus::Finished(MatchOutcome::SuddenDeath)
    );
    // Retail detects TIME! on the next frame (frame_count 3600).
    assert_eq!(ticks, 3724);

    let mut next = game.sudden_death().unwrap();
    assert_eq!(next.config().rules.stocks, 1);
    play(&mut next, &expected(&sudden_death), "sudden death");
    // A Bob-omb KOs Marth at 300%.
    assert_eq!(
        next.status(),
        MatchStatus::Finished(MatchOutcome::Winner(Port::P2))
    );
}

/// w27_fd_time1_tie3of4_walkoff (built from w27_fd_time1_tie3of4_cold's
/// parameters) and w27_fd_time1_tie3of4_sd: a four-player one-minute Time
/// match where Captain Falcon (P4) holds left
/// and walks off Final Destination after every revival. TIME! ranks Fox,
/// Marth and Falco first at 0, so the Sudden Death has those three alone
/// (gm_SetupSuddenDeath: P4's slot is Gm_PKind_NA), its setup drawing for
/// their fighters only. Both matches are compared with retail every tick.
#[test]
fn a_partial_tie_continues_into_a_sudden_death_of_the_tied_players() {
    let load = |name: &str| {
        Scenario::load(&root().join(format!("harness/scenarios/{name}.toml"))).unwrap()
    };
    let start = load("w27_fd_time1_tie3of4_cold");
    let timeout = load("w27_fd_time1_tie3of4_walkoff");
    let sudden_death = load("w27_fd_time1_tie3of4_sd");
    if !melee_test_support::require_files(
        timeout
            .required_files()
            .into_iter()
            .chain(sudden_death.required_files()),
    ) {
        return;
    }
    let mut config = MatchConfig::versus(
        Stage::FinalDestination,
        [
            PlayerConfig::new(Port::P1, Character::Fox),
            PlayerConfig::new(Port::P2, Character::Marth),
            PlayerConfig::new(Port::P3, Character::Falco),
            PlayerConfig::new(Port::P4, Character::CaptainFalcon),
        ],
    )
    .with_seed(Seed(start.seed.unwrap()));
    config.rules.mode = MatchKind::Time;
    config.rules.time_limit_seconds = start.time_limit;
    let assets = GameAssets::load(start.assets_path(), &config).unwrap();
    let mut game = Match::new(&assets, config).unwrap();
    let mut walk_off = Inputs::default();
    walk_off.0[Port::P4.index()].stick = Stick { x: -1.0, y: 0.0 };
    play_holding(&mut game, &expected(&timeout), "timeout", &walk_off);
    assert_eq!(
        game.status(),
        MatchStatus::Finished(MatchOutcome::SuddenDeath)
    );

    let mut next = game.sudden_death().unwrap();
    let ports: Vec<Port> = next.config().players.iter().map(|p| p.port).collect();
    assert_eq!(ports, [Port::P1, Port::P2, Port::P3]);
    play(&mut next, &expected(&sudden_death), "sudden death");
}
