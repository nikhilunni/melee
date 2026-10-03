//! Grounded attack clanks: ftColl_8007699C and ftCo_Rebound.c.
use super::state::{AnimationPhase, PhysicsPhase};
use super::{
    assets::{FighterAssets, Result},
    Fighter, FighterCore, MotionData,
};
use gekko_math::{fma::fmadds, msl::fctiwz};
use hsd_types::Vec3;
use melee_coll::{
    geometry::{hitbox_pair_contact, Capsule},
    hitbox::HitCapsule,
};
use melee_types::{CommonMotionState as S, GroundOrAir, HitElement};

#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    pub priority_gap: i32,
    pub duration_multiplier: f32,
    pub duration_base: f32,
    pub recoil_multiplier: f32,
    pub recoil_base: f32,
}
impl Parameters {
    pub fn read(a: &hsd_archive::Archive, base: u32) -> Result<Self> {
        let r = a.reader();
        Ok(Self {
            priority_gap: r.s32(base + 0x3CC)?,
            duration_multiplier: r.f32(base + 0x3D0)?,
            duration_base: r.f32(base + 0x3D4)?,
            recoil_multiplier: r.f32(base + 0x3D8)?,
            recoil_base: r.f32(base + 0x3DC)?,
        })
    }
}
/// Fighter dmg +1918/+191C/+1920. Cleared at ProcessHit, not motion change.
#[derive(Clone, Copy, Debug, Default)]
pub struct Pending {
    pub damage: i32,
    pub duration: f32,
    pub facing: f32,
}
/// mv.co.rebound +2340/+2344 survives ReboundStop -> Rebound.
#[derive(Clone, Debug)]
pub struct State {
    pub recoil: f32,
    pub animation_rate: f32,
}

/// Receiver-owned mask, built once per opponent traversal in retail hit-ID order.
/// Its entries are removed when inlineA0 logs a newly stopped attack group.
pub(super) fn candidates(receiver: &FighterCore, incoming: &FighterCore) -> [bool; 4] {
    let mut result = [false; 4];
    let mut cursor = melee_coll::detection::PairCursor::default();
    while let Some(i) = cursor.next(
        &receiver.commands.hitboxes,
        incoming.spawn_number,
        incoming.physics.ground_or_air,
    ) {
        let hit = receiver.commands.hitboxes[i].as_ref().unwrap();
        result[i] = hit.descriptor.clank && hit.descriptor.element != HitElement::Inert;
    }
    result
}
fn geometry(hit: &HitCapsule, scale: f32) -> Capsule {
    Capsule {
        start: hit.previous_position,
        end: hit.position,
        radius: if hit.descriptor.ignore_scale {
            hit.descriptor.radius
        } else {
            hit.descriptor.radius * scale
        },
    }
}
fn record_group(owner: &mut FighterCore, group: u8, victim: u32, mut mask: Option<&mut [bool; 4]>) {
    for (i, hit) in owner.commands.hitboxes.iter_mut().enumerate() {
        let Some(hit) = hit else {
            continue;
        };
        if hit.descriptor.group == group && !hit.victims.contains(&victim) {
            // lbColl_80008688 type3 writes victims_1 with rehit timer0.
            hit.victims.push(victim);
            if let Some(mask) = &mut mask {
                mask[i] = false;
            }
        }
    }
}
fn accumulate(owner: &mut FighterCore, hit: &HitCapsule, facing: f32, params: &Parameters) {
    let damage = hit.descriptor.damage;
    let integer = if damage == 0.0 {
        0
    } else {
        let n = fctiwz(damage);
        if n == 0 {
            1
        } else {
            n
        }
    };
    if integer > owner.combat.clank.damage {
        owner.combat.clank.damage = integer;
        if hit.descriptor.rebound && owner.physics.ground_or_air == GroundOrAir::Ground {
            // 80076B14 / 80076C50, each fed the stopped owner's own damage.
            owner.combat.clank.duration = fmadds(
                integer as f32,
                params.duration_multiplier,
                params.duration_base,
            );
            owner.combat.clank.facing = facing;
        }
    }
}
/// ftColl_803C0C4C: the sounds of two blades meeting.
const SWORD_CLANK_SOUNDS: [u32; 3] = [107, 108, 109];
/// The sound of any other pair of attacks clanking.
const CLANK_SOUND: u32 = 0x6A;

