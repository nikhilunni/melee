//! VS. Mode > Special Melee: each mode is a major scene of its own
//! (GameModeKind) whose start callback changes the VS rules or players
//! (gmlightning.c, gmslomo.c, gmgiant.c, ...).

/// A Special Melee mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpecialMelee {
    /// GM_CAMERA_VS (0x2A).
    Camera,
    /// GM_STAMINA_VS (0x1F): 150 HP, one stock (gmstamina.c).
    Stamina,
    /// GM_SUPER_SUDDEN_DEATH_VS (0x10): every player starts at 300%.
    SuperSuddenDeath,
    /// GM_GIANT_VS (0x1E): model scale 1.8, attack ratio 1.5.
    Giant,
    /// GM_TINY_VS (0x1D): model scale 0.35, defense ratio 0.5.
    Tiny,
    /// GM_INVISIBLE_VS (0x11): every fighter cloaked.
    Invisible,
    /// GM_SINGLE_BUTTON_VS (0x2C).
    SingleButton,
    /// GM_LIGHTNING_VS (0x13): StartMeleeRules game_speed 1.25.
    Lightning,
    /// GM_SLOMO_VS (0x12): game_speed 0.5.
    SloMo,
}

impl SpecialMelee {
    pub const ALL: [Self; 9] = [
        Self::Camera,
        Self::Stamina,
        Self::SuperSuddenDeath,
        Self::Giant,
        Self::Tiny,
        Self::Invisible,
        Self::SingleButton,
        Self::Lightning,
        Self::SloMo,
    ];

    /// The mode's GameModeKind (gm/forward.h).
    pub fn game_mode(self) -> u8 {
        match self {
            Self::Camera => 0x2A,
            Self::Stamina => 0x1F,
            Self::SuperSuddenDeath => 0x10,
            Self::Giant => 0x1E,
            Self::Tiny => 0x1D,
            Self::Invisible => 0x11,
            Self::SingleButton => 0x2C,
            Self::Lightning => 0x13,
            Self::SloMo => 0x12,
        }
    }

    pub fn from_game_mode(mode: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.game_mode() == mode)
    }

    /// The spelling scenarios, recordings and `make_boundary.py --special` use.
    pub fn name(self) -> &'static str {
        match self {
            Self::Camera => "camera",
            Self::Stamina => "stamina",
            Self::SuperSuddenDeath => "super-sudden-death",
            Self::Giant => "giant",
            Self::Tiny => "tiny",
            Self::Invisible => "invisible",
            Self::SingleButton => "single-button",
            Self::Lightning => "lightning",
            Self::SloMo => "slomo",
        }
    }

    /// The player callback's overrides (gmgiant.c:132-135,
    /// gmtiny.c:132-135): model scale, attack ratio and defense ratio, which
    /// replace the handicap's.
    pub fn player_size(self) -> Option<(f32, f32, f32)> {
        match self {
            Self::Giant => Some((1.8, 1.5, 1.0)),
            Self::Tiny => Some((0.35, 1.0, 0.5)),
            _ => None,
        }
    }

    /// gmsupersudden.c:133: PlayerInitData x12 = 300, every player's
    /// starting damage (fn_8016D8AC's Player_SetHUDDamage).
    pub fn starting_damage(self) -> f32 {
        match self {
            Self::SuperSuddenDeath => 300.0,
            _ => 0.0,
        }
    }

    /// StartMeleeRules game_speed (gmlightning.c:135, gmslomo.c:133).
    pub fn game_speed(self) -> f32 {
        match self {
            Self::Lightning => 1.25,
            Self::SloMo => 0.5,
            _ => 1.0,
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.name() == name)
    }

    /// Whether the port plays the mode. Lightning and Slo-Mo change the
    /// scene's frame period (fn_8016E730's lb_80019880(1/60/game_speed),
    /// gm_16AE.c:1985): Lightning's polls follow it, so every pass ticks;
    /// Slo-Mo's stay at 1/60 s and every other pass runs nothing
    /// (`crate::frame_timer`). Super Sudden Death only starts
    /// every player at 300%. Giant and Tiny scale every fighter, which
    /// ftCo_800D105C's ftCo_800CF6E8 and the kinds' CalcIndiviParam scale
    /// with (ftchangeparam.c:178-195, guarded in `Fighter::prepare`).
    pub fn ported(self) -> bool {
        matches!(self, Self::Lightning | Self::SloMo | Self::SuperSuddenDeath)
    }

    /// Refuse a mode the port does not play, naming what it changes.
    pub(crate) fn require_ported(self) {
        if !self.ported() {
            unimplemented!(
                "{:?} (GameModeKind {:#x}): this Special Melee mode needs a retail witness",
                self,
                self.game_mode()
            );
        }
    }
}
