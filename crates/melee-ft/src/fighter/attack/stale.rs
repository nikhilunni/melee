//! plStale_UpdateStaleMovesFromFighter (8003722C), ft_80089118.
//! Fixed history in newest-first order; current attack instances register once.
use melee_types::CommonMotionState as S;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroundMove {
    Jab1,
    Jab2,
    Jab3,
    RapidJab,
    Dash,
    SideTilt,
    UpTilt,
    DownTilt,
    SideSmash,
    UpSmash,
    DownSmash,
    NeutralAir,
    ForwardAir,
    BackAir,
    UpAir,
    DownAir,
}
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
    rows
}
pub static GROUND_MOVES: [Option<GroundMove>; super::super::COMMON_COUNT] = attack_moves();
#[derive(Default)]
pub struct StaleHistory {
    entries: [Option<GroundMove>; 10],
    current: Option<GroundMove>,
    recorded: bool,
}
impl StaleHistory {
    /// ft_800890D0: a different move (or leaving attacks) starts a new instance.
    pub fn enter(&mut self, current: Option<GroundMove>) {
        if current.is_none() || current != self.current {
            self.recorded = false;
        }
        self.current = current;
    }
    /// ft_800892A0: rapid-jab animation wrap starts a fresh instance.
    pub fn new_instance(&mut self) {
        self.recorded = false;
    }
    pub fn record(&mut self) {
        if self.current.is_some() && !self.recorded {
            self.entries.rotate_right(1);
            self.entries[0] = self.current;
            self.recorded = true;
        }
    }
    /// ft_80089118: subtract each of nine matching weights in retail order.
    pub fn multiplier(&self, weights: &[f32; 9]) -> f32 {
        let mut multiplier = 1.0;
        for (entry, weight) in self.entries.iter().zip(weights) {
            if entry.is_none() {
                break;
            }
            if *entry == self.current {
                multiplier -= weight;
            }
        }
        multiplier
    }
}
