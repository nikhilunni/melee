//! The copied Rollout, ftkirbyspecialpurin.c: Jigglypuff's states
//! (`ft_purin_family::rollout`) over Kirby's rows 475..491, with his
//! attribute block (+2B4), his saved scale (u.kb.x8C) and his damage
//! callbacks (ftKb_PrSetCallbacks: ftKb_Init_800EE74C / 800EE7B8).
use crate::init::Kirby;
use ft_purin_family::{
    rollout, LandingDust, RolloutAttributes, RolloutKind, RolloutRows, RolloutState,
};
use hsd_types::Vec3;
use melee_ft::fighter::{assets::FighterAssets, Fighter, MotionRow};

/// Kirby with Jigglypuff's ability.
pub struct KirbyPurin;

/// ftKb_SpecialN_800F190C: u.kb.x8C at rest.
pub const SCALE_AT_REST: Vec3 = Vec3::new(1.0, 1.0, 1.0);

/// The copied Rollout's state with u.kb.x8C at rest.
pub fn at_rest() -> RolloutState {
    RolloutState {
        scale: SCALE_AT_REST,
        ..Default::default()
    }
}

impl RolloutKind for KirbyPurin {
    /// ftKb_MS_PrSpecialNStartR (475), ftKb_SM_PrSpecialNStartR (410).
    const ROWS: RolloutRows = RolloutRows {
        first_action: 475,
        first_animation: 410,
    };
    const LANDING_DUST: LandingDust = LandingDust::DirectionAndRoll;
    /// ftKb_DatAttrs +2B4 (`specialn_pr_*`).
    fn attributes(f: &Fighter) -> &RolloutAttributes {
        &f.character.get::<Kirby>().attributes.rollout
    }
    fn state(f: &mut Fighter) -> &mut RolloutState {
        &mut f.character.get_mut::<Kirby>().rollout
    }
    fn state_ref(f: &Fighter) -> &RolloutState {
        &f.character.get::<Kirby>().rollout
    }
    /// ftKb_PrSetCallbacks: death2_cb = ftKb_Init_800EE74C, take_dmg_cb =
    /// ftKb_Init_800EE7B8.
    fn arm_damage_callbacks(f: &mut Fighter) {
        f.character.get_mut::<Kirby>().damage_callbacks = true;
    }
}

/// ftKb_SpecialNPr_80101560 / 80101618 (the grounded and aerial entries).
pub fn enter(f: &mut Fighter, airborne: bool, assets: &FighterAssets) {
    rollout::enter::<KirbyPurin>(f, airborne, assets);
}

/// Rows 475..491 (ftKb_MS_PrSpecialN*): ftKb_PrSpecialN*.
pub const fn rows() -> [MotionRow; 17] {
    rollout::rows::<KirbyPurin>()
}

/// deal_dmg_cb = fn_80100E0C while the copied Rollout installed it.
pub fn deal_damage(f: &mut Fighter, assets: &FighterAssets) {
    rollout::bounce::<KirbyPurin>(f, assets);
}
