//! The N64 stages' parameters and models come from their archives.
use hsd_archive::Archive;
use melee_gr::desc;
use std::path::PathBuf;

fn archive(name: &str) -> Option<Vec<u8>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../harness/roms/files")
        .join(name);
    if !melee_test_support::require_files([&path]) {
        return None;
    }
    Some(std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display())))
}

#[test]
fn yoshis_island_n64_clouds_and_guest_come_from_the_archive() {
    let Some(bytes) = archive("GrOy.dat") else {
        return;
    };
    let archive = Archive::parse(&bytes).unwrap();
    let stage = desc::read_old_yoshi(&archive).unwrap();
    assert_eq!(stage.kind, melee_types::GrKind::OldYoshi);
    assert_eq!(stage.models.len(), 6);
    // Map 2's three clouds carry collision joints 0..2 on descendants 1, 5, 9
    // (grOy_803E6574): the loop, the fade and the return.
    let bones: Vec<_> = stage.models[2]
        .joint_mappings
        .iter()
        .map(|m| (m.joint_index, m.extra))
        .collect();
    assert_eq!(bones, [(0, 1), (1, 5), (2, 9)]);
    assert_eq!(stage.models[2].animation_loops, [true, false, false]);
    let p = desc::read_old_yoshi_parameters(&archive).unwrap();
    assert_eq!((p.vanish_after, p.return_after), (120, 180));
    assert_eq!(p.maximum_speed.to_bits(), 0.15_f32.to_bits());
    assert_eq!(p.acceleration.to_bits(), 0.15_f32.to_bits());
    assert_eq!(p.sink_depth, 6.0);
    assert_eq!((p.disable_frame, p.enable_frame), (30, 30));
    assert_eq!(p.guest_wait, [4000, 3000]);
    assert_eq!(p.guest_height, 30);
    let music = desc::read_music(&archive, 0x1D).unwrap();
    assert_eq!(music.rule, melee_gr::music::MusicRule::Primary);
}

#[test]
fn kongo_jungle_n64_barrel_and_bird_come_from_the_archive() {
    let Some(bytes) = archive("GrOk.dat") else {
        return;
    };
    let archive = Archive::parse(&bytes).unwrap();
    let stage = desc::read_old_kongo(&archive).unwrap();
    assert_eq!(stage.kind, melee_types::GrKind::OldKongo);
    assert_eq!(stage.models.len(), 4);
    // grOk_803E6580 binds the platforms' joints; the archive binds none.
    assert!(stage.models.iter().all(|m| m.joint_mappings.is_empty()));
    assert_eq!(stage.models[1].animation_loops, [true, false, false]);
    // No particle bank: Assets falls back to an empty one.
    assert!(archive.public("map_ptcl").is_none());
    let p = desc::read_old_kongo_parameters(&archive).unwrap();
    assert_eq!(p.bird_wait, [3000, 4000]);
    assert_eq!(p.shot_delay, [479.0, 480.0]);
    assert_eq!(p.catch_radius, 15.0);
    assert_eq!(p.turn_wait, [360.0, 180.0]);
    assert_eq!(p.turn_spin, [110.0, 90.0]);
    assert_eq!(p.aim_weights, [1, 1, 10, 50, 10, 1, 1, 1]);
    assert_eq!(p.stop_interval, [600, 300]);
    assert_eq!(p.drift_frames, [6000, 3000]);
    assert_eq!(
        (p.shot_damage, p.shot_angle, p.shot_base_knockback),
        (0, 361, 180)
    );
    let music = desc::read_music(&archive, 0x1E).unwrap();
    assert_eq!(music.rule, melee_gr::music::MusicRule::Primary);
}
