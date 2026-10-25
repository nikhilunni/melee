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
    Kirby,
}
impl Character {
    /// Every registered character, in the retail character select screen's
    /// reading order (top row left to right, then the bottom row).
    pub const ALL: [Self; 26] = [
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
        Self::Kirby,
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
            Self::Kirby => "Kirby",
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
            melee_types::FighterKind::Kirby => Self::Kirby,
            _ => unreachable!("unregistered match character"),
        }
    }
}

/// Every Versus stage, in `StKind` order (gr/forward.h). A stage the port
/// has not ported is still named, configured and recorded; starting a match
/// on it fails with [`StartError::UnportedStage`](crate::StartError).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    FountainOfDreams,
    PokemonStadium,
    PrincessPeachsCastle,
    KongoJungle,
    Brinstar,
    Corneria,
    YoshisStory,
    Onett,
    MuteCity,
    RainbowCruise,
    JungleJapes,
    GreatBay,
    HyruleTemple,
    BrinstarDepths,
    YoshisIsland,
    GreenGreens,
    Fourside,
    MushroomKingdom,
    MushroomKingdom2,
    Venom,
    PokeFloats,
    BigBlue,
    IcicleMountain,
    FlatZone,
    DreamLand,
    YoshisIsland64,
    KongoJungle64,
    Battlefield,
    FinalDestination,
}
/// One Versus stage: the scenario/Slippi spelling (slp `stage_name`), the
/// name the game prints, and the stage-select `StKind`.
struct StageNames {
    name: &'static str,
    display: &'static str,
    st_kind: u8,
}
const fn names(name: &'static str, display: &'static str, st_kind: u8) -> StageNames {
    StageNames {
        name,
        display,
        st_kind,
    }
}
impl Stage {
    /// The stages a match can start on, in the order menus offer them.
    pub const PORTED: [Self; 15] = [
        Self::Battlefield,
        Self::FinalDestination,
        Self::DreamLand,
        Self::FountainOfDreams,
        Self::PokemonStadium,
        Self::YoshisStory,
        Self::YoshisIsland64,
        Self::KongoJungle64,
        Self::JungleJapes,
        Self::KongoJungle,
        Self::HyruleTemple,
        Self::Fourside,
        Self::BrinstarDepths,
        Self::FlatZone,
        Self::YoshisIsland,
    ];
    /// The ported stages where CPU players are witnessed against retail
    /// (docs/PORT_NOTES/CPU_PLAYERS.md); hosts offer a CPU only here.
    /// Listed by hand: porting a stage does not witness a CPU on it.
    pub const CPU_SUPPORTED: [Self; 14] = [
        Self::Battlefield,
        Self::FinalDestination,
        Self::DreamLand,
        Self::FountainOfDreams,
        Self::PokemonStadium,
        Self::YoshisStory,
        Self::YoshisIsland64,
        Self::KongoJungle64,
        Self::JungleJapes,
        Self::KongoJungle,
        Self::HyruleTemple,
        Self::Fourside,
        Self::BrinstarDepths,
        Self::FlatZone,
    ];
    /// Whether a CPU player is witnessed on this stage ([`Self::CPU_SUPPORTED`]).
    pub fn cpu_supported(self) -> bool {
        Self::CPU_SUPPORTED.contains(&self)
    }
    /// Every Versus stage (the 29 of the stage select screen), `StKind` order.
    pub const VERSUS: [Self; 29] = [
        Self::FountainOfDreams,
        Self::PokemonStadium,
        Self::PrincessPeachsCastle,
        Self::KongoJungle,
        Self::Brinstar,
        Self::Corneria,
        Self::YoshisStory,
        Self::Onett,
        Self::MuteCity,
        Self::RainbowCruise,
        Self::JungleJapes,
        Self::GreatBay,
        Self::HyruleTemple,
        Self::BrinstarDepths,
        Self::YoshisIsland,
        Self::GreenGreens,
        Self::Fourside,
        Self::MushroomKingdom,
        Self::MushroomKingdom2,
        Self::Venom,
        Self::PokeFloats,
        Self::BigBlue,
        Self::IcicleMountain,
        Self::FlatZone,
        Self::DreamLand,
        Self::YoshisIsland64,
        Self::KongoJungle64,
        Self::Battlefield,
        Self::FinalDestination,
    ];
    fn names(self) -> StageNames {
        match self {
            Self::FountainOfDreams => names("FountainOfDreams", "Fountain of Dreams", 0x02),
            Self::PokemonStadium => names("PokemonStadium", "Pokémon Stadium", 0x03),
            Self::PrincessPeachsCastle => {
                names("PrincessPeachsCastle", "Princess Peach's Castle", 0x04)
            }
            Self::KongoJungle => names("KongoJungle", "Kongo Jungle", 0x05),
            Self::Brinstar => names("Brinstar", "Brinstar", 0x06),
            Self::Corneria => names("Corneria", "Corneria", 0x07),
            Self::YoshisStory => names("YoshisStory", "Yoshi's Story", 0x08),
            Self::Onett => names("Onett", "Onett", 0x09),
            Self::MuteCity => names("MuteCity", "Mute City", 0x0A),
            Self::RainbowCruise => names("RainbowCruise", "Rainbow Cruise", 0x0B),
            Self::JungleJapes => names("JungleJapes", "Jungle Japes", 0x0C),
            Self::GreatBay => names("GreatBay", "Great Bay", 0x0D),
            Self::HyruleTemple => names("HyruleTemple", "Temple", 0x0E),
            Self::BrinstarDepths => names("BrinstarDepths", "Brinstar Depths", 0x0F),
            Self::YoshisIsland => names("YoshisIsland", "Yoshi's Island", 0x10),
            Self::GreenGreens => names("GreenGreens", "Green Greens", 0x11),
            Self::Fourside => names("Fourside", "Fourside", 0x12),
            Self::MushroomKingdom => names("MushroomKingdom", "Mushroom Kingdom", 0x13),
            Self::MushroomKingdom2 => names("MushroomKingdom2", "Mushroom Kingdom II", 0x14),
            Self::Venom => names("Venom", "Venom", 0x16),
            Self::PokeFloats => names("PokeFloats", "Poké Floats", 0x17),
            Self::BigBlue => names("BigBlue", "Big Blue", 0x18),
            Self::IcicleMountain => names("IcicleMountain", "Icicle Mountain", 0x19),
            Self::FlatZone => names("FlatZone", "Flat Zone", 0x1B),
            Self::DreamLand => names("DreamLand", "Dream Land N64", 0x1C),
            Self::YoshisIsland64 => names("YoshisIsland64", "Yoshi's Island N64", 0x1D),
            Self::KongoJungle64 => names("KongoJungle64", "Kongo Jungle N64", 0x1E),
            Self::Battlefield => names("Battlefield", "Battlefield", 0x1F),
            Self::FinalDestination => names("FinalDestination", "Final Destination", 0x20),
        }
    }
    /// The name as the game spells it.
    pub fn display_name(self) -> &'static str {
        self.names().display
    }
    /// The scenario and Slippi spelling (`stage = "KongoJungle64"`).
    pub fn name(self) -> &'static str {
        self.names().name
    }
    /// The stage-select `StKind` (gr/forward.h), the Slippi stage id.
    pub fn st_kind(self) -> u8 {
        self.names().st_kind
    }
    pub fn from_name(name: &str) -> Option<Self> {
        Self::VERSUS.into_iter().find(|s| s.name() == name)
    }
    pub fn from_st_kind(st_kind: u16) -> Option<Self> {
        Self::VERSUS
            .into_iter()
            .find(|s| u16::from(s.st_kind()) == st_kind)
    }
    /// Whether a match can start on the stage.
    pub fn is_ported(self) -> bool {
        self.descriptor().is_some()
    }
    /// The most players a match with CPU players may have: a CPU choosing
    /// among two or more opponents ranks them by the standings
    /// (ftCo_800A53DC, gm_8016C6C0), which `melee-cpu` still guards.
    pub const CPU_MAX_PLAYERS: usize = 2;
    /// Whether the Item Switch is witnessed for `players` players on the
    /// stage (it_8026D018's weights scale with the player count); starting
    /// elsewhere with items on fails closed. The single list menus read.
    pub fn items_supported(self, players: usize) -> bool {
        players == 2 && matches!(self, Self::FinalDestination | Self::Battlefield)
    }
    pub(crate) fn descriptor(self) -> Option<&'static crate::scene_stage::StageDescriptor> {
        crate::scene_stage::descriptor(self.name())
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
    /// The player's team in a team battle (PlayerInitData +9: 0 red, 1 blue,
    /// 2 green); `None` in a free-for-all. Every player has one or none does.
    pub team: Option<u8>,
    /// Versus CPU level, 1..=9; None for a human.
    pub cpu_level: Option<u8>,
    /// The player's handicap, 1..=9, with the Handicap rule on; None with
    /// it off. It scales launches (lbl_803B7930's attack and defense
    /// ratios).
    pub handicap: Option<u8>,
}
impl PlayerConfig {
    pub fn new(port: Port, character: Character) -> Self {
        Self {
            port,
            character,
            costume: Costume::default(),
            controller_fix: ControllerFix::Off,
            team: None,
            cpu_level: None,
            handicap: None,
        }
    }
    pub fn with_cpu_level(mut self, level: u8) -> Self {
        self.cpu_level = Some(level);
        self
    }
    pub fn with_team(mut self, team: u8) -> Self {
        self.team = Some(team);
        self
    }
    pub fn with_controller_fix(mut self, fix: ControllerFix) -> Self {
        self.controller_fix = fix;
        self
    }
}
pub use melee_ft::input::ControllerFix;
use crate::ItemRules;

