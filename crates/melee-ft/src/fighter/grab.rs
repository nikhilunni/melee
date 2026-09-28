//! Body-grab startup, ftCo_Catch.c. Linked capture is an explicit proc boundary.
use super::FighterCore;
use super::{
    assets::{FighterAssets, Result},
    Fighter, MotionData,
};
use hsd_types::Vec3;
use melee_gr::wind::Wind;
use melee_types::CommonMotionState as S;

impl Fighter {
    /// ftCo_800D8C54 (800D8C54): Catch begins at frame zero without an immediate step.
    pub fn enter_catch(&mut self, assets: &FighterAssets) -> Result<()> {
        self.enter_catch_motion(S::Catch, assets)
    }

    /// ftCo_800D8A38: the running predicate selects the CatchDash row.
    pub(super) fn try_dash_catch(
        &mut self,
        assets: &FighterAssets,
        context: &crate::input::WaitContext,
    ) -> Result<bool> {
        if self.try_dash_item_throw(assets)? {
            return Ok(true);
        }
        if self.first_ground_transition(assets, context, &[crate::input::WaitPredicate::Grab])
            != crate::input::WaitTransition::Grab
        {
            return Ok(false);
        }
        self.enter_catch_motion(S::CatchDash, assets)?;
        Ok(true)
    }

    pub(super) fn enter_catch_motion(&mut self, state: S, assets: &FighterAssets) -> Result<()> {
        self.character.catch_variant();
        // ftCo_800D8C54: retail 800D8C84 stores +2340 only; +2344 carries over.
        let retained_word = self.inherited_scratch_word();
        self.core.physics.animation_velocity = Vec3::ZERO;
        self.change_motion_state(state.into(), assets)?;
        self.core.state_data = MotionData::Catch { retained_word };
        if let Some(entered) = self.character.table().catch_entered {
            entered(self);
        }
        Ok(())
    }

    /// Fighter_CallAcessoryCallbacks_8006C624's character callbacks: in
    /// hitlag accessory3, otherwise accessory2 (accessory1 follows, in the
    /// scene's grab pairs).
    pub fn character_proc_accessories(
        &mut self,
        map: &mut melee_mp::CollMap,
        rng: &mut gekko_math::HsdRng,
    ) {
        if self.status.disabled {
            return;
        }
        let table = self.character.table();
        if self.in_hitlag() {
            if let Some(accessory) = table.hitlag_accessory {
                accessory(self, rng);
            }
        } else if let Some(accessory) = table.accessory2 {
            accessory(self, map, rng);
        }
    }

    /// ftCo_Catch_Anim (800D8CC8) / ftCo_CatchDash_Anim: the tether's frame
    /// (fn_800D8EC8 / fn_800D9228), then Wait at the animation's end.
    pub(super) fn catch_animation(
        &mut self,
        assets: &FighterAssets,
        map: &mut melee_mp::CollMap,
    ) -> Result<()> {
        if let Some(tether) = self.character.table().tether {
            if (tether.animate)(self, assets, map)? {
                return Ok(());
            }
        }
        if !self.core.animation.frames_remaining(&self.core.skeleton) {
            self.change_motion_state(S::Wait.into(), assets)?;
        }
        Ok(())
    }

    /// ftCo_Catch_Coll (800D8E08) -> ft_800841B8: departure during startup.
    pub(super) fn catch_collision(
        &mut self,
        assets: &FighterAssets,
        map: &mut melee_mp::CollMap,
    ) -> Result<()> {
        use crate::collision::ground::{map_ground_action, WaitGroundResult};
        if map_ground_action(
            &mut self.core.physics,
            &mut self.core.collision,
            map,
            &mut self.core.skeleton,
            self.core.animation.root,
            self.core.input.current.stick.x,
        ) == WaitGroundResult::EnterFall
        {
            // ftCo_Fall_Enter converts only a grounded source; DownBound may
            // already be airborne, with an expired ECB lock.
            self.change_motion_state(S::Fall.into(), assets)?;
        }
        Ok(())
    }
}
impl FighterCore {
    /// ftCo_CatchDash_Phys (800D8DD0) -> ft_80085030: TransN motion or friction.
    pub(super) fn dash_catch_physics(
        &mut self,
        assets: &FighterAssets,
        map: &melee_mp::CollMap,
        wind: Wind,
    ) {
        // ftCo_Catch_Phys's friction: a separate multiplier product.
        let friction = assets.grab_friction_multiplier * self.attributes.ground.ground_friction;
        self.root_motion_or_friction(friction, assets, map, wind);
    }

