//! One fighter animation tick, with action-command dispatch supplied by T6.
use hsd_anim::aobj::{AObj, AObjEndCallback, AOBJ_LOOP, AOBJ_NO_ANIM};
use hsd_anim::jobj::{JObjId, JObjTree, JOBJ_MTX_INDEP_SRT, JOBJ_USE_QUATERNION};
use hsd_anim::mtx::InverseTrig;
use hsd_archive::desc::FigaTree;
use hsd_types::Vec3;
use melee_lb::anim::AttachError;

use super::attach::{
    attach_motion_remapped, select_motion, AnimationPart, MotionRemap, MotionRemapView, PartFlags,
};
use super::blend::{advance_blend, blend_pose, copy_pose};
use super::root_motion::RootMotion;
use super::wait_choice::{choose_wait_animation, WaitChoice, WaitEntry};
use gekko_math::rng::HsdRng;

/// `Fighter.x594_s32`: animation flags, conditional-bone mask and source kind.
#[derive(Clone, Copy, Debug, Default)]
pub struct MotionFlags(pub u32);
impl MotionFlags {
    pub const ROOT_MOTION: u32 = 0x8000_0000;
    pub const LOOP: u32 = 0x4000_0000;
    pub const ACCUMULATE_LOOPS: u32 = 0x2000_0000;
    pub const SECOND_ROOT: u32 = 0x0400_0000;
    pub const ATTRIBUTE_SCALE: u32 = 0x0200_0000;
    pub fn contains(self, flag: u32) -> bool {
        self.0 & flag != 0
    }
    pub fn bone_mask(self) -> u32 {
        (self.0 >> 9) & 0x1fff
    }
    /// Fighter.x596_bits.x7: outgoing joint copied on the next blended entry.
    /// Retail80069C50 extracts the three bits above the source skeleton.
    pub fn blend_exit_joint(self) -> u8 {
        ((self.0 >> 6) & 7) as u8
    }
    /// Fighter.x597_bits, the skeleton that authored this FigaTree.
    pub fn source_skeleton(self) -> u8 {
        (self.0 & 0x3f) as u8
    }
}

/// Loaded FigaTree and animation-table metadata. Id is a submotion id
/// (`Fighter.anim_id`, +0x14), not the action-state id at Fighter+0x10.
#[derive(Clone, Debug)]
pub struct Motion {
    pub id: i32,
    pub animation: FigaTree,
    pub flags: MotionFlags,
    /// `Fighter.x28[id][0]`, usually 6 for Wait1 and 0 for Wait2.
    pub blend_frames: f32,
    pub remap: Option<MotionRemap>,
}

/// One `Fighter.x8B0` entry (stride 0x14); the five bone lists come from
/// `ftData.x1C`. Command handlers select the states and install part poses.
#[derive(Clone, Debug)]
pub struct PartAnimation {
    /// +0: command-owned state.
    pub state: i32,
    /// +4/+8/+C: duration, progress, per-tick rate.
    pub duration: f32,
    pub progress: f32,
    pub rate: f32,
    /// +10/+11, -1 means unselected/inactive.
    pub previous: i8,
    pub current: i8,
    pub joints: melee_types::fixed::FixedVec<usize, { crate::desc::bones::MAX_JOINTS as usize }>,
}
impl Default for PartAnimation {
    fn default() -> Self {
        Self {
            state: 0,
            duration: 0.0,
            progress: 0.0,
            rate: 0.0,
            previous: -1,
            current: -1,
            joints: Default::default(),
        }
    }
}

