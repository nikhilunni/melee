//! Giant Punch, ftdonkeyspecialn.c (8010E840..8010F81C): the states are
//! `ft_donkey_family::giant_punch`'s, shared with Kirby's copy, over
//! Donkey Kong's rows 369..378, colour 57 and efSync 0x4C8 / 0x4C9.
use crate::{common, init::DonkeyKong};
use ft_donkey_family::giant_punch::{self, GiantPunchAttributes, GiantPunchUser};
use melee_ft::fighter::{assets::FighterAssets, ActionId, Fighter, MotionRow};

pub use ft_donkey_family::giant_punch::GiantPunch;

/// ftDk_MS_SpecialNStart (369) .. ftDk_MS_SpecialAirNFull (378).
pub const START: ActionId = ActionId(369);

/// ftCo_800BFFD0(fp, 57, 0): the full charge's glow, and
/// ftDk_Init_UnkMotionStates4's.
pub const FULL_CHARGE_COLOR: u8 = 57;

impl GiantPunchUser for DonkeyKong {
    const START: ActionId = START;
    /// ftDk_SM_SpecialNStart: row 369's anim_id (0x803CB838).
    const FIRST_ANIMATION: i32 = 0x13F;
    const FULL_CHARGE_COLOR: u8 = FULL_CHARGE_COLOR;
    /// efSync_Spawn(1224 / 1225, gobj, TopN, &facing): the punch's models
    /// 0x1F42 (grounded) and 0x1F43 (aerial).
    const PUNCH_EFFECTS: [u16; 2] = [0x4C8, 0x4C9];
    fn attributes(f: &Fighter) -> GiantPunchAttributes {
        f.character.get::<DonkeyKong>().attributes.giant_punch
    }
    fn scratch(f: &mut Fighter) -> &mut GiantPunch {
        &mut f.character.get_mut::<DonkeyKong>().giant_punch
    }
    fn swings(f: &Fighter) -> i32 {
        f.character.get::<DonkeyKong>().punch_swings
    }
    fn store_swings(f: &mut Fighter, swings: i32) {
        set_swings(f, swings);
    }
    /// setCallbacks: take_dmg_cb and death2_cb = ftDk_Init_8010D774,
    /// take_dmg_2_cb = ftDk_SpecialN_DestroyAllEffects, and the efLib
    /// hitlag pair.
    fn arm_callbacks(f: &mut Fighter) {
        common::install_damage_callbacks(f);
        f.character.get_mut::<DonkeyKong>().hit_destroys_effects = true;
    }
    /// clearCallbacks: take_dmg_2_cb and the hitlag pair go.
    fn clear_callbacks(f: &mut Fighter) {
        f.character.get_mut::<DonkeyKong>().hit_destroys_effects = false;
        f.effect_state.hitlag_callbacks = false;
    }
}

pub const fn rows() -> [MotionRow; 10] {
    giant_punch::rows::<DonkeyKong>()
}

/// The stored swings (u.dk.x222C), and ftDk_Init_UnkMotionStates4's glow
/// (colour 57 whenever the secondary colour slot empties) while they are
/// the full charge.
pub fn set_swings(f: &mut Fighter, swings: i32) {
    f.character.get_mut::<DonkeyKong>().punch_swings = swings;
    let full = swings == DonkeyKong::attributes(f).max_swings;
    f.core.combat.secondary_color_fallback = full.then_some(FULL_CHARGE_COLOR);
}

/// ftDk_SpecialN_Enter (8010E840) / ftDk_SpecialAirN_Enter (8010E930).
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    giant_punch::enter::<DonkeyKong>(f, air, a);
}
