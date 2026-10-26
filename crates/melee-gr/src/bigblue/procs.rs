//! grBigBlue_801E57C0 (0x801E57C0): Ground creation and callback order.
use super::{cars::Cars, flyers::Flyers, road::Road, Parameters};
use crate::last::procs::ProcRegistration;
use hsd_types::Vec3;
use melee_lb::radial_force::DirectionalGust;

/// The revival markers' wrapper (grBigBlue_801E5B20).
pub const MARKERS_MAP: u8 = 0x1F;
/// The scenery the road carries (grBigBlue_801E6298, 801E6200).
pub const SCENERY_MAPS: [u8; 2] = [1, 2];
/// The road (grBigBlue_801E613C).
pub const ROAD_MAP: u8 = 0x22;
/// The cars (grBigBlue_801E6364); map 4 is their models, created and
/// released inside its on_init.
pub const CARS_MAP: u8 = 0x21;
pub const CAR_MODELS_MAP: u8 = 4;
/// The small flyers (grBigBlue_801E6904), hidden after creation.
pub const FLYERS_MAP: u8 = 0x20;
/// The Falcon Flyer (grBigBlue_801E8D64).
pub const FALCON_MAP: u8 = 0x23;
/// The route platform (grBigBlue_801E9F3C), hidden after creation.
pub const ROUTE_MAP: u8 = 0x24;
/// GObj creation order.
pub const MAP_ORDER: [u8; 9] = [
    MARKERS_MAP,
    0,
    1,
    2,
    ROAD_MAP,
    CARS_MAP,
    FLYERS_MAP,
    FALCON_MAP,
    ROUTE_MAP,
];

/// grBb_StageCallbacks' gobj procs.
pub const ROAD_PROC: u32 = 0x801E_61C4;
pub const CARS_PROC: u32 = 0x801E_6884;
pub const FLYERS_PROC: u32 = 0x801E_6C60;
pub const FALCON_PROC: u32 = 0x801E_93D8;
pub const ROUTE_PROC: u32 = 0x801E_A05C;

pub fn map_callback(map: u8) -> (&'static str, u32) {
    match map {
        MARKERS_MAP => ("grBigBlue_801E611C", 0x801E_611C),
        0 => ("grBigBlue_801E5B18", 0x801E_5B18),
        1 => ("grBigBlue_801E635C", 0x801E_635C),
        2 => ("grBigBlue_801E6290", 0x801E_6290),
        ROAD_MAP => ("grBigBlue_801E61C4", ROAD_PROC),
        CARS_MAP => ("grBigBlue_801E6884", CARS_PROC),
        FLYERS_MAP => ("grBigBlue_801E6C60", FLYERS_PROC),
        FALCON_MAP => ("grBigBlue_801E93D8", FALCON_PROC),
        ROUTE_MAP => ("grBigBlue_801EA05C", ROUTE_PROC),
        _ => unreachable!("Big Blue map {map}"),
    }
}

/// grBb_803E2938 (grBb_StageData.joints): 28 more of the road's collision
/// joints, on its segment bones.
pub const ROAD_BINDINGS: [crate::desc::JointMapping; 28] = [
    road(36, 10),
    road(37, 10),
    road(38, 7),
    road(39, 7),
    road(40, 7),
    road(41, 14),
    road(42, 14),
    road(43, 14),
    road(44, 15),
    road(45, 15),
    road(46, 15),
    road(47, 16),
    road(48, 16),
    road(49, 16),
    road(52, 32),
    road(53, 32),
    road(54, 32),
    road(55, 35),
    road(56, 35),
    road(57, 35),
    road(58, 38),
    road(59, 38),
    road(60, 38),
    road(61, 41),
    road(62, 41),
    road(63, 41),
    road(64, 44),
    road(65, 44),
];
const fn road(joint_index: i16, extra: i16) -> crate::desc::JointMapping {
    crate::desc::JointMapping {
        joint_index,
        target_index: ROAD_MAP as i16,
        extra,
    }
}

/// The road's joints grBigBlue_801EC58C searches (grBb_803B8134) and
/// grBigBlue_801EF844 names: 33 and 35..65.
pub const ROAD_JOINTS: [i32; 32] = [
    33, 35, 38, 39, 40, 36, 37, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57,
    58, 59, 60, 61, 62, 63, 64, 65,
];
/// mpLib_80058044 (B11) on 0x21..0x41 at the creation's end.
pub const B11_JOINTS: std::ops::RangeInclusive<i32> = 0x21..=0x41;
/// The small flyers' collision joints (fn_801E8560).
pub const FLYER_JOINTS: [i32; 3] = [0, 1, 2];

/// grBigBlue_801E57C0's lb_80011A50(&{-1, 0, 0}, -1, 1.0, 0.0, M_PI / 3,
/// -100000, 100000, 100000, -100000): a wind against the race that never
/// ends.
pub fn wind() -> DirectionalGust {
    DirectionalGust {
        direction: Vec3::new(-1.0, 0.0, 0.0),
        frames: -1,
        strength: 1.0,
        decay: 0.0,
        phase_step: (core::f64::consts::PI / 3.0) as f32,
        rectangle: [-100_000.0, 100_000.0, 100_000.0, -100_000.0],
    }
}

/// grBigBlue_801EF7D8 (0x801EF7D8) through grLib_801C9E60: the stage's
/// drift, (-10, 0, 0), which airborne items and the CPU read.
pub const DRIFT: Vec3 = Vec3::new(-10.0, 0.0, 0.0);

/// The stage's controllers.
#[derive(Clone)]
pub struct BigBlueStage {
    pub parameters: Parameters,
    pub road: Road,
    pub cars: Cars,
    pub flyers: Flyers,
}

impl BigBlueStage {
    /// grBigBlue_801E59F8 registers each map's s_link 1 and 4 Ground procs
    /// in Ground_GetStageGObj, then its own after its on_init.
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
            rows.push(row(4, 5, Some(map), "Ground_801C1D38", 0x801C1D38));
            let (name, address) = map_callback(map);
            rows.push(row(4, 5, Some(map), name, address));
        }
        rows.push(row(10, 5, None, "Ground_801C0C2C", 0x801C0C2C));
        rows
    }
}
