//! Forward smash and charge timing, ftCo_AttackS4.c / ft_0DF0.c.
use super::FighterCore;
use super::{
    assets::{FighterAssets, Result},
    Fighter, MotionData,
};
use crate::input::{pad::Stick, Buttons};
use melee_types::CommonMotionState as S;

use melee_cmd::ChargePhase;
impl Fighter {
    /// ftCo_AttackS4_8008C114: Dash uses facing, without the stick-age check.
    pub(super) fn try_dash_forward_smash(&mut self, assets: &FighterAssets) -> Result<bool> {
        let threshold = assets.input.thresholds.dash_smash_stick_threshold;
        let input = &self.input;
        let forward = input.pressed.intersects(Buttons::A)
            && input.current.stick.x * self.physics.facing >= threshold;
        let cstick = gekko_math::msl::fabsf(input.previous.cstick.x) < threshold
            && gekko_math::msl::fabsf(input.current.cstick.x) >= threshold;
        if forward || cstick {
            let stick = if forward {
                input.current.stick
            } else {
                input.current.cstick
            };
            let facing = if forward {
                self.physics.facing
            } else if stick.x >= 0.0 {
                1.0
            } else {
                -1.0
            };
            self.enter_directed_forward_smash(assets, stick, facing)?;
            return Ok(true);
        }
        Ok(false)
    }
    /// ftCo_AttackS4_CheckInput (8008BFC4): qualifying main-stick+A wins
    /// over C-stick, even when both are held in opposing directions.
    pub(super) fn enter_forward_smash(&mut self, assets: &FighterAssets) -> Result<()> {
        let input = &self.input;
        let t = &assets.input.thresholds;
        let main = input.pressed.intersects(Buttons::A)
            && input.current.stick.x.abs() >= t.dash_smash_stick_threshold
            && i32::from(input.horizontal.tilt) < t.dash_smash_window;
        let stick = if main {
            input.current.stick
        } else {
            input.current.cstick
        };
        let facing = if stick.x >= 0.0 { 1.0 } else { -1.0 };
        self.enter_directed_forward_smash(assets, stick, facing)
    }

