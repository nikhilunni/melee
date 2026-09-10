//! Shared ftFx_ states. Character differences are constants and typed accessors.
pub mod special_n;
pub mod special_s;

use melee_ft::{desc::fox_attributes::FoxAttributes, fighter::CharacterCallbacks};
use melee_types::ItemKind;

#[derive(Clone, Copy, Debug)]
pub struct FamilySounds {
    pub fire: [u32; 2],
    pub holster: u32,
    pub throw_fire: u32,
}

/// Fighter mv.fx.SpecialN and u.fx.blasterGObj, owned by each character.
#[derive(Clone, Debug, Default)]
pub struct SpecialNeutral {
    pub repeat: bool,
    pub blaster_present: bool,
    pub accessory_shot: bool,
}

pub trait FoxFamily: CharacterCallbacks {
    const LASER: ItemKind;
    const BLASTER: ItemKind;
    const GHOST: ItemKind;
    const GHOST_ARTICLE_INDEX: u32;
    const SOUNDS: FamilySounds;
    fn attributes(&self) -> &FoxAttributes;
    fn special_neutral(&mut self) -> &mut SpecialNeutral;
    fn special_side(&mut self) -> &mut special_s::SpecialSide;
}

#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FamilyState {
    SpecialNStart = 341,
    SpecialNLoop,
    SpecialNEnd,
    SpecialAirNStart,
    SpecialAirNLoop,
    SpecialAirNEnd,
    SpecialSStart,
    SpecialS,
    SpecialSEnd,
    SpecialAirSStart,
    SpecialAirS,
    SpecialAirSEnd,
}
impl From<FamilyState> for melee_ft::fighter::ActionId {
    fn from(value: FamilyState) -> Self {
        Self(value as u16)
    }
}
impl FamilyState {
    pub const COUNT: usize = 12;
}

pub const fn rows<C: FoxFamily>() -> [melee_ft::fighter::MotionRow; FamilyState::COUNT] {
    let neutral = special_n::rows::<C>();
    let side = special_s::rows::<C>();
    [
        neutral[0], neutral[1], neutral[2], neutral[3], neutral[4], neutral[5], side[0], side[1],
        side[2], side[3], side[4], side[5],
    ]
}

pub fn enter_special<C: FoxFamily>(
    f: &mut melee_ft::fighter::Fighter,
    slot: melee_ft::fighter::SpecialSlot,
    airborne: bool,
    assets: &melee_ft::fighter::assets::FighterAssets,
) {
    match slot {
        melee_ft::fighter::SpecialSlot::Neutral => {
            special_n::enter_special::<C>(f, slot, airborne, assets)
        }
        melee_ft::fighter::SpecialSlot::Side => special_s::enter::<C>(f, airborne, assets),
        _ => unimplemented!("Fox family {slot:?}"),
    }
}
