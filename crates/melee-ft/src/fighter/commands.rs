//! Movement and idle command interpreter, ftAction_80073240 (0x80073240).
//! Archive pointers are converted to instruction indices by assets.rs.
use super::{assets::FighterAssets, RetailTrig};
use crate::{
    anim::{attach::PartFlags, FighterAnimation},
    collision::pose::GroundPoseFlags,
};
use hsd_anim::{
    aobj::AObjEndCallback,
    jobj::{JObjTree, JOBJ_USE_QUATERNION},
};

use melee_cmd::{ColorAnimationRequest, Command};
use melee_types::fixed::FixedVec;

/// Explicit port bound for side requests; consumers drain before exhaustion.
pub const COMMAND_REQUEST_CAPACITY: usize = 64;
/// Wind (opcode 0x3A family) and radial-impulse requests: a subaction issues at most a
/// few per frame and the scene drains them every tick; explicit port bound, overflow fails.
pub const DYNAMICS_REQUEST_CAPACITY: usize = 8;
/// Opcode 40 carries seven-bit texture indices.
const TEXTURE_SLOT_COUNT: usize = 128;
/// Opcode 31 carries a signed seven-bit model group.
const MODEL_GROUP_COUNT: usize = 128;
const MODEL_GROUP_BIAS: i32 = 64;
/// ftAction_800728F8 (0x800728F8), forwarded to the controller-output owner.
#[derive(Clone, Debug)]
pub struct RumbleRequest {
    pub all_players: bool,
    pub id: u16,
    pub duration: u16,
}

/// ft_PlaySFX versus ft_800881D8, ftaction.c:600-608.
#[derive(Clone, Copy, Debug)]
pub enum SoundChannel {
    Ordinary,
    Action,
    FighterVoice,
    /// ft_80088510: Fighter +2150, AX channel 0x42 + player * 2.
    Effect,
    /// ft_800885A8: Fighter +2154, AX channel 0x4E + player * 2.
    StatusEffect,
    /// ft_80088328: stop the two voice channels, then use Fighter +2148.
    OverrideVoice,
    /// ft_80088770: clear the action handle (+2144).
    StopAction,
    /// ft_800887CC: clear the override voice handle (+2148).
    StopOverrideVoice,
}

/// Ordinary ft_PlaySFX request from ftAction_80071B50 (0x80071B50).
#[derive(Clone, Debug)]
pub struct FootstepSound {
    pub channel: SoundChannel,
    pub id: u32,
    pub volume: u8,
    pub pan: u8,
}

