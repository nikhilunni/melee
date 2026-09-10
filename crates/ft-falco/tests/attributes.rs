use ft_falco::{attributes::read_falco_attributes, init::Falco};
use hsd_archive::{Archive, ArchiveHeader};
use melee_ft::fighter::{AerialJumpStyle, Capabilities, CharacterCallbacks};
use melee_types::{FighterKind, ItemKind};

/// A synthetic ftData.ext_attr link to offset zero, with independently encoded
/// item IDs and a negative-zero payload. No disc data is embedded in this test.
fn synthetic_archive(symbol: &str) -> Archive {
    let mut data = vec![0; 0xDC];
    for (offset, value) in [(0, 0x8000_0000_u32), (0x1C, 0x37), (0x20, 0x4B)] {
        data[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }
    let header = ArchiveHeader {
        file_size: (32 + data.len() + 4 + 8 + symbol.len() + 1) as u32,
        data_size: data.len() as u32,
        nb_reloc: 1,
        nb_public: 1,
        nb_extern: 0,
        version: *b"001B",
    };
    let mut bytes = header.to_bytes().to_vec();
    bytes.extend_from_slice(&data);
    for word in [0xD8_u32, 0xD4, 0] {
        bytes.extend_from_slice(&word.to_be_bytes());
    }
    bytes.extend_from_slice(symbol.as_bytes());
    bytes.push(0);
    Archive::parse(&bytes).unwrap()
}

#[test]
fn falco_root_loads_the_shared_layout_and_reset_keeps_registrations() {
    assert!(read_falco_attributes(&synthetic_archive("ftDataFox")).is_err());
    let mut falco = Falco::from_archive(&synthetic_archive("ftDataFalco")).unwrap();
    assert_eq!(falco.attributes.blaster.unknown_1.to_bits(), 0x8000_0000);
    assert_eq!(falco.kind(), FighterKind::Falco);
    let mut capabilities = Capabilities::default();
    falco.on_load(&mut capabilities);
    assert!(capabilities.can_walljump);
    assert_eq!(capabilities.specials, [true; 4]);
    assert_eq!(falco.aerial_jump_style(), AerialJumpStyle::Basic);
    let registrations = vec![
        ItemKind::FalcoLaser,
        ItemKind::FalcoBlaster,
        ItemKind::FalcoPhantasm,
    ];
    assert_eq!(falco.registered_items, registrations);
    falco.blaster_present = true;
    falco.model_group = 7;
    falco.on_reset();
    assert!(!falco.blaster_present);
    assert_eq!(falco.model_group, 0);
    assert_eq!(falco.registered_items, registrations);
}

#[test]
fn real_falco_attributes_and_empty_dynamics() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files/PlFc.dat");
    if !melee_test_support::require_files([&path]) {
        return;
    }
    let archive = Archive::parse(&std::fs::read(path).unwrap()).unwrap();
    let attributes = read_falco_attributes(&archive).unwrap();
    for (field, value, expected) in [
        ("blaster speed", attributes.blaster.velocity, 0x40A0_0000),
        (
            "phantasm delay",
            attributes.illusion.gravity_delay,
            0x4170_0000,
        ),
        ("fire bird speed", attributes.fire_fox.speed, 0x4086_6666),
        (
            "reflector radius",
            attributes.reflector.reflection.size,
            0x4108_0000,
        ),
    ] {
        assert_eq!(value.to_bits(), expected, "{field}");
    }
    assert_eq!(attributes.blaster.shot_item_kind, ItemKind::FalcoLaser);
    assert_eq!(attributes.blaster.gun_item_kind, ItemKind::FalcoBlaster);
    assert_eq!(attributes.reflector.gravity_delay, 4);
    assert_eq!(attributes.reflector.reflection.joint, 1);
    assert_eq!(attributes.reflector.reflection.max_damage, 50);
    let root = archive.public("ftDataFalco").unwrap();
    assert!(melee_ft::dynamics::read_sets(&archive, root)
        .unwrap()
        .is_empty());
}
