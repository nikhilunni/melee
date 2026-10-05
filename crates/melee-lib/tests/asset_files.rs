//! `GameAssets::files` lists every file a match loads, so hosts that fetch
//! files ahead of time (the browser) never miss one.
use melee_lib::*;
use std::{cell::RefCell, collections::BTreeSet, path::PathBuf};

struct Recording {
    directory: PathBuf,
    names: RefCell<BTreeSet<String>>,
}
impl FileSource for Recording {
    fn read(&self, name: &str) -> anyhow::Result<Vec<u8>> {
        self.names.borrow_mut().insert(name.to_owned());
        self.directory.read(name)
    }
}

fn files() -> Option<PathBuf> {
    let files = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files");
    melee_test_support::require_files([files.join("PlCo.dat")]).then_some(files)
}

fn assert_listed(directory: &std::path::Path, config: &MatchConfig) {
    let source = Recording {
        directory: directory.to_owned(),
        names: RefCell::default(),
    };
    GameAssets::load_from(&source, config).unwrap();
    let listed: BTreeSet<String> = GameAssets::files(config)
        .unwrap()
        .into_iter()
        .map(str::to_owned)
        .collect();
    assert_eq!(*source.names.borrow(), listed, "{config:?}");
}

#[test]
fn every_character_loads_exactly_its_listed_files() {
    let Some(directory) = files() else { return };
    for character in Character::ALL {
        let players = [Port::P1, Port::P2].map(|port| PlayerConfig::new(port, character));
        assert_listed(
            &directory,
            &MatchConfig::versus(Stage::Battlefield, players),
        );
    }
}

#[test]
fn every_stage_loads_exactly_its_listed_files() {
    let Some(directory) = files() else { return };
    for stage in Stage::ALL {
        let players = [
            PlayerConfig::new(Port::P1, Character::Fox),
            PlayerConfig::new(Port::P2, Character::Marth),
        ];
        assert_listed(&directory, &MatchConfig::versus(stage, players));
    }
}

#[test]
fn files_load_from_memory() {
    let Some(directory) = files() else { return };
    let config = MatchConfig::versus(
        Stage::PokemonStadium,
        [
            PlayerConfig::new(Port::P1, Character::Zelda),
            PlayerConfig::new(Port::P2, Character::IceClimbers),
        ],
    );
    let fetched: std::collections::BTreeMap<String, Vec<u8>> = GameAssets::files(&config)
        .unwrap()
        .into_iter()
        .map(|name| {
            (
                name.to_owned(),
                std::fs::read(directory.join(name)).unwrap(),
            )
        })
        .collect();
    GameAssets::load_from(&fetched, &config).unwrap();
}
