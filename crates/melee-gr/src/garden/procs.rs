//! grGarden_80202B70 (0x80202B70): Ground creation and callback order.
use super::Garden;
use crate::last::procs::ProcRegistration;

/// grGarden_80202C78 creation order: background pieces first, Cranky,
/// the Klaptrap, then the main stage.
pub const MAP_ORDER: [u8; 7] = [0, 4, 5, 6, 1, 3, 2];
pub const CRANKY: u8 = 1;
pub const MAIN: u8 = 2;
pub const KLAPTRAP: u8 = 3;
/// The Klaptrap model's root (the leap's x) and the jaw the bite's item
/// follows (grGarden_802031A4: Ground_801C3FA4(gobj, 8)).
pub const KLAPTRAP_ROOT: usize = 0;
pub const KLAPTRAP_JAW: usize = 8;

/// grGd_StageCallbacks gobj procs (s_link 4). Every row has one.
pub fn map_callback(map: u8) -> (&'static str, u32) {
    match map {
        0 => ("grGarden_80202D94", 0x80202D94),
        1 => ("grGarden_80203098", 0x80203098),
        2 => ("grGarden_80202E48", 0x80202E48),
        3 => ("grGarden_80203250", 0x80203250),
        4 => ("grGarden_80202F3C", 0x80202F3C),
        5 => ("grGarden_80202F9C", 0x80202F9C),
        6 => ("grGarden_80202FFC", 0x80202FFC),
        _ => unreachable!("Jungle Japes map {map}"),
    }
}

impl Garden {
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
