//! The whole host flow on the real disc image, without a GPU: open the
//! image, pick, load from the disc, play, quit, and reuse fetched files.
use melee_lib::{Character, Stage};
use melee_platform::{
    app::{App, Screen},
    disc::DiscFiles,
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
