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
    map: &mut melee_mp::CollMap,
    rng: &mut gekko_math::HsdRng,
) -> Result<()> {
    let Some(hit) = &fighter.combat.pending else {
        return Ok(());
    };
    if hit.knockback == 0.0 || fighter.combat.pair_order.is_some() || fighter.status.disabled {
        return Ok(());
    }
    // ftCo_8008E984 / inlineB0: armour (x221A_b3 with x18A8) is unported.
    assert!(
        fighter.combat.armor == 0.0,
        "ftCo_8008EC90: an armoured member of a grab pair"
    );
    // ftCo_800C3538 (cape) and ftCo_800C44CC / ftCo_800D2FA4 are not
    // reachable for the supported kinds and elements.
    let partner_launched = matches!(&partner.combat.pending, Some(hit) if hit.knockback != 0.0);
    match fighter.combat.grab {
        // x221B_b5: this fighter holds the other.
        Some(GrabLink::Holding { .. }) => {
            if !partner_launched {
                // ftCommon_8007DB58, then ftCo_800DCFD4: the captor's second
                // throw record launches the unhit victim.
                partner.interrupt_actions();
                launch_by_captor(partner, fighter, partner_assets, fighter_assets, rng)?;
                release_pair(fighter, partner, partner_assets, map);
                fighter.combat.pair_order = Some(PairHitOrder::Launch);
                return Ok(());
            }
            // inlineB1: the captured fighter's hit came from its captor
            // (x221C_b0) or dealt under PlCo +3C0 this frame.
            if partner.combat.pending_from_captor || partner.combat.frame_damage < LIGHT_HIT_DAMAGE
            {
                take_captor_throw_hit(partner, fighter, partner_assets, fighter_assets);
            }
            release_pair(fighter, partner, partner_assets, map);
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
                // ftCo_800DCE34(captor, gobj) and this fighter's launch; then
                // (after it, `release_captor`) ftCo_800DE2F0 on the captor.
                release_pair(partner, fighter, fighter_assets, map);
                fighter.combat.pair_order = Some(PairHitOrder::Launch);
                fighter.combat.release_captor = Some(partner.spawn_number);
                return Ok(());
            }
            // ftCo_800DCE34(captor, gobj), then this fighter's launch; the
            // captor follows x1828 = 1.
            release_pair(partner, fighter, fighter_assets, map);
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

/// ftCo_800DE854 (800DE854): a captured fighter's light or captor-dealt hit
/// is replaced by its captor's throw record 1 (xDF4[1]): ftColl_80079C70's
/// knockback, the record's angle and element, facing against the captor on
/// the middle hurtbox, and the record's damage taken at once. Its captor is
/// credited (ftColl_8007891C). The launch itself follows x1828.
fn take_captor_throw_hit(
    captured: &mut Fighter,
    captor: &mut Fighter,
    captured_assets: &FighterAssets,
    captor_assets: &FighterAssets,
) {
    let record = captor.commands.throw_hitboxes[1]
        .as_ref()
        .expect("ftCo_800DE854: throw record 1");
    let descriptor = super::grab_throw::throw_descriptor(record);
    let knockback = captured_assets.damage.knockback_for_frame(
        &descriptor,
        captured.physics.percent,
        captured.combat.frame_damage,
        captured.attributes.size.weight,
        captor.commands.throw_damage_counts[1],
    );
    let damage = descriptor.damage;
    let hit = captured
        .combat
        .pending
        .as_mut()
        .expect("ftCo_800DE854: the captured fighter's hit");
    hit.descriptor = descriptor;
    hit.knockback = knockback;
    hit.facing = -captor.physics.facing;
    hit.facing_override = None;
    hit.height = melee_coll::hurtbox::HurtHeight::Middle;
    captured.core.take_percent_damage(damage);
    captor.credit_hit(captured.spawn_number, captor_assets);
}

/// ftCommon_8007DB58, then ftCo_800DE2F0 (800DE2F0): a captor whose victim
/// was launched out of its grab takes PlCo +380's hit (lbColl_80008D30), with
/// knockback from ftColl_80079AB0 on its own percent and this frame's damage,
/// facing its own way, at TransN2 on the middle hurtbox.
pub fn launch_released_captor(
    captor: &mut Fighter,
    assets: &FighterAssets,
    rng: &mut gekko_math::HsdRng,
) -> Result<()> {
    captor.interrupt_actions();
    // ftColl_800788D4 records a sourceless hit for stats only; TransN2's
    // position (x1854_collpos) has no reader in scope.
    let descriptor = super::grab_throw::throw_descriptor(&assets.grab_escape.captor_release_hit);
    let knockback = assets.damage.knockback_for_frame(
        &descriptor,
        captor.physics.percent,
        captor.combat.frame_damage,
        captor.attributes.size.weight,
        fctiwz_damage(descriptor.damage),
    );
    let hit = melee_coll::damage::ReceivedHit {
        facing: captor.physics.facing,
        facing_override: None,
        percent_damage: descriptor.damage,
        descriptor,
        height: melee_coll::hurtbox::HurtHeight::Middle,
        knockback,
    };
    // Fighter_UnkTakeDamage_8006CC30, ftCo_Damage_CalcKnockback and
    // ftCo_8008E908(gobj, 0.0).
    captor.begin_damage_reaction(hit, None, None, None, assets, rng)?;
    Ok(())
}

/// lbColl_80008D30 copies the record's integer damage to unk_count.
fn fctiwz_damage(damage: f32) -> u32 {
    gekko_math::msl::fctiwz(damage) as u32
}

/// PlCo +3C0 (an int, 6): below this frame's damage a captured fighter's
/// hit counts as light (inlineB1).
const LIGHT_HIT_DAMAGE: f32 = 6.0;

/// ftCo_800DCE34 (800DCE34) -> ftCo_800DC920 (800DC920): both links go;
/// a thrown (constrained, x2226_b2) fighter is first set down where its
/// XRotN points (release_thrown). Then the captured fighter's root takes its
/// position. Fighter_UnkSetFlag_8006CFBC only acts on x2219_b7, whose
/// source (x221A_b0) is unported.
fn release_pair(
    captor: &mut Fighter,
    captured: &mut Fighter,
    captured_assets: &FighterAssets,
    map: &mut melee_mp::CollMap,
) {
    captor.combat.grab = None;
    if captured.combat.thrown_pose.is_some() {
        release_thrown(captor, captured, captured_assets, map);
    }
    captured.combat.grab = None;
    let root = captured.animation.root;
    let position = captured.physics.position;
    captured.skeleton.set_translate(root, &position);
}

/// ftCo_800DC920's x2226_b2 path: the release point is XRotN plus the
/// capture offset (retail 800DCA40 / 800DCA54: fmadds, as ftCo_800DE508),
/// the constraint and saved translation are restored, and the fighter lands
/// on a floor under it connected to the captor's, if one is within PlCo
/// +3BC, else sweeps there from the captor's centre.
fn release_thrown(
    captor: &Fighter,
    captured: &mut Fighter,
    assets: &FighterAssets,
    map: &mut melee_mp::CollMap,
) {
    use super::caches::bone_position;
    use hsd_types::Vec3;
    let xrot = usize::from(
        assets
            .parts
            .joint(melee_types::FtPart::XRotN)
            .expect("XRotN"),
    );
    let root = captured.animation.root;
    let mut point = bone_position(&mut captured.skeleton, root, xrot, Vec3::ZERO);
    let offset = captured.combat.capture_geometry.root_offset;
    let scale = captured.player.scale;
    point.x = gekko_math::fma::fmadds(captured.physics.facing, offset.z * scale, point.x);
    point.y = gekko_math::fma::fmadds(offset.y, scale, point.y);
    point.z = 0.0;
    let pose = captured.combat.thrown_pose.take().expect("thrown pose");
    let joint = captured.animation.parts[xrot].joint;
    captured.skeleton.set_position_constraint(joint, None);
    captured
        .skeleton
        .set_translate(joint, &pose.saved_translation);
    let captor_floor = captor.collision.data.floor.index;
    if map.line_is_active(captor_floor) {
        let floor = map.floor_below(&point, -1, -1);
        if floor != -1 && map.lines_connected(floor, captor_floor) {
            captured.collision.data.floor.index = floor;
            if let Some(probe) = map.floor_probe(floor, &point) {
                if probe.delta >= assets.grab_escape.release_floor_reach {
                    let landed = Vec3::new(point.x, point.y + probe.delta, point.z);
                    captured.physics.position = landed;
                    melee_mp::set_position(&mut captured.collision.data, &landed);
                    return;
                }
            }
        }
    }
    // No floor to land on: sweep from the captor's ECB centre (three fadds,
    // 0.5 * (top + bottom) on Y) to the release point.
    let centre = Vec3::new(
        captor.physics.position.x + 0.0,
        captor.physics.position.y
            + 0.5 * (captor.collision.data.ecb.top.y + captor.collision.data.ecb.bottom.y),
        captor.physics.position.z + 0.0,
    );
    let cd = &mut captured.collision.data;
    cd.last_pos = centre;
    melee_mp::mark_ecb_clear(cd);
    cd.cur_pos = point;
    captured.skeleton.set_translate(root, &point);
    let core = &mut captured.core;
    let ecb_pose =
        crate::collision::ecb::EcbPose::read(&mut core.skeleton, root, &core.collision.data);
    let bones = |bone| ecb_pose.position(bone);
    let grounded = captured.physics.ground_or_air == melee_types::GroundOrAir::Ground;
    let touched = if grounded {
        map.air_collide_ecb5(&mut captured.collision.data, Some(&bones))
    } else {
        // ftCommon_UnlockECB.
        captured.collision.lock_frames = 0;
        captured.collision.data.x130_flags &= !melee_types::mp::coll_data_x130::LOCKED;
        map.air_collide_stay(&mut captured.collision.data, Some(&bones))
    };
    if touched {
        if !grounded {
            captured.land();
        }
        captured.physics.position = captured.collision.data.cur_pos;
    } else {
        if captured.physics.ground_or_air != melee_types::GroundOrAir::Air {
            captured.leave_ground();
        }
        captured.physics.position = point;
    }
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