#[derive(Clone, Debug, Default)]
pub struct CommandState {
    pub smash_charge: Option<melee_cmd::SmashCharge>,
    pub airborne_changes: FixedVec<melee_cmd::AirborneMode, COMMAND_REQUEST_CAPACITY>,
    /// ftAction_80072894 requests, applied by the character's parasol hook.
    pub parasol_animations: FixedVec<(usize, f32), COMMAND_REQUEST_CAPACITY>,
    pub thrown_by: Option<u32>,
    pub smash_sound_requests: usize,
    pub random_sounds: FixedVec<melee_cmd::RandomSound, COMMAND_REQUEST_CAPACITY>,
    /// ftData_80085CD8: thrown states execute their captor's command stream.
    pub borrowed_script: Option<std::sync::Arc<[Command]>>,
    pub grab_release: bool,
    /// cmd_timer (+2214): the script timer when the release flag was set
    /// (ftAction_800718A4), for the item throw's in-frame interpolation.
    pub release_timer: f32,
    pub throw_reverse: bool,
    /// Fighter throw_flags_b0, consumed by character throw article callbacks.
    pub throw_accessory: bool,
    /// Fighter throw_flags_b1 (ftAction_80071908), consumed by character
    /// motion callbacks through `take_move_cue`.
    pub move_cue: bool,
    pub throw_hitboxes: [Option<melee_types::combat::ThrowHitbox>; 2],
    /// HitCapsule.unk_count is captured before the stale multiplier.
    pub throw_damage_counts: [u32; 2],
    /// ftLib_80086A4C: article draw visibility; reset true on motion entry.
    pub articles_visible: bool,
    /// Fighter +221E bit 5: hide the fighter model (ftdrawcommon.c:233).
    pub fighter_hidden: bool,
    /// Fighter +221E bit 3, inverted (retail sets it to 1 on reset,
    /// fighter.c:380): a subaction has hidden the held item (opcode 35).
    pub held_item_hidden: bool,
    pub hitboxes: [Option<melee_coll::hitbox::HitCapsule>; 4],
    /// The first recorded contact affects subsequently created hitboxes of
    /// this attack instance. Multiple-entry history remains a combat boundary.
    pub first_hit_stale_penalty: Option<f32>,
    pub stale_multiplier: Option<f32>,
    pub jab_followup: bool,
    pub rapid_jab: bool,
    pub rapid_jab_loop_end: bool,
    pub capsule_status: melee_types::combat::HurtStatus,
    pub capsule_overrides: FixedVec<(usize, melee_types::combat::HurtStatus), 64>,
    pub sword_trail: Option<(i32, bool)>,
    /// Ordered ftCo_8009E318 requests, consumed immediately after commands.
    pub dynamic_toggles: FixedVec<usize, COMMAND_REQUEST_CAPACITY>,
    pub color_animations: FixedVec<ColorAnimationRequest, COMMAND_REQUEST_CAPACITY>,
    /// ftAction_80071D40 -> ftParts_80074B0C: retained DObj group selection.
    /// DObj visibility is renderer output, like texture_frames; it changes no SRT.
    pub model_selections: ModelSelections,
    pub hurt_status: melee_types::combat::HurtStatus,
    pub allow_interrupt: bool,
    /// cmd_vars (+2200): subaction-controlled state variables.
    pub variables: [u32; 4],
    pub graphics: melee_types::fixed::FixedVec<
        melee_types::combat::GraphicsCommand,
        { melee_ef::request::REQUEST_CAPACITY },
    >,
    pub script: melee_cmd::ScriptState,
    /// Requests to costume TObjs (ftAnim_800704F0). Rendering consumes these;
    /// TObj/GX execution remains M8, like the existing HSD model loader.
    pub texture_frames: FixedVec<(usize, f32), TEXTURE_SLOT_COUNT>,
    /// Fighter +221E mask1: ftAnim_800704F0 has installed a texture override.
    pub texture_animation_active: bool,
    /// ftAction_80072E4C requests with the number of graphics commands queued
    /// before each one: both reach ftCo_8009F834 in script order.
    pub landing_effects: FixedVec<(u16, usize), COMMAND_REQUEST_CAPACITY>,
    /// ftAction_80073118: radial dynamics requests, no RNG.
    pub wind_effects: FixedVec<melee_cmd::WindEffect, DYNAMICS_REQUEST_CAPACITY>,
    pub radial_impulses: FixedVec<melee_lb::radial_force::RadialImpulse, DYNAMICS_REQUEST_CAPACITY>,
    /// ftAction_800728F8: controller-output requests, no RNG.
    pub rumble_requests: FixedVec<RumbleRequest, COMMAND_REQUEST_CAPACITY>,
    /// ftAction_80071B50 (0x80071B50) requests; audio is an output request.
    pub footstep_sounds: FixedVec<FootstepSound, COMMAND_REQUEST_CAPACITY>,
    /// ftAction_80072CD8 (0x80072CD8) requests this step, before the floor's
    /// terrain decides which sounds they make (see `resolve_terrain_footsteps`).
    pub terrain_footsteps: FixedVec<TerrainStep, COMMAND_REQUEST_CAPACITY>,
}
/// An ftAction_80072CD8 request: its sound, the foot its terrain effect uses,
/// and the number of graphics commands queued before it (both reach
/// ftCo_8009F834 in script order).
#[derive(Clone, Debug)]
pub struct TerrainStep {
    pub sound: FootstepSound,
    pub alt_foot: bool,
    pub graphics_before: usize,
}
impl CommandState {
    /// ft_80089228 (80089228): stale a hitbox's damage for the current move.
    /// The motion's multiplier applies only when it differs from 1 (retail
    /// 8008927C fmuls); without a staled move, a hit this instance already
    /// landed subtracts the first table weight (ft_80089118).
    pub fn stale_damage(&self, damage: f32) -> f32 {
        if let Some(multiplier) = self.stale_multiplier {
            if multiplier != 1.0 {
                return damage * multiplier;
            }
            damage
        } else if let Some(penalty) = self.first_hit_stale_penalty {
            damage * (1.0 - penalty)
        } else {
            damage
        }
    }

