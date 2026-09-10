use hsd_archive::Archive;
use melee_gr::desc;
use std::path::PathBuf;

#[test]
fn dream_land_collision_wind_and_lights_come_from_the_archive() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files/GrOp.dat");
    if !melee_test_support::require_files([&path]) {
        return;
    }
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) => panic!("{}: {e}", path.display()),
    };
    let archive = Archive::parse(&bytes).unwrap();
    let stage = desc::read_pupupu(&archive).unwrap();
    assert_eq!(stage.kind, melee_types::GrKind::OldPupupu);
    assert_eq!(stage.parameters.map_scale.to_bits(), 1.0_f32.to_bits());
    assert_eq!(stage.section_counts, [1, 8, 0, 38, 10, 8]);
    assert!(stage
        .models
        .iter()
        .all(|model| model.joint_mappings.is_empty()));
    let music = desc::read_music(&archive, 28).unwrap();
    assert_eq!(music.rule, melee_gr::music::MusicRule::Primary);
    assert_eq!((music.primary, music.alternate), (58, -1));
    let parameters = desc::read_pupupu_parameters(&archive).unwrap();
    assert_eq!(parameters.wind_delay, [600, 1200]);
    assert_eq!(parameters.background_delay, [3000, 4000]);
    assert_eq!(parameters.blink_delay, [180, 360]);
    assert_eq!(parameters.wind_speed.to_bits(), 0.2_f32.to_bits());
    let lights = melee_gr::battle::lights::load_model(&archive, &stage, 5).unwrap();
    assert_eq!(lights.len(), 2);
    assert!(matches!(
        lights[0],
        melee_gr::battle::lights::Light::Ambient {
            color: [255, 255, 255, 255]
        }
    ));
    let melee_gr::battle::lights::Light::Point {
        position,
        attenuation,
        ..
    } = &lights[1]
    else {
        panic!("Dream Land point light");
    };
    assert_eq!(position.y.to_bits(), 5.0_f32.to_bits());
    assert_eq!(position.z.to_bits(), 17.0_f32.to_bits());
    assert!(attenuation.reference_distance.is_finite());
    let mut map = desc::load_collision(&archive, &stage).unwrap();
    for (x, line, y, platform) in [
        (0.0, 4, 0.0088_f32, false),
        (-46.6, 0, 30.1421, true),
        (46.6, 1, 30.2425, true),
    ] {
        let hit = map
            .check_floor(x, 40.0, x, 0.0, 0.0, -1, -1, -1, None)
            .unwrap();
        assert_eq!(hit.line_id, line);
        assert_eq!(hit.pos.y.to_bits(), y.to_bits());
        assert_eq!(
            hit.flags & melee_types::mp::line_flag::PLATFORM != 0,
            platform
        );
    }
}
