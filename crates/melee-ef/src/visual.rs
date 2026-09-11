//! Borrowed model views for optional presentation; no mutation or playback.
use super::*;
pub const VISUAL_CAPACITY: usize = INSTANCE_CAPACITY;
#[derive(Clone, Copy)]
pub struct VisualModel<'a> {
    pub descriptor: u32,
    pub bank: u8,
    pub definition: &'a desc::effect_visual::EffectVisual,
    pub tree: &'a JObjTree,
}
impl Effect {
    fn visual_model(&self) -> VisualModel<'_> {
        VisualModel {
            descriptor: self.descriptor,
            bank: self.bank,
            definition: &self.visual,
            tree: &self.tree,
        }
    }
}
impl Resources {
    pub fn visual_models(&self) -> impl Iterator<Item = VisualModel<'_>> {
        self.models.iter().map(|model| model.visual_model())
    }
}
impl Effects {
    pub fn visual_models(&self) -> impl Iterator<Item = VisualModel<'_>> {
        self.instances.iter().map(Effect::visual_model)
    }
}
