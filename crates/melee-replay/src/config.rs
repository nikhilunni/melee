//! Versioned file choices. The core API deliberately has no serialization dependency.
use melee_lib::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub(crate) stage: String,
    /// (port, character, costume) per player, two to four.
    pub(crate) players: Vec<(u8, String, u8)>,
    pub(crate) stocks: u8,
    pub(crate) all_characters_unlocked: bool,
    pub(crate) seed: u32,
    /// A Sudden Death match (`MatchRules::sudden_death`); absent in older
    /// recordings, which are never Sudden Death.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub(crate) sudden_death: bool,
    /// Each player's controller fix (`ControllerFix::name`); absent in
    /// older recordings and when both are off.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) controller_fixes: Option<Vec<String>>,
    /// Each player's team in a team battle; absent in a free-for-all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) teams: Option<Vec<u8>>,
    /// CPU level per player, 0 for a human.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) cpu_levels: Option<Vec<u8>>,
    /// Friendly fire in a team battle.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub(crate) friendly_fire: bool,
    /// The Item Switch (`MatchRules::items`) as frequency and mask; absent
    /// in older recordings and when items are off.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) items: Option<(i8, u64)>,
    /// `MatchRules::mode` other than Stock ("time"); absent in older
    /// recordings, which are stock matches.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) mode: Option<String>,
    /// `MatchRules::time_limit_seconds`; absent without a timer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) time_limit: Option<u32>,
    /// Each player's handicap with the Handicap rule on; absent with it off.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) handicaps: Option<Vec<u8>>,
    /// The Damage Ratio rule in tenths; absent at 1.0x.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) damage_ratio_tenths: Option<u8>,
}

/// A match mode's spelling in recordings, scenarios and the boundary
/// registry.
pub fn mode_name(mode: MatchKind) -> &'static str {
    match mode {
        MatchKind::Time => "time",
        MatchKind::Stock => "stock",
        MatchKind::Coin => "coin",
        MatchKind::Bonus => "bonus",
    }
}

/// The mode a recording or registry names.
pub fn parse_mode(name: Option<&str>) -> Result<MatchKind, String> {
    match name {
        None | Some("stock") => Ok(MatchKind::Stock),
        Some("time") => Ok(MatchKind::Time),
        Some("coin") => Ok(MatchKind::Coin),
        Some(other) => Err(format!("unknown match mode {other:?}")),
    }
}

