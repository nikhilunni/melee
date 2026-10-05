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
    Pikachu,
    Mario,
    Roy,
    DrMario,
    Luigi,
    Pichu,
    Ganondorf,
    /// Popo, with Nana as the player's partner fighter.
    IceClimbers,
    Samus,
    /// Sheik at match start (Zelda's CSS icon with A held while the match
    /// loads); Zelda is loaded beside her as the transformation partner.
    Sheik,
    /// Zelda, with Sheik loaded beside her as the transformation partner.
    Zelda,
    Link,
    YoungLink,
    DonkeyKong,
    Bowser,
    /// Mr. Game & Watch.
    GameAndWatch,
    Ness,
    Mewtwo,
}
impl Character {
    /// Every registered character, in the retail character select screen's
    /// reading order (top row left to right, then the bottom row).
    pub const ALL: [Self; 25] = [
        Self::DrMario,
        Self::Mario,
        Self::Luigi,
        Self::Bowser,
        Self::Peach,
        Self::Yoshi,
        Self::DonkeyKong,
        Self::CaptainFalcon,
        Self::Ganondorf,
        Self::Falco,
        Self::Fox,
        Self::Ness,
        Self::IceClimbers,
        Self::Samus,
        Self::Zelda,
        Self::Sheik,
        Self::Link,
        Self::YoungLink,
        Self::Pichu,
        Self::Pikachu,
        Self::Jigglypuff,
        Self::Mewtwo,
        Self::GameAndWatch,
        Self::Marth,
        Self::Roy,
    ];
    /// The name as the game spells it.
    pub fn display_name(self) -> &'static str {
        match self {
            Self::CaptainFalcon => "Captain Falcon",
            Self::DrMario => "Dr. Mario",
            Self::IceClimbers => "Ice Climbers",
            Self::YoungLink => "Young Link",
            Self::DonkeyKong => "Donkey Kong",
            Self::GameAndWatch => "Mr. Game & Watch",
            other => other.name(),
        }
    }
    /// How many costumes the character has; [`Costume`] indexes below this.
    pub fn costume_count(self) -> u8 {
        self.descriptor().costumes.len() as u8
    }
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Fox => "Fox",
            Self::Marth => "Marth",
            Self::Falco => "Falco",
            Self::CaptainFalcon => "CaptainFalcon",
            Self::Peach => "Peach",
            Self::Yoshi => "Yoshi",
            Self::Jigglypuff => "Jigglypuff",
            Self::Pikachu => "Pikachu",
            Self::Mario => "Mario",
            Self::Roy => "Roy",
            Self::DrMario => "DrMario",
            Self::Luigi => "Luigi",
            Self::Pichu => "Pichu",
            Self::Ganondorf => "Ganondorf",
            Self::IceClimbers => "IceClimbers",
            Self::Samus => "Samus",
            Self::Sheik => "Sheik",
            Self::Zelda => "Zelda",
            Self::Link => "Link",
            Self::YoungLink => "YoungLink",
            Self::DonkeyKong => "DonkeyKong",
            Self::Bowser => "Bowser",
            Self::GameAndWatch => "GameAndWatch",
            Self::Ness => "Ness",
            Self::Mewtwo => "Mewtwo",
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
            melee_types::FighterKind::Pikachu => Self::Pikachu,
            melee_types::FighterKind::Mario => Self::Mario,
            melee_types::FighterKind::Emblem => Self::Roy,
            melee_types::FighterKind::DrMario => Self::DrMario,
            melee_types::FighterKind::Luigi => Self::Luigi,
            melee_types::FighterKind::Pichu => Self::Pichu,
            melee_types::FighterKind::Ganon => Self::Ganondorf,
            melee_types::FighterKind::Popo => Self::IceClimbers,
            melee_types::FighterKind::Samus => Self::Samus,
            melee_types::FighterKind::Seak => Self::Sheik,
            melee_types::FighterKind::Zelda => Self::Zelda,
            melee_types::FighterKind::Link => Self::Link,
            melee_types::FighterKind::CLink => Self::YoungLink,
            melee_types::FighterKind::Donkey => Self::DonkeyKong,
            melee_types::FighterKind::Koopa => Self::Bowser,
            melee_types::FighterKind::GameWatch => Self::GameAndWatch,
            melee_types::FighterKind::Ness => Self::Ness,
            melee_types::FighterKind::Mewtwo => Self::Mewtwo,
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
    FountainOfDreams,
    PokemonStadium,
}
impl Stage {
    pub const ALL: [Self; 6] = [
        Self::Battlefield,
        Self::FinalDestination,
        Self::DreamLand,
        Self::FountainOfDreams,
        Self::PokemonStadium,
        Self::YoshisStory,
    ];
    /// The name as the game spells it.
    pub fn display_name(self) -> &'static str {
        match self {
            Self::FinalDestination => "Final Destination",
            Self::Battlefield => "Battlefield",
            Self::YoshisStory => "Yoshi's Story",
            Self::DreamLand => "Dream Land N64",
            Self::FountainOfDreams => "Fountain of Dreams",
            Self::PokemonStadium => "Pokémon Stadium",
        }
    }
    pub(crate) fn descriptor(self) -> &'static crate::scene_stage::StageDescriptor {
        use crate::scene_stage::*;
        match self {
            Self::FinalDestination => &FINAL_DESTINATION,
            Self::Battlefield => &BATTLEFIELD,
            Self::YoshisStory => &YOSHIS_STORY,
            Self::DreamLand => &DREAM_LAND,
            Self::FountainOfDreams => &FOUNTAIN_OF_DREAMS,
            Self::PokemonStadium => &POKEMON_STADIUM,
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
    /// The controller-fix Gecko code the player's console ran (UCF);
    /// retail is [`ControllerFix::Off`].
    pub controller_fix: ControllerFix,
}
impl PlayerConfig {
    pub fn new(port: Port, character: Character) -> Self {
        Self {
            port,
            character,
            costume: Costume::default(),
            controller_fix: ControllerFix::Off,
        }
    }
    pub fn with_controller_fix(mut self, fix: ControllerFix) -> Self {
        self.controller_fix = fix;
        self
    }
}
pub use melee_ft::input::ControllerFix;

