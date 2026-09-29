//! Resolving a victim's damage logs after hit detection, ftcoll.c.
//!
//! Detection (`damage::detect_hit`, `Fighter::detect_item_hit`) only logs
//! contacts. Retail then resolves both logs for the victim before its next
//! proc (Fighter_8006CB94 -> ftColl_8007AB48 / ftColl_8007AB80):
//!
//! - ordinary hits: each entry's knockback uses the damage of *every* hit
//!   logged this frame, every entry spawns its hit effect in log order, and
//!   the strongest entry becomes the hit ProcessHit reacts to;
//! - phantom contacts: the strongest one is kept. ProcessHit gives it hitlag
//!   now and applies its halved damage when that hitlag lockout expires.
use super::assets::FighterAssets;
use super::damage::{CombatState, PhantomHit};
use super::FighterCore;
use gekko_math::msl::fctiwz;
use hsd_types::Vec3;
use melee_coll::damage::ReceivedHit;
use melee_coll::damage_log::{DamageLog, HitSource, LoggedHit};
use melee_ef::request::EffectRequest;
use melee_types::HitElement;

impl CombatState {
    /// ftColl_80076ED8 ordinary branch: log the hit, accumulate its damage
    /// (inlineB2) and discard this frame's phantom contacts (`dmg_log1_idx = 0`).
    pub fn log_hit(&mut self, entry: LoggedHit) {
        let damage = entry.damage;
        self.frame_damage += damage;
        self.frame_max_damage = self.frame_max_damage.max(damage_count(damage));
        self.phantom_log.clear();
        self.hit_log.push(entry);
    }
}

/// Retail's integer damage count: truncated, but at least 1 when nonzero.
pub(super) fn damage_count(damage: f32) -> i32 {
    if damage == 0.0 {
        0
    } else {
        fctiwz(damage).max(1)
    }
}

impl FighterCore {
    /// ftColl_800765E0: this fighter's detection starts from empty logs.
    pub fn begin_hit_detection(&mut self) {
        self.combat.hit_log.clear();
        self.combat.phantom_log.clear();
    }

    /// ftColl_8007AB48 then ftColl_8007AB80.
    pub fn resolve_hit_logs(&mut self, assets: &FighterAssets) {
        self.resolve_hit_log(assets);
        self.resolve_phantom_log(assets);
    }

    /// ftColl_80079AB0 for a logged entry: the victim's damage count plus
    /// this frame's accumulated damage (dmg.x1838_percentTemp).
    fn logged_knockback(&self, entry: &LoggedHit, assets: &FighterAssets) -> f32 {
        assets.damage.knockback_for_frame(
            &entry.hit.descriptor,
            self.physics.percent,
            self.combat.frame_damage,
            self.attributes.size.weight,
            entry.knockback_damage,
        )
    }

    /// ftColl_8007AB48 -> ftColl_8007A06C(.., dmg_log0, .., 1).
    fn resolve_hit_log(&mut self, assets: &FighterAssets) {
        let log = std::mem::take(&mut self.combat.hit_log);
        for entry in log.iter() {
            let knockback = self.logged_knockback(entry, assets);
            self.push_hit_effects(
                entry.position,
                entry.hit.descriptor.element,
                entry.hit.descriptor.sound_severity,
                entry.effect_damage,
                knockback,
                assets,
            );
        }
        if let Some((index, knockback)) = log.strongest(|e| self.logged_knockback(e, assets)) {
            let entry = log.get(index).expect("strongest entry");
            let captor = match self.combat.grab {
                Some(super::grab::GrabLink::Captured { captor }) => Some(captor),
                _ => None,
            };
            self.combat.pending_from_captor =
                captor.is_some_and(|c| entry.source == HitSource::Fighter(c));
            self.combat.pending = Some(ReceivedHit {
                knockback,
                percent_damage: self.combat.frame_damage,
                ..entry.hit.clone()
            });
        }
        self.combat.hit_log = cleared(log);
    }

    /// ftColl_8007AB80: x187c = 0, then ftColl_8007A06C(.., dmg_log1, .., 0)
    /// keeps the strongest phantom without effects; x18a0 = x187c.
    fn resolve_phantom_log(&mut self, assets: &FighterAssets) {
        self.combat.phantom_knockback = 0.0;
        let log = std::mem::take(&mut self.combat.phantom_log);
        if let Some((index, knockback)) = log.strongest(|e| self.logged_knockback(e, assets)) {
            let entry = log.get(index).expect("strongest entry");
            self.combat.phantom = Some(PhantomHit {
                position: entry.position,
                element: entry.hit.descriptor.element,
                sound_severity: entry.hit.descriptor.sound_severity,
                source: entry.source,
                damage: entry.damage,
            });
            self.combat.phantom_knockback = knockback;
        }
        self.combat.phantom_log = cleared(log);
    }

