//! Every supported stage builds a presentation and keeps capturing; the
//! Ground models drawn follow the simulation's live Ground GObjs.
use super::{stage, ModelSource, Presentation};
use crate::{Character, GameAssets, Inputs, Match, MatchConfig, PlayerConfig, Port, Seed, Stage};
use std::path::PathBuf;

/// The disc's extracted files; `MELEE_DATA_ROOT` names another checkout's
/// data (a worktree reads the main checkout's in place).
fn files() -> Option<PathBuf> {
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    );
    let files = root.join("harness/roms/files");
    melee_test_support::require_files(
        [
            "GrNLa.dat",
            "GrNBa.dat",
            "GrSt.dat",
            "GrOp.dat",
            "GrIz.dat",
            "GrPs.dat",
        ]
        .map(|file| files.join(file)),
    )
    .then_some(files)
}

fn start(files: &std::path::Path, stage: Stage) -> Match {
    let config = MatchConfig::versus(
        stage,
        [
            PlayerConfig::new(Port::P1, Character::Fox),
            PlayerConfig::new(Port::P2, Character::Marth),
        ],
    )
    .with_seed(Seed(42));
    let assets = GameAssets::load(files, &config).unwrap();
    Match::new(&assets, config).unwrap()
}

/// Visible meshes of the Ground model behind `key`, and whether it has any.
fn stage_meshes(view: &Presentation, key: u8) -> (usize, bool) {
    let mut visible = 0;
    let mut any = false;
    for (i, part) in view.parts.iter().enumerate() {
        if matches!(view.models[part.model].source, ModelSource::Stage { key: k, .. } if k == key) {
            any = true;
            visible += usize::from(view.visible[i]);
        }
    }
    (visible, any)
}

#[test]
fn every_stage_draws_exactly_its_live_ground_models() {
    let Some(files) = files() else {
        return;
    };
    for stage in Stage::ALL {
        let mut game = start(&files, stage);
        let mut view =
            Presentation::new(&game).unwrap_or_else(|e| panic!("{stage:?}: presentation: {e}"));
        for tick in 0..400 {
            game.step(&Inputs::default()).unwrap();
            view.capture(&game)
                .unwrap_or_else(|e| panic!("{stage:?} tick {tick}: {e}"));
        }
        let mut drawn = 0;
        for key in 0..16 {
            let (visible, any) = stage_meshes(&view, key);
            if !stage::live(&game, key) {
                assert_eq!(visible, 0, "{stage:?}: map {key} is not live but drawn");
            }
            if any && stage::live(&game, key) {
                drawn += visible;
            }
        }
        assert!(drawn > 0, "{stage:?}: no Ground model drawn");
        assert!(
            !view.directional_lights().is_empty(),
            "{stage:?}: no stage light"
        );
        let camera = view.view_camera();
        assert!(camera.fov > 0.0 && camera.eye.iter().all(|v| v.is_finite()));
    }
}

#[test]
fn battlefield_draws_one_background_and_its_main_stage() {
    let Some(files) = files() else {
        return;
    };
    let mut game = start(&files, Stage::Battlefield);
    let mut view = Presentation::new(&game).unwrap();
    for _ in 0..120 {
        game.step(&Inputs::default()).unwrap();
        view.capture(&game).unwrap();
    }
    // grBattle_OnInit: maps 0, 3 (hidden), 1 and 6; 2 and 4 wait in reserve.
    assert!(stage_meshes(&view, 6).0 > 0, "main stage");
    assert!(stage_meshes(&view, 1).0 > 0, "first background");
    for reserve in [2, 3, 4] {
        assert_eq!(stage_meshes(&view, reserve).0, 0, "map {reserve}");
    }
    let background = |key: u8| {
        view.meshes.iter().zip(&view.parts).any(|(mesh, part)| {
            matches!(view.models[part.model].source, ModelSource::Stage { key: k, .. } if k == key)
                && mesh.background
        })
    };
    assert!(background(1) && !background(6));
}

#[test]
fn dream_land_and_fountain_light_with_point_lights() {
    let Some(files) = files() else {
        return;
    };
    for stage in [Stage::DreamLand, Stage::FountainOfDreams] {
        let game = start(&files, stage);
        let view = Presentation::new(&game).unwrap();
        let point = view
            .directional_lights()
            .iter()
            .filter(|light| light.distance_attenuation.is_some())
            .count();
        assert!(point > 0, "{stage:?}");
    }
    // Ground_801C20E0: Fountain of Dreams' overrides leave its point
    // lights diffuse only.
    let game = start(&files, Stage::FountainOfDreams);
    let view = Presentation::new(&game).unwrap();
    assert!(view
        .directional_lights()
        .iter()
        .all(|light| !light.specular));
}

#[test]
fn pokemon_stadium_draws_a_form_through_its_transformation() {
    let Some(files) = files() else {
        return;
    };
    let mut game = start(&files, Stage::PokemonStadium);
    let mut view = Presentation::new(&game).unwrap();
    const FORMS: [u8; 4] = [3, 4, 6, 9];
    const BASE: u8 = 5;
    let mut risen = None;
    for tick in 0..8000 {
        game.step(&Inputs::default()).unwrap();
        view.capture(&game)
            .unwrap_or_else(|e| panic!("tick {tick}: {e}"));
        let live = FORMS.into_iter().find(|&form| stage::live(&game, form));
        if let Some(form) = live {
            assert!(stage_meshes(&view, form).0 > 0, "tick {tick}: form {form}");
        }
        for form in FORMS.into_iter().filter(|&form| !stage::live(&game, form)) {
            assert_eq!(stage_meshes(&view, form).0, 0, "tick {tick}: form {form}");
        }
        if live.is_some() && !stage::live(&game, BASE) {
            assert_eq!(stage_meshes(&view, BASE).0, 0, "tick {tick}: base");
            risen = live;
            break;
        }
    }
    assert!(risen.is_some(), "no transformation within 8000 ticks");
}

#[test]
fn yoshis_story_draws_its_shy_guys() {
    let Some(files) = files() else {
        return;
    };
    let mut game = start(&files, Stage::YoshisStory);
    let mut view = Presentation::new(&game).unwrap();
    let mut seen = false;
    for _ in 0..600 {
        game.step(&Inputs::default()).unwrap();
        view.capture(&game).unwrap();
        seen |= view.parts.iter().enumerate().any(|(i, part)| {
            matches!(view.models[part.model].source, ModelSource::StageItem(_)) && view.visible[i]
        });
    }
    assert!(seen, "no Shy Guy drawn");
}
