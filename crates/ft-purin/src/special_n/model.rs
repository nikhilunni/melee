//! Rollout's pose, sounds, effects and hitbox upkeep shared by its states.
use super::{attributes, scratch};
use crate::init::Jigglypuff;
use gekko_math::msl::fabsf;
use hsd_types::Vec3;
use melee_ft::fighter::{
    commands::{FootstepSound, RumbleRequest, SoundChannel},
    part_rotation::Axis,
    Fighter,
};
use melee_types::GroundOrAir;

/// fp->parts indices the roll poses directly.
mod part {
    /// FtPart_TopN: faces the model along the facing.
    pub const FACING: usize = 0;
    /// FtPart_YRotN: rolls the ball.
    pub const ROLL: usize = 3;
}

/// ftPr_Init_803D05C8 / ftPr_Init_803D05D8: the landing squash's Y and Z
/// scales per step.
const SQUASH_Y: [f32; 4] = [0.65, 0.7, 0.8, 1.0];
const SQUASH_Z: [f32; 4] = [1.1, 1.35, 1.3, 1.2];
/// @227: the roll angle's wrap, in double precision.
const TAU: f64 = std::f64::consts::TAU;
/// @222: M_PI_2 in double precision.
const HALF_PI: f64 = std::f64::consts::FRAC_PI_2;
/// @224: M_PI_2 as a float.
const HALF_PI_F32: f32 = std::f32::consts::FRAC_PI_2;

/// ft_PlaySFX ids.
pub const BOUNCE_SOUND: u32 = 250070;
pub const RELEASE_SOUND: u32 = 250073;
/// ft_80088510 ids: a roll turning over on the ground or in the air.
const GROUND_ROLL_SOUND: u32 = 250064;
const AIR_ROLL_SOUND: u32 = 250067;
/// efSync_Spawn ids: the turn's and landing's dust (0x3FF), the wall
/// bounce's spark (0x406).
const ROLL_DUST: u16 = 0x3FF;

/// ft_PlaySFX(fp, id, 127, 64). The ids are outside 332..370, so no
/// random pitch is drawn.
pub fn sound(f: &mut Fighter, id: u32) {
    f.commands.footstep_sounds.push(FootstepSound {
        channel: SoundChannel::Ordinary,
        id,
        volume: 0x7F,
        pan: 0x40,
    });
}

/// ftPr_SpecialS_8013DD54 (8013DD54): wrap the roll angle, then play the
/// rolling sound (ft_80088510) each time it turns over; `turning` rolls
/// backward.
pub fn roll_sound(f: &mut Fighter, turning: bool) {
    let rollout = scratch(f);
    rollout.roll_angle = wrap(rollout.roll_angle);
    let angle = rollout.roll_angle;
    let previous = rollout.previous_angle;
    let forward = if turning { -1.0 } else { 1.0 };
    let turned_over = if previous == 0.0 {
        true
    } else if rollout.direction == forward {
        angle < previous
    } else {
        angle > previous
    };
    rollout.previous_angle = angle;
    if turned_over {
        let id = if f.physics.ground_or_air == GroundOrAir::Air {
            AIR_ROLL_SOUND
        } else {
            GROUND_ROLL_SOUND
        };
        f.commands.footstep_sounds.push(FootstepSound {
            channel: SoundChannel::Effect,
            id,
            volume: 0x7F,
            pan: 0x40,
        });
    }
}

/// The double-precision wrap loops (e.g. 8013E22C..8013E268): each step a
/// double add rounded to the stored float.
pub fn wrap(mut angle: f32) -> f32 {
    while angle < 0.0 {
        angle = (f64::from(angle) + TAU) as f32;
    }
    while f64::from(angle) > TAU {
        angle = (f64::from(angle) - TAU) as f32;
    }
    angle
}

/// normalizeAndSetRollAngle: the roll angle into [0, 2pi], then part 3's
/// X rotation.
pub fn set_roll(f: &mut Fighter) {
    let rollout = scratch(f);
    rollout.roll_angle = wrap(rollout.roll_angle);
    let angle = rollout.roll_angle;
    f.core.set_part_rotation(part::ROLL, Axis::X, angle);
}

