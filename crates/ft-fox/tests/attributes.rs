mod support;

use ft_fox::attributes::read_fox_attributes;
use melee_ft::desc::fox_attributes::FoxAttributes;
use melee_types::ItemKind;
use support::{archive, word};

#[test]
fn fox_special_fields_preserve_bits_signed_values_and_byte_flags() {
    let mut data = vec![0; 0xDC];
    for offset in (0..0xD0).step_by(4) {
        word(&mut data, offset, 0x3F00_0000 | offset as u32);
    }
    word(&mut data, 0, 0x8000_0000);
    word(&mut data, 4, 0x7FC0_5678);
    word(&mut data, 0x1C, 0x36);
    word(&mut data, 0x20, 0x4A);
    word(&mut data, 0x6C, (-15_i32) as u32);
    word(&mut data, 0xA4, (-4_i32) as u32);
    word(&mut data, 0xB0, 7);
    word(&mut data, 0xB4, (-50_i32) as u32);
    data[0xD0..0xD4].copy_from_slice(&[0x81, 0xFF, 0x7F, 0x42]);
    let a = archive(&data, &[0xD8], Some(("ftDataFox", 0xD4)));
    let f = read_fox_attributes(&a).unwrap();
    let floats = [
        ("blaster.unknown_1", f.blaster.unknown_1, 0x80000000),
        ("blaster.unknown_2", f.blaster.unknown_2, 0x7FC05678),
        ("blaster.unknown_3", f.blaster.unknown_3, 0x3F000008),
        ("blaster.unknown_4", f.blaster.unknown_4, 0x3F00000C),
        ("blaster.angle", f.blaster.angle, 0x3F000010),
        ("blaster.velocity", f.blaster.velocity, 0x3F000014),
        ("blaster.landing_lag", f.blaster.landing_lag, 0x3F000018),
        (
            "illusion.gravity_delay",
            f.illusion.gravity_delay,
            0x3F000024,
        ),
        (
            "illusion.startup_momentum_divisor",
            f.illusion.startup_momentum_divisor,
            0x3F000028,
        ),
        (
            "illusion.startup_air_friction",
            f.illusion.startup_air_friction,
            0x3F00002C,
        ),
        (
            "illusion.startup_fall_acceleration",
            f.illusion.startup_fall_acceleration,
            0x3F000030,
        ),
        (
            "illusion.ground_end_vel_x",
            f.illusion.ground_end_vel_x,
            0x3F000034,
        ),
        (
            "illusion.ground_friction",
            f.illusion.ground_friction,
            0x3F000038,
        ),
        (
            "illusion.air_end_vel_x",
            f.illusion.air_end_vel_x,
            0x3F00003C,
        ),
        (
            "illusion.end_air_friction",
            f.illusion.end_air_friction,
            0x3F000040,
        ),
        (
            "illusion.end_gravity_delay",
            f.illusion.end_gravity_delay,
            0x3F000044,
        ),
        (
            "illusion.end_fall_acceleration",
            f.illusion.end_fall_acceleration,
            0x3F000048,
        ),
        (
            "illusion.freefall_mobility",
            f.illusion.freefall_mobility,
            0x3F00004C,
        ),
        ("illusion.landing_lag", f.illusion.landing_lag, 0x3F000050),
        (
            "fire_fox.gravity_delay",
            f.fire_fox.gravity_delay,
            0x3F000054,
        ),
        ("fire_fox.vel_x", f.fire_fox.vel_x, 0x3F000058),
        (
            "fire_fox.air_momentum_preserve_x",
            f.fire_fox.air_momentum_preserve_x,
            0x3F00005C,
        ),
        ("fire_fox.fall_accel", f.fire_fox.fall_accel, 0x3F000060),
        (
            "fire_fox.direction_stick_min",
            f.fire_fox.direction_stick_min,
            0x3F000064,
        ),
        ("fire_fox.duration", f.fire_fox.duration, 0x3F000068),
        ("fire_fox.duration_end", f.fire_fox.duration_end, 0x3F000070),
        ("fire_fox.speed", f.fire_fox.speed, 0x3F000074),
        (
            "fire_fox.reverse_accel",
            f.fire_fox.reverse_accel,
            0x3F000078,
        ),
        (
            "fire_fox.ground_momentum_end",
            f.fire_fox.ground_momentum_end,
            0x3F00007C,
        ),
        ("fire_fox.unknown_2", f.fire_fox.unknown_2, 0x3F000080),
        ("fire_fox.bound_vel_x", f.fire_fox.bound_vel_x, 0x3F000084),
        (
            "fire_fox.facing_stick_min",
            f.fire_fox.facing_stick_min,
            0x3F000088,
        ),
        (
            "fire_fox.freefall_mobility",
            f.fire_fox.freefall_mobility,
            0x3F00008C,
        ),
        ("fire_fox.landing_lag", f.fire_fox.landing_lag, 0x3F000090),
        ("fire_fox.bound_angle", f.fire_fox.bound_angle, 0x3F000094),
        ("reflector.release_lag", f.reflector.release_lag, 0x3F000098),
        ("reflector.turn_frames", f.reflector.turn_frames, 0x3F00009C),
        ("reflector.unknown_1", f.reflector.unknown_1, 0x3F0000A0),
        (
            "reflector.momentum_preserve_x",
            f.reflector.momentum_preserve_x,
            0x3F0000A8,
        ),
        ("reflector.fall_accel", f.reflector.fall_accel, 0x3F0000AC),
        (
            "reflection.offset.x",
            f.reflector.reflection.offset.x,
            0x3F0000B8,
        ),
        (
            "reflection.offset.y",
            f.reflector.reflection.offset.y,
            0x3F0000BC,
        ),
        (
            "reflection.offset.z",
            f.reflector.reflection.offset.z,
            0x3F0000C0,
        ),
        ("reflection.size", f.reflector.reflection.size, 0x3F0000C4),
        (
            "reflection.damage_multiplier",
            f.reflector.reflection.damage_multiplier,
            0x3F0000C8,
        ),
        (
            "reflection.speed_multiplier",
            f.reflector.reflection.speed_multiplier,
            0x3F0000CC,
        ),
    ];
    for (name, value, bits) in floats {
        assert_eq!(value.to_bits(), bits, "{name}");
    }
    assert_eq!(f.blaster.shot_item_kind, ItemKind::FoxLaser);
    assert_eq!(i32::from(f.blaster.gun_item_kind), 0x4A);
    assert_eq!(f.fire_fox.bounce_var, -15);
    assert_eq!(f.reflector.gravity_delay, -4);
    assert_eq!(f.reflector.reflection.joint, 7);
    assert_eq!(f.reflector.reflection.max_damage, -50);
    assert_eq!(f.reflector.reflection.skip_ownership_change, 0x81);
}