/// Supported Versus rules: singles, stock, no items, normal damage, no timer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchRules {
    pub stocks: u8,
    /// StartMeleeRules time_limit: a counting-down timer, in seconds (eight
    /// minutes is 480). When it runs out the player with more stocks wins; a
    /// stock tie goes to Sudden Death.
    pub time_limit_seconds: Option<u32>,
    /// A Sudden Death match (gm_SetupSuddenDeath): one stock at 300%, its own
    /// countdown and the Bob-omb rain, no timer. `Match::sudden_death`
    /// continues a timed-out tie into one.
    pub sudden_death: bool,
    /// Retail music rule 6 consults the roster unlock state.
    pub all_characters_unlocked: bool,
}
impl Default for MatchRules {
    fn default() -> Self {
        Self {
            stocks: 4,
            time_limit_seconds: None,
            sudden_death: false,
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
        if self.rules.sudden_death
            && (self.rules.stocks != 1 || self.rules.time_limit_seconds.is_some())
        {
            return Err(StartError::InvalidConfig(
                "Sudden Death has one stock and no timer",
            ));
        }
        if self.rules.time_limit_seconds == Some(0) {
            return Err(StartError::InvalidConfig("a time limit must be positive"));
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
                controller_fix: self.players[p].controller_fix,
            }),
            stage: self.stage.descriptor(),
            seed: Some(self.seed.0),
            all_characters_unlocked: Some(self.rules.all_characters_unlocked),
            time_limit: self.rules.time_limit_seconds,
            sudden_death: self.rules.sudden_death,
            slippi: Default::default(),
        })
    }
}
