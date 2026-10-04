//! Linked throws: ftCo_Throw.c / ftCo_Thrown.c.
use super::FighterCore;
use super::{
    assets::{FighterAssets, Result},
    caches::bone_position,
    Fighter,
};
use hsd_types::Vec3;
use melee_types::{CommonMotionState as S, FtPart};

#[derive(Clone, Copy, Debug, Default)]
pub struct CaptureGeometry {
    pub hip_scale: f32,
    pub root_offset: Vec3,
}
impl CaptureGeometry {
    /// Fighter_UnkUpdateVecFromBones_8006876C: unscaled costume geometry.
    pub(super) fn from_skeleton(
        tree: &mut hsd_anim::jobj::JObjTree,
        root: hsd_anim::jobj::JObjId,
        assets: &FighterAssets,
    ) -> Self {
        let xrot = usize::from(assets.parts.joint(FtPart::XRotN).expect("XRotN"));
        let trans = usize::from(assets.parts.joint(FtPart::TransN).expect("TransN"));
        // retail 800687DC: fdivs; 80068814/24/34: separate fsubs.
        let hip_scale = tree.translation(tree.bone(root, xrot).unwrap()).y / 8.55;
        let origin = bone_position(tree, root, xrot, Vec3::ZERO);
        let translation = bone_position(tree, root, trans, Vec3::ZERO);
        Self {
            hip_scale,
            root_offset: Vec3::new(
                translation.x - origin.x,
                translation.y - origin.y,
                translation.z - origin.z,
            ),
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct ThrownPose {
    pub saved_translation: Vec3,
    pub hip_translation: Vec3,
}

/// ftData_MotionStateList / ftCo_800DD4B0: matched captor and victim motions.
#[derive(Clone, Copy, Debug)]
pub struct Throw {
    pub state: S,
    pub victim_state: S,
    pub victim_motion: i32,
    pub weight_mask: u8,
}
pub static FORWARD: Throw = Throw {
    state: S::ThrowF,
    victim_state: S::ThrownF,
    victim_motion: 262,
    weight_mask: 1,
};
pub static BACK: Throw = Throw {
    state: S::ThrowB,
    victim_state: S::ThrownB,
    victim_motion: 263,
    weight_mask: 2,
};
pub static UP: Throw = Throw {
    state: S::ThrowHi,
    victim_state: S::ThrownHi,
    victim_motion: 264,
    weight_mask: 4,
};
pub static DOWN: Throw = Throw {
    state: S::ThrowLw,
    victim_state: S::ThrownLw,
    victim_motion: 265,
    weight_mask: 8,
};

pub static THROWS: [&Throw; 4] = [&FORWARD, &BACK, &UP, &DOWN];

/// ftCo_800DD1E4 (800DD1E4): main horizontal, C-stick horizontal, up, down.
pub fn requested(f: &FighterCore, assets: &FighterAssets) -> Option<&'static Throw> {
    if f.motion_state.id != S::CatchWait {
        return None;
    }
    let stick = f.input.current.stick;
    let last = f.input.previous.stick;
    let cstick = f.input.current.cstick;
    let previous_cstick = f.input.previous.cstick;
    let threshold = assets.input.side_tilt_threshold;
    // ftCo_800DF7F4 (800DF7F4): both sticks use horizontal crossings;
    // the main stick wins before considering any C-stick direction.
    for (current, previous) in [(stick, last), (cstick, previous_cstick)] {
        if (previous.x < threshold && current.x >= threshold)
            || (previous.x > -threshold && current.x <= -threshold)
        {
            return Some(if current.x * f.physics.facing > 0.0 {
                &FORWARD
            } else {
                &BACK
            });
        }
    }
    let up = assets.input.up_tilt_threshold;
    // ftCo_800DF844 (800DF844): upward C-stick crossing.
    if (last.y < up && stick.y >= up) || (previous_cstick.y < up && cstick.y >= up) {
        return Some(&UP);
    }
    let down = assets.input.down_tilt_threshold;
    // ftCo_800DF878, 800DF888/800DF898: both C-stick samples must be <=.
    // Retail has no downward edge test here.
    if (last.y > down && stick.y <= down) || (previous_cstick.y <= down && cstick.y <= down) {
        return Some(&DOWN);
    }
    None
}

/// Borrowed assets and remap live only for the victim's motion transition.
#[derive(Clone, Copy)]
pub(super) struct ThrowSource<'a> {
    pub assets: &'a FighterAssets,
    /// None: the thrower authors no animation for the row (ftData_80085CD8
    /// leaves x590 NULL), so the victim's current AObjs keep playing.
    pub animation: Option<(
        &'a crate::anim::Motion,
        crate::anim::attach::MotionRemapView<'a>,
    )>,
    /// The thrower's row flags (x594) and blend byte, animated or not.
    pub flags: crate::anim::MotionFlags,
    pub blend_frames: f32,
}

/// ftCo_800DD4B0 -> ftCo_800DD398 -> ftCo_800DE3FC.
pub fn enter_throw(
    throw: &Throw,
    victim: &mut Fighter,
    attacker: &mut Fighter,
    va: &FighterAssets,
    aa: &FighterAssets,
) -> Result<()> {
    attacker.character.throw_variant();
    let rate = prepare_throw(throw, &victim.core, &mut attacker.core, aa);
    attacker.change_motion_state_with_rate(throw.state.into(), aa, 0.0, rate)?;
    attacker.step_animation(aa);
    enter_thrown(
        victim,
        attacker,
        va,
        aa,
        throw.victim_state,
        throw.victim_motion,
        rate,
    )?;
    // ftCo_800DD398's tail, ftColl_8007B7A4(gobj, PlCo +348): x1994 =
    // max(x1994, frames) and colour animation 9 (x198C's flash type is
    // renderer state), as the revival platform's exit does.
    let status = &mut attacker.core.status;
    status.revival_invincibility = status
        .revival_invincibility
        .max(aa.grab_escape.throw_invincible_frames);
    attacker
        .core
        .commands
        .color_animations
        .push(melee_cmd::ColorAnimationRequest { id: 9, duration: 0 });
    Ok(())
}

/// The thrower's animation and script for row `victim_motion`, which the
/// victim plays (Fighter_ChangeMotionState's last argument, ftData_80085CD8).
pub(super) fn throw_source<'a>(
    aa: &'a FighterAssets,
    va: &'a FighterAssets,
    victim_motion: i32,
) -> ThrowSource<'a> {
    match aa.motions.get(&victim_motion) {
        Some(motion) => {
            let remap = motion.remap.as_ref().expect("prepared throw skeleton");
            ThrowSource {
                assets: aa,
                animation: Some((
                    motion,
                    crate::anim::attach::MotionRemapView {
                        source: &remap.source,
                        destination: &va.parts,
                        source_masks: &remap.source_masks,
                    },
                )),
                flags: motion.flags,
                blend_frames: motion.blend_frames,
            }
        }
        None => {
            let (flags, blend_frames) = aa.unanimated[&victim_motion];
            ThrowSource {
                assets: aa,
                animation: None,
                flags,
                blend_frames,
            }
        }
    }
}

