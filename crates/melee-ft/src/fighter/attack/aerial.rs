//! Five common aerials, ftCommon/ftCo_AttackAir.c (8008CD68..8008D5FC).
use crate::fighter::{
    assets::{FighterAssets, Result},
    state::{AnimationPhase, CollisionPhase, InputPhase},
    ActionId, Fighter, Interaction, MotionData,
};
use crate::input::{Buttons, FighterInput, InputCommonData};
use gekko_math::msl::fabsf;
use melee_types::CommonMotionState as S;

/// ftCo_800DF478 (800DF478): a threshold crossing on either C-stick axis.
pub fn cstick_edge(input: &FighterInput, common: &InputCommonData) -> bool {
    let old = input.previous.cstick;
    let new = input.current.cstick;
    (fabsf(old.x) < common.aerial_horizontal_threshold
        && fabsf(new.x) >= common.aerial_horizontal_threshold)
        || (fabsf(old.y) < common.aerial_vertical_threshold
            && fabsf(new.y) >= common.aerial_vertical_threshold)
}

/// ftCo_AttackAir_CheckItemThrowInput (8008CD68), item-free input predicate.
#[inline(always)]
pub fn requested(input: &FighterInput, common: &InputCommonData) -> bool {
    input.pressed.intersects(Buttons::A) || cstick_edge(input, common)
}

/// ftCo_AttackAir_GetMsidFromCStick (8008CE68): C-stick edges override A's stick.
pub fn select(input: &FighterInput, common: &InputCommonData, facing: f32) -> S {
    let stick = if cstick_edge(input, common) {
        input.current.cstick
    } else {
        input.current.stick
    };
    let angle = melee_lb::trigf::atan2f(stick.y, fabsf(stick.x));
    if fabsf(stick.x) < common.aerial_horizontal_threshold
        && fabsf(stick.y) < common.aerial_vertical_threshold
    {
        S::AttackAirN
    } else if angle > common.tilt_angle {
        S::AttackAirHi
    } else if angle < -common.tilt_angle {
        S::AttackAirLw
    // retail 8008CF38: separate fmuls, no fusion.
    } else if stick.x * facing >= 0.0 {
        S::AttackAirF
    } else {
        S::AttackAirB
    }
}

/// ftCo_AttackAir_EnterFromCStick / EnterFromMsid (8008CF70 / 8008CFAC).
/// The character table supplies decideFighter's entry. Held-item throw selection
/// (ftCo_AttackAir.c:52-61) remains unported: HeldItem is an explicit status stop.
pub fn enter(fighter: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    let state = select(
        &fighter.core.input,
        &assets.input,
        fighter.core.physics.facing,
    );
    enter_action(fighter, assets, state.into())
}

/// ftCo_AttackAir_EnterFromMsid (8008CFAC) for an already chosen row: the
/// common aerials, or a character's counterpart (Peach's float aerials,
/// ftPe_8011BF34).
pub fn enter_action(fighter: &mut Fighter, assets: &FighterAssets, action: ActionId) -> Result<()> {
    let retained_drop_timer = fighter.retained_drop_timer();
    let fast_fall = fighter.core.physics.fast_fall;
    fighter.core.commands.allow_interrupt = false;
    fighter.core.commands.variables[0] = 0;
    fighter.core.commands.grab_release = false;
    fighter.core.commands.throw_reverse = false;
    fighter.core.commands.rapid_jab_loop_end = false;
    fighter.change_motion_state(action, assets)?;
    // Ft_MF_KeepFastFall; restore before the entry animation step and physics.
    fighter.core.physics.fast_fall = fast_fall;
    fighter.core.state_data = MotionData::Aerial {
        retained_drop_timer,
    };
    fighter.step_animation(assets);
    fighter.core.status.interaction = Interaction::Attack;
    Ok(())
}

/// ftCo_AttackAir_Anim (8008D010): throw flag b3 reverses facing once.
pub fn animation(
    fighter: &mut Fighter,
    phase: AnimationPhase<'_>,
) -> Result<Option<crate::anim::WaitChoice>> {
    fighter.step_animation(phase.assets);
    if std::mem::take(&mut fighter.core.commands.grab_release) {
        fighter.core.physics.facing = -fighter.core.physics.facing;
    }
    if !fighter
        .core
        .animation
        .frames_remaining(&fighter.core.skeleton)
    {
        fighter.change_motion_state(S::Fall.into(), phase.assets)?;
    }
    Ok(None)
}

/// AttackAirN/F/B/Hi/Lw_IASA (8008D08C..8008D4AC) share DO_IASA.
/// Its item-free branch admits another aerial or an air jump after script unlock;
/// unlike Fall_IASA it has no special, air-dodge or Peach float predicates.
pub fn input(fighter: &mut Fighter, phase: InputPhase<'_>) {
    if !fighter.core.commands.allow_interrupt {
        return;
    }
    // ftCo_80095328 needs a held item; ftCo_800D7100 catches one.
    if fighter.try_aerial_item_catch(phase.assets) {
        return;
    }
    fighter.character.air_dodge_tether();
    if requested(&fighter.core.input, &phase.assets.input) {
        (fighter.character.table().enter_aerial)(fighter, phase.assets).expect("aerial interrupt");
    } else if fighter.aerial_jump_requested(phase.assets) {
        fighter
            .enter_aerial_jump(phase.assets)
            .expect("aerial jump interrupt");
    }
}

/// ftCo_AttackAir_Coll (8008D5D4) -> ft_80082C74: ordinary air collision,
/// no ledge-grab or soft-landing predicate before LandingAir_EnterWithLag.
pub fn collision(fighter: &mut Fighter, phase: CollisionPhase<'_>) -> Result<()> {
    let assets = phase.assets.expect("aerial collision assets");
    crate::collision::air::begin_map(
        &fighter.core.physics,
        &mut fighter.core.collision,
        &mut fighter.core.skeleton,
        fighter.core.animation.root,
    );
    if crate::collision::air::collide_air_dodge(
        &mut fighter.core.physics,
        &mut fighter.core.collision,
        phase.map,
        &mut fighter.core.skeleton,
        fighter.core.animation.root,
    ) {
        fighter.land_from_aerial(assets)?;
    }
    Ok(())
}