/// The Item Switch kinds whose it/kinds files the port registers
/// (`scene_items::Resources::load`, common items); a random spawn of any
/// other kind fails closed (`spawn_random_item`). Containers draw their
/// contents through the same switch, so these alone keep a match inside
/// the port. Party Ball's monsters (Goomba, ReDead) are registered too.
pub const PORTED_RANDOM_ITEMS: [melee_types::ItemKind; 10] = {
    use melee_types::ItemKind::*;
    [
        Capsule, Box, Taru, Kusudama, BombHei, Dosei, Bat, Sword, StarRod, Harisen,
    ]
};
/// An Item Switch at `frequency` (0 Very Low .. 4 Very High) with only
/// [`PORTED_RANDOM_ITEMS`] on. Bits from [`melee_it::random_spawn::PICKABLE_KINDS`]
/// up stay set, as on the retail switch (gm_1601.c:3534).
pub fn ported_item_switch(frequency: u8) -> ItemRules {
    let unpickable = !((1u64 << melee_it::random_spawn::PICKABLE_KINDS) - 1);
    let mask = PORTED_RANDOM_ITEMS
        .iter()
        .fold(unpickable, |mask, &kind| mask | 1 << kind as i32);
    ItemRules {
        frequency: frequency as i8,
        mask,
    }
}