/// ftCo_800DE3FC (800DE3FC): the victim's half of a throw's entry. Its
/// XRotN is pinned to the thrower (ftCo_800DB368), it takes the thrower's
/// facing and plays the thrower's row `victim_motion` in `victim_state` at
/// `rate`.
pub(super) fn enter_thrown(
    victim: &mut Fighter,
    attacker: &mut Fighter,
    va: &FighterAssets,
    aa: &FighterAssets,
    victim_state: S,
    victim_motion: i32,
    rate: f32,
) -> Result<()> {
    let saved_translation = prepare_thrown_pose(&mut victim.core, &mut attacker.core, va);
    let held_in_mouth = attacker.character.mouth_capture_scale().is_some();
    let source = throw_source(aa, va, victim_motion);
    victim.change_motion_state_with_source(victim_state.into(), va, 0.0, rate, Some(source))?;
    if held_in_mouth {
        // ftCo_800DE3FC: ftColl_8007B62C(gobj, 2), intangible with colour
        // animation 2, before the pose is kept and the new motion animated.
        victim.core.commands.hurt_status = melee_types::combat::HurtStatus::Intangible;
        victim
            .core
            .commands
            .color_animations
            .push(melee_cmd::ColorAnimationRequest { id: 2, duration: 0 });
    }
    finish_thrown_pose(
        &mut victim.core,
        &mut attacker.core,
        va,
        aa,
        saved_translation,
    );
    Ok(())
}

/// lb_8000C1C0: resolve the cross-fighter target before this fighter's callback.
pub fn update_constraint(
    victim: &mut FighterCore,
    attacker: &mut FighterCore,
    va: &FighterAssets,
    aa: &FighterAssets,
) {
    let target = bone_position(
        &mut attacker.skeleton,
        attacker.animation.root,
        usize::from(aa.parts.joint(FtPart::TransN2).expect("throw TransN2")),
        Vec3::ZERO,
    );
    let xrot =
        victim.animation.parts[usize::from(va.parts.joint(FtPart::XRotN).expect("XRotN"))].joint;
    victim.skeleton.set_position_constraint(xrot, Some(target));
}

