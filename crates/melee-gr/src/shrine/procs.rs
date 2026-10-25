//! grShrine_80201C64 (0x80201C64): Ground creation and callback order.
//! grShrine_80201D20 creates maps 0, 1, 2; Ground_SetupStageCallbacks gives
//! each its row of grSh_StageCallbacks, all three with a gobj proc.
use super::Shrine;
use crate::last::procs::ProcRegistration;

/// grShrine_80201C64's grShrine_80201D20 calls.
pub const MAP_ORDER: [u8; 3] = [0, 1, 2];
/// The collision map: grShrine_80201E9C binds its joints (Ground_801C2ED0).
pub const COLLISION_MAP: u8 = 2;
/// grShrine_80201F1C (map 2): Ground_801C2FE0, then lb_800115F4.
pub const DYNAMICS_PROC: u32 = 0x8020_1F1C;

/// grSh_StageCallbacks gobj procs.
pub fn map_callback(map: u8) -> (&'static str, u32) {
    match map {
        0 => ("grShrine_80201E3C", 0x8020_1E3C),
        1 => ("grShrine_80201E90", 0x8020_1E90),
        2 => ("grShrine_80201F1C", DYNAMICS_PROC),
        _ => unreachable!("Hyrule Temple map {map}"),
    }
}

impl Shrine {
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
