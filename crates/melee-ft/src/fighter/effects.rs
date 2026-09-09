//! Requests at the fighter/effect boundary; particle lifetimes belong to ef.
use hsd_types::{Mtx, Vec3};
// ftCo_09F7.c:75-97: special part selectors bypass the common part table.
const ROTATING_EFFECT_BONE: usize = 0x8D;
const TRANSLATION_EFFECT_BONE: usize = 0x8E;
#[derive(Clone, Debug, PartialEq)]
pub enum EffectRequest {
    /// ftYs_Init_8012BE3C, efSync_Spawn 0x4CF: positional shell burst.
    EggShell {
        bone: usize,
        scale: f32,
    },
    Death {
        position: Vec3,
        scale: f32,
    },
    /// fn_800DA1D8: async kind 1, hold-bone position sampled at queue flush.
    CaptureFlash {
        bone: usize,
    },
    /// Fighter_ChangeMotionState flushes the queue using the outgoing pose.
    /// The scene consumes this batch at the owning proc boundary, in order.
    FlushDeferred(Vec<ResolvedEffect>),
    /// efSync_Spawn: shield model attached to the shield joint.
    Shield {
        id: u16,
        bone: usize,
    },
    /// ftColl_8007A06C -> efSync_Spawn: world-space contact effect.
    ShieldSpark {
        position: Vec3,
    },
    HitSpark {
        position: Vec3,
        element: melee_types::HitElement,
        damage: f32,
    },
    /// ftCommon_8007DB24 -> efLib_DestroyAll: remove this fighter's owned effects.
    DestroyOwned,
    /// ftCliffCommon_80081370: async kind 2 with no bone, absolute position.
    LedgeGrab {
        position: Vec3,
    },
    /// efAsync kind 0 passes the live fighter joint without offset RNG.
    Attached {
        id: u16,
        bone: usize,
    },
    /// efAsync kinds 2/5/6 retain the bone and local offset until s_link 9.
    Graphics {
        id: u16,
        bone: usize,
        offset: Vec3,
        facing: f32,
        floor_angle: f32,
    },
    /// efAsync_Spawn(..., 3, 0x43E, root, scale), ft_0C31.c:130.
    EntryWarp {
        id: u16,
        scale: Vec3,
    },
    /// ftAction_80072E4C / ftCo_8009F834: root-relative landing dust.
    Landing {
        id: u16,
        offset: Vec3,
        floor_angle: f32,
    },
}
/// A deferred request whose joint transform was sampled at its retail flush.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedEffect {
    pub request: EffectRequest,
    pub matrix: Mtx,
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
    /// Fighter_ChangeMotionState (800693AC), fighter.c:950-951:
    /// translate the root, then efAsync_QueueFlush before replacing the pose.
    pub(super) fn flush_effects_on_motion_change(&mut self) {
        self.skeleton
            .set_translate(self.animation.root, &self.physics.position);
        let mut pending = Vec::new();
        let mut immediate = Vec::new();
        for request in std::mem::take(&mut self.effects) {
            if request.is_immediate() {
                immediate.push(request);
            } else {
                pending.push(request);
            }
        }
        let mut resolved = Vec::new();
        for request in pending.into_iter().rev() {
            let joint = match &request {
                EffectRequest::EggShell { bone, .. }
                | EffectRequest::CaptureFlash { bone }
                | EffectRequest::Attached { bone, .. }
                | EffectRequest::Graphics { bone, .. } => self.animation.parts[*bone].joint,
                _ => self.animation.root,
            };
            self.skeleton.setup_matrix(joint);
            resolved.push(ResolvedEffect {
                request,
                matrix: self.skeleton.get(joint).mtx,
            });
        }
        if !resolved.is_empty() {
            immediate.push(EffectRequest::FlushDeferred(resolved));
        }
        self.effects = immediate;
    }
    /// efSync_Spawn / efLib_DestroyAll: dispatch at the owning fighter callback.
    /// Deferred efAsync requests retain their original order until link 9.
    pub fn drain_immediate_effects(&mut self, sink: &mut impl EffectSink) {
        let mut deferred = Vec::new();
        for effect in self.effects.drain(..) {
            if effect.is_immediate() {
                sink.spawn_effect(effect);
            } else {
                deferred.push(effect);
            }
        }
        self.effects = deferred;
    }
    /// Forward requests in call order; rendering/particle code supplies the sink.
    pub fn drain_effects(&mut self, sink: &mut impl EffectSink) {
        for effect in self.effects.drain(..) {
            sink.spawn_effect(effect);
        }
    }
}
impl EffectRequest {
    fn is_immediate(&self) -> bool {
        matches!(
            self,
            Self::Death { .. }
                | Self::Shield { .. }
                | Self::HitSpark { .. }
                | Self::ShieldSpark { .. }
                | Self::DestroyOwned
                | Self::FlushDeferred(_)
        )
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
            if matches!(id, 0x402 | 0x403) {
                // ftCo_09F7.c:115-133: kind 0, before randomized branches.
                self.effects.push(EffectRequest::Attached { id, bone });
                continue;
            }
            if !(id < 0x250
                || id / 1000 == 30
                || matches!(
                    id,
                    0x3F8
                        | 0x406
                        | 0x514
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
        draws
    }
}
