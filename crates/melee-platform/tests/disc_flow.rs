//! The whole host flow on the real disc image, without a GPU: open the
//! image, pick, load from the disc, play, quit, and reuse fetched files.
use melee_lib::{Character, Stage};
use melee_platform::{
    app::{App, Screen},
    disc::DiscFiles,
    players::{Controller, Mode, Rules},
    pads::{Input, RawPad},
    session::Action,
};
use std::{path::PathBuf, time::Duration};

fn iso() -> Option<PathBuf> {
    let root = std::env::var_os("MELEE_DATA_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));
    let iso = root.join("harness/roms/GALE01.iso");
    melee_test_support::require_files([&iso]).then_some(iso)
}

fn load(app: &mut App) {
    while app.load_next_file().unwrap() {}
    app.finish_loading().unwrap();
}

#[test]
fn a_match_loads_from_the_disc_image_and_plays() {
    let Some(iso) = iso() else {
        return;
    };
    let mut app = App::new();
    app.open_disc(DiscFiles::open_path(&iso).unwrap()).unwrap();
    app.choose_character(0, Character::Fox).unwrap();
    app.choose_character(1, Character::Marth).unwrap();
    app.confirm_characters().unwrap();
    app.choose_stage(Stage::FinalDestination, 1234).unwrap();
    let total = app.load_progress();
    assert!(total.files_total > 5 && total.bytes_total > 1_000_000);
    load(&mut app);
    assert_eq!(app.screen(), Screen::Match);
    assert_eq!(app.disc().unwrap().cached_files() as u32, total.files_total);
    for _ in 0..130 {
        app.advance(Duration::from_nanos(16_666_667)).unwrap();
    }
    app.set_action(0, Action::Right, true);
    for _ in 0..30 {
        app.advance(Duration::from_nanos(16_666_667)).unwrap();
    }
    let hud = app.hud().unwrap();
    assert_eq!(hud.tick, 160);
    assert_eq!(hud.players[0].character, Character::Fox);
    assert_eq!(hud.players[1].stocks, 4);
    assert_eq!(
        app.session()
            .unwrap()
            .recording()
            .config
            .decode()
            .unwrap()
            .seed
            .0,
        1234
    );

    // A rematch needs no new files; a new opponent needs only theirs.
    app.rematch(5).unwrap();
    assert_eq!(app.load_progress().files_total, 0);
    app.finish_loading().unwrap();
    assert_eq!(app.hud().unwrap().tick, 0);
    app.quit_to_menu();
    assert_eq!(app.screen(), Screen::Characters);
    app.choose_character(1, Character::Falco).unwrap();
    app.confirm_characters().unwrap();
    app.choose_stage(Stage::FinalDestination, 6).unwrap();
    let falco = app.pending_requests();
    assert!(falco.iter().all(|r| r.name.contains("Fc")), "{falco:?}");
    load(&mut app);
    assert_eq!(app.screen(), Screen::Match);
}

#[test]
fn the_hud_shows_one_entry_per_player_when_players_own_two_fighters() {
    let Some(iso) = iso() else {
        return;
    };
    let mut app = App::new();
    app.open_disc(DiscFiles::open_path(&iso).unwrap()).unwrap();
    // Zelda loads Sheik beside her; Ice Climbers are Popo and Nana.
    app.choose_character(0, Character::Zelda).unwrap();
    app.choose_character(1, Character::IceClimbers).unwrap();
    app.confirm_characters().unwrap();
    app.choose_stage(Stage::Battlefield, 7).unwrap();
    load(&mut app);
    app.advance(Duration::from_nanos(16_666_667)).unwrap();
    let hud = app.hud().unwrap();
    let players: Vec<_> = hud.players.iter().map(|p| (p.port, p.character)).collect();
    assert_eq!(
        players,
        [
            (melee_lib::Port::P1, Character::Zelda),
            (melee_lib::Port::P2, Character::IceClimbers),
        ]
    );
}

const FRAME: Duration = Duration::from_nanos(16_666_667);

#[test]
fn a_cpu_plays_on_final_destination_and_reads_no_keyboard() {
    let Some(iso) = iso() else {
        return;
    };
    let mut app = App::new();
    app.open_disc(DiscFiles::open_path(&iso).unwrap()).unwrap();
    app.choose_character(0, Character::Fox).unwrap();
    app.set_controller(0, Controller::Cpu).unwrap();
    app.set_cpu_level(0, 9).unwrap();
    app.choose_character(1, Character::Marth).unwrap();
    app.confirm_characters().unwrap();
    assert_eq!(
        app.stage_unavailable(Stage::DreamLand).is_none(),
        Stage::DreamLand.cpu_supported()
    );
    app.choose_stage(Stage::FinalDestination, 99).unwrap();
    load(&mut app);
    // Keyboard layout 0 drives the first human: port 2, not the CPU.
    assert_eq!(app.human_ports(), [1]);
    app.set_action(0, Action::Left, true);
    for _ in 0..600 {
        app.advance(FRAME).unwrap();
    }
    let hud = app.hud().unwrap();
    assert_eq!(hud.tick, 600);
    assert!(hud.players[0].cpu && !hud.players[1].cpu);
    let recording = app.session().unwrap().recording();
    let samples = recording.samples();
    assert!(samples
        .iter()
        .all(|s| s.inputs().0[0] == Default::default()));
    assert!(samples.iter().any(|s| s.inputs().0[1].stick.x < 0.0));
}

#[test]
fn four_players_play_a_team_battle_driven_by_gamepads() {
    let Some(iso) = iso() else {
        return;
    };
    let mut app = App::new();
    app.open_disc(DiscFiles::open_path(&iso).unwrap()).unwrap();
    for (player, character) in [
        Character::Fox,
        Character::Marth,
        Character::Peach,
        Character::Falco,
    ]
    .into_iter()
    .enumerate()
    {
        app.choose_character(player, character).unwrap();
        app.set_team(player, (player % 2) as u8).unwrap();
    }
    app.set_rules(Rules {
        teams: true,
        item_frequency: Some(4),
        ..app.rules()
    })
    .unwrap();
    app.confirm_characters().unwrap();
    // Items are witnessed with two players only: the menu says so.
    let reason = app.stage_unavailable(Stage::Battlefield).unwrap();
    assert!(reason.contains("two players"), "{reason}");
    app.back();
    app.set_rules(Rules {
        item_frequency: None,
        ..app.rules()
    })
    .unwrap();
    app.confirm_characters().unwrap();
    app.choose_stage(Stage::Battlefield, 2026).unwrap();
    load(&mut app);
    assert_eq!(app.human_ports(), [0, 1, 2, 3]);
    assert_eq!(app.session().unwrap().game().config().players.len(), 4);
    // A pad given port 4 drives it: a held jump (X, the right face
    // button) with the stick right.
    app.pad_connect(40, "Xbox Wireless Controller", "Xbox One", 0x045e, 0x0b13);
    app.assign_pad(40, Some(3));
    let mut pad = RawPad::default();
    pad.buttons[Input::East.index()] = 1.0;
    pad.axes[0] = 1.0;
    for frame in 0..900 {
        app.pad_update(40, &pad, f64::from(frame) * 16.0);
        app.advance(FRAME).unwrap();
    }
    let hud = app.hud().unwrap();
    assert_eq!(hud.tick, 900);
    let teams: Vec<_> = hud.players.iter().map(|p| p.team).collect();
    assert_eq!(teams, [Some(0), Some(1), Some(0), Some(1)]);
    let samples = app.session().unwrap().recording().samples();
    assert!(samples.iter().any(|s| s.inputs().0[3].stick.x == 1.0));
}

#[test]
fn random_items_fall_in_a_two_player_match() {
    let Some(iso) = iso() else {
        return;
    };
    let mut app = App::new();
    app.open_disc(DiscFiles::open_path(&iso).unwrap()).unwrap();
    app.choose_character(0, Character::Fox).unwrap();
    app.choose_character(1, Character::Marth).unwrap();
    app.set_rules(Rules {
        item_frequency: Some(4),
        ..app.rules()
    })
    .unwrap();
    app.confirm_characters().unwrap();
    assert!(app.stage_unavailable(Stage::DreamLand).is_some());
    app.choose_stage(Stage::FinalDestination, 31).unwrap();
    load(&mut app);
    assert_eq!(
        app.session().unwrap().game().config().rules.items.frequency,
        4
    );
    let mut seen = 0;
    for _ in 0..1800 {
        if app.advance(FRAME).is_err() {
            // An unwitnessed item branch fails closed; the replay is kept.
            break;
        }
        let view = app.session().unwrap().game().observe().unwrap();
        seen = seen.max(view.items().count());
    }
    assert!(seen > 0, "Very High spawns an item within 30 seconds");
}

#[test]
fn a_tied_time_match_continues_into_sudden_death() {
    let Some(iso) = iso() else {
        return;
    };
    let mut app = App::new();
    app.open_disc(DiscFiles::open_path(&iso).unwrap()).unwrap();
    app.choose_character(0, Character::Fox).unwrap();
    app.choose_character(1, Character::Marth).unwrap();
    app.set_handicap(1, 9).unwrap();
    app.set_rules(Rules {
        mode: Mode::Time,
        time_minutes: Some(1),
        damage_ratio_tenths: 15,
        ..app.rules()
    })
    .unwrap();
    app.confirm_characters().unwrap();
    app.choose_stage(Stage::FinalDestination, 8).unwrap();
    load(&mut app);
    let config = app.session().unwrap().game().config().clone();
    assert_eq!(config.rules.time_limit_seconds, Some(60));
    assert_eq!(config.players[1].handicap, Some(9));
    let hud = app.hud().unwrap();
    assert!(hud.timed_scores && !hud.sudden_death);
    assert_eq!(hud.seconds_left, Some(60));
    let generation = app.generation();
    // Nobody moves: 0-0 at TIME!, and the tie plays on.
    for _ in 0..(60 * 60 + 600) {
        app.advance(FRAME).unwrap();
        if app.generation() != generation {
            break;
        }
    }
    assert_eq!(app.screen(), Screen::Match, "{:?}", app.notice());
    let hud = app.hud().unwrap();
    assert!(hud.sudden_death);
    assert_eq!(hud.players.len(), 2);
    assert!(hud.players.iter().all(|p| p.stocks == 1));
    for _ in 0..120 {
        app.advance(FRAME).unwrap();
    }
    // Restart goes back to the configured Time match.
    app.restart().unwrap();
    assert!(!app.hud().unwrap().sudden_death);
}
