//! Versioned file choices. The core API deliberately has no serialization dependency.
use melee_lib::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub(crate) stage: String,
    pub(crate) players: [(u8, String, u8); 2],
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
    pub(crate) controller_fixes: Option<[String; 2]>,
}

const CHARACTERS: [(Character, &str); 20] = [
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
];
const STAGES: [(Stage, &str); 6] = [
    (Stage::FinalDestination, "FinalDestination"),
    (Stage::Battlefield, "Battlefield"),
    (Stage::YoshisStory, "YoshisStory"),
    (Stage::DreamLand, "DreamLand"),
    (Stage::FountainOfDreams, "FountainOfDreams"),
    (Stage::PokemonStadium, "PokemonStadium"),
];
impl From<&MatchConfig> for Config {
    fn from(config: &MatchConfig) -> Self {
        Self {
            stage: STAGES
                .iter()
                .find(|(s, _)| *s == config.stage)
                .unwrap()
                .1
                .into(),
            players: config.players.each_ref().map(|p| {
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
            }),
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
                        .each_ref()
                        .map(|p| p.controller_fix.name().to_owned())
                }),
        }
    }
}
impl Config {
    pub fn decode(&self) -> Result<MatchConfig, String> {
        let stage = STAGES
            .iter()
            .find(|(_, name)| *name == self.stage)
            .ok_or("unknown replay stage")?
            .0;
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
                    Some(names) => ControllerFix::from_name(&names[index])
                        .ok_or("unknown replay controller fix")?,
                },
            })
        };
        let mut config = MatchConfig::versus(stage, [player(0)?, player(1)?])
            .with_stocks(self.stocks)
            .with_seed(Seed(self.seed));
        config.rules.all_characters_unlocked = self.all_characters_unlocked;
        config.rules.sudden_death = self.sudden_death;
        Ok(config)
    }
}
