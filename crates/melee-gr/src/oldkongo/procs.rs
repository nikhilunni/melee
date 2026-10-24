//! grOldKongo_8020F46C (0x8020F46C): Ground creation and callback order.
//! Every map's StageCallbacks row has a gobj proc.
use super::OldKongo;
use crate::last::procs::ProcRegistration;

/// grOldKongo_8020F46C's setupStageCallbacks calls.
pub const MAP_ORDER: [u8; 4] = [0, 3, 1, 2];
pub const BARREL_MAP: u8 = 1;
pub const BIRD_MAP: u8 = 2;
/// Map 3 carries the platforms (collision joints 0 and 1).
pub const PLATFORM_MAP: u8 = 3;
/// stageGObj3_GObjProc (map 3): Ground_801C2FE0, then lb_800115F4.
pub const DYNAMICS_PROC: u32 = 0x8020_F6BC;

/// grOk_803E6580: collision joints 0 and 1 follow map 3's descendants 1, 2.
pub const COLLISION_BINDINGS: [crate::desc::JointMapping; 2] = [
    crate::desc::JointMapping {
        joint_index: 0,
        target_index: 3,
        extra: 1,
    },
    crate::desc::JointMapping {
        joint_index: 1,
        target_index: 3,
        extra: 2,
    },
];

/// stage_callbacks gobj procs.
pub fn map_callback(map: u8) -> (&'static str, u32) {
    match map {
        0 => ("stageGObj0_GObjProc", 0x8020F64C),
        1 => ("stageGObj1_GObjProc", 0x8020F888),
        2 => ("stageGObj2_GObjProc", 0x802100FC),
        3 => ("stageGObj3_GObjProc", DYNAMICS_PROC),
        _ => unreachable!("Kongo Jungle N64 map {map}"),
    }
}

impl OldKongo {
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
        for map in MAP_ORDER {
            let (name, address) = map_callback(map);
            rows.push(row(4, 5, Some(map), "Ground_801C1D38", 0x801C1D38));
            rows.push(row(4, 5, Some(map), name, address));
        }
        rows.push(row(10, 5, None, "Ground_801C0C2C", 0x801C0C2C));
        rows
    }
}
