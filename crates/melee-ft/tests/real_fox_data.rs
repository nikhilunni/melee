//! Pinned values measured from the owned NTSC-U 1.02 disc, docs/FOX_DATA.md.
use hsd_archive::{desc::read_public_jobj, Archive};
use melee_ft::desc::common::read_common_data;
use melee_ft::desc::{read_fighter_attributes, read_fighter_bones, read_part_table};
use melee_types::{
    snapshot::{SnapValue, Snapshot},
    FighterKind, FtPart,
};
use std::path::{Path, PathBuf};

fn disc_files() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files");
    if path.is_dir() {
        Some(path)
    } else {
        eprintln!("skipping real Fox data: {} absent", path.display());
        None
    }
}

fn read(dir: &Path, name: &str) -> Archive {
    Archive::parse(&std::fs::read(dir.join(name)).unwrap()).unwrap()
}

#[test]
fn fox_spawn_attributes() {
    let Some(dir) = disc_files() else { return };
    let a = read(&dir, "PlFx.dat");
    let attrs = read_fighter_attributes(&a, a.public("ftDataFox").unwrap()).unwrap();
    let mut values: Vec<(String, SnapValue)> = Vec::new();
    attrs.snapshot(&mut values);
    for (name, value) in &values {
        println!("{name}: {value:?}");
    }
    let expected = [
        (
            "walk acceleration multiplier",
            attrs.walking.walk_accel_mul,
            0x3E4C_CCCD,
        ),
        (
            "walk acceleration base",
            attrs.walking.walk_accel_base,
            0x3DCC_CCCD,
        ),
        (
            "walk maximum speed",
            attrs.walking.walk_max_vel,
            0x3FCC_CCCD,
        ),
        ("ground friction", attrs.ground.ground_friction, 0x3DA3_D70A),
        (
            "initial dash speed",
            attrs.running.dash_initial_velocity,
            0x3FF3_3333,
        ),
        (
            "maximum dash speed",
            attrs.running.dash_max_velocity,
            0x400C_CCCD,
        ),
        (
            "ground speed cap",
            attrs.ground.ground_max_horizontal_velocity,
            0x4040_0000,
        ),
        ("jump startup", attrs.jumping.jump_startup_time, 0x4040_0000),
        (
            "jump vertical speed",
            attrs.jumping.jump_v_initial_velocity,
            0x406B_851F,
        ),
        (
            "short hop speed",
            attrs.jumping.hop_v_initial_velocity,
            0x4006_6666,
        ),
        ("gravity", attrs.air.gravity, 0x3E6B_851F),
        (
            "terminal velocity",
            attrs.air.terminal_velocity,
            0x4033_3333,
        ),
        ("air drift cap", attrs.air.air_drift_max, 0x3F54_7AE1),
        ("fast fall speed", attrs.air.fast_fall_velocity, 0x4059_999A),
        ("weight", attrs.size.weight, 0x4296_0000),
        ("model scale", attrs.size.model_scaling, 0x3F75_C28F),
        ("shield size", attrs.shield.initial_shield_size, 0x4166_0000),
        ("landing lag", attrs.landing.normal_landing_lag, 0x4080_0000),
    ];
    for (name, value, bits) in expected {
        assert_eq!(value.to_bits(), bits, "{name}");
    }
    assert_eq!(attrs.jumping.max_jumps, 2);
    assert_eq!(attrs.combat.rapid_jab_window, 4);
    assert_eq!(attrs.combat.weight_independent_throws_mask, 9);
}

