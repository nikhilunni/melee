//! Big Blue's models, collision bindings and parameters come from GrBb.dat.
use hsd_archive::Archive;
use melee_gr::{bigblue::procs, desc};
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
fn big_blue_maps_bind_the_road_cars_and_flyers() {
    let Some(bytes) = archive("GrBb.dat") else {
        return;
    };
    let archive = Archive::parse(&bytes).unwrap();
    let stage = desc::read_big_blue(&archive).unwrap();
    assert_eq!(stage.kind, melee_types::GrKind::BigBlue);
    assert_eq!(stage.parameters.map_scale, 0.5);
    let joints = |map: u8| -> Vec<(i16, i16)> {
        stage.models[usize::from(map)]
            .joint_mappings
            .iter()
            .map(|m| (m.joint_index, m.extra))
            .collect()
    };
    // The small flyers on bones 1, 6 and 11; a car on each of bones 1..30.
    assert_eq!(joints(procs::FLYERS_MAP), [(0, 1), (1, 6), (2, 11)]);
    let cars: Vec<_> = (0..30).map(|slot| (slot as i16 + 3, slot as i16 + 1)).collect();
    assert_eq!(joints(procs::CARS_MAP), cars);
    // The road's own six; grBb_803E2938 binds the rest (and rebinds 38, 39).
    assert_eq!(
        joints(procs::ROAD_MAP),
        [(33, 1), (35, 4), (38, 7), (39, 10), (50, 19), (51, 29)]
    );
    assert_eq!(joints(procs::FALCON_MAP), [(34, 1)]);
    assert_eq!(joints(procs::ROUTE_MAP), [(66, 1)]);
    // Map 0x1F (created first) and map 0 both pair markers 4..7.
    let pairs = |model: usize| -> Vec<(usize, i16)> {
        stage
            .position_bindings
            .iter()
            .filter(|b| b.model_id == model && (4..8).contains(&b.stage_position))
            .map(|b| (b.joint_index as usize, b.stage_position))
            .collect()
    };
    assert_eq!(pairs(0), [(17, 4), (18, 5), (19, 6), (20, 7)]);
    assert_eq!(pairs(usize::from(procs::MARKERS_MAP)), [(4, 4), (5, 5), (6, 6), (7, 7)]);
    let collision = desc::load_collision(&archive, &stage).unwrap();
    assert_eq!(collision.data().joints.len(), 67);
}

#[test]
fn the_retail_parameters_are_grbb_dat_s() {
    let Some(bytes) = archive("GrBb.dat") else {
        return;
    };
    let archive = Archive::parse(&bytes).unwrap();
    let read = desc::read_big_blue_parameters(&archive).unwrap();
    assert_eq!(read, melee_gr::bigblue::RETAIL_PARAMETERS);
}