    /// ftAction_8007349C (8007349C): UpdateCmd executes control flow only.
    pub(super) fn advance_control(&mut self, animation: &FighterAnimation, assets: &FighterAssets) {
        self.script
            .begin_frame(animation.frame + animation.remainder, animation.speed);
        while self
            .script
            .next(&assets.commands, animation.speed)
            .is_some()
        {}
    }
    /// ftaction.c:1318-1348; retail --fused has no multiply-add sites.
    /// `held_item_hand` names what a held-item visibility command acts on
    /// (see [`HeldItemHand`]).
    pub fn step(
        &mut self,
        animation: &mut FighterAnimation,
        tree: &mut JObjTree,
        pose: &mut GroundPoseFlags,
        assets: &FighterAssets,
        held_item_hand: HeldItemHand,
    ) {
        self.step_inner(animation, tree, pose, assets, held_item_hand, false);
    }

    /// ftAction_80073354 (0x80073354): seek a newly installed script to a
    /// nonzero animation phase, skipping transient effects and part blending.
    pub(super) fn seek(
        &mut self,
        animation: &mut FighterAnimation,
        tree: &mut JObjTree,
        pose: &mut GroundPoseFlags,
        assets: &FighterAssets,
        held_item_hand: HeldItemHand,
    ) {
        self.step_inner(animation, tree, pose, assets, held_item_hand, true);
    }

