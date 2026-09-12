//! Requests at the fighter/effect boundary; particle lifetimes belong to ef.
use hsd_types::Mtx;
// ftCo_09F7.c:75-97: special part selectors bypass the common part table.
const ROTATING_EFFECT_BONE: usize = 0x8D;
const TRANSLATION_EFFECT_BONE: usize = 0x8E;
use melee_ef::request::{EffectOwner, EffectQueue, EffectRequest};

impl EffectOwner for super::FighterCore {
    fn effect_queue(&mut self) -> &mut EffectQueue {
        &mut self.effects
    }
    fn effect_matrix(&mut self, bone: Option<usize>) -> Mtx {
        let joint = bone.map_or(self.animation.root, |bone| self.animation.parts[bone].joint);
        self.skeleton.setup_matrix(joint);
        self.skeleton.get(joint).mtx
    }
    fn effect_scale(&self) -> hsd_types::Vec3 {
        self.skeleton.scale(self.animation.root)
    }
    fn effect_facing(&self) -> f32 {
        self.physics.facing
    }
}

impl super::FighterCore {
    /// Fighter_ChangeMotionState (800693AC), fighter.c:950-951: flush
    /// efAsync against the outgoing pose before motion resources are replaced.
    pub(super) fn flush_effects_on_motion_change(&mut self) {
        self.skeleton
            .set_translate(self.animation.root, &self.physics.position);
        let mut queue = std::mem::take(&mut self.effects);
        queue.resolve_pending(|bone| self.effect_matrix(bone));
        self.effects = queue;
    }
}

/// Fighter flags x2219_b0 and x2220_b0 (three-bit rotating index).
#[derive(Clone, Debug, Default)]
pub struct FighterEffects {
    pub destroy_on_state_change: bool,
    /// Installed pre/post-hitlag efLib pause callbacks, cleared on motion entry.
    pub hitlag_callbacks: bool,
    pub rotating_bone_index: u8,
    pub invisible: bool,
}
impl super::FighterCore {
    /// ftCo_8009F834 (0x8009F834). Called at the command-owning proc boundary,
    /// before another fighter or stage can draw from the shared RNG.
    pub fn resolve_graphics_commands(
        &mut self,
        assets: &super::assets::FighterAssets,
        rng: &mut gekko_math::HsdRng,
    ) -> usize {
        let mut draws = 0;
        let mut graphics = std::mem::take(&mut self.commands.graphics);
        while !graphics.is_empty() {
            let command = graphics.remove(0);
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
            if matches!(id, 0x423 | 0x424) {
                let normal = self.collision.data.floor.normal;
                let floor_angle = if self.physics.ground_or_air == melee_types::GroundOrAir::Ground
                {
                    melee_lb::trigf::atan2f(-normal.x, normal.y)
                } else {
                    0.0
                };
                self.effects.push(EffectRequest::Graphics {
                    id,
                    bone,
                    offset: hsd_types::Vec3::ZERO,
                    facing: self.physics.facing,
                    floor_angle,
                });
                continue;
            }
            if id == 0x429 {
                // ftCo_09F7.c:142-150: dizzy stars use character effect scale.
                self.effects.push(EffectRequest::DizzyStars {
                    bone,
                    scale: self.attributes.size.unknown_168,
                });
                continue;
            }
            if matches!(id, 0x402 | 0x403 | 0x412 | 0x413 | 0x414) {
                // ftCo_09F7.c:115-133: kind 0, before randomized branches.
                self.effects.push(EffectRequest::Attached { id, bone });
                continue;
            }
            if !(id < 0x250
                || id / 1000 == 30
                || matches!(
                    id,
                    0x404
                        | 0x3E9
                        | 0x3F1
                        | 0x3F2
                        | 0x3FA
                        | 0x3FB
                        | 0x3FD
                        | 0x3F8
                        | 0x3F9
                        | 0x406
                        | 0x513
                        | 0x514
                        | 0x515
                        | 0x3F3
                        | 0x3F7
                        | 0x407
                        | 0x3FE
                        | 0x3FF
                        | 0x400
                        | 0x401
                        | 0x402
                ))
            {
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
                // 3F3/3F7/407/3FE/3FF/400/401 use block_70: 8009FCF8/FD1C/FD44 fmadds.
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
        self.effects.finish_graphics();
        self.commands.graphics = graphics;
        draws
    }
}
