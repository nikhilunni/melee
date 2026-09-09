mod support;

use melee_ft::desc::{read_fighter_bones, read_part_table, PartTable};
use melee_types::{FighterKind, FtPart};
use support::{archive, word};

#[test]
fn part_maps_have_independent_lengths_and_preserve_absent_parts() {
    let mut data = vec![0; 0x80];
    data[..6].copy_from_slice(&[0, 2, 0xFF, 1, 0xFF, 53]);
    data[8..62].fill(0xFF);
    for (part, joint) in [(0, 0), (1, 3), (2, 1), (53, 5)] {
        data[8 + part] = joint;
    }
    word(&mut data, 0x44, 8);
    word(&mut data, 0x48, 6);
    word(&mut data, 0x54, 0x40); // Fox slot in root's table
    word(&mut data, 0x70, 0x50); // PlCo root+16
    let a = archive(
        &data,
        &[0x40, 0x44, 0x54, 0x70],
        Some(("ftLoadCommonData", 0x60)),
    );
    let table = read_part_table(&a, FighterKind::Fox, 54).unwrap();
    assert_eq!(table.joint_count(), 6);
    assert_eq!(table.joint(FtPart::TopN), Some(0));
    assert_eq!(table.joint(FtPart::TransN), Some(3));
    assert_eq!(table.joint(FtPart::XRotN), Some(1));
    assert_eq!(table.joint(FtPart::YRotN), None);
    assert_eq!(table.joint(FtPart::Unknown56), None);
    assert_eq!(table.joint(FtPart::Unknown109), None);
    assert_eq!(
        table.joint_to_part,
        [Some(0), Some(2), None, Some(1), None, Some(53)]
    );
    assert_eq!(table.part_to_joint[53], Some(5));
    assert!(read_part_table(&a, FighterKind::None, 54).is_err());
    for part in FtPart::ALL {
        assert_eq!(FtPart::try_from(i32::from(*part)), Ok(*part));
    }
    assert!(FtPart::try_from(53).is_err()); // present on disc, unnamed in the header
}

#[test]
fn malformed_part_tables_fail_before_allocating_or_indexing() {
    let mut data = vec![0; 32];
    word(&mut data, 20, 4);
    word(&mut data, 24, 1);
    for count in [141, u32::MAX] {
        word(&mut data, 24, count);
        assert!(PartTable::read(&archive(&data, &[16, 20], None), 16, 1).is_err());
    }
    word(&mut data, 24, 1);
    assert!(PartTable::read(&archive(&data, &[16, 20], None), 16, 256).is_err());
    assert!(PartTable::read(&archive(&data, &[16], None), 16, 1).is_err());
    assert!(PartTable::read(&archive(&data, &[20], None), 16, 1).is_err());
    data[4] = 1; // joint 1 does not exist
    assert!(PartTable::read(&archive(&data, &[16, 20], None), 16, 1).is_err());
    data[4] = 0;
    data[0] = 1; // part 1 does not exist
    assert!(PartTable::read(&archive(&data, &[16, 20], None), 16, 1).is_err());
    data[0] = 0;
    word(&mut data, 20, 31);
    word(&mut data, 24, 2);
    assert!(PartTable::read(&archive(&data, &[16, 20], None), 16, 2).is_err());
}

fn bone_data() -> (Vec<u8>, Vec<u32>) {
    let mut d = vec![0; 0x360];
    d[0x10..0x15].copy_from_slice(&[7, 6, 5, 4, 3]);
    for (i, bone) in [-1_i16, 8, 0, 3, 4, 5].into_iter().enumerate() {
        d[0x20 + i * 2..0x22 + i * 2].copy_from_slice(&bone.to_be_bytes());
    }
    for (off, bits) in [
        (0x2C, 0x8000_0000),
        (0x30, 0x3F80_0000),
        (0x34, 0x4000_0000),
        (0x38, 0x4040_0000),
    ] {
        word(&mut d, off, bits);
    }
    word(&mut d, 0x40, 0x60);
    word(&mut d, 0x60, (7 << 16) | 3);
    word(&mut d, 0x64, 0x70);
    d[0x70..0x73].copy_from_slice(&[9, 2, 6]);
    for (off, value) in [
        (0x80, 1),
        (0x84, 0xA0),
        (0x88, 1),
        (0x8C, 0xC0),
        (0xA0, 17),
        (0xC0, 41),
        (0xE0, 2),
        (0xE4, 0x100),
        (0x100, 55),
        (0x128, 25),
        (0x180, 4),
        (0x188, 6),
        (0x31C, 0x40),
        (0x32C, 0x80),
        (0x330, 0xE0),
        (0x334, 0x180),
        (0x338, 0x188),
        (0x344, 0x20),
    ] {
        word(&mut d, off, value);
    }
    (
        d,
        vec![
            0x40, 0x64, 0x84, 0x8C, 0xE4, 0x308, 0x31C, 0x32C, 0x330, 0x334, 0x338, 0x344,
        ],
    )
}

#[test]
fn bone_lists_keep_order_width_and_relocated_zero() {
    let (data, relocs) = bone_data();
    let b = read_fighter_bones(&archive(&data, &relocs, None), 0x300).unwrap();
    assert_eq!(b.ecb.joints, [-1, 8, 0, 3, 4, 5]);
    assert_eq!(b.ecb.center_y.to_bits(), 0x8000_0000);
    assert_eq!(b.ecb.ledge_snap_x.to_bits(), 0x3F80_0000);
    assert_eq!(b.ecb.ledge_snap_y.to_bits(), 0x4000_0000);
    assert_eq!(b.ecb.ledge_snap_height.to_bits(), 0x4040_0000);
    assert_eq!(b.model.animation_translation, 7);
    assert_eq!(b.model.shield, 6);
    assert_eq!(b.model.held_item, 5);
    assert_eq!(b.model.left_foot, 4);
    assert_eq!(b.model.right_foot, 3);
    assert_eq!(b.animation_sets[0].as_ref().unwrap().root_joint, 7);
    assert_eq!(b.animation_sets[0].as_ref().unwrap().joints, [9, 2, 6]);
    assert!(b.animation_sets[1..].iter().all(Option::is_none));
    assert_eq!(b.dynamics_roots, [17]);
    assert_eq!(b.dynamics_collision, [41]);
    assert_eq!(b.hurtboxes, [55, 25]);
    assert_eq!(b.scaled_joint, Some(4));
    assert_eq!(b.attachment_joint, Some(6));
}

#[test]
fn bone_arrays_reject_bad_counts_links_and_extents() {
    for (slot, value) in [
        (0x80, 10),
        (0x88, 12),
        (0xE0, 16),
        (0xE0, u32::MAX),
        (0xE4, 0x350),
        (0x344, 0x350),
        (0x64, 0x35F),
    ] {
        let (mut data, relocs) = bone_data();
        word(&mut data, slot, value);
        assert!(
            read_fighter_bones(&archive(&data, &relocs, None), 0x300).is_err(),
            "slot {slot:#x}"
        );
    }
    let (data, relocs) = bone_data();
    for omitted in [0x308, 0x344, 0x31C, 0x40, 0x64, 0xE4] {
        let relocs: Vec<_> = relocs
            .iter()
            .copied()
            .filter(|&slot| slot != omitted)
            .collect();
        assert!(read_fighter_bones(&archive(&data, &relocs, None), 0x300).is_err());
    }
}
