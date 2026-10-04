//! Whirling Fortress, ftkoopaspecialhi.c (80135A24..80135F6C).
//!
//! Bowser withdraws into his shell (the script's cmd_vars pick the shell
//! model and make his limbs intangible) and spins. On the ground the stick
//! slides him until the script's cmd_vars[0]; walking off an edge
//! continues in the aerial row and landing while spinning returns to the
//! grounded one, both at the current frame. The aerial spin rises once,
//! drifts, and ends in a special fall.
use crate::{
    common::{self, change},
    init::Koopa,
};
use melee_ef::request::EffectRequest;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        part_rotation::Axis,
        state::{callbacks, AnimationPhase, CollisionPhase, MotionRow, PhysicsPhase},
        ActionId, Fighter, MotionEntryFlags,
    },
};
use melee_types::{combat::HurtStatus, GroundOrAir};

/// ftKp_MS_SpecialHi (359) and ftKp_MS_SpecialAirHi (360).
pub const GROUND: ActionId = ActionId(359);
pub const AIR: ActionId = ActionId(360);

/// efSync_Spawn(0x4DA, gobj, parts[0].joint): the spin's two models.
const SPIN_EFFECT: u16 = 0x4DA;
/// The ground/air switch keeps the spin going: ftCommon_GroundAirColl_MF
/// with KeepGfx, KeepColAnimHitStatus and KeepSfx (0x0C4C5292).
const SWITCH_FLAGS: MotionEntryFlags = MotionEntryFlags(0x0C4C_5292);
/// The script's cmd_vars[2] value that ends the aerial spin.
const SPIN_OVER: u32 = 2;
/// The hurt capsules' bones ftKp_SpecialN_80135780 makes intangible while
/// Bowser is in his shell: arms and legs, in retail's order.
const LIMB_BONES: [usize; 12] = [
    0x37, 0x30, 0x3C, 0x21, 0x3D, 0x22, 0x0F, 0x06, 0x10, 0x07, 0x13, 0x0A,
];

/// mv.kp for the Fortress rows (Fighter +234C, +2350).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Fortress {
    /// +0xC: frames spun, counted by both animation callbacks.
    pub frames: f32,
    /// +0x10: the last collision pass found Bowser on a floor.
    pub on_floor: bool,
}

pub const fn rows() -> [MotionRow; 2] {
    [
        common::row(
            GROUND,
            311,
            ground_anim,
            common::no_input,
            ground_physics,
            ground_collision,
        ),
        common::row(
            AIR,
            312,
            air_anim,
            common::no_input,
            air_physics,
            air_collision,
        ),
    ]
}

fn attributes(f: &Fighter) -> &crate::attributes::FortressAttributes {
    &f.character.get::<Koopa>().attributes.fortress
}

fn scratch(f: &mut Fighter) -> &mut Fortress {
    &mut f.character.get_mut::<Koopa>().fortress
}

/// ftKp_SpecialHi_Enter (80135A24) / ftKp_SpecialAirHi_Enter (80135B08).
/// take_dmg_cb and death2_cb become ftKp_Init_80132B38, which is empty.
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    change(
        f,
        if air { AIR } else { GROUND },
        MotionEntryFlags(0),
        0.0,
        a,
    )
    .expect("Whirling Fortress assets");
    f.commands.variables = [0; 4];
    if air {
        let (limit, rise) = {
            let s = attributes(f);
            (s.ground_max, s.air_rise)
        };
        common::clamp_ground_velocity(f, limit);
        f.physics.self_velocity.y = rise;
    } else {
        f.physics.self_velocity.y = 0.0;
    }
    f.physics.jumps_used = f.core.attributes.jumping.max_jumps as u8;
    *scratch(f) = Fortress::default();
    f.effects.push(EffectRequest::SyncAttached {
        id: SPIN_EFFECT,
        bone: 0,
    });
    // x2219_b0 and Fighter_SetEffectHitlagCallbacks.
    f.effect_state.destroy_on_state_change = true;
    f.effect_state.hitlag_callbacks = true;
    // ftAnim_8006EBA4.
    f.step_animation(a);
    if air {
        // ftKp_SpecialAirHi_Enter ends by running ftKp_SpecialHi_Anim.
        spin(f);
        assert!(
            f.animation.frames_remaining(&f.skeleton),
            "ftKp_SpecialHi_Anim: the aerial spin ended on its first frame"
        );
    }
}

