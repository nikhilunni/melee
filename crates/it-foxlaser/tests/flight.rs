//! Behavior gates derived from the retail item callbacks and disc attributes.
use hsd_archive::Archive;
use hsd_types::Vec3;
use it_foxlaser::{initialize_laser, FoxBlaster, FoxLaser};
use melee_it::{
    desc::{ItemAssets, ItemCommonData},
    *,
};
use melee_types::ItemKind;

item_kinds! { enum Items {Laser:FoxLaser,Blaster:FoxBlaster} }
fn archives() -> Option<(Archive, Archive)> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files");
    let paths = [root.join("PlFx.dat"), root.join("ItCo.dat")];
    if !melee_test_support::require_files(paths.clone()) {
        return None;
    }
    Some((
        Archive::parse(&std::fs::read(&paths[0]).unwrap()).unwrap(),
        Archive::parse(&std::fs::read(&paths[1]).unwrap()).unwrap(),
    ))
}
#[test]
fn laser_damage_callback_waits_for_item_event_phase() {
    let Some((fox, common)) = archives() else {
        return;
    };
    let data = ItemCommonData::read(&common, common.public("itPublicData").unwrap()).unwrap();
    let mut pool = ItemPool::new(data);
    let assets = ItemAssets::from_fighter(&fox, fox.public("ftDataFox").unwrap(), 0, 2).unwrap();
    let id = pool
        .spawn::<Items>(
            SpawnItem::ray(ItemKind::FoxLaser, 0, Vec3::ZERO, 1.0),
            &assets,
        )
        .unwrap();
    pool.record_damage_dealt(id, 3.0);
    pool.record_damage_dealt(id, 2.0);
    assert!(!pool.get_mut(id).unwrap().destroyed);
    assert_eq!(pool.get_mut(id).unwrap().pending_damage_dealt, 3);
    pool.process_events::<Items>(id, &assets);
    assert!(pool.get_mut(id).unwrap().destroyed);
    assert_eq!(pool.get_mut(id).unwrap().pending_damage_dealt, 0);
    pool.remove_destroyed::<Items>();
    assert!(pool.is_empty());
}
#[test]
fn laser_moves_expires_and_keeps_spawn_order_after_removal() {
    let Some((fox, common)) = archives() else {
        return;
    };
    let data = ItemCommonData::read(&common, common.public("itPublicData").unwrap()).unwrap();
    assert_eq!(
        data.hold_limits[8], None,
        "retail character category is unbounded"
    );
    let mut pool = ItemPool::new(data);
    // A one-floor map far away: the lasers' animation never touches it.
    let mut map = {
        let mut builder = melee_mp::CollMapBuilder::new();
        let left = builder.vertex(0, -1000.0, -1000.0);
        let right = builder.vertex(0, -990.0, -1000.0);
        builder.floor(0, left, right, 0);
        melee_mp::CollMap::load(builder.build(), 1.0, melee_types::GrKind::Last)
    };
    let root = fox.public("ftDataFox").unwrap();
    let laser = ItemAssets::from_fighter(&fox, root, 0, 2).unwrap();
    let blaster = ItemAssets::from_fighter(&fox, root, 1, 9).unwrap();
    let gun = pool
        .spawn::<Items>(
            SpawnItem::held(ItemKind::FoxBlaster, 0, Vec3::ZERO, 1.0),
            &blaster,
        )
        .unwrap();
    let ray = pool
        .spawn::<Items>(
            SpawnItem::ray(ItemKind::FoxLaser, 0, Vec3::ZERO, 1.0),
            &laser,
        )
        .unwrap();
    initialize_laser(pool.get_mut(ray).unwrap(), &laser, 0.0, 7.0, 0);
    assert_eq!(pool.iter().map(|i| i.id).collect::<Vec<_>>(), [gun, ray]);
    pool.animate::<Items>(ray, &laser, None, None, &mut map);
    pool.physics::<Items>(
        ray,
        None,
        &melee_it::ItemBounds {
            left: -246.0,
            right: 246.0,
            bottom: -140.0,
            top: 188.0,
            camera_offset: hsd_types::Vec2::ZERO,
        },
        &laser,
        &std::cell::Cell::new(gekko_math::HsdRng::default()),
    );
    let item = pool.get_mut(ray).unwrap();
    assert_eq!(item.position.x.to_bits(), 7.0f32.to_bits());
    assert_eq!(
        item.life_timer.to_bits(),
        (laser.special_attributes[0] - 1.0).to_bits()
    );
    assert!(
        item.hitboxes[3].is_none(),
        "script clears tip capsule after one frame"
    );
    pool.retire(gun);
    pool.remove_destroyed::<Items>();
    assert_eq!(pool.iter().map(|i| i.id).collect::<Vec<_>>(), [ray]);
    for _ in 1..laser.special_attributes[0] as usize {
        pool.animate::<Items>(ray, &laser, None, None, &mut map);
    }
    pool.remove_destroyed::<Items>();
    assert!(pool.is_empty());
}

#[test]
fn blaster_sounds_follow_open_latch_and_visibility_transitions() {
    let Some((fox, common)) = archives() else {
        return;
    };
    let data = ItemCommonData::read(&common, common.public("itPublicData").unwrap()).unwrap();
    let mut pool = ItemPool::new(data);
    let assets = ItemAssets::from_fighter(&fox, fox.public("ftDataFox").unwrap(), 1, 9).unwrap();
    let id = pool
        .spawn::<Items>(
            SpawnItem::held(ItemKind::FoxBlaster, 0, Vec3::ZERO, 1.0),
            &assets,
        )
        .unwrap();
    for control in [
        ItemControl::Open,
        ItemControl::Open,
        ItemControl::Close,
        ItemControl::Open,
        ItemControl::Visibility(2),
        ItemControl::Visibility(2),
        ItemControl::Visibility(1),
        ItemControl::Visibility(2),
    ] {
        pool.control::<Items>(0, ItemKind::FoxBlaster, control, &assets);
    }
    assert_eq!(
        pool.get_mut(id)
            .unwrap()
            .sound_requests
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        [0x1AE05, 0x1AE14, 0x1AE14]
    );
}