/// ftPartSetRotY(fp, FtPart_TopN, M_PI_2): the model faces the camera's
/// right whatever the facing (the float constant).
pub fn face_forward(f: &mut Fighter) {
    f.core.set_part_rotation(part::FACING, Axis::Y, HALF_PI_F32);
}

/// ftPartSetRotY(fp, FtPart_TopN, M_PI_2 * facing): double product, rounded.
pub fn face(f: &mut Fighter) {
    let angle = (HALF_PI * f64::from(f.physics.facing)) as f32;
    f.core.set_part_rotation(part::FACING, Axis::Y, angle);
}

/// The saved model scale back on the root joint (HSD_JObjSetScale).
fn restore_scale(f: &mut Fighter) {
    let saved = f.character.get::<Jigglypuff>().rollout_scale;
    let root = f.animation.root;
    f.skeleton.set_scale(root, &saved);
}

/// ftPr_SpecialS_8013D658 (8013D658): the saved scale returns, the model
/// faces the facing and a pending facing applies.
pub fn restore(f: &mut Fighter) {
    restore_scale(f);
    face(f);
    let pending = scratch(f).pending_facing;
    if pending != 0.0 {
        f.physics.facing = pending;
    }
    scratch(f).pending_facing = 0.0;
}

/// scaleAnimStep: the landing squash for four frames, else the saved scale.
pub fn squash(f: &mut Fighter) {
    let step = scratch(f).squash_step;
    if !(0..4).contains(&step) {
        restore_scale(f);
        return;
    }
    let saved = f.character.get::<Jigglypuff>().rollout_scale;
    // 8013E48C / 8013E4A0: separate fmuls.
    let scale = Vec3::new(
        saved.x,
        saved.y * SQUASH_Y[step as usize],
        saved.z * SQUASH_Z[step as usize],
    );
    let root = f.animation.root;
    f.skeleton.set_scale(root, &scale);
    scratch(f).squash_step += 1;
}

/// Retail keeps a disabled capsule's contents (lbColl_80008428 only sets
/// its state); ftPr_SpecialS_8013D8E4 can enable it again. The port drops
/// disabled capsules, so hitbox 0 is remembered before any change that
/// would disable it and before each frame's script.
pub fn remember_hitbox(f: &mut Fighter) {
    if let Some(hit) = &f.commands.hitboxes[0] {
        let hit = hit.clone();
        f.character.get_mut::<Jigglypuff>().rollout_hitbox = Some(hit);
    }
}

/// ftColl_8007AFF8 (8007AFF8): every hitbox off.
pub fn clear_hitboxes(f: &mut Fighter) {
    remember_hitbox(f);
    f.commands.hitboxes.fill(None);
}

/// hitCapsuleToggle: every `hitbox_refresh_interval` frames the live
/// hitbox changes group (x4 = (x4 + 1) & 1), so it may hit again.
pub fn toggle_group(f: &mut Fighter) {
    let interval = attributes(f).hitbox_refresh_interval;
    let rollout = scratch(f);
    rollout.group_timer += 1;
    if rollout.group_timer < interval {
        return;
    }
    if let Some(hit) = &mut f.commands.hitboxes[0] {
        hit.descriptor.group = (hit.descriptor.group + 1) & 1;
        scratch(f).group_timer = 0;
    }
}

/// ftPr_SpecialS_8013D8E4 (8013D8E4): too slow a roll disables hitbox 0
/// (and clears the secondary colour animation); fast enough enables it
/// again; its damage follows the speed.
pub fn update_hitbox(f: &mut Fighter, assets: &melee_ft::fighter::assets::FighterAssets) {
    let a = attributes(f);
    let (minimum, base, multiplier) = (
        a.minimum_hitbox_speed,
        a.damage_speed_offset,
        a.damage_multiplier,
    );
    let speed = if f.physics.ground_or_air == GroundOrAir::Air {
        fabsf(f.physics.self_velocity.x)
    } else {
        fabsf(f.physics.ground_velocity)
    };
    let enabled = f.commands.hitboxes[0].is_some();
    if speed < minimum {
        clear_hitbox_zero(f);
        // ftCo_800BFFAC: lb_80014498(&fp->x488).
        f.core.clear_secondary_color_overlay(&assets.color_overlays);
    } else if !enabled {
        enable_hitbox_zero(f);
    }
    if f.commands.hitboxes[0].is_none() {
        return;
    }
    // 8013D9A8 fadds, 8013D9AC fmuls, 8013D9B0 fctiwz.
    let damage = gekko_math::msl::fctiwz(multiplier * (base + speed)).max(1);
    set_hitbox_damage(f, damage);
}

