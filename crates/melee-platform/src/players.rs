//! Who plays and under which rules: the four character select slots
//! (human, CPU or off, with a team) and the Versus rules the menus offer.
//! [`crate::app::App`] owns one [`Lineup`]; hosts read and change it
//! through the app so its checks run on every change.
use melee_lib::{
    Character, Costume, ItemRules, MatchConfig, MatchKind, PlayerConfig, Port, Seed, SpecialMelee,
    Stage,
};

/// Character select slots, one per controller port.
pub const SLOTS: usize = 4;
/// The highest Versus CPU level.
pub const MAX_CPU_LEVEL: u8 = 9;
/// Retail's starting CPU level in the Versus character select.
pub const DEFAULT_CPU_LEVEL: u8 = 3;
pub const DEFAULT_STOCKS: u8 = 4;
/// Team colours (PlayerInitData +9): red, blue, green.
pub const TEAMS: u8 = 3;

/// What drives a slot: a person at a keyboard or gamepad, the game's CPU,
/// or nobody (the slot is out of the match).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum Controller {
    #[default]
    Off = 0,
    Human = 1,
    Cpu = 2,
}
impl Controller {
    pub fn from_u32(value: u32) -> Option<Self> {
        Some(match value {
            0 => Self::Off,
            1 => Self::Human,
            2 => Self::Cpu,
            _ => return None,
        })
    }
}

/// One player's character select slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot {
    pub controller: Controller,
    pub character: Option<Character>,
    pub costume: u8,
    /// 1..=9; used while the slot is a CPU.
    pub cpu_level: u8,
    /// 0 red, 1 blue, 2 green; used in a team battle.
    pub team: u8,
    /// Handicap 1..=9 (PlayerInitData handicap), or 0 for none.
    pub handicap: u8,
}
impl Slot {
    fn new(controller: Controller, team: u8) -> Self {
        Self {
            controller,
            character: None,
            costume: 0,
            cpu_level: DEFAULT_CPU_LEVEL,
            team,
            handicap: 0,
        }
    }
    /// Whether the slot takes part in the match.
    pub fn is_active(&self) -> bool {
        self.controller != Controller::Off
    }
}

/// How a match is won: the last player with stocks, or the most KOs when
/// the timer runs out. (Coin and Bonus matches are not ported.)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum Mode {
    #[default]
    Stock = 0,
    Time = 1,
}
impl Mode {
    pub fn from_u32(value: u32) -> Option<Self> {
        Some(match value {
            0 => Self::Stock,
            1 => Self::Time,
            _ => return None,
        })
    }
}

/// The Special Melee modes the menus offer, in menu order: those the
/// simulator plays through. Stamina is ported but its KO at 0 HP is still
/// guarded, so it waits.
pub const SPECIAL_MODES: [SpecialMelee; 3] = [
    SpecialMelee::SuperSuddenDeath,
    SpecialMelee::Lightning,
    SpecialMelee::SloMo,
];

/// The Versus rules the menus offer. Further modes join here as the
/// simulator gains them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rules {
    pub mode: Mode,
    /// Stocks in a stock match, 1..=99.
    pub stocks: u8,
    /// The time limit in minutes, 1..=99: a Time match's length, or an
    /// optional limit on a stock match.
    pub time_minutes: Option<u8>,
    /// A team battle: every player is on one of [`TEAMS`] teams.
    pub teams: bool,
    /// In a team battle, teammates' attacks land.
    pub friendly_fire: bool,
    /// The Item Switch frequency: `None` off, `Some(0)` Very Low ..
    /// `Some(4)` Very High; every ported item kind switched on.
    pub item_frequency: Option<u8>,
    /// Damage Ratio in tenths, 5..=20 (10 is 1.0x).
    pub damage_ratio_tenths: u8,
    /// One of [`SPECIAL_MODES`] instead of a plain Melee.
    pub special: Option<SpecialMelee>,
}
impl Default for Rules {
    fn default() -> Self {
        Self {
            mode: Mode::Stock,
            stocks: DEFAULT_STOCKS,
            time_minutes: None,
            teams: false,
            friendly_fire: false,
            item_frequency: None,
            damage_ratio_tenths: 10,
            special: None,
        }
    }
}
/// The Item Switch's highest frequency (Very High).
pub const MAX_ITEM_FREQUENCY: u8 = 4;
/// A Time match's default length, as retail's rules screen starts.
pub const DEFAULT_TIME_MINUTES: u8 = 2;
impl Rules {
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=99).contains(&self.stocks) {
            return Err("stocks must be 1 to 99".into());
        }
        if self.time_minutes.is_some_and(|m| !(1..=99).contains(&m)) {
            return Err("a time limit is 1 to 99 minutes".into());
        }
        if self.mode == Mode::Time && self.time_minutes.is_none() {
            return Err("a Time match needs a time limit".into());
        }
        if self.item_frequency.is_some_and(|f| f > MAX_ITEM_FREQUENCY) {
            return Err("item frequency must be 0 (very low) to 4 (very high)".into());
        }
        if !(5..=20).contains(&self.damage_ratio_tenths) {
            return Err("the damage ratio is 0.5 to 2.0".into());
        }
        if self
            .special
            .is_some_and(|mode| !SPECIAL_MODES.contains(&mode))
        {
            return Err("that Special Melee mode is not playable yet".into());
        }
        Ok(())
    }
    /// The switch with every ported item kind on
    /// ([`melee_lib::PORTED_RANDOM_ITEMS`]).
    fn items(&self) -> ItemRules {
        self.item_frequency
            .map_or(ItemRules::OFF, melee_lib::ported_item_switch)
    }
}

