use hsd_types::{Mtx, Vec3};
use melee_coll::{
    detection::{self, PairCursor, PositionedCollider},
    hitbox::{self, CapsulePhase, HitCapsule},
    hurtbox::{HurtCapsule, HurtHeight},
};
use melee_types::{combat::HitboxDescriptor, GroundOrAir, HitElement};
fn descriptor(group: u8) -> HitboxDescriptor {
    HitboxDescriptor {
        group,
        bone: 0,
        common_bone: false,
        requires_throw_owner: false,
        damage: 5.0,
        shield_damage: 0,
        sound_severity: 0,
        radius: 1.0,
        offset: Vec3::ZERO,
        angle: 45,
        growth: 100,
        weight_knockback: 0,
        base_knockback: 0,
        element: HitElement::Normal,
        hit_ground: true,
        hit_air: true,
        ignore_scale: false,
        clank: true,
        rebound: true,
    }
}
#[test]
fn group_history_is_inherited_and_same_group_replacement_preserves_sweep() {
    let mut boxes: [Option<HitCapsule>; 4] = Default::default();
    hitbox::spawn(&mut boxes, 0, &descriptor(1));
    boxes[0].as_mut().unwrap().update_position(Vec3::ZERO);
    detection::record_victim(&mut boxes, 1, 42);
    hitbox::spawn(&mut boxes, 1, &descriptor(1));
    assert!(boxes[1].as_ref().unwrap().victims.contains(&42));
    hitbox::spawn(&mut boxes, 0, &descriptor(1));
    assert_eq!(
        boxes[0].as_ref().unwrap().phase,
        CapsulePhase::FirstPosition
    );
    hitbox::spawn(&mut boxes, 0, &descriptor(2));
    assert_eq!(boxes[0].as_ref().unwrap().phase, CapsulePhase::Enabled);
    assert!(boxes[0].as_ref().unwrap().victims.is_empty());
}
#[test]
fn pair_cursor_rechecks_history_after_each_applied_contact() {
    let mut boxes: [Option<HitCapsule>; 4] = Default::default();
    for id in 0..3 {
        hitbox::spawn(&mut boxes, id, &descriptor(if id == 2 { 2 } else { 1 }));
    }
    let mut cursor = PairCursor::default();
    assert_eq!(cursor.next(&boxes, 7, GroundOrAir::Ground), Some(0));
    detection::record_victim(&mut boxes, 1, 7);
    assert_eq!(cursor.next(&boxes, 7, GroundOrAir::Ground), Some(2));
    assert_eq!(cursor.next(&boxes, 7, GroundOrAir::Ground), None);
}
#[test]
fn positioned_item_geometry_uses_same_first_hurt_contact_order() {
    let mut boxes: [Option<HitCapsule>; 4] = Default::default();
    hitbox::spawn(&mut boxes, 0, &descriptor(0));
    let hit = boxes[0].as_mut().unwrap();
    hit.update_position(Vec3::ZERO);
    let hurt = HurtCapsule {
        grabbable: false,
        height: HurtHeight::Low,
        bone: 0,
        offsets: [Vec3::ZERO; 2],
        radius: 1.0,
        positions: [Vec3::ZERO; 2],
        cached: true,
    };
    let hurts = [
        hurt.clone(),
        HurtCapsule {
            height: HurtHeight::High,
            grabbable: true,
            ..hurt
        },
    ];
    let matrices = [Mtx::IDENTITY; 2];
    let mut collider = PositionedCollider {
        capsules: &hurts,
        matrices: &matrices,
        scale: 1.0,
    };
    assert!(matches!(
        detection::first_contact(&mut collider, hit, 1.0),
        Some((_, HurtHeight::Low))
    ));
    hit.descriptor.element = HitElement::Catch;
    assert!(matches!(
        detection::first_contact(&mut collider, hit, 1.0),
        Some((_, HurtHeight::High))
    ));
}
#[test]
fn clank_priority_uses_truncated_damage_and_strict_threshold() {
    use melee_coll::defense::{clank_priority, ClankPriority};
    assert_eq!(
        clank_priority(10.9, 1.9, 9),
        ClankPriority {
            stop_first: false,
            stop_second: true
        }
    );
    assert_eq!(
        clank_priority(9.9, 1.9, 9),
        ClankPriority {
            stop_first: true,
            stop_second: true
        }
    );
    assert_eq!(
        clank_priority(1.9, 10.9, 9),
        ClankPriority {
            stop_first: true,
            stop_second: false
        }
    );
}
#[test]
fn hitlag_reports_only_expiry_and_preserves_zero_sign_when_inactive() {
    let mut remaining = 1.0;
    assert!(melee_coll::damage::tick_hitlag(&mut remaining));
    assert!(!melee_coll::damage::tick_hitlag(&mut remaining));
    remaining = -0.0;
    assert!(!melee_coll::damage::tick_hitlag(&mut remaining));
    assert_eq!(remaining.to_bits(), (-0.0f32).to_bits());
    assert_eq!(melee_coll::damage::hitlag(5, 0.5, 3.0, 30.0), 5.0);
}
