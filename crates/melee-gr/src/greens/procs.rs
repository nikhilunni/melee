//! grGreens_Init (0x80213460): maps 0, 2, 6, 5, 4 (grGreens_80213524);
//! each block makes a map-1 or map-3 GObj of its own.
use super::{blocks::Blocks, whispy::Whispy, Parameters};
use crate::last::procs::ProcRegistration;
use crate::wind::{Gust, Wind};
use hsd_types::Vec3;

pub const MAP_ORDER: [u8; 5] = [0, 2, 6, 5, 4];
pub const BLOCKS_MAP: u8 = 6;
pub const WHISPY_MAP: u8 = 5;
pub const FACE_MAP: u8 = 4;
/// grGreens_8021479C (map 6): blocks, Ground_801C2FE0, stitching,
/// lb_800115F4.
pub const DYNAMICS_PROC: u32 = 0x8021_479C;

/// grGr_callbacks' gobj procs by map.
pub fn map_callback(map: u8) -> (&'static str, u32) {
    match map {
        0 => ("grGreens_802138B4", 0x8021_38B4),
        1 => ("grGreens_80213944", 0x8021_3944),
        2 => ("grGreens_80213908", 0x8021_3908),
        3 => ("grGreens_80213980", 0x8021_3980),
        4 => ("grGreens_802139BC", 0x8021_39BC),
        5 => ("grGreens_80213C10", 0x8021_3C10),
        6 => ("grGreens_8021479C", DYNAMICS_PROC),
        _ => unreachable!("Green Greens map {map}"),
    }
}

/// The stage's controllers.
#[derive(Clone, Debug)]
pub struct GreensStage {
    pub parameters: Parameters,
    pub blocks: Blocks,
    pub whispy: Whispy,
    /// grGr_804D6AAD: set until the stage starts (fn_80214658).
    pub waiting: bool,
}

impl GreensStage {
    /// fn_80213B1C (0x80213B1C), the type-9 device: Whispy's wind over the
    /// side he blows to, strict bounds.
    pub fn wind(&self) -> Wind {
        let p = &self.parameters;
        let (x_bounds, speed) = match self.whispy.wind {
            1 => ([-p.wind_left, -p.wind_right], -p.wind_speed),
            2 => ([p.wind_right, p.wind_left], p.wind_speed),
            _ => return Wind::CALM,
        };
        Wind::gust(Gust {
            velocity: Vec3::new(speed, 0.0, 0.0),
            x_bounds,
            y_bounds: [p.wind_top, p.wind_bottom],
        })
    }

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