/// The four slots and the rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lineup {
    pub slots: [Slot; SLOTS],
    pub rules: Rules,
}
impl Default for Lineup {
    /// Two humans, two empty slots; teams red, blue, red, blue.
    fn default() -> Self {
        Self {
            slots: [
                Slot::new(Controller::Human, 0),
                Slot::new(Controller::Human, 1),
                Slot::new(Controller::Off, 0),
                Slot::new(Controller::Off, 1),
            ],
            rules: Rules::default(),
        }
    }
}

pub fn check_player(player: usize) -> Result<(), String> {
    if player < SLOTS {
        Ok(())
    } else {
        Err(format!("player {} does not exist", player + 1))
    }
}

impl Lineup {
    pub fn active(&self) -> impl Iterator<Item = (usize, &Slot)> {
        self.slots.iter().enumerate().filter(|(_, s)| s.is_active())
    }
    /// Whether another active slot wears `character`'s `costume`.
    pub fn taken_by_other(&self, player: usize, character: Character, costume: u8) -> bool {
        self.active().any(|(index, slot)| {
            index != player && slot.character == Some(character) && slot.costume == costume
        })
    }
    /// The first costume of `character` no other active slot wears.
    pub fn free_costume(&self, player: usize, character: Character) -> u8 {
        (0..character.costume_count())
            .find(|&c| !self.taken_by_other(player, character, c))
            .unwrap_or(0)
    }
    pub fn has_cpu(&self) -> bool {
        self.active().any(|(_, s)| s.controller == Controller::Cpu)
    }
    /// Why the picks cannot start a match yet, or `None` when they can.
    pub fn not_ready(&self) -> Option<&'static str> {
        let active: Vec<_> = self.active().map(|(_, s)| s).collect();
        if active.len() < 2 {
            return Some("a match needs at least two players");
        }
        if active.iter().any(|s| s.character.is_none()) {
            return Some("every player needs a character");
        }
        if self.rules.teams && active.iter().all(|s| s.team == active[0].team) {
            return Some("a team battle needs two teams");
        }
        None
    }
    /// Why `stage` cannot host these picks, or `None` when it can.
    pub fn stage_unavailable(&self, stage: Stage) -> Option<&'static str> {
        if !stage.is_ported() {
            return Some("not ported yet");
        }
        if self.has_cpu() && !stage.cpu_supported() {
            return Some("no CPU players here yet");
        }
        if self.has_cpu() && self.active().count() > Stage::CPU_MAX_PLAYERS {
            return Some("CPU players in matches of two only, so far");
        }
        if self.rules.item_frequency.is_some() && !stage.items_supported(self.active().count()) {
            return Some(
                if (2..=4).contains(&self.active().count()) && stage.items_supported(2) {
                    "items here with two players only, so far"
                } else {
                    "no items here yet"
                },
            );
        }
        None
    }
    /// The match these picks describe.
    pub fn config(&self, stage: Stage, seed: u32) -> Result<MatchConfig, String> {
        if let Some(reason) = self.not_ready() {
            return Err(reason.into());
        }
        if let Some(reason) = self.stage_unavailable(stage) {
            return Err(format!("{}: {reason}", stage.display_name()));
        }
        self.rules.validate()?;
        let players = self.active().map(|(index, slot)| {
            let character = slot.character.expect("checked by not_ready");
            let mut player = PlayerConfig::new(Port::ALL[index], character);
            player.costume = Costume(slot.costume);
            if slot.controller == Controller::Cpu {
                player = player.with_cpu_level(slot.cpu_level);
            }
            if self.rules.teams {
                player = player.with_team(slot.team);
            }
            if slot.handicap != 0 {
                player.handicap = Some(slot.handicap);
            }
            player
        });
        let mut config = MatchConfig::versus(stage, players.collect::<Vec<_>>())
            .with_stocks(self.rules.stocks)
            .with_seed(Seed(seed));
        let rules = &mut config.rules;
        rules.mode = match self.rules.mode {
            Mode::Stock => MatchKind::Stock,
            Mode::Time => MatchKind::Time,
        };
        rules.time_limit_seconds = self.rules.time_minutes.map(|m| u32::from(m) * 60);
        rules.friendly_fire = self.rules.teams && self.rules.friendly_fire;
        rules.items = self.rules.items();
        rules.damage_ratio_tenths = self.rules.damage_ratio_tenths;
        rules.special = self.rules.special;
        config.validate().map_err(|e| e.to_string())?;
        Ok(config)
    }
}
