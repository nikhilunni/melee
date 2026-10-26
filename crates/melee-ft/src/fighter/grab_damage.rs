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
) -> Result<bool> {
    let Some(hit) = &fighter.combat.pending else {
        return Ok(false);
    };
    if hit.knockback == 0.0 || fighter.combat.pair_order.is_some() || fighter.status.disabled {
        return Ok(false);
    }
    if [&fighter.motion_state, &partner.motion_state]
        .iter()
        .any(|state| super::capture_yoshi::on_tongue(state.id))
    {
        // Yoshi's tongue leaves x221B_b5 clear on both members.
        unimplemented!("ftCo_8008EC90: a launch through Egg Lay's catch");
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
                partner.interrupt_actions(partner_assets);
                launch_by_captor(partner, fighter, partner_assets, fighter_assets, rng)?;
                release_pair(fighter, partner, fighter_assets, partner_assets, map);
                fighter.combat.pair_order = Some(PairHitOrder::Launch);
                return Ok(false);
            }
            // inlineB1: the captured fighter's hit came from its captor
            // (x221C_b0) or dealt under PlCo +3C0 this frame.
            if partner.combat.pending_from_captor || partner.combat.frame_damage < LIGHT_HIT_DAMAGE
            {
                take_captor_throw_hit(partner, fighter, partner_assets, fighter_assets);
            }
            release_pair(fighter, partner, fighter_assets, partner_assets, map);
            fighter.combat.pair_order = Some(PairHitOrder::Launch);
            partner.combat.pair_order = Some(PairHitOrder::Launch);
        }
        // !x221B_b5: this fighter is held. The captor's own light hit keeps
        // the grab (grab_escape::capture_damage).
        Some(GrabLink::Captured { .. }) => {
            // inlineB1: from the captor (x221C_b0) or dealt under PlCo +3C0.
            if fighter.combat.pending_from_captor || fighter.combat.frame_damage < LIGHT_HIT_DAMAGE
            {
                if partner_launched {
                    // inlineB0(captor), ftCo_Damage.c:861-868: the captor
                    // keeps the grab and follows x1828 = 3 when its hit is
                    // one ftCo_8008E984 would not launch: it is still in an
                    // earlier knockback hit's hitlag (allow_sdi, x221A_b3)
                    // and this knockback is under that one's (x18A8) plus
                    // PlCo +140. The port does not keep x18A8, so any captor
                    // in such hitlag stops here. (A holder is in one only
                    // if its grab landed as it was struck.)
                    if partner.combat.armor != 0.0 || partner.combat.struck_hitlag {
                        unimplemented!(
                            "ftCo_8008EC90 (ftCo_Damage.c:861-868): a light hit on a captured \
                             member whose captor is struck again in a knockback hit's hitlag \
                             (ftCo_8008E984, x1828 = 3)"
                        );
                    }
                    // ftCo_Damage.c:875-880: the captor's launch throws this
                    // fighter off with its throw record 1 (ftCo_800DE854),
                    // then ftCo_800DCE34(captor, gobj) and this fighter's
                    // launch; the captor follows x1828 = 1.
                    //
                    // This is the fighter's own ProcessHit, which took the
                    // hit's damage first (Fighter_UnkTakeDamage_8006CC30,
                    // fighter.c:2856): ftColl_80079C70 reads x1830_percent
                    // with it, and the record's damage adds after it.
                    let own_damage = hit.percent_damage;
                    fighter.core.take_percent_damage(own_damage, fighter_assets);
                    fighter.combat.pending_percent_taken = true;
                    take_captor_throw_hit(fighter, partner, fighter_assets, partner_assets);
                    assert!(
                        matches!(&fighter.combat.pending, Some(h) if h.knockback != 0.0),
                        "ftCo_800DE854: throw record 1 without knockback"
                    );
                    release_pair(partner, fighter, partner_assets, fighter_assets, map);
                    fighter.combat.pair_order = Some(PairHitOrder::Launch);
                    partner.combat.pair_order = Some(PairHitOrder::Launch);
                    return Ok(false);
                }
                return Ok(keep_capture_on_light_hit(fighter, partner, partner_assets));
            }
            if !partner_launched {
                // ftCo_800DCE34(captor, gobj) and this fighter's launch; then
                // (after it, `release_captor`) ftCo_800DE2F0 on the captor.
                release_pair(partner, fighter, partner_assets, fighter_assets, map);
                fighter.combat.pair_order = Some(PairHitOrder::Launch);
                fighter.combat.release_captor = Some(partner.spawn_number);
                return Ok(false);
            }
            // ftCo_800DCE34(captor, gobj), then this fighter's launch; the
            // captor follows x1828 = 1.
            release_pair(partner, fighter, partner_assets, fighter_assets, map);
            fighter.combat.pair_order = Some(PairHitOrder::Launch);
            partner.combat.pair_order = Some(PairHitOrder::Launch);
        }
        None => {}
    }
    Ok(false)
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
        captured.knockback_percent(&captured_assets.damage),
        captured.combat.frame_damage,
        captured.attributes.size.weight,
        captor.commands.throw_damage_counts[1],
        // ftColl_80079C70(captured, captor): the record's owner attacks.
        captured.player.ratios_against(captor.player.attack_ratio),
    );
    // ftColl_80078710 (0x80078710): the captor is the KO credit.
    captured
        .combat
        .ko_source
        .record(melee_coll::damage_log::HitCredit::Player(captor.player.id));
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
    captured.core.refuse_unwitnessed_coin_spill("ftCo_CaptureCut.c:233 (ftCo_800DCFD4)");
    captured.begin_damage_reaction(hit, None, None, None, false, captured_assets, rng)?;
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
        captured.knockback_percent(&captured_assets.damage),
        captured.combat.frame_damage,
        captured.attributes.size.weight,
        captor.commands.throw_damage_counts[1],
        // ftColl_80079C70(captured, captor): the record's owner attacks.
        captured.player.ratios_against(captor.player.attack_ratio),
    );
    let damage = descriptor.damage;
    let combat = &mut captured.core.combat;
    let hit = combat
        .pending
        .as_mut()
        .expect("ftCo_800DE854: the captured fighter's hit");
    // x1960_vibrateMult is ftColl_8007A06C's, from the hit this record
    // replaces: an electric one keeps its hitlag scale.
    combat.pending_electric_hitlag =
        hit.descriptor.element == melee_types::HitElement::Electric;
    hit.descriptor = descriptor;
    hit.knockback = knockback;
    hit.facing = -captor.physics.facing;
    hit.facing_override = None;
    hit.height = melee_coll::hurtbox::HurtHeight::Middle;
    captured.core.refuse_unwitnessed_coin_spill("ftCo_Thrown.c:202 (ftCo_800DE854)");
    captured.core.take_percent_damage(damage, captured_assets);
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
    captor.interrupt_actions(assets);
    // ftColl_800788D4 (0x800788D4): a sourceless hit, so the captor's KO
    // credit goes. TransN2's position (x1854_collpos) has no reader in scope.
    captor
        .combat
        .ko_source
        .record(melee_coll::damage_log::HitCredit::Nobody);
    let descriptor = super::grab_throw::throw_descriptor(&assets.grab_escape.captor_release_hit);
    let knockback = assets.damage.knockback_for_frame(
        &descriptor,
        captor.knockback_percent(&assets.damage),
        captor.combat.frame_damage,
        captor.attributes.size.weight,
        fctiwz_damage(descriptor.damage),
        // ftCo_Throw.c:605: no attacker (1.0), the captor's own defense.
        captor.player.ratios_against(1.0),
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
    captor.core.refuse_unwitnessed_coin_spill("ftCo_Throw.c:612 (ftCo_800DE2F0, a captor launched)");
    captor.begin_damage_reaction(hit, None, None, None, false, assets, rng)?;
    Ok(())
}