impl FighterCore {
    /// ftCommon_8007E3EC (8007E3EC): proportion the borrowed HipN translation.
    pub(super) fn thrown_animation(&mut self, assets: &FighterAssets) {
        let hip = self.animation.parts
            [usize::from(assets.parts.joint(FtPart::HipN).expect("HipN"))]
        .joint;
        if !self.skeleton.mtx_is_dirty(hip) {
            return;
        }
        let pose = self.combat.thrown_pose.as_mut().expect("thrown pose");
        let current = self.skeleton.translation(hip);
        let scale = self.combat.capture_geometry.hip_scale;
        // retail 8007E4C8/E4DC/E4F0: fmadds after separately rounded differences.
        pose.hip_translation = Vec3::new(
            gekko_math::fma::fmadds(
                current.x - pose.hip_translation.x,
                scale,
                pose.hip_translation.x,
            ),
            gekko_math::fma::fmadds(
                current.y - pose.hip_translation.y,
                scale,
                pose.hip_translation.y,
            ),
            gekko_math::fma::fmadds(
                current.z - pose.hip_translation.z,
                scale,
                pose.hip_translation.z,
            ),
        );
        self.skeleton.set_translate(hip, &pose.hip_translation);
    }
    /// ftCo_800DE508 (800DE508): constrained XRotN becomes fighter position.
    pub fn thrown_accessory(&mut self, assets: &FighterAssets) {
        let mut position = bone_position(
            &mut self.skeleton,
            self.animation.root,
            usize::from(assets.parts.joint(FtPart::XRotN).expect("XRotN")),
            Vec3::ZERO,
        );
        let offset = self.combat.capture_geometry.root_offset;
        // retail 800DE558/800DE56C: fmadds, with a separate Z-offset scale product.
        position.x = gekko_math::fma::fmadds(
            self.physics.facing,
            offset.z * self.player.scale,
            position.x,
        );
        position.y = gekko_math::fma::fmadds(offset.y, self.player.scale, position.y);
        position.z = 0.0;
        self.physics.position = position;
        self.skeleton.set_translate(self.animation.root, &position);
    }
}

/// ftCo_800DDDE4 (800DDDE4) and ftCo_800DE7C0 (800DE7C0): release without hitlag.
pub fn release_throw(
    victim: &mut Fighter,
    attacker: &mut Fighter,
    va: &FighterAssets,
    aa: &FighterAssets,
    map: &mut melee_mp::CollMap,
    rng: &mut gekko_math::HsdRng,
) -> Result<()> {
    // ftCo_800DE7C0's integer argument is a motion override (90), not an angle.
    let forced_motion = (attacker.motion_state.id == S::ThrowLw).then_some(S::DamageFlyTop);
    let hit = release_captured(&mut victim.core, &mut attacker.core, va, aa, map, true);
    // fn_800DE798 restores the owner inside motion entry, before initial
    // damage-animation commands can create throw-owner-only hitboxes.
    launch_thrown(
        victim,
        hit,
        forced_motion,
        Some(attacker.spawn_number),
        va,
        rng,
    )
}

/// ftCo_800DE7C0's launch: the damage entry, then throw DI without hitlag
/// or ASDI.
pub(super) fn launch_thrown(
    victim: &mut Fighter,
    hit: melee_coll::damage::ReceivedHit,
    forced_motion: Option<S>,
    throw_owner: Option<u32>,
    va: &FighterAssets,
    rng: &mut gekko_math::HsdRng,
) -> Result<()> {
    // ftCo_800DDDE4 always routes the throw damage through ftColl_80076640.
    victim.begin_damage_reaction(hit, forced_motion, None, throw_owner, true, va, rng)?;
    let stick = victim.input.current.stick;
    super::damage::apply_directional_influence(
        &mut victim.physics.knockback_velocity,
        stick,
        va.damage.influence.maximum_angle_degrees,
    );
    Ok(())
}

