//! Swallowing and spitting the inhaled fighter, ftkirbyspecialn.c
//! (SpecialNDrink0 367 / 378 and SpecialNSpit0 369 / 380). The ability a
//! swallow leaves is `crate::ability`; a hit's draw for its loss is here.
//!
//! The rows for a swallowed or spat item (SpecialNDrink1 368 / 379,
//! SpecialNSpit1 370 / 381: it_802F28C8, it_802F295C) are not ported.
use crate::{
    common,
    init::Kirby,
    mouthful::{self, CAPTURE_COUNTERPART},
};
use hsd_types::Vec2;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        capture_kirby::{CaptorRequest, StarParameters},
        ledge::GrabExclusions,
        state::{AnimationPhase, CollisionPhase, MotionRow},
        ActionId, Fighter,
    },
};
use melee_types::FighterKind;

pub const SWALLOW: ActionId = ActionId(367);
pub const SPIT: ActionId = ActionId(369);
pub const AIR_SWALLOW: ActionId = ActionId(378);
pub const AIR_SPIT: ActionId = ActionId(380);

/// ftKb_EatWait_IASA's change into the swallow: KeepGfx.
const SWALLOW_ENTRY: u32 = 0x2;
/// Its change into the spit: KeepGfx and SkipModel.
const SPIT_ENTRY: u32 = 0x12;
/// ftKb_MF_SpecialN_Coll: ftCommon_GroundAirColl_MF with KeepGfx.
const SWALLOW_COUNTERPART: u32 = 0x0C4C_5082;

pub const fn rows() -> [MotionRow; 4] {
    use melee_ft::fighter::state::callbacks::physics;
    [
        common::row(
            SWALLOW,
            318,
            swallow_anim,
            common::no_input,
            physics::guard_on,
            swallow_collision,
        ),
        common::row(
            SPIT,
            319,
            spit_anim,
            common::no_input,
            physics::guard_on,
            spit_collision,
        ),
        common::row(
            AIR_SWALLOW,
            318,
            swallow_anim,
            common::no_input,
            physics::air_friction,
            air_swallow_collision,
        ),
        common::row(
            AIR_SPIT,
            319,
            spit_anim,
            common::no_input,
            physics::air_friction,
            air_spit_collision,
        ),
    ]
}

fn enter(f: &mut Fighter, state: ActionId, flags: u32, assets: &FighterAssets) -> Result<()> {
    mouthful::change(f, state, flags, assets)?;
    // x2222_b2.
    f.character.get_mut::<Kirby>().cape_turn_blocked = true;
    mouthful::install_callbacks(f);
    f.step_animation(assets);
    mouthful::ungrabbable(f);
    Ok(())
}

/// ftKb_EatWait_FighterEat / ftKb_SpecialAirNCaptureWait_FighterEat.
pub(crate) fn enter_swallow(f: &mut Fighter, air: bool, assets: &FighterAssets) -> Result<()> {
    enter(
        f,
        if air { AIR_SWALLOW } else { SWALLOW },
        SWALLOW_ENTRY,
        assets,
    )
}

/// ftKb_EatWait_FighterSpit / ftKb_SpecialAirNCaptureWait_FighterSpit.
pub(crate) fn enter_spit(f: &mut Fighter, air: bool, assets: &FighterAssets) -> Result<()> {
    enter(f, if air { AIR_SPIT } else { SPIT }, SPIT_ENTRY, assets)
}

/// What both stars read of Kirby's attributes.
fn star(f: &Fighter, velocity: Vec2, deceleration: f32, duration: f32) -> StarParameters {
    let a = &f.character.get::<Kirby>().attributes.inhale;
    StarParameters {
        velocity,
        deceleration,
        duration,
        decay: a.star_decay,
        mash_decrement: a.star_mash_decrement,
        collision_box: a.star_box,
        exit_velocity: a.star_exit_velocity,
        growth_frames: a.regrow_frames,
        growth: a.shrink,
    }
}

/// ftKb_SpecialN_800F58AC (800F58AC) / ftKb_SpecialN_800F5A88: the spat
/// star flies level at +88.
fn spit_star(f: &Fighter) -> StarParameters {
    let a = &f.character.get::<Kirby>().attributes.inhale;
    star(
        f,
        Vec2::new(a.spit_speed, 0.0),
        a.spit_deceleration,
        a.spit_star_time,
    )
}

/// ftKb_SpecialN_800F58D8 (800F58D8) / ftKb_SpecialN_800F5AB0: the copy
/// star leaves at the angle +90 and the speed +94 (separate fmuls).
fn copy_star(f: &Fighter) -> StarParameters {
    let a = &f.character.get::<Kirby>().attributes.inhale;
    let cosine = gekko_math::msl::cosf(a.copy_star_angle);
    let across = a.copy_star_speed * cosine;
    let up = a.copy_star_speed * gekko_math::msl::sinf(a.copy_star_angle);
    star(
        f,
        Vec2::new(across, up),
        a.copy_star_deceleration,
        a.copy_star_time,
    )
}