/// Animation-owned Fighter state (`ft/types.h`). The two trees share joint
/// ids; the secondary tree represents FighterBone.x4_jobj2 and fp+0x8AC.
#[derive(Clone, Debug)]
pub struct FighterAnimation {
    /// `anim_id`, Fighter+0x14; -1 bypasses the main playback step.
    pub motion_id: i32,
    /// `cur_anim_frame`, Fighter+0x894: sampled from an eligible AObj.
    pub frame: f32,
    /// `x898_unk`, Fighter+0x898: accumulated completed loop lengths.
    pub remainder: f32,
    /// `frame_speed_mul`, Fighter+0x89C.
    pub speed: f32,
    /// `x8A0_unk`, Fighter+0x8A0: rate saved while x2223_b0 is set.
    pub saved_speed: f32,
    /// `x8A4_animBlendFrames`, Fighter+0x8A4 (not cleared at completion).
    pub blend_duration: f32,
    /// `x8A8_anim_frame`, Fighter+0x8A8.
    pub blend_progress: f32,
    /// `x8AC_animSkeleton`, Fighter+0x8AC, and FighterBone.x4_jobj2.
    pub blend_tree: JObjTree,
    pub flags: MotionFlags,
    pub parts: Vec<AnimationPart>,
    /// `x8B0[5]`, Fighter+0x8B0..0x913.
    pub part_animations: [PartAnimation; 5],
    pub root_motion: Option<RootMotion>,
    /// `ftData.x8.x10`: joint whose scale is the reciprocal model scale.
    pub translation_joint: Option<usize>,
    /// `co_attrs.model_scaling` (Fighter+0x19C).
    pub model_scale: f32,
    pub root: JObjId,
    rest_pose: std::sync::Arc<JObjTree>,
}

impl FighterAnimation {
    /// Build an unmasked runtime skeleton. Spawn may then install conditional
    /// masks and locked/dynamic/part-animation flags from its part setup.
    pub fn new(tree: &JObjTree, root: JObjId) -> Self {
        // ftParts_8007482C -> ftParts_IntpJObjLoad: interpolation joints
        // are loaded without DObjs. They must not duplicate material ticks.
        let mut blend_tree = tree.clone();
        for id in blend_tree.depth_first(root).collect::<Vec<_>>() {
            blend_tree.get_mut(id).dobj.clear();
            blend_tree.get_mut(id).aobj = None;
        }
        // FigaTree node counts are signed bytes (lbanim.h); -1 terminates
        // the table, so a joint can own at most 127 tracks.
        blend_tree.reserve_animation_tracks(i8::MAX as usize);
        let parts = tree
            .depth_first(root)
            .map(|joint| {
                let mut depth = 0;
                let mut parent = tree.parent(joint);
                while let Some(id) = parent {
                    depth += 1;
                    parent = tree.parent(id);
                }
                AnimationPart {
                    joint,
                    depth,
                    flags: PartFlags(PartFlags::PRESENT),
                    motion_mask: 0,
                }
            })
            .collect();
        Self {
            motion_id: -1,
            frame: 0.0,
            remainder: 0.0,
            speed: 1.0,
            saved_speed: 1.0,
            blend_duration: 0.0,
            blend_progress: 0.0,
            blend_tree,
            flags: MotionFlags::default(),
            parts,
            part_animations: std::array::from_fn(|_| PartAnimation::default()),
            root_motion: None,
            translation_joint: None,
            model_scale: 1.0,
            root,
            rest_pose: std::sync::Arc::new(tree.clone()),
        }
    }

    /// Reset clocks while retaining both trees and their track buffers.
    /// Fighter_UnkProcessDeath (80068354) probes support using the retained
    /// model pose. Resetting its SRT here changes the locked respawn ECB.
    pub fn reset_for_spawn(&mut self, tree: &mut JObjTree) {
        self.clear_motion(tree);
        self.frame = 0.0;
        self.part_animations.fill_with(PartAnimation::default);
        if let Some(root) = &mut self.root_motion {
            root.primary_history = Default::default();
            root.secondary_history = Default::default();
            root.compensate_joint = None;
        }
        // Fighter_UnkInitReset does not reconstruct the parts or secondary
        // skeleton. Keep dynamic locks, cached matrices and retained SRT: the
        // next motion's ftCo_8009CB40 selects only actual ownership changes.
        tree.events.clear();
        self.blend_tree.events.clear();
    }

