//! Flat Zone's parameters and models come from GrFz.dat.
use hsd_archive::Archive;
use melee_gr::desc;
use std::path::PathBuf;

fn archive(name: &str) -> Option<Vec<u8>> {
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    );
    let path = root.join("harness/roms/files").join(name);
    if !melee_test_support::require_files([&path]) {
        return None;
    }
    Some(std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display())))
}

#[test]
fn flat_zone_hazard_timings_and_platform_layouts_come_from_the_archive() {
    let Some(bytes) = archive("GrFz.dat") else {
        return;
    };
    let archive = Archive::parse(&bytes).unwrap();
    let stage = desc::read_flatzone(&archive).unwrap();
    assert_eq!(stage.kind, melee_types::GrKind::Flatzone);
    // Eight models: map 8 (the tools) is past them and gets an empty JObj.
    assert_eq!(stage.models.len(), 8);
    // Map 5 plays the 22 layout animations of grFz_803E7A68 and carries
    // collision joints 0..21 on descendants 1..22.
    assert_eq!(stage.models[5].animations.len(), 22);
    let platforms: Vec<_> = stage.models[5]
        .joint_mappings
        .iter()
        .map(|m| (m.joint_index, m.extra))
        .collect();
    assert_eq!(platforms, (0..22).map(|j| (j, j + 1)).collect::<Vec<_>>());
    let p = desc::read_flatzone_parameters(&archive).unwrap();
    assert_eq!(p.event_wait, [3000, 1800]);
    assert_eq!(p.variant_wait, [420, 120]);
    assert_eq!((p.tools_frames, p.tools_interval), (1200, 60));
    assert_eq!((p.tools_right, p.tools_left), (40, -60));
    assert_eq!(p.oil_walk_in, 130);
    assert_eq!((p.oil_right_x, p.oil_left_x), (-72.5, 58.5));
    assert_eq!((p.oil_step_frames, p.oil_steps, p.oil_step), (24, [2, 8], 8.0));
    assert_eq!(p.oil_frames, 1200);
}
