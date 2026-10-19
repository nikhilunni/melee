//! Rollout, ftpurinspecialn.c: the states are `ft_purin_family::rollout`'s
//! (shared with Kirby's copy), over Jigglypuff's rows 346..362.
use crate::init::Jigglypuff;
use ft_purin_family::{
    rollout, LandingDust, RolloutAttributes, RolloutKind, RolloutRows, RolloutState,
};
use melee_ft::fighter::{assets::FighterAssets, Fighter, MotionRow};

impl RolloutKind for Jigglypuff {
    /// ftPr_MS_SpecialNStartR (346), ftPr_SM_SpecialNStartR (300).
    const ROWS: RolloutRows = RolloutRows {
        first_action: 346,
        first_animation: 300,
    };
    const LANDING_DUST: LandingDust = LandingDust::FacingAndFloor;
    fn attributes(f: &Fighter) -> &RolloutAttributes {
        &f.character.get::<Jigglypuff>().attributes.rollout
    }
    fn state(f: &mut Fighter) -> &mut RolloutState {
        &mut f.character.get_mut::<Jigglypuff>().rollout
    }
    fn state_ref(f: &Fighter) -> &RolloutState {
        &f.character.get::<Jigglypuff>().rollout
    }
    /// setupPurinCallbacks: death2_cb and take_dmg_cb = ftPr_SpecialS_8013D658.
    fn arm_damage_callbacks(f: &mut Fighter) {
        f.character.get_mut::<Jigglypuff>().rollout_restore = true;
    }
}

/// ftPr_MS_SpecialNStartR..SpecialNHit (346..362).
pub const fn rows() -> [MotionRow; 17] {
    rollout::rows::<Jigglypuff>()
}

/// ftPr_SpecialN_Enter (8013DEA8) / ftPr_SpecialAirN_Enter (8013DF60).
pub fn enter(f: &mut Fighter, air: bool, assets: &FighterAssets) {
    rollout::enter::<Jigglypuff>(f, air, assets);
}

/// deal_dmg_cb = ftPr_SpecialS_8013D764 while Rollout installed it.
pub fn bounce(f: &mut Fighter, assets: &FighterAssets) {
    rollout::bounce::<Jigglypuff>(f, assets);
}

/// ftPr_SpecialS_8013D658 (8013D658), death2_cb and take_dmg_cb: the
/// model's scale and facing return.
pub fn restore_on_callback(f: &mut Fighter) {
    if f.character.get::<Jigglypuff>().rollout_restore {
        rollout::restore::<Jigglypuff>(f);
    }
}
