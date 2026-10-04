//! Headbutt, ftdonkeyspecials.c (8010E1C4..8010E4EC).
//!
//! The script's hitbox buries a grounded victim (the bury element; the
//! victim's side is ftCo_Bury in melee-ft). The code only moves Donkey
//! Kong: grounded friction, or in the air a slowed drift that starts
//! falling once the script sets cmd_vars[0].
use crate::{
    common::{self, change, row},
    init::{Accessory, DonkeyKong},
};
use melee_ef::request::EffectRequest;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{AnimationPhase, CollisionPhase, MotionRow, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
};
use melee_types::FtPart;

/// ftDk_MS_SpecialS (379) and ftDk_MS_SpecialAirS (380).
pub const GROUND: ActionId = ActionId(379);
pub const AIR: ActionId = ActionId(380);

/// efSync_Spawn(1222 / 1223, gobj, TransN): models 0x1F40 (grounded) and
/// 0x1F41 (aerial), turned with the fighter.
const EFFECT: u16 = 0x4C6;
const AIR_EFFECT: u16 = 0x4C7;

/// coll_mf (ftdonkeyspecials.c:21): ftCommon_GroundAirColl_MF | KeepGfx |
/// SkipHit.
const GROUND_AIR: MotionEntryFlags =
    MotionEntryFlags(common::GROUND_AIR.0 | common::KEEP_GFX | common::SKIP_HIT);

pub const fn rows() -> [MotionRow; 2] {
    [
        row(
            GROUND,
            0x149,
            anim::<false>,
            common::no_input,
            common::ground_friction,
            ground_collision,
        ),
        row(
            AIR,
            0x14A,
            anim::<true>,
            common::no_input,
            air_physics,
            air_collision,
        ),
    ]
}

fn attributes(f: &Fighter) -> &crate::attributes::HeadbuttAttributes {
    &f.character.get::<DonkeyKong>().attributes.headbutt
}

/// ftDk_SpecialS_Enter (8010E1C4) / ftDk_SpecialAirS_Enter (8010E22C): the
/// aerial entry divides the drift by x3C and stops the fall.
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    if air {
        f.physics.self_velocity.x /= attributes(f).entry_velocity_divisor;
        f.physics.self_velocity.y = 0.0;
    }
    change(
        f,
        if air { AIR } else { GROUND },
        MotionEntryFlags(0),
        0.0,
        1.0,
        a,
    )
    .expect("Headbutt assets");
    // ftAnim_8006EBA4.
    f.step_animation(a);
    // Fighter_UnsetCmdVar0.
    f.commands.variables[0] = 0;
    f.character.get_mut::<DonkeyKong>().accessory = if air {
        Accessory::AirHeadbuttEffect
    } else {
        Accessory::HeadbuttEffect
    };
    f.core.arm_accessory4();
}

/// ftDk_SpecialLw_8010E0CC (8010E0CC) / ftDk_SpecialLw_8010E148 (8010E148):
/// the move's model on TransN unless effects are already kept (x2219_b0),
/// the efLib hitlag pair, and the accessory removes itself.
pub fn spawn_effect(f: &mut Fighter, air: bool) {
    f.core.accessory4_armed = false;
    if !f.effect_state.destroy_on_state_change {
        f.effects.push(EffectRequest::SyncAttached {
            id: if air { AIR_EFFECT } else { EFFECT },
            bone: common::part(FtPart::TransN),
        });
        f.effect_state.destroy_on_state_change = true;
    }
    f.effect_state.hitlag_callbacks = true;
}

/// ftDk_SpecialS_Anim (8010E2A8) / ftDk_SpecialAirS_Anim (8010E2E4).
fn anim<const AIR: bool>(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::seal_graphics(f, p.assets, p.rng);
        if AIR {
            common::fall(f, p.assets)?;
        } else {
            common::wait(f, p.assets)?;
        }
    }
    Ok(None)
}

/// ftDk_SpecialAirS_Phys (8010E348): gravity x44 (to the PlCo terminal
/// velocity) once the script sets cmd_vars[0], and the aerial friction x40
/// throughout.
fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    let (gravity, friction) = {
        let a = attributes(f);
        (a.gravity, a.air_friction)
    };
    if f.commands.variables[0] != 0 {
        let terminal = f.attributes.air.terminal_velocity;
        common::fall_at(f, gravity, terminal);
    }
    common::air_friction(f, friction);
    f.core.finish_air_update(p.assets, p.wind);
}

/// ftDk_SpecialS_Coll (8010E3B8): once the script sets cmd_vars[0] the
/// floor's edge holds Donkey Kong (ft_800827A0); before it he walks off
/// (ft_80082708). Either way losing the floor continues in the air.
fn ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    let supported = if f.commands.variables[0] != 0 {
        common::stays_on_edge(f, &mut p)
    } else {
        common::stays_grounded(f, &mut p)
    };
    if !supported {
        let assets = p.assets.expect("Headbutt collision assets");
        common::ground_to_air(f, AIR, GROUND_AIR, assets)?;
        keep_hitlag_callbacks(f);
    }
    Ok(())
}

/// ftDk_SpecialAirS_Coll (8010E43C): ft_80081D0C; landing continues on
/// the ground.
fn air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::lands(f, &mut p) {
        let assets = p.assets.expect("Headbutt landing assets");
        common::air_to_ground(f, GROUND, GROUND_AIR, assets)?;
        keep_hitlag_callbacks(f);
    }
    Ok(())
}

/// doAirTransition / doGroundTransition: with the model kept (x2219_b0),
/// Fighter_SetEffectHitlagCallbacks again.
fn keep_hitlag_callbacks(f: &mut Fighter) {
    if f.effect_state.destroy_on_state_change {
        f.effect_state.hitlag_callbacks = true;
    }
}