/// The head of both animation callbacks: one more frame spun, then the
/// shell follows the script.
fn spin(f: &mut Fighter) {
    // 80135C28: fadds.
    scratch(f).frames += 1.0;
    follow_shell(f);
}

/// ftKp_SpecialN_80135780 (80135780): the script's flag (cmd_vars[2] on the
/// ground, cmd_vars[1] in the air, bit 0) puts Bowser in his shell: model
/// group 0's second selection, the limbs' capsules intangible, the held
/// item and articles hidden. Clear, everything returns.
fn follow_shell(f: &mut Fighter) {
    let flag = if f.physics.ground_or_air == GroundOrAir::Ground {
        f.commands.variables[2]
    } else {
        f.commands.variables[1]
    };
    let shelled = flag & 1 != 0;
    select_model(f, i32::from(shelled));
    if shelled {
        for bone in LIMB_BONES {
            set_capsule(f, bone, HurtStatus::Intangible);
        }
    } else {
        // ftColl_8007B0C0(gobj, HurtCapsule_Enabled).
        f.core.set_hurt_capsules(HurtStatus::Normal);
    }
    show_held_item(f, !shelled);
    // x221E_b4.
    f.commands.articles_visible = !shelled;
}

/// ftParts_80074B0C(gobj, 0, variant): model group 0.
fn select_model(f: &mut Fighter, variant: i32) {
    f.character.get_mut::<Koopa>().model_group = variant;
    f.commands.model_selections.insert(0, variant);
}

/// ftColl_8007B128 (8007B128): the first capsule on `bone` takes `status`.
fn set_capsule(f: &mut Fighter, bone: usize, status: HurtStatus) {
    let overrides = &mut f.core.commands.capsule_overrides;
    let mut existing = false;
    for entry in overrides.iter_mut() {
        if entry.0 == bone {
            entry.1 = status;
            existing = true;
            break;
        }
    }
    if !existing {
        overrides.push((bone, status));
    }
}

/// ftCommon_8007F5CC. A special cannot start with an item in hand here
/// (SPECIALS_KEEP_HELD_ITEM is false), so only the visibility bit changes.
fn show_held_item(f: &mut Fighter, visible: bool) {
    assert!(
        f.held_item.is_none(),
        "ftCommon_8007F5CC: Whirling Fortress holding an item"
    );
    f.commands.held_item_hidden = !visible;
}

/// ftKp_SpecialHi_Anim (80135C08): Wait at the animation's end.
fn ground_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    spin(f);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::wait(f, p.assets)?;
    }
    Ok(None)
}

/// ftKp_SpecialAirHi_Anim (80135C60): once the script marks the spin over
/// (cmd_vars[2] == 2) the effect goes and Bowser falls special; so does the
/// animation's end.
fn air_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    spin(f);
    if f.commands.variables[2] == SPIN_OVER {
        if f.physics.ground_or_air == GroundOrAir::Air {
            f.leave_ground_with_spent_jumps();
            // efLib_DestroyAll, x2219_b0 and the hitlag callbacks.
            f.effects.push(EffectRequest::DestroyOwned);
            f.effect_state.destroy_on_state_change = false;
            f.effect_state.hitlag_callbacks = false;
            fall_special(f, p.assets)?;
        }
    } else if !f.animation.frames_remaining(&f.skeleton) {
        f.leave_ground_with_spent_jumps();
        fall_special(f, p.assets)?;
    }
    Ok(None)
}

/// ftCo_80096900(gobj, 1, 0, 1, 1.0, x7C), or a plain fall with no lag.
fn fall_special(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    let lag = attributes(f).landing_lag;
    if lag == 0.0 {
        common::fall(f, assets)
    } else {
        f.enter_special_fall(assets, true, false, true, 1.0, lag)
    }
}

