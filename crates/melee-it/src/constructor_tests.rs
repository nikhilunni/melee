//! Constructor context cannot be omitted and never evaluates for ordinary kinds.
use super::*;
struct Ordinary;
impl ItemLogic for Ordinary {
    const KIND: ItemKind = ItemKind::Capsule;
    const STATES: &'static [ItemStateRow] = &[];
    fn spawned(item: &mut ItemCore, _: &ItemAssets) {
        assert!(item.spin_speed != 0.0, "spin already initialized");
        assert!(
            item.spawn_intangible_frames.is_none(),
            "kind callback precedes common intangible tail"
        );
        item.command_variables[0] = 17;
    }
}
struct Contextual;
fn before(item: &mut ItemCore, _: &ItemAssets) {
    assert!(
        item.collision.is_some(),
        "initial collision precedes constructor"
    );
    assert!(item.spin_speed != 0.0);
    assert_eq!(item.position, item.root_translation);
    item.position.x = 23.0;
    item.command_variables[0] = 31;
}
fn after(item: &mut ItemCore, _: &ItemAssets) {
    assert_eq!(item.command_variables[0], 31);
    assert_eq!(item.facing, -1.0);
    assert!(item.spawn_intangible_frames.is_none());
    item.command_variables[0] = 47;
}
impl ItemLogic for Contextual {
    const KIND: ItemKind = ItemKind::Capsule;
    const STATES: &'static [ItemStateRow] = &[];
    const FACING_CONSTRUCTOR: Option<FacingConstructor> = Some(FacingConstructor {
        before_query: before,
        after_query: after,
    });
    fn spawned(_: &mut ItemCore, _: &ItemAssets) {
        panic!("context may not fall back to ordinary callback")
    }
}
item_kinds! { enum OrdinaryKinds { Entry: Ordinary } }
item_kinds! { enum ContextKinds { Entry: Contextual } }
fn common() -> ItemCommonData {
    ItemCommonData {
        random_items: Default::default(),
        hold_limits: [None; 13],
        lifetime: 1.0,
        spawn_intangible_frames: 0.0,
        half_life_scale: 0.5,
        speed_damage_scale: 0.0,
        speed_damage_base: 0.0,
        shield_bounce_degrees: 0.0,
        maximum_reflected_damage: 999,
        clank_priority_gap: 9,
        hitlag_scale: 0.0,
        hitlag_base: 0.0,
        explosion_lifetime: 0.0,
        spawn_spin_degrees: 0.0,
        fall_spin_degrees: 0.0,
        knockback: Default::default(),
        launch: Default::default(),
        victim_bounce: Default::default(),
        camera_extents: [0.0; 4],
        push_speed: 0.0,
        release_floor_depth: 0.0,
        slide_rule: crate::desc::SlideRule::NEVER,
    }
}
#[test]
fn required_context_refuses_before_allocation_and_preserves_order() {
    let _ = (OrdinaryKinds::Entry, ContextKinds::Entry);
    let mut data = common();
    data.spawn_spin_degrees = 10.0;
    let mut pool = ItemPool::new(data);
    let mut assets = tests::assets();
    assets.spin_rate = 100.0;
    let mut spawn = SpawnItem::held(ItemKind::Capsule, 0, Vec3::new(0.0, 20.0, 0.0), 1.0);
    spawn.hold_kind = 0;
    let mut gobjs = hsd_gobj::GObjAddresses::default();
    let original = pool.next_id;
    let refused = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        pool.spawn::<ContextKinds>(spawn, &assets, &mut gobjs);
    }));
    assert!(refused.is_err());
    assert_eq!(pool.len(), 0);
    assert_eq!(pool.next_id, original);
    let ordinary = pool
        .spawn::<OrdinaryKinds>(spawn, &assets, &mut gobjs)
        .unwrap();
    assert_eq!(pool.get_mut(ordinary).unwrap().command_variables[0], 17);
    let mut b = melee_mp::CollMapBuilder::new();
    let a = b.vertex(0, -10.0, 0.0);
    let z = b.vertex(0, 10.0, 0.0);
    b.floor(0, a, z, 0);
    let mut map = melee_mp::CollMap::load(b.build(), 1.0, melee_types::GrKind::Last);
    let calls = std::cell::Cell::new(0);
    let contextual = pool
        .spawn_with_facing_constructor::<ContextKinds>(
            spawn,
            &assets,
            &mut gobjs,
            &mut map,
            |position| {
                calls.set(calls.get() + 1);
                assert_eq!(position.x, 23.0);
                -1.0
            },
        )
        .unwrap();
    assert_eq!(calls.get(), 1);
    let item = pool.get_mut(contextual).unwrap();
    assert_eq!(item.command_variables[0], 47);
    assert_eq!(item.spawn_intangible_frames, Some(0.0));
}
