//! Optional retail fixture; game data stays in the gitignored harness directory.
use std::{fs, io::ErrorKind, path::Path};

use hsd_archive::Archive;
use hsd_types::Vec3;
use melee_mp::{desc::read_public_coll_data, CollMap};
use melee_types::{
    mp::{line_flag, LineSection, MapCollData, NO_ID},
    GrKind,
};

// Measured directly from the NTSC-U 1.02 data section, before writing these
// assertions. docs/DISC.md rounds these floats to four decimal places.
const LEFT: u32 = 0xc2ab_21a3;
const RIGHT: u32 = 0x42ab_21a3;
const BOUNDS: [u32; 4] = [0xc2bb_21a3, 0xc27d_8d84, 0x42bb_21a3, 0x4100_0000];

fn final_destination() -> Option<MapCollData> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files/GrNLa.dat");
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            eprintln!("skipping real stage test: {} is absent", path.display());
            return None;
        }
        Err(error) => panic!("cannot read {}: {error}", path.display()),
    };
    let archive = Archive::parse(&bytes).expect("parse GrNLa.dat");
    Some(read_public_coll_data(&archive).expect("read coll_data"))
}

fn assert_float(name: &str, value: f32, bits: u32) {
    println!("{name}: {value:.17} ({:#010x})", value.to_bits());
    assert_eq!(value.to_bits(), bits, "{name}");
}

#[test]
fn final_destination_descriptor_matches_disc_measurements() {
    let Some(data) = final_destination() else {
        return;
    };
    assert_eq!(data.verts.len(), 16);
    assert_eq!(data.lines.len(), 16);
    assert_eq!(data.joints.len(), 1);
    let sections = [(0, 3), (3, 3), (6, 5), (11, 5), (0, 0)];
    for (section, expected) in LineSection::ALL.into_iter().zip(sections) {
        assert_eq!(data.section(section), expected);
        assert_eq!(data.joints[0].section(section), expected);
    }
    assert_eq!(data.x2c, 0);
    for (id, (v0, v1)) in [(4, 15), (15, 6), (6, 5)].into_iter().enumerate() {
        let line = data.lines[id];
        assert_eq!((line.v0_idx, line.v1_idx), (v0, v1));
        assert_eq!(line.hi_flags, 1);
        assert_eq!(
            line.lo_flags,
            if id == 1 { 0 } else { line_flag::LEDGE as u16 }
        );
        for vertex in [v0, v1] {
            assert_float("floor y", data.verts[usize::from(vertex)].y, 0);
        }
    }
    assert_float("floor left", data.verts[4].x, LEFT);
    assert_float("floor right", data.verts[5].x, RIGHT);
    assert_float("middle floor left", data.verts[15].x, (-75.0_f32).to_bits());
    assert_float("middle floor right", data.verts[6].x, 75.0_f32.to_bits());
    let joint = data.joints[0];
    assert_eq!((joint.vtx_start, joint.vtx_count), (0, 16));
    for ((name, value), bits) in [
        ("joint left", joint.left_bound),
        ("joint bottom", joint.bottom_bound),
        ("joint right", joint.right_bound),
        ("joint top", joint.top_bound),
    ]
    .into_iter()
    .zip(BOUNDS)
    {
        assert_float(name, value, bits);
    }
}