    fn step_inner(
        &mut self,
        animation: &mut FighterAnimation,
        tree: &mut JObjTree,
        pose: &mut GroundPoseFlags,
        assets: &FighterAssets,
        held_item_hand: HeldItemHand,
        seeking: bool,
    ) {
        let borrowed_script = self.borrowed_script.clone();
        let script = borrowed_script.as_deref().unwrap_or(&assets.commands);
        self.script
            .begin_frame(animation.frame + animation.remainder, animation.speed);
        while let Some(command) = self.script.next(script, animation.speed) {
            match command {
                Command::BeginLoop(_)
                | Command::EndLoop
                | Command::End
                | Command::Goto(_)
                | Command::WaitAnimationLoop
                | Command::Wait(_)
                | Command::AtFrame(_)
                | Command::Call { .. }
                | Command::Return
                | Command::Unported(_) => unreachable!("interpreter consumes control flow"),
                Command::ThrowAccessory => {
                    self.throw_accessory = true;
                }
                Command::MoveCue => self.move_cue = true,
                Command::SmashSound => {
                    if !seeking {
                        self.smash_sound_requests += 1;
                    }
                }
                Command::GrabRelease => {
                    // ftAction_800718A4: throw_flags_b3 is also the rapid-jab loop checkpoint.
                    self.grab_release = true;
                    self.rapid_jab_loop_end = true;
                    self.release_timer = self.script.timer;
                }
                Command::ThrowReverse => self.throw_reverse = true,
                Command::SetThrowHitbox { id, descriptor } => {
                    // ftAction_80071F0C skips these records when seeking.
                    if !seeking {
                        let mut descriptor = descriptor.clone();
                        self.throw_damage_counts[*id] =
                            gekko_math::msl::fctiwz(descriptor.damage) as u32;
                        descriptor.damage = self.stale_damage(descriptor.damage);
                        self.throw_hitboxes[*id] = Some(descriptor);
                    }
                }
                Command::SpawnHitbox { id, descriptor } => {
                    if !seeking && (!descriptor.requires_throw_owner || self.thrown_by.is_some()) {
                        let mut descriptor = descriptor.clone();
                        if descriptor.common_bone {
                            descriptor.bone = usize::from(
                                assets.parts.part_to_joint[descriptor.bone]
                                    .expect("hitbox semantic bone"),
                            );
                            descriptor.common_bone = false;
                        }
                        if let Some(charge) = &self.smash_charge {
                            descriptor.damage = charge.scale_damage(descriptor.damage);
                        }
                        let knockback_damage = gekko_math::msl::fctiwz(descriptor.damage) as u32;
                        descriptor.damage = self.stale_damage(descriptor.damage);
                        melee_coll::hitbox::spawn(&mut self.hitboxes, *id, &descriptor);
                        self.hitboxes[*id]
                            .as_mut()
                            .expect("spawned hitbox")
                            .knockback_damage = knockback_damage;
                    }
                }
                Command::HitboxTargets { id, items, enabled } => {
                    // ftAction_80071774 skips the record when seeking.
                    if !seeking {
                        if let Some(hit) = &mut self.hitboxes[*id] {
                            if *items {
                                hit.hits_items = *enabled;
                            } else {
                                hit.hits_fighters = *enabled;
                            }
                        }
                    }
                }
                Command::ClearHitbox(id) => {
                    if !seeking {
                        self.hitboxes[*id] = None;
                    }
                }
                Command::SetHitboxDamage { id, damage } => {
                    // ftAction_8007162C -> ftColl_8007ABD0 (no fused ops):
                    // charge-scaled damage, its integer knockback copy, then
                    // staling. Seeking skips the word (ftAction_8007168C).
                    // Fighter y-scale is always 1 here (8007AC0C branch).
                    if !seeking {
                        let damage = match &self.smash_charge {
                            Some(charge) => charge.scale_damage(*damage),
                            None => *damage,
                        };
                        let staled = self.stale_damage(damage);
                        if let Some(hit) = &mut self.hitboxes[*id] {
                            hit.knockback_damage = gekko_math::msl::fctiwz(damage) as u32;
                            hit.descriptor.damage = staled;
                        }
                    }
                }
                Command::SetHitboxRadius { id, radius } => {
                    // ftAction_8007169C; seeking skips the word
                    // (ftAction_800716F8). A disabled capsule is not kept.
                    if !seeking {
                        if let Some(hit) = &mut self.hitboxes[*id] {
                            hit.descriptor.radius = *radius;
                        }
                    }
                }
                Command::ClearHitboxes => {
                    if !seeking {
                        self.hitboxes.fill(None);
                    }
                }
                Command::ToggleDynamics(bone) => {
                    if !seeking {
                        self.dynamic_toggles
                            .push(usize::try_from(*bone).expect("dynamic bone index"));
                    }
                }
                Command::SwordTrail { duration, reverse } => {
                    if !seeking {
                        self.sword_trail = Some((*duration, *reverse));
                    }
                }
                Command::RapidJab(enabled) => self.rapid_jab = *enabled,
                Command::JabFollowup(disabled) => {
                    if !disabled {
                        self.jab_followup = true;
                    }
                }
                Command::HurtCapsuleStatus { bone, status } => {
                    if let Some(bone) = bone {
                        let index = self
                            .capsule_overrides
                            .iter()
                            .position(|entry| entry.0 == *bone);
                        if let Some(index) = index {
                            self.capsule_overrides
                                .iter_mut()
                                .nth(index)
                                .expect("capsule override")
                                .1 = *status;
                        } else {
                            self.capsule_overrides.push((*bone, *status));
                        }
                    } else {
                        self.capsule_status = *status;
                        self.capsule_overrides.clear();
                    }
                }
                Command::ColorAnimation(request) => {
                    // ftAction_80072A4C (80072A4C): seeking only advances the word.
                    if !seeking {
                        self.color_animations.push(*request);
                    }
                }
                Command::ModelSelection { group, variant } => {
                    self.model_selections.insert(*group, *variant);
                }
                Command::FighterVisibility(hidden) => self.fighter_hidden = *hidden,
                Command::ArticleVisibility(visible) => self.articles_visible = *visible,
                Command::HeldItemVisibility(visible) => {
                    // ftAction_80071F34 (80071F34) -> ftCommon_8007F5CC.
                    self.set_held_item_visibility(*visible, animation, tree, assets, held_item_hand)
                }
                Command::SmashCharge(charge) => self.smash_charge = Some(*charge),
                Command::SetAirborne(state) => self.airborne_changes.push(*state),
                Command::ParasolAnimation { index, frames } => {
                    self.parasol_animations.push((*index, *frames))
                }
                Command::HurtStatus(status) => self.hurt_status = *status,
                Command::AllowInterrupt => self.allow_interrupt = true,
                Command::Graphics(command) => {
                    if !seeking {
                        self.graphics.push(command.clone());
                    }
                }
                Command::SetVariable { index, value } => self.variables[*index] = *value,
                Command::Rumble {
                    all_players,
                    id,
                    duration,
                } => {
                    if !seeking {
                        self.rumble_requests.push(RumbleRequest {
                            all_players: *all_players,
                            id: *id,
                            duration: *duration,
                        });
                    }
                }
                Command::WindEffect(wind) => {
                    if !seeking {
                        self.wind_effects.push(*wind);
                    }
                }
                Command::RandomSound(sound) => {
                    if !seeking {
                        self.random_sounds.push(*sound);
                    }
                }
                Command::FootstepSound {
                    behavior,
                    id,
                    volume,
                    pan,
                    terrain,
                    alt_foot,
                } => {
                    if !seeking {
                        let channel = match behavior {
                            0 => SoundChannel::Ordinary,
                            1 => SoundChannel::Action,
                            2 => SoundChannel::FighterVoice,
                            3 => SoundChannel::Effect,
                            4 => SoundChannel::StatusEffect,
                            6 => SoundChannel::OverrideVoice,
                            // ftaction.c:623-651: the one-word stops.
                            11 => SoundChannel::StopAction,
                            15 => SoundChannel::StopOverrideVoice,
                            _ => unimplemented!("ftaction.c:598-651: sound behavior {behavior}"),
                        };
                        let sound = FootstepSound {
                            channel,
                            id: *id,
                            volume: *volume,
                            pan: *pan,
                        };
                        if *terrain {
                            self.terrain_footsteps.push(TerrainStep {
                                sound,
                                alt_foot: *alt_foot,
                                graphics_before: self.graphics.len(),
                            });
                        } else {
                            self.footstep_sounds.push(sound);
                        }
                    }
                }
                Command::LandingEffect(id) => {
                    if !seeking {
                        self.landing_effects.push((*id, self.graphics.len()));
                    }
                }
                Command::GroundPose(flags) => {
                    pose.0 = *flags;
                    if flags & 4 == 0 {
                        tree.set_rotation_x(animation.root, 0.0);
                    }
                }
                Command::Part {
                    group,
                    variant,
                    blend,
                } => apply_part(
                    animation,
                    tree,
                    assets,
                    *group,
                    *variant,
                    if seeking { 0.0 } else { *blend },
                ),
                Command::Texture { indices, frame } => {
                    for &index in indices {
                        self.set_texture_frame(index, *frame);
                    }
                }
            }
        }
    }
}