/// ftKp_SpecialHi_Phys (80135D80): the stick slides Bowser until the
/// script's cmd_vars[0], then ordinary friction (ft_80084F3C).
fn ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    if f.commands.variables[0] != 0 {
        return common::ground_friction(f, p);
    }
    let (acceleration, limit) = {
        let s = attributes(f);
        (s.ground_accel, s.ground_max)
    };
    common::walk_toward_stick(f, 0.0, acceleration, limit);
    common::move_on_ground(f, &p);
}

/// ftKp_SpecialAirHi_Phys (80135DE0): the spin's own gravity and drift
/// until the script's cmd_vars[0], then the ordinary fall (ft_80084DB0).
fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    if f.commands.variables[0] != 0 {
        return callbacks::physics::fall(f, p);
    }
    let (gravity, terminal, acceleration, limit) = {
        let s = attributes(f);
        (s.gravity, s.terminal_velocity, s.air_accel, s.air_max)
    };
    common::gravity(f, gravity, terminal);
    common::drift_with_friction(f, 0.0, acceleration, limit);
    f.core.finish_air_update(p.assets, p.wind);
}

/// ftKp_SpecialHi_Coll_inline: while the script's cmd_vars[3] is set and
/// Bowser is on a floor, the model root tips with it (ftPartSetRotX of
/// facing * atan2f(normal.x, normal.y)); otherwise it is level.
fn tip_with_floor(f: &mut Fighter) {
    let angle = if f.commands.variables[3] != 0 && scratch(f).on_floor {
        let normal = f.collision.data.floor.normal;
        // 801360C4: fmuls.
        f.physics.facing * melee_lb::trigf::atan2f(normal.x, normal.y)
    } else {
        0.0
    };
    f.core.set_part_rotation(0, Axis::X, angle);
}

/// ftKp_SpecialHi_Coll (80135E4C): off the floor (ft_80082708) the spin
/// continues in the aerial row with every jump spent.
fn ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::stays_grounded(f, &mut p) {
        scratch(f).on_floor = true;
    } else {
        let assets = p.assets.expect("Whirling Fortress collision assets");
        f.leave_ground_with_spent_jumps();
        let frame = f.animation.frame;
        change(f, AIR, SWITCH_FLAGS, frame, assets)?;
        let limit = attributes(f).air_max;
        common::clamp_self_velocity_x(f, limit);
        f.effect_state.hitlag_callbacks = true;
        scratch(f).on_floor = false;
    }
    follow_shell(f);
    tip_with_floor(f);
    Ok(())
}

/// ftKp_SpecialAirHi_Coll_inline: landing continues in the grounded row at
/// the current frame. Retail first tests the spin's frames against the
/// attribute window with two comparisons that exclude each other
/// (80135FC4..80135FD0: x70 < frames and frames < x70), so the rewound
/// start frame it guards is never taken.
fn land(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    f.land();
    let frame = f.animation.frame;
    change(f, GROUND, SWITCH_FLAGS, frame, assets)?;
    let limit = attributes(f).ground_max;
    common::clamp_ground_velocity(f, limit);
    Ok(())
}

/// ftKp_SpecialAirHi_Coll (80135F6C): rising, the ordinary pass
/// (ft_80081D0C); falling, the ledge-aware pass on either side
/// (ft_CheckGroundAndLedge(gobj, 0)), with a ledge caught when no floor
/// is.
fn air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    let assets = p.assets.expect("Whirling Fortress collision assets");
    if f.physics.self_velocity.y >= 0.0 {
        if common::lands(f, &mut p) {
            land(f, assets)?;
            f.effect_state.hitlag_callbacks = true;
            scratch(f).on_floor = true;
        } else {
            scratch(f).on_floor = false;
        }
        follow_shell(f);
        tip_with_floor(f);
    } else if common::lands_or_finds_ledge_either_side(f, &mut p) {
        land(f, assets)?;
        scratch(f).on_floor = true;
        follow_shell(f);
        tip_with_floor(f);
        f.effect_state.hitlag_callbacks = true;
    } else {
        scratch(f).on_floor = false;
        if f.try_grab_ledge(assets, p.map)? {
            // ftCliffCommon_80081298 enters CliffCatch itself;
            // ftCliffCommon_80081370 then runs a second time.
            f.enter_cliff_catch(assets, p.map)?;
        }
    }
    Ok(())
}
