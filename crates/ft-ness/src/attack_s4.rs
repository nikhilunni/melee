//! The baseball bat, ftnessattacks4.c (80114BF4..80114E98): the forward
//! smash holds a bat article and, while the script's first command
//! variable is set, reflects projectiles from a bubble on the bat.
use crate::{
    common::{self, row},
    init::Ness,
};
use melee_coll::defense::ReflectDescriptor;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, InputPhase, MotionRow, PhysicsPhase},
        ActionId, Fighter, Interaction, MotionData,
    },
};
use melee_it::{ItemRequest, SpawnItem};
use melee_types::ItemKind;

/// ftNs_MS_AttackS4 (341).
pub const ATTACK_S4: ActionId = ActionId(341);
/// it_802AD478(gobj, &fp->cur_pos, 0x2A, facing): the fp->parts index the
/// bat hangs from.
const BAT_PART: u8 = 0x2A;
/// ftNs_AttackS4_OnReflect: ft_PlaySFX(fp, 0xE0, 127, 64).
const REFLECT_SOUND: u32 = 0xE0;

pub const fn rows() -> [MotionRow; 1] {
    [row(
        ATTACK_S4,
        0x3E,
        anim,
        input,
        physics,
        callbacks::collision::escape,
    )]
}

/// ftNs_AttackS4_Enter (80114C24), decideFighter's FTKIND_NESS arm.
pub fn enter(f: &mut Fighter, assets: &FighterAssets, _: &mut gekko_math::HsdRng) -> Result<()> {
    f.commands.allow_interrupt = false;
    f.commands.variables[0] = 0;
    // The smash writes no mv field, so mv+4 stays the predecessor's.
    let retained_word = f.inherited_scratch_word();
    f.change_motion_state(ATTACK_S4, assets)?;
    f.step_animation(assets);
    f.core.state_data = MotionData::Smash { retained_word };
    f.core.status.interaction = Interaction::Attack;
    // it_802AD478: Item_InitSpawnOnPlaneNoInitialCollision at Ness, then
    // Item_8026AB54 into the hand.
    let spawn = SpawnItem::held(
        ItemKind::NessBat,
        f.player.id,
        f.physics.position,
        f.physics.facing,
    );
    f.core.item_requests.push(ItemRequest::SpawnInHand {
        spawn,
        part: BAT_PART,
        hold: false,
        catch_item: false,
        scale_by_owner: false,
    });
    f.character.get_mut::<Ness>().bat = true;
    common::install_damage_callbacks(f);
    Ok(())
}

/// ftNs_AttackS4_ItemNessBatRemove (80114CF4): it_802AD6B8.
pub fn remove_bat(f: &mut Fighter) {
    if std::mem::take(&mut f.character.get_mut::<Ness>().bat) {
        common::remove_article(f, ItemKind::NessBat);
    }
}

/// ftNs_AttackS4_Anim (80114D50): the script's first command variable
/// raises the reflect bubble (ftColl_CreateReflectHit with xB8) and lowers
/// it; the bat goes with the animation.
fn anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    f.advance_smash_charge(p.assets);
    let raised = f.commands.variables[0] != 0;
    if !f.combat.reflector_enabled {
        if raised {
            f.combat.reflector_enabled = true;
            f.shield.reflect.volume.position_cached = false;
        }
    } else if !raised {
        f.combat.reflector_enabled = false;
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        remove_bat(f);
        common::wait(f, p.assets)?;
    }
    Ok(None)
}

/// ftNs_AttackS4_IASA (80114E04): once the script allows it, the bat goes
/// and Wait's interrupts apply.
fn input(f: &mut Fighter, p: InputPhase<'_>) {
    if f.commands.allow_interrupt {
        remove_bat(f);
        callbacks::input::tilt(f, p);
    }
}

/// ftNs_AttackS4_Phys (80114E64): ft_80084FA8, then ftColl_8007AEF8 moves
/// the reflect bubble.
fn physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::jab(f, p);
    f.shield.reflect.volume.position_cached = false;
}

/// ftColl_CreateReflectHit (8007B240), literal descriptor transfer.
fn descriptor(f: &Fighter) -> ReflectDescriptor {
    let a = &f.character.get::<Ness>().attributes.bat_reflection;
    ReflectDescriptor {
        bone: a.joint as usize,
        maximum_damage: a.max_damage,
        offset: a.offset,
        radius: a.size,
        damage_multiplier: a.damage_multiplier,
        speed_multiplier: a.speed_multiplier,
        exclude_master_ball_ownership: a.skip_ownership_change != 0,
    }
}

/// The live bubble's contact (ftColl_80077464's reflect volume).
pub fn reflector_contact(
    f: &mut Fighter,
    hit: &melee_coll::hitbox::HitCapsule,
    scale: f32,
) -> Option<ReflectDescriptor> {
    if !f.combat.reflector_enabled || f.motion_state.action != ATTACK_S4 {
        return None;
    }
    let descriptor = descriptor(f);
    f.core
        .reflector_contact(hit, scale, &descriptor)
        .then_some(descriptor)
}

/// ftNs_AttackS4_OnReflect (80114BF4): only the sound.
pub fn reflect_hit(f: &mut Fighter, _: f32, _: &FighterAssets) -> Result<()> {
    common::play_sound(f, REFLECT_SOUND);
    Ok(())
}