/// ftCo_8008EC90's light branch for a captured fighter (inlineB1: the hit came
/// from its captor or dealt under PlCo +3C0) whose captor has no knockback of
/// its own: the grab holds and the captor freezes for the same hit. Returns
/// whether the captured fighter's effects pause now: retail runs the captor's
/// pre_hitlag_cb (efLib_PauseAll) on the captured fighter's gobj, not its own.
fn keep_capture_on_light_hit(
    captured: &mut Fighter,
    captor: &mut Fighter,
    captor_assets: &FighterAssets,
) -> bool {
    let mut pause_captured_effects = false;
    // dmg.x183C_applied: the captor's hitlag is ftCommon_CalcHitlag on the
    // captured fighter's damage, the captor's motion and its x1960, with none
    // of Fighter_ProcessHit's clamps.
    let damage = captured.combat.frame_max_damage;
    if damage != 0 {
        // x1960 is PlCo +1A4 only while an electric hit is pending on the
        // captor (ftColl_8007A06C); its own ProcessHit resets it.
        let electric = matches!(
            &captor.combat.pending,
            Some(hit) if hit.descriptor.element == melee_types::HitElement::Electric
        );
        let multiplier = if electric {
            captor_assets.damage.electric_hitlag_scale
        } else {
            1.0
        };
        let crouching = matches!(
            captor.motion_state.id,
            melee_types::CommonMotionState::Squat | melee_types::CommonMotionState::SquatWait
        );
        let was_frozen = captor.in_hitlag();
        captor.core.combat.hitlag_remaining = captor_assets
            .damage
            .frozen_frames(damage, multiplier, crouching);
        // allow_sdi (x221A_b2) follows the countdown in the port. x2219_b5
        // is set only when clear, after the captor's pre_hitlag_cb, which
        // runs on the captured fighter's gobj (8008F050..8008F074).
        if !was_frozen {
            if captor.combat.hitlag_remaining == 0.0 {
                captor.core.combat.hitlag_link.frozen = true;
            }
            captor.core.status.interaction = super::Interaction::Hitlag;
            pause_captured_effects = captor.effect_state.hitlag_callbacks;
        } else if captor.combat.hitlag_remaining > 0.0 {
            captor.core.status.interaction = super::Interaction::Hitlag;
        }
    }
    // fp->input.pressed_buttons = released_buttons = 0 and inlineB2 follow in
    // the captured fighter's own ProcessHit (grab_escape::capture_damage).
    captured.combat.light_capture_hit = !captured.combat.pending_from_captor;
    pause_captured_effects
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
/// XRotN points (release_thrown). That is the captured fighter, or the
/// captor when it hangs from its grounded victim (x221B_b7, Falcon Dive:
/// ftCo_CaptureCut.c:105-110 swaps the roles), which only a third fighter's
/// hit on the pair parts. Then the captured fighter's root takes its
/// position. Fighter_UnkSetFlag_8006CFBC only acts on x2219_b7, whose
/// source (x221A_b0) is unported.
pub(super) fn release_pair(
    captor: &mut Fighter,
    captured: &mut Fighter,
    captor_assets: &FighterAssets,
    captured_assets: &FighterAssets,
    map: &mut melee_mp::CollMap,
) {
    captor.combat.grab = None;
    super::capture_kirby::refuse_parting_in_the_pull(&captured.core);
    if captured.combat.thrown_pose.is_some() {
        release_thrown(captor, captured, captured_assets, map);
    } else if captor.combat.thrown_pose.is_some() {
        release_thrown(captured, captor, captor_assets, map);
    }
    captured.combat.grab = None;
    let root = captured.animation.root;
    let position = captured.physics.position;
    captured.skeleton.set_translate(root, &position);
}

/// Which callback found a grounded hold's floor gone (`hold_floor_lost`):
/// ft_800827A0 stops at the floor's edge, so the floor went from under the
/// captor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LostHold {
    /// fn_800DA004 (800DA004, CatchPull) / fn_800DA440 (800DA440,
    /// CatchWait): ftCo_800DC920, then ftCo_Fall_Enter on the captor, then
    /// on the victim.
    Catch,
    /// fn_800DD684 (800DD684, the grounded throws): if the throw still
    /// holds its victim, ftCo_800DC920 and ftCo_Fall_Enter on the victim;
    /// then ftCo_Fall_Enter on the thrower.
    Throw,
    /// ftKp_SpecialS_801332C4 / 80133398 / 80133484 (the Koopa Klaw's
    /// grounded hold, bite, wait and throws): ftCommon_8007D5D4 on Bowser
    /// first, then as `Throw`.
    Klaw,
}

