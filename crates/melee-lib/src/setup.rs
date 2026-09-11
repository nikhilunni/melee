//! Validated setup shared by the public API and the oracle adapter.
use melee_ft::fighter::assets::CharacterDescriptor;
#[derive(Clone)]
pub struct PlayerSetup {
    pub slot: u8,
    pub descriptor: &'static CharacterDescriptor,
    pub costume: u8,
    pub spawn_point: i8,
    pub stocks: u8,
}
impl PlayerSetup {
    pub fn descriptor(&self) -> &'static CharacterDescriptor {
        self.descriptor
    }
}
#[derive(Clone)]
pub struct Setup {
    pub fighters: [PlayerSetup; 2],
    pub stage: &'static crate::scene_stage::StageDescriptor,
    pub seed: Option<u32>,
    pub all_characters_unlocked: Option<bool>,
}
impl Setup {
    pub fn stage_descriptor(&self) -> &'static crate::scene_stage::StageDescriptor {
        self.stage
    }
}