const CHARACTERS: [(Character, &str); 26] = [
    (Character::Fox, "Fox"),
    (Character::Marth, "Marth"),
    (Character::Falco, "Falco"),
    (Character::CaptainFalcon, "CaptainFalcon"),
    (Character::Peach, "Peach"),
    (Character::Yoshi, "Yoshi"),
    (Character::Jigglypuff, "Jigglypuff"),
    (Character::Pikachu, "Pikachu"),
    (Character::Mario, "Mario"),
    (Character::Roy, "Roy"),
    (Character::DrMario, "DrMario"),
    (Character::Luigi, "Luigi"),
    (Character::Pichu, "Pichu"),
    (Character::Ganondorf, "Ganondorf"),
    (Character::IceClimbers, "IceClimbers"),
    (Character::Samus, "Samus"),
    (Character::Sheik, "Sheik"),
    (Character::Zelda, "Zelda"),
    (Character::Link, "Link"),
    (Character::YoungLink, "YoungLink"),
    (Character::DonkeyKong, "DonkeyKong"),
    (Character::Bowser, "Bowser"),
    (Character::GameAndWatch, "GameAndWatch"),
    (Character::Ness, "Ness"),
    (Character::Mewtwo, "Mewtwo"),
    (Character::Kirby, "Kirby"),
];
impl From<&MatchConfig> for Config {
    fn from(config: &MatchConfig) -> Self {
        Self {
            stage: config.stage.name().into(),
            players: config
                .players
                .iter()
                .map(|p| {
                    (
                        p.port as u8,
                        CHARACTERS
                            .iter()
                            .find(|(c, _)| *c == p.character)
                            .unwrap()
                            .1
                            .into(),
                        p.costume.0,
                    )
                })
                .collect(),
            stocks: config.rules.stocks,
            all_characters_unlocked: config.rules.all_characters_unlocked,
            seed: config.seed.0,
            sudden_death: config.rules.sudden_death,
            controller_fixes: config
                .players
                .iter()
                .any(|p| p.controller_fix != ControllerFix::Off)
                .then(|| {
                    config
                        .players
                        .iter()
                        .map(|p| p.controller_fix.name().to_owned())
                        .collect()
                }),
            teams: config.players[0].team.map(|_| {
                config
                    .players
                    .iter()
                    .map(|p| p.team.unwrap_or_default())
                    .collect()
            }),
            friendly_fire: config.rules.friendly_fire,
            items: config
                .rules
                .items
                .enabled()
                .then_some((config.rules.items.frequency, config.rules.items.mask)),
            cpu_levels: config
                .players
                .iter()
                .any(|p| p.cpu_level.is_some())
                .then(|| {
                    config
                        .players
                        .iter()
                        .map(|p| p.cpu_level.unwrap_or(0))
                        .collect()
                }),
            mode: (config.rules.mode != MatchKind::Stock)
                .then(|| mode_name(config.rules.mode).to_owned()),
            time_limit: config.rules.time_limit_seconds,
            handicaps: config.players.iter().any(|p| p.handicap.is_some()).then(|| {
                config
                    .players
                    .iter()
                    .map(|p| p.handicap.unwrap_or(0))
                    .collect()
            }),
            damage_ratio_tenths: (config.rules.damage_ratio_tenths != 10)
                .then_some(config.rules.damage_ratio_tenths),
        }
    }
}
impl Config {
    pub fn decode(&self) -> Result<MatchConfig, String> {
        let stage = Stage::from_name(&self.stage).ok_or("unknown replay stage")?;
        let player = |index: usize| -> Result<PlayerConfig, String> {
            let (port, name, costume) = &self.players[index];
            Ok(PlayerConfig {
                port: *Port::ALL
                    .get(usize::from(*port))
                    .ok_or("invalid replay port")?,
                character: CHARACTERS
                    .iter()
                    .find(|(_, n)| *n == name)
                    .ok_or("unknown replay character")?
                    .0,
                costume: Costume(*costume),
                controller_fix: match &self.controller_fixes {
                    None => ControllerFix::Off,
                    Some(names) => names
                        .get(index)
                        .and_then(|name| ControllerFix::from_name(name))
                        .ok_or("unknown replay controller fix")?,
                },
                cpu_level: match &self.cpu_levels {
                    None => None,
                    Some(levels) => {
                        let level = *levels.get(index).ok_or("one CPU level per player")?;
                        (level != 0).then_some(level)
                    }
                },
                team: match &self.teams {
                    None => None,
                    Some(teams) => Some(*teams.get(index).ok_or("one team per player")?),
                },
                handicap: match &self.handicaps {
                    None => None,
                    Some(levels) => {
                        let level = *levels.get(index).ok_or("one handicap per player")?;
                        (level != 0).then_some(level)
                    }
                },
            })
        };
        let players = (0..self.players.len())
            .map(player)
            .collect::<Result<Vec<_>, _>>()?;
        let mut config = MatchConfig::versus(stage, players)
            .with_stocks(self.stocks)
            .with_seed(Seed(self.seed));
        config.rules.all_characters_unlocked = self.all_characters_unlocked;
        config.rules.sudden_death = self.sudden_death;
        config.rules.friendly_fire = self.friendly_fire;
        config.rules.mode = parse_mode(self.mode.as_deref())?;
        config.rules.time_limit_seconds = self.time_limit;
        config.rules.damage_ratio_tenths = self.damage_ratio_tenths.unwrap_or(10);
        if let Some((frequency, mask)) = self.items {
            config.rules.items = ItemRules { frequency, mask };
        }
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(fix: ControllerFix) -> MatchConfig {
        MatchConfig::versus(
            Stage::Battlefield,
            [
                PlayerConfig::new(Port::P1, Character::Fox).with_controller_fix(fix),
                PlayerConfig::new(Port::P3, Character::Marth),
            ],
        )
    }

    #[test]
    fn controller_fixes_round_trip_and_stay_absent_when_off() {
        let off = Config::from(&config(ControllerFix::Off));
        let json = serde_json::to_value(&off).unwrap();
        assert!(json.get("controller_fixes").is_none());
        assert_eq!(off.decode().unwrap(), config(ControllerFix::Off));
        let ucf = Config::from(&config(ControllerFix::Ucf080));
        let text = serde_json::to_string(&ucf).unwrap();
        let read: Config = serde_json::from_str(&text).unwrap();
        assert_eq!(read.decode().unwrap(), config(ControllerFix::Ucf080));
    }
    #[test]
    fn mixed_cpu_team_config_round_trips_all_four_players() {
        let mut config = MatchConfig::versus(
            Stage::Battlefield,
            [
                PlayerConfig::new(Port::P1, Character::Fox).with_team(0),
                PlayerConfig::new(Port::P2, Character::Marth)
                    .with_team(1)
                    .with_cpu_level(9),
                PlayerConfig::new(Port::P3, Character::Falco)
                    .with_team(0)
                    .with_cpu_level(4),
                PlayerConfig::new(Port::P4, Character::Peach).with_team(1),
            ],
        );
        config.rules.friendly_fire = true;
        let text = serde_json::to_string(&Config::from(&config)).unwrap();
        let decoded: Config = serde_json::from_str(&text).unwrap();
        assert_eq!(decoded.decode().unwrap(), config);
    }
}
