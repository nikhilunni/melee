//! Registered retail match-start boundaries (`harness/boundaries.toml`).
//!
//! A boundary is a retail savestate whose cold construction the port
//! reproduces exactly (`harness/make_boundary.py` creates, gates and registers
//! them). A recording that starts from one replays in Dolphin through
//! `harness/replay_to_scenario.py`, so the explorer starts every case here.
use crate::config::Config;
use melee_lib::MatchConfig;
use serde::Deserialize;
use std::path::Path;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Boundary {
    pub name: String,
    /// The `_cold` scenario that builds the same scene from parameters.
    pub cold: String,
    pub savestate: String,
    pub stage: String,
    /// Character per port, P1 first (the recording format's spelling).
    pub players: Vec<String>,
    pub stocks: u8,
    pub seed: u32,
    #[serde(default)]
    pub sudden_death: bool,
    /// Costume per port when any is not the port's first (`make_boundary.py
    /// --costumes`); absent means every port wears costume 0.
    #[serde(default)]
    pub costumes: Option<Vec<u8>>,
}

#[derive(Deserialize)]
struct Registry {
    boundary: Vec<Boundary>,
}

/// Every boundary in a registry file, in file order.
pub fn load(path: &Path) -> Result<Vec<Boundary>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let registry: Registry =
        toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(registry.boundary)
}

impl Boundary {
    /// The match this boundary starts: its costumes, the unlocked roster
    /// every boundary is recorded with, and the savestate's seed.
    pub fn config(&self) -> Result<MatchConfig, String> {
        let [first, second] = self.players.as_slice() else {
            return Err(format!(
                "{}: the port runs two-player matches only",
                self.name
            ));
        };
        let [first_costume, second_costume] = match self.costumes.as_deref() {
            None => [0, 0],
            Some(&[a, b]) => [a, b],
            Some(_) => return Err(format!("{}: one costume per player", self.name)),
        };
        Config {
            stage: self.stage.clone(),
            players: [
                (0, first.clone(), first_costume),
                (1, second.clone(), second_costume),
            ],
            stocks: self.stocks,
            all_characters_unlocked: true,
            seed: self.seed,
            sudden_death: self.sudden_death,
            controller_fixes: None,
        }
        .decode()
        .map_err(|e| format!("{}: {e}", self.name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use melee_lib::{Character, Port, Stage};

    #[test]
    fn registry_boundaries_become_match_configs() {
        let registry: Registry = toml::from_str(
            r#"
            [[boundary]]
            name = "start_bf_marth_fox4"
            cold = "start_bf_marth_fox4_cold"
            savestate = "harness/roms/start_bf_marth_fox4.sav"
            stage = "Battlefield"
            players = ["Marth", "Fox"]
            stocks = 4
            seed = 7
            "#,
        )
        .unwrap();
        let config = registry.boundary[0].config().unwrap();
        assert_eq!(config.stage, Stage::Battlefield);
        assert_eq!(config.players[0].port, Port::P1);
        assert_eq!(config.players[0].character, Character::Marth);
        assert_eq!(config.players[1].character, Character::Fox);
        assert_eq!((config.rules.stocks, config.seed.0), (4, 7));
        assert!(config.rules.all_characters_unlocked && !config.rules.sudden_death);
    }

    #[test]
    fn a_costume_boundary_starts_its_costumes() {
        let registry: Registry = toml::from_str(
            r#"
            [[boundary]]
            name = "start_fd_jigglypuff_c2_fox4"
            cold = "start_fd_jigglypuff_c2_fox4_cold"
            savestate = "harness/roms/start_fd_jigglypuff_c2_fox4.sav"
            stage = "FinalDestination"
            players = ["Jigglypuff", "Fox"]
            stocks = 4
            seed = 7
            costumes = [2, 0]
            "#,
        )
        .unwrap();
        let config = registry.boundary[0].config().unwrap();
        assert_eq!(config.players[0].costume.0, 2);
        assert_eq!(config.players[1].costume.0, 0);
    }

    #[test]
    fn three_player_boundaries_are_rejected() {
        let boundary = Boundary {
            name: "b".into(),
            cold: "b_cold".into(),
            savestate: String::new(),
            stage: "FinalDestination".into(),
            players: vec!["Fox".into(), "Fox".into(), "Fox".into()],
            stocks: 4,
            seed: 0,
            sudden_death: false,
            costumes: None,
        };
        assert!(boundary.config().is_err());
    }

    #[test]
    fn the_checked_in_registry_parses() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let boundaries = load(&root.join("harness/boundaries.toml")).unwrap();
        for boundary in &boundaries {
            boundary.config().unwrap();
        }
        assert!(boundaries.iter().any(|b| b.name == "start_fd_fox4"));
    }
}
