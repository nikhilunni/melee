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
        SpecialSlot::Up => crate::special_hi::enter(f, airborne, assets),
        SpecialSlot::Down => crate::special_lw::enter(f, airborne, assets),
    }
}

/// ftCommon_8007DB58's take_dmg_cb: ftPp_Init_8011F060 while the Ice
/// Shot, the Squall Hammer, the Blizzard or the Belay's rope installed it.
pub fn take_damage(f: &mut Fighter) {
    let v = crate::climber::vars(f);
    if v.ice_callbacks || v.belay.rope_take_damage {
        lose_articles(f);
    }
}

/// ftCo_800D331C's death2_cb (the Ice Shot's, Squall Hammer's and
/// Blizzard's) and
/// death3_cb (the Belay's rope's): ftPp_Init_8011F060.
pub fn death(f: &mut Fighter) {
    let v = crate::climber::vars(f);
    if v.ice_callbacks || v.belay.rope_death {
        lose_articles(f);
    }
}

/// ftPp_Init_8011F060 (8011F060), in retail order: the held block breaks
/// (ftPp_Init_8011F190), the Blizzard's breath stops
/// (ftPp_SpecialHi_80122898, the Blizzard's despite its name), the Belay's
/// rope goes (ftPp_SpecialS_80121164), and both climbers leave each
/// other's hitlag (ftPp_SpecialS_8011F68C). Nana's Squall Hammer rows
/// install its tail, ftNn_Init_80122FAC (ftNn_Init_801238E4), which finds
/// no block.
fn lose_articles(f: &mut Fighter) {
    crate::special_n::break_held_ice(f);
    crate::special_lw::stop_breath(f);
    crate::special_hi::drop_rope(f);
    crate::partner::separate(f);
}
