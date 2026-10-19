//! Kirby's take-damage and death callbacks, ftKb_Init_800EE7B8 and
//! ftKb_Init_800EE74C (ftkirby.c:2580-2601), and the two ways they go in.
//!
//! The specials install them as take_dmg_cb and death2_cb
//! (ftKb_SpecialN_set_cbs, `Kirby::damage_callbacks`), which every motion
//! change clears (fighter.c:1385-1389). A copied ability installs them as
//! death1_cb and death3_cb (ftKb_SpecialN_800F1BAC), which only the
//! callbacks themselves and Fighter_UnkInitReset (fighter.c:467-468) clear:
//! the first hit after a swallow, in any state, runs the damage callback.
use crate::init::Kirby;
use melee_ft::fighter::Fighter;

/// death1_cb = ftKb_Init_800EE7B8 and death3_cb = ftKb_Init_800EE74C, set
/// by ftKb_SpecialN_800F1BAC.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AbilityCallbacks {
    /// death1_cb: the next hit (ftCommon_8007DB58) or death.
    pub hit: bool,
    /// death3_cb: the next death (ftCo_800D331C).
    pub death: bool,
}

/// ftKb_Init_800EE7B8 (800EE7B8): the specials are released,
/// ftKb_SpecialN_800F9090 lets go of an inhaled item (none: the port
/// inhales no item), the copied special is put away (800F1A8C), and
/// death1_cb goes.
fn on_hit(f: &mut Fighter) {
    crate::special_hi::release_specials(f);
    crate::ability::put_away_on_hit(f);
    f.character.get_mut::<Kirby>().ability_callbacks.hit = false;
}

/// ftKb_Init_800EE74C (800EE74C): the specials are released, the inhaled
/// item let go (800F9090, none), the copied special put away (800F19AC)
/// and the ability lost without its star (800F5D04, which puts the special
/// away again); death3_cb goes.
fn on_death(f: &mut Fighter) {
    crate::special_hi::release_specials(f);
    crate::ability::lose(f, false);
    f.character.get_mut::<Kirby>().ability_callbacks.death = false;
}

/// ftCommon_8007DB58 (8007DB58): take_dmg_cb, then death1_cb.
pub fn take_damage(f: &mut Fighter) {
    if std::mem::take(&mut f.character.get_mut::<Kirby>().damage_callbacks) {
        on_hit(f);
    }
    if f.character.get::<Kirby>().ability_callbacks.hit {
        on_hit(f);
    }
}

/// ftCo_800D331C (ft_0D31.c:99-107): death2_cb, death3_cb, then death1_cb.
pub fn death(f: &mut Fighter) {
    if std::mem::take(&mut f.character.get_mut::<Kirby>().damage_callbacks) {
        on_death(f);
    }
    if f.character.get::<Kirby>().ability_callbacks.death {
        on_death(f);
    }
    if f.character.get::<Kirby>().ability_callbacks.hit {
        on_hit(f);
    }
}
