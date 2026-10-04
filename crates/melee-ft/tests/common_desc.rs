mod support;

use melee_ft::desc::common::{read_common_data, CommonFighterData};
use support::{archive, word};

#[test]
fn common_data_uses_root_slot_zero_and_reads_only_selected_fields() {
    let mut data = vec![0; 0x818];
    for offset in (0..0x814).step_by(4) {
        word(&mut data, offset, 0x3F00_0000 | offset as u32);
    }
    word(&mut data, 0x40, (-3_i32) as u32);
    word(&mut data, 0x74, 7);
    let a = archive(&data, &[0x814], Some(("ftLoadCommonData", 0x814)));
    let c = read_common_data(&a).unwrap();
    let fields = [
        (c.input.horizontal_stick_deadzone, 0x000),
        (c.input.vertical_stick_deadzone, 0x004),
        (c.input.horizontal_stick_smash_deadzone, 0x008),
        (c.input.vertical_stick_smash_deadzone, 0x00C),
        (c.input.analog_shoulder_deadzone, 0x010),
        (c.input.z_press_analog_value, 0x014),
        (c.input.shield_press_threshold, 0x018),
        (c.input.walk_stick_threshold, 0x024),
        (c.input.turn_stick_threshold, 0x034),
        (c.input.dash_smash_stick_threshold, 0x03C),
        (c.friction_when_above_walk_speed, 0x06C),
        (c.input.tap_jump_threshold, 0x070),
        (c.input.squat_stick_threshold, 0x090),
        (c.ground_knockback_speed_limit, 0x164),
        (c.ledge_snap_height_multiplier, 0x1CC),
        (c.ground_pose_max_angle_degrees, 0x804),
        (c.pinned_hip_offset.x, 0x808),
        (c.pinned_hip_offset.y, 0x80C),
        (c.pinned_hip_offset.z, 0x810),
    ];
    for (value, offset) in fields {
        assert_eq!(value.to_bits(), 0x3F00_0000 | offset);
    }
    assert_eq!(c.input.dash_smash_window, -3);
    assert_eq!(c.input.tap_jump_window, 7);
}

#[test]
fn common_data_rejects_missing_public_null_and_truncated_blocks() {
    let data = vec![0; 0x1D4];
    assert!(read_common_data(&archive(&data, &[], None)).is_err());
    assert!(read_common_data(&archive(&data, &[], Some(("ftLoadCommonData", 0x1D0)))).is_err());
    assert!(CommonFighterData::read(&archive(&data[..0x1CF], &[], None), 0).is_err());
    let mut data = data;
    word(&mut data, 0x1D0, 4);
    assert!(read_common_data(&archive(&data, &[], Some(("ftLoadCommonData", 0x1D0)))).is_err());
}
