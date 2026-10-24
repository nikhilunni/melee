//! Charge Shot, ftsamusspecialn.c (801291A8..8012A068): the states are
//! `ft_samus_family::charge_shot`'s, shared with Kirby's copy, over
//! Samus's rows 343..348, her hand (RHandNb) and muzzle (ThrowN), colour
//! 53 and efAlt 0x486.
use crate::init::{install_damage_callbacks, Samus};
use ft_samus_family::charge_shot::{self, ChargeShotAttributes, ChargeShotUser, ChargeState};
use melee_ft::fighter::{assets::FighterAssets, ActionId, Fighter, MotionRow};
use melee_types::ItemKind;

pub use ft_samus_family::charge_shot::ChargeShot;

/// ftSs_MS_SpecialNStart (343) .. SpecialAirN (348).
pub const START: ActionId = ActionId(343);

/// ftCo_800BFFD0(fp, 53, 0): the full-charge colour animation, and
/// ftSs_Init_UnkMotionStates4's.
const FULL_CHARGE_COLOR: u8 = 53;

impl ChargeShotUser for Samus {
    const START: ActionId = START;
    /// ftSs_SM_SpecialNStart.
    const START_ANIMATION: i32 = 297;
    const SHOT: ItemKind = ItemKind::SamusCharge;
    /// `fp->parts[FtPart_RHandNb]` / `[FtPart_ThrowN]`: raw parts indices.
    const HAND_PART: usize = 50;
    const MUZZLE_PART: usize = 51;
    const FULL_CHARGE_COLOR: u8 = FULL_CHARGE_COLOR;
    /// efSync_Spawn(1158, gobj, &pos, &facing): efAlt 0x486, the muzzle
    /// flash model 0x7D1 turned to the facing.
    const MUZZLE_FLASH: u16 = 0x486;
    fn attributes(f: &Fighter) -> ChargeShotAttributes {
        f.character.get::<Samus>().attributes.charge_shot
    }
    fn state(f: &Fighter) -> &ChargeState {
        &f.character.get::<Samus>().charge
    }
    fn state_mut(f: &mut Fighter) -> &mut ChargeState {
        &mut f.character.get_mut::<Samus>().charge
    }
    fn arm_callbacks(f: &mut Fighter) {
        install_damage_callbacks(f);
    }
    /// ftSs_Init_UnkMotionStates4: colour 53 whenever the secondary colour
    /// slot empties, while u.ss.x2230 is the full level x18.
    fn level_changed(f: &mut Fighter) {
        let full = charge_shot::is_full::<Samus>(f);
        f.core.combat.secondary_color_fallback = full.then_some(FULL_CHARGE_COLOR);
    }
    /// ftSamus_destroyAllEF: efLib_DestroyAll while x2234 is set.
    fn take_charge_effects(f: &mut Fighter) -> bool {
        std::mem::take(&mut f.character.get_mut::<Samus>().charge.effects)
    }
}

pub const fn rows() -> [MotionRow; 6] {
    charge_shot::rows::<Samus>()
}

/// ftSs_SpecialN_Enter (8012954C) / ftSs_SpecialAirN_Enter (801295F0).
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    charge_shot::enter::<Samus>(f, air, a);
}

/// Samus's charge level (u.ss.x2230) and the colour that follows it.
pub fn set_charge_level(f: &mut Fighter, level: i32) {
    charge_shot::set_charge_level::<Samus>(f, level);
}

/// The shot's full level, fctiwz of the float x18.
pub fn full_charge(f: &Fighter) -> i32 {
    charge_shot::full_charge::<Samus>(f)
}

/// ftSs_SpecialN_801291F0 (801291F0): the shot is no longer Samus's
/// (x222C), and its charge effects go (ftSamus_destroyAllEF).
pub fn let_go(f: &mut Fighter) {
    charge_shot::let_go::<Samus>(f);
}

/// ftSamus_UnkAndDestroyAllEF: the shot in hand is destroyed
/// (it_802B5974), then the charge effects.
pub fn drop_shot(f: &mut Fighter) {
    charge_shot::drop_shot::<Samus>(f);
}
