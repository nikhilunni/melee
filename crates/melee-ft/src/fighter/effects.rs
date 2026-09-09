//! Requests at the fighter/effect boundary; particle lifetimes belong to ef.
use hsd_types::Vec3;
#[derive(Clone, Debug, PartialEq)]
pub enum EffectRequest {
    /// efAsync_Spawn(..., 3, 0x43E, root, scale), ft_0C31.c:130.
    EntryWarp { id: u16, scale: Vec3 },
    /// ftAction_80072E4C / ftCo_8009F834: root-relative landing dust.
    Landing {
        id: u16,
        offset: Vec3,
        floor_angle: f32,
    },
}
/// Scene implementations drain Fighter.effects through this interface.
pub trait EffectSink {
    fn spawn_effect(&mut self, request: EffectRequest);
}
impl EffectSink for Vec<EffectRequest> {
    fn spawn_effect(&mut self, request: EffectRequest) {
        self.push(request);
    }
}

impl<C: super::CharacterCallbacks> super::Fighter<C> {
    /// Forward requests in call order; rendering/particle code supplies the sink.
    pub fn drain_effects(&mut self, sink: &mut impl EffectSink) {
        for effect in self.effects.drain(..) {
            sink.spawn_effect(effect);
        }
    }
}