    /// ft_80085030 (80085030): the animation's TransN drives the ground
    /// speed when it has root motion, otherwise `friction` slows it.
    pub(super) fn root_motion_or_friction(
        &mut self,
        friction: f32,
        assets: &FighterAssets,
        map: &melee_mp::CollMap,
        wind: Wind,
    ) {
        if self
            .animation
            .flags
            .contains(crate::anim::MotionFlags::ROOT_MOTION)
        {
            let offset = self
                .animation
                .root_motion
                .as_ref()
                .expect("root motion TransN")
                .primary_history
                .offset
                .z;
            // retail ft_80085030, 8008505C: fmsubs.
            self.physics.ground_acceleration =
                gekko_math::fma::fmsubs(offset, self.physics.facing, self.physics.ground_velocity);
        } else {
            self.physics.ground_acceleration = crate::physics::friction::friction_acceleration(
                self.physics.ground_velocity,
                friction,
            );
        }
        use crate::physics::grounded::{self, GroundedParameters};
        grounded::apply_ground_movement(
            &mut self.physics,
            self.collision.data.floor.normal,
            map.floor_speed_scale(&self.collision.data),
        );
        grounded::finish_ground_update(
            &mut self.physics,
            &self.collision.data,
            &GroundedParameters::from_attributes(&self.attributes, &assets.common),
            map,
            wind,
        );
    }
    /// ftCo_Catch_Phys (800D8D88): separate multiplier product; no fused sites.
    pub(super) fn catch_physics(
        &mut self,
        assets: &FighterAssets,
        map: &melee_mp::CollMap,
        wind: Wind,
    ) {
        use crate::physics::{
            friction::friction_acceleration,
            grounded::{self, GroundedParameters},
        };
        self.physics.ground_acceleration = friction_acceleration(
            self.physics.ground_velocity,
            assets.grab_friction_multiplier * self.attributes.ground.ground_friction,
        );
        grounded::apply_ground_movement(
            &mut self.physics,
            self.collision.data.floor.normal,
            map.floor_speed_scale(&self.collision.data),
        );
        grounded::finish_ground_update(
            &mut self.physics,
            &self.collision.data,
            &GroundedParameters::from_attributes(&self.attributes, &assets.common),
            map,
            wind,
        );
    }
}

/// Reciprocal fighter identity; unlike a slot index, it distinguishes respawns.
#[derive(Clone, Copy, Debug)]
pub enum GrabLink {
    Holding { victim: u32, vertical_offset: f32 },
    Captured { captor: u32 },
}

/// ftColl_80078A2C (80078A2C): return distance only for a legal capsule contact.
/// The scene chooses the strictly nearest candidate in fighter-list order.
pub fn candidate(victim: &mut FighterCore, attacker: &FighterCore) -> Option<f32> {
    use melee_types::{GroundOrAir, HitElement};
    // x221E_b6 / x1A68: Catch arms category 1 (ftCo_Catch.c:115); a
    // character special arms its own.
    let category = if matches!(attacker.motion_state.id, S::Catch | S::CatchDash) {
        1
    } else if let Some(category) = attacker.status.special_grab {
        category.0
    } else {
        return None;
    };
    if attacker.status.disabled
        || victim.status.disabled
        || victim.combat.grab.is_some()
        || victim.status.grab_exclusions.0 & category != 0
        || victim.status.ledge_intangibility != 0
        || victim.status.revival_invincibility != 0
        || victim.commands.hurt_status != melee_types::combat::HurtStatus::Normal
    {
        return None;
    }
    for hit in attacker.commands.hitboxes.iter().flatten() {
        let desc = &hit.descriptor;
        if desc.element != HitElement::Catch
            || (victim.physics.ground_or_air == GroundOrAir::Ground && !desc.hit_ground)
            || (victim.physics.ground_or_air == GroundOrAir::Air && !desc.hit_air)
        {
            continue;
        }
        if victim
            .contact_with_hurtboxes(hit, attacker.player.scale)
            .is_some()
        {
            // ftGrabDist, inlined in 80078A2C: separate subtract and sign test.
            return Some(gekko_math::msl::fabsf(
                victim.physics.position.x - attacker.physics.position.x,
            ));
        }
    }
    None
}

