//! Bowser Bomb, ftkoopaspeciallw.c (80134518..80134A5C).
//!
//! The grounded start hops (TransN drives both axes) and hands over to the
//! aerial row at frame 30. The aerial row drifts down slowly until its
//! animation ends, then drops at the attribute speed with a trail; once
//! the script arms the landing (cmd_vars[0]) a floor lands it with a
//! burst and a ledge can be caught.
use crate::{
    common::{self, change},
    init::{Accessory, Koopa},
};
use melee_ef::request::EffectRequest;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{AnimationPhase, CollisionPhase, MotionRow, PhysicsPhase},
        ActionId, Fighter,
    },
};

/// ftKp_MS_SpecialLw (361): the grounded start, entered airborne.
pub const START: ActionId = ActionId(361);
/// ftKp_MS_SpecialAirLw (362): the aerial start and the drop.
pub const AIR: ActionId = ActionId(362);
/// ftKp_MS_SpecialLwLanding (363).
pub const LANDING: ActionId = ActionId(363);

/// ftKp_SpecialLw_80134988's start frame in the aerial animation.
const DROP_START_FRAME: f32 = 30.0;
/// efSync_Spawn(0x4D8, gobj, &cur_pos): the landing burst.
const LANDING_EFFECT: u16 = 0x4D8;
/// efSync_Spawn(0x4DF, gobj, parts[0].joint): the drop's trail.
const DROP_EFFECT: u16 = 0x4DF;

/// ftKp_Init_MotionStateTable[20..23]: submotions 313..315.
pub const fn rows() -> [MotionRow; 3] {
    [
        common::row(
            START,
            313,
            start_anim,
            common::no_input,
            start_physics,
            collision,
        ),
        common::row(AIR, 314, air_anim, common::no_input, air_physics, collision),
        common::row(
            LANDING,
            315,
            landing_anim,
            common::no_input,
            landing_physics,
            landing_collision,
        ),
    ]
}

fn attributes(f: &Fighter) -> &crate::attributes::BombAttributes {
    &f.character.get::<Koopa>().attributes.bomb
}

/// ftKp_SpecialLw_Enter (80134610) / ftKp_SpecialAirLw_Enter (801346A4).
pub fn enter(f: &mut Fighter, airborne: bool, assets: &FighterAssets) {
    let state = if airborne {
        let (scale_x, scale_y) = {
            let a = attributes(f);
            (a.entry_scale_x, a.entry_scale_y)
        };
        // 801346D4 / 801346E4: fmuls.
        f.physics.self_velocity.x *= scale_x;
        f.physics.self_velocity.y *= scale_y;
        AIR
    } else {
        f.physics.self_velocity.x = 0.0;
        f.physics.self_velocity.y = 0.0;
        // ftCommon_8007D5D4.
        f.leave_ground();
        START
    };
    f.change_motion_state(state, assets)
        .expect("Bowser Bomb assets");
    // ftAnim_8006EBA4.
    f.step_animation(assets);
    // ftKp_SpecialLw_Enter_inline, after the first animation step: the
    // frame-zero commands' variables and TransN's vertical delta go.
    f.commands.variables[1] = 0;
    f.commands.variables[0] = 0;
    f.commands.throw_accessory = false;
    if let Some(root) = &mut f.animation.root_motion {
        root.primary_history.offset.y = 0.0;
    }
    // x2223_b4 disables ft_80081A00's item landing, which is not modelled.
}

/// ftKp_SpecialLw_Anim (8013474C): at the hop's end, ftKp_SpecialLw_80134988
/// (80134988) continues in the aerial row at frame 30, hitboxes kept.
fn start_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        change(
            f,
            AIR,
            common::GROUND_AIR_KEEP_HIT,
            DROP_START_FRAME,
            p.assets,
        )?;
    }
    Ok(None)
}

/// ftKp_SpecialAirLw_Anim (80134788): the finished animation starts the
/// drop (cmd_vars[1]).
fn air_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.commands.variables[1] = 1;
    }
    Ok(None)
}

