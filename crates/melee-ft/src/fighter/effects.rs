//! Requests at the fighter/effect boundary; particle lifetimes belong to ef.
use hsd_types::Vec3;
// ftCo_09F7.c:75-97: special part selectors bypass the common part table.
const ROTATING_EFFECT_BONE: usize = 0x8D;
const TRANSLATION_EFFECT_BONE: usize = 0x8E;
#[derive(Clone, Debug, PartialEq)]
pub enum EffectRequest {
    /// ftCommon_8007DB24 -> efLib_DestroyAll: remove this fighter's owned effects.
    DestroyOwned,
    /// efAsync kinds 2/5/6 retain the bone and local offset until s_link 9.
    Graphics {
        id: u16,
        bone: usize,
        offset: Vec3,
        facing: f32,
        floor_angle: f32,
    },
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

/// ftAction_80071028's five command words, decoded at the archive boundary.
#[derive(Clone, Debug)]
pub struct GraphicsCommand {
    pub bone: usize,
    pub common_bone: bool,
    pub item_bone: bool,
    pub destroy_on_state_change: bool,
    pub id: u16,
    pub parameter: f32,
    pub offset: Vec3,
    pub range: Vec3,
}
/// Fighter flags x2219_b0 and x2220_b0 (three-bit rotating index).
#[derive(Clone, Debug, Default)]
pub struct FighterEffects {
    pub destroy_on_state_change: bool,
    pub rotating_bone_index: u8,
    pub invisible: bool,
}
impl<C: super::CharacterCallbacks> super::Fighter<C> {
    /// ftCo_8009F834 (0x8009F834). Called at the command-owning proc boundary,
    /// before another fighter or stage can draw from the shared RNG.
    pub fn resolve_graphics_commands(
        &mut self,
        assets: &super::assets::FighterAssets,
        rng: &mut gekko_math::HsdRng,
    ) -> usize {
        let mut draws = 0;
        for command in std::mem::take(&mut self.commands.graphics) {
            if self.effect_state.invisible {
                continue;
            }
            self.effect_state.destroy_on_state_change |= command.destroy_on_state_change;
            let bone = if command.item_bone {
                usize::from(self.bones.model.held_item)
            } else {
                command.bone
            };
            let bone = match bone {
                ROTATING_EFFECT_BONE => {
                    let bone = assets.rotating_effect_bones
                        [usize::from(self.effect_state.rotating_bone_index)];
                    self.effect_state.rotating_bone_index =
                        (self.effect_state.rotating_bone_index + 1) % 5;
                    bone
                }
                TRANSLATION_EFFECT_BONE => usize::from(self.bones.model.animation_translation),
                _ if command.common_bone => usize::from(
                    assets.parts.part_to_joint[bone].expect("effect common bone missing"),
                ),
                _ => bone,
            };
            let id = command.id;
            if !(id < 0x250 || id / 1000 == 30 || matches!(id, 0x3FE | 0x3FF | 0x401)) {
                unimplemented!("ftCo_09F7.c:115-311: graphics dispatch {id:#x}");
            }
            let mut offset = command.offset;
            for (value, range) in [
                (&mut offset.x, command.range.x),
                (&mut offset.y, command.range.y),
                (&mut offset.z, command.range.z),
            ] {
                let random = rng.randf();
                // Early branch: retail 8009F94C/F970/F9A4 fmadds.
                // 3FE/3FF/401 use block_70: 8009FCF8/FD1C/FD44 fmadds.
                // The range doubling and random subtraction round separately.
                *value = gekko_math::fma::fmadds(2.0 * range, random - 0.5, *value);
                draws += 1;
            }
            let normal = self.collision.data.floor.normal;
            let floor_angle = if self.physics.ground_or_air == melee_types::GroundOrAir::Ground {
                melee_lb::trigf::atan2f(-normal.x, normal.y)
            } else {
                0.0
            };
            self.effects.push(EffectRequest::Graphics {
                id,
                bone,
                offset,
                facing: self.physics.facing,
                floor_angle,
            });
        }
        draws
    }
}
