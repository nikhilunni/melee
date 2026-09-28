//! A fighter's hitboxes landing on items: it_802703E8's per-fighter pass
//! (itcoll.c:397-488). The item owns the damage log and resolves it
//! (melee_it::hurt); the attacker records the item as a victim and the
//! damage it dealt.
use super::Fighter;
use melee_coll::geometry::{capsule_contact, Capsule};
use melee_it::{hurt::HurtCapsules, hurt::ItemHit, ItemCore};
use melee_types::{fixed::FixedVec, GroundOrAir, HitElement};

impl Fighter {
    /// it_802703E8 for this fighter: each hitbox in ID order tests the item's
    /// hurt capsules in order and the first contact lands.
    pub fn strike_item(
        &mut self,
        item: &mut ItemCore,
        capsules: &HurtCapsules,
    ) -> FixedVec<ItemHit, 4> {
        let mut hits = FixedVec::default();
        if capsules.is_empty() {
            return hits;
        }
        // The owner's hits land only once it dropped or threw the item (xDCE b0).
        if item.owner == Some(self.player.id) && !item.hurt_by_owner {
            return hits;
        }
        // x1064_thrownHitbox.owner: a thrown fighter's hits count as its
        // thrower's; only unowned items are supported here.
        assert!(
            self.commands.thrown_by.is_none() || item.owner.is_none(),
            "it_802703E8: a thrown fighter against an owned item"
        );
        let victim = item.hitbox_victim();
        for id in 0..self.commands.hitboxes.len() {
            let Some(hit) = &self.commands.hitboxes[id] else {
                continue;
            };
            let desc = &hit.descriptor;
            // it_802703E8: x42_b7 (hits items), cleared only by
            // ftAction_80071708.
            let grounded = item.ground_or_air == GroundOrAir::Ground;
            if !hit.hits_items
                || desc.element == HitElement::Catch
                || !((desc.hit_air && !grounded) || (desc.hit_ground && grounded))
                || hit.victims.contains(&victim)
                || item.hurt_intangible
            {
                continue;
            }
            let radius = if desc.ignore_scale {
                desc.radius
            } else {
                desc.radius * self.player.scale
            };
            let contact = capsules.iter().find_map(|capsule| {
                capsule_contact(
                    Capsule {
                        start: hit.previous_position,
                        end: hit.position,
                        radius,
                    },
                    Capsule {
                        start: capsule.start,
                        end: capsule.end,
                        radius: capsule.radius,
                    },
                    &capsule.matrix,
                    3.0 * item.scale,
                )
            });
            let Some(contact) = contact else {
                continue;
            };
            assert_ne!(
                desc.element,
                HitElement::Inert,
                "it_802703E8: an inert hitbox touching an item (fighter->unk_gobj)"
            );
            let hit = ItemHit {
                source: melee_it::hurt::ItemHitSource::Fighter {
                    player: self.player.id,
                    x: self.physics.position.x,
                },
                damage: desc.damage,
                angle: desc.angle,
                growth: desc.growth,
                weight_knockback: desc.weight_knockback,
                base_knockback: desc.base_knockback,
                contact: contact.position,
            };
            let group = desc.group;
            // ftColl_80076808(fp, hit, 0, item, 0): the group remembers the item.
            melee_coll::detection::record_victim(&mut self.commands.hitboxes, group, victim);
            // dmg.x1914 takes this hit's damage outright, and the item totals it.
            let damage = gekko_math::msl::fctiwz(hit.damage);
            self.combat.dealt_damage = damage;
            item.pending_damage_taken += damage;
            if damage > item.largest_damage_taken {
                item.largest_damage_taken = damage;
            }
            hits.push(hit);
        }
        hits
    }
}
