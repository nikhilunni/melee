//! The copied Toad, ftkirbyspecialpeach.c: Peach's states
//! (`ft_peach_family::toad`) over Kirby's rows 449..452, his attributes
//! (ftKb_DatAttrs +178..+18C and the AbsorbDesc at +3EC), his bone 44 and
//! his article kinds; the damage callbacks are his own (ftKb_Init_800EE74C /
//! 800EE7B8, which put Toad away through 800F19AC / 800F1A8C).
use crate::init::{Accessory, Kirby};
use ft_peach_family::toad::{
    self, CounterVolume, SpecialN, ToadAccessory, ToadAttributes, ToadUser,
};
use melee_ft::fighter::{assets::FighterAssets, Fighter, MotionRow};
use melee_it::{ItemControl, ItemRequest};
use melee_types::{FighterKind, ItemKind};

/// Kirby with Peach's ability.
pub struct KirbyPeach;

/// fp->parts[FtPart_R3rdNa]: Kirby's bone 44 (fn_8010C288, fn_8010C344).
const ARTICLE_JOINT: usize = 44;

/// The copied Toad's scratch: Toad's volume and caught hit, and u.kb.xD0.
#[derive(Clone, Debug, Default)]
pub struct KirbyToad {
    pub special_n: SpecialN,
    /// u.kb.xD0: Toad is out.
    pub toad_out: bool,
}

fn state(f: &mut Fighter) -> &mut KirbyToad {
    &mut f.character.get_mut::<Kirby>().toad
}

impl ToadUser for KirbyPeach {
    /// ftKb_MS_PeSpecialLw..PeSpecialAirLwHit (449..452).
    const FIRST_ROW: u16 = 449;
    const ARTICLE_JOINT: usize = ARTICLE_JOINT;
    const TOAD: ItemKind = ItemKind::KirbyPeachToad;
    const SPORE: ItemKind = ItemKind::KirbyPeachToadSpore;
    /// ftKb_DatAttrs +178..+18C (`specialn_pe_*`).
    fn attributes(f: &Fighter) -> &ToadAttributes {
        &f.character.get::<Kirby>().attributes.peach_toad
    }
    /// ftKb_DatAttrs +3EC (`specialn_pe_absorbdesc`).
    fn volume(f: &Fighter) -> &CounterVolume {
        &f.character.get::<Kirby>().attributes.peach_toad_volume
    }
    fn scratch(f: &mut Fighter) -> &mut SpecialN {
        &mut state(f).special_n
    }
    /// u.kb.xCC.
    fn lift_used(f: &mut Fighter) -> &mut bool {
        &mut f.character.get_mut::<Kirby>().toad_lift_used
    }
    fn toad_out(f: &Fighter) -> bool {
        f.character.get::<Kirby>().toad.toad_out
    }
    /// fn_8010C288: u.kb.xD0, and death2_cb / take_dmg_cb = ftKb_Init_800EE74C
    /// / 800EE7B8 (no x1984_heldItemSpec, unlike Peach's).
    fn hold_toad(f: &mut Fighter) {
        let kirby = f.character.get_mut::<Kirby>();
        kirby.toad.toad_out = true;
        kirby.damage_callbacks = true;
    }
    fn arm_damage_callbacks(f: &mut Fighter) {
        f.character.get_mut::<Kirby>().damage_callbacks = true;
    }
    fn install_accessory(f: &mut Fighter, accessory: ToadAccessory) {
        f.character.get_mut::<Kirby>().accessory = match accessory {
            ToadAccessory::DrawToad => Accessory::ToadDraw,
            ToadAccessory::ReleaseSpore => Accessory::ToadSpore,
        };
    }
}

/// ftKb_SpecialNPe_8010C4D4 / 8010C560 (the grounded and aerial entries).
pub fn enter(f: &mut Fighter, airborne: bool, assets: &FighterAssets) {
    toad::enter::<KirbyPeach>(f, airborne, assets);
}

/// Rows 449..452, ftKb_SM_PeSpecialLw.. (384..387).
pub const fn rows() -> [MotionRow; 4] {
    toad::rows::<KirbyPeach>(384)
}

/// accessory4_cb = fn_8010C288 / fn_8010C344, each uninstalling itself.
pub fn accessory(f: &mut Fighter, accessory: Accessory) {
    f.core.accessory4_armed = false;
    f.character.get_mut::<Kirby>().accessory = Accessory::None;
    match accessory {
        Accessory::ToadDraw => toad::draw_toad::<KirbyPeach>(f),
        Accessory::ToadSpore => toad::release_spore::<KirbyPeach>(f),
        _ => unreachable!("{accessory:?} is not Toad's"),
    }
}

fn is_peach(f: &Fighter) -> bool {
    f.character.get::<Kirby>().copied == FighterKind::Peach
}

/// fn_8010CD88, Toad's shield_hit_cb.
pub fn process_hit(f: &mut Fighter, a: &FighterAssets) {
    if is_peach(f) {
        toad::process_hit::<KirbyPeach>(f, a);
    }
}

/// Toad's volume against a fighter's hit.
pub fn contact(f: &mut Fighter, attacker: &mut Fighter, a: &FighterAssets, id: usize) -> bool {
    is_peach(f) && toad::contact::<KirbyPeach>(f, attacker, a, id)
}

/// Toad's volume against an item's hit.
pub fn item_contact(
    f: &mut Fighter,
    item: &mut melee_it::ItemCore,
    id: usize,
    assets: &FighterAssets,
) -> bool {
    is_peach(f) && toad::item_contact::<KirbyPeach>(f, item, id, assets)
}

/// itPeachToad_Logic91_Destroyed for Kirby's kind -> ftKb_SpecialNPe_8010C3C0
/// (8010C3C0): its thaw (8010C47C, the unmodelled hitlag pair) and u.kb.xD0
/// cleared. Kirby's damage callbacks stay.
pub fn article_destroyed(f: &mut Fighter, kind: ItemKind) {
    if kind == ItemKind::KirbyPeachToad {
        state(f).toad_out = false;
    }
}

/// ftKb_SpecialNPe_8010C3F4 (8010C3F4), the Peach arm of 800F19AC and
/// 800F1A8C: Toad, if out, is destroyed (it_802BDF40: 8010C3C0, then
/// Item_8026A8EC inside Kirby's proc).
pub fn put_away(f: &mut Fighter) {
    if !state(f).toad_out {
        return;
    }
    let owner = f.player.id;
    f.core.item_requests.push(ItemRequest::Control {
        owner,
        kind: ItemKind::KirbyPeachToad,
        control: ItemControl::Remove,
    });
    article_destroyed(f, ItemKind::KirbyPeachToad);
    // Item_8026A8EC's efLib_DestroyAll leaves the generator insertion
    // cursor on the list's tail (as Peach's own removal of Toad).
    f.core
        .effects
        .push(melee_ef::request::EffectRequest::ArticleDestroyed);
}
