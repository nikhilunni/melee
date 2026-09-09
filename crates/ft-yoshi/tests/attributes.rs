mod support;
use ft_yoshi::{
    attributes::{read_yoshi_attributes, YoshiAttributes},
    init::{Yoshi, DESCRIPTOR},
};
use melee_ft::fighter::{AerialJumpStyle, Capabilities, CharacterCallbacks};
use melee_types::{FighterKind, ItemKind};
use support::{archive, word};

#[test]
fn relocated_zero_preserves_payloads_and_rejects_truncated_attributes() {
    let mut bytes = vec![0; 0x140];
    word(&mut bytes, 0, (-12_i32) as u32);
    word(&mut bytes, 4, 0x8000_0000);
    word(&mut bytes, 8, 0x7FC0_1234);
    word(&mut bytes, 0xA4, (-3_i32) as u32);
    word(&mut bytes, 0xEC, 0x3F26_6666);
    bytes[0x137] = 0x1D;
    let source = archive(&bytes, &[0x13C], Some(("ftDataYoshi", 0x138)));
    let attributes = read_yoshi_attributes(&source).unwrap();
    assert_eq!(attributes.double_jump.turn_frames, -12);
    assert_eq!(
        attributes.double_jump.reverse_threshold.to_bits(),
        0x8000_0000
    );
    assert_eq!(attributes.double_jump.armor.to_bits(), 0x7FC0_1234);
    assert_eq!(attributes.egg_roll.effect_interval, -3);
    assert_eq!(
        attributes.egg_throw.angle_stick_divisor.to_bits(),
        0x3F26_6666
    );
    assert_eq!(attributes.trailing_table[11], 0x1D);
    assert!(read_yoshi_attributes(&archive(&bytes, &[], Some(("ftDataYoshi", 0x138)))).is_err());
    assert!(YoshiAttributes::read(&archive(&bytes[..0x137], &[], None), 0).is_err());
}

#[test]
fn death_clears_egg_handle_and_model_but_preserves_roll_scale() {
    let attributes = YoshiAttributes::read(&archive(&[0; 0x138], &[], None), 0).unwrap();
    let mut yoshi = Yoshi::new(attributes.clone());
    let mut capabilities = Capabilities::default();
    yoshi.on_load(&mut capabilities);
    assert_eq!(yoshi.kind(), FighterKind::Yoshi);
    assert_eq!(capabilities.specials, [true; 4]);
    assert!(!capabilities.can_walljump);
    assert!(capabilities.grounded_down_bound);
    assert_eq!(yoshi.aerial_jump_style(), AerialJumpStyle::Yoshi);
    assert_eq!(
        yoshi.registered_items,
        [
            ItemKind::YoshiEggThrow,
            ItemKind::YoshiStar,
            ItemKind::YoshiEggLay
        ]
    );
    let mut raw = vec![0; 0x223C];
    word(&mut raw, 0x222C, 2.0_f32.to_bits());
    word(&mut raw, 0x2230, 3.0_f32.to_bits());
    word(&mut raw, 0x2234, 4.0_f32.to_bits());
    word(&mut raw, 0x2238, 0x81234560);
    yoshi.restore_saved(&raw);
    yoshi.model_group = 1;
    assert!(yoshi.egg_active);
    yoshi.on_reset();
    assert!(!yoshi.egg_active);
    assert_eq!(yoshi.model_group, 0);
    assert_eq!(yoshi.egg_roll_scale, hsd_types::Vec3::new(2.0, 3.0, 4.0));
    assert_eq!(yoshi.attributes, attributes);
}

#[test]
fn disc_data_and_all_six_costume_materials() {
    let files = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files");
    let required = [DESCRIPTOR.data_file, "PlCo.dat", DESCRIPTOR.animation_file]
        .into_iter()
        .chain(DESCRIPTOR.costumes.iter().map(|c| c.file));
    if let Some(missing) = required.map(|f| files.join(f)).find(|f| !f.is_file()) {
        eprintln!("skipping local Yoshi data: {} absent", missing.display());
        return;
    }
    let load =
        |name| hsd_archive::Archive::parse(&std::fs::read(files.join(name)).unwrap()).unwrap();
    let data = load(DESCRIPTOR.data_file);
    let root = data.public(DESCRIPTOR.data_symbol).unwrap();
    let mut yoshi = Yoshi::from_archive(&data).unwrap();
    assert_eq!(yoshi.attributes.double_jump.turn_frames, 12);
    assert_eq!(
        yoshi.attributes.double_jump.reverse_threshold.to_bits(),
        0x3E99_999A
    );
    assert_eq!(yoshi.attributes.double_jump.armor.to_bits(), 0x42F0_0000);
    assert_eq!(yoshi.attributes.egg_roll.duration, 360);
    assert_eq!(
        yoshi.attributes.ground_pound.fall_speed.to_bits(),
        0xC0A0_0000
    );
    assert_eq!(yoshi.attributes.shield_material_frames.to_bits(), 0);
    assert!(melee_ft::dynamics::read_sets(&data, root)
        .unwrap()
        .is_empty());
    let bones =
        melee_ft::desc::read_fighter_bones(&data, root, DESCRIPTOR.part_animation_count).unwrap();
    assert!(bones.dynamics_collision.is_empty());
    let common = load("PlCo.dat");
    let parts = melee_ft::desc::read_part_table(&common, FighterKind::Yoshi, DESCRIPTOR.part_count)
        .unwrap();
    assert_eq!(parts.joint_count(), 70);
    assert_eq!(parts.part_to_joint.len(), 54);
    assert!(melee_ft::desc::playback::read_wait_table(&data, root)
        .unwrap()
        .is_none());
    for (costume, descriptor) in DESCRIPTOR.costumes.iter().enumerate() {
        yoshi
            .on_costume_loaded(&load(descriptor.file), costume as u8)
            .unwrap();
        assert_eq!(yoshi.frozen_materials, [35, 34]);
        eprintln!(
            "{}: egg material frames {}",
            descriptor.file, yoshi.attributes.shield_material_frames
        );
        assert_eq!(
            yoshi.attributes.shield_material_frames.to_bits(),
            100.0_f32.to_bits()
        );
    }
}
