//! Absorbing bubbles: ftColl_CreateAbsorbHit (8007B2C4), the item pass of
//! ftColl_8007925C that tests them (ftcoll.c:2133-2183) and
//! Fighter_ProcessHit's ftData_OnAbsorb branch (fighter.c:2947-2950).
use super::{shield::ShieldVolume, FighterCore};
use gekko_math::msl::fctiwz;
use melee_coll::{defense::AbsorbDescriptor, hitbox::HitCapsule};

/// Fighter x2218_b6, absorb_hit (+1A08) and AbsorbAttr (+1A40..+1A48).
#[derive(Clone, Debug, Default)]
pub struct AbsorbState {
    /// x2218_b6: the bubble is up. Every motion change takes it down
    /// (fighter.c:1057).
    pub active: bool,
    /// absorb_hit: the bubble, a point capsule on a part.
    pub volume: ShieldVolume,
    /// AbsorbAttr.x1A40_absorbHitDirection: the side the last absorbed
    /// hitbox's item was on, or zero.
    pub direction: f32,
    /// AbsorbAttr.x1A44_damageTaken: the absorbed hitboxes' damage this
    /// frame, each truncated (at least 1 unless it was zero).
    pub damage: i32,
    /// AbsorbAttr.x1A48_hitsTaken: hitboxes absorbed this frame.
    pub hits: i32,
}

/// What Fighter_ProcessHit hands ftData_OnAbsorb: this frame's AbsorbAttr.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Absorbed {
    pub direction: f32,
    pub damage: i32,
    pub hits: i32,
}

impl FighterCore {
    /// ftColl_CreateAbsorbHit (8007B2C4): the bubble goes up on
    /// `descriptor`'s part. Its cached position is left as it stands.
    pub fn create_absorb_hit(&mut self, descriptor: &AbsorbDescriptor) {
        let absorb = &mut self.combat.absorb;
        absorb.active = true;
        absorb.volume.bone = descriptor.bone;
        absorb.volume.radius = descriptor.radius;
        absorb.volume.offset = descriptor.offset;
    }
    /// ftColl_8007AF10 (8007AF10): the bubble's position is read again at
    /// its next test.
    pub fn refresh_absorb_position(&mut self) {
        self.combat.absorb.volume.position_cached = false;
    }
    /// ftColl_8007925C's absorb step for item hitbox `id` (ftcoll.c:2133-
    /// 2183), after the reflectors: true when the bubble took it. Only a
    /// hitbox its script marks absorbable (x42_b0) counts toward the
    /// frame's damage and hits; x2218_b7, which would let a reflectable one
    /// in, is never set (ftColl_CreateAbsorbHit clears it).
    pub(super) fn absorb_item_hit(
        &mut self,
        item: &mut melee_it::ItemCore,
        id: usize,
        hit: &HitCapsule,
    ) -> bool {
        if !self.combat.absorb.active || !item.hit_flags[id].absorbable {
            return false;
        }
        if self.absorb_contact(hit, item.scale).is_none() {
            return false;
        }
        // it_8026FAC4(item, hurt, 6, fp, 0): an untimed victim record.
        melee_coll::detection::record_victim(
            &mut item.hitboxes,
            hit.descriptor.group,
            self.spawn_number,
        );
        let damage = hit.descriptor.damage;
        let count = if damage == 0.0 {
            0
        } else {
            match fctiwz(damage) {
                0 => 1,
                n => n,
            }
        };
        // item->xC90_absorbGObj.
        item.pending_absorb = Some(self.player.id);
        let absorb = &mut self.combat.absorb;
        absorb.direction = if self.physics.position.x > item.position.x {
            -1.0
        } else {
            1.0
        };
        absorb.damage += count;
        absorb.hits += 1;
        true
    }
    /// Fighter_ProcessHit, fighter.c:3036-3038: this frame's AbsorbAttr,
    /// consumed.
    pub(super) fn take_absorbed(&mut self) -> Option<Absorbed> {
        let absorb = &mut self.combat.absorb;
        let taken = Absorbed {
            direction: std::mem::take(&mut absorb.direction),
            damage: std::mem::take(&mut absorb.damage),
            hits: std::mem::take(&mut absorb.hits),
        };
        (taken.direction != 0.0).then_some(taken)
    }
}