/// ftColl_800784B4 (800784B4): the receiver plays the clank. Two slashing
/// hitboxes pick one of three sword sounds, drawing HSD_Randi(3) here, during
/// hit detection (retail 0x800784E4), before the sound is queued.
fn clank_sound(
    receiver: &mut FighterCore,
    incoming_hit: &HitCapsule,
    receiver_hit: &HitCapsule,
    rng: &mut gekko_math::HsdRng,
) {
    let id = if incoming_hit.descriptor.element == HitElement::Slash
        && receiver_hit.descriptor.element == HitElement::Slash
    {
        SWORD_CLANK_SOUNDS[rng.randi(SWORD_CLANK_SOUNDS.len() as i32) as usize]
    } else {
        CLANK_SOUND
    };
    receiver.shield_sound(id);
}

/// Called only for opponents after this receiver in the fighter list.
/// Returns true exactly when inlineA1 suppresses this incoming hitbox.
pub(super) fn contact(
    receiver: &mut FighterCore,
    incoming: &mut FighterCore,
    id: usize,
    mask: &mut [bool; 4],
    params: &Parameters,
    rng: &mut gekko_math::HsdRng,
) -> bool {
    if receiver.physics.ground_or_air != GroundOrAir::Ground
        || incoming.physics.ground_or_air != GroundOrAir::Ground
        || matches!(
            receiver.combat.grab,
            Some(super::grab::GrabLink::Holding { .. })
        )
        || matches!(incoming.combat.grab, Some(super::grab::GrabLink::Holding { victim, .. }) if victim == receiver.spawn_number)
    {
        return false;
    }
    let a = incoming.commands.hitboxes[id]
        .as_ref()
        .expect("incoming clank hit")
        .clone();
    if !a.descriptor.clank || a.descriptor.element == HitElement::Inert {
        return false;
    }
    let mut next_id = 0;
    while next_id < mask.len() {
        let m = next_id;
        next_id += 1;
        if !mask[m] {
            continue;
        }
        let b = receiver.commands.hitboxes[m]
            .as_ref()
            .expect("receiver clank hit")
            .clone();
        // lbColl_80007AFC reverses the nominal a/b order for the geometry call.
        let Some((receiver_point, incoming_point)) = hitbox_pair_contact(
            geometry(&b, receiver.player.scale),
            geometry(&a, incoming.player.scale),
        ) else {
            continue;
        };
        let midpoint = Vec3::new(
            0.5 * (incoming_point.x + receiver_point.x),
            0.5 * (incoming_point.y + receiver_point.y),
            0.5 * (incoming_point.z + receiver_point.z),
        );
        let priority = melee_coll::defense::clank_priority(
            a.descriptor.damage,
            b.descriptor.damage,
            params.priority_gap,
        );
        if priority.stop_second {
            record_group(
                receiver,
                b.descriptor.group,
                incoming.spawn_number,
                Some(&mut *mask),
            );
            let facing = if receiver.physics.position.x < incoming.physics.position.x {
                1.0
            } else {
                -1.0
            };
            accumulate(receiver, &b, facing, params);
            receiver
                .effects
                .push(melee_ef::request::EffectRequest::Clank { position: midpoint });
        }
        if priority.stop_first {
            record_group(incoming, a.descriptor.group, receiver.spawn_number, None);
            let facing = if receiver.physics.position.x > incoming.physics.position.x {
                1.0
            } else {
                -1.0
            };
            accumulate(incoming, &a, facing, params);
            receiver
                .effects
                .push(melee_ef::request::EffectRequest::Clank { position: midpoint });
            clank_sound(receiver, &a, &b, rng);
            return true;
        }
    }
    false
}

