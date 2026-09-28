//! Screw Attack, ftsamusspecialhi.c (8012A674..8012ADEC).
//!
//! The grounded row rises once the script sets cmd_vars[0] (the physics
//! lifts Samus with every jump spent and a fixed take-off speed); while
//! the script's cmd_vars[1] is clear a firm stick against the facing turns
//! her around once. The animation's end falls special; falling onto a
//! floor or ledge ends it early.
use crate::{
    common::{self, change},
    init::Samus,
};
use melee_ef::request::EffectRequest;
use melee_ft::{
    anim::WaitChoice,
    collision::air,
    fighter::{
        assets::{FighterAssets, Result},
        part_rotation::Axis,
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, MotionRow, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
};
use melee_types::{FtPart, GroundOrAir};

/// ftSs_MS_SpecialHi (353) and ftSs_MS_SpecialAirHi (354).
pub const GROUND: ActionId = ActionId(353);
pub const AIR: ActionId = ActionId(354);

/// efSync_Spawn(1154, gobj, YRotN): efAlt 0x482, model 0x7D0 attached.
const SCREW_EFFECT: u16 = 0x482;

/// mv.ss.unk5 (ftSamus/types.h): the turnaround was used.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScrewAttack {
    pub turned: bool,
}

pub const fn rows() -> [MotionRow; 2] {
    [
        common::row(GROUND, anim, input, ground_physics, collision),
        common::row(AIR, anim, input, air_physics, collision),
    ]
}

fn attributes(f: &Fighter) -> &crate::attributes::ScrewAttackAttributes {
    &f.character.get::<Samus>().attributes.screw_attack
}

/// ftSs_SpecialHi_Enter (8012A674) / ftSs_SpecialAirHi_Enter (8012A738).
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    change(
        f,
        if air { AIR } else { GROUND },
        MotionEntryFlags(0),
        0.0,
        1.0,
        a,
    )
    .expect("Screw Attack assets");
    crate::init::install_damage_callbacks(f);
    // Fighter_SetEffectHitlagCallbacks.
    f.effect_state.hitlag_callbacks = true;
    if air {
        // ftCommon_8007D60C.
        f.leave_ground_with_spent_jumps();
    } else {
        // ftCommon_8007D7FC.
        f.land();
    }
    f.commands.variables = [0; 4];
    f.character.get_mut::<Samus>().screw_attack = ScrewAttack::default();
    if air {
        let (launch, maximum) = {
            let a = attributes(f);
            (a.air_launch_y, a.air_max)
        };
        f.physics.self_velocity.y = launch;
        common::clamp_self_velocity_x(f, maximum);
    }
    // ftAnim_8006EBA4.
    f.step_animation(a);
    let bone = common::part(FtPart::YRotN);
    f.effects.push(EffectRequest::SyncAttached {
        id: SCREW_EFFECT,
        bone,
    });
    f.character.get_mut::<Samus>().screw_effect = true;
}

/// ftSamus_DestroyAllUnsetx2444: efLib_DestroyAll and x2244 = 0.
fn end_effect(f: &mut Fighter) {
    f.effects.push(EffectRequest::DestroyOwned);
    f.character.get_mut::<Samus>().screw_effect = false;
}

/// ftSs_SpecialHi_Anim (8012A81C) / ftSs_SpecialAirHi_Anim (8012A8C4): at
/// the end the effect goes, every jump is spent and Samus falls special
/// (ftCo_80096900(gobj, 1, 1, 0, x48, x50)), or plainly with no lag.
fn anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        end_effect(f);
        f.leave_ground_with_spent_jumps();
        let (mobility, lag) = {
            let a = attributes(f);
            (a.freefall_mobility, a.landing_lag)
        };
        if lag == 0.0 {
            common::fall(f, p.assets)?;
        } else {
            f.enter_special_fall(p.assets, true, true, false, mobility, lag)?;
        }
    }
    Ok(None)
}