/// Fighter_UnkProcessGrab (8006CA5C): grab_cb runs before grabbed_cb.
pub fn capture_pair(
    victim: &mut Fighter,
    attacker: &mut Fighter,
    victim_assets: &FighterAssets,
    attacker_assets: &FighterAssets,
    map: &mut melee_mp::CollMap,
    victim_rank: u8,
) -> Result<()> {
    if !matches!(attacker.motion_state.id, S::Catch | S::CatchDash) {
        let special_grab = attacker.character.table().special_grab;
        return special_grab(attacker, victim, attacker_assets, victim_assets);
    }
    attacker.character.catch_variant();
    attacker.core.physics.ground_velocity = 0.0;
    // fn_800D9CE8's kind arm, before the pull's motion change.
    if let Some(tether) = attacker.character.table().tether {
        (tether.caught)(attacker);
    }
    let frame = attacker.core.animation.frame;
    let (pull, frame) = if attacker.motion_state.id == S::Catch {
        let start = attacker.character.table().catch_pull_start;
        (S::CatchPull, start(attacker, attacker_assets, frame))
    } else {
        (S::CatchDashPull, frame)
    };
    attacker.core.commands.grab_release = false;
    attacker.core.commands.throw_reverse = false;
    attacker.change_motion_state_at(pull.into(), attacker_assets, frame)?;
    if let Some(pulled) = attacker.character.table().catch_pulled {
        pulled(attacker);
    }
    attacker.core.combat.grab = Some(GrabLink::Holding {
        victim: victim.core.spawn_number,
        vertical_offset: 0.0,
    });
    // fn_800DA8E4 (800DA8E4): ftCommon_8007DB58 before the capture entry.
    victim.interrupt_actions();
    victim.core.physics.facing = -attacker.core.physics.facing;
    let pulled = if victim.physics.ground_or_air == melee_types::GroundOrAir::Air {
        S::CapturePulledHi
    } else {
        S::CapturePulledLw
    };
    victim.change_motion_state(pulled.into(), victim_assets)?;
    let mut capture = super::grab_escape::CaptureState::new(
        victim.physics.percent,
        victim.grab_handicap,
        victim_rank,
        &victim_assets.grab_escape,
    );
    capture.mouth_scale = attacker.character.mouth_capture_scale();
    victim.core.state_data = MotionData::Capture(capture);
    finish_capture(&mut victim.core, &mut attacker.core, victim_assets);
    map_capture(victim, &mut attacker.core, victim_assets, map, false)?;
    Ok(())
}

/// fn_800DAC78 (800DAC78), no fused sites: hold bone minus captured XRotN.
/// The hold bone is mv.co.capturedamage.x18: a tether's claw while it
/// reels the catch in.
fn capture_positions(
    victim: &mut FighterCore,
    attacker: &mut FighterCore,
    assets: &FighterAssets,
) -> (Vec3, Vec3) {
    let target = match attacker.grafted_part {
        // lb_8000B1CC on the tether's tip model (x18 = its jobj).
        Some(grafted) if attacker.holds_by_graft => grafted.position,
        _ => super::caches::bone_position(
            &mut attacker.skeleton,
            attacker.animation.root,
            usize::from(attacker.bones.model.shield),
            Vec3::ZERO,
        ),
    };
    let origin = super::caches::bone_position(
        &mut victim.skeleton,
        victim.animation.root,
        usize::from(
            assets
                .parts
                .joint(melee_types::FtPart::XRotN)
                .expect("capture XRotN"),
        ),
        Vec3::ZERO,
    );
    (target, origin)
}

