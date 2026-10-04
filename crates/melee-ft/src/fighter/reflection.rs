//! ftColl_80077464 and Fighter_ProcessHit: contact is deferred until link14.
use super::{
    assets::{FighterAssets, Result},
    Fighter, FighterCore,
};
use gekko_math::msl::fctiwz;
use melee_coll::{defense::ReflectDescriptor, hitbox::HitCapsule};

pub type CharacterContact = fn(&mut Fighter, &HitCapsule, f32) -> Option<ReflectDescriptor>;
pub type CharacterResponse = fn(&mut Fighter, f32, &FighterAssets) -> Result<()>;
#[derive(Clone, Copy, Debug)]
pub enum Response {
    Powershield,
    Character,
    None,
}
#[derive(Clone, Copy, Debug)]
pub struct Pending {
    pub direction: f32,
    pub overflow: i32,
    pub response: Response,
}

pub(super) struct Settings {
    pub maximum: i32,
    pub damage_multiplier: f32,
    pub speed_multiplier: f32,
    pub exclude_master_ball_ownership: bool,
    /// x2218_b4: the item keeps its owner (xDCC b1).
    pub preserve_owner: bool,
}
impl FighterCore {
    /// The character owns the descriptor; shared lbColl owns its contact geometry.
    pub fn reflector_contact(
        &mut self,
        hit: &HitCapsule,
        scale: f32,
        descriptor: &ReflectDescriptor,
    ) -> bool {
        // The shared reflection volume is the same retail owner used by Guard.
        // Positions are invalidated by the normal fighter update, not by every hit.
        self.shield.reflect.volume.bone = descriptor.bone;
        self.shield.reflect.volume.offset = descriptor.offset;
        self.shield.reflect.volume.radius = descriptor.radius;
        self.shield_reflect_contact(hit, scale).is_some()
    }
    pub(super) fn record_reflection(
        &mut self,
        item: &mut melee_it::ItemCore,
        id: usize,
        settings: Settings,
        response: Response,
    ) {
        let hit = item.hitboxes[id].as_ref().expect("reflection capsule");
        let damage = if hit.descriptor.damage == 0.0 {
            0
        } else {
            let n = fctiwz(hit.descriptor.damage);
            if n == 0 {
                1
            } else {
                n
            }
        };
        let group = hit.descriptor.group;
        item.record_reflector(group, self.spawn_number);
        let facing = if self.physics.position.x > item.position.x {
            -1.0
        } else {
            1.0
        };
        let direction = if item.velocity.x != 0.0 {
            if item.velocity.x > 0.0 {
                -1.0
            } else {
                1.0
            }
        } else if item.position.x > self.physics.position.x {
            -1.0
        } else {
            1.0
        };
        if damage > settings.maximum {
            if item.hit_flags[id].damage_without_hitlag {
                item.pending_damage_without_hitlag = item.pending_damage_without_hitlag.max(damage);
            } else {
                item.pending_damage_dealt = item.pending_damage_dealt.max(damage);
            }
            item.reflection_direction = facing;
            self.combat.reflection = Some(Pending {
                direction,
                overflow: damage,
                response,
            });
        } else {
            item.pending_reflection = Some(melee_it::PendingReflection {
                owner: self.player.id,
                facing,
                damage_multiplier: settings.damage_multiplier,
                speed_multiplier: settings.speed_multiplier,
                exclude_master_ball_ownership: settings.exclude_master_ball_ownership,
                preserve_owner: settings.preserve_owner,
                // Filled when the item's event proc runs (the reflector's
                // pose then, after its own ProcessHit).
                reflector_position: hsd_types::Vec3::ZERO,
            });
            item.reflection_direction = facing;
            self.combat.reflection = Some(Pending {
                direction,
                overflow: self.combat.reflection.map_or(0, |pending| pending.overflow),
                response,
            });
        }
    }
}
impl Fighter {
    pub(super) fn process_reflection(
        &mut self,
        pending: Pending,
        assets: &FighterAssets,
    ) -> Result<()> {
        if pending.overflow != 0 {
            self.enter_shield_break(assets)?;
            // efAsync_Spawn (800679B0): link14 dispatches the burst now.
            self.core.flush_effects_on_motion_change();
            return Ok(());
        }
        match pending.response {
            Response::Powershield => {
                self.effects
                    .push(melee_ef::request::EffectRequest::DestroyOwned);
                self.core.queue_shield_effect(1050);
                self.core.shield_sound(128);
            }
            Response::Character => {
                let callback = self
                    .character
                    .table()
                    .reflect_hit
                    .expect("live character reflector callback");
                callback(self, pending.direction, assets)?;
            }
            Response::None => {}
        }
        Ok(())
    }
}
