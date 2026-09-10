//! Forward smash and charge timing, ftCo_AttackS4.c / ft_0DF0.c.
use super::FighterCore;
use super::{
    assets::{FighterAssets, Result},
    CharacterCallbacks, Fighter, MotionData,
};
use crate::input::Buttons;
use melee_types::CommonMotionState as S;

#[derive(Clone, Copy, Debug)]
pub enum ChargePhase {
    PreCharge,
    Charging,
    Release,
}
#[derive(Clone, Copy, Debug)]
pub struct SmashCharge {
    pub phase: ChargePhase,
    pub frames: f32,
    pub maximum_frames: f32,
    pub maximum_multiplier: f32,
    pub saved_rate: f32,
    pub color_animation: u8,
}
impl SmashCharge {
    /// ftCo_800DEEB8: retail 800DEEDC fmadds, then fmuls.
    pub fn scale_damage(&self, damage: f32) -> f32 {
        if !matches!(self.phase, ChargePhase::Release) {
            return damage;
        }
        damage
            * gekko_math::fma::fmadds(
                self.maximum_multiplier - 1.0,
                self.frames / self.maximum_frames,
                1.0,
            )
    }
}
impl<C: CharacterCallbacks> Fighter<C> {
    /// doEnter (8008C3E0), ftCo_AttackS4.c; no fused sites in this unit.
    pub(super) fn enter_forward_smash(&mut self, assets: &FighterAssets) -> Result<()> {
        self.character.forward_smash_variant();
        if self.core.input.current.stick.y != 0.0 || self.core.input.current.cstick.y != 0.0 {
            unimplemented!("ftCo_AttackS4.c doEnter: angled smash");
        }
        let x = if self.core.input.current.cstick.x != 0.0 {
            self.core.input.current.cstick.x
        } else {
            self.core.input.current.stick.x
        };
        self.core.physics.facing = if x >= 0.0 { 1.0 } else { -1.0 };
        self.core.commands.variables[0] = 0;
        self.core.commands.grab_release = false;
        self.core.commands.throw_reverse = false;
        self.change_motion_state(S::AttackS4S, assets)?;
        self.step_animation(assets);
        self.core.state_data = MotionData::Smash;
        self.core.status.interaction = super::Interaction::Attack;
        Ok(())
    }
}
impl FighterCore {
    /// ftCo_800DEF38 (800DEF38): charging advances before the state's Anim.
    pub(super) fn advance_smash_charge(&mut self, assets: &FighterAssets) {
        let Some(charge) = &mut self.commands.smash_charge else {
            return;
        };
        if matches!(charge.phase, ChargePhase::Charging) {
            charge.frames += 1.0;
            if charge.frames == 1.0 {
                self.commands
                    .graphics
                    .push(assets.charge_start_graphics[&charge.color_animation].clone());
            }
            if charge.frames > 1.0 {
                unimplemented!("ftCo_800DEF38: sustained charge shake and sound");
            }
        }
    }
    /// ftCo_800DF0D0 (800DF0D0): hold/release before the state's IASA.
    pub(super) fn update_smash_charge_input(&mut self) {
        let Some(charge) = &mut self.commands.smash_charge else {
            return;
        };
        match charge.phase {
            ChargePhase::PreCharge => {
                if self.input.current.held.intersects(Buttons::A) {
                    charge.phase = ChargePhase::Charging;
                    charge.saved_rate = self.animation.speed;
                    self.animation.set_rate(&mut self.skeleton, 0.0, false);
                    if charge.color_animation != 0x7B {
                        self.commands.color_animations.push(
                            super::commands::ColorAnimationRequest {
                                id: charge.color_animation,
                                duration: 0,
                            },
                        );
                    }
                } else {
                    self.commands.smash_charge = None;
                }
            }
            ChargePhase::Charging if !self.input.current.held.intersects(Buttons::A) => {
                charge.phase = ChargePhase::Release;
                self.animation
                    .set_rate(&mut self.skeleton, charge.saved_rate, false);
            }
            _ => {}
        }
    }
}
