//! grOldYoshi_8020E79C (0x8020E79C): Ground creation and callback order.
//! Every map's StageCallbacks row has a gobj proc, so each map runs
//! Ground_801C1D38 and then its own proc at s_link 4.
use super::OldYoshi;
use crate::last::procs::ProcRegistration;

/// grOldYoshi_8020E79C's grOldYoshi_8020E854 calls.
pub const MAP_ORDER: [u8; 6] = [0, 1, 4, 5, 2, 3];
/// The clouds' map and the guest's.
pub const CLOUD_MAP: u8 = 2;
pub const GUEST_MAP: u8 = 3;
/// grOldYoshi_8020EA64 (map 4): Ground_801C2FE0, then lb_800115F4.
pub const DYNAMICS_PROC: u32 = 0x8020_EA64;

/// grOy_StageCallbacks gobj procs.
pub fn map_callback(map: u8) -> (&'static str, u32) {
    match map {
        0 => ("grOldYoshi_8020E970", 0x8020E970),
        1 => ("grOldYoshi_8020E9E8", 0x8020E9E8),
        2 => ("grOldYoshi_8020EC10", 0x8020EC10),
        3 => ("grOldYoshi_8020F088", 0x8020F088),
        4 => ("grOldYoshi_8020EA64", DYNAMICS_PROC),
        5 => ("grOldYoshi_8020EAF4", 0x8020EAF4),
        _ => unreachable!("Yoshi's Island N64 map {map}"),
    }
}

impl OldYoshi {
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