/// The rows' ends: ft_8008A2BC on the floor, ftCo_Fall_Enter in the air.
fn finish(f: &mut Fighter, p: &mut AnimationPhase<'_>, air: bool) -> Result<()> {
    if f.animation.frames_remaining(&f.skeleton) {
        return Ok(());
    }
    assert!(
        f.combat.mouth_request.is_none(),
        "ftKb_SpecialNSpit_Anim / ftKb_SpecialNDrink_Anim: the release on the row's last frame"
    );
    common::seal_graphics(f, p.assets, p.rng);
    if air {
        common::enter_fall(f, p.assets)
    } else {
        common::finish(f, false, p.assets)
    }
}

/// ftKb_SpecialNSpit_Anim (800F7808) / ftKb_SpecialAirNSpit_Anim
/// (800F79C0): on the script's flag the fighter leaves as a star
/// (ftCo_800DE2CC, ftCo_800BDB58, ftColl_8007B8CC, run by the scene on the
/// pair) and Kirby can be grabbed again.
fn spit_anim(f: &mut Fighter, mut p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if f.commands.variables[0] != 0 && mouthful::holding(f) {
        f.status.grab_exclusions = GrabExclusions::NONE;
        f.combat.mouth_request = Some(CaptorRequest::Spit(spit_star(f)));
        f.commands.variables[0] = 0;
    }
    let air = f.motion_state.action == AIR_SPIT;
    finish(f, &mut p, air)?;
    Ok(None)
}

/// ftKb_SpecialNDrink_Anim (800F7B20) / ftKb_SpecialAirNDrink_Anim
/// (800F7C94): on the script's flag the fighter leaves as the copy star
/// (ftCo_800DE2CC, ftCo_800BE000) and Kirby takes its ability.
fn swallow_anim(f: &mut Fighter, mut p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if f.commands.variables[0] != 0 && mouthful::holding(f) {
        f.status.grab_exclusions = GrabExclusions::NONE;
        f.combat.mouth_request = Some(CaptorRequest::Swallow(copy_star(f)));
        // u.kb.xE0 = ftCo_800BD9E0(gobj, victim), read at the catch.
        let ability = f.character.get::<Kirby>().mouthful_ability;
        crate::ability::take(f, ability);
        f.commands.variables[0] = 0;
    }
    let air = f.motion_state.action == AIR_SWALLOW;
    finish(f, &mut p, air)?;
    Ok(None)
}

/// ftKb_SpecialNDrink_Coll (800F8F08) -> fn_800F6E68.
fn swallow_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::grounded(f, &mut p) {
        return Ok(());
    }
    f.leave_ground();
    mouthful::counterpart(
        f,
        AIR_SWALLOW,
        SWALLOW_COUNTERPART,
        p.assets.expect("Inhale assets"),
    )?;
    mouthful::ungrabbable(f);
    Ok(())
}

/// ftKb_SpecialAirNDrink_Coll (800F8F58) -> fn_800F6F50.
fn air_swallow_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    f.land();
    mouthful::counterpart(
        f,
        SWALLOW,
        SWALLOW_COUNTERPART,
        p.assets.expect("Inhale assets"),
    )?;
    mouthful::ungrabbable(f);
    Ok(())
}

/// ftKb_SpecialNSpit_Coll (800F8E68) -> fn_800F6C98.
fn spit_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::grounded(f, &mut p) {
        return Ok(());
    }
    f.leave_ground();
    mouthful::counterpart(
        f,
        AIR_SPIT,
        CAPTURE_COUNTERPART,
        p.assets.expect("Inhale assets"),
    )?;
    mouthful::ungrabbable(f);
    Ok(())
}

/// ftKb_SpecialAirNSpit_Coll (800F8EB8) -> fn_800F6D80.
fn air_spit_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    f.land();
    mouthful::counterpart(
        f,
        SPIT,
        CAPTURE_COUNTERPART,
        p.assets.expect("Inhale assets"),
    )?;
    mouthful::ungrabbable(f);
    Ok(())
}

/// ftKb_SpecialN_800F5BA4 (800F5BA4) / ftKb_SpecialN_800F5C34 (800F5C34),
/// from Fighter_ProcessHit: a hit on a Kirby with a copied ability and
/// nothing in his mouth draws for its loss, one in +B0, and a 0 throws the
/// ability away as a star (ftKb_SpecialN_800F5D04).
pub fn ability_loss_draw(f: &mut Fighter, cape: bool, rng: &mut gekko_math::HsdRng) {
    let state = f.character.get::<Kirby>();
    // hat.x8_b0 (Player_GetFlagsAEBit1) is set in no supported mode;
    // victim_gobj names the other fighter of a grab pair, either way round.
    if state.copied == FighterKind::Kirby || f.combat.grab.is_some() || cape {
        return;
    }
    let odds = gekko_math::msl::fctiwz(state.attributes.inhale.ability_loss_odds);
    if rng.randi(odds) == 0 {
        crate::ability::lose(f, true);
    }
}
