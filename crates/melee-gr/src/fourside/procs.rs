//! grFourside_801F2D10 (0x801F2D10): Ground creation and callback order.
//! grFourside_801F2DD0 creates maps 0, 4, 6, 2, 1, 3, 5; every row of
//! grFs_StageCallbacks has a gobj proc.
use super::Fourside;
use crate::last::procs::ProcRegistration;

pub const MAP_ORDER: [u8; 7] = [0, 4, 6, 2, 1, 3, 5];
pub const CRANE_MAP: u8 = 1;
pub const HELI_MAP: u8 = 3;
pub const UFO_MAP: u8 = 5;
/// The buildings and the roof (collision joint 0 on bone 10).
pub const COLLISION_MAP: u8 = 6;
/// The roof's collision joint, whose callback counts fighters on it.
pub const ROOF_JOINT: i32 = 0;
/// grFourside_801F3078 (map 6): Ground_801C2FE0, then lb_800115F4.
pub const DYNAMICS_PROC: u32 = 0x801F_3078;

const fn binding(joint_index: i16, target_index: i16, extra: i16) -> crate::desc::JointMapping {
    crate::desc::JointMapping {
        joint_index,
        target_index,
        extra,
    }
}
/// grFs_803E3CE8 (grFs_StageData.joints), each map's rows: Ground_801C2ED0
/// binds collision joint `x` to bone `z` of map `y` (GrFs.dat has no
/// bindings of its own).
pub const CRANE_BINDINGS: [crate::desc::JointMapping; 1] = [binding(3, 1, 7)];
pub const UFO_BINDINGS: [crate::desc::JointMapping; 1] = [binding(4, 5, 1)];
pub const ROOF_BINDINGS: [crate::desc::JointMapping; 1] = [binding(0, 6, 10)];
/// The bindings of `map`.
pub fn bindings(map: u8) -> &'static [crate::desc::JointMapping] {
    match map {
        CRANE_MAP => &CRANE_BINDINGS,
        UFO_MAP => &UFO_BINDINGS,
        COLLISION_MAP => &ROOF_BINDINGS,
        _ => &[],
    }
}

/// grFs_StageCallbacks gobj procs.
pub fn map_callback(map: u8) -> (&'static str, u32) {
    match map {
        0 => ("grFourside_801F2EF0", 0x801F2EF0),
        1 => ("grFourside_801F3274", 0x801F3274),
        2 => ("grFourside_801F2F2C", 0x801F2F2C),
        3 => ("grFourside_801F3CC8", 0x801F3CC8),
        4 => ("grFourside_801F314C", 0x801F314C),
        5 => ("grFourside_801F3894", 0x801F3894),
        6 => ("grFourside_801F3078", DYNAMICS_PROC),
        _ => unreachable!("Fourside map {map}"),
    }
}

impl Fourside {
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
