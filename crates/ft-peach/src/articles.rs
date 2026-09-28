//! Peach's hold on her articles: fp->u.pe parasol and Toad references,
//! the callbacks they install (ftPe_Init_OnDeath2) and the parasol hooks of
//! the shared parasol states (ftGetParasolStatus, ftCommon_8007E83C).
//!
//! The pre/post-hitlag callbacks the parasol and Toad install
//! (it_8026B724 / it_8026B73C, freezing the item for the owner's hitlag)
//! are not modelled: both items hold still in Peach's hand and the freeze
//! only delays their animation frame, which no compared state reads.
use crate::init::Peach;
use crate::special_hi::{it_parasol, PARASOL_FALL, PARASOL_OPEN};
use melee_ft::fighter::{
    parasol::{ParasolStatus, SpecialParasol},
    Fighter,
};
use melee_it::{ItemControl, ItemRequest};
use melee_types::ItemKind;

/// ftGetParasolStatus / ftCo_ItemParasolGetFallMotionId for Peach.
pub fn special_parasol(peach: &Peach) -> Option<SpecialParasol> {
    peach.items.parasol[0].then_some(SpecialParasol {
        status: if peach.items.parasol_motion == it_parasol::OPENING {
            ParasolStatus::Opening
        } else {
            ParasolStatus::Open
        },
        open: PARASOL_OPEN,
        fall: PARASOL_FALL,
    })
}

/// ftCommon_8007E83C (8007E83C), parasol_table_3/4: index 4 plays the
/// opening (it_802BDD40), index 6 holds it open (it_802BDDB4), at a rate
/// that spans `frames` (zero: the fighter's own rate).
pub fn set_parasol_animation(f: &mut Fighter, index: usize, frames: f32) {
    let motion = match index {
        4 => it_parasol::OPENING,
        6 => it_parasol::OPEN,
        _ => unimplemented!("ftcommon.c parasol_table_3[{index}]: no Peach parasol animation"),
    };
    let speed = f.animation.speed;
    let rate = if frames == 0.0 {
        speed
    } else {
        // retail 8007E91C..24: the int frame count as f32, fdivs, fmuls.
        let length = it_parasol::ANIMATION_FRAMES[usize::from(motion) - 1] as f32;
        speed * (length / frames)
    };
    f.character.get_mut::<Peach>().items.parasol_motion = motion;
    let control = if motion == it_parasol::OPENING {
        ItemControl::ParasolOpening(rate)
    } else {
        ItemControl::ParasolOpen(rate)
    };
    f.core.item_requests.push(ItemRequest::Control {
        owner: f.player.id,
        kind: ItemKind::PeachParasol,
        control,
    });
}

/// ftPe_8011D518 (8011D518): the parasol lets go of Peach, taking its
/// death3/take-damage callbacks with it.
fn release_parasol(f: &mut Fighter) {
    let peach = f.character.get_mut::<Peach>();
    peach.items.parasol[0] = false;
    peach.items.death3_armed = false;
    peach.items.take_damage_armed = false;
    // Item_8026A8EC releases fp->item_gobj with the item.
    if f
        .core
        .article_in_hand
        .is_some_and(|a| a.kind == ItemKind::PeachParasol)
    {
        f.core.article_in_hand = None;
    }
    // ftPe_8011D518: the item stowed under the parasol comes back to hand.
    if std::mem::take(&mut f.character.get_mut::<Peach>().items.parasol[1]) {
        f.core.restore_stowed_item();
    }
}

/// ftPe_SpecialN_DoDeath2 (8011E2A8): Toad lets go of Peach, taking its
/// death2/take-damage callbacks with it.
fn release_toad(peach: &mut Peach) {
    peach.items.toad = false;
    peach.items.death2_armed = false;
    peach.items.take_damage_armed = false;
}

/// The Destroyed callbacks of Peach's articles (itPeachParasol_Logic60_
/// Destroyed, itPeachToad_Logic91_Destroyed, itPeachTurnip_Logic56_
/// Destroyed); the spores and the blast have none.
pub fn destroyed(f: &mut Fighter, kind: ItemKind) {
    match kind {
        ItemKind::PeachParasol => release_parasol(f),
        ItemKind::PeachToad => release_toad(f.character.get_mut::<Peach>()),
        // itPeachTurnip_Logic56_Destroyed -> ftPe_SpecialLw_UnsetVeg.
        ItemKind::PeachTurnip => f.character.get_mut::<Peach>().items.vegetable = false,
        _ => {}
    }
}

fn remove(f: &mut Fighter, kind: ItemKind) {
    f.core.item_requests.push(ItemRequest::Control {
        owner: f.player.id,
        kind,
        control: ItemControl::Remove,
    });
}

/// ftPe_8011D598 (8011D598): it_802BDB94 destroys the parasol.
pub fn remove_parasol(f: &mut Fighter) {
    if f.character.get::<Peach>().items.parasol[0] {
        remove(f, ItemKind::PeachParasol);
        release_parasol(f);
    }
}

/// ftCommon_8007DB58: take_dmg_cb.
pub fn take_damage(f: &mut Fighter) {
    if f.character.get::<Peach>().items.take_damage_armed {
        put_away(f);
    }
}

/// ftCo_800D331C: death2_cb, then death3_cb (each ftPe_Init_OnDeath2; the
/// first leaves the second nothing to do).
pub fn death(f: &mut Fighter) {
    let items = &f.character.get::<Peach>().items;
    if items.death2_armed || items.death3_armed {
        put_away(f);
    }
}

/// ftPe_Init_OnDeath2 (8011B704): parasol, Toad, then the turnip.
pub fn put_away(f: &mut Fighter) {
    remove_parasol(f);
    // ftPe_SpecialN_OnDeath2: it_802BDF40.
    if f.character.get::<Peach>().items.toad {
        remove(f, ItemKind::PeachToad);
        release_toad(f.character.get_mut::<Peach>());
    }
    crate::special_lw::put_away(f);
}

/// ft_8008A348 (8008A348), ft_08A1.c:85-91: entering Wait with the parasol
/// in hand destroys it (it_802BDB94).
pub fn wait(f: &mut Fighter) {
    if f
        .core
        .article_in_hand
        .is_some_and(|a| a.kind == ItemKind::PeachParasol)
    {
        remove_parasol(f);
    }
}

/// ftCo_Landing_Enter (800D5AEC), ftCo_Landing.c:54-58: a landing with its
/// interrupt allowed puts the parasol away.
pub fn landing(f: &mut Fighter, allow_interrupt: bool) {
    if allow_interrupt {
        remove_parasol(f);
    }
}