#[test]
fn fox_ecb_bones() {
    let Some(dir) = disc_files() else { return };
    let common = read(&dir, "PlCo.dat");
    let parts = read_part_table(&common, FighterKind::Fox, 54).unwrap();
    let skeleton = read(&dir, "PlFxNr.dat");
    let root = read_public_jobj(&skeleton, "PlyFox5K_Share_joint").unwrap();
    assert_eq!(root.descendants().len(), 73);
    assert_eq!(parts.joint_count(), 73);
    let forward = [
        0, 1, 2, 3, 4, 255, 5, 6, 7, 8, 9, 11, 12, 13, 14, 15, 21, 22, 23, 24, 25, 26, 27, 28, 29,
        30, 31, 32, 33, 34, 35, 255, 37, 38, 40, 41, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
        64, 65, 67, 68, 69, 71, 72,
    ];
    assert_eq!(
        parts
            .part_to_joint
            .iter()
            .map(|v| v.unwrap_or(255))
            .collect::<Vec<_>>(),
        forward
    );
    for (part, joint) in forward
        .into_iter()
        .enumerate()
        .filter(|(_, joint)| *joint != 255)
    {
        assert_eq!(parts.joint_to_part[joint as usize], Some(part as u8));
    }
    for (part, joint) in [
        (FtPart::TopN, 0),
        (FtPart::TransN, 1),
        (FtPart::XRotN, 2),
        (FtPart::YRotN, 3),
    ] {
        assert_eq!(parts.joint(part), Some(joint));
    }
    assert_eq!(parts.joint(FtPart::WaistN), None);
    assert_eq!(parts.joint(FtPart::LThumbNb), None);
    assert_eq!(parts.joint(FtPart::Unknown56), None);
    assert_eq!(parts.joint(FtPart::Unknown109), None);
    let a = read(&dir, "PlFx.dat");
    let b = read_fighter_bones(&a, a.public("ftDataFox").unwrap(), 5).unwrap();
    assert_eq!(b.ecb.joints, [41, 55, 25, 13, 7, 4]);
    assert!(b
        .ecb
        .joints
        .iter()
        .all(|&v| v >= 0 && (v as usize) < parts.joint_count()));
    for (value, bits) in [
        (b.ecb.center_y, 0),
        (b.ecb.ledge_snap_x, 0x4130_0000),
        (b.ecb.ledge_snap_y, 0x4150_0000),
        (b.ecb.ledge_snap_height, 0x4110_0000),
    ] {
        assert_eq!(value.to_bits(), bits);
    }
    assert_eq!(
        [
            b.model.animation_translation,
            b.model.shield,
            b.model.held_item,
            b.model.left_foot,
            b.model.right_foot
        ],
        [67, 71, 41, 9, 15]
    );
    let animation_sets: [(u16, &[u8]); 5] = [
        (27, &[27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38]),
        (57, &[57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69]),
        (41, &[51]),
        (41, &[42, 43, 44, 45]),
        (41, &[46, 47, 48, 49]),
    ];
    for (actual, (root, joints)) in b.animation_sets.iter().zip(animation_sets) {
        let actual = actual.as_ref().unwrap();
        assert_eq!(actual.root_joint, root);
        assert_eq!(actual.joints, joints);
    }
    assert_eq!(b.dynamics_roots, [17]);
    assert_eq!(b.dynamics_collision, [41]);
    assert_eq!(
        b.hurtboxes,
        [4, 22, 41, 41, 55, 25, 56, 26, 12, 6, 13, 7, 18]
    );
    assert_eq!(b.scaled_joint, Some(4));
    assert_eq!(b.attachment_joint, Some(4));
    let pose = b.ground_pose.unwrap();
    assert_eq!(pose.right_leg, [12, 13, 15]);
    assert_eq!(pose.left_leg, [6, 7, 9]);
    assert_eq!(pose.upper_length.to_bits(), 0x404A_E148);
    assert_eq!(pose.lower_length.to_bits(), 0x4028_F5C3);
    assert_eq!(pose.foot_extension.to_bits(), 0x3FB4_7AE1);
}

#[test]
fn common_idle_attributes_match_disc_bits() {
    let Some(dir) = disc_files() else { return };
    let c = read_common_data(&read(&dir, "PlCo.dat")).unwrap();
    for (value, bits) in [
        (c.input.horizontal_stick_deadzone, 0x3E8F_5C29),
        (c.input.vertical_stick_deadzone, 0x3E8F_5C29),
        (c.input.horizontal_stick_smash_deadzone, 0x3E80_0000),
        (c.input.vertical_stick_smash_deadzone, 0x3E80_0000),
        (c.input.analog_shoulder_deadzone, 0x3E99_999A),
        (c.input.z_press_analog_value, 0x3EB3_3333),
        (c.input.shield_press_threshold, 0x3E80_0000),
        (c.input.walk_stick_threshold, 0x3E38_51EC),
        (c.input.turn_stick_threshold, 0xBE80_0000),
        (c.input.dash_smash_stick_threshold, 0x3F4C_CCCD),
        (c.input.tap_jump_threshold, 0x3F29_999A),
        (c.input.squat_stick_threshold, 0x3F30_0000),
        (c.friction_when_above_walk_speed, 0x4000_0000),
        (c.ground_knockback_speed_limit, 0x4104_CCCD),
        (c.ledge_snap_height_multiplier, 0x3F19_999A),
        (c.ground_pose_max_angle_degrees, 0x41A0_0000),
    ] {
        assert_eq!(value.to_bits(), bits);
    }
    assert_eq!(c.input.dash_smash_window, 2);
    assert_eq!(c.input.tap_jump_window, 4);
}
