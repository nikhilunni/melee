//! grZebes_801D84A4 (0x801D84A4): Ground creation and callback order.
//! grZebes_801D8558 creates maps 0, 1, 6, 4, 8; map 6's creation
//! (grZebes_801D8644) first creates map 7, then the bubbles' map 3 models,
//! so the GObj list (and each s_link's proc order) is 0, 1, 6, 7, the
//! bubbles, 4, 8. The bubbles' models and the splashes (map 5) have no
//! state their procs change: they are not in the table.
use super::{
    acid::Acid,
    bubbles::Bubbles,
    island::Island,
    plates::Plate,
    scenery::{AcidReaction, Ornament},
    Parameters,
};
use crate::last::procs::ProcRegistration;

pub const MAP_ORDER: [u8; 6] = [0, 1, 6, 7, 4, 8];
/// Map 1 shakes with the acid.
pub const REACTION_MAP: u8 = 1;
/// The pulse model, hung on a bubble.
pub const PULSE_MAP: u8 = 2;
/// A bubble's model.
pub const BUBBLE_MAP: u8 = 3;
/// Rides the top platform.
pub const ORNAMENT_MAP: u8 = 4;
/// A splash where a fighter enters the acid.
pub const SPLASH_MAP: u8 = 5;
/// The left part, its top platform and plate.
pub const LEFT_MAP: u8 = 6;
/// The right part and plate.
pub const RIGHT_MAP: u8 = 7;
/// The acid.
pub const ACID_MAP: u8 = 8;
/// grZebes_801D881C (map 6): the plate, the bubbles, the platform, joint 0,
/// Ground_801C2FE0, lb_800115F4.
pub const DYNAMICS_PROC: u32 = 0x801D_881C;

/// Map 6's bones: the top platform (14, collision joint 4, also the
/// bubbles' and splash segments' frame), its plate's collision (21), crack
/// (28) and wobbling bones (30, 32, the item on 30).
pub const LEFT_PLATFORM_BONE: usize = 14;
pub const LEFT_PLATE_BONE: usize = 21;
pub const LEFT_CRACK_BONE: usize = 28;
pub const LEFT_WOBBLE_BONES: [usize; 2] = [30, 32];
/// Map 7's bones: the part (1, collision joint 5, the right points'
/// frame), its plate's collision (6), crack (13) and wobbling bones
/// (15, 17, the item on 15).
pub const RIGHT_PART_BONE: usize = 1;
pub const RIGHT_PLATE_BONE: usize = 6;
pub const RIGHT_CRACK_BONE: usize = 13;
pub const RIGHT_WOBBLE_BONES: [usize; 2] = [15, 17];
/// The acid's surface bone.
pub const ACID_BONE: usize = 0;

/// The bubbles' collision joint (mpJointSetB10(0), its vertices set by map
/// 6's proc).
pub const CHAIN_JOINT: i32 = 0;

const fn binding(joint_index: i16, target_index: i16, extra: i16) -> crate::desc::JointMapping {
    crate::desc::JointMapping {
        joint_index,
        target_index,
        extra,
    }
}
/// grZe_803E1A10 (grZe_StageData.joints): Ground_801C2ED0 binds collision
/// joints to these maps' bones (GrZe.dat has no bindings of its own).
pub const LEFT_BINDINGS: [crate::desc::JointMapping; 3] =
    [binding(1, 6, 21), binding(4, 6, 14), binding(3, 6, 1)];
pub const RIGHT_BINDINGS: [crate::desc::JointMapping; 2] = [binding(2, 7, 6), binding(5, 7, 1)];

/// grZe_callbacks gobj procs (maps 0 and 3 have none).
pub fn map_callback(map: u8) -> Option<(&'static str, u32)> {
    match map {
        0 => None,
        1 => Some(("grZebes_801D9F84", 0x801D_9F84)),
        4 => Some(("grZebes_801D95B8", 0x801D_95B8)),
        6 => Some(("grZebes_801D881C", DYNAMICS_PROC)),
        7 => Some(("grZebes_801D925C", 0x801D_925C)),
        8 => Some(("grZebes_801D99E0", 0x801D_99E0)),
        _ => unreachable!("Brinstar map {map}"),
    }
}

/// A splash's model: its scheduler key (`SPLASH_KEY_BASE` + a pool slot)
/// and its GObj.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Splash {
    pub key: u8,
    pub object: hsd_gobj::GObjAddress,
}
/// The splash models' scheduler keys start here, past every map id.
pub const SPLASH_KEY_BASE: u8 = 20;

/// grZebes_801DCBFC's collision device type (ftCo_800C07F8(gobj, 3, ..)).
pub const ACID_DEVICE: u32 = 3;
/// grZebes_801DA3F4 keeps at most this many splashes (grZe_804D6998).
pub const MOST_SPLASHES: i16 = 10;

/// The stage's controllers.
#[derive(Clone, Debug)]
pub struct ZebesStage {
    pub parameters: Parameters,
    pub acid: Acid,
    /// stage_info.x724 and x728 (Ground_801C438C): the acid's surface now
    /// and before its last move.
    pub floor: f32,
    pub previous_floor: f32,
    /// The plates (map 6's `xC8`, map 7's `xC4`) and the phase each proc
    /// last saw (xEC, xE8).
    pub left_plate: Plate,
    pub right_plate: Plate,
    pub left_plate_seen: i32,
    pub right_plate_seen: i32,
    pub island: Island,
    pub bubbles: Bubbles,
    pub ornament: Ornament,
    pub reaction: AcidReaction,
    /// grZe_804D6998: splashes out.
    pub splashes: i16,
    /// The splashes' models, oldest first (their procs' order).
    pub live_splashes: melee_types::fixed::FixedVec<Splash, { MOST_SPLASHES as usize }>,
    /// The pool slot the last splash took.
    pub splash_cursor: u8,
    /// The acid's collision device is registered (grZebes_801D9758, at
    /// the stage's start).
    pub acid_device: bool,
    /// The material items: the left plate's, the right plate's, the
    /// bubbles'.
    pub left_plate_item: Option<u32>,
    pub right_plate_item: Option<u32>,
    pub bubble_item: Option<u32>,
    /// Ticks from a pulse's start to its animation's end (map 2's first
    /// AObj, looping cleared).
    pub pulse_ticks: u32,
}

impl ZebesStage {
    /// Ground_801C438C (0x801C438C).
    pub fn set_floor(&mut self, floor: f32) {
        self.previous_floor = self.floor;
        self.floor = floor;
    }

    /// Ground_801C4368's second output: the surface's last move.
    pub fn floor_motion(&self) -> f32 {
        self.floor - self.previous_floor
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