    /// `ftAnim_8006EBE8` + `ftAnim_8006FE08`. Requesting does not evaluate;
    /// Wait's restart calls `step` immediately after this (0x8008A6D8).
    /// Existing frame/remainder and Fighter speed are retained, as in that
    /// restart path. `rate` sets AObj rates; action-state entry separately
    /// sets Fighter speed through `set_rate` (0x8006F0FC).
    pub fn set_animation(
        &mut self,
        tree: &mut JObjTree,
        motion: &Motion,
        start: f32,
        rate: f32,
    ) -> Result<(), AttachError> {
        self.set_animation_remapped(
            tree,
            motion,
            start,
            rate,
            motion.remap.as_ref().map(MotionRemap::view),
            None,
        )
    }

    pub(crate) fn set_animation_remapped(
        &mut self,
        tree: &mut JObjTree,
        motion: &Motion,
        start: f32,
        rate: f32,
        remap: Option<MotionRemapView<'_>>,
        blend_override: Option<f32>,
    ) -> Result<(), AttachError> {
        let blend_frames = blend_override.unwrap_or(motion.blend_frames);
        // Selection depends only on links/part metadata. Validate before taking
        // the running tree, then reset and attach in the original order.
        select_motion(
            if blend_frames == 0.0 {
                tree
            } else {
                &self.blend_tree
            },
            &self.parts,
            &motion.animation,
            motion.flags.bone_mask(),
            remap,
        )?;
        let mut target = if blend_frames == 0.0 {
            std::mem::take(tree)
        } else {
            std::mem::take(&mut self.blend_tree)
        };
        target.remove_anim_all_by_flags(self.root, 1);
        self.reset_pose(&mut target, blend_frames != 0.0);
        attach_motion_remapped(
            &mut target,
            &self.parts,
            &motion.animation,
            motion.flags.bone_mask(),
            remap,
        )?;
        if blend_frames == 0.0 {
            self.blend_tree.remove_anim_all_by_flags(self.root, 1);
            *tree = target;
        } else {
            tree.remove_anim_all_by_flags(self.root, 1);
            self.blend_tree = target;
        }
        self.flags = motion.flags;
        self.motion_id = motion.id;
        let configure = |active: &mut JObjTree| {
            active.req_anim_all_by_flags(self.root, 1, start);
            configure_aobjs(
                active,
                self.root,
                rate,
                motion.flags.contains(MotionFlags::LOOP),
            );
        };
        if blend_frames != 0.0 {
            configure(&mut self.blend_tree);
        }
        configure(tree);
        self.blend_duration = blend_frames;
        self.blend_progress = 0.0;
        Ok(())
    }

    /// ftAnim_8006EDD0 (8006EDD0): replace only the secondary skeleton's
    /// animation. Main submotion, frame, command stream and blend timer survive.
    pub fn set_secondary_animation(
        &mut self,
        motion: &Motion,
        start: f32,
        rate: f32,
    ) -> Result<(), AttachError> {
        select_motion(
            &self.blend_tree,
            &self.parts,
            &motion.animation,
            self.flags.bone_mask(),
            None,
        )?;
        let mut secondary = std::mem::take(&mut self.blend_tree);
        secondary.remove_anim_all_by_flags(self.root, 1);
        self.reset_pose(&mut secondary, true);
        attach_motion_remapped(
            &mut secondary,
            &self.parts,
            &motion.animation,
            self.flags.bone_mask(),
            None,
        )?;
        secondary.req_anim_all(self.root, start);
        configure_aobjs(
            &mut secondary,
            self.root,
            rate,
            motion.flags.contains(MotionFlags::LOOP),
        );
        self.blend_tree = secondary;
        Ok(())
    }

    /// ftCo_800CC988 (800CC988) / ftAnim_8006FE9C (8006FE9C) /
    /// ftAnim_8006FF74 (8006FF74): advance secondary pose and blend below TopN.
    /// Reuses the audited lb_8000C490 quaternion/SRT kernel in blend_pose.
    pub fn apply_fall_pose<T: InverseTrig>(&mut self, tree: &mut JObjTree, weight: f32) {
        self.blend_tree.anim_all::<T>(self.root);
        let inverse = 1.0 - weight;
        for part in self.parts.iter().skip(1).filter(|p| p.flags.eligible()) {
            let source = self.blend_tree.get(part.joint);
            if weight == 1.0 || part.flags.contains(PartFlags::COPY) {
                copy_pose(source, tree, part.joint);
            } else {
                blend_pose::<T>(source, tree, part.joint, weight, inverse);
            }
        }
    }

