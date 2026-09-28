//! Falcon Dive's catch and throw, ftcaptainspecialhi.c (800E5128..800E5534).
//!
//! On a grounded victim Falcon hangs from it: his XRotN follows the
//! victim's TransN2 and accessory4 copies the victim's position. The catch
//! animation's end starts the throw (356) and releases the pair; the throw
//! drifts after the script's cmd_vars[0] and lands in LandingFallSpecial.
use melee_ft::{
    anim::WaitChoice,
    collision::air,
    fighter::{
        assets::{FighterAssets, Result},
        capture_captain,
        grab::GrabLink,
        ledge::GrabExclusions,
        state::{AnimationPhase, CollisionPhase, PhysicsPhase},
        Fighter,
    },
};
use melee_types::{CommonMotionState, GroundOrAir};

use crate::special_hi::{self, CATCH, THROW};
use crate::CaptainFamily;

/// ftCommon_8007E2D0(fp, 2, ...): Falcon Dive's grab type (x1A68).
pub const GRAB_TYPE: GrabExclusions = GrabExclusions(2);

fn attributes<C: CaptainFamily>(f: &Fighter) -> &crate::attributes::FalconDiveAttributes {
    &crate::attributes::<C>(f).falcon_dive
}

fn dive<C: CaptainFamily>(f: &mut Fighter) -> &mut special_hi::FalconDive {
    &mut crate::family::<C>(f).specials().dive
}

/// Falcon hangs from the victim (x221B_b7) while the attachment holds.
fn attached(f: &Fighter) -> bool {
    f.combat.thrown_pose.is_some()
}

/// ftCa_SpecialLw_800E5128 (800E5128), the grab_cb, then the common
/// grabbed_cb ftCo_8009CA0C on the victim.
pub fn grab(
    falcon: &mut Fighter,
    victim: &mut Fighter,
    falcon_assets: &FighterAssets,
    victim_assets: &FighterAssets,
) -> Result<()> {
    // Ft_MF_KeepGfx.
    falcon.change_motion_state_keeping_effects(CATCH, falcon_assets, 0.0)?;
    // x2222_b2 has no simulation reader.
    // ftCommon_8007E2F4(fp, 511), ftCommon_8007E2FC.
    falcon.core.status.grab_exclusions = GrabExclusions::ALL;
    falcon.core.clear_movement();
    if victim.physics.ground_or_air == GroundOrAir::Ground {
        // ftCo_800DB368(vic_fp, fp): Falcon hangs from the victim's TransN2;
        // accessory4 = ftCa_SpecialLw_800E550C.
        capture_captain::attach(
            &mut falcon.core,
            &mut victim.core,
            falcon_assets,
            victim_assets,
        );
    }
    falcon.core.combat.grab = Some(GrabLink::Holding {
        victim: victim.spawn_number,
        vertical_offset: 0.0,
    });
    capture_captain::enter(victim, falcon, victim_assets, falcon_assets)
}

/// ftCa_SpecialLw_800E550C, accessory4 while hanging: Falcon takes the
/// victim's position.
pub fn follow_victim(f: &mut Fighter) {
    if f.motion_state.action != CATCH || !attached(f) {
        return;
    }
    if let Some(position) = f.core.partner_position {
        f.core.follow(position);
    }
}

/// ftCa_SpecialHiCatch_Anim: at the end, doCatchAnim: the throw, then the
/// release (ftCo_800DE2A8, ftCo_800DE7C0) the scene runs on the pair.
pub fn catch_anim<C: CaptainFamily>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.commands.variables[0] = 0;
        let dive = dive::<C>(f);
        dive.throw_drift = false;
        dive.velocity = hsd_types::Vec2::new(0.0, 0.0);
        // Ft_MF_Unk19 | Ft_MF_KeepGfx.
        f.change_motion_state_keeping_effects(THROW, p.assets, 0.0)?;
        // ftCommon_8007E2F4(fp, 0).
        f.core.status.grab_exclusions = GrabExclusions::NONE;
        f.combat.special_throw_release = true;
    }
    Ok(None)
}

/// ftCa_SpecialHiCatch_IASA / _Phys are empty.
pub fn catch_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    special_hi::finish_update(f, &p);
}

/// ftCa_SpecialHiCatch_Coll: no collision while hanging (x221B_b7).
pub fn catch_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !attached(f) {
        let c = &mut f.core;
        air::begin_map(
            &c.physics,
            &mut c.collision,
            &mut c.skeleton,
            c.animation.root,
        );
        special_hi::stay_airborne(f, &mut p);
    }
    Ok(())
}

/// ftCa_SpecialHiThrow0_Anim: Fall at the end; every tick spends the jumps
/// and locks the ECB (ftCommon_8007D60C); the script's cmd_vars[0] starts
/// the drift.
pub fn throw_anim<C: CaptainFamily>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.change_motion_state(CommonMotionState::Fall.into(), p.assets)?;
    }
    f.leave_ground_with_spent_jumps();
    if f.commands.variables[0] != 0 {
        f.commands.variables[0] = 0;
        dive::<C>(f).throw_drift = true;
    }
    Ok(None)
}

/// ftCa_SpecialHiThrow0_Phys (800E5288): the dive's drift with the catch
/// gravity on the carried vertical velocity; no fused sites.
pub fn throw_physics<C: CaptainFamily>(f: &mut Fighter, p: PhysicsPhase<'_>) {
    if dive::<C>(f).throw_drift {
        special_hi::drift::<C>(f, p.assets);
        let gravity = attributes::<C>(f).catch_gravity;
        let terminal = f.attributes.air.terminal_velocity;
        let carried = dive::<C>(f).velocity.y;
        // 800E52D4: fsubs.
        let own = f.physics.self_velocity.y - carried;
        // ftCommon_Fall.
        f.physics.self_velocity.y -= gravity;
        if f.physics.self_velocity.y < -terminal {
            f.physics.self_velocity.y = -terminal;
        }
        let vertical = f.physics.self_velocity.y - own;
        dive::<C>(f).velocity.y = vertical;
    } else {
        special_hi::root_motion_air(f);
    }
    special_hi::finish_update(f, &p);
}

/// ftCa_SpecialHiThrow0_Coll: landing enters LandingFallSpecial.
pub fn throw_collision<C: CaptainFamily>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    let c = &mut f.core;
    air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    if air::collide_air_dodge(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
    ) {
        let lag = attributes::<C>(f).landing_lag;
        f.enter_special_landing(p.assets.expect("Falcon Dive landing assets"), false, lag)?;
    }
    Ok(())
}

/// ftCa_SpecialHiCatch_IASA / ftCa_SpecialHiThrow0_IASA are empty.
pub fn no_input(_: &mut Fighter, _: melee_ft::fighter::state::InputPhase<'_>) {}
