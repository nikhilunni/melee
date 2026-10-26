//! Validated setup shared by the public API and the oracle adapter.
use melee_ft::fighter::assets::CharacterDescriptor;
#[derive(Clone)]
pub struct PlayerSetup {
    pub slot: u8,
    pub descriptor: &'static CharacterDescriptor,
    pub costume: u8,
    pub spawn_point: i8,
    pub stocks: u8,
    /// The player's controller-fix Gecko code (UCF), keyed by its port.
    pub controller_fix: melee_ft::input::ControllerFix,
    /// PlayerInitData team in a team battle (StartMeleeRules is_teams);
    /// `None` in a free-for-all. Either every player has one or none does.
    pub team: Option<u8>,
    /// Versus CPU level, 1..=9; None for a human.
    pub cpu_level: Option<u8>,
    /// The player's handicap, 1..=9, with the Handicap rule on
    /// (PlayerInitData +8); None with it off.
    pub handicap: Option<u8>,
}
impl PlayerSetup {
    /// gm_80167BC8 (gm_1601.c:3613-3638): the attack and defense ratios
    /// lbl_803B7930 gives the handicap, 1.0 with the rule off.
    pub fn handicap_ratios(&self) -> (f32, f32) {
        match self.handicap {
            None => (1.0, 1.0),
            Some(level) => HANDICAP_RATIOS[usize::from(level) - 1],
        }
    }
}
/// lbl_803B7930 (0x803B7930): attack and defense ratio by handicap 1..=9.
pub const HANDICAP_RATIOS: [(f32, f32); 9] = [
    (0.35, 2.857143),
    (0.4, 2.5),
    (0.46, 2.173913),
    (0.53, 1.8867925),
    (0.61, 1.6393442),
    (0.7, 1.4285715),
    (0.8, 1.25),
    (0.93, 1.0752689),
    (1.07, 0.9345794),
];
impl PlayerSetup {
    pub fn descriptor(&self) -> &'static CharacterDescriptor {
        self.descriptor
    }
}
impl Setup {
    /// Player `p`'s attack and defense ratios: a Giant or Tiny match's,
    /// else the handicap's.
    pub fn ratios(&self, p: usize) -> (f32, f32) {
        match self.special.and_then(crate::SpecialMelee::player_size) {
            Some((_, attack, defense)) => (attack, defense),
            None => self.fighters[p].handicap_ratios(),
        }
    }
    /// Player_GetModelScale for every player.
    pub fn model_scale(&self) -> f32 {
        self.special
            .and_then(crate::SpecialMelee::player_size)
            .map_or(1.0, |(scale, ..)| scale)
    }
}
#[derive(Clone)]
pub struct Setup {
    pub fighters: Vec<PlayerSetup>,
    pub stage: &'static crate::scene_stage::StageDescriptor,
    pub seed: Option<u32>,
    pub all_characters_unlocked: Option<bool>,
    /// StartMeleeRules match_kind: Stock, or Time (KOs score, the timer
    /// ends the match).
    pub mode: melee_types::MatchKind,
    /// A counting-down match timer, in seconds.
    pub time_limit: Option<u32>,
    /// A Special Melee mode's scene instead of Melee's.
    pub special: Option<crate::SpecialMelee>,
    /// StartMeleeRules x30 (gm_8016B248), the Damage Ratio rule:
    /// 0.1f * the rule's tenths.
    pub damage_ratio: f32,
    /// The Sudden Death scene after a timed-out tie (gm_SetupSuddenDeath,
    /// gm_Scene_SuddenDeath_OnEnter): one stock each at 300%, its own
    /// countdown and the Bob-omb rain.
    pub sudden_death: bool,
    /// StartMeleeRules friendly_fire: teammates' attacks land.
    pub friendly_fire: bool,
    /// The Item Switch: frequency and the kinds that spawn.
    pub items: crate::ItemRules,
    /// Slippi codes the recording ran (all off in retail).
    pub slippi: crate::slippi::SlippiCodes,
    /// The stale heap word each new Fan reads as xD4C, in creation order
    /// (`melee_it::ItemPool::set_heap_ammo_words`): uninitialised memory
    /// that no game state decides, named by a scenario from its recording.
    /// Empty for a match the port starts itself.
    pub heap_ammo_words: Vec<i32>,
}
/// One fighter GObj a match creates, in creation order (the fighter list
/// order): each player's fighter, then its partner (Player_80031AD0 creates
/// `player_entity[0]`, then `player_entity[1]` for ftMapping_list's
/// `extra_internal_id`, e.g. Nana with Popo).
#[derive(Clone, Copy)]
pub struct RosterEntry {
    /// Index into `Setup::fighters`.
    pub player: usize,
    pub descriptor: &'static CharacterDescriptor,
    /// x221F_b4: the player's second fighter (plAllocInfo.b0).
    pub secondary: bool,
}
impl Setup {
    /// Every fighter the players create, in fighter-list order.
    pub fn roster(&self) -> Vec<RosterEntry> {
        let mut roster = Vec::with_capacity(2 * self.fighters.len());
        for (player, setup) in self.fighters.iter().enumerate() {
            roster.push(RosterEntry {
                player,
                descriptor: setup.descriptor,
                secondary: false,
            });
            if let Some(partner) = crate::scene_fighter::SceneFighter::partner_for(setup.descriptor)
            {
                roster.push(RosterEntry {
                    player,
                    descriptor: partner,
                    secondary: true,
                });
            }
        }
        roster
    }
    pub fn roster_descriptors(&self) -> Vec<&'static CharacterDescriptor> {
        self.roster().iter().map(|entry| entry.descriptor).collect()
    }
    pub fn stage_descriptor(&self) -> &'static crate::scene_stage::StageDescriptor {
        self.stage
    }
    /// Each port's controller fix (Off for an empty port), or why one
    /// cannot be simulated.
    pub fn controller_fixes(&self) -> anyhow::Result<[melee_ft::input::ControllerFix; 4]> {
        let mut fixes = [melee_ft::input::ControllerFix::Off; 4];
        for player in &self.fighters {
            if let Some(reason) = player.controller_fix.unsupported() {
                anyhow::bail!("port {}: {reason}", player.slot);
            }
            fixes[usize::from(player.slot)] = player.controller_fix;
        }
        Ok(fixes)
    }
}
