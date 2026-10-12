//! Every character builds a `Presentation` and captures a short match of
//! idle, attacks, specials and shield: a character cannot silently stop
//! rendering (a model, part set, costume material or article that fails to
//! load or pose). No GPU: this checks the renderer-independent scene.
use melee_lib::{presentation::Presentation, *};
use std::path::PathBuf;

/// The asset directory: `MELEE_DATA_ROOT` (a checkout with harness data,
/// for worktrees) or this checkout.
fn files() -> Option<PathBuf> {
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    );
    let files = root.join("harness/roms/files");
    melee_test_support::require_files([files.join("PlCo.dat")]).then_some(files)
}

/// Player 1's input on `tick`: idle, then a jab, a neutral special, shield,
/// a side special and a down special.
fn script(tick: u32) -> ControllerState {
    let (buttons, stick) = match tick {
        120 => (Buttons::A, [0, 0]),
        150 => (Buttons::B, [0, 0]),
        200..=214 => (Buttons::L, [0, 0]),
        230 => (Buttons::B, [80, 0]),
        270 => (Buttons::B, [0, -80]),
        _ => (Buttons::default(), [0, 0]),
    };
    ControllerState::from_origin_adjusted(buttons, stick, [0, 0], [0, 0])
}

const TICKS: u32 = 300;

/// The meshes of fighter `slot` visible this capture.
fn visible_fighter_meshes(view: &Presentation, slot: usize) -> usize {
    view.meshes()
        .iter()
        .zip(view.visibility())
        .filter(|(mesh, visible)| **visible && mesh.shadow_owner == Some(slot))
        .count()
}

#[test]
fn every_character_presents_idle_attacks_specials_and_shield() {
    let Some(files) = files() else { return };
    for character in Character::ALL {
        let config = MatchConfig::versus(
            Stage::FinalDestination,
            [
                PlayerConfig::new(Port::P1, character),
                PlayerConfig::new(Port::P2, Character::Fox),
            ],
        )
        .with_seed(Seed(42));
        let assets = GameAssets::load(&files, &config).unwrap();
        let mut game = Match::new(&assets, config).unwrap();
        let mut view =
            Presentation::new(&game).unwrap_or_else(|e| panic!("{character:?}: presentation: {e}"));
        let fox = view
            .meshes()
            .iter()
            .filter_map(|m| m.shadow_owner)
            .max()
            .unwrap();
        let full_model = visible_fighter_meshes(&view, 0);
        let mut hidden_parts = false;
        for tick in 0..TICKS {
            let mut inputs = Inputs::default();
            inputs.0[0] = script(tick);
            game.step(&inputs)
                .unwrap_or_else(|e| panic!("{character:?} tick {tick}: {e}"));
            view.capture(&game)
                .unwrap_or_else(|e| panic!("{character:?} tick {tick}: capture: {e}"));
            let player = visible_fighter_meshes(&view, 0);
            // Moves may hide the fighter (Falco's and Fox's side specials);
            // standing, it always draws.
            assert!(
                player > 0 || tick >= 120,
                "{character:?} tick {tick}: player 1 draws nothing"
            );
            assert!(
                visible_fighter_meshes(&view, fox) > 0,
                "{character:?} tick {tick}: Fox draws nothing"
            );
            hidden_parts |= player
                < view
                    .meshes()
                    .iter()
                    .filter(|m| m.shadow_owner == Some(0))
                    .count();
            for material in view.materials() {
                assert!(
                    material
                        .diffuse
                        .iter()
                        .chain(&material.overlay)
                        .all(|v| v.is_finite()),
                    "{character:?} tick {tick}: non-finite material"
                );
            }
        }
        assert!(full_model > 0, "{character:?}: no fighter meshes");
        // Every model carries parts a normal draw hides (low poly, metal,
        // or unselected variants).
        assert!(hidden_parts, "{character:?}: every model part drew");
    }
}
