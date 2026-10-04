//! PK Fire, ftnessspecials.c (80116B70..80116EA4): the script's throw flag
//! sends a bolt from the hand, level on the ground and downward in the air.
use crate::{
    common::{self, row},
    init::{Accessory, Ness},
};
use gekko_math::msl::{cosf, sinf};
use hsd_types::Vec3;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{AnimationPhase, CollisionPhase, MotionRow},
        ActionId, Fighter,
    },
};
use melee_it::{ItemRequest, SpawnItem};
use melee_types::{GroundOrAir, ItemKind};

/// ftNs_MS_SpecialS (356) and ftNs_MS_SpecialAirS (357).
pub const GROUND: ActionId = ActionId(356);
pub const AIR: ActionId = ActionId(357);
/// fp->parts[42] (retail 0x80116BD4: lwz 0x2A0 of the parts array), the
/// hand the bolt leaves.
const HAND_PART: usize = 0x2A;

pub const fn rows() -> [MotionRow; 2] {
    [
        row(
            GROUND,
            0x133,
            anim::<false>,
            common::no_input,
            common::ground_friction,
            ground_collision,
        ),
        row(
            AIR,
            0x134,
            anim::<true>,
            common::no_input,
            common::air_friction_fall,
            air_collision,
        ),
    ]
}

/// ftNs_SpecialS_Enter (80116C94) / ftNs_SpecialAirS_Enter (80116D04).
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    f.commands.clear_throw_flags();
    f.commands.variables[0] = 0;
    // The move writes no mv field, so mv+4 stays the predecessor's.
    let retained_word = f.inherited_scratch_word();
    f.change_motion_state(if air { AIR } else { GROUND }, a)
        .expect("PK Fire assets");
    f.character.get_mut::<Ness>().retained_word = retained_word;
    f.step_animation(a);
    f.character.get_mut::<Ness>().accessory = Accessory::PkFire;
    f.core.arm_accessory4();
}

/// ftNs_SpecialS_ItemPKFireSpawn (80116B70), the accessory4 callback: on
/// the script's throw flag the bolt leaves fp->parts[42], offset by x30
/// along the facing (retail 0x80116BEC: fmadds) and x34 up, at the
/// grounded or aerial angle and speed (0x80116C38..50: separate fmuls).
pub fn fire(f: &mut Fighter) {
    if !std::mem::take(&mut f.commands.throw_accessory) {
        return;
    }
    let (angle, speed, spawn_x, spawn_y) = {
        let a = &f.character.get::<Ness>().attributes.pk_fire;
        let (angle, speed) = if f.physics.ground_or_air == GroundOrAir::Air {
            (a.air_angle, a.air_speed)
        } else {
            (a.ground_angle, a.ground_speed)
        };
        (angle, speed, a.spawn_x, a.spawn_y)
    };
    let c = &mut f.core;
    let facing = c.physics.facing;
    let mut position = melee_ft::fighter::caches::part_position(
        &mut c.skeleton,
        &c.animation,
        HAND_PART,
        Vec3::ZERO,
    );
    position.x = gekko_math::fma::fmadds(spawn_x, facing, position.x);
    position.y += spawn_y;
    let velocity = Vec3::new(facing * (speed * cosf(angle)), speed * sinf(angle), 0.0);
    // it_802AA054: prev_pos is the point on the stage plane; pos is
    // it_8026BB68's ECB midpoint (ftLib_80086990: fadds, fmuls, fadds).
    let mut spawn = SpawnItem::ray(ItemKind::NessPKFire, c.player.id, position, facing);
    let midpoint = 0.5 * (c.collision.data.ecb.top.y + c.collision.data.ecb.bottom.y);
    spawn.position = Vec3::new(
        c.physics.position.x + 0.0,
        c.physics.position.y + midpoint,
        c.physics.position.z + 0.0,
    );
    spawn.velocity = velocity;
    spawn.spawn_argument = it_ness::pk_fire::angle_argument(angle * facing);
    c.item_requests.push(ItemRequest::Spawn(spawn));
}

/// ftNs_SpecialS_Anim (80116D74) / ftNs_SpecialAirS_Anim (80116DB0).
fn anim<const AIR: bool>(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        if AIR {
            common::fall(f, p.assets)?;
        } else {
            common::wait(f, p.assets)?;
        }
    }
    Ok(None)
}

/// ftNs_SpecialS_Coll (80116E2C): ft_800827A0, Fall off the edge.
fn ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::stays_on_edge(f, &mut p) {
        common::fall(f, p.assets.expect("PK Fire collision assets"))?;
    }
    Ok(())
}

/// ftNs_SpecialAirS_Coll (80116E68): ft_80081D0C, then the special
/// landing with x38 of lag (ftCo_LandingFallSpecial_Enter(gobj, false, x38)).
fn air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::lands(f, &mut p) {
        let lag = f.character.get::<Ness>().attributes.pk_fire.landing_lag;
        f.enter_special_landing(p.assets.expect("PK Fire landing assets"), false, lag)?;
    }
    Ok(())
}
