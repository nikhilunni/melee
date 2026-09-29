//! Per-victim damage logs, ftcoll.c `dmg_log0` (hits) and `dmg_log1` (phantom
//! "tip" hits).
//!
//! Retail detects every contact against one victim first, logging each, and
//! only then resolves them (ftColl_8007A06C): every entry spawns its hit
//! effect in log order, and the entry with the strictly largest knockback
//! decides the reaction. Knockback is evaluated at resolution time because it
//! depends on the damage of *all* hits logged this frame (Fighter
//! `dmg.x1838_percentTemp`). Fighters and items share these types.
use crate::damage::ReceivedHit;
use hsd_types::Vec3;
use melee_types::fixed::FixedVec;

/// `dmg_log0` / `dmg_log1` length (ftcoll.c:78-79). Retail reports and drops
/// entries past this; so does the port.
pub const DAMAGE_LOG_CAPACITY: usize = 20;

/// Who dealt a logged hit (DmgLogEntry.x0 / gobj).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HitSource {
    /// Entry kind 1: a fighter's hitbox, by spawn number.
    Fighter(u32),
    /// Entry kind 2: an item's hitbox.
    Item(ItemSource),
}

/// What a phantom's credit (ftColl_8007BE3C) reads of the item that dealt
/// it: its owner (ip->owner, the player's second fighter when `secondary`)
/// and its attack (xD88 / xD8C).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ItemSource {
    pub owner: Option<u8>,
    pub secondary: bool,
    pub attack: Option<melee_types::combat::AttackInstance>,
}

/// One DmgLogEntry. `hit.knockback` is filled in when the log is resolved.
#[derive(Clone, Debug)]
pub struct LoggedHit {
    pub source: HitSource,
    pub hit: ReceivedHit,
    /// DmgLogEntry.pos: the hitbox's hurt-contact position (hit0->hurt_coll_pos).
    pub position: Vec3,
    /// DmgLogEntry.size_of_xC: the hitbox's unstaled damage count used by
    /// the knockback formula (HitCapsule.unk_count).
    pub knockback_damage: u32,
    /// Damage this entry adds to the victim's frame damage
    /// (dmg.x1838_percentTemp); for phantoms, the halved damage applied later.
    pub damage: f32,
    /// DmgLogEntry.x20: the damage the entry's hit effect receives. Items
    /// report their hitbox damage before a captured victim's scaling.
    pub effect_damage: f32,
}

/// A fixed-capacity damage log.
#[derive(Clone, Debug, Default)]
pub struct DamageLog {
    entries: FixedVec<LoggedHit, DAMAGE_LOG_CAPACITY>,
}

impl DamageLog {
    /// ftColl_800765E0: each victim's detection starts from empty logs.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    /// Append an entry; past capacity retail's HSD_ASSERTREPORT drops it.
    pub fn push(&mut self, entry: LoggedHit) {
        if self.entries.len() < DAMAGE_LOG_CAPACITY {
            self.entries.push(entry);
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = &LoggedHit> {
        self.entries.iter()
    }
    /// ftColl_8007A06C's `kb > best_kb` scan (best starts at -1): the first
    /// entry with the strictly largest knockback wins. Returns its index.
    pub fn strongest(&self, knockback: impl Fn(&LoggedHit) -> f32) -> Option<(usize, f32)> {
        let mut best: Option<(usize, f32)> = None;
        for (index, entry) in self.entries.iter().enumerate() {
            let kb = knockback(entry);
            if kb > best.map_or(-1.0, |(_, b)| b) {
                best = Some((index, kb));
            }
        }
        best
    }
    pub fn get(&self, index: usize) -> Option<&LoggedHit> {
        self.entries.iter().nth(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hurtbox::HurtHeight;
    use melee_types::combat::HitboxDescriptor;

    fn entry(damage: f32) -> LoggedHit {
        LoggedHit {
            source: HitSource::Item(Default::default()),
            hit: ReceivedHit {
                descriptor: HitboxDescriptor {
                    group: 0,
                    bone: 0,
                    common_bone: false,
                    requires_throw_owner: false,
                    damage,
                    shield_damage: 0,
                    sound_severity: 0,
                    radius: 1.0,
                    offset: Vec3::ZERO,
                    angle: 0,
                    growth: 0,
                    weight_knockback: 0,
                    base_knockback: 0,
                    element: melee_types::HitElement::Normal,
                    hit_ground: true,
                    hit_air: true,
                    ignore_scale: false,
                    clank: true,
                    rebound: true,
                },
                height: HurtHeight::Middle,
                facing: 1.0,
                knockback: 0.0,
                facing_override: None,
                percent_damage: damage,
            },
            position: Vec3::ZERO,
            knockback_damage: damage as u32,
            damage,
            effect_damage: damage,
        }
    }

    #[test]
    fn strongest_prefers_the_first_of_equal_knockbacks() {
        let mut log = DamageLog::default();
        for damage in [3.0, 9.0, 9.0, 4.0] {
            log.push(entry(damage));
        }
        assert_eq!(log.strongest(|e| e.damage), Some((1, 9.0)));
    }

    #[test]
    fn strongest_of_an_empty_log_is_none_and_zero_knockback_still_wins() {
        let mut log = DamageLog::default();
        assert_eq!(log.strongest(|e| e.damage), None);
        log.push(entry(0.0));
        assert_eq!(log.strongest(|e| e.damage), Some((0, 0.0)));
    }

    #[test]
    fn entries_past_capacity_are_dropped() {
        let mut log = DamageLog::default();
        for _ in 0..DAMAGE_LOG_CAPACITY + 3 {
            log.push(entry(1.0));
        }
        assert_eq!(log.len(), DAMAGE_LOG_CAPACITY);
    }
}
