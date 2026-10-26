//! grInishie2_801FCBC4 (0x801FCBC4): Ground creation and callback order.
//! Maps 0 and 2 exist from the start; the event maps (1, 3, 4, 15) come and
//! go with map 2's timers.
use super::{Carpet, Controller, Lift, Parameters};
use crate::{desc::JointMapping, last::procs::ProcRegistration};

pub const MAP_ORDER: [u8; 2] = [0, 2];
/// The backdrop (grInishie2_801FCDC8: animation 0).
pub const BACKDROP_MAP: u8 = 0;
/// The stage and its timers.
pub const STAGE_MAP: u8 = 2;
/// Pidgit's carpet.
pub const CARPET_MAP: u8 = 1;
/// The lifts out of the waterfall.
pub const LIFT_MAPS: [u8; 2] = [3, 4];
/// Birdo's platform: beyond GrI2.dat's five models, an identity JObj.
pub const BIRDO_MAP: u8 = 15;
/// The event maps, in GObj tag order.
pub const EVENT_MAPS: [u8; 4] = [CARPET_MAP, LIFT_MAPS[0], LIFT_MAPS[1], BIRDO_MAP];
/// grInishie2_801FCFCC (map 2): the timers, the music, lb_800115F4.
pub const DYNAMICS_PROC: u32 = 0x801F_CFCC;
/// The collision joints: the carpet's, each lift's, Birdo's platform's.
pub const CARPET_JOINT: i32 = 0;
pub const LIFT_JOINTS: [i32; 2] = [0xC, 0xD];
pub const BIRDO_JOINT: i32 = 0xE;
/// grInishie2_801FCBC4: every collision joint but 1 starts disabled.
pub const DISABLED_AT_SETUP: [i32; 14] = [0, 0xC, 0xD, 0xE, 3, 4, 5, 6, 7, 8, 9, 0xA, 0xB, 2];
/// The carpet's flying bone (grAnime_801C83D0(gobj, 1, 1), the translation
/// grInishie2_801FD9EC reads) and its root (Ground_801C3FA4(gobj, 0)).
pub const CARPET_FLIGHT_BONE: usize = 1;
pub const CARPET_ROOT_BONE: usize = 0;

const fn joint(joint_index: i16, target_index: i16, extra: i16) -> JointMapping {
    JointMapping {
        joint_index,
        target_index,
        extra,
    }
}
/// grI2_803E4A60 (grI2_StageData.joints). Joints 2..11 sit on maps 5..14,
/// which have no model and are never made.
pub const STAGE_JOINTS: [JointMapping; 14] = [
    joint(0, 1, 1),
    joint(12, 3, 1),
    joint(13, 4, 1),
    joint(14, 15, 0),
    joint(3, 5, 0),
    joint(4, 6, 0),
    joint(5, 7, 0),
    joint(6, 8, 0),
    joint(7, 9, 0),
    joint(8, 10, 0),
    joint(9, 11, 0),
    joint(10, 12, 0),
    joint(11, 13, 0),
    joint(2, 14, 0),
];

/// grI2_StageCallbacks gobj procs.
pub fn map_callback(map: u8) -> (&'static str, u32) {
    match map {
        BACKDROP_MAP => ("grInishie2_801FCDFC", 0x801F_CDFC),
        CARPET_MAP => ("grInishie2_801FDED8", 0x801F_DED8),
        STAGE_MAP => ("grInishie2_801FCFCC", DYNAMICS_PROC),
        3 | 4 => ("grInishie2_801FD224", 0x801F_D224),
        BIRDO_MAP => ("grInishie2_801FD654", 0x801F_D654),
        _ => unreachable!("Mushroom Kingdom II map {map}"),
    }
}

/// The stage: map 2's controller and the live event maps' own Grounds.
#[derive(Clone, Debug)]
pub struct Inishie2Stage {
    pub parameters: Parameters,
    pub controller: Controller,
    /// Maps 3 and 4 while they exist.
    pub lifts: [Option<Lift>; 2],
    /// Map 1 while it exists.
    pub carpet: Option<Carpet>,
    /// Map 15's `xC4`: Birdo, while map 15 exists (the item may be gone).
    pub birdo: Option<Option<u32>>,
    /// Ground_804D6950 per map: Ground_801C4A08 disabled its joints, which
    /// the next Ground_801C2ED0 for it enables again.
    pub released: [bool; 16],
    pub lights: Vec<crate::battle::lights::Light>,
}

impl Inishie2Stage {
    pub fn live(&self, map: u8) -> bool {
        match map {
            CARPET_MAP => self.carpet.is_some(),
            3 | 4 => self.lifts[usize::from(map - 3)].is_some(),
            BIRDO_MAP => self.birdo.is_some(),
            _ => true,
        }
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
            let (name, address) = map_callback(map);
            rows.push(row(4, 5, Some(map), "Ground_801C1D38", 0x801C1D38));
            rows.push(row(4, 5, Some(map), name, address));
        }
        rows.push(row(10, 5, None, "Ground_801C0C2C", 0x801C0C2C));
        rows
    }
}
