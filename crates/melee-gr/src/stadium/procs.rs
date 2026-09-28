//! `grStadium_OnInit` (0x801D10F8 for maps 0, 1, 2; map 2 creates map 5
//! inside its init): Ground creation and callback order, and the collision
//! set-up around it.
use super::Stadium;
use crate::desc::JointMapping;
use crate::last::procs::ProcRegistration;

/// Ground list order at the start boundary.
pub const MAP_ORDER: [u8; 4] = [0, 1, 2, 5];
/// The screen's gobj proc.
pub const SCREEN_CALLBACK: u32 = 0x801D_1390;
/// The controller's gobj proc.
pub const CONTROLLER_CALLBACK: u32 = 0x801D_1520;
/// The base arena's gobj proc (Ground_801C2FE0 only).
pub const BASE_CALLBACK: u32 = 0x801D_1604;

/// `grPs_803E1248`: collision joint, map and descendant index. Collision
/// joint 0 (the water form's windmill) is bound by GrPs3.dat's own table.
pub const STAGE_JOINTS: [JointMapping; 6] = [
    joint(1, 3),
    joint(2, 3),
    joint(3, 4),
    joint(4, 5),
    joint(5, 6),
    joint(7, 9),
];
const fn joint(collision: i16, map: i16) -> JointMapping {
    JointMapping {
        joint_index: collision,
        target_index: map,
        extra: 0,
    }
}
/// The collision joints `grStadium_OnInit` disables (mpLib_80057BC0), in
/// order: every form's but the base's (4) and the permanent frame's (6).
pub const DISABLED_AT_INIT: [i32; 6] = [1, 2, 3, 5, 7, 0];
/// `mpLib_800581DC(6, 4)`: the frame and the base arena share vertices.
pub const STITCHED_AT_INIT: (i32, i32) = (6, 4);
/// Lines `grStadium_801D13E0` disables while the base form stands and the
/// transformation enables while the arena is sunk (TODO(meaning)).
pub const PIT_LINES: [i32; 2] = [0x55, 0x6F];

/// A map's gobj proc (grPs_StageCallbacks[map].gobj_proc).
pub fn map_callback(map: u8) -> u32 {
    match map {
        1 => SCREEN_CALLBACK,
        2 => CONTROLLER_CALLBACK,
        3 => 0x801D_19D8,
        4 => 0x801D_16DC,
        5 => BASE_CALLBACK,
        6 => 0x801D_17E8,
        7 | 8 => 0x801D_1E18,
        9 => 0x801D_1B48,
        _ => unreachable!("Pokemon Stadium map {map} has no gobj proc"),
    }
}

/// The stage's joints bound to `map`'s descendants.
pub fn stage_joints(map: u8) -> impl Iterator<Item = &'static JointMapping> {
    STAGE_JOINTS
        .iter()
        .filter(move |j| j.target_index == i16::from(map))
}

impl Stadium {
    pub fn proc_table(&self) -> Vec<ProcRegistration> {
        let row = |s_link, p_link, map_id, callback, address| ProcRegistration {
            s_link,
            p_link,
            p_priority: 0,
            map_id,
            callback,
            address,
        };
        let mut rows = vec![
            row(0, 3, None, "Ground_801C461C", 0x801C461C),
            row(0, 4, None, "fn_801CADBC", 0x801CADBC),
        ];
        for map in MAP_ORDER {
            rows.push(row(1, 5, Some(map), "Ground_801C1CD0", 0x801C1CD0));
        }
        rows.push(row(4, 5, Some(0), "Ground_801C1D38", 0x801C1D38));
        for (map, name, address) in [
            (1, "grStadium_801D1390", SCREEN_CALLBACK),
            (2, "grStadium_801D1520", CONTROLLER_CALLBACK),
            (5, "grStadium_801D1604", BASE_CALLBACK),
        ] {
            rows.push(row(4, 5, Some(map), "Ground_801C1D38", 0x801C1D38));
            rows.push(row(4, 5, Some(map), name, address));
        }
        rows.push(row(10, 5, None, "Ground_801C0C2C", 0x801C0C2C));
        rows
    }
}
