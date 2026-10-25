//! plStale_UpdateStaleMovesFromFighter (8003722C), ft_80089118.
//! Fixed history in newest-first order; current attack instances register once.
use melee_types::combat::{AttackInstance, StaleMove as GroundMove};
use melee_types::CommonMotionState as S;
const fn attack_moves() -> [Option<GroundMove>; super::super::COMMON_COUNT] {
    let mut rows = [None; super::super::COMMON_COUNT];
    rows[S::Attack11 as usize] = Some(GroundMove::Jab1);
    rows[S::Attack12 as usize] = Some(GroundMove::Jab2);
    rows[S::Attack13 as usize] = Some(GroundMove::Jab3);
    rows[S::Attack100Start as usize] = Some(GroundMove::RapidJab);
    rows[S::Attack100Loop as usize] = Some(GroundMove::RapidJab);
    rows[S::Attack100End as usize] = Some(GroundMove::RapidJab);
    rows[S::AttackDash as usize] = Some(GroundMove::Dash);
    let mut i = S::AttackS3Hi as usize;
    while i <= S::AttackS3Lw as usize {
        rows[i] = Some(GroundMove::SideTilt);
        i += 1;
    }
    rows[S::AttackHi3 as usize] = Some(GroundMove::UpTilt);
    rows[S::AttackLw3 as usize] = Some(GroundMove::DownTilt);
    i = S::AttackS4Hi as usize;
    while i <= S::AttackS4Lw as usize {
        rows[i] = Some(GroundMove::SideSmash);
        i += 1;
    }
    rows[S::AttackHi4 as usize] = Some(GroundMove::UpSmash);
    rows[S::AttackLw4 as usize] = Some(GroundMove::DownSmash);
    rows[S::AttackAirN as usize] = Some(GroundMove::NeutralAir);
    rows[S::AttackAirF as usize] = Some(GroundMove::ForwardAir);
    rows[S::AttackAirB as usize] = Some(GroundMove::BackAir);
    rows[S::AttackAirHi as usize] = Some(GroundMove::UpAir);
    rows[S::AttackAirLw as usize] = Some(GroundMove::DownAir);
    // ftData_MotionStateList[70..74]: landing lag keeps the aerial's move id.
    rows[S::LandingAirN as usize] = Some(GroundMove::NeutralAir);
    rows[S::LandingAirF as usize] = Some(GroundMove::ForwardAir);
    rows[S::LandingAirB as usize] = Some(GroundMove::BackAir);
    rows[S::LandingAirHi as usize] = Some(GroundMove::UpAir);
    rows[S::LandingAirLw as usize] = Some(GroundMove::DownAir);
    rows[S::DownAttackU as usize] = Some(GroundMove::GetupAttackFaceUp);
    rows[S::DownAttackD as usize] = Some(GroundMove::GetupAttackFaceDown);
    rows[S::CliffAttackSlow as usize] = Some(GroundMove::LedgeAttackSlow);
    rows[S::CliffAttackQuick as usize] = Some(GroundMove::LedgeAttackQuick);
    rows[S::CatchAttack as usize] = Some(GroundMove::Pummel);
    rows[S::ThrowF as usize] = Some(GroundMove::ThrowForward);
    rows[S::ThrowB as usize] = Some(GroundMove::ThrowBack);
    rows[S::ThrowHi as usize] = Some(GroundMove::ThrowUp);
    rows[S::ThrowLw as usize] = Some(GroundMove::ThrowDown);
    // ftData_MotionStateList[120..=123]: the rows' x4 low bytes 0x40..0x43.
    rows[S::SwordSwing1 as usize] = Some(GroundMove::SwordSwing1);
    rows[S::SwordSwing3 as usize] = Some(GroundMove::SwordSwing3);
    rows[S::SwordSwing4 as usize] = Some(GroundMove::SwordSwing4);
    rows[S::SwordSwingDash as usize] = Some(GroundMove::SwordSwingDash);
    // [124..=143]: x4 low bytes 0x44..0x57, four per battering item.
    rows[S::BatSwing1 as usize] = Some(GroundMove::BatSwing1);
    rows[S::BatSwing3 as usize] = Some(GroundMove::BatSwing3);
    rows[S::BatSwing4 as usize] = Some(GroundMove::BatSwing4);
    rows[S::BatSwingDash as usize] = Some(GroundMove::BatSwingDash);
    rows[S::ParasolSwing1 as usize] = Some(GroundMove::ParasolSwing1);
    rows[S::ParasolSwing3 as usize] = Some(GroundMove::ParasolSwing3);
    rows[S::ParasolSwing4 as usize] = Some(GroundMove::ParasolSwing4);
    rows[S::ParasolSwingDash as usize] = Some(GroundMove::ParasolSwingDash);
    rows[S::HarisenSwing1 as usize] = Some(GroundMove::HarisenSwing1);
    rows[S::HarisenSwing3 as usize] = Some(GroundMove::HarisenSwing3);
    rows[S::HarisenSwing4 as usize] = Some(GroundMove::HarisenSwing4);
    rows[S::HarisenSwingDash as usize] = Some(GroundMove::HarisenSwingDash);
    rows[S::StarRodSwing1 as usize] = Some(GroundMove::StarRodSwing1);
    rows[S::StarRodSwing3 as usize] = Some(GroundMove::StarRodSwing3);
    rows[S::StarRodSwing4 as usize] = Some(GroundMove::StarRodSwing4);
    rows[S::StarRodSwingDash as usize] = Some(GroundMove::StarRodSwingDash);
    rows[S::LipstickSwing1 as usize] = Some(GroundMove::LipstickSwing1);
    rows[S::LipstickSwing3 as usize] = Some(GroundMove::LipstickSwing3);
    rows[S::LipstickSwing4 as usize] = Some(GroundMove::LipstickSwing4);
    rows[S::LipstickSwingDash as usize] = Some(GroundMove::LipstickSwingDash);
    rows
}
pub static GROUND_MOVES: [Option<GroundMove>; super::super::COMMON_COUNT] = attack_moves();
/// Where a player's second fighter (Nana) starts numbering its attack
/// instances, so they never equal the first fighter's: retail draws both
/// from one counter (plStale_IncrementAttackInstance).
const SECONDARY_SERIALS: u64 = 1 << 40;

