//! Spot dodge and roll, ftCo_Escape.c. Movement is sampled from TransN tracks.
use super::FighterCore;
use super::{
    assets::{FighterAssets, Result},
    Fighter, MotionData,
};
use gekko_math::msl::fabsf;
use melee_types::CommonMotionState as S;

#[derive(Clone, Debug)]
pub struct EscapeState {
    /// Guard scratch survives in the retail union during a roll.
    pub retained_guard: Option<super::shield::GuardState>,
    /// mv.co.escape.x0: item-throw window, decremented by ftCo_8009563C.
    /// C calls this bool, but retail preserves PlCo.x324 (five frames).
    pub interrupt_frames: i32,
    /// facing_dir1, retained when the subaction reverses facing_dir.
    pub entry_facing: f32,
    /// mv+4 as the predecessor left it: ftCo_80099314 writes only
    /// mv.co.escape.x0 (Samus's ftCo_80099390 also clears x4 and has its
    /// own entry). Out of Guard this is the shield's smoothed tilt
    /// (`None` where the port does not model the predecessor's word).
    pub retained_word: Option<f32>,
}
impl Fighter {
    /// ftCo_80099314 / ftCo_800998EC (0x80099314 / 0x800998EC).
    pub fn enter_escape(&mut self, assets: &FighterAssets, state: S) -> Result<()> {
        let retained_guard = if let MotionData::Guard(guard) = &self.core.state_data {
            Some(guard.clone())
        } else {
            None
        };
        let retained_word = self.inherited_scratch_word();
        if state == S::EscapeN {
            (self.character.table().escape_variant)(self, assets, false)?;
        } else if let Some(prepare) = self.character.table().prepare_roll {
            prepare(self);
        }
        self.core.commands.grab_release = false;
        self.change_motion_state(state.into(), assets)?;
        self.step_animation(assets);
        self.core.status.ignore_fighter_nudge = true;
        self.core.state_data = MotionData::Escape(EscapeState {
            retained_guard,
            interrupt_frames: assets.shield.roll_interrupt_frames,
            entry_facing: self.core.physics.facing,
            retained_word,
        });
        if state != S::EscapeN {
            (self.character.table().escape_variant)(self, assets, true)?;
        }
        Ok(())
    }
    /// ftCo_Escape_Anim / ftCo_EscapeN_Anim (0x800994D8 / 0x800999D8).
    pub fn escape_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        if self.core.motion_state.id != S::EscapeN
            && std::mem::take(&mut self.core.commands.grab_release)
        {
            self.core.physics.facing = -self.core.physics.facing;
        }
        if !self.core.animation.frames_remaining(&self.core.skeleton) {
            if self.core.motion_state.id != S::EscapeN {
                self.core.physics.ground_velocity = 0.0;
            }
            if let Some(result) = (self.character.table().escape_finished)(self, assets) {
                return result;
            }
            self.change_motion_state(S::Wait.into(), assets)?;
        }
        (self.character.table().escape_animated)(self);
        Ok(())
    }
}
impl FighterCore {
    /// ftCo_8009980C (0x8009980C): main-stick down smash or C-stick down,
    /// unless UCF's hook in the entry ftCo_80099894 (0x800998A4) refuses.
    pub(super) fn spot_dodge_input(&self, assets: &FighterAssets) -> bool {
        ((self.input.current.stick.y <= assets.input.escape_threshold
            && i32::from(self.input.vertical.tilt) < assets.input.escape_window)
            || self.input.current.cstick.y <= assets.input.escape_threshold)
            && !crate::input::controller_fix::blocks_spot_dodge(
                &self.input,
                &crate::input::controller_fix::SpotDodgeFacts {
                    escape_threshold: assets.input.escape_threshold,
                    roll_window: assets.input.roll_window,
                    floor: self.collision.data.floor,
                },
            )
    }
    /// ftCo_8009917C (0x8009917C): main-stick horizontal smash, then C-stick.
    pub fn roll_input(&self, assets: &FighterAssets) -> Option<S> {
        let p = &assets.shield;
        let x = if fabsf(self.input.current.stick.x) >= p.roll_threshold
            && i32::from(self.input.horizontal.tilt) < p.roll_window
        {
            self.input.current.stick.x
        } else if fabsf(self.input.current.cstick.x) >= p.roll_threshold {
            self.input.current.cstick.x
        } else {
            return None;
        };
        Some(if x * self.physics.facing >= 0.0 {
            S::EscapeF
        } else {
            S::EscapeB
        })
    }
    /// ftCo_Escape_Phys -> ft_80085004 -> ft_80085030 (0x80085030).
    pub(super) fn escape_physics(&mut self, map: &melee_mp::CollMap) {
        use crate::physics::{friction::friction_acceleration, grounded};
        let MotionData::Escape(escape) = &self.state_data else {
            panic!("escape data missing")
        };
        if self
            .animation
            .flags
            .contains(crate::anim::MotionFlags::ROOT_MOTION)
        {
            let offset = self
                .animation
                .root_motion
                .as_ref()
                .expect("escape root motion")
                .primary_history
                .offset
                .z;
            // retail 8008505C fmsubs, facing_dir1 rather than current facing.
            self.physics.ground_acceleration =
                gekko_math::fma::fmsubs(offset, escape.entry_facing, self.physics.ground_velocity);
        } else {
            self.physics.ground_acceleration = friction_acceleration(
                self.physics.ground_velocity,
                self.attributes.ground.ground_friction,
            );
        }
        grounded::apply_ground_movement(
            &mut self.physics,
            self.collision.data.floor.normal,
            map.floor_speed_scale(&self.collision.data),
        );
    }
}
