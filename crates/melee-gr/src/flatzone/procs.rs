//! grFlatzone_80216E78 (0x80216E78): Ground creation and callback order.
//! Every created map's StageCallbacks row has a gobj proc, so each runs
//! Ground_801C1D38 and then its own proc at s_link 4.
use super::FlatZone;
use crate::last::procs::ProcRegistration;

/// grFlatzone_80216E78's grFlatzone_80216F48 calls (map 7 is never made).
pub const MAP_ORDER: [u8; 8] = [0, 1, 2, 3, 4, 5, 6, 8];
/// grFlatzone_802171D4 (map 2): the controller; it ends with lb_800115F4.
pub const CONTROLLER_PROC: u32 = 0x8021_71D4;
/// mpLib_800587FC (0x800587FC), the collision GObj's proc (p_link 6,
/// s_link 4): mpLib_80058614_Floor, then grDynamicAttr_801CA224 ages the
/// spills.
pub const COLLISION_PROC: u32 = 0x8005_87FC;

/// grFz_StageCallbacks gobj procs.
pub fn map_callback(map: u8) -> (&'static str, u32) {
    match map {
        0 => ("grFlatzone_80217064", 0x8021_7064),
        1 => ("grFlatzone_80217438", 0x8021_7438),
        2 => ("grFlatzone_802171D4", CONTROLLER_PROC),
        3 => ("grFlatzone_802176BC", 0x8021_76BC),
        4 => ("grFlatzone_80217D30", 0x8021_7D30),
        5 => ("grFlatzone_802174EC", 0x8021_74EC),
        6 => ("grFlatzone_80217E04", 0x8021_7E04),
        8 => ("grFlatzone_80217EF0", 0x8021_7EF0),
        _ => unreachable!("Flat Zone map {map}"),
    }
}

impl FlatZone {
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
        rows.push(row(4, 6, None, "mpLib_800587FC", COLLISION_PROC));
        rows.push(row(10, 5, None, "Ground_801C0C2C", 0x801C0C2C));
        rows
    }
}