    /// doEnter (8008C3E0), ftCo_AttackS4.c: ordered strict angle thresholds
    /// probe authored submotions; absent variants fall back to straight smash.
    fn enter_directed_forward_smash(
        &mut self,
        assets: &FighterAssets,
        stick: Stick,
        facing: f32,
    ) -> Result<()> {
        self.character.forward_smash_variant();
        // ftCo_GetLStickAngle/GetCStickAngle (8007D964/8007D99C).
        let angle = melee_lb::trigf::atan2f(stick.y, stick.x.abs());
        let [high, high_mid, low_mid, low] = assets.attacks.smash_angles;
        let variants = assets.forward_smash_variants;
        let state = if angle > high && variants[0] {
            S::AttackS4Hi
        } else if angle > high_mid && variants[1] {
            S::AttackS4HiS
        } else if angle < low && variants[3] {
            S::AttackS4Lw
        } else if angle < low_mid && variants[2] {
            S::AttackS4LwS
        } else {
            S::AttackS4S
        };
        self.core.physics.facing = facing;
        self.core.commands.variables[0] = 0;
        self.core.commands.grab_release = false;
        self.core.commands.throw_reverse = false;
        // The smash attacks write no mv field, so mv+4 stays the
        // predecessor's.
        let retained_word = self.inherited_scratch_word();
        self.change_motion_state(state.into(), assets)?;
        self.step_animation(assets);
        self.core.state_data = MotionData::Smash { retained_word };
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
            // The charge color program itself runs in ftCo_800C0408's
            // secondary slot, after the primary (`advance_color_overlay`).
            let overlay = &mut self.combat.charge_overlay;
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
                    // ftCo_800DF0D0 installs the charge color (unless 0x7B) in
                    // the secondary slot; `charge_overlay` runs that program.
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
    Color {
        rgba: u32,
        frames: u32,
    },
    Graphics(melee_types::combat::GraphicsCommand),
    Sound(super::commands::FootstepSound),
    Wait(u32),
    Goto(usize),
    Loop(u32),
    LoopEnd,
    ClearColor,
    /// Opcodes 13-17: lighting only, renderer state.
    Light,
    End,
}
#[derive(Clone, Debug, Default)]
pub struct ChargeOverlay {
    instruction: usize,
    loops: melee_types::fixed::FixedVec<(usize, u32), 4>,
    timer: u32,
    /// Renderer-facing target and blend duration; no gameplay depends on color.
    pub color: Option<(u32, u32)>,
    sound_played: bool,
}
impl ChargeOverlay {
    /// lb_80014258's command loop; returns whether the program reached its end.
    /// Without `emit` (ft_800BFF70), effect and sound commands are skipped.
    pub(super) fn step(
        &mut self,
        emit: bool,
        script: &[OverlayCommand],
        graphics: &mut melee_types::fixed::FixedVec<
            melee_types::combat::GraphicsCommand,
            { melee_ef::request::REQUEST_CAPACITY },
        >,
        sounds: &mut melee_types::fixed::FixedVec<
            super::commands::FootstepSound,
            { super::commands::COMMAND_REQUEST_CAPACITY },
        >,
    ) -> bool {
        self.timer = self.timer.saturating_sub(1);
        while self.timer == 0 {
            match &script[self.instruction] {
                OverlayCommand::Color { rgba, frames } => self.color = Some((*rgba, *frames)),
                OverlayCommand::Graphics(command) if emit => graphics.push(command.clone()),
                OverlayCommand::Sound(sound) if emit => sounds.push(sound.clone()),
                OverlayCommand::Graphics(_) | OverlayCommand::Sound(_) => {}
                OverlayCommand::Wait(frames) => self.timer = *frames,
                OverlayCommand::Goto(target) => {
                    self.instruction = *target;
                    continue;
                }
                OverlayCommand::Loop(count) => self.loops.push((self.instruction + 1, *count)),
                OverlayCommand::LoopEnd => {
                    let (target, count) = self.loops.last_mut().expect("overlay loop stack");
                    *count = count.wrapping_sub(1);
                    if *count != 0 {
                        self.instruction = *target;
                        continue;
                    }
                    self.loops.pop();
                }
                OverlayCommand::ClearColor => self.color = None,
                OverlayCommand::Light => {}
                OverlayCommand::End => return true,
            }
            self.instruction += 1;
        }
        false
    }
}
/// Archive-only adapter for the color overlay vocabulary (lb_013B.c).
pub fn read_overlay(archive: &hsd_archive::Archive, entry: u32) -> Result<Vec<OverlayCommand>> {
    read_overlay_inner(archive, entry, 0)
}
fn read_overlay_inner(
    archive: &hsd_archive::Archive,
    entry: u32,
    depth: u8,
) -> Result<Vec<OverlayCommand>> {
    assert!(depth < 8, "color script subroutine depth");
    let r = archive.reader();
    let mut commands = Vec::new();
    let mut offsets = Vec::new();
    let mut offset = entry;
    loop {
        offsets.push(offset);
        let word = r.u32(offset)?;
        let opcode = word >> 26;
        let command = match opcode {
            5 => {
                let target = archive.link(offset + 4)?.ok_or("overlay subroutine")?;
                let mut body = read_overlay_inner(archive, target, depth + 1)?;
                assert!(
                    matches!(body.pop(), Some(OverlayCommand::End)),
                    "overlay subroutine return"
                );
                let base = commands.len();
                for command in &mut body {
                    if let OverlayCommand::Goto(target) = command {
                        *target += base;
                    }
                }
                commands.extend(body);
                offset += 8;
                continue;
            }
            3 => OverlayCommand::Loop(word & 0x03ff_ffff),
            4 => OverlayCommand::LoopEnd,
            12 | 20 => OverlayCommand::ClearColor,
            // lb_80013C18 / lb_80013D68 / lb_80013E3C: light setup reads a
            // second word; lighting has no gameplay effect.
            13..=15 => {
                offset += 4;
                OverlayCommand::Light
            }
            // lb_80013F78 / lb_80013FF0: light rotation and light off.
            16 | 17 => OverlayCommand::Light,
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
            0 | 6 | 10 => {
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
            22 => {
                use super::commands::{FootstepSound, SoundChannel};
                let behavior = (word >> 18) & 255;
                let channel = match behavior {
                    0 => SoundChannel::Ordinary,
                    1 => SoundChannel::Action,
                    2 => SoundChannel::FighterVoice,
                    3 => SoundChannel::Effect,
                    4 => SoundChannel::StatusEffect,
                    6 => SoundChannel::OverrideVoice,
                    _ => return Err(format!("unported overlay sound behavior {behavior}").into()),
                };
                let id = r.u32(offset + 4)?;
                let volume_pan = r.u32(offset + 8)?;
                offset += 8;
                OverlayCommand::Sound(FootstepSound {
                    channel,
                    id,
                    volume: (volume_pan >> 8) as u8,
                    pan: volume_pan as u8,
                })
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
