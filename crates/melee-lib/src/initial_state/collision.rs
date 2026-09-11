//! Restore CollData history from the initial raw boundary (lb/types.h:202).
use super::{float, vector, word};
use hsd_types::Vec2;
use melee_ft::collision::ground::EnvironmentCollision;
use melee_types::mp::{EcbFlags, FtEcb, SurfaceData};

pub fn restore(environment: &mut EnvironmentCollision, raw: &[u8]) {
    let flags = |b: u8| EcbFlags {
        b0: b & 128 != 0,
        b1234: (b >> 3) & 15,
        b5: b & 4 != 0,
        b6: b & 2 != 0,
        b7: b & 1 != 0,
    };
    let point = |i| Vec2::new(float(raw, i), float(raw, i + 4));
    let diamond = |i| FtEcb {
        top: point(i),
        bottom: point(i + 8),
        right: point(i + 16),
        left: point(i + 24),
    };
    let surface = |i| SurfaceData {
        index: word(raw, i) as i32,
        flags: word(raw, i + 4),
        normal: vector(raw, i + 8),
    };
    let cd = &mut environment.data;
    cd.cur_pos = vector(raw, 0x6F4);
    cd.prev_pos = vector(raw, 0x700);
    cd.last_pos = vector(raw, 0x70C);
    cd.x28_vec = vector(raw, 0x718);
    cd.x34_flags = flags(raw[0x724]);
    cd.x35_flags = flags(raw[0x725]);
    cd.facing_dir = i16::from_be_bytes([raw[0x726], raw[0x727]]);
    cd.x38 = word(raw, 0x728) as i32;
    cd.floor_skip = word(raw, 0x72C) as i32;
    cd.ledge_id_right = word(raw, 0x730) as i32;
    cd.ledge_id_left = word(raw, 0x734) as i32;
    cd.joint_id_skip = word(raw, 0x738) as i32;
    cd.joint_id_only = word(raw, 0x73C) as i32;
    cd.x50 = float(raw, 0x740);
    cd.ledge_snap_x = float(raw, 0x744);
    cd.ledge_snap_y = float(raw, 0x748);
    cd.ledge_snap_height = float(raw, 0x74C);
    cd.lstick_x = float(raw, 0x750);
    cd.x64_ecb = diamond(0x754);
    cd.desired_ecb = diamond(0x774);
    cd.ecb = diamond(0x794);
    cd.prev_ecb = diamond(0x7B4);
    cd.xe4_ecb = diamond(0x7D4);
    // Bone pointers were resolved from Fox's descriptor when allocating.
    assert_eq!(cd.ecb_source.kind, word(raw, 0x7F4) as i32);
    cd.ecb_source.x124 = float(raw, 0x814);
    cd.ecb_source.x128 = float(raw, 0x818);
    cd.ecb_source.x12c = float(raw, 0x81C);
    cd.x130_flags = word(raw, 0x820);
    cd.env_flags = word(raw, 0x824) as i32;
    cd.prev_env_flags = word(raw, 0x828) as i32;
    cd.x13c = word(raw, 0x82C) as i32;
    cd.contact = vector(raw, 0x830);
    cd.floor = surface(0x83C);
    cd.left_facing_wall = surface(0x850);
    cd.right_facing_wall = surface(0x864);
    cd.ceiling = surface(0x878);
    environment.lock_frames = word(raw, 0x88C) as i32;
    environment.collision_flag = raw[0x2223] & 4 != 0;
    environment.teeter_disabled = raw[0x2228] & 32 != 0;
}