/// ftAnim_ApplyPartAnim (0x80070B0C), ftanim.c:1257-1278, and
/// ftAnim_80070904 (0x80070904), ftanim.c:1165-1185.
fn apply_part(
    animation: &mut FighterAnimation,
    _tree: &mut JObjTree,
    assets: &FighterAssets,
    group: usize,
    variant: usize,
    blend: f32,
) {
    let resource = &assets.part_animations[&(group, variant)];
    let state = &mut animation.part_animations[group];
    state.current = variant as i8;
    state.duration = blend;
    state.progress = 0.0;
    // ftAnim_ApplyPartAnim: fdivs after lb_8000BFF0; no eligible FMA.
    state.rate = if blend != 0.0 {
        resource.duration / blend
    } else {
        0.0
    };
    state.joints.clear();
    for &joint in &assets.bones.animation_sets[group].as_ref().unwrap().joints {
        state.joints.push(usize::from(joint));
    }
    let mut index = resource.root;
    for (node, prepared) in resource.nodes.iter().zip(&resource.prepared) {
        while animation.parts[index].motion_mask != 0 {
            index += 1;
        }
        let part = &mut animation.parts[index];
        if !part.flags.contains(PartFlags::LOCKED) && node.aobjdesc.is_some() {
            let joint = part.joint;
            animation.blend_tree.add_prepared_joint_anim(
                joint,
                node,
                prepared.as_ref().expect("part animation"),
            );
            animation.blend_tree.clear_flags(joint, JOBJ_USE_QUATERNION);
            animation.blend_tree.req_anim(joint, 0.0);
            animation
                .blend_tree
                .anim::<RetailTrig>(joint, &mut AObjEndCallback::default());
            part.flags.0 |= PartFlags::PART_ANIMATION;
        }
        index += 1;
    }
}

