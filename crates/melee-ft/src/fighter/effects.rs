//! Requests at the fighter/effect boundary; particle lifetimes belong to ef.
use hsd_types::Mtx;
// ftCo_09F7.c:75-97: special part selectors bypass the common part table.
const ROTATING_EFFECT_BONE: usize = 0x8D;
const TRANSLATION_EFFECT_BONE: usize = 0x8E;
use melee_ef::request::{EffectOwner, EffectQueue, EffectRequest};

/// fp->parts[1], the joint efSync 0x500 follows (Zelda's sparkle).
const SPARKLE_PART: usize = 1;
/// An animlist id ftCo_8009F834 has no row for (block_70's default: "no
/// effect from animlist"); Zelda's scripts name it.
const NO_EFFECT: u16 = 0x43A;

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
    /// Installed pre/post-hitlag callbacks that freeze the fighter's article
    /// of this kind (it_8026B724 / it_8026B73C: Mario's cape), cleared on
    /// motion entry like the efLib pair.
    pub article_hitlag: Option<melee_types::ItemKind>,
    pub rotating_bone_index: u8,
    pub invisible: bool,
}
impl super::FighterCore {
    /// efAsync_Spawn from an entry callback, after its motion change: the
    /// graphics and landing effects already issued (the new script's frame-0
    /// commands, ftCo_800C0408's step in Fighter_ChangeMotionState) were
    /// queued first in retail; those issued later in the proc come after.
    pub fn push_effect_after_issued_graphics(&mut self, request: EffectRequest) {
        let issued = self.commands.graphics.len() + self.commands.landing_effects.len();
        self.effects.push_after_graphics(request, issued);
    }
    /// Install a one-shot accessory4 after this entry's motion change.
    pub fn arm_accessory4(&mut self) {
        self.accessory4_armed = true;
    }
    /// A one-shot accessory4 call: the character's pending action runs only
    /// while its accessory is still installed, and uninstalls it
    /// (accessory4_cb = NULL). A pending action whose accessory a motion
    /// change removed is dropped; with nothing pending the accessory stays.
    /// Not generic, so character crates share one definition.
    pub fn run_accessory4(&mut self, pending: bool) -> bool {
        pending && std::mem::take(&mut self.accessory4_armed)
    }
    /// ftCo_8009F834 (0x8009F834). Called at the command-owning proc boundary,
    /// before another fighter or stage can draw from the shared RNG.
    pub fn resolve_graphics_commands(
        &mut self,
        assets: &super::assets::FighterAssets,
        rng: &mut gekko_math::HsdRng,
    ) -> usize {
        let mut draws = 0;
        let mut index = 0;
        // Consumed in place: nothing below queues graphics, and moving the
        // fixed-capacity queue out and back would copy all of its slots.
        while !self.commands.graphics.is_empty() {
            draws += self.resolve_landing_effects(rng, index);
            index += 1;
            let command = self.commands.graphics.remove(0);
            if self.effect_state.invisible {
                self.effects.skip_graphics();
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
            // ftCo_09F7.c block_67: efAsync kind 3 with the floor angle.
            if matches!(id, 0x423 | 0x424 | 0x4C1 | 0x4C2 | 0x4C4 | 0x4C5) {
                let normal = self.collision.data.floor.normal;
                let floor_angle = if self.physics.ground_or_air == melee_types::GroundOrAir::Ground
                {
                    melee_lb::trigf::atan2f(-normal.x, normal.y)
                } else {
                    0.0
                };
                self.effects.push_graphics(EffectRequest::Graphics {
                    id,
                    bone,
                    offset: hsd_types::Vec3::ZERO,
                    facing: command.issued_facing.unwrap_or(self.physics.facing),
                    floor_angle,
                });
                continue;
            }
            if id == 0x429 {
                // ftCo_09F7.c:142-150: dizzy stars use character effect scale.
                self.effects.push_graphics(EffectRequest::DizzyStars {
                    bone,
                    scale: self.attributes.size.unknown_168,
                });
                continue;
            }
            if id == 0x446 {
                // ftCo_09F7.c:136-142: efAsync kind 7 with the command's
                // offset as given, before the randomized branches.
                self.effects
                    .push_graphics(EffectRequest::FollowingGenerator {
                        id,
                        bone,
                        offset: command.offset,
                    });
                continue;
            }
            if matches!(
                id,
                0x402
                    | 0x403
                    | 0x412
                    | 0x413
                    | 0x414
                    | 0x422
                    | 0x487
                    | 0x4D1
                    | 0x4FE
                    | 0x4FF
                    | 0x500
                    | 0x501
                    | 0x502
            ) {
                // ftCo_09F7.c:115-133: kind 0, before randomized branches.
                // efsync.c:500-521: 0x500 and 0x501 ignore the joint they
                // are given and follow parts[1].
                let bone = if matches!(id, 0x4FF..=0x501) {
                    SPARKLE_PART
                } else {
                    bone
                };
                self.effects
                    .push_graphics(EffectRequest::Attached { id, bone });
                continue;
            }
            if !(id < 0x250
                || id / 1000 == 30
                || matches!(
                    id,
                    0x404
                        | 0x40C
                        | 0x3E9
                        | 0x416
                        | 0x3EF
                        | 0x3F0
                        | 0x405
                        | 0x41C
                        | 0x41D
                        | 0x40E
                        | 0x40F
                        | 0x411
                        | 0x3F1
                        | 0x3F2
                        | 0x3F5
                        | 0x3FA
                        | 0x3FB
                        | 0x3FC
                        | 0x3FD
                        | 0x3F6
                        | 0x3F8
                        | 0x3F9
                        | 0x406
                        | 0x513
                        | 0x514
                        | 0x515
                        | 0x3F3
                        | 0x3F4
                        | 0x3F7
                        | 0x407
                        | 0x3FE
                        | 0x3FF
                        | 0x400
                        | 0x401
                        | 0x402
                        | 0x40D
                        | NO_EFFECT
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
                // 3F3/3F4/3F7/407/3FE/3FF/400/401 use block_70: 8009FCF8/FD1C/FD44 fmadds.
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
            if id == NO_EFFECT {
                // ftCo_09F7.c:306-309: block_70's default, an OSReport
                // after the three draws.
                continue;
            }
            self.effects.push_graphics(EffectRequest::Graphics {
                id,
                bone,
                offset,
                facing: command.issued_facing.unwrap_or(self.physics.facing),
                floor_angle,
            });
        }
        draws += self.resolve_landing_effects(rng, usize::MAX);
        self.effects.finish_graphics();
        draws
    }
}
