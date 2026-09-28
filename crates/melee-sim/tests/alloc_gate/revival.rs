//! Repeated revivals exercise owner retention beyond the one-stock oracle scene.
use gekko_math::rng::HsdRng;
use hsd_types::Vec3;
use melee_ft::fighter::{CharacterCallbacks, Fighter, PlayerSlot, SpawnContext, SpawnCounter};
use melee_sim::{assets::Assets, scene_stage::FINAL_DESTINATION};
use melee_types::{snapshot::Snapshot, PlayerKind};
use std::{cell::Cell, path::Path};

#[test]
fn repeated_revival_retains_owners_without_allocating() {
    let scenario = melee_sim::scenario::Scenario::load(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/scenarios/ko_fd_marth.toml"),
    )
    .unwrap();
    if !melee_test_support::require_files(scenario.required_files()) {
        return;
    }
    let assets = Assets::load(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files"),
        [
            ft_fox::init::Fox::descriptor(),
            ft_mars::init::Marth::descriptor(),
        ],
        &FINAL_DESTINATION,
    )
    .unwrap();
    check::<ft_fox::init::Fox>(&assets, 0);
    check::<ft_mars::init::Marth>(&assets, 1);
}

fn check<C: CharacterCallbacks>(assets: &Assets, slot: usize) {
    let archive = &assets.characters[slot];
    let resources = &assets.fighters()[slot];
    let mut map = melee_gr::desc::load_collision(&assets.stage, &assets.stage_desc).unwrap();
    let player = PlayerSlot {
        id: slot as u8,
        control: PlayerKind::Human,
        costume: 0,
        stocks: 4,
        position: Vec3::new(0.0, 100.0, 0.0),
        facing: 1.0,
        scale: 1.0,
        damage: 0.0,
        cpu_mode: 4,
        cpu_level: 1,
    };
    let mut character = C::from_archive(&archive.data).unwrap();
    let costume_bytes = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../harness/roms/files")
            .join(archive.descriptor.costumes[0].file),
    )
    .unwrap();
    let costume = &hsd_archive::Archive::parse(&costume_bytes).unwrap();
    character.on_costume_loaded(costume, 0).unwrap();
    let descriptor =
        hsd_archive::desc::read_public_jobj(costume, archive.descriptor.costumes[0].joint_symbol)
            .unwrap();
    let (tree, root) = hsd_anim::load::load_joint_tree(costume, &descriptor).unwrap();
    let mut fighter = Fighter::prepare(player, character.into_state(), resources, tree, root, &map);
    let mut counter = SpawnCounter(1);
    let owners = owner_addresses(&fighter);
    let mut expected = None;
    for life in 1..=3 {
        // Dirty the clocks, body state and platform between stock resets.
        fighter.physics.percent = 123.0;
        fighter.physics.self_velocity = Vec3::new(1.0, 2.0, 3.0);
        fighter.status.ledge_cooldown = 30;
        for _ in 0..40 {
            fighter.update_revival_platform();
        }
        let mut rng = HsdRng::new(0x12345678);
        super::ALLOCATIONS.with(|count| count.set(0));
        super::COUNTING.with(|enabled| enabled.set(true));
        let result = fighter.reset_for_revival(
            resources,
            &assets.arena,
            &mut Default::default(),
            SpawnContext {
                map: &mut map,
                stage_camera: &assets.stage_camera,
                rng: &mut rng,
                counter: &mut counter,
            },
        );
        fighter.update_revival_platform();
        super::COUNTING.with(|enabled| enabled.set(false));
        result.unwrap();
        assert_eq!(super::ALLOCATIONS.with(Cell::get), 0, "life {life}");
        assert_eq!(owner_addresses(&fighter), owners, "life {life}");
        assert_eq!(fighter.spawn_number, life);
        assert_eq!(rng.seed, 0x5B3F58B2, "exactly two reset-time CPU draws");
        assert_eq!(fighter.player.stocks, 4);
        let mut actual = Vec::new();
        fighter.snapshot(&mut actual);
        let platform = &fighter.revival_platform;
        let pose = platform.tree.get(platform.root).clone();
        if let Some((expected_snapshot, expected_pose)) = &expected {
            assert_eq!(&actual, expected_snapshot, "life {life}");
            assert_eq!(&pose, expected_pose, "platform restarted at life {life}");
        } else {
            expected = Some((actual, pose));
        }
    }
}

fn owner_addresses(fighter: &Fighter) -> [usize; 8] {
    [
        fighter.skeleton.get(fighter.animation.root) as *const _ as usize,
        fighter.animation.blend_tree.get(fighter.animation.root) as *const _ as usize,
        fighter.animation.parts.as_ptr() as usize,
        fighter.hurtboxes.as_ptr() as usize,
        fighter.dynamic_colliders.as_ptr() as usize,
        fighter.dynamics.as_ptr() as usize,
        fighter.dynamics_first_bone.as_ptr() as usize,
        fighter
            .revival_platform
            .tree
            .get(fighter.revival_platform.root) as *const _ as usize,
    ]
}
