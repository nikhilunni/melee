//! Shared ftFx_ states. Character differences are constants and typed accessors.
pub mod special_n;

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
    const SOUNDS: FamilySounds;
    fn attributes(&self) -> &FoxAttributes;
    fn special_neutral(&mut self) -> &mut SpecialNeutral;
}

pub use special_n::{enter_special, rows, FamilyState};