/// ftCo_800DDDE4 (800DDDE4): release geometry and damage data, before the
/// victim's character-aware motion entry. `offset` is its third argument:
/// ftCo_800DE2A8 (throws) places the victim's root at the captor's TransN2
/// plus the capture offset (x1A70), ftCo_800DE2CC (Egg Lay) at TransN2.
pub(super) fn release_captured(
    victim: &mut FighterCore,
    attacker: &mut FighterCore,
    va: &FighterAssets,
    aa: &FighterAssets,
    map: &mut melee_mp::CollMap,
    offset: bool,
) -> melee_coll::damage::ReceivedHit {
    let hit = record_throw_hit(victim, attacker, va, aa);
    // ftCommon_8007D5D4 on the constrained fighter, here the victim.
    victim.leave_ground();
    detach(victim, attacker, va, aa, map, offset);
    victim.combat.grab = None;
    attacker.combat.grab = None;
    hit
}

/// ftCo_800DDDE4's damage half: the captor's throw record 0 against the
/// victim, the captor's stale-move entry and the victim's launch data.
pub(super) fn record_throw_hit(
    victim: &mut FighterCore,
    attacker: &mut FighterCore,
    va: &FighterAssets,
    aa: &FighterAssets,
) -> melee_coll::damage::ReceivedHit {
    attacker.commands.grab_release = false;
    let hit = attacker.commands.throw_hitboxes[0]
        .as_ref()
        .expect("throw damage");
    let descriptor = throw_descriptor(hit);
    let knockback = va.damage.knockback_with_damage(
        &descriptor,
        victim.physics.percent,
        va.damage.throw_weight,
        attacker.commands.throw_damage_counts[0],
    );
    attacker.combat.has_recorded_hit = true;
    // ftColl_8007891C -> plStale_UpdateStaleMovesFromFighter: one entry per throw.
    attacker.combat.stale.record();
    attacker.commands.stale_multiplier = Some(attacker.combat.stale.multiplier(&aa.stale_weights));
    attacker.commands.first_hit_stale_penalty = Some(aa.first_stale_penalty);
    melee_coll::damage::ReceivedHit {
        facing: -attacker.physics.facing,
        facing_override: if descriptor.angle > 90 && descriptor.angle < 270 {
            Some(attacker.physics.facing)
        } else {
            None
        },
        percent_damage: descriptor.damage,
        descriptor,
        height: melee_coll::hurtbox::HurtHeight::Middle,
        knockback,
    }
}

/// ftCo_800DDDE4's release geometry (x2226_b2): the constrained fighter
/// lets go of `anchor`'s TransN2, takes back its XRotN translation (x2174)
/// and re-enters the map from the anchor's centre to that point.
pub(super) fn detach(
    constrained: &mut FighterCore,
    anchor: &mut FighterCore,
    constrained_assets: &FighterAssets,
    anchor_assets: &FighterAssets,
    map: &mut melee_mp::CollMap,
    offset: bool,
) {
    let mut position = bone_position(
        &mut anchor.skeleton,
        anchor.animation.root,
        usize::from(
            anchor_assets
                .parts
                .joint(FtPart::TransN2)
                .expect("throw TransN2"),
        ),
        Vec3::ZERO,
    );
    if offset {
        let offset = constrained.combat.capture_geometry.root_offset;
        // retail 800DE084/800DE098: fmadds, separate scale of local Z first.
        position.x = gekko_math::fma::fmadds(
            constrained.physics.facing,
            offset.z * constrained.player.scale,
            position.x,
        );
        position.y = gekko_math::fma::fmadds(offset.y, constrained.player.scale, position.y);
        position.z = 0.0;
    }
    let pose = constrained.combat.thrown_pose.take().expect("throw pose");
    let xrot = constrained.animation.parts[usize::from(
        constrained_assets
            .parts
            .joint(FtPart::XRotN)
            .expect("XRotN"),
    )]
    .joint;
    constrained.skeleton.set_position_constraint(xrot, None);
    constrained
        .skeleton
        .set_translate(xrot, &pose.saved_translation);
    let cd = &mut constrained.collision.data;
    cd.last_pos = Vec3::new(
        anchor.physics.position.x,
        anchor.physics.position.y
            + 0.5 * (anchor.collision.data.ecb.top.y + anchor.collision.data.ecb.bottom.y),
        anchor.physics.position.z,
    );
    melee_mp::mark_ecb_clear(cd);
    cd.cur_pos = position;
    constrained
        .skeleton
        .set_translate(constrained.animation.root, &position);
    constrained.collision.lock_frames = 0;
    constrained.collision.data.x130_flags &= !melee_types::mp::coll_data_x130::LOCKED;
    let ecb_pose = crate::collision::ecb::EcbPose::read(
        &mut constrained.skeleton,
        constrained.animation.root,
        &constrained.collision.data,
    );
    map.air_collide_pass(
        &mut constrained.collision.data,
        Some(&|bone| ecb_pose.position(bone)),
    );
    constrained.physics.position = constrained.collision.data.cur_pos;
    constrained
        .skeleton
        .set_translate(constrained.animation.root, &constrained.physics.position);
}