/// ftKp_SpecialLw_Phys (801347C4): ft_80085134; the hop never descends.
fn start_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    common::root_motion_velocity(f);
    if f.physics.self_velocity.y < 0.0 {
        f.physics.self_velocity.y = 0.0;
    }
    f.core.finish_air_update(p.assets, p.wind);
}

/// ftKp_SpecialAirLw_Phys (80134804): the attribute gravity and air
/// friction; at least the drop speed, exactly it (and no drift) once the
/// animation has ended, when the trail is armed on accessory4.
fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let (gravity, terminal, friction, drop) = {
        let a = attributes(f);
        (
            a.gravity,
            a.terminal_velocity,
            a.air_friction,
            a.drop_velocity,
        )
    };
    common::gravity(f, gravity, terminal);
    common::air_friction(f, friction);
    let dropping = f.commands.variables[1] != 0;
    if f.physics.self_velocity.y < drop || dropping {
        f.physics.self_velocity.y = drop;
    }
    if dropping {
        f.physics.self_velocity.x = 0.0;
        if !f.effect_state.destroy_on_state_change {
            crate::init::install_accessory(f, Accessory::BombDrop);
        }
    }
    f.core.finish_air_update(p.assets, p.wind);
}

/// ftKp_SpecialAirLw_Coll (801348C0), also ftKp_SpecialLw_Coll (801348A0):
/// once the script arms the landing a falling touch lands and a ledge can
/// be caught; before that a floor touch keeps Bowser airborne
/// (ftCommon_8007D5D4) with his ECB unlocked at once.
fn collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    let touched = common::lands_or_finds_ledge(f, &mut p);
    let armed = f.commands.variables[0] != 0;
    let assets = p.assets.expect("Bowser Bomb collision assets");
    if touched {
        if armed && f.physics.self_velocity.y <= 0.0 {
            land(f, assets)?;
        } else {
            f.leave_ground();
            common::unlock_ecb(f);
        }
    } else if armed && f.try_grab_ledge(assets, p.map)? {
        // ftCliffCommon_80081298 enters CliffCatch itself;
        // ftCliffCommon_80081370 then runs a second time.
        f.enter_cliff_catch(assets, p.map)?;
    }
    Ok(())
}

/// ftKp_SpecialLw_80134A5C (80134A5C): land with the burst on accessory4.
fn land(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    f.physics.ground_velocity = 0.0;
    // ftCommon_8007D7FC.
    f.land();
    f.change_motion_state(LANDING, assets)?;
    crate::init::install_accessory(f, Accessory::BombLanding);
    Ok(())
}

/// ftKp_SpecialLwLanding_Anim (801349C4): Wait at the end (ft_8008A2BC).
fn landing_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::wait(f, p.assets)?;
    }
    Ok(None)
}

/// ftKp_SpecialLwLanding_Phys (80134A00): ft_80084F3C.
fn landing_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    common::ground_friction(f, p);
}

/// ftKp_SpecialLwLanding_Coll (80134A20): Fall when the floor is lost.
fn landing_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::stays_grounded(f, &mut p) {
        let assets = p.assets.expect("Bowser Bomb landing assets");
        common::fall(f, assets)?;
    }
    Ok(())
}

/// fn_80134518 (80134518), the landing's accessory4: the burst at the
/// fighter's position once per motion (x2219_b0), then the effect hitlag
/// callbacks.
pub(crate) fn landing_burst(f: &mut Fighter) {
    if !f.effect_state.destroy_on_state_change {
        let position = f.physics.position;
        f.effects.push(EffectRequest::PositionalGenerator {
            id: LANDING_EFFECT,
            position,
        });
        f.effect_state.destroy_on_state_change = true;
    }
    f.effect_state.hitlag_callbacks = true;
}

/// fn_80134590 (80134590), the drop's accessory4: the trail on the root
/// joint once per motion, then the effect hitlag callbacks.
pub(crate) fn drop_trail(f: &mut Fighter) {
    if !f.effect_state.destroy_on_state_change {
        f.effects.push(EffectRequest::SyncAttached {
            id: DROP_EFFECT,
            bone: 0,
        });
        f.effect_state.destroy_on_state_change = true;
    }
    f.effect_state.hitlag_callbacks = true;
}
