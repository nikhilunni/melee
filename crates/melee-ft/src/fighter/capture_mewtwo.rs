//! A Confusion victim, ftCo_CaptureMewtwo.c and ftCo_ThrownMewtwo.c
//! (800BCF18..800BD1D0).
//!
//! Mewtwo's side special catches a fighter without leaving its own motion.
//! The victim hangs from the captor's TransN2 (ftCo_800DB368) and plays the
//! captor's ThrownMewtwo animation, placed each tick by accessory1
//! (ftCo_800DE508) like a thrown fighter; the captor's script then lets it
//! go (ftCo_800DE2A8) into DamageFall with the throw record's hit pending.
use super::{
    assets::{FighterAssets, Result},
    capture_captain,
    grab::GrabLink,
    grab_throw, hit_log, Fighter,
};
use melee_types::CommonMotionState as S;

/// ftCo_SM_ThrownMewtwo / ftCo_SM_ThrownMewtwoAir: the captor's animations
/// the victim plays.
pub const VICTIM_MOTION: i32 = 292;
pub const AIR_VICTIM_MOTION: i32 = 293;

/// ftCo_800BCF18 (800BCF18) / ftCo_800BD000 (800BD000), Confusion's
/// grabbed_cb from a grounded or an airborne captor: CaptureMewtwo(Air),
/// which has no animation and no callbacks, then at once ftCo_800BD0E8's
/// ThrownMewtwo(Air).
pub fn enter(
    victim: &mut Fighter,
    captor: &mut Fighter,
    victim_assets: &FighterAssets,
    captor_assets: &FighterAssets,
    air: bool,
) -> Result<()> {
    // ftCommon_8007DB58.
    victim.interrupt_actions(victim_assets);
    // ftCo_8009750C drops a heavy item; ftCo_800DD168 releases the victim's
    // own victim.
    if victim.core.held_item.as_ref().is_some_and(|held| held.heavy) {
        unimplemented!("ftCo_8009750C: caught by Confusion with a heavy item");
    }
    if victim.core.combat.grab.is_some() {
        unimplemented!("ftCo_800DD168: caught by Confusion while holding another");
    }
    victim.core.physics.facing = captor.core.physics.facing;
    // ftCo_800DB368(captor, victim).
    capture_captain::attach(
        &mut victim.core,
        &mut captor.core,
        victim_assets,
        captor_assets,
    );
    let (capture, thrown, motion) = if air {
        (S::CaptureMewtwoAir, S::ThrownMewtwoAir, AIR_VICTIM_MOTION)
    } else {
        (S::CaptureMewtwo, S::ThrownMewtwo, VICTIM_MOTION)
    };
    victim.change_motion_state(capture.into(), victim_assets)?;
    // ftCommon_8007D5D4.
    victim.core.leave_ground();
    // ftCommon_8007E2F4(fp, 0x1FF).
    victim.core.status.grab_exclusions = super::ledge::GrabExclusions::ALL;
    victim.step_animation(victim_assets);
    // ftCommon_8007E2FC.
    victim.core.clear_movement();
    victim.core.combat.grab = Some(GrabLink::Captured {
        captor: captor.core.spawn_number,
    });
    // ftCo_800BD0E8 (800BD0E8): the captor's facing again, then the thrown
    // row with the captor's animation. x2222_b6 (Ft_MF_FreezeState) is set
    // by no supported mode.
    victim.core.physics.facing = captor.core.physics.facing;
    let source = grab_throw::throw_source(captor_assets, victim_assets, motion);
    victim.change_motion_state_with_source(thrown.into(), victim_assets, 0.0, 1.0, Some(source))?;
    victim.core.status.grab_exclusions = super::ledge::GrabExclusions::ALL;
    victim.step_animation(victim_assets);
    grab_throw::update_constraint(
        &mut victim.core,
        &mut captor.core,
        victim_assets,
        captor_assets,
    );
    Ok(())
}

/// ftCo_ThrownMewtwo_Anim / ftCo_ThrownMewtwoAir_Anim (empty): the
/// animation still advances.
pub fn animation(
    fighter: &mut Fighter,
    phase: super::state::AnimationPhase<'_>,
) -> Result<Option<crate::anim::WaitChoice>> {
    fighter.step_animation(phase.assets);
    Ok(None)
}

/// ftCo_ThrownMewtwo_Coll (empty).
pub fn collision(_: &mut Fighter, _: super::state::CollisionPhase<'_>) -> Result<()> {
    Ok(())
}

/// ftMewtwo_SetGrabVictim (ftMt_SpecialS_Anim, 80146858): ftCo_800DE2A8,
/// which logs the captor's throw record 0 on the victim (ftColl_80076640)
/// and sets it free at the captor's TransN2, then ftCo_80090780: DamageFall.
/// That motion change clears the record's knockback (dmg.kb_applied,
/// fighter.c:1043), so the victim's Fighter_ProcessHit only applies the
/// logged damage: Confusion never launches.
pub fn release(
    victim: &mut Fighter,
    captor: &mut Fighter,
    victim_assets: &FighterAssets,
    captor_assets: &FighterAssets,
    map: &mut melee_mp::CollMap,
    _rng: &mut gekko_math::HsdRng,
) -> Result<()> {
    let hit = grab_throw::release_captured(
        &mut victim.core,
        &mut captor.core,
        victim_assets,
        captor_assets,
        map,
        true,
    );
    // ftColl_80076640: x1838_percentTemp and x183C_applied.
    let damage = hit.percent_damage;
    victim.core.combat.frame_damage += damage;
    victim.core.combat.frame_max_damage = victim
        .core
        .combat
        .frame_max_damage
        .max(hit_log::damage_count(damage));
    victim.enter_damage_fall(victim_assets)
}
