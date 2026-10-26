//! grVenom_80203B18 (0x80203B18): Ground creation and callback order.
//! Maps 4, 0, 5, 9 and 7 are created in that order; an Arwing's path,
//! ship and platform follow them in the GObj list when they come (each
//! s_link's procs run in GObj order).
use super::{
    arwings::{Platform, Ship, Spawner},
    sequence::Sequence,
    Parameters,
};
use crate::last::procs::ProcRegistration;

pub const MAP_ORDER: [u8; 5] = [4, 0, 5, 9, 7];
/// The Great Fox: collision joints 0 and 1, the spawner.
pub const SHIP_MAP: u8 = 5;
/// The tunnel floor's identity root (collision joint 2).
pub const TUNNEL_MAP: u8 = 9;
/// The flight's sequence.
pub const FLIGHT_MAP: u8 = 7;
/// Carries the markers; map 5's proc copies its translation here.
pub const MARKER_MAP: u8 = 0;
/// grVenom_80204284: map 5's proc (the spawner, lb_800115F4,
/// Ground_801C2FE0).
pub const DYNAMICS_PROC: u32 = 0x8020_4284;

const fn binding(joint_index: i16, target_index: i16, extra: i16) -> crate::desc::JointMapping {
    crate::desc::JointMapping {
        joint_index,
        target_index,
        extra,
    }
}
/// grVe_803E5348.joints (grVe_StageData.joints): collision joints 0 and 1
/// on map 5's root, 2 on map 9's, 4 on map 10's and 3 on map 13's.
pub const BINDINGS: [crate::desc::JointMapping; 5] = [
    binding(0, 5, 0),
    binding(1, 5, 0),
    binding(2, 9, 0),
    binding(4, 10, 0),
    binding(3, 13, 0),
];

pub fn bindings(map: u8) -> impl Iterator<Item = &'static crate::desc::JointMapping> {
    BINDINGS
        .iter()
        .filter(move |b| b.target_index == i16::from(map))
}

/// grVe_StageCallbacks gobj procs (s_link 4). Every map has one, the
/// empty ones included.
pub fn map_callback(map: u8) -> (&'static str, u32) {
    match map {
        0 => ("grVenom_80203FCC", 0x8020_3FCC),
        2 => ("grVenom_802053B0", 0x8020_53B0),
        3 => ("grVenom_80205F30", 0x8020_5F30),
        4 => ("grVenom_802040AC", 0x8020_40AC),
        5 => ("grVenom_80204284", DYNAMICS_PROC),
        6 => ("grVenom_80206B70", 0x8020_6B70),
        7 => ("grVenom_8020454C", 0x8020_454C),
        9 => ("grVenom_80204EFC", 0x8020_4EFC),
        10..=13 => ("grVenom_80205758", 0x8020_5758),
        _ => unreachable!("Venom map {map}"),
    }
}

/// A pass while any of its maps lives.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flight {
    pub kind: i32,
    pub group: i32,
    /// The path lives (grVe_803E5348.arwing.arwing_gobj[slot] != NULL);
    /// the ship and platform go in their next procs once it is gone.
    pub path: bool,
    /// The path's first tick (`u.venom.xD4`): its animation starts then.
    pub path_starting: bool,
    /// The ship's map (3 or 6) and controller.
    pub ship_map: u8,
    pub ship: Option<Ship>,
    /// A near pass's platform map (10 or 13) and controller.
    pub platform_map: u8,
    pub platform: Option<Platform>,
}

/// The stage's controllers.
#[derive(Clone, Debug)]
pub struct VenomStage {
    pub parameters: Parameters,
    pub spawner: Spawner,
    pub sequence: Sequence,
    /// Slot 0's pass.
    pub flight: Option<Flight>,
    /// A laser the ship's proc fired, for the scene to spawn
    /// (it_802E7654) before any other proc runs.
    pub pending_shot: Option<PendingShot>,
}

/// it_802E7654's arguments from the ship's proc.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PendingShot {
    /// The gun bone's world position.
    pub gun: hsd_types::Vec3,
    pub target: hsd_types::Vec3,
    /// 1 an Arwing's laser, 3 a Wolfen's.
    pub laser: i32,
}

impl VenomStage {
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
            let (name, address) = map_callback(map);
            rows.push(row(4, 5, Some(map), name, address));
        }
        rows.push(row(10, 5, None, "Ground_801C0C2C", 0x801C0C2C));
        rows
    }
}