#[test]
fn final_destination_floor_and_ledge_queries() {
    let Some(data) = final_destination() else {
        return;
    };
    let mut map = CollMap::load(data, 1.0, GrKind::Last);
    for (x, id) in [(-80.0, 0), (0.0, 1), (80.0, 2)] {
        let hit = map
            .check_floor(x, 10.0, x, -10.0, 0.0, NO_ID, NO_ID, NO_ID, None)
            .unwrap();
        assert_eq!(hit.line_id, id);
        assert_eq!(hit.pos.x.to_bits(), x.to_bits());
        assert_eq!(hit.pos.y.to_bits(), 0);
        assert_eq!(hit.normal, Vec3::new(0.0, 1.0, 0.0));
    }
    // mpLib_8004ED5C (mplib.c:1565) extends linked floor endpoints for
    // check_floor. floor_below uses the actual vertices instead.
    for (x, id) in [(-85.57, 0), (85.57, 2)] {
        assert_eq!(
            map.floor_below(&Vec3::new(x, 10.0, 0.0), NO_ID, NO_ID),
            NO_ID
        );
        let hit = map
            .check_floor(x, 10.0, x, -10.0, 0.0, NO_ID, NO_ID, NO_ID, None)
            .unwrap();
        assert_eq!(hit.line_id, id);
        assert_eq!(hit.pos.x.to_bits(), x.to_bits());
        assert_eq!(hit.pos.y.to_bits(), 0);
    }
    // Beyond even the extended endpoints, sweeps also miss.
    for x in [-88.0, 88.0] {
        assert!(map
            .check_floor(x, 10.0, x, -10.0, 0.0, NO_ID, NO_ID, NO_ID, None)
            .is_none());
    }
    assert_eq!(map.floor_below(&Vec3::new(0.0, 10.0, 0.0), NO_ID, NO_ID), 1);
    let left = map
        .find_ledge(NO_ID, NO_ID, NO_ID, 1, -90.0, -5.0, -85.0, 5.0)
        .unwrap();
    assert_eq!(left.line_id, 0);
    assert_eq!(left.pos.x.to_bits(), LEFT);
    assert_eq!(left.pos.y.to_bits(), 0);
    let right = map
        .find_ledge(NO_ID, NO_ID, NO_ID, -1, 85.0, -5.0, 90.0, 5.0)
        .unwrap();
    assert_eq!(right.line_id, 2);
    assert_eq!(right.pos.x.to_bits(), RIGHT);
    assert_eq!(right.pos.y.to_bits(), 0);
    assert_eq!(map.floor_get_left(1), left.pos);
    assert_eq!(map.floor_get_right(1), right.pos);
    assert!(map
        .find_ledge(NO_ID, NO_ID, NO_ID, 1, -5.0, -5.0, 5.0, 5.0)
        .is_none());
}

#[test]
fn final_destination_walls_and_underside_queries() {
    let Some(data) = final_destination() else {
        return;
    };
    let mut map = CollMap::load(data, 1.0, GrKind::Last);
    // The outer walls are vertical only down to y=-10.5: lines 9 and 11.
    let right = map
        .check_right_wall(90.0, -5.0, 80.0, -5.0, NO_ID, NO_ID)
        .unwrap();
    assert_eq!(right.line_id, 9);
    assert_eq!(right.pos.x.to_bits(), RIGHT);
    assert_eq!(right.pos.y.to_bits(), (-5.0_f32).to_bits());
    assert_eq!(right.normal, Vec3::new(1.0, 0.0, 0.0));
    let left = map
        .check_left_wall(-90.0, -5.0, -80.0, -5.0, NO_ID, NO_ID)
        .unwrap();
    assert_eq!(left.line_id, 11);
    assert_eq!(left.pos.x.to_bits(), LEFT);
    assert_eq!(left.pos.y.to_bits(), (-5.0_f32).to_bits());
    assert_eq!(left.normal, Vec3::new(-1.0, 0.0, 0.0));
    for y in [5.0, -25.0] {
        assert!(map
            .check_right_wall(90.0, y, 80.0, y, NO_ID, NO_ID)
            .is_none());
        assert!(map
            .check_left_wall(-90.0, y, -80.0, y, NO_ID, NO_ID)
            .is_none());
    }
    // Flat central underside is line 4, between vertices 9 and 7.
    let bottom = map
        .check_ceiling(0.0, -60.0, 0.0, -50.0, NO_ID, NO_ID)
        .unwrap();
    assert_eq!(bottom.line_id, 4);
    assert_eq!(bottom.pos.y.to_bits(), 0xc25d_8d84); // -55.3881988525390625
    assert_eq!(bottom.normal, Vec3::new(0.0, -1.0, 0.0));
}
