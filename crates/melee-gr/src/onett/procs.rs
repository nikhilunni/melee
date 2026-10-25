//! grOnett_801E3738 (0x801E3738): Ground creation and callback order. Maps
//! 0, 2, 5, 4, 3 (grOnett_801E37F4); map 0's row of grOt_803E27E0 is
//! empty, so it has no gobj proc.
use super::{cars::Cars, Awning, Parameters, Roof, Smoke};
use crate::last::procs::ProcRegistration;

pub const MAP_ORDER: [u8; 5] = [0, 2, 5, 4, 3];
/// The street: awnings, smoke, the lane A camera subject.
pub const STREET_MAP: u8 = 5;
/// The drugstore roof.
pub const ROOF_MAP: u8 = 4;
/// The cars.
pub const CAR_MAP: u8 = 3;
/// grOnett_801E3C60 (map 5): awnings, smoke, the subject, lb_800115F4,
/// Ground_801C2FE0.
pub const DYNAMICS_PROC: u32 = 0x801E_3C60;
/// The awnings' bones and collision joints (grOnett_801E3A34,
/// mpJointSetCb1(0 / 1)).
pub const AWNING_BONES: [usize; 2] = [0xE, 0xF];
pub const AWNING_JOINTS: [i32; 2] = [0, 1];
/// The roof's collision joints (mpJointSetCb1(3 / 4)).
pub const ROOF_JOINTS: [i32; 2] = [3, 4];
/// The roof bones grOnett_801E3DA0 hides while it blinks.
pub const ROOF_BLINK_BONES: [usize; 3] = [5, 2, 4];
/// The cars' bones and their smoke anchors (grOnett_801E41C8).
pub const CAR_BONES: [usize; 4] = [0xB, 0x1, 0x6, 0x10];
pub const CAR_SMOKE_BONES: [usize; 4] = [0xF, 0x5, 0xA, 0x14];
/// grOnett_801E5538: the chimney bone and the smoke's offset on it.
pub const CHIMNEY_BONE: usize = 6;
pub const CHIMNEY_OFFSET: [f32; 3] = [15.75, 40.0, 28.25];
/// grLib_801C96F8(0x7530, 0x1E): the chimney's generator; grLib_801C9808
/// (0x7533, 0x1E): an awning's puff; grLib_801C9808(0x2C, 0): a struck
/// car's smoke (the common bank).
pub const SMOKE_PARTICLE: (u32, u8) = (0x7530, 0x1E);
pub const AWNING_PARTICLE: (u32, u8) = (0x7533, 0x1E);
pub const CAR_PARTICLE: (u32, u8) = (0x2C, 0);
/// grOnett_801E3930: lane A's camera subject.
pub const SUBJECT_EXTENTS: [f32; 4] = [-40.0, 40.0, 20.0, -20.0];
pub const SUBJECT_DEPTH: f32 = 1.0;

/// grOt_803E27E0 gobj procs (map 0 has none).
pub fn map_callback(map: u8) -> Option<(&'static str, u32)> {
    match map {
        0 => None,
        2 => Some(("grOnett_801E3928", 0x801E_3928)),
        5 => Some(("grOnett_801E3C60", DYNAMICS_PROC)),
        4 => Some(("grOnett_801E3DA0", 0x801E_3DA0)),
        3 => Some(("grOnett_801E43E0", 0x801E_43E0)),
        _ => unreachable!("Onett map {map}"),
    }
}

/// The stage's controllers.
#[derive(Clone, Debug)]
pub struct OnettStage {
    pub parameters: Parameters,
    pub awnings: [Awning; 2],
    pub smoke: Smoke,
    /// map 5's `subject`: made by the deferred grOnett_801E3930.
    pub subject: Option<melee_cm::Subject>,
    /// stage_info.x740 (Ground_801C5784): lane A's phase.
    pub camera_phase: i32,
    pub roof: Roof,
    pub cars: Cars,
    /// Each car's stage material item.
    pub car_items: [Option<u32>; 4],
    /// The chimney's generator while it puffs.
    pub smoke_generator: Option<usize>,
    pub lights: Vec<crate::battle::lights::Light>,
}

impl OnettStage {
    pub fn car_of_item(&self, item: u32) -> Option<usize> {
        self.car_items.iter().position(|&i| i == Some(item))
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
            if let Some((name, address)) = map_callback(map) {
                rows.push(row(4, 5, Some(map), name, address));
            }
        }
        rows.push(row(10, 5, None, "Ground_801C0C2C", 0x801C0C2C));
        rows
    }
}
