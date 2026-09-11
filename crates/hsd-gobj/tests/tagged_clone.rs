use hsd_gobj::{TaggedWorld, WorldConfig};

#[test]
fn tagged_clone_preserves_order_generations_and_continued_mutation() {
    fn assert_send<T: Send>() {}
    assert_send::<TaggedWorld>();
    let mut original = TaggedWorld::new(WorldConfig::MELEE);
    let dead = original.create(0, 4, 0);
    let first = original.create(0, 4, 0);
    original.add_tagged_proc(first, 1, 10);
    original.destroy(dead);
    let mut cloned = original.clone();
    let run = |world: &mut TaggedWorld| {
        let reused = world.create(0, 4, 0);
        assert_ne!(reused, dead);
        assert_eq!(reused.index(), dead.index());
        world.add_tagged_proc(reused, 1, 20);
        let mut calls = Vec::new();
        world.run_procs_with(|world, id, tag| {
            calls.push((id, tag));
            if tag == 10 {
                world.destroy(id);
                let added = world.create(0, 4, 0);
                world.add_tagged_proc(added, 2, 30);
            }
        });
        calls
    };
    let expected = run(&mut original);
    assert!(cloned.contains(first));
    assert_eq!(run(&mut cloned), expected);
    let mut replaced = TaggedWorld::new(WorldConfig::MELEE);
    replaced.clone_from(&cloned);
    let tags = |world: &mut TaggedWorld| {
        let mut tags = Vec::new();
        world.run_procs_with(|_, id, tag| tags.push((id, tag)));
        tags
    };
    assert_eq!(tags(&mut replaced), tags(&mut original));
}