    /// Fighter_ChangeMotionState (0x800693AC), fighter.c:1349-1357.
    /// The state table's SM_None removes AObjs and keeps the current pose.
    pub fn clear_motion(&mut self, tree: &mut JObjTree) {
        tree.remove_anim_all_by_flags(self.root, 1);
        self.blend_tree.remove_anim_all_by_flags(self.root, 1);
        self.motion_id = -1;
        self.flags = MotionFlags(0);
        self.frame = -1.0;
        self.remainder = 0.0;
        self.speed = 1.0;
        self.saved_speed = 1.0;
        self.blend_duration = 0.0;
        self.blend_progress = 0.0;
    }

    /// ftAnim_8006EED4 (8006EED4): restore and attach one unlocked subtree
    /// at the current frame, preserving the fighter's animation clock.
    pub fn resume_dynamic_subtree<T: InverseTrig>(
        &mut self,
        tree: &mut JObjTree,
        bone: usize,
        motion: &Motion,
    ) -> Result<(), AttachError> {
        let joint = self.parts[bone].joint;
        let depth = self.parts[bone].depth;
        let end = (bone + 1..self.parts.len())
            .find(|&i| self.parts[i].depth <= depth)
            .unwrap_or(self.parts.len());
        let blending = self.blend_duration != 0.0;
        // ftAnim_GetNextJointInTree also visits the starting descriptor's
        // following siblings. Attachment and evaluation stop at the subtree.
        let reset_end = (bone + 1..self.parts.len())
            .find(|&i| self.parts[i].depth < depth)
            .unwrap_or(self.parts.len());
        // MAX_FT_PARTS (ftparts.h); only the character's actual parts are used.
        let mut storage = [self.parts[0]; crate::desc::bones::MAX_JOINTS as usize];
        let selected = &mut storage[..self.parts.len()];
        selected.copy_from_slice(&self.parts);
        for (index, part) in selected.iter_mut().enumerate() {
            if index < bone || index >= end {
                part.flags.0 |= PartFlags::LOCKED;
            }
        }
        select_motion(
            if blending { &self.blend_tree } else { tree },
            selected,
            &motion.animation,
            motion.flags.bone_mask(),
            motion.remap.as_ref().map(MotionRemap::view),
        )?;
        let mut target = if blending {
            std::mem::take(&mut self.blend_tree)
        } else {
            std::mem::take(tree)
        };
        self.reset_pose_range(&mut target, blending, bone, reset_end);
        attach_motion_remapped(
            &mut target,
            selected,
            &motion.animation,
            motion.flags.bone_mask(),
            motion.remap.as_ref().map(MotionRemap::view),
        )?;
        target.req_anim_all_by_flags(joint, 1, self.frame);
        configure_aobjs(
            &mut target,
            joint,
            self.speed,
            self.flags.contains(MotionFlags::LOOP),
        );
        if blending {
            self.blend_tree = target;
            tree.req_anim_all_by_flags(joint, 1, self.frame);
            configure_aobjs(
                tree,
                joint,
                self.speed,
                self.flags.contains(MotionFlags::LOOP),
            );
            self.blend_tree.anim_all::<T>(joint);
        } else {
            *tree = target;
        }
        let mut callback = AObjEndCallback::default();
        for part in &self.parts[bone..end] {
            if part.flags.eligible() {
                tree.anim::<T>(part.joint, &mut callback);
            }
        }
        Ok(())
    }