/// x914[0].state = HitCapsule_Disabled.
fn clear_hitbox_zero(f: &mut Fighter) {
    remember_hitbox(f);
    f.commands.hitboxes[0] = None;
}

/// x914[0].state = HitCapsule_Enabled: the remembered capsule starts a new
/// sweep.
fn enable_hitbox_zero(f: &mut Fighter) {
    let Some(mut hit) = f.character.get::<Jigglypuff>().rollout_hitbox.clone() else {
        unimplemented!("ftPr_SpecialS_8013D8E4: enabling a capsule this port never saw");
    };
    hit.phase = melee_coll::hitbox::CapsulePhase::Enabled;
    f.commands.hitboxes[0] = Some(hit);
}

/// ftColl_8007ABD0 (8007ABD0): hitbox 0's damage from `rate`, then staled.
fn set_hitbox_damage(f: &mut Fighter, rate: i32) {
    if f.player.scale != 1.0 {
        unimplemented!("ftColl_8007ABD0: ftCo_CalcYScaledKnockback for a scaled fighter");
    }
    // ftCo_800DEEB8 scales only a released smash charge; the roll has none.
    assert!(
        f.commands.smash_charge.is_none(),
        "ftCo_800DEEB8: charged roll"
    );
    let damage = rate as f32;
    let staled = f.commands.stale_damage(damage);
    let hit = f.commands.hitboxes[0].as_mut().expect("roll hitbox");
    hit.knockback_damage = gekko_math::msl::fctiwz(damage) as u32;
    hit.descriptor.damage = staled;
}

/// efSync_Spawn(0x3FF, gobj, &cur_pos, &facing_dir, &floor_angle): dust
/// turned to the facing and rotated to the floor.
pub fn dust(f: &mut Fighter) {
    let normal = f.collision.data.floor.normal;
    let angle = melee_lb::trigf::atan2f(-normal.x, normal.y);
    let position = f.physics.position;
    let facing = f.physics.facing;
    f.effects
        .push(melee_ef::request::EffectRequest::PositionalGraphics {
            id: ROLL_DUST,
            position,
            facing,
            angle,
        });
}

/// wallBounceEffect: the bounce spark on the wall at the ECB's side and
/// half height, a medium quake, rumble and the bounce sound. The angle
/// always reads the left-facing wall's normal, signed by `direction`.
pub fn wall_bounce_effect(f: &mut Fighter, direction: f32) {
    let cd = &f.collision.data;
    let normal = cd.left_facing_wall.normal;
    let mut position = f.physics.position;
    // 801412C8 fneg, 801412CC fmuls.
    let angle = melee_lb::trigf::atan2f(-normal.x * direction, normal.y);
    if direction > 0.0 {
        // 801412F4 fadds.
        position.x += fabsf(cd.ecb.right.x);
    } else {
        // 801413D8 fsubs.
        position.x -= fabsf(cd.ecb.left.x);
    }
    // 80141304 fadds, 80141328 fmadds.
    let height = fabsf(cd.ecb.top.y + cd.ecb.bottom.y);
    position.y = gekko_math::fma::fmadds(0.5, height, position.y);
    f.effects
        .push(melee_ef::request::EffectRequest::SurfaceRebound { position, angle });
    f.core.quake_request = Some(melee_cm::QuakeKind::Medium);
    // ftCommon_8007EBAC: controller rumble, an output request only.
    f.commands.rumble_requests.push(RumbleRequest {
        all_players: false,
        id: 0xC,
        duration: 0xA,
    });
    sound(f, BOUNCE_SOUND);
}
