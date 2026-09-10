//! Linked back throw: ftCo_Throw.c / ftCo_Thrown.c.
use super::FighterCore;
use super::{
    assets::{FighterAssets, Result},
    caches::bone_position,
    CharacterCallbacks, Fighter,
};
use hsd_types::Vec3;
use melee_types::{CommonMotionState as S, FtPart};

#[derive(Clone, Copy, Debug, Default)]
pub struct CaptureGeometry {
    pub hip_scale: f32,
    pub root_offset: Vec3,
}
#[derive(Clone, Copy, Debug)]
pub struct ThrownPose {
    pub saved_translation: Vec3,
    pub hip_translation: Vec3,
}

/// ftCo_800DD1E4 (800DD1E4): horizontal stick crossing before the other throws.
pub fn back_throw_requested(f: &FighterCore, assets: &FighterAssets) -> bool {
    if f.motion_state.id != S::CatchWait {
        return false;
    }
    let x = f.input.current.stick.x;
    let last = f.input.previous.stick.x;
    let threshold = assets.input.side_tilt_threshold;
    if (last < threshold && x >= threshold) || (last > -threshold && x <= -threshold) {
        if x * f.physics.facing > 0.0 {
            unimplemented!("ftCo_800DD1E4: ThrowF");
        }
        return true;
    }
    if f.input.pressed.intersects(crate::input::Buttons::A) {
        unimplemented!("fn_800DA4C0: CatchAttack");
    }
    false
}

/// ftCo_800DD4B0 -> ftCo_800DD398 -> ftCo_800DE3FC.
pub fn enter_back_throw<V: CharacterCallbacks, A: CharacterCallbacks>(
    victim: &mut Fighter<V>,
    attacker: &mut Fighter<A>,
    va: &FighterAssets,
    aa: &FighterAssets,
) -> Result<()> {
    attacker.character.throw_variant();
    let rate = prepare_back_throw(&victim.core, &mut attacker.core, aa);
    attacker.change_motion_state_with_rate(S::ThrowB, aa, 0.0, rate)?;
    attacker.step_animation(aa);
    let (saved_translation, motion) =
        prepare_thrown_pose(&mut victim.core, &mut attacker.core, va, aa);
    victim.change_motion_state_with_source(S::ThrownB, va, 0.0, rate, Some((aa, &motion)))?;
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
pub fn release_back_throw<V: CharacterCallbacks, A: CharacterCallbacks>(
    victim: &mut Fighter<V>,
    attacker: &mut Fighter<A>,
    va: &FighterAssets,
    aa: &FighterAssets,
    map: &mut melee_mp::CollMap,
) -> Result<()> {
    let hit = prepare_throw_release(&mut victim.core, &mut attacker.core, va, aa, map);
    victim.begin_damage_reaction(hit, va)?;
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
) -> super::damage::ReceivedHit {
    attacker.commands.grab_release = false;
    if victim.input.current.stick != crate::input::Stick::default() {
        unimplemented!("ftCo_8008E5A4: throw DI");
    }
    let hit = attacker.commands.throw_hitboxes[0]
        .as_ref()
        .expect("throw damage");
    // Throw records use the same damage shape, with unused collision fields zero.
    let descriptor = super::hitbox::HitboxDescriptor {
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
    };
    let knockback =
        va.damage
            .knockback(&descriptor, victim.physics.percent, va.damage.throw_weight);
    attacker.combat.has_recorded_hit = true;
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
    super::damage::ReceivedHit {
        facing: -attacker.physics.facing,
        facing_override: if descriptor.angle > 90 && descriptor.angle < 270 {
            Some(attacker.physics.facing)
        } else {
            None
        },
        descriptor,
        height: super::caches::HurtHeight::Middle,
        knockback,
    }
}

/// ftCo_800DD4B0: weight-dependent playback rate before captor motion entry.
fn prepare_back_throw(victim: &FighterCore, attacker: &mut FighterCore, aa: &FighterAssets) -> f32 {
    // 800DD4B0 --fused: separate weight multiplication, reciprocal division.
    let rate = if attacker.attributes.combat.weight_independent_throws_mask & 2 != 0 {
        1.0
    } else {
        1.0 / (victim.attributes.size.weight * aa.throw_weight_scale)
    };
    attacker.commands.variables[0] = 0;
    attacker.commands.grab_release = false;
    attacker.commands.throw_reverse = false;
    rate
}
/// ftCo_800DD398 / ftCo_800DE3FC: pose remapping before victim motion entry.
fn prepare_thrown_pose(
    victim: &mut FighterCore,
    attacker: &mut FighterCore,
    va: &FighterAssets,
    aa: &FighterAssets,
) -> (Vec3, crate::anim::Motion) {
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
    let mut motion = aa.motions[&263].clone();
    motion.remap = Some(crate::anim::attach::MotionRemap {
        source: aa.parts.clone(),
        destination: va.parts.clone(),
        source_masks: attacker
            .animation
            .parts
            .iter()
            .map(|part| part.motion_mask)
            .collect(),
    });
    victim.commands.thrown_by = Some(attacker.spawn_number);
    (saved_translation, motion)
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
