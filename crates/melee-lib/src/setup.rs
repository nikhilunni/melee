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
}
impl PlayerSetup {
    pub fn descriptor(&self) -> &'static CharacterDescriptor {
        self.descriptor
    }
}
#[derive(Clone)]
pub struct Setup {
    pub fighters: [PlayerSetup; 2],
    pub stage: &'static crate::scene_stage::StageDescriptor,
    pub seed: Option<u32>,
    pub all_characters_unlocked: Option<bool>,
    /// A counting-down match timer, in seconds.
    pub time_limit: Option<u32>,
    /// The Sudden Death scene after a timed-out tie (gm_SetupSuddenDeath,
    /// gm_Scene_SuddenDeath_OnEnter): one stock each at 300%, its own
    /// countdown and the Bob-omb rain.
    pub sudden_death: bool,
    /// Slippi codes the recording ran (all off in retail).
    pub slippi: crate::slippi::SlippiCodes,
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