/// ftSs_SpecialHi_IASA (8012A96C) / ftSs_SpecialAirHi_IASA (8012AA3C): while the
/// script's cmd_vars[1] is clear, a stick past x4C against the facing
/// turns Samus once (ftCommon_UpdateFacing, ftPartSetRotY(fp, 0,
/// M_PI_2 * facing) in double, rounded once).
fn input(f: &mut Fighter, _: InputPhase<'_>) {
    if f.commands.variables[1] != 0 || f.character.get::<Samus>().screw_attack.turned {
        return;
    }
    let x = f.input.current.stick.x;
    let magnitude = if x < 0.0 { -x } else { x };
    if magnitude <= attributes(f).reverse_stick {
        return;
    }
    let facing = f.physics.facing;
    if (facing == 1.0 && x < 0.0) || (facing == -1.0 && x > 0.0) {
        f.commands.variables[1] = 1;
        f.character.get_mut::<Samus>().screw_attack.turned = true;
        f.physics.facing = if x >= 0.0 { 1.0 } else { -1.0 };
        let rotation = (std::f64::consts::FRAC_PI_2 * f64::from(f.physics.facing)) as f32;
        f.core.set_part_rotation(0, Axis::Y, rotation);
    }
}

/// ftSs_SpecialHi_Phys (8012AB0C): the script's cmd_vars[0] lifts Samus
/// (ftCommon_8007D60C) at x38 times her facing; airborne, TransN's y
/// (ft_800851C0) and the ordinary drift (ftCommon_8007D344 is overwritten
/// by ftCommon_8007D268); grounded, ft_80084F3C.
fn ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    if f.commands.variables[0] != 0 {
        f.leave_ground_with_spent_jumps();
        f.commands.variables[0] = 0;
        // 8012AB54: fmuls.
        f.physics.self_velocity.x = attributes(f).ground_launch_x * f.physics.facing;
    }
    if f.physics.ground_or_air == GroundOrAir::Air {
        let offset = f
            .animation
            .root_motion
            .as_ref()
            .expect("Screw Attack TransN")
            .primary_history
            .offset;
        f.physics.self_velocity.y = offset.y;
        let (mobility, maximum) = {
            let a = attributes(f);
            (a.air_mobility, a.air_max)
        };
        common::drift_with_friction(f, 0.0, mobility, maximum);
        common::ordinary_drift(f);
        f.core.finish_air_update(p.assets, p.wind);
        return;
    }
    common::ground_friction(f, p);
}

/// ftSs_SpecialAirHi_Phys (8012ABB4): ft_80084DB0, then the attribute
/// drift (ftCommon_8007D344).
fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    f.core.fall_physics(p.assets);
    let (mobility, maximum) = {
        let a = attributes(f);
        (a.air_mobility, a.air_max)
    };
    common::drift_with_friction(f, 0.0, mobility, maximum);
    f.core.finish_air_update(p.assets, p.wind);
}

/// ftSs_SpecialHi_Coll (8012AC00) / ftSs_SpecialAirHi_Coll (8012ACF8):
/// grounded, ft_80084104; rising, ft_80081D0C; falling, landing
/// (ft_CheckGroundAndLedge toward the facing) lands special, otherwise a
/// ledge (ftCliffCommon_80081298) is caught.
fn collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if f.physics.ground_or_air == GroundOrAir::Ground {
        return callbacks::collision::escape(f, p);
    }
    if f.physics.self_velocity.y >= 0.0 {
        common::lands(f, &mut p);
        return Ok(());
    }
    let assets = p.assets.expect("Screw Attack collision assets");
    let c = &mut f.core;
    air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    let landed = air::collide_pass(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        c.status.ledge_cooldown == 0,
    );
    if landed {
        end_effect(f);
        let lag = attributes(f).landing_lag;
        f.enter_special_landing(assets, false, lag)?;
    } else if f.try_grab_ledge(assets, p.map)? {
        // ftCliffCommon_80081298 enters CliffCatch itself; the effect goes
        // and ftCliffCommon_80081370 runs a second time.
        end_effect(f);
        f.enter_cliff_catch(assets, p.map)?;
    }
    Ok(())
}
