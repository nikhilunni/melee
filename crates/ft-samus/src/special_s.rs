//! Missiles, ftsamusspecials.c and ftSs_SpecialS_8012A074
//! (8012A068..8012A674).
//!
//! A side special with a fresh horizontal smash (x673 within x28 frames)
//! fires a super missile, otherwise a homing one. The script raises
//! throw_flags_b0 on the firing frame; accessory4 then counts the missile
//! (u.ss.x2238) and spawns it at bone 56, once per state.
use crate::{
    common::{self, change},
    init::Samus,
};
use hsd_types::Vec3;
use melee_ef::request::EffectRequest;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{AnimationPhase, CollisionPhase, MotionRow, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
};
use melee_it::{ItemRequest, SpawnItem};
use melee_types::ItemKind;

/// ftSs_MS_SpecialS (349), SpecialSSmash (350), SpecialAirS (351) and
/// SpecialAirSSmash (352).
pub const GROUND: ActionId = ActionId(349);
pub const GROUND_SMASH: ActionId = ActionId(350);
pub const AIR: ActionId = ActionId(351);
pub const AIR_SMASH: ActionId = ActionId(352);

/// `fp->parts[FtPart_56]`: the arm cannon's joint (a raw parts index).
const CANNON_PART: usize = 56;
/// efSync_Spawn(1155, gobj, &pos): efAlt 0x483, efLib_CreateGenerator(0x7D7).
const FIRE_FLASH: u16 = 0x483;

pub const fn rows() -> [MotionRow; 4] {
    [
        common::row(
            GROUND,
            ground_anim,
            common::no_input,
            common::ground_friction,
            edge_collision,
        ),
        common::row(
            GROUND_SMASH,
            ground_anim,
            common::no_input,
            common::ground_friction,
            walk_off_collision,
        ),
        common::row(
            AIR,
            air_anim,
            common::no_input,
            air_physics,
            air_collision,
        ),
        common::row(
            AIR_SMASH,
            air_anim,
            common::no_input,
            air_physics,
            air_collision,
        ),
    ]
}

fn attributes(f: &Fighter) -> &crate::attributes::MissileAttributes {
    &f.character.get::<Samus>().attributes.missile
}

/// ftSs_SpecialS_Enter (8012A1D8) / ftSs_SpecialAirS_Enter (8012A2AC): the
/// ground speed (or aerial x velocity) divided by x2C, the super missile
/// for a fresh smash, then ftSamus_ClearThrowFlagsUnk.
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    let divisor = attributes(f).velocity_divisor;
    if air {
        f.physics.self_velocity.x /= divisor;
    } else {
        f.physics.ground_velocity /= divisor;
        f.physics.self_velocity.y = 0.0;
    }
    let smash = f32::from(f.input.horizontal.held) < attributes(f).smash_frames;
    let state = match (air, smash) {
        (false, true) => GROUND_SMASH,
        (false, false) => GROUND,
        (true, true) => AIR_SMASH,
        (true, false) => AIR,
    };
    change(f, state, MotionEntryFlags(0), 0.0, 1.0, a).expect("Missile assets");
    // ftAnim_8006EBA4.
    f.step_animation(a);
    f.commands.clear_throw_flags();
    f.character.get_mut::<Samus>().accessory = crate::init::Accessory::Missile;
    f.core.arm_accessory4();
}

/// ftSs_SpecialS_8012A074 (8012A074): on the script's throw_flags_b0 the
/// missile count rises, the missile spawns at the cannon plus x34 along the
/// facing (8012A0F0: fmadds), the flash follows, and the accessory goes.
pub fn fire(f: &mut Fighter) {
    if !std::mem::take(&mut f.commands.throw_accessory) {
        return;
    }
    // fp->accessory4_cb = 0 once the missile is out.
    f.core.accessory4_armed = false;
    let count = {
        let samus = f.character.get_mut::<Samus>();
        samus.missiles_fired = samus.missiles_fired.wrapping_add(1);
        samus.missiles_fired
    };
    let offset = attributes(f).spawn_offset_x;
    let c = &mut f.core;
    let mut position =
        melee_ft::fighter::caches::part_position(&mut c.skeleton, &c.animation, CANNON_PART, Vec3::ZERO);
    position.x = gekko_math::fma::fmadds(offset, c.physics.facing, position.x);
    let smash = matches!(c.motion_state.action, GROUND_SMASH | AIR_SMASH);
    // it_802B62D0: prev_pos is the point on the stage plane; pos is
    // it_8026BB68's ECB midpoint (ftLib_80086990: fadds, fmuls, fadds).
    let mut spawn = SpawnItem::ray(ItemKind::SamusMissile, c.player.id, position, c.physics.facing);
    let midpoint = 0.5 * (c.collision.data.ecb.top.y + c.collision.data.ecb.bottom.y);
    spawn.position = Vec3::new(
        c.physics.position.x + 0.0,
        c.physics.position.y + midpoint,
        c.physics.position.z + 0.0,
    );
    spawn.spawn_argument = it_samus::missile::spawn_argument(smash, count);
    c.item_requests.push(ItemRequest::Spawn(spawn));
    // ftSs_SpecialS_8012A168: the flash once per state (x2219_b0), and
    // Fighter_SetEffectHitlagCallbacks.
    if !c.effect_state.destroy_on_state_change {
        c.effects_after_items.push(EffectRequest::PositionalGenerator {
            id: FIRE_FLASH,
            position,
        });
        c.effect_state.destroy_on_state_change = true;
    }
    c.effect_state.hitlag_callbacks = true;
}

/// ftSs_SpecialS_Anim (8012A380) / ftSs_SpecialSSmash_Anim (8012A4E0):
/// Wait at the end.
fn ground_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::wait(f, p.assets)?;
    }
    Ok(None)
}

/// ftSs_SpecialAirS_Anim (8012A3BC) / ftSs_SpecialAirSSmash_Anim
/// (8012A51C): Fall at the end.
fn air_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::fall(f, p.assets)?;
    }
    Ok(None)
}

/// ftSs_SpecialAirS_Phys (8012A420) / ftSs_SpecialAirSSmash_Phys:
/// ftCommon_FallBasic, then ftCommon_ApplyFrictionAir with x30.
fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    common::fall_basic(f);
    let friction = attributes(f).air_friction;
    common::air_friction(f, friction);
    f.core.finish_air_update(p.assets, p.wind);
}

/// ftSs_SpecialS_Coll (8012A468): ft_800827A0, Fall off the edge.
fn edge_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::stays_on_edge(f, &mut p) {
        common::fall(f, p.assets.expect("Missile collision assets"))?;
    }
    Ok(())
}

/// ftSs_SpecialSSmash_Coll (8012A5C8): ft_80082708, Fall off the floor.
fn walk_off_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::stays_grounded(f, &mut p) {
        common::fall(f, p.assets.expect("Missile collision assets"))?;
    }
    Ok(())
}

/// ftSs_SpecialAirS_Coll (8012A4A4) / ftSs_SpecialAirSSmash_Coll:
/// ft_80081D0C, then ft_80082B1C's landing.
fn air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::lands(f, &mut p) {
        f.land_from_air(p.assets.expect("Missile landing assets"))?;
    }
    Ok(())
}
