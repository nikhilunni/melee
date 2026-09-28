//! Air dodge, ftCommon/ftCo_EscapeAir.c. The script controls hurt status and
//! the switch from multiplicative decay to ordinary aerial physics.
use super::{
    assets::{FighterAssets, Result},
    Fighter, MotionData,
};
use gekko_math::msl::{cosf, fabsf, sinf};
use hsd_archive::Archive;
use hsd_types::{Vec2, Vec3};
use melee_types::CommonMotionState;

#[derive(Clone, Copy, Debug)]
pub struct AirDodgeParameters {
    pub deadzone: Vec2,
    pub item_throw_frames: i32,
    pub force: f32,
    pub decay: f32,
    pub special_fall_mobility: f32,
    pub landing_lag: f32,
}
impl AirDodgeParameters {
    /// PlCo ftCommonData +32C..344, read at the archive boundary.
    pub fn read(archive: &Archive, base: u32) -> Result<Self> {
        let r = archive.reader();
        Ok(Self {
            deadzone: Vec2::new(r.f32(base + 0x32C)?, r.f32(base + 0x330)?),
            item_throw_frames: r.s32(base + 0x334)?,
            force: r.f32(base + 0x338)?,
            decay: r.f32(base + 0x33C)?,
            special_fall_mobility: r.f32(base + 0x340)?,
            landing_lag: r.f32(base + 0x344)?,
        })
    }
}
/// mv.co.escapeair: item-throw timer and momentum restored on item throw.
#[derive(Clone, Debug)]
pub struct AirDodgeState {
    pub item_throw_frames: i32,
    pub saved_velocity: Vec3,
}
impl Fighter {
    /// ftCo_80099A9C (80099A9C): retain momentum, then select the stick direction.
    pub(super) fn enter_air_dodge(&mut self, assets: &FighterAssets) -> Result<()> {
        let p = assets.air_dodge;
        let saved_velocity = self.core.physics.self_velocity;
        let stick = self.core.input.current.stick;
        if fabsf(stick.x) < p.deadzone.x && fabsf(stick.y) < p.deadzone.y {
            self.core.physics.self_velocity.x = 0.0;
            self.core.physics.self_velocity.y = 0.0;
        } else {
            // ftCommon_8007D9D4; retail 80099B4C/B64: separate fmuls.
            let angle = melee_lb::trigf::atan2f(stick.y, stick.x);
            self.core.physics.self_velocity.x = p.force * cosf(angle);
            self.core.physics.self_velocity.y = p.force * sinf(angle);
        }
        self.core.commands.variables[0] = 0;
        self.core.state_data = MotionData::EscapeAir(AirDodgeState {
            item_throw_frames: p.item_throw_frames,
            saved_velocity,
        });
        self.change_motion_state(CommonMotionState::EscapeAir.into(), assets)?;
        self.step_animation(assets);
        Ok(())
    }
    /// ftCo_EscapeAir_Anim (80099BD0), animation completion enters FallSpecial.
    pub(super) fn air_dodge_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        if !self.core.animation.frames_remaining(&self.core.skeleton) {
            self.enter_air_dodge_fall(assets)?;
        }
        Ok(())
    }
    /// ftCo_EscapeAir_IASA (80099C24), item-free path; item interaction is
    /// rejected by Status::require_supported before the saved-momentum throw branch.
    pub(super) fn air_dodge_input(&mut self, assets: &FighterAssets) {
        let MotionData::EscapeAir(dodge) = &mut self.core.state_data else {
            panic!("air dodge scratch missing")
        };
        if dodge.item_throw_frames != 0 {
            dodge.item_throw_frames -= 1;
        }
        // ftCo_800C3B10 last: a tether keeps a tenth of the drift (fmuls).
        if self.try_air_tether(assets) {
            self.core.physics.self_velocity.x *= 0.1;
        }
    }
    /// ftCo_EscapeAir_Phys (80099CEC): separate fmuls; no velocity table.
    pub(super) fn air_dodge_physics(&mut self, assets: &FighterAssets) {
        if self.core.commands.variables[0] == 0 {
            self.core.physics.self_velocity.x *= assets.air_dodge.decay;
            self.core.physics.self_velocity.y *= assets.air_dodge.decay;
        } else {
            self.airborne_physics(assets);
        }
    }
}

impl Fighter {
    /// ftCo_800C3B10 (800C3B10), ftCo_AirCatch.c:54-79: the common tests,
    /// then the kind's tether (Link, Young Link, Samus), which also checks
    /// its own article and enters ftCo_800C3BE8's state. Sets used_tether
    /// when it does.
    pub fn try_air_tether(&mut self, assets: &FighterAssets) -> bool {
        use crate::input::Buttons;
        if self.core.status.used_tether {
            return false;
        }
        let table = self.character.table();
        let hook = table.air_tether;
        if hook.is_none() && !(table.descriptor)().common_behavior.air_dodge_tether {
            return false;
        }
        if self.core.held_item.is_some()
            || !self.core.input.current.held.intersects(Buttons::SHIELD)
            || !self.core.input.pressed.intersects(Buttons::A)
        {
            return false;
        }
        let Some(hook) = hook else {
            unimplemented!("ftCo_AirCatch.c:54-79: {:?}'s tether", self.core.kind);
        };
        if !hook(self, assets) {
            return false;
        }
        self.core.status.used_tether = true;
        true
    }
}