    /// ftColl_8007A06C / ftColl_8007BE3C effect switch for one hit:
    /// ftColl_80078538 for normal hits (large spark from PlCo +3F0, plus the
    /// character's extra spark when the hit has a sound severity), the element
    /// effect otherwise. The effect receives the damage converted to u32.
    fn push_hit_effects(
        &mut self,
        position: Vec3,
        element: HitElement,
        sound_severity: u8,
        damage: f32,
        knockback: f32,
        assets: &FighterAssets,
    ) {
        // ftcoll.c hit_effect_ids: these elements have no effect (-1, and
        // ReDead's 0, which the switch's default ignores).
        if matches!(
            element,
            HitElement::Nap
                | HitElement::Sleep
                | HitElement::Catch
                | HitElement::Inert
                | HitElement::Disable
                | HitElement::ScrewAttack
                | HitElement::Lipstick
                | HitElement::ReDead
        ) {
            return;
        }
        // hit_effect_ids: Ground and Cape hits take the normal spark
        // (Ef_Id_Unk1000 -> ftColl_80078538).
        let element = match element {
            HitElement::Ground | HitElement::Cape => HitElement::Normal,
            element => element,
        };
        self.effects.push(EffectRequest::HitSpark {
            position,
            element,
            damage: fctiwz(damage) as f32,
            large: knockback >= assets.damage.large_spark_threshold,
        });
        if element == HitElement::Normal && sound_severity >= 1 {
            let variant = self.attributes.combat.hit_spark_variant;
            if let Some(&random_bound) = assets.damage.extra_spark_bounds.get(variant as usize) {
                self.effects.push(EffectRequest::NormalSparkExtra {
                    position,
                    facing: self.physics.facing,
                    variant,
                    random_bound,
                });
            }
        }
    }

    /// Fighter_ProcessHit, fighter.c:2844-2850: count down the phantom
    /// lockout (dmg.x189C). When it runs out without an ordinary hit this
    /// frame, ftColl_8007BE3C applies the phantom's damage and effect.
    pub(super) fn expire_phantom_lockout(
        &mut self,
        received_knockback: bool,
        assets: &FighterAssets,
    ) {
        if self.combat.phantom_lockout <= 0.0 {
            return;
        }
        self.combat.phantom_lockout -= 1.0;
        if self.combat.phantom_lockout > 0.0 || received_knockback {
            return;
        }
        self.combat.phantom_lockout = 0.0;
        let Some(phantom) = self.combat.phantom.clone() else {
            return;
        };
        self.combat.frame_damage += phantom.damage;
        self.combat.frame_max_damage = self
            .combat
            .frame_max_damage
            .max(damage_count(phantom.damage));
        // plStale_UpdateStaleMovesFromFighter / ftColl_80076444 run on the
        // source fighter (for an item, plStale_UpdateStaleMovesFromItem /
        // ftColl_8007646C on its owner); the scene applies this credit after
        // ProcessHit.
        self.combat.pending_credit = Some(phantom.source);
        let knockback = self.combat.phantom_knockback;
        self.push_hit_effects(
            phantom.position,
            phantom.element,
            phantom.sound_severity,
            phantom.damage,
            knockback,
            assets,
        );
    }

    /// ftColl_8007891C without the stats call: plStale_UpdateStaleMovesFromFighter
    /// and ftColl_80076444 credit this fighter's current move for a hit on `victim`.
    pub fn credit_hit(&mut self, victim: u32, assets: &FighterAssets) {
        self.combat
            .combo
            .record(victim, self.combat.stale.current_move(), &assets.combo);
        self.combat.stale.record();
        self.commands.stale_multiplier = Some(self.combat.stale.multiplier(&assets.stale_weights));
        // The first queue entry is visible to later hitbox commands of this move.
        self.commands.first_hit_stale_penalty = Some(assets.first_stale_penalty);
    }
}

/// Hand a taken log back emptied, keeping it allocation-free.
fn cleared(mut log: DamageLog) -> DamageLog {
    log.clear();
    log
}
