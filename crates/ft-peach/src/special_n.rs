//! Toad, ftpeachspecialn.c (8011E174..8011ED38): the states live in
//! `ft_peach_family::toad`, shared with Kirby's copy; this is Peach's side
//! of them (her rows, attributes, article kinds and callbacks).
use crate::init::{Accessory, Peach};
use ft_peach_family::toad::{
    self, CounterVolume, SpecialN, ToadAccessory, ToadAttributes, ToadUser,
};
use melee_ft::fighter::{assets::FighterAssets, Fighter, MotionRow};
use melee_types::ItemKind;

/// fp->parts[109]: the joint Toad and the parasol hang from.
pub const ARTICLE_JOINT: usize = 109;

impl ToadUser for Peach {
    /// ftPe_MS_SpecialN..SpecialAirNHit (365..368).
    const FIRST_ROW: u16 = 365;
    const ARTICLE_JOINT: usize = ARTICLE_JOINT;
    const TOAD: ItemKind = ItemKind::PeachToad;
    const SPORE: ItemKind = ItemKind::PeachToadSpore;
    /// ftPe_DatAttrs +94..+A8.
    fn attributes(f: &Fighter) -> &ToadAttributes {
        &f.character.get::<Peach>().attributes.toad
    }
    /// ftPe_DatAttrs +AC.
    fn volume(f: &Fighter) -> &CounterVolume {
        &f.character.get::<Peach>().attributes.toad_volume
    }
    fn scratch(f: &mut Fighter) -> &mut SpecialN {
        &mut f.character.get_mut::<Peach>().special_n
    }
    /// Fighter +2240.
    fn lift_used(f: &mut Fighter) -> &mut bool {
        &mut f.character.get_mut::<Peach>().aerial_toad_used
    }
    /// Fighter +2244.
    fn toad_out(f: &Fighter) -> bool {
        f.character.get::<Peach>().items.toad
    }
    /// onAccessory4: u.pe.toad_gobj, x1984_heldItemSpec, and death2_cb /
    /// take_dmg_cb = ftPe_Init_OnDeath2.
    fn hold_toad(f: &mut Fighter) {
        let items = &mut f.character.get_mut::<Peach>().items;
        items.toad = true;
        items.death2_armed = true;
        items.take_damage_armed = true;
    }
    fn arm_damage_callbacks(f: &mut Fighter) {
        let items = &mut f.character.get_mut::<Peach>().items;
        items.death2_armed = true;
        items.take_damage_armed = true;
    }
    fn install_accessory(f: &mut Fighter, accessory: ToadAccessory) {
        f.character.get_mut::<Peach>().accessory = match accessory {
            ToadAccessory::DrawToad => Accessory::DrawToad,
            ToadAccessory::ReleaseSpore => Accessory::ReleaseSpore,
        };
    }
    fn keep_scratch_word(f: &mut Fighter, word: Option<f32>) {
        f.character.get_mut::<Peach>().retained_word = word;
    }
}

/// ftPe_SpecialN_Enter (8011E3C8) / ftPe_SpecialAirN_Enter (8011E45C).
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    toad::enter::<Peach>(f, air, a);
}

/// Rows 365..368, ftPe_SM 312..315.
pub const fn rows() -> [MotionRow; 4] {
    toad::rows::<Peach>(312)
}

/// onAccessory4 (8011E174).
pub fn draw_toad(f: &mut Fighter) {
    toad::draw_toad::<Peach>(f);
}

/// onHitAccessory4 (8011E230).
pub fn release_spore(f: &mut Fighter) {
    toad::release_spore::<Peach>(f);
}

/// onUnkHit (8011EC6C).
pub fn process_hit(f: &mut Fighter, a: &FighterAssets) {
    toad::process_hit::<Peach>(f, a);
}

/// Toad's volume against an item's hit.
pub fn item_contact(
    f: &mut Fighter,
    item: &mut melee_it::ItemCore,
    id: usize,
    assets: &FighterAssets,
) -> bool {
    toad::item_contact::<Peach>(f, item, id, assets)
}

/// Toad's volume against a fighter's hit.
pub fn contact(f: &mut Fighter, attacker: &mut Fighter, a: &FighterAssets, id: usize) -> bool {
    toad::contact::<Peach>(f, attacker, a, id)
}