/// fn_800DA004 / fn_800DA440 / fn_800DD684 on the pair, `captured` the
/// fighter the captor still holds. The thrown fighter's XRotN is first
/// rebuilt from the thrower's TransN2 as it stands (lb_8000B1CC in
/// ftCo_800DC920), as when it mashes out of a throw.
pub fn part_on_lost_floor(
    hold: LostHold,
    captor: &mut Fighter,
    captured: Option<(&mut Fighter, &FighterAssets)>,
    captor_assets: &FighterAssets,
    map: &mut melee_mp::CollMap,
) -> Result<()> {
    use melee_types::CommonMotionState::Fall;
    if hold == LostHold::Klaw {
        captor.leave_ground();
    }
    let Some((captured, captured_assets)) = captured else {
        assert_ne!(
            hold,
            LostHold::Catch,
            "fn_800DA004 / fn_800DA440 read victim_gobj without a NULL test"
        );
        return captor.change_motion_state(Fall.into(), captor_assets);
    };
    if captured.combat.thrown_pose.is_some() {
        super::grab_throw::update_constraint(
            &mut captured.core,
            &mut captor.core,
            captured_assets,
            captor_assets,
        );
    }
    release_pair(captor, captured, captor_assets, captured_assets, map);
    match hold {
        LostHold::Catch => {
            captor.change_motion_state(Fall.into(), captor_assets)?;
            captured.change_motion_state(Fall.into(), captured_assets)
        }
        LostHold::Throw | LostHold::Klaw => {
            captured.change_motion_state(Fall.into(), captured_assets)?;
            captor.change_motion_state(Fall.into(), captor_assets)
        }
    }
}

/// ftCo_800DC920's x2226_b2 path: the release point is XRotN plus the
/// capture offset (retail 800DCA40 / 800DCA54: fmadds, as ftCo_800DE508),
/// the constraint and saved translation are restored, and the fighter lands
/// on a floor under it connected to the captor's, if one is within PlCo
/// +3BC, else sweeps there from the captor's centre.
pub(super) fn release_thrown(
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
    captured.set_pinned(false);
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
        self.interrupt_actions(assets);
        self.begin_damage_reaction(hit, None, None, None, false, assets, rng)
    }
}
