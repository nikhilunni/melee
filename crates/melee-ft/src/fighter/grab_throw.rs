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
    pub motion: &'a crate::anim::Motion,
    pub remap: crate::anim::attach::MotionRemapView<'a>,
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
    let saved_translation = prepare_thrown_pose(&mut victim.core, &mut attacker.core, va);
    let motion = &aa.motions[&throw.victim_motion];
    let remap = motion.remap.as_ref().expect("prepared throw skeleton");
    let source = ThrowSource {
        assets: aa,
        motion,
        remap: crate::anim::attach::MotionRemapView {
            source: &remap.source,
            destination: &va.parts,
            source_masks: &remap.source_masks,
        },
    };
    victim.change_motion_state_with_source(
        throw.victim_state.into(),
        va,
        0.0,
        rate,
        Some(source),
    )?;
    finish_thrown_pose(
        &mut victim.core,
        &mut attacker.core,
        va,
        aa,
        saved_translation,
    );
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
    let hit = prepare_throw_release(&mut victim.core, &mut attacker.core, va, aa, map);
    // fn_800DE798 restores the owner inside motion entry, before initial
    // damage-animation commands can create throw-owner-only hitboxes.
    victim.begin_damage_reaction(
        hit,
        forced_motion,
        None,
        Some(attacker.spawn_number),
        va,
        rng,
    )?;
    // ftCo_800DE7C0: throw DI follows damage entry, without hitlag/ASDI.
    let stick = victim.input.current.stick;
    super::damage::apply_directional_influence(
        &mut victim.physics.knockback_velocity,
        stick,
        va.damage.influence.maximum_angle_degrees,
    );
    Ok(())
}

/// ftCo_800DDDE4 / ftCo_800DE7C0: release geometry and damage data,
/// before the victim's character-aware motion entry.
fn prepare_throw_release(
    victim: &mut FighterCore,
    attacker: &mut FighterCore,
    va: &FighterAssets,
    aa: &FighterAssets,
    map: &mut melee_mp::CollMap,
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
    let mut position = bone_position(
        &mut attacker.skeleton,
        attacker.animation.root,
        usize::from(aa.parts.joint(FtPart::TransN2).expect("throw TransN2")),
        Vec3::ZERO,
    );
    let offset = victim.combat.capture_geometry.root_offset;
    // retail 800DE084/800DE098: fmadds, separate scale of local Z first.
    position.x = gekko_math::fma::fmadds(
        victim.physics.facing,
        offset.z * victim.player.scale,
        position.x,
    );
    position.y = gekko_math::fma::fmadds(offset.y, victim.player.scale, position.y);
    position.z = 0.0;
    let pose = victim.combat.thrown_pose.take().expect("throw pose");
    let xrot =
        victim.animation.parts[usize::from(va.parts.joint(FtPart::XRotN).expect("XRotN"))].joint;
    victim.skeleton.set_position_constraint(xrot, None);
    victim.skeleton.set_translate(xrot, &pose.saved_translation);
    victim.leave_ground();
    let cd = &mut victim.collision.data;
    cd.last_pos = Vec3::new(
        attacker.physics.position.x,
        attacker.physics.position.y
            + 0.5 * (attacker.collision.data.ecb.top.y + attacker.collision.data.ecb.bottom.y),
        attacker.physics.position.z,
    );
    melee_mp::mark_ecb_clear(cd);
    cd.cur_pos = position;
    victim
        .skeleton
        .set_translate(victim.animation.root, &position);
    victim.collision.lock_frames = 0;
    victim.collision.data.x130_flags &= !melee_types::mp::coll_data_x130::LOCKED;
    let ecb_pose = crate::collision::ecb::EcbPose::read(
        &mut victim.skeleton,
        victim.animation.root,
        &victim.collision.data,
    );
    map.air_collide_pass(
        &mut victim.collision.data,
        Some(&|bone| ecb_pose.position(bone)),
    );
    victim.physics.position = victim.collision.data.cur_pos;
    victim
        .skeleton
        .set_translate(victim.animation.root, &victim.physics.position);
    victim.combat.grab = None;
    attacker.combat.grab = None;
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
/// ftCo_800DD398 / ftCo_800DE3FC: pose remapping before victim motion entry.
fn prepare_thrown_pose(
    victim: &mut FighterCore,
    attacker: &mut FighterCore,
    va: &FighterAssets,
) -> Vec3 {
    attacker.status.grab_exclusions = super::ledge::GrabExclusions::ALL;
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
    let hip =
        victim.animation.parts[usize::from(va.parts.joint(FtPart::HipN).expect("HipN"))].joint;
    victim.combat.thrown_pose = Some(ThrownPose {
        saved_translation,
        hip_translation: victim.skeleton.translation(hip),
    });
    victim.status.grab_exclusions = super::ledge::GrabExclusions::ALL;
    victim.step_animation(va);
    update_constraint(victim, attacker, va, aa);
}