fn capture_delta(
    victim: &mut FighterCore,
    attacker: &mut FighterCore,
    assets: &FighterAssets,
) -> Vec3 {
    let (target, origin) = capture_positions(victim, attacker, assets);
    Vec3::new(
        target.x - origin.x,
        target.y - origin.y,
        target.z - origin.z,
    )
}

/// fn_800DA054 (800DA054): accessory alignment after both fighters' Map.
/// Return true when the linked pair must cut; otherwise retain the capped recoil.
pub fn capture_accessory(
    victim: &mut FighterCore,
    attacker: &mut FighterCore,
    assets: &FighterAssets,
) -> bool {
    if victim.combat.thrown_pose.is_some() {
        return false;
    }
    let Some(GrabLink::Holding {
        vertical_offset, ..
    }) = attacker.combat.grab
    else {
        unreachable!()
    };
    let (target, origin) = capture_positions(victim, attacker, assets);
    // 800DA0C8/D8/E0/EC: separate subtraction, multiplication and addition.
    let dx = origin.x - target.x;
    let dy = (origin.y - target.y) + vertical_offset;
    if dx * attacker.physics.facing > assets.grab_escape.horizontal_release_distance
        || dy.abs() > assets.grab_escape.vertical_release_distance
    {
        return true;
    }
    if dx * attacker.physics.facing < 0.0 {
        let distance = dx.abs();
        let speed = attacker.attributes.walking.walk_max_vel;
        let velocity = if distance > speed { speed } else { distance };
        attacker.physics.ground_velocity = if dx > 0.0 { velocity } else { -velocity };
    }
    false
}

/// ftCo_CapturePulledLw_Phys (800DB00C): scene calls at this fighter's Update proc.
pub fn align_capture(
    victim: &mut Fighter,
    attacker: &mut FighterCore,
    assets: &FighterAssets,
    map: &mut melee_mp::CollMap,
) -> Result<()> {
    let delta = capture_delta(&mut victim.core, attacker, assets);
    let lifted = matches!(
        victim.motion_state.id,
        S::CapturePulledLw | S::CaptureWaitLw | S::CaptureDamageLw
    ) && delta.y > assets.grab_escape.lift_threshold * victim.player.scale;
    victim.physics.position.x += delta.x;
    victim.physics.position.y += delta.y;
    victim.physics.position.z += delta.z;
    if lifted {
        capture_departure(victim, attacker, assets, map)?;
    }
    Ok(())
}

/// ftCo_CapturePulled/Wait/Damage counterparts share UpdateCmd-only flags.
fn counterpart(state: S, air: bool) -> S {
    match (state, air) {
        (S::CapturePulledLw | S::CapturePulledHi, true) => S::CapturePulledHi,
        (S::CapturePulledLw | S::CapturePulledHi, false) => S::CapturePulledLw,
        (S::CaptureWaitLw | S::CaptureWaitHi, true) => S::CaptureWaitHi,
        (S::CaptureWaitLw | S::CaptureWaitHi, false) => S::CaptureWaitLw,
        (S::CaptureDamageLw | S::CaptureDamageHi, true) => S::CaptureDamageHi,
        (S::CaptureDamageLw | S::CaptureDamageHi, false) => S::CaptureDamageLw,
        _ => unreachable!("capture counterpart outside the capture family"),
    }
}

/// fn_800DAA40: ground owns the captor's Y offset; air aligns immediately.
fn align_link(victim: &mut FighterCore, attacker: &mut FighterCore, assets: &FighterAssets) {
    let delta = capture_delta(victim, attacker, assets);
    let vertical_offset = if victim.physics.ground_or_air == melee_types::GroundOrAir::Ground {
        delta.y + victim.physics.position.y - attacker.physics.position.y
    } else {
        victim.physics.position.x += delta.x;
        victim.physics.position.y += delta.y;
        victim.physics.position.z += delta.z;
        0.0
    };
    attacker.combat.grab = Some(GrabLink::Holding {
        victim: victim.spawn_number,
        vertical_offset,
    });
}

