//! Owned fighter data read from HSD archives. Layout citations use paths under
//! `third_party/melee-decomp/src/melee`. No floating-point arithmetic occurs here.

mod animation;
pub mod attributes;
pub mod bones;
pub mod common;
pub mod fox_attributes;
pub mod playback;
mod read;

pub use animation::*;
pub use attributes::{read_fighter_attributes, FighterAttributes};
pub use bones::{
    graft_conditional_joint, read_conditional_parts, read_fighter_bones, read_part_table,
    ConditionalPart, EcbBones, FighterBones, PartTable,
};
pub use read::{special_attributes_offset, FighterDescError};