    /// `ftAnim_SetAnimRate` (0x8006F190) / `ftAnim_8006F0FC`: defer into
    /// fp+0x8A0 while Fighter.x2223_b0 is set, otherwise change both trees.
    pub fn set_rate(&mut self, tree: &mut JObjTree, rate: f32, deferred: bool) {
        if deferred {
            self.saved_speed = rate;
            return;
        }
        configure_aobjs(tree, self.root, rate, false);
        configure_aobjs(&mut self.blend_tree, self.root, rate, false);
        self.speed = rate;
    }

    /// `ftAnim_8006FA58` / `ftAnim_8006FB88`: reset the descriptor bones
    /// below TopN, preserving locked rotations and independently animated parts.
    pub fn reset_pose(&self, target: &mut JObjTree, blending: bool) {
        self.reset_pose_range(target, blending, 1, self.parts.len());
    }

    /// `ftAnim_8006FA58` (8006FA58) from part `bone` when no motion is
    /// attached: the descriptor pose returns to its subtree, and to its
    /// following siblings, which ftAnim_GetNextJointInTree walks on into.
    pub fn reset_subtree_pose(&self, tree: &mut JObjTree, bone: usize) {
        let depth = self.parts[bone].depth;
        let end = (bone + 1..self.parts.len())
            .find(|&i| self.parts[i].depth < depth)
            .unwrap_or(self.parts.len());
        self.reset_pose_range(tree, false, bone, end);
    }

    fn reset_pose_range(&self, target: &mut JObjTree, blending: bool, start: usize, end: usize) {
        for (index, part) in self.parts.iter().enumerate().take(end).skip(start) {
            if part.motion_mask != 0 {
                continue;
            }
            let locked = part.flags.contains(PartFlags::LOCKED);
            if !locked && part.flags.contains(PartFlags::PART_ANIMATION) {
                if Some(index) == self.translation_joint {
                    let reciprocal = 1.0 / self.model_scale;
                    target.set_scale(part.joint, &Vec3::new(reciprocal, reciprocal, reciprocal));
                }
                continue;
            }
            let rest = self.rest_pose.get(part.joint);
            let joint = target.get_mut(part.joint);
            joint.translate = rest.translate;
            if Some(index) != self.translation_joint {
                joint.scale = rest.scale;
            }
            if !locked {
                joint.rotate.x = rest.rotate.x;
                joint.rotate.y = rest.rotate.y;
                joint.rotate.z = rest.rotate.z;
            }
            if !locked || blending {
                target.clear_flags(part.joint, JOBJ_USE_QUATERNION);
            }
            if target.flags(part.joint) & JOBJ_MTX_INDEP_SRT == 0 {
                target.set_mtx_dirty(part.joint);
            }
            if Some(index) == self.translation_joint {
                // ftCommon_8007F6A4: fdivs, no multiply-add.
                let reciprocal = 1.0 / self.model_scale;
                target.set_scale(part.joint, &Vec3::new(reciprocal, reciprocal, reciprocal));
            }
        }
    }

    /// `ftAnim_8006EBA4`: main playback, command callback, then part blends.
    /// The command interpreter (`ftAction_80073240`, T6) and accessory update
    /// (`ftCo_800DB500`) have explicit hooks in `step_with_hooks`.
    pub fn step<T: InverseTrig>(&mut self, tree: &mut JObjTree) {
        self.step_with_hooks::<T>(tree, |_, _| {}, |_, _| {});
    }

    /// `step` with retail trigonometry, compiled once here: callers in other
    /// crates (the savestate importer) reuse it instead of instantiating the
    /// whole generic playback path again.
    #[inline(never)]
    pub fn step_retail(&mut self, tree: &mut JObjTree) {
        // `step` with no command or accessory hooks.
        self.advance_main::<crate::fighter::RetailTrig>(tree);
        self.advance_parts::<crate::fighter::RetailTrig>(tree);
    }

    pub fn step_with_hooks<T: InverseTrig>(
        &mut self,
        tree: &mut JObjTree,
        commands: impl FnOnce(&mut Self, &mut JObjTree),
        accessories: impl FnOnce(&mut Self, &mut JObjTree),
    ) {
        self.advance_main::<T>(tree);
        commands(self, tree);
        self.advance_parts::<T>(tree);
        accessories(self, tree);
    }