fn capture_departure(
    victim: &mut Fighter,
    attacker: &mut FighterCore,
    assets: &FighterAssets,
    map: &mut melee_mp::CollMap,
) -> Result<()> {
    if victim.combat.thrown_pose.is_some() {
        unimplemented!(
            "ftCo_CaptureWait.c:256-270: lifted out of a mouth hold (fn_800DB5D8 again)"
        );
    }
    if victim.motion_state.id != S::CaptureDamageLw {
        victim.leave_ground(); // ftCommon_8007D5D4, not the spent-jumps variant.
        victim.collision.lock_frames = 0; // ftCommon_UnlockECB.
        victim.collision.data.x130_flags &= !melee_types::mp::coll_data_x130::LOCKED;
    }
    let state = counterpart(victim.motion_state.id, true);
    let frame = victim.animation.frame;
    victim.change_motion_with_updated_commands(
        state.into(),
        assets,
        frame,
        super::MotionColorPolicy::ResetSecondary,
    )?;
    if !matches!(state, S::CapturePulledLw | S::CapturePulledHi) {
        victim.status.grab_exclusions = super::ledge::GrabExclusions::ALL;
    }
    align_link(&mut victim.core, attacker, assets);
    // Retail invokes the new airborne collision callback immediately.
    map_capture(victim, attacker, assets, map, false)
}

/// Paired collision preserves source order: map, counterpart motion, link alignment.
/// Direct entry/physics calls do not run Fighter_procMap's ECB countdown again.
pub fn map_capture(
    victim: &mut Fighter,
    attacker: &mut FighterCore,
    assets: &FighterAssets,
    map: &mut melee_mp::CollMap,
    normal_phase: bool,
) -> Result<()> {
    let c = &mut victim.core;
    if normal_phase {
        crate::collision::air::begin_map(
            &c.physics,
            &mut c.collision,
            &mut c.skeleton,
            c.animation.root,
        );
    }
    let air = matches!(
        c.motion_state.id,
        S::CapturePulledHi | S::CaptureWaitHi | S::CaptureDamageHi
    );
    let cd = &mut c.collision.data;
    let pose = crate::collision::ecb::EcbPose::read(&mut c.skeleton, c.animation.root, cd);
    cd.last_pos = cd.cur_pos;
    cd.cur_pos = c.physics.position;
    let supported = if air {
        {
            map.air_collide_stay(cd, Some(&|i| pose.position(i)));
            cd.env_flags as u32 & melee_types::mp::collide::FLOOR_MASK != 0
        }
    } else {
        map.ground_collide_pass(cd, Some(&|i| pose.position(i)))
    };
    c.physics.position = cd.cur_pos;
    if air && supported {
        if victim.motion_state.id != S::CaptureDamageHi {
            victim.land();
        }
        let state = counterpart(victim.motion_state.id, false);
        let frame = victim.animation.frame;
        victim.change_motion_with_updated_commands(
            state.into(),
            assets,
            frame,
            super::MotionColorPolicy::ResetSecondary,
        )?;
        if state != S::CapturePulledLw {
            victim.status.grab_exclusions = super::ledge::GrabExclusions::ALL;
        }
        align_link(&mut victim.core, attacker, assets);
    } else if !air && !supported {
        capture_departure(victim, attacker, assets, map)?;
    }
    let c = &mut victim.core;
    c.skeleton
        .set_translate(c.animation.root, &c.physics.position);
    // Only the Capture rows' own collision callback consumes this; a
    // CaptureDamage row (possibly just re-entered by a departure) has none.
    if normal_phase {
        if let MotionData::Capture(capture) = &mut victim.state_data {
            capture.map_prepared = true;
        }
    }
    Ok(())
}

impl Fighter {
    /// fn_800DA1D8 (800DA1D8): switch captor first, then linked victim in the scene.
    pub fn enter_catch_wait(&mut self, assets: &FighterAssets) -> Result<()> {
        self.core.physics.ground_velocity = 0.0;
        self.core.commands.grab_release = false;
        self.change_motion_state(S::CatchWait.into(), assets)?;
        // fn_800DA1D8: x18 is the hold bone again.
        self.core.holds_by_graft = false;
        self.core.status.grab_exclusions = super::ledge::GrabExclusions::ALL;
        // The flash is queued after graphics the motion change issued (a color
        // program step in Fighter_ChangeMotionState: corpus_v2_s1_effffffff_p1,
        // tick 3832) and before those of the proc's own color step
        // (corpus_v3_fd_fox_falco4_eb8786a38_p1, tick 5334).
        let bone = usize::from(self.core.bones.model.shield);
        self.core
            .push_effect_after_issued_graphics(melee_ef::request::EffectRequest::CaptureFlash {
                bone,
            });
        Ok(())
    }
}

