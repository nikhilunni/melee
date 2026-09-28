//! Egg shield callbacks, ftYoshi/ftyoshiguard.c and ftCommon/ftCo_Escape.c.
use crate::init::Yoshi;
use gekko_math::fma::fmadds;
use hsd_types::Vec3;
use melee_ef::request::EffectRequest;
use melee_ft::fighter::{
    assets::{FighterAssets, Result},
    commands::{FootstepSound, SoundChannel},
    shield::{GuardState, ReflectHitCallback, ReflectVolume, ShieldImpact, ShieldVolume},
    Fighter, MotionData,
};
use melee_ft::input::Buttons;
use melee_types::combat::HurtStatus;
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
pub fn material(fighter: &mut Fighter) {
    // retail 8012B8D0/E0/E4: fdivs, fsubs, fmuls, no contraction.
    let character = fighter.character.get_mut::<Yoshi>();
    character.shield_material_frame = character.attributes.shield_material_frames
        * (1.0 - fighter.core.status.shield_health / character.shield_maximum_health);
}
fn size(fighter: &mut Fighter) {
    // ftCo_80091D58 -> inlineB0: egg size is independent of health and analog L/R.
    let size = fighter.attributes.shield.initial_shield_size;
    fighter.shield.size = size;
    let joint = fighter.animation.parts[usize::from(fighter.bones.model.shield)].joint;
    fighter
        .skeleton
        .set_scale(joint, &Vec3::new(size, size, size));
}
/// ftYs_Init_8012BDA0 (8012BDA0): the body's capsules intangible and capsule
/// 0 replaced by one enabled, grabbable egg capsule on the shield bone.
fn egg_body(fighter: &mut Fighter) {
    fighter.core.set_hurt_capsules(HurtStatus::Intangible);
    let bone = usize::from(fighter.bones.model.shield);
    fighter.core.replace_hurt_capsule(
        0,
        melee_coll::hurtbox::HurtCapsule {
            height: melee_coll::hurtbox::HurtHeight::Middle,
            grabbable: true,
            bone,
            // ftYs_Unk1_803B75C0: both ends at the bone origin.
            offsets: [Vec3::ZERO; 2],
            radius: 1.0,
            positions: [Vec3::ZERO; 2],
            cached: false,
        },
    );
}
fn model(fighter: &mut Fighter, variant: i32) {
    fighter.character.get_mut::<Yoshi>().model_group = variant;
    fighter.commands.model_selections.insert(0, variant);
}
/// ftYs_Init_8012BE3C (8012BE3C): restore the body and burst twelve shell pieces.
pub fn leave_egg(fighter: &mut Fighter, assets: &FighterAssets) {
    model(fighter, 0);
    // The replaced capsule 0 stays until the next motion change.
    fighter.core.set_hurt_capsules(HurtStatus::Normal);
    let bone = usize::from(assets.parts.joint(FtPart::HipN).expect("HipN"));
    fighter.core.effects.push(EffectRequest::EggShell {
        bone,
        scale: fighter.core.attributes.yoshi_egg.size,
    });
}
pub fn enter(fighter: &mut Fighter, assets: &FighterAssets, reflect: bool) -> Result<()> {
    // Plain GuardOn does not write the old reflect/powershield countdowns.
    let retained = match &fighter.state_data {
        MotionData::Guard(guard) => Some(guard),
        MotionData::Escape(escape) => escape.retained_guard.as_ref(),
        _ => None,
    };
    let windows = retained.map_or([0.0; 2], |guard| {
        [guard.reflect_frames, guard.powershield_frames]
    });
    fighter.change_motion_state(
        (if reflect { S::GuardReflect } else { S::GuardOn }).into(),
        assets,
    )?;
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
    // HurtCapsule_Disabled throughout the startup animation.
    fighter.core.set_hurt_capsules(HurtStatus::Invincible);
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
pub fn hold(fighter: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    if let MotionData::Escape(escape) = &fighter.state_data {
        let mut guard = escape.retained_guard.clone().unwrap_or_default();
        // Escape's integer timer and Guard's elapsed float share mv +0 in
        // retail. Direct roll-to-hold entry preserves those bits, not the value.
        guard.elapsed = f32::from_bits(escape.interrupt_frames as u32);
        fighter.state_data = MotionData::Guard(guard);
    }
    fighter.change_motion_state(S::Guard.into(), assets)?;
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
pub fn off(fighter: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    fighter.change_motion_state(S::GuardOff.into(), assets)?;
    leave_egg(fighter, assets);
    Ok(())
}
/// ftYs_Shield_8012C600 (8012C600), ftCo_80092E50's Yoshi branch: the
/// GuardDamage row (344) without an animation of its own, the egg model and
/// capsule, and a pushback without the powershield factor or maximum.
pub fn stun(fighter: &mut Fighter, impact: &ShieldImpact, assets: &FighterAssets) -> Result<()> {
    // Yoshi authors no animation 40, so ftAnim_8006F3DC finds no AObj and
    // returns f1 as ftAnim_8006E7B8 left it: the end frame of the last egg
    // MObj AObj interpreted (HSD_AObjInterpretAnim 803642C0), all of which
    // share the one end frame (ftYs_Init_8012B6E8).
    let yoshi = fighter.character.get::<Yoshi>();
    fighter.animation.unanimated_frame = Some(yoshi.attributes.shield_material_frames);
    fighter.change_motion_state(S::GuardSetOff.into(), assets)?;
    if !fighter.shield.powershield_window {
        model(fighter, 1);
    }
    egg_body(fighter);
    let p = &assets.shield;
    // retail 8012C728..8012C73C: fsubs, fsubs, fmuls, fmadds (no lightshield
    // interpolation, unlike ftCo_80092F2C).
    let frames = fmadds(
        p.stun_multiplier,
        impact.damage as f32 * (1.0 - fighter.shield.lightshield),
        p.stun_base,
    );
    // Cape hits (ftCo_80092E50's x19B0 == 10) never reach here.
    let push = frames * p.pushback_multiplier;
    fighter.physics.ground_velocity = if impact.facing < 0.0 { push } else { -push };
    fighter.install_shield();
    size(fighter);
    Ok(())
}
pub fn animate(fighter: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    let state = fighter.motion_state.id;
    if state == S::GuardSetOff {
        // ftYs_GuardDamage_Anim (8012C7A4): no shield size update.
        fighter.update_reflect_windows();
        if !fighter.animation.frames_remaining(&fighter.skeleton) {
            if fighter.guard().released {
                return off(fighter, assets);
            }
            return hold(fighter, assets);
        }
        return Ok(());
    }
    if state == S::GuardReflect {
        fighter.update_reflect_windows();
    }
    fighter.guard().elapsed += 1.0;
    if state == S::GuardOff {
        if !fighter.animation.frames_remaining(&fighter.skeleton) {
            fighter.change_motion_state(S::Wait.into(), assets)?;
        }
        return Ok(());
    }
    if !fighter.input.current.held.intersects(Buttons::SHIELD) {
        fighter.guard().released = true;
    }
    fighter.drain_shield(assets);
    if fighter.break_drained_shield(assets)? {
        // ftYs_GuardOn_0/GuardHold/GuardOn_1_Anim: the break, then the
        // egg bursts (spawnEffect, ftyoshiguard.c:100-117).
        leave_egg(fighter, assets);
        return Ok(());
    }
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
pub fn input(fighter: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    let state = fighter.motion_state.id;
    if state == S::GuardSetOff {
        // ftYs_GuardDamage_IASA (8012C80C) is empty.
        return Ok(());
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
        // ftCo_Catch_CheckInput (800D8990) -> ftCo_800D8C54: the motion
        // change restores the capsules; no shell burst.
        return fighter.enter_catch(assets);
    }
    // ftCo_8009A080 (8009A080): with LR held, a fresh stick tap down on a
    // platform (ftCo_80099F1C) drops through it (ftCo_8009A228); the motion
    // change restores the capsules, as for the grab.
    if fighter.input.current.held.intersects(Buttons::SHIELD)
        && fighter.input.current.stick.y <= -assets.movement.platform_drop_threshold
        && f32::from(fighter.input.vertical.tilt) < assets.movement.platform_drop_window
        && fighter.collision.data.floor.flags & 0x100 != 0
    {
        return fighter.enter_pass(assets);
    }
    Ok(())
}
pub fn escape_entered(fighter: &mut Fighter, assets: &FighterAssets, rolling: bool) -> Result<()> {
    if rolling {
        model(fighter, 1);
        egg_body(fighter);
    } else if fighter.character.get::<Yoshi>().model_group == 1 {
        leave_egg(fighter, assets);
    }
    Ok(())
}
pub fn escape_finished(fighter: &mut Fighter, assets: &FighterAssets) -> Option<Result<()>> {
    if fighter.motion_state.id == S::EscapeN {
        return None;
    }
    if fighter.input.current.held.intersects(Buttons::SHIELD) && fighter.status.shield_health >= 0.0
    {
        return Some(hold(fighter, assets));
    }
    leave_egg(fighter, assets);
    Some(fighter.change_motion_state(S::Wait.into(), assets))
}
