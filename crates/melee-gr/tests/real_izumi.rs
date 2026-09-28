use hsd_archive::Archive;
use melee_gr::desc;
use std::path::PathBuf;

#[test]
fn fountain_of_dreams_platform_parameters_and_models_come_from_the_archive() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files/GrIz.dat");
    if !melee_test_support::require_files([&path]) {
        return;
    }
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) => panic!("{}: {e}", path.display()),
    };
    let archive = Archive::parse(&bytes).unwrap();
    let stage = desc::read_izumi(&archive).unwrap();
    assert_eq!(stage.kind, melee_types::GrKind::Izumi);
    assert_eq!(stage.parameters.map_scale.to_bits(), 0.75_f32.to_bits());
    assert_eq!(stage.models.len(), 5);
    // grIzumi_801CBE64 binds collision through grIz_803E0D60, not the archive.
    assert!(stage.models.iter().all(|m| m.joint_mappings.is_empty()));
    // The platform model's animation plays once.
    assert_eq!(stage.models[4].animation_loops, [false]);
    let music = desc::read_music(&archive, 2).unwrap();
    assert_eq!((music.primary, music.alternate), (49, -1));
    let p = desc::read_izumi_parameters(&archive).unwrap();
    assert_eq!(p.initial_heights, [20.0, 28.0]);
    assert_eq!(p.rest_height, 25.0);
    assert_eq!(p.step, [5.0, 10.0]);
    assert_eq!([p.highest_target, p.lowest_target], [35.0, 15.0]);
    assert_eq!(p.rise_speed.to_bits(), 0.15_f32.to_bits());
    assert_eq!(p.sink_speed.to_bits(), 0.1_f32.to_bits());
    assert_eq!(
        [p.sink_chance_below_rest, p.rise_chance_above_rest],
        [0.25, 0.25]
    );
    assert_eq!(p.wait_frames, [1080.0, 600.0]);
    assert_eq!(
        [p.submerge_weight, p.stay_weight, p.step_weight],
        [4.0, 10.0, 8.0]
    );
    assert_eq!(p.submerged_frames, [1680.0, 480.0]);
}