    /// `ftAnim_8006E9B4`: blend timing precedes both skeleton evaluations;
    /// current frame is read only after the pose has been transferred.
    pub(crate) fn advance_main<T: InverseTrig>(&mut self, tree: &mut JObjTree) {
        if self.motion_id == -1 {
            return;
        }
        if self.blend_duration == 0.0 {
            tree.clear_flags_all(self.root, JOBJ_USE_QUATERNION);
            if self.flags.contains(MotionFlags::ROOT_MOTION) {
                self.root_motion
                    .as_mut()
                    .expect("root-motion bone setup required")
                    .animate::<T>(tree, self.root, self.flags, self.model_scale);
            } else {
                animate_parts::<T>(tree, &self.parts, 0);
            }
        } else {
            let rate = first_aobj(&self.blend_tree, self.root).map_or(0.0, |a| a.framerate);
            let (weight, inverse) =
                advance_blend(self.blend_duration, &mut self.blend_progress, rate);
            animate_parts::<T>(tree, &self.parts, 0);
            if self.flags.contains(MotionFlags::ROOT_MOTION) {
                self.root_motion
                    .as_mut()
                    .expect("root-motion bone setup required")
                    .animate_blend::<T>(
                        &mut self.blend_tree,
                        tree,
                        self.root,
                        self.flags,
                        self.model_scale,
                    );
            } else {
                self.blend_tree.anim_all::<T>(self.root);
            }
            for part in self.parts.iter().skip(1).filter(|p| p.flags.eligible()) {
                let source = self.blend_tree.get(part.joint);
                if inverse == 0.0 || part.flags.contains(PartFlags::COPY) {
                    copy_pose(source, tree, part.joint);
                } else {
                    blend_pose::<T>(source, tree, part.joint, weight, inverse);
                }
            }
        }
        let frame = if self.blend_duration == 0.0 {
            self.current_aobj(tree)
                .expect("active motion has no eligible AObj")
                .curr_frame
        } else {
            // lbGetJObjCurrFrame returns zero for a tree without an AObj.
            self.current_aobj(tree).map_or(0.0, |a| a.curr_frame)
        };
        // retail 0x8006EB70/0x8006EB74: two fadds, not a modulus or an
        // accumulation of the post-wrap frame. Preserve the nested rounding.
        if self.flags.contains(MotionFlags::ACCUMULATE_LOOPS) && frame < self.frame {
            self.remainder += self.frame + self.speed;
        }
        self.frame = frame;
    }

    /// `ftAnim_800707B0`: independent part blends run even without a main motion.
    pub fn advance_parts<T: InverseTrig>(&mut self, tree: &mut JObjTree) {
        for animation in &mut self.part_animations {
            if animation.current == -1 {
                continue;
            }
            let (weight, inverse) =
                advance_blend(animation.duration, &mut animation.progress, animation.rate);
            for &index in animation.joints.iter() {
                let part = &self.parts[index];
                if part.flags.contains(PartFlags::PART_ANIMATION) {
                    let source = self.blend_tree.get(part.joint);
                    if inverse == 0.0 {
                        copy_pose(source, tree, part.joint);
                    } else {
                        blend_pose::<T>(source, tree, part.joint, weight, inverse);
                    }
                }
            }
        }
    }

    /// `ftAnim_8006F3DC`: first eligible main AObj, or first blend-tree AObj.
    pub fn current_aobj<'a>(&'a self, tree: &'a JObjTree) -> Option<&'a AObj> {
        if self.blend_duration != 0.0 {
            first_aobj(&self.blend_tree, self.root)
        } else {
            self.parts
                .iter()
                .filter(|p| p.flags.eligible())
                .find_map(|p| tree.get(p.joint).aobj.as_ref())
        }
    }

