//! Spinning Kong, ftdonkeyspecialhi.c (8010FCD4..80110074).
//!
//! Grounded, Donkey Kong walks under the stick with his own acceleration
//! and cap; airborne he rises on the entry velocity under a reduced
//! gravity (the script's cmd_vars[0] restores the full one) and drifts
//! with a wider cap. Either row spends every jump. Falling, the aerial row
//! lands back into the grounded one or catches a ledge.
use crate::{
    common::{self, change, row},
    init::DonkeyKong,
};
use melee_ef::request::EffectRequest;
use melee_ft::{
    anim::WaitChoice,
    collision::{air, ecb::EcbPose},
    fighter::{
        assets::{FighterAssets, Result},
        state::{AnimationPhase, CollisionPhase, MotionRow, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
};
use melee_types::FtPart;

/// ftDk_MS_SpecialHi (381) and ftDk_MS_SpecialAirHi (382).
pub const GROUND: ActionId = ActionId(381);
pub const AIR: ActionId = ActionId(382);

/// efSync_Spawn(1226, gobj, TopN): model 0x1F44 on the fighter's root.
const EFFECT: u16 = 0x4CA;

pub const fn rows() -> [MotionRow; 2] {
    [
        row(GROUND, 0x14B, ground_anim, common::no_input, ground_physics, ground_collision),
        row(AIR, 0x14C, air_anim, common::no_input, air_physics, air_collision),
    ]
}

fn attributes(f: &Fighter) -> &crate::attributes::SpinningKongAttributes {
    &f.character.get::<DonkeyKong>().attributes.spinning_kong
}

/// ftDk_SpecialHi_Enter (8010FCD4) / ftDk_SpecialAirHi_Enter (8010FDA4).
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    change(f, if air { AIR } else { GROUND }, MotionEntryFlags(0), 0.0, 1.0, a)
        .expect("Spinning Kong assets");
    common::install_damage_callbacks(f);
    f.commands.variables = [0; 4];
    let (ground_max, launch) = {
        let a = attributes(f);
        (a.ground_max, a.air_launch_y)
    };
    if air {
        // The aerial entry clamps to the grounded cap x54 too.
        common::clamp_self_velocity_x(f, ground_max);
        f.physics.self_velocity.y = launch;
    } else {
        common::clamp_ground_velocity(f, ground_max);
        f.physics.self_velocity.x = f.physics.ground_velocity;
        f.physics.self_velocity.y = 0.0;
    }
    f.physics.jumps_used = f.attributes.jumping.max_jumps as u8;
    // ftAnim_8006EBA4.
    f.step_animation(a);
    f.effects.push(EffectRequest::SyncAttached {
        id: EFFECT,
        bone: common::part(FtPart::TopN),
    });
}

/// ftDk_SpecialHi_Anim (8010FE60).
fn ground_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::seal_graphics(f, p.assets, p.rng);
        common::wait(f, p.assets)?;
    }
    Ok(None)
}

/// ftDk_SpecialAirHi_Anim (8010FE9C): every jump spent
/// (ftCommon_8007D60C), then the special fall with landing lag x64
/// (ftCo_80096900(gobj, 1, 0, 1, 1.0, x64)), or a plain fall without one.
fn air_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::seal_graphics(f, p.assets, p.rng);
        f.leave_ground_with_spent_jumps();
        let lag = attributes(f).landing_lag;
        if lag == 0.0 {
            common::fall(f, p.assets)?;
        } else {
            f.enter_special_fall(p.assets, true, false, true, 1.0, lag)?;
        }
    }
    Ok(None)
}

/// ftDk_SpecialHi_Phys (8010FF14): ftCommon_8007CADC(fp, 0, x5C, x54),
/// then ftCommon_ApplyGroundMovement.
fn ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let (mobility, maximum) = {
        let a = attributes(f);
        (a.ground_mobility, a.ground_max)
    };
    common::walk_toward_stick(f, 0.0, mobility, maximum);
    common::move_on_ground(f, &p);
}

/// ftDk_SpecialAirHi_Phys (8010FF58): gravity scaled by x50 until the
/// script sets cmd_vars[0] (8010FF90: fmuls), then
/// ftCommon_8007D344(fp, 0, x60, x58).
fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let (scale, mobility, maximum) = {
        let a = attributes(f);
        (a.gravity_scale, a.air_mobility, a.air_max)
    };
    let scale = if f.commands.variables[0] != 0 { 1.0 } else { scale };
    let gravity = scale * f.attributes.air.gravity;
    let terminal = f.attributes.air.terminal_velocity;
    common::fall_at(f, gravity, terminal);
    common::drift_with_friction(f, 0.0, mobility, maximum);
    f.core.finish_air_update(p.assets, p.wind);
}

/// ftDk_SpecialHi_Coll (8010FFCC): off the floor (ft_80082708) every jump
/// is spent and the aerial row continues at the same frame, its drift
/// clamped to x58.
fn ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::stays_grounded(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Spinning Kong collision assets");
    f.leave_ground_with_spent_jumps();
    let frame = f.animation.frame;
    change(f, AIR, common::GROUND_AIR, frame, 1.0, assets)?;
    common::install_damage_callbacks(f);
    let maximum = attributes(f).air_max;
    common::clamp_self_velocity_x(f, maximum);
    Ok(())
}

/// ft_CheckGroundAndLedge (800822A4) with CLIFFCATCH_BOTH (0): true on
/// landing; with no ledge cooldown the ledges on either side are tested.
fn lands_or_finds_ledge(f: &mut Fighter, p: &mut CollisionPhase<'_>) -> bool {
    let c = &mut f.core;
    air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    let cd = &mut c.collision.data;
    cd.last_pos = cd.cur_pos;
    cd.cur_pos = c.physics.position;
    let pose = EcbPose::read(&mut c.skeleton, c.animation.root, cd);
    let landed = if c.status.ledge_cooldown == 0 {
        melee_mp::set_facing_dir(cd, 0);
        p.map.air_collide_ledge(cd, Some(&|i| pose.position(i)))
    } else {
        p.map.air_collide_pass(cd, Some(&|i| pose.position(i)))
    };
    c.physics.position = cd.cur_pos;
    c.skeleton
        .set_translate(c.animation.root, &c.physics.position);
    landed
}

/// ftDk_SpecialAirHi_Coll (80110074): rising, ft_80081D0C; falling,
/// ft_CheckGroundAndLedge, else a ledge (ftCliffCommon_80081298 and
/// ftCliffCommon_80081370). Landing continues in the grounded row with the
/// ground speed clamped to x54.
fn air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    let assets = p.assets.expect("Spinning Kong collision assets");
    let landed = if f.physics.self_velocity.y >= 0.0 {
        common::lands(f, &mut p)
    } else if lands_or_finds_ledge(f, &mut p) {
        true
    } else {
        if f.try_grab_ledge(assets, p.map)? {
            f.enter_cliff_catch(assets, p.map)?;
        }
        false
    };
    if landed {
        common::air_to_ground(f, GROUND, common::GROUND_AIR, assets)?;
        common::install_damage_callbacks(f);
        let maximum = attributes(f).ground_max;
        common::clamp_ground_velocity(f, maximum);
    }
    Ok(())
}
