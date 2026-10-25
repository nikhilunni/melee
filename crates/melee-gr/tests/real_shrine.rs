//! Hyrule Temple's models come from GrSh.dat.
use hsd_archive::Archive;
use melee_gr::desc;
use std::path::PathBuf;

fn archive(name: &str) -> Option<Vec<u8>> {
    let root = std::env::var_os("MELEE_DATA_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));
    let path = root.join("harness/roms/files").join(name);
    if !melee_test_support::require_files([&path]) {
        return None;
    }
    Some(std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display())))
}

#[test]
fn hyrule_temple_has_three_static_maps() {
    let Some(bytes) = archive("GrSh.dat") else {
        return;
    };
    let archive = Archive::parse(&bytes).unwrap();
    let stage = desc::read_shrine(&archive).unwrap();
    assert_eq!(stage.kind, melee_types::GrKind::Shrine);
    assert_eq!(stage.models.len(), 3);
    // Only map 1 animates (a loop, joint and material); no map carries
    // collision joints, and the archive has no particle bank.
    let animations: Vec<_> = stage.models.iter().map(|m| m.animation_loops.clone()).collect();
    assert_eq!(animations, [vec![], vec![true], vec![]]);
    assert!(stage.models.iter().all(|m| m.joint_mappings.is_empty()));
    assert!(archive.public("map_ptcl").is_none());
    assert_eq!(stage.parameters.map_scale, 0.9);
}