/// Supported Versus rules: singles, stock, normal damage; an optional
/// timer and items.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchRules {
    /// StartMeleeRules match_kind: Stock (the default) or Time, where KOs
    /// score and the timer (`time_limit_seconds`, required) ends the match.
    pub mode: crate::MatchKind,
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
    /// StartMeleeRules friendly_fire: in a team battle, teammates' attacks
    /// land (gm_8016B0D4).
    pub friendly_fire: bool,
    /// The Item Switch (StartMeleeRules xB and x20): off by default.
    pub items: ItemRules,
    /// The Damage Ratio rule in tenths, 5..=20 (1.0x is 10): it scales
    /// every launch (StartMeleeRules x30).
    pub damage_ratio_tenths: u8,
    /// A Special Melee mode instead of Melee (only the ported ones).
    pub special: Option<crate::SpecialMelee>,
}
impl Default for MatchRules {
    fn default() -> Self {
        Self {
            mode: crate::MatchKind::Stock,
            stocks: 4,
            time_limit_seconds: None,
            sudden_death: false,
            all_characters_unlocked: true,
            friendly_fire: false,
            items: ItemRules::OFF,
            damage_ratio_tenths: 10,
            special: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchConfig {
    pub stage: Stage,
    /// Two to four players on ascending ports.
    pub players: Vec<PlayerConfig>,
    pub rules: MatchRules,
    pub seed: Seed,
}
impl MatchConfig {
    /// Deterministic defaults: four stocks, default costumes, and seed zero.
    pub fn versus(stage: Stage, players: impl IntoIterator<Item = PlayerConfig>) -> Self {
        Self {
            stage,
            players: players.into_iter().collect(),
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
    /// The checks `Match::new` makes of the configuration alone (no
    /// assets): a menu can refuse an unsupported combination before loading.
    pub fn validate(&self) -> Result<(), StartError> {
        self.setup().map(|_| ())
    }
    pub(crate) fn setup(&self) -> Result<Setup, StartError> {
        if !(2..=4).contains(&self.players.len()) {
            return Err(StartError::InvalidConfig("a match has two to four players"));
        }
        if self
            .players
            .windows(2)
            .any(|pair| pair[0].port.index() >= pair[1].port.index())
        {
            return Err(StartError::InvalidConfig(
                "players must occupy distinct ascending ports",
            ));
        }
        let teams: Vec<Option<u8>> = self.players.iter().map(|p| p.team).collect();
        if teams.iter().any(Option::is_some) {
            if teams.iter().any(|t| !matches!(t, Some(0..=2))) {
                return Err(StartError::InvalidConfig(
                    "a team battle gives every player a team, 0 to 2",
                ));
            }
            if teams.iter().all(|t| *t == teams[0]) {
                return Err(StartError::InvalidConfig("a team battle needs two teams"));
            }
        }
        match self.rules.mode {
            crate::MatchKind::Stock => {}
            crate::MatchKind::Time if self.rules.time_limit_seconds.is_some() => {}
            crate::MatchKind::Time => {
                return Err(StartError::InvalidConfig("a Time match needs its time limit"));
            }
            crate::MatchKind::Coin | crate::MatchKind::Bonus => {
                return Err(StartError::InvalidConfig(
                    "Coin and Bonus matches need a retail witness",
                ));
            }
        }
        if self.rules.sudden_death
            && (self.rules.stocks != 1
                || self.rules.time_limit_seconds.is_some()
                || self.rules.mode != crate::MatchKind::Stock)
        {
            return Err(StartError::InvalidConfig(
                "Sudden Death has one stock and no timer",
            ));
        }
        if self.rules.time_limit_seconds == Some(0) {
            return Err(StartError::InvalidConfig("a time limit must be positive"));
        }
        if self.rules.items.validate().is_err() {
            return Err(StartError::InvalidConfig("item frequency must be in -1..=4"));
        }
        if self.rules.special.is_some_and(|mode| !mode.ported()) {
            return Err(StartError::InvalidConfig(
                "this Special Melee mode needs a retail witness",
            ));
        }
        if !(5..=20).contains(&self.rules.damage_ratio_tenths) {
            return Err(StartError::InvalidConfig("the damage ratio is 0.5..=2.0"));
        }
        if !(1..=99).contains(&self.rules.stocks) {
            return Err(StartError::InvalidConfig("stocks must be in 1..=99"));
        }
        for player in &self.players {
            if player.handicap.is_some_and(|level| !(1..=9).contains(&level)) {
                return Err(StartError::InvalidConfig("a handicap is 1..=9"));
            }
            if player.cpu_level.is_some_and(|level| !(1..=9).contains(&level)) {
                return Err(StartError::InvalidConfig("a CPU level is 1..=9"));
            }
            if usize::from(player.costume.0) >= player.character.descriptor().costumes.len() {
                return Err(StartError::InvalidConfig("unsupported character costume"));
            }
        }
        Ok(Setup {
            fighters: self
                .players
                .iter()
                .map(|player| PlayerSetup {
                    slot: player.port as u8,
                    descriptor: player.character.descriptor(),
                    costume: player.costume.0,
                    spawn_point: -1,
                    stocks: self.rules.stocks,
                    controller_fix: player.controller_fix,
                    team: player.team,
                    cpu_level: player.cpu_level,
                    handicap: player.handicap,
                })
                .collect(),
            stage: self.stage.descriptor().ok_or(StartError::UnportedStage(self.stage))?,
            seed: Some(self.seed.0),
            all_characters_unlocked: Some(self.rules.all_characters_unlocked),
            mode: self.rules.mode,
            special: self.rules.special,
            // gm_80167BC8: 0.1f * damage_ratio (gm_1601.c:3643).
            damage_ratio: 0.1 * f32::from(self.rules.damage_ratio_tenths),
            time_limit: self.rules.time_limit_seconds,
            sudden_death: self.rules.sudden_death,
            friendly_fire: self.rules.friendly_fire,
            items: self.rules.items,
            slippi: Default::default(),
        })
    }
}
