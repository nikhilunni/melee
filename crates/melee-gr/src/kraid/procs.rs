//! grKraid_OnInit: Ground creation and callback order. grKraid_801FE0C4
//! creates maps 0, 2, 3, 4, 1; every row of grKr_StageCallbacks has a gobj
//! proc.
use super::{Kraid, Turn};
use crate::last::procs::ProcRegistration;

pub const MAP_ORDER: [u8; 5] = [0, 2, 3, 4, 1];
/// Kraid's effects (grKraid_801FE1EC; its proc grKraid_801FE2D0).
pub const EFFECTS_MAP: u8 = 1;
/// The stage that turns, with every collision joint.
pub const STAGE_MAP: u8 = 3;
pub const KRAID_MAP: u8 = 4;
/// grKraid_801FE440 (map 3): its turn, lb_800115F4, Ground_801C2FE0.
pub const DYNAMICS_PROC: u32 = 0x801F_E440;

const fn binding(joint_index: i16) -> crate::desc::JointMapping {
    crate::desc::JointMapping {
        joint_index,
        target_index: STAGE_MAP as i16,
        extra: 12,
    }
}
/// grKr_803E4C78 (grKr_StageData.joints): Ground_801C2ED0 binds collision
/// joints 0..5 to map 3's bone 12 (GrKr.dat has no bindings of its own).
pub const STAGE_BINDINGS: [crate::desc::JointMapping; 6] = [
    binding(0),
    binding(1),
    binding(2),
    binding(3),
    binding(4),
    binding(5),
];

/// grKr_StageCallbacks gobj procs.
pub fn map_callback(map: u8) -> (&'static str, u32) {
    match map {
        0 => ("grKraid_801FE1E4", 0x801FE1E4),
        1 => ("grKraid_801FE2D0", 0x801FE2D0),
        2 => ("grKraid_801FE3AC", 0x801FE3AC),
        3 => ("grKraid_801FE440", DYNAMICS_PROC),
        4 => ("grKraid_801FEA00", 0x801FEA00),
        _ => unreachable!("Brinstar Depths map {map}"),
    }
}

/// The stage's controllers.
#[derive(Clone, Debug)]
pub struct KraidStage {
    pub parameters: super::Parameters,
    pub turn: Turn,
    pub kraid: Kraid,
    /// grKr_804D6A0C: the effects animation the second port's A button
    /// last asked for (grKraid_801FE2D0, debug code left in retail).
    pub effect_request: i32,
    /// HSD_PadMasterStatus[1].button as the last tick left it, for that
    /// button's trigger.
    pub second_port_buttons: u32,
}

impl KraidStage {
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