    /// `ftAnim_IsFramesRemaining` (0x8006F238), using AOBJ_NO_ANIM, not
    /// a frame comparison: first play, zero rate, loops and overshoot matter.
    pub fn frames_remaining(&self, tree: &JObjTree) -> bool {
        let active = if self.blend_duration == 0.0 {
            tree
        } else {
            &self.blend_tree
        };
        self.parts.iter().filter(|p| p.flags.eligible()).any(|p| {
            active
                .get(p.joint)
                .aobj
                .as_ref()
                .is_some_and(|a| a.flags & AOBJ_NO_ANIM == 0)
        })
    }

    /// Animation-only `ftCo_Wait_Anim` (0x8008A494) -> `ftCo_8008A7A8`.
    /// Run after `step` in Fighter_8006A360 (s_link 1). Passing None models
    /// the null table / disallowed held-item branch, which consumes no RNG.
    /// State-machine transitions and scripts remain caller-owned (T10/T6).
    pub fn update_wait<'a, T: InverseTrig>(
        &mut self,
        tree: &mut JObjTree,
        rng: &mut HsdRng,
        table: Option<&[WaitEntry]>,
        motion: impl FnOnce(i32) -> &'a Motion,
    ) -> Result<Option<WaitChoice>, AttachError> {
        self.update_wait_with_restart(tree, rng, table, motion, |state, tree| {
            state.step::<T>(tree);
        })
    }

    /// T6 integration variant: after attachment, `restart_and_step` resets
    /// the selected motion's command stream/dynamics and immediately calls
    /// `step_with_hooks`. This keeps first-play command dispatch inside the
    /// restart, as in ftCo_8008A6D8, instead of delaying it to the next tick.
    pub fn update_wait_with_restart<'a>(
        &mut self,
        tree: &mut JObjTree,
        rng: &mut HsdRng,
        table: Option<&[WaitEntry]>,
        motion: impl FnOnce(i32) -> &'a Motion,
        restart_and_step: impl FnOnce(&mut Self, &mut JObjTree),
    ) -> Result<Option<WaitChoice>, AttachError> {
        if self.frames_remaining(tree) {
            return Ok(None);
        }
        let choice = table.map_or(
            WaitChoice {
                motion: self.motion_id,
                draws: 0,
            },
            |table| choose_wait_animation(rng, table, self.motion_id),
        );
        if choice.motion != -1 {
            self.set_animation(tree, motion(choice.motion), 0.0, 1.0)?;
            restart_and_step(self, tree);
        }
        Ok(Some(choice))
    }
}

/// `ftAnim_8006E7B8`: eligible joints inside the selected preorder depth.
pub fn animate_parts<T: InverseTrig>(
    tree: &mut JObjTree,
    parts: &[AnimationPart],
    start: usize,
) -> AObjEndCallback {
    let mut cb = AObjEndCallback::default();
    let depth = parts[start].depth;
    for (index, part) in parts.iter().enumerate().skip(start) {
        if !part.flags.contains(PartFlags::PRESENT) {
            continue;
        }
        if index != start && part.depth <= depth {
            break;
        }
        if part.flags.eligible() {
            tree.anim::<T>(part.joint, &mut cb);
        }
    }
    cb
}

fn first_aobj(tree: &JObjTree, root: JObjId) -> Option<&AObj> {
    tree.depth_first(root)
        .find_map(|id| tree.get(id).aobj.as_ref())
}

/// HSD_ForeachAnim's reachable JObj and material AObjs. TObj/PObj animation
/// is not represented by hsd-anim yet; preserve that engine boundary.
fn configure_aobjs(tree: &mut JObjTree, root: JObjId, rate: f32, set_loop: bool) {
    let visit = |aobj: &mut AObj| {
        if set_loop {
            aobj.set_flags(AOBJ_LOOP);
        }
        aobj.set_rate(rate);
    };
    let mut next = Some(root);
    while let Some(id) = next {
        next = tree.next_depth_first(id);
        let joint = tree.get_mut(id);
        if let Some(aobj) = &mut joint.aobj {
            visit(aobj);
        }
        for dobj in &mut joint.dobj {
            if let Some(aobj) = &mut dobj.aobj {
                visit(aobj);
            }
            if let Some(aobj) = dobj.mobj.as_mut().and_then(|m| m.aobj.as_mut()) {
                visit(aobj);
            }
        }
    }
}