/// What ftCommon_8007F5CC's per-kind visibility hooks act on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeldItemHand {
    /// fp->item_gobj is NULL: only the visibility bit changes.
    Empty,
    /// A heavy item: Fighter_OnItemInvisible/Visible (ft/inlines.h:167-184)
    /// skip the hand animation for it (itIsHeavy).
    Heavy,
    /// A light item whose hand pose shows in this x8B0 slot: every ported
    /// kind's ftData_OnItemInvisible/Visible entry passes the same slot as
    /// its OnItemPickup `shown` argument (e.g. ftFx_Init_OnItemInvisible).
    Light(usize),
}

impl CommandState {
    /// ftCommon_8007F5CC (8007F5CC): with an item in hand, a change of
    /// visibility runs the kind's hook. Hiding releases the hand pose to the
    /// main motion (ftAnim_80070CC4); showing reapplies the slot's selection
    /// (ftAnim_80070C48).
    pub(super) fn set_held_item_visibility(
        &mut self,
        visible: bool,
        animation: &mut FighterAnimation,
        tree: &mut JObjTree,
        assets: &FighterAssets,
        hand: HeldItemHand,
    ) {
        if hand != HeldItemHand::Empty && self.held_item_hidden == visible {
            if let HeldItemHand::Light(slot) = hand {
                if visible {
                    show_part_selection(animation, tree, assets, slot);
                } else {
                    remove_part_animation(animation, tree, assets, slot);
                }
            }
        }
        self.held_item_hidden = !visible;
    }
}

/// ftAnim_80070C48 (80070C48): apply slot `group`'s persistent selection.
pub(super) fn show_part_selection(
    animation: &mut FighterAnimation,
    tree: &mut JObjTree,
    assets: &FighterAssets,
    group: usize,
) {
    let selection = animation.part_animations[group].previous;
    if selection != -1 {
        apply_part(animation, tree, assets, group, selection as usize, 0.0);
    }
}

/// ftAnim_80070CC4 (80070CC4): a live part animation's parts drop their
/// ownership (as ftAnim_80070F28 does), the slot goes inactive, and the
/// main motion takes the subtree back at its current frame
/// (ftAnim_8006EED4), or with none attached the descriptor pose does.
pub(super) fn remove_part_animation(
    animation: &mut FighterAnimation,
    tree: &mut JObjTree,
    assets: &FighterAssets,
    group: usize,
) {
    let slot = &mut animation.part_animations[group];
    if slot.current == -1 {
        return;
    }
    slot.current = -1;
    for &bone in slot.joints.iter() {
        animation.parts[bone].flags.0 &= !PartFlags::PART_ANIMATION;
    }
    let root = usize::from(
        assets.bones.animation_sets[group]
            .as_ref()
            .expect("part animation set")
            .root_joint,
    );
    if animation.motion_id < 0 {
        // x590 is NULL: ftAnim_8006FA58 from the costume's descriptor.
        animation.reset_subtree_pose(tree, root);
        return;
    }
    animation
        .resume_dynamic_subtree::<RetailTrig>(tree, root, &assets.motions[&animation.motion_id])
        .expect("part animation subtree");
}

/// ftAnim_80070F28 (0x80070F28), then ftAnim_80070E74 (0x80070E74).
/// Fighter_ChangeMotionState calls both before attaching the new main motion
/// (fighter.c:996-997). Remove temporary part ownership before pose reset;
/// then reinstall any persistent selection from x8B0[i].x10.
pub(super) fn reset_parts(
    animation: &mut FighterAnimation,
    tree: &mut JObjTree,
    assets: &FighterAssets,
) {
    for slot in &mut animation.part_animations {
        if slot.current != -1 {
            for &bone in slot.joints.iter() {
                animation.parts[bone].flags.0 &= !PartFlags::PART_ANIMATION;
            }
            slot.current = -1;
        }
    }
    for group in 0..animation.part_animations.len() {
        let previous = animation.part_animations[group].previous;
        if previous != -1 {
            apply_part(animation, tree, assets, group, previous as usize, 0.0);
        }
    }
}

impl std::ops::Deref for CommandState {
    type Target = melee_cmd::ScriptState;
    fn deref(&self) -> &Self::Target {
        &self.script
    }
}
impl std::ops::DerefMut for CommandState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.script
    }
}