/// fn_800DB6C8 -> fn_800DBAE4 (800DBAE4), before victim's own Anim proc.
pub fn capture_wait(victim: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    if victim.core.tether_article {
        unimplemented!("fn_800DB6C8: a captured Link's hookshot reels in (it_802A7840)");
    }
    let state = if matches!(
        victim.motion_state.id,
        S::CapturePulledHi | S::CaptureDamageHi
    ) {
        S::CaptureWaitHi
    } else {
        S::CaptureWaitLw
    };
    victim.change_motion_state(state.into(), assets)?;
    hold_in_mouth(&mut victim.core, assets);
    victim.core.status.grab_exclusions = super::ledge::GrabExclusions::ALL;
    Ok(())
}

/// fn_800DB5D8 (800DB5D8), and the FTKIND_YOSHI arms of fn_800DB790 and
/// fn_800DBAE4, after the capture motion entry: a victim in its captor's
/// mouth is hidden and intangible except for one capsule on XRotN. Its
/// accessory (ftCo_800DB464) is the thrown one, run while constrained.
pub(super) fn hold_in_mouth(victim: &mut FighterCore, assets: &FighterAssets) {
    let MotionData::Capture(capture) = &victim.state_data else {
        unreachable!("capture scratch missing")
    };
    let Some(scale) = capture.mouth_scale else {
        return;
    };
    victim.effect_state.invisible = true;
    victim.set_hurt_capsules(melee_types::combat::HurtStatus::Intangible);
    // ftCommon_GetModelScale: fmuls, then the capsule scale's fdivs.
    let model_scale = victim.player.scale * victim.attributes.size.model_scaling;
    victim.replace_hurt_capsule(
        0,
        melee_coll::hurtbox::HurtCapsule {
            height: melee_coll::hurtbox::HurtHeight::Middle,
            grabbable: false,
            bone: usize::from(
                assets
                    .parts
                    .joint(melee_types::FtPart::XRotN)
                    .expect("capture XRotN"),
            ),
            offsets: [Vec3::ZERO; 2],
            radius: scale / model_scale,
            positions: [Vec3::ZERO; 2],
            cached: false,
        },
    );
}

/// fn_800DB6C8's FTKIND_YOSHI arm, ftCo_800DB368 (800DB368): on CatchWait
/// entry a victim held in the mouth is pinned to the captor's TransN2.
pub fn constrain_in_mouth(
    victim: &mut FighterCore,
    attacker: &mut FighterCore,
    va: &FighterAssets,
    aa: &FighterAssets,
) {
    let MotionData::Capture(capture) = &victim.state_data else {
        unreachable!("capture scratch missing")
    };
    if capture.mouth_scale.is_none() || victim.combat.thrown_pose.is_some() {
        return;
    }
    super::grab_throw::constrain_to_captor(victim, attacker, va, aa);
}

/// fn_800DAADC / fn_800DAA40: common linkage and initial pose alignment,
/// after both motion entries and before the victim's collision callback.
fn finish_capture(
    victim: &mut FighterCore,
    attacker: &mut FighterCore,
    victim_assets: &FighterAssets,
) {
    victim.step_animation(victim_assets);
    victim.combat.grab = Some(GrabLink::Captured {
        captor: attacker.spawn_number,
    });
    victim.physics.self_velocity = Vec3::ZERO;
    victim.physics.knockback_velocity = Vec3::ZERO;
    victim.physics.shield_knockback_velocity = Vec3::ZERO;
    victim.physics.ground_velocity = 0.0;
    victim.physics.ground_knockback_velocity = 0.0;
    victim.physics.ground_shield_knockback_velocity = 0.0;
    victim.status.grab_exclusions = super::ledge::GrabExclusions::ALL;
    align_link(victim, attacker, victim_assets);
}
