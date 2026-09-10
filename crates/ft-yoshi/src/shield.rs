//! Egg shield callbacks, ftYoshi/ftyoshiguard.c and ftCommon/ftCo_Escape.c.
use crate::init::Yoshi;
use hsd_types::Vec3;
use melee_ft::fighter::{
    assets::{FighterAssets, Result},
    commands::{FootstepSound, SoundChannel},
    effects::EffectRequest,
    escape::HurtStatus,
    shield::{GuardState, ReflectHitCallback, ReflectVolume, ShieldVolume},
    Fighter, MotionData,
};
use melee_ft::input::Buttons;
use melee_types::{CommonMotionState as S, FtPart};

/// ftYs_Init_MotionStateTable (ftyoshi.c): custom rows reuse common motion IDs.
#[repr(i32)]
pub enum EggShieldMotion {
    On = 341,
    Hold = 342,
    Off = 343,
    Damage = 344,
    Reflect = 345,
}

/// ftYs_Init_8012B8A4 (8012B8A4): shield health selects the egg material frame.
/// Renderer output is retained just like the common texture/model commands.
pub fn material(fighter: &mut Fighter<Yoshi>) {
    // retail 8012B8D0/E0/E4: fdivs, fsubs, fmuls, no contraction.
    fighter.character.shield_material_frame = fighter.character.attributes.shield_material_frames
        * (1.0 - fighter.status.shield_health / fighter.character.shield_maximum_health);
}
fn size(fighter: &mut Fighter<Yoshi>) {
    // ftCo_80091D58 -> inlineB0: egg size is independent of health and analog L/R.
    let size = fighter.attributes.shield.initial_shield_size;
    fighter.shield.size = size;
    let joint = fighter.animation.parts[usize::from(fighter.bones.model.shield)].joint;
    fighter
        .skeleton
        .set_scale(joint, &Vec3::new(size, size, size));
}
/// ftYs_Init_8012BDA0: intangible body and one normal, grabbable egg capsule.
fn egg_body(fighter: &mut Fighter<Yoshi>) {
    fighter.character.egg_body = true;
    fighter.character.egg_hurtbox = Some(melee_ft::fighter::caches::Hurtbox {
        height: melee_ft::fighter::caches::HurtHeight::Middle,
        grabbable: true,
        bone: usize::from(fighter.bones.model.shield),
        offsets: [Vec3::ZERO; 2],
        radius: 1.0,
        positions: [Vec3::ZERO; 2],
        cached: false,
    });
    fighter.commands.hurt_status = HurtStatus::Intangible;
}
fn model(fighter: &mut Fighter<Yoshi>, variant: i32) {
    fighter.character.model_group = variant;
    fighter.commands.model_selections.insert(0, variant);
}
/// ftYs_Init_8012BE3C (8012BE3C): restore the body and burst twelve shell pieces.
pub fn leave_egg(fighter: &mut Fighter<Yoshi>, assets: &FighterAssets) {
    model(fighter, 0);
    fighter.character.egg_body = false;
    fighter.character.egg_hurtbox = None;
    fighter.commands.hurt_status = HurtStatus::Normal;
    let bone = usize::from(assets.parts.joint(FtPart::HipN).expect("HipN"));
    fighter.core.effects.push(EffectRequest::EggShell {
        bone,
        scale: fighter.core.attributes.yoshi_egg.size,
    });
}
pub fn enter(fighter: &mut Fighter<Yoshi>, assets: &FighterAssets, reflect: bool) -> Result<()> {
    // Plain GuardOn does not write the old reflect/powershield countdowns.
    let retained = match &fighter.state_data {
        MotionData::Guard(guard) => Some(guard),
        MotionData::Escape(escape) => escape.retained_guard.as_ref(),
        _ => None,
    };
    let windows = retained.map_or([0.0; 2], |guard| {
        [guard.reflect_frames, guard.powershield_frames]
    });
    fighter.change_motion_state(if reflect { S::GuardReflect } else { S::GuardOn }, assets)?;
    fighter.step_animation(assets);
    fighter.state_data = MotionData::Guard(GuardState {
        minimum_hold: assets.shield.minimum_hold,
        reflect_frames: if reflect {
            assets.shield.reflect_frames
        } else {
            windows[0]
        },
        powershield_frames: if reflect {
            assets.shield.powershield_frames
        } else {
            windows[1]
        },
        ..Default::default()
    });
    fighter.character.egg_body = false;
    fighter.character.egg_hurtbox = None;
    // HurtCapsule_Disabled throughout the startup animation.
    fighter.commands.hurt_status = HurtStatus::Intangible;
    if reflect {
        fighter.input.shoulder.tilt = 0xFE;
        fighter.shield.fresh_powershield = true;
        fighter.shield.reflect_window = true;
        fighter.shield.powershield_window = true;
        fighter.shield.reflecting = true;
        fighter.shield.reflect = ReflectVolume {
            volume: ShieldVolume {
                bone: usize::from(fighter.bones.model.shield),
                radius: assets.shield.reflect_radius,
                ..Default::default()
            },
            maximum_damage: fighter.status.shield_health,
            damage_multiplier: assets.shield.reflect_damage,
            speed_multiplier: assets.shield.reflect_speed,
            reflect_behavior: true,
        };
        fighter.shield.on_reflect = Some(ReflectHitCallback::Egg);
    } else {
        fighter.install_shield();
    }
    let joint = fighter.animation.parts[usize::from(fighter.bones.model.shield)].joint;
    fighter.skeleton.set_translate(joint, &Vec3::ZERO);
    material(fighter);
    size(fighter);
    fighter.commands.footstep_sounds.push(FootstepSound {
        channel: SoundChannel::Ordinary,
        id: 110,
        volume: 127,
        pan: 64,
    });
    Ok(())
}
/// ftYs_Shield_8012C1D4 (8012C1D4): SM_None, rest pose, egg model and capsule.
pub fn hold(fighter: &mut Fighter<Yoshi>, assets: &FighterAssets) -> Result<()> {
    if let MotionData::Escape(escape) = &fighter.state_data {
        let mut guard = escape.retained_guard.clone().unwrap_or_default();
        // Escape's integer timer and Guard's elapsed float share mv +0 in
        // retail. Direct roll-to-hold entry preserves those bits, not the value.
        guard.elapsed = f32::from_bits(escape.interrupt_frames as u32);
        fighter.state_data = MotionData::Guard(guard);
    }
    fighter.change_motion_state(S::Guard, assets)?;
    fighter
        .core
        .animation
        .reset_pose(&mut fighter.core.skeleton, false);
    model(fighter, 1);
    egg_body(fighter);
    fighter.install_shield();
    fighter.commands.articles_visible = false;
    material(fighter);
    size(fighter);
    Ok(())
}
pub fn off(fighter: &mut Fighter<Yoshi>, assets: &FighterAssets) -> Result<()> {
    fighter.change_motion_state(S::GuardOff, assets)?;
    leave_egg(fighter, assets);
    Ok(())
}
pub fn animate(fighter: &mut Fighter<Yoshi>, assets: &FighterAssets) -> Result<()> {
    let state = fighter.motion_state.id;
    if state == S::GuardSetOff {
        unimplemented!("ftYs_Shield_8012C600: egg shield damage");
    }
    if state == S::GuardReflect {
        fighter.update_reflect_windows();
    }
    fighter.guard().elapsed += 1.0;
    if state == S::GuardOff {
        if !fighter.animation.frames_remaining(&fighter.skeleton) {
            fighter.change_motion_state(S::Wait, assets)?;
        }
        return Ok(());
    }
    if !fighter.input.current.held.intersects(Buttons::SHIELD) {
        fighter.guard().released = true;
    }
    fighter.drain_shield(assets);
    if state != S::Guard && !fighter.animation.frames_remaining(&fighter.skeleton) {
        return hold(fighter, assets);
    }
    if state == S::Guard
        && (fighter.guard().released || (!fighter.shield.active && !fighter.shield.reflecting))
    {
        return off(fighter, assets);
    }
    material(fighter);
    Ok(())
}
/// ftYs_GuardOn_0 / GuardHold / GuardOff IASA (ftyoshiguard.c:135,204,258).
/// Hold release belongs to Anim. These IASAs have neither shield jumping
/// nor the common C-stick spot dodge; GuardOn_1 uses shared GuardReflect.
pub fn input(fighter: &mut Fighter<Yoshi>, assets: &FighterAssets) -> Result<()> {
    let state = fighter.motion_state.id;
    if state == S::GuardSetOff {
        unimplemented!("ftyoshiguard.c:273-320: egg shield damage");
    }
    if state == S::GuardOn
        && fighter.guard().elapsed < assets.input.powershield_window as f32
        && fighter.input.pressed.intersects(Buttons::DIGITAL_SHOULDERS)
        && i32::from(fighter.input.shoulder.tilt) < assets.input.powershield_window
    {
        unimplemented!("ftyoshiguard.c:332-347: delayed egg powershield");
    }
    // ftCo_8009515C is false without a held item. ftCo_80099794 requires LR.
    if fighter.input.current.held.intersects(Buttons::SHIELD)
        && fighter.input.current.stick.y <= assets.input.escape_threshold
        && i32::from(fighter.input.vertical.tilt) < assets.input.escape_window
    {
        return fighter.enter_escape(assets, S::EscapeN);
    }
    if state == S::GuardOff {
        return Ok(());
    }
    if let Some(roll) = fighter.roll_input(assets) {
        return fighter.enter_escape(assets, roll);
    }
    if fighter.input.pressed.intersects(Buttons::A)
        && fighter.input.current.held.intersects(Buttons::SHIELD)
    {
        unimplemented!("ftyoshiguard.c:141,207: grab out of egg shield");
    }
    if fighter.collision.data.floor.flags & 0x100 != 0
        && fighter.input.current.stick.y <= -assets.movement.platform_drop_threshold
        && i32::from(fighter.input.vertical.tilt) < assets.movement.platform_drop_window
    {
        unimplemented!("ftyoshiguard.c:142,207: egg shield platform drop");
    }
    Ok(())
}
pub fn escape_entered(
    fighter: &mut Fighter<Yoshi>,
    assets: &FighterAssets,
    rolling: bool,
) -> Result<()> {
    if rolling {
        model(fighter, 1);
        egg_body(fighter);
    } else if fighter.character.model_group == 1 {
        leave_egg(fighter, assets);
    }
    Ok(())
}
pub fn escape_finished(fighter: &mut Fighter<Yoshi>, assets: &FighterAssets) -> Option<Result<()>> {
    if fighter.motion_state.id == S::EscapeN {
        return None;
    }
    if fighter.input.current.held.intersects(Buttons::SHIELD) && fighter.status.shield_health >= 0.0
    {
        return Some(hold(fighter, assets));
    }
    leave_egg(fighter, assets);
    Some(fighter.change_motion_state(S::Wait, assets))
}
