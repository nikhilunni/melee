mod support;

use melee_ft::desc::{read_fighter_attributes, FighterAttributes};
use melee_types::snapshot::{SnapValue, Snapshot};
use support::{archive, word};

#[test]
fn every_common_attribute_keeps_its_offset_type_and_bits() {
    let mut data = vec![0; 0x188];
    // Distinct words make swapped fields observable; NaN and negative zero
    // specifically guard against float normalization during a pure data read.
    for offset in (0..0x180).step_by(4) {
        word(&mut data, offset, 0x3F00_0000 | offset as u32);
    }
    word(&mut data, 0, 0x8000_0000);
    word(&mut data, 4, 0x7FC0_1234);
    word(&mut data, 0x58, (-2_i32) as u32);
    data[0x180..0x184].copy_from_slice(&[0xA5, 0xFF, 0x7F, 0x42]);
    // Relocated zero points to the attribute block; the root is after it.
    let archive = archive(&data, &[0x184], None);
    let attrs = read_fighter_attributes(&archive, 0x184).unwrap();
    let mut actual: Vec<(String, SnapValue)> = Vec::new();
    attrs.snapshot(&mut actual);
    let expected = [
        ("walking.walk_accel_mul", SnapValue::F32(0x80000000)),
        ("walking.walk_accel_base", SnapValue::F32(0x7FC01234)),
        ("walking.walk_max_vel", SnapValue::F32(0x3F000008)),
        ("walking.slow_walk_max", SnapValue::F32(0x3F00000C)),
        ("walking.mid_walk_point", SnapValue::F32(0x3F000010)),
        ("walking.fast_walk_min", SnapValue::F32(0x3F000014)),
        ("ground.ground_friction", SnapValue::F32(0x3F000018)),
        ("running.dash_initial_velocity", SnapValue::F32(0x3F00001C)),
        ("running.dash_accel_mul", SnapValue::F32(0x3F000020)),
        ("running.dash_accel_base", SnapValue::F32(0x3F000024)),
        ("running.dash_max_velocity", SnapValue::F32(0x3F000028)),
        ("running.run_animation_scaling", SnapValue::F32(0x3F00002C)),
        ("running.max_run_brake_frames", SnapValue::F32(0x3F000030)),
        (
            "ground.ground_max_horizontal_velocity",
            SnapValue::F32(0x3F000034),
        ),
        ("jumping.jump_startup_time", SnapValue::F32(0x3F000038)),
        (
            "jumping.jump_h_initial_velocity",
            SnapValue::F32(0x3F00003C),
        ),
        (
            "jumping.jump_v_initial_velocity",
            SnapValue::F32(0x3F000040),
        ),
        (
            "jumping.ground_to_air_jump_momentum_multiplier",
            SnapValue::F32(0x3F000044),
        ),
        ("jumping.jump_h_max_velocity", SnapValue::F32(0x3F000048)),
        ("jumping.hop_v_initial_velocity", SnapValue::F32(0x3F00004C)),
        ("jumping.air_jump_v_multiplier", SnapValue::F32(0x3F000050)),
        ("jumping.air_jump_h_multiplier", SnapValue::F32(0x3F000054)),
        ("jumping.max_jumps", SnapValue::I64(-2)),
        ("air.gravity", SnapValue::F32(0x3F00005C)),
        ("air.terminal_velocity", SnapValue::F32(0x3F000060)),
        ("air.air_drift_stick_mul", SnapValue::F32(0x3F000064)),
        ("air.aerial_drift_base", SnapValue::F32(0x3F000068)),
        ("air.air_drift_max", SnapValue::F32(0x3F00006C)),
        ("air.aerial_friction", SnapValue::F32(0x3F000070)),
        ("air.fast_fall_velocity", SnapValue::F32(0x3F000074)),
        (
            "air.air_max_horizontal_velocity",
            SnapValue::F32(0x3F000078),
        ),
        ("combat.jab_2_input_window", SnapValue::F32(0x3F00007C)),
        ("combat.jab_3_input_window", SnapValue::F32(0x3F000080)),
        ("ground.standing_turn_frames", SnapValue::F32(0x3F000084)),
        ("size.weight", SnapValue::F32(0x3F000088)),
        ("size.model_scaling", SnapValue::F32(0x3F00008C)),
        ("shield.initial_shield_size", SnapValue::F32(0x3F000090)),
        (
            "shield.shield_break_initial_velocity",
            SnapValue::F32(0x3F000094),
        ),
        ("combat.rapid_jab_window", SnapValue::I64(0x3F000098)),
        ("combat.clank_animation_length", SnapValue::F32(0x3F00009C)),
        ("combat.hit_spark_variant", SnapValue::I64(0x3F0000A0)),
        ("combat.unused_0", SnapValue::I64(0x3F0000A4)),
        (
            "ledge.ledge_jump_horizontal_velocity",
            SnapValue::F32(0x3F0000A8),
        ),
        (
            "ledge.ledge_jump_vertical_velocity",
            SnapValue::F32(0x3F0000AC),
        ),
        (
            "items.item_throw_velocity_multiplier",
            SnapValue::F32(0x3F0000B0),
        ),
        (
            "items.heavy_throw_velocity_multiplier",
            SnapValue::F32(0x3F0000B4),
        ),
        (
            "specials.specials_ground_speed_retention",
            SnapValue::F32(0x3F0000B8),
        ),
        ("yoshi_egg.size", SnapValue::F32(0x3F0000BC)),
        ("yoshi_egg.hurtbox_start.x", SnapValue::F32(0x3F0000C0)),
        ("yoshi_egg.hurtbox_start.y", SnapValue::F32(0x3F0000C4)),
        ("yoshi_egg.hurtbox_start.z", SnapValue::F32(0x3F0000C8)),
        ("yoshi_egg.hurtbox_end.x", SnapValue::F32(0x3F0000CC)),
        ("yoshi_egg.hurtbox_end.y", SnapValue::F32(0x3F0000D0)),
        ("yoshi_egg.hurtbox_end.z", SnapValue::F32(0x3F0000D4)),
        ("yoshi_egg.hurtbox_scale", SnapValue::F32(0x3F0000D8)),
        ("kirby_throw.star_scale", SnapValue::F32(0x3F0000DC)),
        ("combat.kirby_b_star_damage", SnapValue::F32(0x3F0000E0)),
        ("landing.normal_landing_lag", SnapValue::F32(0x3F0000E4)),
        ("landing.landingairn_lag", SnapValue::F32(0x3F0000E8)),
        ("landing.landingairf_lag", SnapValue::F32(0x3F0000EC)),
        ("landing.landingairb_lag", SnapValue::F32(0x3F0000F0)),
        ("landing.landingairhi_lag", SnapValue::F32(0x3F0000F4)),
        ("landing.landingairlw_lag", SnapValue::F32(0x3F0000F8)),
        ("size.name_tag_height", SnapValue::F32(0x3F0000FC)),
        ("wall.passivewall_vel_x", SnapValue::F32(0x3F000100)),
        (
            "wall.wall_jump_horizontal_velocity",
            SnapValue::F32(0x3F000104),
        ),
        (
            "wall.wall_jump_vertical_velocity",
            SnapValue::F32(0x3F000108),
        ),
        ("wall.passiveceil_vel_x", SnapValue::F32(0x3F00010C)),
        ("size.trophy_scale", SnapValue::F32(0x3F000110)),
        ("items.bunny_hood_offset_1.x", SnapValue::F32(0x3F000114)),
        ("items.bunny_hood_offset_1.y", SnapValue::F32(0x3F000118)),
        ("items.bunny_hood_offset_1.z", SnapValue::F32(0x3F00011C)),
        ("items.bunny_hood_offset_2.x", SnapValue::F32(0x3F000120)),
        ("items.bunny_hood_offset_2.y", SnapValue::F32(0x3F000124)),
        ("items.bunny_hood_offset_2.z", SnapValue::F32(0x3F000128)),
        ("items.bunny_hood_scale", SnapValue::F32(0x3F00012C)),
        ("items.flower_flame_offset.x", SnapValue::F32(0x3F000130)),
        ("items.flower_flame_offset.y", SnapValue::F32(0x3F000134)),
        ("items.flower_flame_offset.z", SnapValue::F32(0x3F000138)),
        ("items.flower_flame_scale", SnapValue::F32(0x3F00013C)),
        (
            "items.screw_attack_launch_velocity",
            SnapValue::F32(0x3F000140),
        ),
        ("items.unknown_144", SnapValue::F32(0x3F000144)),
        (
            "wall.wall_jump_min_approach_speed",
            SnapValue::F32(0x3F000148),
        ),
        ("ice.damageice_ice_size", SnapValue::F32(0x3F00014C)),
        ("ice.unknown_150", SnapValue::F32(0x3F000150)),
        ("ice.unknown_154", SnapValue::F32(0x3F000154)),
        ("ice.damageicejump_vel_y", SnapValue::F32(0x3F000158)),
        ("ice.damageicejump_vel_x_mult", SnapValue::F32(0x3F00015C)),
        ("size.respawn_platform_scale", SnapValue::F32(0x3F000160)),
        ("size.warp_star_hitbox_scale", SnapValue::F32(0x3F000164)),
        ("size.unknown_168", SnapValue::F32(0x3F000168)),
        ("camera.camera_zoom_target_bone", SnapValue::I64(0x3F00016C)),
        ("camera.zoom_offset.x", SnapValue::F32(0x3F000170)),
        ("camera.zoom_offset.y", SnapValue::F32(0x3F000174)),
        ("camera.zoom_offset.z", SnapValue::F32(0x3F000178)),
        ("camera.damage_camera_y_offset", SnapValue::F32(0x3F00017C)),
        (
            "combat.weight_independent_throws_mask",
            SnapValue::U64(0x000000A5),
        ),
    ];
    assert_eq!(actual.len(), 97);
    for (name, value) in expected {
        assert_eq!(
            actual.iter().find(|(key, _)| key == name).map(|(_, v)| v),
            Some(&value),
            "{name}"
        );
    }
}

#[test]
fn common_attributes_reject_missing_relocation_null_and_truncation() {
    let data = vec![0; 0x188];
    assert!(read_fighter_attributes(&archive(&data, &[], None), 0x184).is_err());
    let mut bad = data.clone();
    word(&mut bad, 0x184, 4);
    assert!(read_fighter_attributes(&archive(&bad, &[], None), 0x184).is_err());
    let short = archive(&data[..0x183], &[], None);
    assert!(FighterAttributes::read(&short, 0).is_err());
    assert!(FighterAttributes::read(&short, u32::MAX).is_err());
    assert!(read_fighter_attributes(&short, u32::MAX).is_err());
}
