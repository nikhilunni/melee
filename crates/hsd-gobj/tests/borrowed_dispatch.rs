use hsd_gobj::{World, WorldConfig};
#[test]
fn borrowed_dispatch_preserves_in_frame_creation_and_deferred_destruction() {
    let mut world = World::new(WorldConfig::MELEE);
    let first = world.create(0, 4, 0);
    let second = world.create(0, 4, 0);
    world.add_tagged_proc(first, 1, 10);
    world.add_tagged_proc(second, 1, 20);
    let mut calls = Vec::new();
    world.run_procs_with(|world, object, tag| {
        calls.push(tag);
        if tag == 10 {
            world.destroy(object);
            let added = world.create(0, 4, 0);
            world.add_tagged_proc(added, 2, 30);
        }
    });
    assert_eq!(calls, [10, 20, 30]);
    assert!(!world.contains(first));
    calls.clear();
    world.run_procs_with(|_, _, tag| calls.push(tag));
    assert_eq!(calls, [20, 30]);
}
