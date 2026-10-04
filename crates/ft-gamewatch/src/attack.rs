//! The ground attacks with an article: the jab and rapid jab
//! (ftgamewatchattack11.c, ftgamewatchattack100.c), the down tilt
//! (ftgamewatchattacklw3.c) and the forward smash (ftgamewatchattacks4.c).
//! Their rows reuse the common callbacks; each Coll also puts the articles
//! away once the fighter is airborne (ftGw_Init_8014A538).
use crate::articles::{self, Accessory};
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{self, callbacks, AnimationPhase, CollisionPhase, InputPhase},
        ActionId, Fighter, MotionData, MotionRow,
    },
    input::WaitPredicate as P,
};
use melee_types::CommonMotionState as S;

pub const ATTACK_11: ActionId = ActionId(341);
pub const ATTACK_100_START: ActionId = ActionId(342);
pub const ATTACK_100_LOOP: ActionId = ActionId(343);
pub const ATTACK_100_END: ActionId = ActionId(344);
pub const ATTACK_LW3: ActionId = ActionId(345);
pub const ATTACK_S4: ActionId = ActionId(346);

pub const fn rows() -> [MotionRow; 6] {
    [
        // ftGw_Attack11_Anim/IASA/Phys are the common jab's.
        MotionRow {
            action: ATTACK_11,
            collision,
            ..state::COMMON[S::Attack11 as usize]
        },
        MotionRow {
            action: ATTACK_100_START,
            anim: rapid_start_animation,
            collision,
            ..state::COMMON[S::Attack100Start as usize]
        },
        MotionRow {
            action: ATTACK_100_LOOP,
            anim: rapid_loop_animation,
            collision,
            ..state::COMMON[S::Attack100Loop as usize]
        },
        MotionRow {
            action: ATTACK_100_END,
            collision,
            ..state::COMMON[S::Attack100End as usize]
        },
        MotionRow {
            action: ATTACK_LW3,
            anim: down_tilt_animation,
            iasa: down_tilt_input,
            collision,
            ..state::COMMON[S::AttackLw3 as usize]
        },
        MotionRow {
            action: ATTACK_S4,
            iasa: forward_smash_input,
            collision,
            ..state::COMMON[S::AttackS4S as usize]
        },
    ]
}

/// ftGw_Attack11_Coll (8014C1B4) and its five siblings: ft_80084104, then
/// ftGw_Init_8014A538.
fn collision(f: &mut Fighter, phase: CollisionPhase<'_>) -> Result<()> {
    callbacks::collision::escape(f, phase)?;
    articles::airborne_cleanup(f);
    Ok(())
}

/// ftGw_Attack11_Enter (8014C07C): checkAttack11 on the character's row,
/// then the sprayer's accessory.
pub fn enter_jab(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    if f.enter_jab_row(ATTACK_11, false, assets)? {
        articles::install(f, Accessory::GreenhouseSetup);
    }
    Ok(())
}

/// ftGw_Attack100Start_Enter (8014C1E8): ftCo_800D6B00 on the character's
/// row, then the sprayer's accessory (installed even when an item was
/// picked up instead: the pickup's own motion change then removes it).
pub fn enter_rapid_jab(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    if f.enter_rapid_jab_row(ATTACK_100_START, assets)? {
        articles::install(f, Accessory::GreenhouseSetup);
    } else {
        unimplemented!(
            "ftGw_Attack100Start_Enter (ftgamewatchattack100.c:26): the Greenhouse accessory over an item pickup"
        );
    }
    Ok(())
}

/// ftGw_Attack100Start_Anim (8014C224) -> ftGw_Attack100Loop_Enter
/// (8014C2B8).
fn rapid_start_animation(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    f.advance_smash_charge(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.change_motion_state(ATTACK_100_LOOP, p.assets)?;
        articles::install(f, Accessory::GreenhouseMotion);
    }
    Ok(None)
}

/// ftGw_Attack100Loop_Anim (8014C308): ftCo_800D6C60 with
/// ftGw_Attack100End_Enter (8014C3A4).
fn rapid_loop_animation(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    f.advance_smash_charge(p.assets);
    if f.rapid_loop_ended(p.assets)? {
        f.change_motion_state(ATTACK_100_END, p.assets)?;
        articles::install(f, Accessory::GreenhouseMotion);
    }
    Ok(None)
}

/// ftGw_AttackLw3_Enter (8014ADB8): no new attack instance (the common
/// doEnter's x21EC is not installed) and no repeat latch.
pub fn enter_down_tilt(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    if f.try_item_pickup(assets)? {
        return Ok(());
    }
    f.commands.allow_interrupt = false;
    let retained_word = f.inherited_scratch_word();
    f.change_motion_state(ATTACK_LW3, assets)?;
    f.step_animation(assets);
    f.core.state_data = MotionData::Tilt { retained_word };
    articles::install(f, Accessory::ManholeSetup);
    Ok(())
}

/// ftGw_AttackLw3_Anim (8014AE3C): SquatWait when the animation ends
/// (ftCo_800D638C).
fn down_tilt_animation(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    f.advance_smash_charge(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.enter_squat_wait(p.assets)?;
    }
    Ok(None)
}

/// ftGw_AttackLw3_IASA (8014AE78): once the script allows it, the attacks,
/// then jump, dash, turn and walk unless down is held.
fn down_tilt_input(f: &mut Fighter, p: InputPhase<'_>) {
    if !f.commands.allow_interrupt {
        return;
    }
    f.interrupt_ground(
        p.assets,
        &[
            P::SmashSide,
            P::SmashUp,
            P::SmashDown,
            P::TiltSide,
            P::TiltUp,
            P::TiltDown,
            P::Jab,
            P::Jump,
            P::Dash,
            // ftCo_Squat_CheckInput is the pure test: down held keeps the
            // tilt and skips Turn and Walk.
            P::SquatHeld,
            P::Turn,
            P::Walk,
        ],
    )
    .expect("down tilt IASA");
}

/// ftGw_AttackS4_Enter (8014AA10), decideFighter's arm after the facing is
/// set: the one forward smash row, whatever the stick's angle.
pub fn enter_forward_smash(
    f: &mut Fighter,
    assets: &FighterAssets,
    _rng: &mut gekko_math::HsdRng,
) -> Result<()> {
    f.commands.allow_interrupt = false;
    f.commands.variables[0] = 0;
    let retained_word = f.inherited_scratch_word();
    f.change_motion_state(ATTACK_S4, assets)?;
    f.step_animation(assets);
    f.core.state_data = MotionData::Smash { retained_word };
    articles::install(f, Accessory::TorchSetup);
    Ok(())
}

/// ftGw_AttackS4_IASA (8014AAC4): ftCo_Wait_IASA once the script allows it.
fn forward_smash_input(f: &mut Fighter, p: InputPhase<'_>) {
    if f.commands.allow_interrupt {
        callbacks::input::wait(f, p);
    }
}
