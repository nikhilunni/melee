//! grKongo_801D523C (0x801D523C): Ground creation and callback order.
use super::{Kongo, MAP_ORDER};
use crate::last::procs::ProcRegistration;

/// grKg_StageCallbacks gobj procs (s_link 4); map 0 has none.
pub fn map_callback(map: u8) -> Option<(&'static str, u32)> {
    match map {
        0 => None,
        1 | 2 => Some(("grKongo_801D6198", 0x801D6198)),
        3 => Some(("grKongo_801D606C", 0x801D606C)),
        4 => Some(("grKongo_801D5FDC", 0x801D5FDC)),
        5 => Some(("grKongo_801D577C", 0x801D577C)),
        6 => Some(("grKongo_801D6030", 0x801D6030)),
        7..=9 => Some(("grKongo_801D64BC", 0x801D64BC)),
        10 => Some(("grKongo_801D557C", 0x801D557C)),
        11 => Some(("grKongo_801D6668", 0x801D6668)),
        _ => unreachable!("Kongo Jungle map {map}"),
    }
}

impl Kongo {
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
            rows.push(row(4, 5, Some(map), "Ground_801C1D38", 0x801C1D38));
            if let Some((name, address)) = map_callback(map) {
                rows.push(row(4, 5, Some(map), name, address));
            }
        }
        rows.push(row(10, 5, None, "Ground_801C0C2C", 0x801C0C2C));
        rows
    }
}

/// Ground_801C1CD0 for map11 has no postbirth witness. This is a static
/// callback capability boundary, never a tick/progress/invocation-count test.
pub fn require_ground_animation(map: u8) {
    if map == 11 {
        unimplemented!("Ground_801C1CD0: Klaptrap rider Ground Animation after birth");
    }
}
