//! Match choices, independent of controllers, policies, and replay formats.
use crate::{
    setup::{PlayerSetup, Setup},
    StartError,
};

/// A physical controller port. Fighter iteration order is not port identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Port {
    P1,
    P2,
    P3,
    P4,
}
impl Port {
    pub const ALL: [Self; 4] = [Self::P1, Self::P2, Self::P3, Self::P4];
    pub const fn index(self) -> usize {
        self as usize
    }
    pub(crate) fn from_index(index: u8) -> Self {
        Self::ALL[usize::from(index)]
    }
}

/// Characters currently registered by the match composition layer.
/// A registered character is not a promise that every reachable move is ported.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Character {
    Fox,
    Marth,
    Falco,
    CaptainFalcon,
    Peach,
    Yoshi,
    Jigglypuff,
}
impl Character {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Fox => "Fox",
            Self::Marth => "Marth",
            Self::Falco => "Falco",
            Self::CaptainFalcon => "CaptainFalcon",
            Self::Peach => "Peach",
            Self::Yoshi => "Yoshi",
            Self::Jigglypuff => "Jigglypuff",
        }
    }
    pub(crate) fn descriptor(self) -> &'static melee_ft::fighter::assets::CharacterDescriptor {
        crate::scene_fighter::SceneFighter::descriptor_for(self.name())
            .expect("registered character")
    }
    pub(crate) fn from_kind(kind: melee_types::FighterKind) -> Self {
        match kind {
            melee_types::FighterKind::Fox => Self::Fox,
            melee_types::FighterKind::Mars => Self::Marth,
            melee_types::FighterKind::Falco => Self::Falco,
            melee_types::FighterKind::Captain => Self::CaptainFalcon,
            melee_types::FighterKind::Peach => Self::Peach,
            melee_types::FighterKind::Yoshi => Self::Yoshi,
            melee_types::FighterKind::Purin => Self::Jigglypuff,
            _ => unreachable!("unregistered match character"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    FinalDestination,
    Battlefield,
    YoshisStory,
    DreamLand,
}
impl Stage {
    pub(crate) fn descriptor(self) -> &'static crate::scene_stage::StageDescriptor {
        use crate::scene_stage::*;
        match self {
            Self::FinalDestination => &FINAL_DESTINATION,
            Self::Battlefield => &BATTLEFIELD,
            Self::YoshisStory => &YOSHIS_STORY,
            Self::DreamLand => &DREAM_LAND,
        }
    }
}

/// RNG seed at the retail post-creation, pre-music boundary. Construction
/// reverses the audited fixed setup draws, then reproduces setup normally.
/// Music selection consumes this seed before the first public tick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Seed(pub u32);
impl Seed {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }
}

/// Character-relative costume index, validated when assets or a match are created.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Costume(pub u8);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerConfig {
    pub port: Port,
    pub character: Character,
    pub costume: Costume,
}
impl PlayerConfig {
    pub fn new(port: Port, character: Character) -> Self {
        Self {
            port,
            character,
            costume: Costume::default(),
        }
    }
}

/// Supported Versus rules: singles, stock, no items, normal damage, no timer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchRules {
    pub stocks: u8,
    /// Retail music rule 6 consults the roster unlock state.
    pub all_characters_unlocked: bool,
}
impl Default for MatchRules {
    fn default() -> Self {
        Self {
            stocks: 4,
            all_characters_unlocked: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchConfig {
    pub stage: Stage,
    pub players: [PlayerConfig; 2],
    pub rules: MatchRules,
    pub seed: Seed,
}
impl MatchConfig {
    /// Deterministic defaults: four stocks, default costumes, and seed zero.
    pub fn versus(stage: Stage, players: [PlayerConfig; 2]) -> Self {
        Self {
            stage,
            players,
            rules: MatchRules::default(),
            seed: Seed::default(),
        }
    }
    pub fn with_stocks(mut self, stocks: u8) -> Self {
        self.rules.stocks = stocks;
        self
    }
    pub fn with_seed(mut self, seed: Seed) -> Self {
        self.seed = seed;
        self
    }
    pub(crate) fn setup(&self) -> Result<Setup, StartError> {
        if self.players[0].port.index() >= self.players[1].port.index() {
            return Err(StartError::InvalidConfig(
                "players must occupy distinct ascending ports",
            ));
        }
        if !(1..=99).contains(&self.rules.stocks) {
            return Err(StartError::InvalidConfig("stocks must be in 1..=99"));
        }
        for player in &self.players {
            if usize::from(player.costume.0) >= player.character.descriptor().costumes.len() {
                return Err(StartError::InvalidConfig("unsupported character costume"));
            }
        }
        Ok(Setup {
            fighters: std::array::from_fn(|p| PlayerSetup {
                slot: self.players[p].port as u8,
                descriptor: self.players[p].character.descriptor(),
                costume: self.players[p].costume.0,
                spawn_point: -1,
                stocks: self.rules.stocks,
            }),
            stage: self.stage.descriptor(),
            seed: Some(self.seed.0),
            all_characters_unlocked: Some(self.rules.all_characters_unlocked),
        })
    }
}
