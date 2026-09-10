//! Forward smash and charge timing, ftCo_AttackS4.c / ft_0DF0.c.
use super::FighterCore;
use super::{
    assets::{FighterAssets, Result},
    CharacterCallbacks, Fighter, MotionData,
};
use crate::input::Buttons;
use melee_types::CommonMotionState as S;

use melee_cmd::ChargePhase;
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
                self.combat.charge_overlay = ChargeOverlay::default();
            }
            let overlay = &mut self.combat.charge_overlay;
            overlay.step(
                &assets.charge_overlays[&charge.color_animation],
                &mut self.commands.graphics,
            );
            if !overlay.sound_played && charge.frames >= assets.attacks.charge_sound_frame {
                self.commands
                    .footstep_sounds
                    .push(super::commands::FootstepSound {
                        channel: super::commands::SoundChannel::Ordinary,
                        id: 0x7B,
                        volume: 127,
                        pan: 64,
                    });
                overlay.sound_played = true;
            }
            if charge.frames >= charge.maximum_frames {
                charge.frames = charge.maximum_frames;
                charge.phase = ChargePhase::Release;
                self.animation
                    .set_rate(&mut self.skeleton, charge.saved_rate, false);
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
                        self.commands
                            .color_animations
                            .push(melee_cmd::ColorAnimationRequest {
                                id: charge.color_animation,
                                duration: 0,
                            });
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

/// lb_80014258 / ft_800BFF34: decoded charge-overlay program.
#[derive(Clone, Debug)]
pub enum OverlayCommand {
    Color { rgba: u32, frames: u32 },
    Graphics(melee_types::combat::GraphicsCommand),
    Wait(u32),
    Goto(usize),
    End,
}
#[derive(Clone, Debug, Default)]
pub struct ChargeOverlay {
    instruction: usize,
    timer: u32,
    /// Renderer-facing target and blend duration; no gameplay depends on color.
    pub color: Option<(u32, u32)>,
    sound_played: bool,
}
impl ChargeOverlay {
    fn step(
        &mut self,
        script: &[OverlayCommand],
        graphics: &mut melee_types::fixed::FixedVec<
            melee_types::combat::GraphicsCommand,
            { melee_ef::request::REQUEST_CAPACITY },
        >,
    ) {
        self.timer = self.timer.saturating_sub(1);
        while self.timer == 0 {
            match &script[self.instruction] {
                OverlayCommand::Color { rgba, frames } => self.color = Some((*rgba, *frames)),
                OverlayCommand::Graphics(command) => graphics.push(command.clone()),
                OverlayCommand::Wait(frames) => self.timer = *frames,
                OverlayCommand::Goto(target) => {
                    self.instruction = *target;
                    continue;
                }
                OverlayCommand::End => break,
            }
            self.instruction += 1;
        }
    }
}
/// Archive-only adapter for the color overlay vocabulary (lb_013B.c).
pub fn read_overlay(archive: &hsd_archive::Archive, entry: u32) -> Result<Vec<OverlayCommand>> {
    let r = archive.reader();
    let mut commands = Vec::new();
    let mut offsets = Vec::new();
    let mut offset = entry;
    loop {
        offsets.push(offset);
        let word = r.u32(offset)?;
        let opcode = word >> 26;
        let command = match opcode {
            7 => {
                let target = archive.link(offset + 4)?.ok_or("overlay goto")?;
                commands.push(OverlayCommand::Goto(
                    offsets
                        .iter()
                        .position(|&p| p == target)
                        .ok_or("overlay forward goto")?,
                ));
                break;
            }
            10 => {
                commands.push(OverlayCommand::End);
                break;
            }
            11 => OverlayCommand::Wait(word & 0x03ff_ffff),
            18 | 19 => {
                let rgba = r.u32(offset + 4)?;
                offset += 4;
                OverlayCommand::Color {
                    rgba,
                    frames: if opcode == 18 { 0 } else { word & 0x03ff_ffff },
                }
            }
            21 => {
                let words = [
                    word,
                    r.u32(offset + 4)?,
                    r.u32(offset + 8)?,
                    r.u32(offset + 12)?,
                    r.u32(offset + 16)?,
                ];
                offset += 16;
                OverlayCommand::Graphics(melee_cmd::decode::graphics(&words))
            }
            _ => return Err(format!("unported charge overlay opcode {opcode}").into()),
        };
        commands.push(command);
        offset += 4;
    }
    Ok(commands)
}