impl Fighter {
    pub(super) fn enter_rebound(&mut self, assets: &FighterAssets, pending: Pending) -> Result<()> {
        // It does not clear velocity.
        self.interrupt_actions();
        self.change_motion_state(S::ReboundStop.into(), assets)?;
        let animation_rate =
            (self.core.attributes.combat.clank_animation_length + 0.1) / pending.duration;
        let recoil = -pending.facing
            * fmadds(
                pending.duration,
                assets.clank.recoil_multiplier,
                assets.clank.recoil_base,
            );
        self.core.state_data = MotionData::Rebound(State {
            recoil,
            animation_rate,
        });
        let friction = crate::physics::grounded::floor_friction(&self.core.collision.data);
        // ftCommon_800804A0 writes +E8; the common update consumes it later.
        self.core.physics.secondary_ground_acceleration = if friction < 1.0 {
            recoil * friction
        } else {
            recoil
        };
        Ok(())
    }
}
pub fn stop_animation(
    fighter: &mut Fighter,
    phase: AnimationPhase<'_>,
) -> Result<Option<crate::anim::WaitChoice>> {
    fighter.step_animation(phase.assets);
    fighter.advance_smash_charge(phase.assets);
    let MotionData::Rebound(state) = &fighter.core.state_data else {
        panic!("rebound scratch missing")
    };
    let rate = state.animation_rate;
    fighter.change_motion_state_with_rate(S::Rebound.into(), phase.assets, 0.0, rate)?;
    Ok(None)
}
pub fn animation(
    fighter: &mut Fighter,
    phase: AnimationPhase<'_>,
) -> Result<Option<crate::anim::WaitChoice>> {
    fighter.step_animation(phase.assets);
    fighter.advance_smash_charge(phase.assets);
    if !fighter
        .core
        .animation
        .frames_remaining(&fighter.core.skeleton)
    {
        fighter.change_motion_state(S::Wait.into(), phase.assets)?;
    }
    Ok(None)
}
pub fn physics(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    let core = &mut fighter.core;
    let MotionData::Rebound(state) = &mut core.state_data else {
        panic!("rebound scratch missing")
    };
    let params = crate::physics::grounded::GroundedParameters::from_attributes(
        &core.attributes,
        &phase.assets.common,
    );
    if state.recoil != 0.0 {
        state.recoil = 0.0;
    } else {
        crate::physics::grounded::friction_physics(
            &mut core.physics,
            &params,
            core.collision.data.floor.normal,
            phase.map.floor_speed_scale(&core.collision.data),
        );
    }
    crate::physics::grounded::finish_ground_update(
        &mut core.physics,
        &core.collision.data,
        &params,
        phase.map,
        phase.wind,
    );
}
/// ReboundStop has no retail Phys callback; preserve the scheduler's common tail.
pub fn stop_physics(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    let core = &mut fighter.core;
    let params = crate::physics::grounded::GroundedParameters::from_attributes(
        &core.attributes,
        &phase.assets.common,
    );
    crate::physics::grounded::finish_ground_update(
        &mut core.physics,
        &core.collision.data,
        &params,
        phase.map,
        phase.wind,
    );
}