/// Model selector is signed seven-bit data in opcode 31 (ftAction_80071D40).
#[derive(Clone, Debug)]
pub struct ModelSelections {
    entries: [Option<i32>; MODEL_GROUP_COUNT],
}
impl Default for ModelSelections {
    fn default() -> Self {
        Self {
            entries: [None; MODEL_GROUP_COUNT],
        }
    }
}
impl ModelSelections {
    pub fn insert(&mut self, group: i32, variant: i32) -> Option<i32> {
        self.entries[usize::try_from(group + MODEL_GROUP_BIAS).expect("model group")]
            .replace(variant)
    }
    pub fn get(&self, group: &i32) -> Option<&i32> {
        self.entries
            .get(usize::try_from(group + MODEL_GROUP_BIAS).ok()?)
            .and_then(Option::as_ref)
    }
}

impl CommandState {
    /// `fp->throw_flags = 0`: every throw flag bit at once.
    pub fn clear_throw_flags(&mut self) {
        self.throw_accessory = false;
        self.move_cue = false;
        self.throw_reverse = false;
        self.grab_release = false;
        self.rapid_jab_loop_end = false;
    }
    /// throw_flags_b1, read and cleared.
    pub fn take_move_cue(&mut self) -> bool {
        std::mem::take(&mut self.move_cue)
    }
    /// throw_flags_b3, read and cleared (ftCheckThrowB3). The port tracks the
    /// one retail bit as two consumers' latches; both clear together.
    pub fn take_throw_flag_b3(&mut self) -> bool {
        self.rapid_jab_loop_end = false;
        std::mem::take(&mut self.grab_release)
    }
}

impl super::FighterCore {
    /// ftAction_80071FC8 (80072014): select before graphics or effect procs draw.
    pub fn resolve_random_sound_commands(&mut self, rng: &mut gekko_math::HsdRng) {
        while !self.commands.random_sounds.is_empty() {
            let sound = self.commands.random_sounds.remove(0);
            assert!((1..=6).contains(&sound.range), "retail random sound range");
            let index = rng.randi(i32::from(sound.range)) as usize;
            let channel = match sound.behavior {
                0 => SoundChannel::Ordinary,
                1 => SoundChannel::Action,
                2 => SoundChannel::FighterVoice,
                3 => SoundChannel::Effect,
                _ => unimplemented!("ftAction_80071FC8 sound behavior {}", sound.behavior),
            };
            self.commands.footstep_sounds.push(FootstepSound {
                channel,
                id: sound.ids[index],
                volume: sound.volume,
                pan: sound.pan,
            });
        }
    }
}

impl super::FighterCore {
    /// Character callbacks use the same part-source installation as command 30.
    /// Copies into the visible skeleton remain owned by normal animation playback.
    pub fn apply_part_animation(
        &mut self,
        assets: &FighterAssets,
        group: usize,
        variant: usize,
        blend: f32,
    ) {
        apply_part(
            &mut self.animation,
            &mut self.skeleton,
            assets,
            group,
            variant,
            blend,
        );
    }

    /// Fighter_OnKnockbackEnter/Exit(gobj, 1): authored texture slots 1 then 0.
    pub fn set_knockback_texture_frames(&mut self, frame: f32) {
        for index in [1, 0] {
            self.commands.set_texture_frame(index, frame);
        }
    }
}

impl CommandState {
    /// ftAnim_800704F0; the modeled texture request and its reset ownership.
    pub fn set_texture_frame(&mut self, index: usize, frame: f32) {
        let existing = self.texture_frames.iter_mut().find(|(i, _)| *i == index);
        if let Some(entry) = existing {
            entry.1 = frame;
        } else {
            self.texture_frames.push((index, frame));
        }
        self.texture_animation_active = true;
    }

    /// ftAnim_80070654: every costume texture animation is requested at frame0.
    /// Untouched modeled slots already denote frame0; retain explicit slots so
    /// consumers see the reset rather than retain the previously requested frame.
    pub(super) fn reset_texture_animation(&mut self) {
        if self.texture_animation_active {
            for (_, frame) in self.texture_frames.iter_mut() {
                *frame = 0.0;
            }
            self.texture_animation_active = false;
        }
    }
}