/// A throw record (xDF4) as a hit descriptor: the same damage shape, with
/// the unused collision fields zero.
pub(super) fn throw_descriptor(
    hit: &melee_types::combat::ThrowHitbox,
) -> melee_types::combat::HitboxDescriptor {
    melee_types::combat::HitboxDescriptor {
        group: 0,
        bone: 0,
        common_bone: false,
        requires_throw_owner: false,
        damage: hit.damage,
        shield_damage: 0,
        sound_severity: hit.sound_severity,
        radius: 0.0,
        offset: Vec3::ZERO,
        angle: hit.angle,
        growth: hit.growth,
        weight_knockback: hit.weight_knockback,
        base_knockback: hit.base_knockback,
        element: hit.element,
        hit_ground: true,
        hit_air: true,
        ignore_scale: false,
        clank: false,
        rebound: false,
    }
}

/// ftCo_800DD4B0: weight-dependent playback rate before captor motion entry.
fn prepare_throw(
    throw: &Throw,
    victim: &FighterCore,
    attacker: &mut FighterCore,
    aa: &FighterAssets,
) -> f32 {
    // 800DD4B0 --fused: separate weight multiplication, reciprocal division.
    let rate = if attacker.attributes.combat.weight_independent_throws_mask & throw.weight_mask != 0
    {
        1.0
    } else {
        1.0 / (victim.attributes.size.weight * aa.throw_weight_scale)
    };
    attacker.commands.variables[0] = 0;
    attacker.commands.grab_release = false;
    attacker.commands.throw_reverse = false;
    attacker.commands.throw_accessory = false;
    rate
}
/// ftCo_800DB368 (800DB368), unconstrained branch: zero XRotN's rotation
/// and keep its local translation (x2174) for the release.
fn detach_xrot(victim: &mut FighterCore, va: &FighterAssets) -> Vec3 {
    let xrot =
        victim.animation.parts[usize::from(va.parts.joint(FtPart::XRotN).expect("XRotN"))].joint;
    let saved_translation = victim.skeleton.translation(xrot);
    victim.skeleton.set_rotation(
        xrot,
        &hsd_anim::quat::Quaternion {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 0.0,
        },
    );
    saved_translation
}
fn hip_translation(victim: &FighterCore, va: &FighterAssets) -> Vec3 {
    let hip =
        victim.animation.parts[usize::from(va.parts.joint(FtPart::HipN).expect("HipN"))].joint;
    victim.skeleton.translation(hip)
}
/// ftCo_800DB368 (800DB368): pin XRotN to the captor's TransN2 (x2226_b2).
pub(super) fn constrain_to_captor(
    victim: &mut FighterCore,
    attacker: &mut FighterCore,
    va: &FighterAssets,
    aa: &FighterAssets,
) {
    let saved_translation = detach_xrot(victim, va);
    victim.combat.thrown_pose = Some(ThrownPose {
        saved_translation,
        // ftCommon_8007E358 sets this again at the throw.
        hip_translation: hip_translation(victim, va),
    });
    update_constraint(victim, attacker, va, aa);
}
/// ftCo_800DD398 / ftCo_800DE3FC: pose remapping before victim motion entry.
/// A victim already pinned (x2226_b2) keeps its saved translation.
fn prepare_thrown_pose(
    victim: &mut FighterCore,
    attacker: &mut FighterCore,
    va: &FighterAssets,
) -> Vec3 {
    attacker.status.grab_exclusions = super::ledge::GrabExclusions::ALL;
    let saved_translation = match &victim.combat.thrown_pose {
        Some(pose) => pose.saved_translation,
        None => detach_xrot(victim, va),
    };
    victim.physics.facing = attacker.physics.facing;
    victim.commands.thrown_by = Some(attacker.spawn_number);
    saved_translation
}
/// ftCo_800DE3FC: retain HipN and apply the constraint after victim entry.
fn finish_thrown_pose(
    victim: &mut FighterCore,
    attacker: &mut FighterCore,
    va: &FighterAssets,
    aa: &FighterAssets,
    saved_translation: Vec3,
) {
    victim.combat.thrown_pose = Some(ThrownPose {
        saved_translation,
        hip_translation: hip_translation(victim, va),
    });
    victim.status.grab_exclusions = super::ledge::GrabExclusions::ALL;
    victim.step_animation(va);
    update_constraint(victim, attacker, va, aa);
}
