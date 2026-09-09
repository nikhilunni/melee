//! Body-grab startup, ftCo_Catch.c. Linked capture is an explicit proc boundary.
use super::{
    assets::{FighterAssets, Result},
    CharacterCallbacks, Fighter, MotionData,
};
use hsd_types::Vec3;
use melee_types::CommonMotionState as S;

impl<C: CharacterCallbacks> Fighter<C> {
    /// ftCo_800D8C54 (800D8C54): Catch begins at frame zero without an immediate step.
    pub(super) fn enter_catch(&mut self, assets: &FighterAssets) -> Result<()> {
        self.character.catch_variant();
        self.physics.animation_velocity = Vec3::ZERO;
        self.change_motion_state(S::Catch, assets)?;
        self.state_data = MotionData::Catch;
        Ok(())
    }

    /// ftCo_Catch_Anim (800D8CC8): item/tether callbacks are character hooks.
    pub(super) fn catch_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        if !self.animation.frames_remaining(&self.skeleton) {
            self.change_motion_state(S::Wait, assets)?;
        }
        Ok(())
    }

    /// ftCo_Catch_Phys (800D8D88): separate multiplier product; no fused sites.
    pub(super) fn catch_physics(
        &mut self,
        assets: &FighterAssets,
        map: &melee_mp::CollMap,
        wind: Vec3,
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

    /// ftCo_Catch_Coll (800D8E08) -> ft_800841B8: departure during startup.
    pub(super) fn catch_collision(
        &mut self,
        assets: &FighterAssets,
        map: &mut melee_mp::CollMap,
    ) -> Result<()> {
        use crate::collision::ground::{map_ground_action, WaitGroundResult};
        if map_ground_action(
            &mut self.physics,
            &mut self.collision,
            map,
            &mut self.skeleton,
            self.animation.root,
            self.input.current.stick.x,
        ) == WaitGroundResult::EnterFall
        {
            self.leave_ground();
            self.change_motion_state(S::Fall, assets)?;
        }
        Ok(())
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
pub fn candidate<V: CharacterCallbacks, A: CharacterCallbacks>(
    victim: &mut Fighter<V>,
    attacker: &Fighter<A>,
) -> Option<f32> {
    use melee_types::{GroundOrAir, HitElement};
    if attacker.status.disabled
        || attacker.motion_state.id != S::Catch
        || victim.status.disabled
        || victim.combat.grab.is_some()
        || victim.status.grab_exclusions.0 & 1 != 0
        || victim.status.ledge_intangibility != 0
        || victim.commands.hurt_status != super::escape::HurtStatus::Normal
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
pub fn capture_pair<V: CharacterCallbacks, A: CharacterCallbacks>(
    victim: &mut Fighter<V>,
    attacker: &mut Fighter<A>,
    victim_assets: &FighterAssets,
    attacker_assets: &FighterAssets,
    map: &mut melee_mp::CollMap,
) -> Result<()> {
    attacker.character.catch_variant();
    if victim.physics.ground_or_air != melee_types::GroundOrAir::Ground {
        unimplemented!("fn_800DAADC: CapturePulledHi");
    }
    attacker.physics.ground_velocity = 0.0;
    let frame = attacker.animation.frame;
    attacker.commands.grab_release = false;
    attacker.commands.throw_reverse = false;
    attacker.change_motion_state_at(S::CatchPull, attacker_assets, frame)?;
    attacker.combat.grab = Some(GrabLink::Holding {
        victim: victim.spawn_number,
        vertical_offset: 0.0,
    });
    victim.physics.facing = -attacker.physics.facing;
    victim.change_motion_state(S::CapturePulledLw, victim_assets)?;
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
    // fn_800DAA40: initial grounded alignment retains the vertical offset on captor.
    let delta = capture_delta(victim, attacker, victim_assets);
    attacker.combat.grab = Some(GrabLink::Holding {
        victim: victim.spawn_number,
        vertical_offset: delta.y + victim.physics.position.y - attacker.physics.position.y,
    });
    victim.capture_collision(victim_assets, map)?;
    Ok(())
}

/// fn_800DAC78 (800DAC78), no fused sites: hold bone minus captured XRotN.
fn capture_delta<V: CharacterCallbacks, A: CharacterCallbacks>(
    victim: &mut Fighter<V>,
    attacker: &mut Fighter<A>,
    assets: &FighterAssets,
) -> Vec3 {
    let target = super::caches::bone_position(
        &mut attacker.skeleton,
        attacker.animation.root,
        usize::from(attacker.bones.model.shield),
        Vec3::ZERO,
    );
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
    Vec3::new(
        target.x - origin.x,
        target.y - origin.y,
        target.z - origin.z,
    )
}

/// ftCo_CapturePulledLw_Phys (800DB00C): scene calls at this fighter's Update proc.
pub fn align_capture<V: CharacterCallbacks, A: CharacterCallbacks>(
    victim: &mut Fighter<V>,
    attacker: &mut Fighter<A>,
    assets: &FighterAssets,
) {
    let delta = capture_delta(victim, attacker, assets);
    victim.physics.position.x += delta.x;
    victim.physics.position.y += delta.y;
    victim.physics.position.z += delta.z;
}

impl<C: CharacterCallbacks> Fighter<C> {
    /// ftCo_CapturePulledLw_Coll (800DB1F8) -> ft_8008403C.
    pub(super) fn capture_collision(
        &mut self,
        assets: &FighterAssets,
        map: &mut melee_mp::CollMap,
    ) -> Result<()> {
        self.catch_collision(assets, map)?;
        if !matches!(self.motion_state.id, S::CapturePulledLw | S::CaptureWaitLw) {
            unimplemented!("fn_800DB230: captured fighter leaves ground");
        }
        self.skeleton
            .set_translate(self.animation.root, &self.physics.position);
        Ok(())
    }
}

impl<C: CharacterCallbacks> Fighter<C> {
    /// fn_800DA1D8 (800DA1D8): switch captor first, then linked victim in the scene.
    pub(super) fn enter_catch_wait(&mut self, assets: &FighterAssets) -> Result<()> {
        self.physics.ground_velocity = 0.0;
        self.commands.grab_release = false;
        self.change_motion_state(S::CatchWait, assets)?;
        self.status.grab_exclusions = super::ledge::GrabExclusions::ALL;
        self.effects
            .push(super::effects::EffectRequest::CaptureFlash {
                bone: usize::from(self.bones.model.shield),
            });
        Ok(())
    }
}

/// fn_800DB6C8 -> fn_800DBAE4 (800DBAE4), before victim's own Anim proc.
pub fn capture_wait<C: CharacterCallbacks>(
    victim: &mut Fighter<C>,
    assets: &FighterAssets,
) -> Result<()> {
    victim.change_motion_state(S::CaptureWaitLw, assets)?;
    victim.status.grab_exclusions = super::ledge::GrabExclusions::ALL;
    Ok(())
}
