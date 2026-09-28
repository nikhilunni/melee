//! Marth and Roy's response to stage wind, ftCo_8009E614 (0x8009E614) and
//! ftCo_8009E4A8 (0x8009E4A8), ftdynamics.c:525-583. When the dynamics
//! field pool (lb_800115F4) first reports wind, every dynamic bone goes to
//! the solver and the motion is reattached at the current frame without a
//! blend; on the first calm tick the motion's own ownership comes back.
use super::assets::{CommonBehavior, FighterAssets};
use super::FighterCore;
use crate::anim::playback::MotionFlags;
use melee_lb::radial_force::WindState;

impl FighterCore {
    /// ftCo_8009E614, run by ftCo_8009DD94 after ftColl_8007AF60.
    pub(super) fn respond_to_stage_wind(&mut self, assets: &FighterAssets) {
        // The frozen flag (x2227_b6, ftCo_DamageIce) is not modelled.
        if !CommonBehavior::for_kind(assets.kind).stage_wind_dynamics {
            return;
        }
        match self.stage_wind {
            WindState::Stopped => self.restore_motion_dynamics(assets),
            WindState::Started => self.release_dynamics_to_wind(assets),
            WindState::Calm | WindState::Blowing => {}
        }
    }

    /// ftdynamics.c:565-581.
    fn release_dynamics_to_wind(&mut self, assets: &FighterAssets) {
        let flags = self.animation.flags;
        if !flags.contains(MotionFlags::DYNAMICS_TABLE)
            && !flags.contains(MotionFlags::DYNAMICS_OFF)
        {
            return;
        }
        for (set, first) in self.dynamics.iter_mut().zip(&mut self.dynamics_first_bone) {
            // ftCo_8009CB40(fp, i, 1, NULL): the chain from its first bone.
            *first = 0;
            crate::dynamics::select(set, &mut self.skeleton, &mut self.animation.parts, true, 0);
        }
        if self.animation.motion_id < 0 {
            return;
        }
        // ftAnim_8006EBE8(gobj, cur_anim_frame, frame_speed_mul, 0.0), then
        // ftAnim_8006E9B4: the blend ends and the pose is re-evaluated.
        let motion = &assets.motions[&self.animation.motion_id];
        let (frame, speed) = (self.animation.frame, self.animation.speed);
        self.animation
            .set_animation_remapped(
                &mut self.skeleton,
                motion,
                frame,
                speed,
                motion.remap.as_ref().map(|remap| remap.view()),
                Some(0.0),
            )
            .expect("wind motion reattachment");
        self.animation
            .advance_main::<super::RetailTrig>(&mut self.skeleton);
    }

    /// ftCo_8009E4A8 (0x8009E4A8): the motion's table row (x594_b4) or no
    /// dynamics (x594_b3), each set's subtree reattached at the current frame.
    fn restore_motion_dynamics(&mut self, assets: &FighterAssets) {
        let flags = self.animation.flags;
        let motion_id = self.animation.motion_id;
        let starts: Option<Vec<u32>> = if flags.contains(MotionFlags::DYNAMICS_TABLE) {
            let dynamics = &assets.dynamics_motion_starts[&motion_id];
            if !dynamics.table_row {
                return;
            }
            Some(dynamics.starts.clone())
        } else if flags.contains(MotionFlags::DYNAMICS_OFF) {
            None
        } else {
            return;
        };
        for index in 0..self.dynamics.len() {
            let (enabled, first) = match &starts {
                Some(starts) => (true, starts[index]),
                None => (false, 0x100),
            };
            self.dynamics_first_bone[index] = first;
            crate::dynamics::select(
                &mut self.dynamics[index],
                &mut self.skeleton,
                &mut self.animation.parts,
                enabled,
                if enabled { first as usize } else { 0 },
            );
            if motion_id >= 0 {
                let bone = assets.bones.dynamics_roots[index] as usize;
                self.animation
                    .resume_dynamic_subtree::<super::RetailTrig>(
                        &mut self.skeleton,
                        bone,
                        &assets.motions[&motion_id],
                    )
                    .expect("dynamic subtree animation");
            }
        }
    }
}
