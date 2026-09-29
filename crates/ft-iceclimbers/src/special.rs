//! The climbers' special entries (ftData_Special*[FTKIND_POPO / NANA]) and
//! the article clean-up their callbacks share.
use melee_ft::fighter::{assets::FighterAssets, Fighter, SpecialSlot};

/// ftData_SpecialN / SpecialAirN: ftPp_SpecialN_Enter for both climbers.
/// Nana's Hi and S entries are NULL (the input checks never reach here
/// for them).
pub fn enter(f: &mut Fighter, slot: SpecialSlot, airborne: bool, assets: &FighterAssets) {
    match slot {
        SpecialSlot::Neutral => crate::special_n::enter(f, airborne, assets),
        SpecialSlot::Side => crate::special_s::enter(f, airborne, assets),
        SpecialSlot::Up | SpecialSlot::Down => unimplemented!(
            "ftData_Special{slot:?}[{:?}] (airborne: {airborne}): the climbers' special",
            f.core.kind
        ),
    }
}

/// ftPp_Init_8011F060 (8011F060), the take_dmg_cb and death2_cb the Ice
/// Shot and the Squall Hammer install: the held block breaks
/// (ftPp_Init_8011F190), and both climbers leave each other's hitlag
/// (ftPp_SpecialS_8011F68C). Nana's Squall Hammer rows install its tail,
/// ftNn_Init_80122FAC (ftNn_Init_801238E4), which finds no block. The
/// Belay's clean-ups (ftPp_SpecialHi_80122898, ftPp_SpecialS_80121164)
/// find nothing of theirs while the Belay is unported.
pub fn lose_articles(f: &mut Fighter) {
    if !crate::climber::vars(f).ice_callbacks {
        return;
    }
    crate::special_n::break_held_ice(f);
    crate::partner::separate(f);
}