#[derive(Default, Clone, Debug)]
pub struct StaleHistory {
    entries: [Option<AttackInstance>; 10],
    current: Option<GroundMove>,
    serial: u64,
    /// Counts insertions, so a player's two fighters can tell whose copy
    /// of the player's table is newer (`share_table`).
    revision: u64,
}
impl StaleHistory {
    /// A fresh history for one of a player's fighters.
    pub fn for_fighter(secondary: bool) -> Self {
        Self {
            serial: if secondary { SECONDARY_SERIALS } else { 0 },
            ..Self::default()
        }
    }
    /// Fighter_UnkInitReset (80067C98) at a revival: the fighter's own attack
    /// (x2068_attackID) is forgotten. The table is the player's and only
    /// plStale_ResetStaleMoveTableForPlayer empties it, at the stock loss
    /// (ftCo_800D34E0, retail 0x800D3528) and when the player is created
    /// (pl_8003891C, 0x80038940): what the player's second fighter landed
    /// between its leader's death and the revival still stales.
    pub fn reset(&mut self) {
        self.current = None;
    }
    /// plStale_ResetStaleMoveTableForPlayer (ftCo_800D34E0): the player's
    /// table empties when either of its fighters loses a stock or falls; the
    /// clearing counts as a change the other fighter takes.
    pub fn reset_table(&mut self) {
        self.entries = [None; 10];
        self.revision += 1;
    }
    /// The table is the player's (Player_GetStaleMoveTableIndexPtr), shared
    /// by its two fighters (Popo and Nana); each fighter keeps a copy. Take
    /// `other`'s entries if they are newer. Returns whether they were.
    pub fn share_table(&mut self, other: &StaleHistory) -> bool {
        if other.revision <= self.revision {
            return false;
        }
        self.entries = other.entries;
        self.revision = other.revision;
        true
    }
    pub fn current_move(&self) -> Option<GroundMove> {
        self.current
    }
    /// ft_800890D0: a different move (or leaving attacks) starts a new instance.
    pub fn enter(&mut self, current: Option<GroundMove>) {
        if current.is_none() || current != self.current {
            self.serial += 1;
        }
        self.current = current;
    }
    /// ft_800892A0: rapid-jab animation wrap starts a fresh instance.
    pub fn new_instance(&mut self) {
        self.serial += 1;
    }
    pub fn attack(&self) -> Option<AttackInstance> {
        self.current.map(|move_id| AttackInstance {
            move_id,
            serial: self.serial,
        })
    }
    pub fn record(&mut self) {
        if let Some(attack) = self.attack() {
            self.record_attack(attack);
        }
    }
    /// plStale_UpdateStaleMovesFromItem: use the instance captured at item creation.
    pub fn record_attack(&mut self, attack: AttackInstance) {
        if !self.entries.contains(&Some(attack)) {
            self.entries.rotate_right(1);
            self.entries[0] = Some(attack);
            self.revision += 1;
        }
    }
    /// ft_80089118: subtract each of nine matching weights in retail order.
    pub fn multiplier(&self, weights: &[f32; 9]) -> f32 {
        self.multiplier_for(self.current, weights)
    }
    pub fn multiplier_for(&self, current: Option<GroundMove>, weights: &[f32; 9]) -> f32 {
        let mut multiplier = 1.0;
        for (entry, weight) in self.entries.iter().zip(weights) {
            if entry.is_none() {
                break;
            }
            if entry.map(|attack| attack.move_id) == current {
                multiplier -= weight;
            }
        }
        multiplier
    }
}
