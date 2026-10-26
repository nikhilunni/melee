//! The Metal Box's metal (ftmetal.c 800C8170..800C8540, ftLib_800871A8,
//! ftchangeparam.c:216-225): heavier, falling faster, until its frames run
//! out or it has taken its damage. The material swap is presentation.

use super::assets::{FighterAssets, Result};
use super::FighterCore;

/// Fighter_804D651C (PlCo `ftLoadCommonData` entry 14): the metal's
/// attribute multipliers (ftCo_800D105C).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MetalMultipliers {
    /// +4: jump_v_initial_velocity, and the multijumps' rises
    /// (ftCo_800D0CBC).
    pub jump: f32,
    /// +8: hop_v_initial_velocity.
    pub hop: f32,
    /// +C / +10 / +14: gravity, terminal_velocity, fast_fall_velocity.
    pub gravity: f32,
    pub terminal_velocity: f32,
    pub fast_fall_velocity: f32,
    /// +18: weight.
    pub weight: f32,
    /// +1C / +20: ledge_jump_vertical_velocity, wall_jump_vertical_velocity.
    pub ledge_jump: f32,
    pub wall_jump: f32,
    /// PlCo +6F0 (metal_armor): knockback a metal fighter shrugs off
    /// (ftCo_Damage_CalcKnockback).
    pub armor: f32,
}
impl MetalMultipliers {
    pub fn read(common: &hsd_archive::Archive, root: u32) -> Result<Self> {
        let table = common.link(root + 14 * 4)?.ok_or("missing metal multipliers")?;
        let r = common.reader();
        let common_data = common.link(root)?.ok_or("missing common data")?;
        Ok(Self {
            armor: r.f32(common_data + 0x6F0)?,
            jump: r.f32(table + 0x4)?,
            hop: r.f32(table + 0x8)?,
            gravity: r.f32(table + 0xC)?,
            terminal_velocity: r.f32(table + 0x10)?,
            fast_fall_velocity: r.f32(table + 0x14)?,
            weight: r.f32(table + 0x18)?,
            ledge_jump: r.f32(table + 0x1C)?,
            wall_jump: r.f32(table + 0x20)?,
        })
    }
}

/// is_metal, metal_timer, metal_health.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MetalState {
    pub metal: bool,
    pub frames: i32,
    pub health: i32,
}

impl FighterCore {
    /// ftLib_800871A8 (800871A8): ftCo_800C8348's timer and health (the
    /// metal begins only once), the attributes again, the coll data's
    /// weight (ft_80081C88) and a small quake. The rumble and sound are
    /// not simulated.
    pub fn become_metal(&mut self, frames: i32, health: i32, assets: &FighterAssets) {
        // ftCo_800C8348 (800C8348).
        self.status.metal.frames = frames;
        self.status.metal.health = health;
        self.status.metal.metal = true;
        self.recompute_attributes(assets);
        self.quake_request = Some(melee_cm::QuakeKind::Small);
    }

    /// ftCo_800C8540 (800C8540): the metal ends; the attributes again.
    pub fn end_metal(&mut self, assets: &FighterAssets) {
        self.status.metal = MetalState::default();
        self.recompute_attributes(assets);
    }

    /// Fighter_8006A360 (fighter.c:1525-1531): the metal's countdown, which
    /// ends it at 0 frames or once its health is spent.
    pub(super) fn tick_metal(&mut self, assets: &FighterAssets) {
        let metal = &mut self.status.metal;
        if !metal.metal || metal.frames == 0 {
            return;
        }
        metal.frames -= 1;
        if metal.frames == 0 || metal.health <= 0 {
            self.end_metal(assets);
        }
    }

    /// ftCo_800D105C (800D105C) and ft_80081C88 at the same scale: the
    /// dat's attributes, the model scale's curves, the metal's
    /// multipliers (separate fmuls each), then the coll data's weight.
    /// The kind's LoadSpecialAttrs recopies attributes the port keeps
    /// unscaled; the multijumps' rises take the metal at the jump
    /// (`metal_jump_scale`).
    pub(super) fn recompute_attributes(&mut self, assets: &FighterAssets) {
        let mut attributes = assets.attributes.clone();
        assets
            .scale_curves
            .scale_attributes(&mut attributes, self.player.scale);
        if self.status.metal.metal {
            let m = &assets.metal;
            attributes.jumping.jump_v_initial_velocity *= m.jump;
            attributes.jumping.hop_v_initial_velocity *= m.hop;
            attributes.air.gravity *= m.gravity;
            attributes.air.terminal_velocity *= m.terminal_velocity;
            attributes.air.fast_fall_velocity *= m.fast_fall_velocity;
            attributes.size.weight *= m.weight;
            attributes.ledge.ledge_jump_vertical_velocity *= m.ledge_jump;
            attributes.wall.wall_jump_vertical_velocity *= m.wall_jump;
        }
        self.attributes = attributes;
        // ft_80081C88: the ECB's sizes follow the unchanged scale; x50 is
        // the weight.
        self.collision.data.x50 = self.attributes.size.weight;
    }

    /// ftCo_800D0CBC (800D0CBC)'s factor on the multijumps' rises: the
    /// metal's +4 (1 otherwise).
    pub fn metal_jump_scale(&self, assets: &FighterAssets) -> f32 {
        if self.status.metal.metal {
            assets.metal.jump
        } else {
            1.0
        }
    }

    /// Fighter_TakeDamage_8006CC7C (fighter.c:2652): while timed, the
    /// metal's health falls with the percent (retail 8006CCC4..CCF4: the
    /// int as a float, fsubs, fctiwz).
    pub(super) fn wear_metal(&mut self, amount: f32) {
        let metal = &mut self.status.metal;
        if metal.frames != 0 {
            metal.health = gekko_math::msl::fctiwz(metal.health as f32 - amount);
        }
    }
}
