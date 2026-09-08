use hsd_archive::{Archive, ArchiveHeader};
use melee_mp::desc::{read_coll_data, read_public_coll_data, CollDescError};
use melee_mp::CollMapBuilder;
use melee_types::mp::{line_flag, MapLine};

const LINE: usize = 0x10;
const JOINT: usize = 0x20;
const ROOT: usize = 0x48;

fn word(data: &mut [u8], offset: usize, value: u32) {
    data[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}

fn half(data: &mut [u8], offset: usize, value: u16) {
    data[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
}

/// Independent hand-encoded layout: vertices at zero deliberately exercise
/// the distinction between a relocated zero pointer and null.
fn tiny_data() -> Vec<u8> {
    let mut data = vec![0; ROOT + 0x30];
    for (offset, value) in [(0, -2.5_f32), (4, 1.25), (8, 7.0), (12, 1.25)] {
        word(&mut data, offset, value.to_bits());
    }
    for (i, value) in [0, 1, 0xffff, 0xffff, 0xffff, 0xffff, 1, 0x0205]
        .into_iter()
        .enumerate()
    {
        half(&mut data, LINE + i * 2, value);
    }
    half(&mut data, JOINT + 2, 1); // floor count
    for (offset, value) in [(0x14, -2.5_f32), (0x18, 1.25), (0x1c, 7.0), (0x20, 1.25)] {
        word(&mut data, JOINT + offset, value.to_bits());
    }
    half(&mut data, JOINT + 0x26, 2); // vertex count
    word(&mut data, ROOT + 4, 2);
    word(&mut data, ROOT + 8, LINE as u32);
    word(&mut data, ROOT + 12, 1);
    half(&mut data, ROOT + 0x12, 1); // floor count
    word(&mut data, ROOT + 0x24, JOINT as u32);
    word(&mut data, ROOT + 0x28, 1);
    data
}

fn archive(data: &[u8], relocs: &[u32], public: bool) -> Archive {
    let strings = if public {
        b"coll_data\0".as_slice()
    } else {
        &[]
    };
    let public_bytes = if public { 8 } else { 0 };
    let header = ArchiveHeader {
        file_size: (ArchiveHeader::SIZE
            + data.len()
            + relocs.len() * 4
            + public_bytes
            + strings.len()) as u32,
        data_size: data.len() as u32,
        nb_reloc: relocs.len() as u32,
        nb_public: u32::from(public),
        nb_extern: 0,
        version: *b"001B",
    };
    let mut bytes = header.to_bytes().to_vec();
    bytes.extend_from_slice(data);
    for slot in relocs {
        bytes.extend_from_slice(&slot.to_be_bytes());
    }
    if public {
        bytes.extend_from_slice(&(ROOT as u32).to_be_bytes());
        bytes.extend_from_slice(&0_u32.to_be_bytes());
    }
    bytes.extend_from_slice(strings);
    Archive::parse(&bytes).unwrap()
}

fn tiny_archive(data: &[u8]) -> Archive {
    archive(
        data,
        &[ROOT as u32, ROOT as u32 + 8, ROOT as u32 + 0x24],
        true,
    )
}

#[test]
fn tiny_floor_matches_builder_field_for_field() {
    let archive = tiny_archive(&tiny_data());
    assert_eq!(archive.link(ROOT as u32).unwrap(), Some(0));
    let mut builder = CollMapBuilder::new();
    let left = builder.vertex(0, -2.5, 1.25);
    let right = builder.vertex(0, 7.0, 1.25);
    builder.floor(0, left, right, line_flag::LEDGE as u16 | 5);
    let expected = builder.build();
    assert_eq!(read_public_coll_data(&archive).unwrap(), expected);
    assert_eq!(read_coll_data(&archive, ROOT as u32).unwrap(), expected);
}

#[test]
fn line_fields_keep_unsigned_indices_flags_and_signed_links() {
    let mut data = tiny_data();
    for (i, value) in [
        0x8001, 0xfffe, 0x8000, 0x1234, 0xfffe, 0x5678, 0xabcd, 0xfedc,
    ]
    .into_iter()
    .enumerate()
    {
        half(&mut data, LINE + i * 2, value);
    }
    word(&mut data, ROOT + 0x2c, 0x8765_4321);
    let read = read_public_coll_data(&tiny_archive(&data)).unwrap();
    assert_eq!(
        read.lines[0],
        MapLine {
            v0_idx: 0x8001,
            v1_idx: 0xfffe,
            prev_id0: i16::MIN,
            next_id0: 0x1234,
            prev_id1: -2,
            next_id1: 0x5678,
            hi_flags: 0xabcd,
            lo_flags: 0xfedc,
        }
    );
    assert_eq!(read.x2c, 0x8765_4321_u32 as i32);
}

#[test]
fn null_empty_arrays_are_allowed() {
    let data = vec![0; ROOT + 0x30];
    assert_eq!(
        read_public_coll_data(&archive(&data, &[], true)).unwrap(),
        Default::default()
    );
}

#[test]
fn missing_public_is_descriptive() {
    let error = read_public_coll_data(&archive(&tiny_data(), &[], false)).unwrap_err();
    assert_eq!(error, CollDescError::MissingPublic);
    assert_eq!(error.to_string(), "archive has no coll_data public");
}

#[test]
fn null_and_unrelocated_nonnull_arrays_are_rejected() {
    let data = tiny_data();
    let a = archive(&data, &[], true);
    assert!(matches!(
        read_public_coll_data(&a),
        Err(CollDescError::BadCount { field: "verts", .. })
    ));
    let a = archive(&data, &[ROOT as u32], true);
    assert!(matches!(
        read_public_coll_data(&a),
        Err(CollDescError::UnrelocatedPointer { field: "lines", .. })
    ));
}

#[test]
fn negative_and_overflowing_array_counts_are_rejected() {
    for (field, offset) in [("verts", 4), ("lines", 12), ("joints", 0x28)] {
        for count in [-1_i32, i32::MAX] {
            let mut data = tiny_data();
            word(&mut data, ROOT + offset, count as u32);
            let error = read_public_coll_data(&tiny_archive(&data)).unwrap_err();
            assert!(
                matches!(error, CollDescError::BadCount { field: found, count: found_count, .. }
                if found == field && found_count == count)
            );
        }
    }
}

#[test]
fn pointers_and_array_extents_cannot_exceed_data_section() {
    let a = tiny_archive(&tiny_data());
    for offset in [ROOT as u32 + 1, u32::MAX] {
        assert!(matches!(
            read_coll_data(&a, offset),
            Err(CollDescError::OutOfRangePointer {
                field: "coll_data",
                ..
            })
        ));
    }
    for (field, slot) in [("verts", 0), ("lines", 8), ("joints", 0x24)] {
        for target in [ROOT as u32 + 0x2f, u32::MAX] {
            let mut data = tiny_data();
            word(&mut data, ROOT + slot, target);
            let error = read_public_coll_data(&tiny_archive(&data)).unwrap_err();
            assert!(
                matches!(error, CollDescError::OutOfRangePointer { field: found, .. } if found == field)
            );
        }
    }
}

#[test]
fn section_and_vertex_ranges_are_checked() {
    for base in [ROOT + 0x10, JOINT] {
        for section in 0..5 {
            for count in [0xffff, 2] {
                let mut data = tiny_data();
                half(&mut data, base + section * 4 + 2, count);
                assert!(matches!(
                    read_public_coll_data(&tiny_archive(&data)),
                    Err(CollDescError::BadCount { .. })
                ));
            }
        }
    }
    let mut data = tiny_data();
    half(&mut data, JOINT + 0x26, 3);
    assert!(matches!(
        read_public_coll_data(&tiny_archive(&data)),
        Err(CollDescError::BadCount {
            field: "joint vertices",
            ..
        })
    ));
}
