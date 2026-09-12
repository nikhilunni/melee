//! Versioned file choices. The core API deliberately has no serialization dependency.
use melee_lib::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    stage: String,
    players: [(u8, String, u8); 2],
    stocks: u8,
    all_characters_unlocked: bool,
    seed: u32,
}

const CHARACTERS: [(Character, &str); 7] = [
    (Character::Fox, "Fox"),
    (Character::Marth, "Marth"),
    (Character::Falco, "Falco"),
    (Character::CaptainFalcon, "CaptainFalcon"),
    (Character::Peach, "Peach"),
    (Character::Yoshi, "Yoshi"),
    (Character::Jigglypuff, "Jigglypuff"),
];
const STAGES: [(Stage, &str); 4] = [
    (Stage::FinalDestination, "FinalDestination"),
    (Stage::Battlefield, "Battlefield"),
    (Stage::YoshisStory, "YoshisStory"),
    (Stage::DreamLand, "DreamLand"),
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
            })
        };
        let mut config = MatchConfig::versus(stage, [player(0)?, player(1)?])
            .with_stocks(self.stocks)
            .with_seed(Seed(self.seed));
        config.rules.all_characters_unlocked = self.all_characters_unlocked;
        Ok(config)
    }
}