impl FighterAnimation {
    /// ftCo_80091E78 (80091E78) composes ftAnim_8006F4C8/FB88/70108/FE9C.
    /// Pose sources are owned costume and shield descriptors, never render caches.
    /// Neutral descriptor blending is lb_8000C868: its six fused SRT sites
    /// 8000C8A8/C8BC/C8D0/C8E4/C8F8/C90C and quaternion ordering match
    /// blend_pose for an Euler source. Copy is lb_8000B4FC (no fused sites).
    pub fn apply_guard_pose<T: InverseTrig>(
        &mut self,
        tree: &mut JObjTree,
        tilt: &Motion,
        neutral: &[hsd_anim::jobj::JObj],
        magnitude: f32,
        frame: f32,
        weight: f32,
    ) -> Result<(), AttachError> {
        if magnitude != 0.0 {
            attach_motion_remapped(
                &mut self.blend_tree,
                &self.parts,
                &tilt.animation,
                self.flags.bone_mask(),
                None,
            )?;
            self.blend_tree.req_anim_all(self.root, frame);
            // ftAnim_8006FB88 resets SRT while preserving the newly attached AObjs.
            let mut reset = std::mem::take(&mut self.blend_tree);
            self.reset_pose(&mut reset, true);
            self.blend_tree = reset;
            self.blend_tree.anim_all::<T>(self.root);
        }
        let mut pose_index = 0;
        for (index, part) in self.parts.iter().enumerate().skip(1) {
            if part.motion_mask != 0 {
                continue;
            }
            let source = neutral.get(pose_index).expect("shield pose mapping");
            pose_index += 1;
            if magnitude == 0.0 && weight >= 1.0 {
                // ftAnim_8006FA58: a full neutral pose also restores S/T
                // on dynamics-owned joints; their rotation remains owned.
                let locked = part.flags.contains(PartFlags::LOCKED);
                let translation = Some(index) == self.translation_joint;
                if locked || !part.flags.contains(PartFlags::PART_ANIMATION) {
                    let joint = tree.get_mut(part.joint);
                    joint.translate = source.translate;
                    if !translation {
                        joint.scale = source.scale;
                    }
                    if !locked {
                        // lb_8000B4FC / B5DC preserve the unused rotation W.
                        joint.rotate.x = source.rotate.x;
                        joint.rotate.y = source.rotate.y;
                        joint.rotate.z = source.rotate.z;
                        tree.clear_flags(part.joint, JOBJ_USE_QUATERNION);
                    }
                    if tree.flags(part.joint) & JOBJ_MTX_INDEP_SRT == 0 {
                        tree.set_mtx_dirty(part.joint);
                    }
                }
                if translation {
                    // ftCommon_8007F6A4, also called for part-animation ownership.
                    let reciprocal = 1.0 / self.model_scale;
                    tree.set_scale(part.joint, &Vec3::new(reciprocal, reciprocal, reciprocal));
                }
                continue;
            }
            if !part.flags.eligible() {
                continue;
            }
            if magnitude != 0.0 {
                if magnitude < 1.0 {
                    if part.flags.contains(PartFlags::COPY) {
                        copy_pose(source, &mut self.blend_tree, part.joint);
                    } else {
                        blend_pose::<T>(
                            source,
                            &mut self.blend_tree,
                            part.joint,
                            1.0 - magnitude,
                            magnitude,
                        );
                    }
                }
                let tilted = self.blend_tree.get(part.joint);
                if weight >= 1.0 || part.flags.contains(PartFlags::COPY) {
                    copy_pose(tilted, tree, part.joint);
                } else {
                    blend_pose::<T>(tilted, tree, part.joint, weight, 1.0 - weight);
                }
            } else if weight >= 1.0 || part.flags.contains(PartFlags::COPY) {
                copy_pose(source, tree, part.joint);
            } else {
                blend_pose::<T>(source, tree, part.joint, weight, 1.0 - weight);
            }
        }
        Ok(())
    }
}