#[test]
fn fox_attributes_reject_bad_links_enums_and_truncation() {
    let mut data = vec![0; 0xDC];
    assert!(read_fox_attributes(&archive(&data, &[], None)).is_err());
    assert!(read_fox_attributes(&archive(&data, &[], Some(("ftDataFox", 0xD4)))).is_err());
    word(&mut data, 0xD8, 4);
    assert!(read_fox_attributes(&archive(&data, &[], Some(("ftDataFox", 0xD4)))).is_err());
    word(&mut data, 0xD8, 0);
    for slot in [0x1C, 0x20] {
        word(&mut data, slot, u32::MAX);
        assert!(FoxAttributes::read(&archive(&data, &[], None), 0).is_err());
        word(&mut data, slot, 0);
    }
    assert!(FoxAttributes::read(&archive(&data[..0xD3], &[], None), 0).is_err());
    assert!(FoxAttributes::read(&archive(&data, &[], None), u32::MAX).is_err());
}

#[test]
fn real_fox_special_attributes() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files");
    if !melee_test_support::require_files(["PlFx.dat"].map(|name| dir.join(name))) {
        return;
    }
    let a = hsd_archive::Archive::parse(&std::fs::read(dir.join("PlFx.dat")).unwrap()).unwrap();
    let f = read_fox_attributes(&a).unwrap();
    println!("{f:#?}");
    for (name, value, bits) in [
        ("blaster velocity", f.blaster.velocity, 0x40E0_0000),
        (
            "illusion startup delay",
            f.illusion.gravity_delay,
            0x4170_0000,
        ),
        (
            "illusion end delay",
            f.illusion.end_gravity_delay,
            0x40A0_0000,
        ),
        (
            "illusion end acceleration",
            f.illusion.end_fall_acceleration,
            0x3DA3_D70A,
        ),
        ("fire fox travel duration", f.fire_fox.duration, 0x41F0_0000),
        ("fire fox speed", f.fire_fox.speed, 0x4073_3333),
        (
            "reflector release lag",
            f.reflector.release_lag,
            0x4190_0000,
        ),
        (
            "reflector fall acceleration",
            f.reflector.fall_accel,
            0x3CDA_740E,
        ),
        ("reflector radius", f.reflector.reflection.size, 0x4108_0000),
        (
            "reflected damage multiplier",
            f.reflector.reflection.damage_multiplier,
            0x3FC0_0000,
        ),
    ] {
        assert_eq!(value.to_bits(), bits, "{name}");
    }
    assert_eq!(f.blaster.shot_item_kind, ItemKind::FoxLaser);
    assert_eq!(i32::from(f.blaster.gun_item_kind), 0x4A);
    assert_eq!(f.fire_fox.bounce_var, 15);
    assert_eq!(f.reflector.gravity_delay, 4);
    assert_eq!(f.reflector.reflection.joint, 1);
    assert_eq!(f.reflector.reflection.max_damage, 50);
    assert_eq!(f.reflector.reflection.skip_ownership_change, 0);
}
