//! The climbers' special entries (ftData_Special*[FTKIND_POPO / NANA]) and
//! the article clean-up their callbacks share.
use melee_ft::fighter::{assets::FighterAssets, Fighter, SpecialSlot};

/// ftData_SpecialN / SpecialAirN: ftPp_SpecialN_Enter for both climbers.
/// Nana's Hi and S entries are NULL (the input checks never reach here
/// for them).
pub fn enter(f: &mut Fighter, slot: SpecialSlot, airborne: bool, assets: &FighterAssets) {
    match slot {
        SpecialSlot::Neutral => crate::special_n::enter(f, airborne, assets),
        SpecialSlot::Side | SpecialSlot::Up | SpecialSlot::Down => unimplemented!(
            "ftData_Special{slot:?}[{:?}] (airborne: {airborne}): the climbers' special",
            f.core.kind
        ),
    }
}

/// ftPp_Init_8011F060 (8011F060), the take_dmg_cb and death2_cb the Ice
/// Shot installs: its block breaks (ftPp_Init_8011F190); the Belay's and
/// Squall Hammer's clean-ups (ftPp_SpecialHi_80122898,
/// ftPp_SpecialS_80121164, ftPp_SpecialS_8011F68C) find nothing of theirs
/// while those specials are unported.
pub fn lose_articles(f: &mut Fighter) {
    if !crate::climber::vars(f).ice_callbacks {
        return;
    }
    crate::special_n::break_held_ice(f);
}