/// ReboundStop has no Coll or Cam callback in ftData_MotionStateList.
pub fn stop_collision(
    _fighter: &mut Fighter,
    _phase: super::state::CollisionPhase<'_>,
) -> Result<()> {
    Ok(())
}
pub fn stop_camera(_fighter: &mut Fighter, _phase: super::state::CameraPhase<'_>) {}

/// ftColl_8007925C's per-item clank mask (ftColl_804D6560): this fighter's
/// hitboxes that may clank with `item`, in hit-ID order. Returns the mask and
/// whether any entry is set. (x221B_b5 and x43_b2 belong to states and
/// commands the port does not reach.)
pub(super) fn item_candidates(
    fighter: &FighterCore,
    item: &melee_it::ItemCore,
) -> ([bool; 4], bool) {
    let victim = item.hitbox_victim();
    let grounded = item.ground_or_air == GroundOrAir::Ground;
    let mut mask = [false; 4];
    for (slot, hit) in mask.iter_mut().zip(&fighter.commands.hitboxes) {
        let Some(hit) = hit else {
            continue;
        };
        let desc = &hit.descriptor;
        // ftcoll.c:2066: x42_b5 gates this mask too.
        *slot = hit.hits_fighters
            && desc.element != HitElement::Catch
            && (if grounded {
                desc.hit_ground
            } else {
                desc.hit_air
            })
            && !hit.victims.contains(&victim);
    }
    let any = mask.contains(&true);
    (mask, any)
}

/// ftColl_8007925C's clank step for item hitbox `id`, then ftColl_80077970
/// (0x80077970): the first overlapping candidate clanks and ends this item
/// hitbox's tests. Returns true when it clanked.
pub(super) fn item_contact(
    fighter: &mut FighterCore,
    item: &mut melee_it::ItemCore,
    id: usize,
    mask: &mut [bool; 4],
    params: &Parameters,
) -> bool {
    let a = item.hitboxes[id].as_ref().expect("item clank hit").clone();
    for m in 0..mask.len() {
        if !mask[m] {
            continue;
        }
        let b = fighter.commands.hitboxes[m]
            .as_ref()
            .expect("fighter clank hit")
            .clone();
        let inert = HitElement::Inert;
        if a.descriptor.element == inert || b.descriptor.element == inert {
            if a.descriptor.element == b.descriptor.element {
                continue;
            }
            // ftcoll.c:2200-2213: a touch marks the item (xDCE b6, toucher)
            // and ends this item hitbox's tests. Only the Motion-Sensor Bomb
            // and a castle stage object install the `touched` callback that
            // reads the mark (item.c:1836), and neither is ported.
            if hitbox_pair_contact(geometry(&b, fighter.player.scale), geometry(&a, item.scale))
                .is_some()
            {
                return true;
            }
            continue;
        }
        if !a.descriptor.clank || !b.descriptor.clank {
            continue;
        }
        // lbColl_80007AFC(item hit, fighter hit): the fighter capsule first.
        let Some((fighter_point, item_point)) =
            hitbox_pair_contact(geometry(&b, fighter.player.scale), geometry(&a, item.scale))
        else {
            continue;
        };
        let midpoint = Vec3::new(
            (item_point.x + fighter_point.x) * 0.5,
            (item_point.y + fighter_point.y) * 0.5,
            (item_point.z + fighter_point.z) * 0.5,
        );
        let priority = melee_coll::defense::clank_priority(
            a.descriptor.damage,
            b.descriptor.damage,
            params.priority_gap,
        );
        if priority.stop_second {
            // inlineItemA0: the fighter's hit group remembers the item.
            record_group(
                fighter,
                b.descriptor.group,
                item.hitbox_victim(),
                Some(&mut *mask),
            );
            let facing = if fighter.physics.position.x < item.position.x {
                1.0
            } else {
                -1.0
            };
            accumulate(fighter, &b, facing, params);
            fighter
                .effects
                .push(melee_ef::request::EffectRequest::Clank { position: midpoint });
        }
        if priority.stop_first {
            // inlineItemA1: it_8026FAC4 records the fighter, then xC48 keeps
            // the strongest clank. (xCF4 and the knockback direction xCB8
            // have no reader among the supported items.)
            item.record_clank_victim(id, a.descriptor.group, fighter.spawn_number);
            let damage = truncated_damage(a.descriptor.damage);
            if damage > item.pending_clank_damage {
                item.pending_clank_damage = damage;
            }
            fighter
                .effects
                .push(melee_ef::request::EffectRequest::Clank { position: midpoint });
        }
        return true;
    }
    false
}

/// ftcoll.c's integer damage: zero stays zero, a fraction becomes one.
fn truncated_damage(damage: f32) -> i32 {
    if damage == 0.0 {
        0
    } else {
        let n = fctiwz(damage);
        if n == 0 {
            1
        } else {
            n
        }
    }
}
