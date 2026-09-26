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
    rows
}
pub static GROUND_MOVES: [Option<GroundMove>; super::super::COMMON_COUNT] = attack_moves();
#[derive(Default, Clone, Debug)]
pub struct StaleHistory {
    entries: [Option<AttackInstance>; 10],
    current: Option<GroundMove>,
    serial: u64,
}
impl StaleHistory {
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
