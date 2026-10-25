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
    /// Team per port in a team battle (`make_boundary.py --teams`).
    #[serde(default)]
    pub teams: Option<Vec<u8>>,
    /// CPU level per player, 0 for a human.
    #[serde(default)]
    pub cpu_levels: Option<Vec<u8>>,
    /// Friendly fire in a team battle.
    #[serde(default)]
    pub friendly_fire: bool,
    /// The Item Switch frequency the match runs (`make_boundary.py
    /// --item-frequency`); absent means items off.
    #[serde(default)]
    pub item_frequency: Option<i8>,
    /// The match's item mask, hex (StartMeleeRules x20).
    #[serde(default)]
    pub item_mask: Option<String>,
    /// The match's mode other than Stock (`make_boundary.py --mode`).
    #[serde(default)]
    pub mode: Option<String>,
    /// A Time match's timer, in seconds (`make_boundary.py --time-limit`).
    #[serde(default)]
    pub time_limit: Option<u32>,
    /// Each player's handicap with the Handicap rule on.
    #[serde(default)]
    pub handicaps: Option<Vec<u8>>,
    /// The Damage Ratio rule as the match holds it (StartMeleeRules x30).
    #[serde(default)]
    pub damage_ratio: Option<f32>,
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
    /// The item rules as the recording format holds them.
    fn items(&self) -> Result<Option<(i8, u64)>, String> {
        match (self.item_frequency, &self.item_mask) {
            (None, None) => Ok(None),
            (Some(frequency), Some(mask)) => {
                let mask = u64::from_str_radix(mask.trim_start_matches("0x"), 16)
                    .map_err(|e| format!("{}: item_mask: {e}", self.name))?;
                Ok(Some((frequency, mask)))
            }
            _ => Err(format!("{}: item_frequency and item_mask go together", self.name)),
        }
    }
    /// The match this boundary starts: its costumes, the unlocked roster
    /// every boundary is recorded with, and the savestate's seed.
    pub fn config(&self) -> Result<MatchConfig, String> {
        if !(2..=4).contains(&self.players.len()) {
            return Err(format!("{}: a match has two to four players", self.name));
        }
        let costumes = match self.costumes.as_deref() {
            None => vec![0; self.players.len()],
            Some(c) if c.len() == self.players.len() => c.to_vec(),
            Some(_) => return Err(format!("{}: one costume per player", self.name)),
        };
        Config {
            stage: self.stage.clone(),
            players: self
                .players
                .iter()
                .zip(costumes)
                .enumerate()
                .map(|(port, (name, costume))| (port as u8, name.clone(), costume))
                .collect(),
            stocks: self.stocks,
            all_characters_unlocked: true,
            seed: self.seed,
            sudden_death: self.sudden_death,
            controller_fixes: None,
            teams: self.teams.clone(),
            cpu_levels: self.cpu_levels.clone(),
            friendly_fire: self.friendly_fire,
            items: self.items()?,
            mode: self.mode.clone(),
            time_limit: self.time_limit,
            handicaps: self.handicaps.clone(),
            damage_ratio_tenths: match self.damage_ratio {
                None => None,
                Some(ratio) => Some(
                    (5..=20u8)
                        .find(|&t| 0.1 * f32::from(t) == ratio)
                        .ok_or_else(|| format!("{}: damage_ratio {ratio}", self.name))?,
                ),
            },
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
    fn five_player_boundaries_are_rejected() {
        let boundary = Boundary {
            name: "b".into(),
            cold: "b_cold".into(),
            savestate: String::new(),
            stage: "FinalDestination".into(),
            players: vec!["Fox".into(); 5],
            stocks: 4,
            seed: 0,
            sudden_death: false,
            costumes: None,
            teams: None,
            cpu_levels: None,
            friendly_fire: false,
            item_frequency: None,
            item_mask: None,
            mode: None,
            time_limit: None,
            handicaps: None,
            damage_ratio: None,
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
