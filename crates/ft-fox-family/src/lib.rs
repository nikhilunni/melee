//! Shared ftFx_ states. Character differences are constants and typed accessors.
pub mod special_hi;
pub mod special_lw;
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
    fn special_lw(&mut self) -> &mut special_lw::SpecialLw;
    fn special_hi(&mut self) -> &mut special_hi::SpecialHi;
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
    SpecialHiHold,
    SpecialHiHoldAir,
    SpecialHi,
    SpecialAirHi,
    SpecialHiLanding,
    SpecialHiFall,
    SpecialHiBound,
    SpecialLwStart,
    SpecialLwLoop,
    SpecialLwHit,
    SpecialLwEnd,
    SpecialLwTurn,
    SpecialAirLwStart,
    SpecialAirLwLoop,
    SpecialAirLwHit,
    SpecialAirLwEnd,
    SpecialAirLwTurn,
}
impl From<FamilyState> for melee_ft::fighter::ActionId {
    fn from(value: FamilyState) -> Self {
        Self(value as u16)
    }
}
impl FamilyState {
    pub const COUNT: usize = 29;
}

pub const fn rows<C: FoxFamily>() -> [melee_ft::fighter::MotionRow; FamilyState::COUNT] {
    let neutral = special_n::rows::<C>();
    let side = special_s::rows::<C>();
    let hi = special_hi::rows::<C>();
    let lw = special_lw::rows::<C>();
    let mut bound = melee_ft::fighter::state::unimplemented_row();
    bound.action = melee_ft::fighter::ActionId(FamilyState::SpecialHiBound as u16);
    [
        neutral[0], neutral[1], neutral[2], neutral[3], neutral[4], neutral[5], side[0], side[1],
        side[2], side[3], side[4], side[5], hi[0], hi[1], hi[2], hi[3], hi[4], hi[5], bound, lw[0],
        lw[1], lw[2], lw[3], lw[4], lw[5], lw[6], lw[7], lw[8], lw[9],
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
        melee_ft::fighter::SpecialSlot::Up => special_hi::enter::<C>(f, airborne, assets),
        melee_ft::fighter::SpecialSlot::Down => special_lw::enter::<C>(f, airborne, assets),
    }
}

/// ftFox_Init_MotionStateTable's FtMoveId values, shared with Falco.
pub const fn special_moves() -> [Option<melee_types::combat::StaleMove>; 29] {
    use melee_types::combat::StaleMove as M;
    let mut moves = [None; 29];
    let mut i = 0;
    while i < 29 {
        moves[i] = Some(if i < 6 {
            M::SpecialNeutral
        } else if i < 12 {
            M::SpecialSide
        } else if i < 19 {
            M::SpecialUp
        } else {
            M::SpecialDown
        });
        i += 1;
    }
    moves
}
