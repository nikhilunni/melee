//! ftCo_8008EC90's grab-pair half (ftCo_Damage.c): what a launch does to a
//! grab pair. Fighter_ProcessHit runs it for each member in entity order;
//! the first member with knockback decides for both and leaves the other an
//! order in x1828, which that member's own ProcessHit then follows.
use super::assets::{FighterAssets, Result};
use super::grab::GrabLink;
use super::Fighter;

/// Fighter x1828: the reaction a partner's ftCo_8008EC90 chose for this
/// fighter's next Fighter_ProcessHit (0, no order, is `None`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PairHitOrder {
    /// 1: ftCommon_8007DB58, then ftCo_8008E908(gobj, 0.0), with no down or
    /// special-state checks.
    Launch,
}

/// ftCo_8008EC90's linked branch for `fighter`, before its own
/// Fighter_ProcessHit: `fighter` has knockback this frame and no order yet,
/// and `partner` is the other member of its grab pair. Returns without
/// acting when the pair branch is not the one retail takes.
pub fn resolve_linked_hit(
    fighter: &mut Fighter,
    partner: &mut Fighter,
    fighter_assets: &FighterAssets,
    partner_assets: &FighterAssets,
    rng: &mut gekko_math::HsdRng,
) -> Result<()> {
    let Some(knockback) = fighter.combat.pending.as_ref().map(|hit| hit.knockback) else {
        return Ok(());
    };
    if knockback == 0.0 || fighter.combat.pair_order.is_some() || fighter.status.disabled {
        return Ok(());
    }
    // ftCo_8008E984 / inlineB0: armour (x221A_b3 with x18A8) is unported.
    assert!(
        fighter.combat.armor == 0.0,
        "ftCo_8008EC90: an armoured member of a grab pair"
    );
    // ftCo_800C3538 (cape) and ftCo_800C44CC / ftCo_800D2FA4 are not
    // reachable for the supported kinds and elements.
    let partner_hit = partner.combat.pending.as_ref().map(|hit| hit.knockback);
    let partner_launched = partner_hit.is_some_and(|knockback| knockback != 0.0);
    match fighter.combat.grab {
        // x221B_b5: this fighter holds the other.
        Some(GrabLink::Holding { .. }) => {
            if !partner_launched {
                // ftCommon_8007DB58, then ftCo_800DCFD4: the captor's second
                // throw record launches the unhit victim.
                partner.interrupt_actions();
                launch_by_captor(partner, fighter, partner_assets, fighter_assets, rng)?;
                release_pair(fighter, partner);
                fighter.combat.pair_order = Some(PairHitOrder::Launch);
                return Ok(());
            }
            // inlineB1: the captured fighter's hit came from its captor
            // (x221C_b0) or dealt under PlCo +3C0 this frame.
            if partner.combat.pending_from_captor || partner.combat.frame_damage < LIGHT_HIT_DAMAGE
            {
                unimplemented!("ftCo_800DE854: a light hit on the captured member");
            }
            release_pair(fighter, partner);
            fighter.combat.pair_order = Some(PairHitOrder::Launch);
            partner.combat.pair_order = Some(PairHitOrder::Launch);
        }
        // !x221B_b5: this fighter is held. The captor's own light hit keeps
        // the grab (grab_escape::capture_damage).
        Some(GrabLink::Captured { .. }) => {
            // inlineB1: from the captor (x221C_b0) or dealt under PlCo +3C0.
            if fighter.combat.pending_from_captor || fighter.combat.frame_damage < LIGHT_HIT_DAMAGE
            {
                if !fighter.combat.pending_from_captor {
                    unimplemented!("ftCo_8008EC90: a light third-party hit on the captured member");
                }
                return Ok(());
            }
            if !partner_launched {
                unimplemented!("ftCo_800DE2F0: the captured member launched, its captor not hit");
            }
            // ftCo_800DCE34(captor, gobj), then this fighter's launch; the
            // captor follows x1828 = 1.
            release_pair(partner, fighter);
            fighter.combat.pair_order = Some(PairHitOrder::Launch);
            partner.combat.pair_order = Some(PairHitOrder::Launch);
        }
        None => {}
    }
    Ok(())
}

/// ftCo_800DCFD4 (800DCFD4): the captured fighter leaves the ground and is
/// launched by its captor's throw record 1 (xDF4[1]), whose knockback is
/// ftColl_80079C70's (the victim's own weight, this frame's damage and the
/// record's damage count).
fn launch_by_captor(
    captured: &mut Fighter,
    captor: &mut Fighter,
    captured_assets: &FighterAssets,
    captor_assets: &FighterAssets,
    rng: &mut gekko_math::HsdRng,
) -> Result<()> {
    // x1988 (ftColl_8007B62C): a subaction hurtbox override is unported.
    assert!(
        captured.commands.hurt_status == melee_types::combat::HurtStatus::Normal,
        "ftCo_800DCFD4: a captured fighter with a hurtbox override"
    );
    captured.leave_ground();
    // ftColl_8007891C: the captor's move is credited and staled.
    captor.credit_hit(captured.spawn_number, captor_assets);
    let record = captor.commands.throw_hitboxes[1]
        .as_ref()
        .expect("ftCo_800DCFD4: throw record 1");
    let descriptor = super::grab_throw::throw_descriptor(record);
    let knockback = captured_assets.damage.knockback_for_frame(
        &descriptor,
        captured.physics.percent,
        captured.combat.frame_damage,
        captured.attributes.size.weight,
        captor.commands.throw_damage_counts[1],
    );
    // ftColl_80078710 only records the source for stats (x18C0/x18C4).
    let hit = melee_coll::damage::ReceivedHit {
        facing: -captor.physics.facing,
        facing_override: None,
        percent_damage: descriptor.damage,
        descriptor,
        height: melee_coll::hurtbox::HurtHeight::Middle,
        knockback,
    };
    // Fighter_UnkTakeDamage_8006CC30, ftCo_Damage_CalcKnockback and
    // ftCo_8008E908(gobj, 0.0). The damage motion change zeroes
    // dmg.kb_applied (fighter.c:1043), so the victim's own ProcessHit then
    // takes the no-knockback branch.
    captured.begin_damage_reaction(hit, None, None, None, captured_assets, rng)?;
    Ok(())
}

/// PlCo +3C0 (an int, 6): below this frame's damage a captured fighter's
/// hit counts as light (inlineB1).
const LIGHT_HIT_DAMAGE: f32 = 6.0;

/// ftCo_800DCE34 (800DCE34) -> ftCo_800DC920's unconstrained path: both
/// links go, then the captured fighter's root takes its position.
/// Fighter_UnkSetFlag_8006CFBC only acts on x2219_b7, whose source
/// (x221A_b0) is unported.
fn release_pair(captor: &mut Fighter, captured: &mut Fighter) {
    assert!(
        captured.combat.thrown_pose.is_none(),
        "ftCo_800DC920: releasing a constrained (thrown) fighter"
    );
    captor.combat.grab = None;
    captured.combat.grab = None;
    let root = captured.animation.root;
    let position = captured.physics.position;
    captured.skeleton.set_translate(root, &position);
}

impl Fighter {
    /// Fighter_ProcessHit's x1828 == 1: ftCommon_8007DB58 and
    /// ftCo_8008E908(gobj, 0.0).
    pub(super) fn launch_by_pair_order(
        &mut self,
        hit: melee_coll::damage::ReceivedHit,
        assets: &FighterAssets,
        rng: &mut gekko_math::HsdRng,
    ) -> Result<i32> {
        self.interrupt_actions();
        self.begin_damage_reaction(hit, None, None, None, assets, rng)
    }
}
