use hsd_archive::Archive;
use melee_gr::desc;
use std::path::PathBuf;

#[test]
fn story_platforms_and_randall_binding_come_from_the_archive() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files/GrSt.dat");
    if !melee_test_support::require_files([&path]) {
        return;
    }
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) => panic!("{}: {e}", path.display()),
    };
    let archive = Archive::parse(&bytes).unwrap();
    let music = desc::read_music(&archive, 8).unwrap();
    assert_eq!(music.rule, melee_gr::music::MusicRule::Primary);
    let desc = desc::read_story(&archive).unwrap();
    assert_eq!(desc.models.len(), 4);
    assert_eq!(desc.parameters.map_scale.to_bits(), 0.7_f32.to_bits());
    let bindings = &desc.models[2].joint_mappings;
    assert_eq!(bindings.len(), 1);
    assert_eq!((bindings[0].joint_index, bindings[0].extra), (0, 1));
    let mut map = desc::load_collision(&archive, &desc).unwrap();
    for (x, line) in [(-42.0, 1), (42.0, 5)] {
        let hit = map
            .check_floor(x, 30.0, x, 0.0, 0.0, -1, -1, -1, None)
            .unwrap();
        assert_eq!(hit.line_id, line);
        assert_ne!(hit.flags & melee_types::mp::line_flag::PLATFORM, 0);
        assert_eq!(hit.pos.y.to_bits(), (33.5_f32 * 0.7).to_bits());
    }
    let parameters = desc::read_story_parameters(&archive).unwrap();
    assert_eq!(
        (
            parameters.timer_minimum,
            parameters.timer_range,
            parameters.group_rarity
        ),
        (600.0, 1800, 8)
    );
    assert_eq!(parameters.heights, [30.0, 45.0, 60.0, 75.0, 90.0, 0.0]);
}
