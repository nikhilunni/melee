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
    fn effect_orientation_target(
        &mut self,
        bone: usize,
    ) -> hsd_anim::orientation::OrientationTarget {
        self.skeleton
            .orientation_target(self.animation.parts[bone].joint)
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
        let sealed = queue.resolve_pending(|bone| self.effect_matrix(bone));
        self.effects = queue;
        if sealed != 0 {
            self.commands.mark_effect_seal();
        }
        // The graphics commands issued so far queued their effects in
        // retail, so this flush takes them too.
        self.commands.graphics_before_seal = self.commands.graphics.len();
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
    /// Set while a proc at or past link 9 runs: efAsync_Spawn (0x800679B0)
    /// creates effects at once there instead of queuing them for link 9.
    pub dispatch_at_once: bool,
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
        self.commands.graphics_before_seal = 0;
        let draws = self.resolve_graphics_while(assets, rng, false);
        self.commands.graphics_resolved = 0;
        draws
    }
    /// The graphics commands issued before this proc's last motion change
    /// (below link 9): Fighter_ChangeMotionState's efAsync_QueueFlush
    /// (fighter.c:951, retail 0x800693AC..) created their effects with the
    /// requests pending then, newest first. Each draws its offsets now and
    /// joins the sealed requests where that flush had it. Returns whether
    /// any was resolved: the caller dispatches the sealed requests next,
    /// before the new script's commands draw.
    pub fn resolve_graphics_commands_before_seal(
        &mut self,
        assets: &super::assets::FighterAssets,
        rng: &mut gekko_math::HsdRng,
    ) -> bool {
        if self.commands.graphics_before_seal == 0 {
            return false;
        }
        self.resolve_graphics_while(assets, rng, true);
        true
    }
    /// One pass of ftCo_8009F834 over the queued commands: all of them, or
    /// (`sealed`) those a motion change's flush took.
    fn resolve_graphics_while(
        &mut self,
        assets: &super::assets::FighterAssets,
        rng: &mut gekko_math::HsdRng,
        sealed: bool,
    ) -> usize {
        let mut draws = 0;
        let mut index = self.commands.graphics_resolved;
        // Consumed in place: nothing below queues graphics, and moving the
        // fixed-capacity queue out and back would copy all of its slots.
        while !self.commands.graphics.is_empty() {
            if sealed {
                if self.commands.graphics_before_seal == 0 {
                    break;
                }
                self.commands.graphics_before_seal -= 1;
                self.commands.graphics_resolved += 1;
            }
            draws += self.resolve_landing_effects(rng, index);
            index += 1;
            self.resolve_sound_draws(assets, rng);
            let command = self.commands.graphics.remove(0);
            self.advance_sound_draws();
            if self.effect_state.invisible {
                self.effects.skip_graphics();
                continue;
            }
            self.effect_state.destroy_on_state_change |= command.destroy_on_state_change;
            let mut rotating = self.effect_state.rotating_bone_index;
            let bone = self.graphics_bone(&command, assets, &mut rotating);
            self.effect_state.rotating_bone_index = rotating;
            // The joint as the command found it, when it ran past link 9.
            let issued = command.issued_joint.filter(|issued| issued.bone == bone);
            let id = command.id;
            // ftCo_09F7.c block_67: efAsync kind 3 with the floor angle.
            if matches!(id, 0x423 | 0x424 | 0x4C1 | 0x4C2 | 0x4C4 | 0x4C5) {
                let normal = self.collision.data.floor.normal;
                let floor_angle = if self.issued_on_ground(&command) {
                    melee_lb::trigf::atan2f(-normal.x, normal.y)
                } else {
                    0.0
                };
                self.push_graphics_request_with(
                    sealed,
                    issued,
                    EffectRequest::Graphics {
                        id,
                        bone,
                        offset: hsd_types::Vec3::ZERO,
                        facing: command.issued_facing.unwrap_or(self.physics.facing),
                        floor_angle,
                    },
                );
                continue;
            }
            if matches!(id, 0x428..=0x42A) {
                // ftCo_09F7.c:142-150: efAsync kind 3 with the character's
                // effect scale (ftData x0 +168): the dizzy stars (0x429,
                // generator 0xCE), the heal's sparkle (0x428, 0xCA) and
                // 0x42A (0xCF).
                self.push_graphics_request_with(
                    sealed,
                    issued,
                    EffectRequest::DizzyStars {
                        generator: match id {
                            0x428 => 0xCA,
                            0x429 => 0xCE,
                            _ => 0xCF,
                        },
                        bone,
                        scale: self.attributes.size.unknown_168,
                    },
                );
                continue;
            }
            if id == 0x49D {
                // ftCo_09F7.c block_65: efAsync kind 6 with the command's
                // offset as given, the facing, and the X range as an angle
                // in 256ths of a degree (two fmuls; Kirby's rapid jab).
                let angle = 0.017453292 * (256.0 * command.range.x);
                self.push_graphics_request_with(
                    false,
                    issued,
                    EffectRequest::Graphics {
                        id,
                        bone,
                        offset: command.offset,
                        facing: command.issued_facing.unwrap_or(self.physics.facing),
                        floor_angle: angle,
                    },
                );
                continue;
            }
            if id == 0x495 {
                // ftCo_09F7.c block_64: efAsync kind 3 with the facing
                // (Kirby's dash attack flame, efAlt 0x495).
                self.push_graphics_request_with(
                    false,
                    issued,
                    EffectRequest::AttachedParameter {
                        id,
                        bone,
                        parameter: self.physics.facing,
                    },
                );
                continue;
            }
            if matches!(id, 0x446 | 0x4E1) {
                // ftCo_09F7.c:136-142: efAsync kind 7 with the command's
                // offset as given, before the randomized branches.
                self.push_graphics_request_with(
                    sealed,
                    issued,
                    EffectRequest::FollowingGenerator {
                        id,
                        bone,
                        offset: command.offset,
                    },
                );
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
                    | 0x4E5
                    | 0x4E6
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
                self.push_graphics_request_with(
                    sealed,
                    issued,
                    EffectRequest::Attached { id, bone },
                );
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
                        | 0x426
                        | 0x44B
                        | 0x4D9
                        | 0x4E2
                        | 0x4E3
                        | 0x4E4
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
            let floor_angle = if self.issued_on_ground(&command) {
                melee_lb::trigf::atan2f(-normal.x, normal.y)
            } else {
                0.0
            };
            if id == NO_EFFECT {
                // ftCo_09F7.c:306-309: block_70's default, an OSReport
                // after the three draws.
                continue;
            }
            self.push_graphics_request_with(
                sealed,
                issued,
                EffectRequest::Graphics {
                    id,
                    bone,
                    offset,
                    facing: command.issued_facing.unwrap_or(self.physics.facing),
                    floor_angle,
                },
            );
        }
        if sealed {
            // The new script's commands and sounds follow the flush.
            return draws;
        }
        draws += self.resolve_landing_effects(rng, usize::MAX);
        // Sound draws behind the last graphics command (or behind this
        // command, when the caller hands graphics over one at a time).
        self.resolve_sound_draws(assets, rng);
        self.effects.finish_graphics();
        draws
    }
    /// ftCo_09F7.c's `fp->ground_or_air == GA_Ground` for the floor angle,
    /// as the command found it (a set-air command later in the same step
    /// does not reach it).
    fn issued_on_ground(&self, command: &melee_types::combat::GraphicsCommand) -> bool {
        command
            .issued_grounded
            .unwrap_or(self.physics.ground_or_air == melee_types::GroundOrAir::Ground)
    }
    /// One resolved command's request: pending, or sealed with the joint's
    /// matrix when a motion change's flush took it. A command that ran past
    /// link 9 carries the joint it read then ([`Self::capture_issued_joints`]).
    fn push_graphics_request_with(
        &mut self,
        sealed: bool,
        issued: Option<melee_types::combat::IssuedJoint>,
        request: EffectRequest,
    ) {
        // Only the positional kinds (2/5/6) read the joint once, at creation.
        // (Plain matches, not Option combinators: a closure here is emitted
        // twice in melee-ft, which the perf gate's duplicate census counts.)
        let issued = match (issued, &request) {
            (Some(issued), EffectRequest::Graphics { bone, .. }) if *bone == issued.bone => {
                Some(issued)
            }
            _ => None,
        };
        if sealed {
            // Rare (two motion changes in one proc): the queue is moved
            // out for the joint lookup, as the flush itself does.
            let mut queue = std::mem::take(&mut self.effects);
            queue.push_sealed_graphics(request, |bone| match issued {
                Some(issued) => issued.matrix,
                None => self.effect_matrix(bone),
            });
            self.effects = queue;
        } else if let Some(issued) = issued {
            self.effects.push_graphics_at(request, issued.matrix);
        } else {
            self.effects.push_graphics(request);
        }
    }
    /// The joint a graphics command's effect follows (ftCo_09F7.c:75-97):
    /// the held item's, the next of the five rotating effect bones, the
    /// animation's translation bone, a common part, or the command's own.
    fn graphics_bone(
        &self,
        command: &melee_types::combat::GraphicsCommand,
        assets: &super::assets::FighterAssets,
        rotating: &mut u8,
    ) -> usize {
        let bone = if command.item_bone {
            usize::from(self.bones.model.held_item)
        } else {
            command.bone
        };
        match bone {
            ROTATING_EFFECT_BONE => {
                let bone = assets.rotating_effect_bones[usize::from(*rotating)];
                *rotating = (*rotating + 1) % 5;
                bone
            }
            TRANSLATION_EFFECT_BONE => usize::from(self.bones.model.animation_translation),
            _ if command.common_bone => {
                usize::from(assets.parts.part_to_joint[bone].expect("effect common bone missing"))
            }
            _ => bone,
        }
    }
    /// A proc at or past link 9 is about to move the skeleton (an animation
    /// step) while graphics commands it issued are unresolved. Retail's
    /// efAsync_Spawn (0x800679B0) created those effects when the commands
    /// ran (lb_8000B1CC sets up the joint's matrix then), so each keeps the
    /// joint it would have read: ftCo_8008DCE0's color program step inside
    /// Fighter_ChangeMotionState (fighter.c:1346) reads the blended frame
    /// zero, before its ftAnim_8006EBA4 (ftCo_Damage.c:447).
    pub(super) fn capture_issued_joints(&mut self, assets: &super::assets::FighterAssets) {
        if !self.effect_state.dispatch_at_once
            || self.effect_state.invisible
            || self.commands.graphics.is_empty()
        {
            return;
        }
        // The rotating bones advance in command order, as resolution will.
        let mut rotating = self.effect_state.rotating_bone_index;
        // Moved out for the joint lookups; rare (a hit's motion change).
        let mut graphics = std::mem::take(&mut self.commands.graphics);
        for command in graphics.iter_mut() {
            let bone = self.graphics_bone(command, assets, &mut rotating);
            if command.issued_joint.is_none() {
                let matrix = self.effect_matrix(Some(bone));
                command.issued_joint = Some(melee_types::combat::IssuedJoint { bone, matrix });
            }
        }
        self.commands.graphics = graphics;
    }
}
