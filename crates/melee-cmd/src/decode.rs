//! Typed decoding of the supported ftaction.c subaction vocabulary.
use crate::Command;
pub type Result<T> = std::result::Result<T, &'static str>;
/// Number of words consumed, including the opcode; an unported code counts
/// one word and decodes to [`Command::Unported`].
pub fn word_count(opcode: u32) -> usize {
    match opcode {
        38 => 7,
        58 => 4,
        5 | 7 | 56 => 2,
        10 | 11 => 5,
        17 | 34 | 54 | 55 => 3,
        _ => 1,
    }
}
pub fn decode(words: &[u32], target: Option<usize>, continuation: usize) -> Result<Command> {
    let word = *words.first().ok_or("empty command")?;
    let opcode = word >> 26;
    if words.len() < word_count(opcode) {
        return Err("truncated command");
    }
    Ok(match opcode {
        0 => Command::End,
        58 => Command::WindEffect(crate::WindEffect {
            bone: word as u8,
            x: (words[1] >> 16) as i16,
            y: words[1] as i16,
            magnitude: (words[2] >> 16) as i16,
            decay: words[2] as i16,
            timer: (words[3] >> 16) as i16,
            angle: words[3] as i16,
        }),
        1 => Command::Wait((word & 0x03ff_ffff) as f32),
        2 => Command::AtFrame((word & 0x03ff_ffff) as f32),
        3 => Command::BeginLoop(word & 0x03ff_ffff),
        4 => Command::EndLoop,
        5 => Command::Call {
            target: target.ok_or("null command call")?,
            continuation,
        },
        6 => Command::Return,
        7 => Command::Goto(target.ok_or("null command goto")?),
        8 => Command::WaitAnimationLoop,
        // ftAction_80071D40: signed 7-bit model index, signed 19-bit selection.
        31 => Command::ModelSelection {
            group: ((word << 6) as i32) >> 25,
            variant: ((word << 13) as i32) >> 13,
        },
        // ftAction_80072A5C (80072A80/84): eight-bit ID, low 18-bit duration.
        46 => Command::ColorAnimation(crate::ColorAnimationRequest {
            id: ((word >> 18) & 255) as u8,
            duration: word & 0x3FFFF,
        }),
        18 => Command::SmashSound,
        35 => Command::HeldItemVisibility(word & 0x03ff_ffff != 0),
        // ftAction_80072894 (800728B4..C4): extrwi 13 bits after the opcode,
        // and the low 13 bits as an unsigned frame count.
        42 => Command::ParasolAnimation {
            index: ((word >> 13) & 0x1FFF) as usize,
            frames: (word & 0x1FFF) as f32,
        },
        24 => Command::ThrowAccessory,
        21 => Command::MoveCue,
        19 => Command::SetVariable {
            index: ((word >> 24) & 3) as usize,
            value: word & 0xFFFFFF,
        },
        10 => Command::Graphics(graphics(
            words[..5].try_into().expect("validated graphics length"),
        )),
        // ftAction_8007121C: five command words per attack capsule.
        11 => Command::SpawnHitbox {
            id: ((word >> 23) & 7) as usize,
            descriptor: hitbox(words)?,
        },
        34 => {
            let id = ((word >> 23) & 7) as usize;
            assert!(id < 2, "ftAction_80071E04: throw hitbox index");
            Command::SetThrowHitbox {
                id,
                descriptor: throw_hitbox(words)?,
            }
        }
        // ftAction_800718A4, 800718B0: clrlwi clears the six opcode bits.
        20 => match word & 0x03ff_ffff {
            0 => Command::GrabRelease,
            1 => Command::ThrowReverse,
            _ => return Err("unknown throw flag"),
        },
        // ftAction_8007162C (80071654 extrwi, 80071664 clrlwi): three-bit
        // capsule id, unsigned 23-bit damage.
        12 => Command::SetHitboxDamage {
            id: ((word >> 23) & 7) as usize,
            damage: (word & 0x007f_ffff) as f32,
        },
        // ftAction_8007169C (800716B4 clrlwi, 800716C0 extrwi): three-bit
        // capsule id, unsigned 23-bit size; 800716DC fmuls by 0.003906f.
        13 => Command::SetHitboxRadius {
            id: ((word >> 23) & 7) as usize,
            radius: 0.003906 * (word & 0x007f_ffff) as f32,
        },
        // ftAction_80071708: idx:24, type:1, value:1 below the opcode.
        14 => Command::HitboxTargets {
            id: ((word >> 2) & 0x00ff_ffff) as usize,
            items: word & 2 != 0,
            enabled: word & 1 != 0,
        },
        // ftAction_80071784, 800717A0: clrlwi keeps all 26 bits as the index.
        15 => Command::ClearHitbox((word & 0x03ff_ffff) as usize),
        16 => Command::ClearHitboxes,
        27 => Command::HurtCapsuleStatus {
            bone: None,
            status: hurt_status(word & 0x03ff_ffff)?,
        },
        28 => Command::HurtCapsuleStatus {
            bone: Some(((word >> 18) & 255) as usize),
            status: hurt_status(word & 0x3ffff)?,
        },
        // ftAction_80071AE8: enable ordinary jab continuation.
        29 => Command::JabFollowup(word & 0x03ff_ffff != 0),
        30 => Command::RapidJab(word & 0x03ff_ffff != 0),
        36 => Command::ArticleVisibility(word & 1 != 0),
        37 => Command::FighterVisibility(word & 1 != 0),
        38 => Command::RandomSound(crate::RandomSound {
            ids: words[1..7]
                .try_into()
                .expect("validated random sound length"),
            range: (word & 63) as u8,
            behavior: ((word >> 6) & 15) as u8,
            volume: ((word >> 18) & 255) as u8,
            pan: ((word >> 10) & 255) as u8,
        }),
        50 => Command::ToggleDynamics(((word << 6) as i32) >> 6),
        51 => Command::SelfDamage(((word << 6) as i32) >> 6),
        49 => Command::SwordTrail {
            duration: ((word << 7) as i32) >> 7,
            reverse: word & (1 << 25) != 0,
        },
        // ftAction_80073008: separate fmuls at retail 80073048.
        56 => Command::SmashCharge(crate::SmashCharge {
            phase: crate::ChargePhase::PreCharge,
            frames: 0.0,
            maximum_frames: ((word >> 16) & 1023) as f32,
            maximum_multiplier: 0.003906 * f32::from(word as u16),
            saved_rate: 1.0,
            color_animation: (words[1] >> 24) as u8,
        }),
        25 => Command::SetAirborne(match word & 0x03ff_ffff {
            0 => crate::AirborneMode::Ground,
            1 => crate::AirborneMode::Air,
            2 => crate::AirborneMode::AirUseAllJumps,
            _ => return Err("unsupported airborne mode"),
        }),
        23 => Command::AllowInterrupt,
        // ftAction_80071A14 (80071A30 clrlwi): low 26-bit vulnerability enum.
        26 => Command::HurtStatus(match word & 0x03ff_ffff {
            0 => melee_types::combat::HurtStatus::Normal,
            1 => melee_types::combat::HurtStatus::Invincible,
            2 => melee_types::combat::HurtStatus::Intangible,
            _ => return Err("unknown hurt status"),
        }),
        41 => Command::Part {
            group: ((word >> 19) & 127) as usize,
            variant: ((word >> 12) & 127) as usize,
            blend: (word & 4095) as f32,
        },
        43 => Command::Rumble {
            all_players: word & (1 << 25) != 0,
            id: ((word >> 13) & 4095) as u16,
            duration: (word & 8191) as u16,
        },
        17 | 54 => Command::FootstepSound {
            behavior: ((word >> 18) & 255) as u8,
            id: words[1],
            volume: (words[2] >> 8) as u8,
            pan: words[2] as u8,
            // ftAction_80072CD8: opcode 54 consults the floor's terrain.
            terrain: opcode == 54,
            alt_foot: opcode == 54 && word & (1 << 17) != 0,
        },
        55 => Command::LandingEffect((word & 0xFFFF) as u16),
        52 => Command::GroundPose((word & 7) as u8),
        40 => {
            let mut indices = vec![((word >> 18) & 127) as usize];
            if word & (1 << 25) != 0 {
                indices.push(((word >> 11) & 127) as usize);
            }
            Command::Texture {
                indices,
                frame: (word & 2047) as f32,
            }
        }
        _ => Command::Unported(opcode),
    })
}
fn half(words: &[u32], index: usize) -> u16 {
    (words[index / 2] >> if index.is_multiple_of(2) { 16 } else { 0 }) as u16
}
fn hitbox(words: &[u32]) -> Result<melee_types::combat::HitboxDescriptor> {
    let first = words[0];
    let flags = words[3];
    let last = words[4];
    // ftAction_8007121C --fused: none. Literal is 0.003906f, not 1/256.
    const SCALE: f32 = 0.003906;
    Ok(melee_types::combat::HitboxDescriptor {
        requires_throw_owner: flags & 8 != 0,
        common_bone: first & (1 << 10) != 0,
        group: ((first >> 20) & 7) as u8,
        bone: ((first >> 11) & 255) as usize,
        damage: (first & 1023) as f32,
        shield_damage: (last >> 10) as u8 as i8,
        sound_severity: ((last >> 7) & 7) as u8,
        radius: SCALE * f32::from(half(words, 2)),
        offset: [
            SCALE * f32::from(half(words, 3) as i16),
            SCALE * f32::from(half(words, 4) as i16),
            SCALE * f32::from(half(words, 5) as i16),
        ]
        .into(),
        angle: (flags >> 23) as u16,
        growth: ((flags >> 14) & 511) as u16,
        weight_knockback: ((flags >> 5) & 511) as u16,
        base_knockback: (last >> 23) as u16,
        element: melee_types::HitElement::try_from(((last >> 18) & 31) as i32)
            .map_err(|_| "invalid hit element")?,
        hit_ground: last & 2 != 0,
        hit_air: last & 1 != 0,
        ignore_scale: flags & 4 != 0,
        clank: flags & 2 != 0,
        rebound: flags & 1 != 0,
    })
}
fn throw_hitbox(words: &[u32]) -> Result<melee_types::combat::ThrowHitbox> {
    let first = words[0];
    let second = words[1];
    let third = words[2];
    Ok(melee_types::combat::ThrowHitbox {
        damage: (first & 0x7f_ffff) as f32,
        angle: (second >> 23) as u16,
        growth: ((second >> 14) & 511) as u16,
        weight_knockback: ((second >> 5) & 511) as u16,
        base_knockback: (third >> 23) as u16,
        element: melee_types::HitElement::try_from(((third >> 19) & 15) as i32)
            .map_err(|_| "invalid hit element")?,
        sound_severity: ((third >> 16) & 7) as u8,
        sound_kind: ((third >> 12) & 15) as u8,
    })
}
/// Shared graphics payload: fighter opcode 10 and color-overlay opcode 21.
// The script decoder and overlay loader share this initialization-only body.
// Keep it out of downstream IR rather than cloning it into each loader.
#[inline(never)]
pub fn graphics(words: &[u32; 5]) -> melee_types::combat::GraphicsCommand {
    let word = words[0];
    // ftAction_80071028 (0x80071028): signed offsets, unsigned ranges.
    // Retail uses the literal 0.003906f, not exact 1/256; no fusion.
    const SCALE: f32 = 0.003906;
    melee_types::combat::GraphicsCommand {
        bone: ((word >> 18) & 255) as usize,
        common_bone: word & (1 << 17) != 0,
        destroy_on_state_change: word & (1 << 16) != 0,
        item_bone: word & (1 << 15) != 0,
        id: half(words, 2),
        parameter: f32::from(half(words, 3)),
        offset: [
            SCALE * f32::from(half(words, 4) as i16),
            SCALE * f32::from(half(words, 5) as i16),
            SCALE * f32::from(half(words, 6) as i16),
        ]
        .into(),
        range: [
            SCALE * f32::from(half(words, 7)),
            SCALE * f32::from(half(words, 8)),
            SCALE * f32::from(half(words, 9)),
        ]
        .into(),
    }
}

fn hurt_status(value: u32) -> Result<melee_types::combat::HurtStatus> {
    use melee_types::combat::HurtStatus;
    match value {
        0 => Ok(HurtStatus::Normal),
        1 => Ok(HurtStatus::Invincible),
        2 => Ok(HurtStatus::Intangible),
        _ => Err("unknown hurt capsule status"),
    }
}
