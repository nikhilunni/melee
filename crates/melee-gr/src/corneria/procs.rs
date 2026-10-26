//! grCorneria_801DD350 (0x801DD350) and grCorneria_801DD478 (on_load):
//! the maps a match starts with and their callbacks. Creation makes maps
//! 7, 3 (whose init makes map 0 and the cannons' material items), 8, 9, 4
//! and 11; on_load takes 11, 8, 9 and 4 away again and makes a moving map
//! 8, so a match begins with 7, 3, 0 and 8. The other backgrounds and the
//! passes follow them in the GObj list as they come.
use super::{
    arwings::{Ship, Spawner},
    backgrounds::Background,
    great_fox::{Cannon, GreatFox},
    Parameters,
};
use crate::last::procs::ProcRegistration;
use hsd_types::Vec3;

/// The maps alive when the match begins, in GObj order.
pub const MAP_ORDER: [u8; 4] = [SKY_MAP, SHIP_MAP, MARKER_MAP, 8];
/// Map 7: the sky and its fog (grCorneria_801E0C3C); its proc keeps it at
/// the sea's height.
pub const SKY_MAP: u8 = 7;
/// The Great Fox.
pub const SHIP_MAP: u8 = 3;
/// Carries the markers; the Great Fox's proc copies its translation here.
pub const MARKER_MAP: u8 = 0;
/// grCorneria_801DD9A8: the Great Fox's proc (lb_800115F4 at its end).
pub const DYNAMICS_PROC: u32 = 0x801D_D9A8;
/// The Great Fox's collision joints (on its root).
pub const SHIP_JOINTS: [i32; 2] = [3, 4];
/// The joints grCorneria_801DD350 turns off: the platforms' (0, 1, 2, 5,
/// 6, 7); a pass turns its own on.
pub const PLATFORM_JOINTS: [i32; 6] = [0, 1, 2, 5, 6, 7];

const fn binding(joint_index: i16, target_index: i16, extra: i16) -> crate::desc::JointMapping {
    crate::desc::JointMapping {
        joint_index,
        target_index,
        extra,
    }
}
/// grCn_803E1D38 (grCn_StageData.joints): joints 3 and 4 on map 3's root,
/// 0, 1, 2 on maps 13, 14, 15 and 5, 6, 7 on maps 16, 17, 18.
pub const BINDINGS: [crate::desc::JointMapping; 8] = [
    binding(3, 3, 0),
    binding(4, 3, 0),
    binding(0, 13, 0),
    binding(1, 14, 0),
    binding(2, 15, 0),
    binding(5, 16, 0),
    binding(6, 17, 0),
    binding(7, 18, 0),
];

pub fn bindings(map: u8) -> impl Iterator<Item = &'static crate::desc::JointMapping> {
    BINDINGS
        .iter()
        .filter(move |b| b.target_index == i16::from(map))
}

/// grCn_StageCallbacks gobj procs (s_link 4). Every map has one, the
/// empty ones included.
pub fn map_callback(map: u8) -> (&'static str, u32) {
    match map {
        0 => ("grCorneria_801DD654", 0x801D_D654),
        1 => ("grCorneria_801DE024", 0x801D_E024),
        2 => ("grCorneria_801DED50", 0x801D_ED50),
        3 => ("grCorneria_801DD9A8", DYNAMICS_PROC),
        4 => ("grCorneria_801E01A8", 0x801E_01A8),
        7 => ("grCorneria_801E0D30", 0x801E_0D30),
        8 => ("grCorneria_801DFC98", 0x801D_FC98),
        9 => ("grCorneria_801DFF20", 0x801D_FF20),
        10 => ("grCorneria_801DFBCC", 0x801D_FBCC),
        13..=18 => ("grCorneria_801DE568", 0x801D_E568),
        _ => unreachable!("Corneria map {map}"),
    }
}

/// A pass while any of its maps lives.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flight {
    pub kind: i32,
    pub group: i32,
    /// The path lives (arwing_gobjs[slot] != NULL); the ship and platform
    /// go in their next procs once it is gone.
    pub path: bool,
    /// The path's first tick (`u.arwing.xD4`): its animation starts then.
    pub path_starting: bool,
    pub ship_map: u8,
    pub ship: Option<Ship>,
    pub platform_map: u8,
    /// The platform's controller: the ship's bone 1 position and bank
    /// (`u.arwing.xE0`, `xDC`).
    pub platform: Option<crate::venom::arwings::Platform>,
}

/// A shot a ship's proc fired, for the scene to spawn right after it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PendingShot {
    /// The gun bone's world position.
    pub gun: Vec3,
    /// An aimed shot's target (5 above the fighter); a cannon's none.
    pub target: Option<Vec3>,
    pub laser: i32,
}

/// The stage's controllers.
#[derive(Clone, Debug)]
pub struct CorneriaStage {
    pub parameters: Parameters,
    /// xC4 b0 of map 3: the Great Fox waits for the countdown
    /// (grCorneria_801DD65C clears it).
    pub waiting: bool,
    pub great_fox: GreatFox,
    pub cannon: Cannon,
    /// The cannons' material items (`left_cannon`, `right_cannon`).
    pub cannon_items: [Option<u32>; 2],
    pub spawner: Spawner,
    /// Maps 8, 9 and 4, while they live.
    pub backgrounds: [Option<Background>; 3],
    /// Slot 0's pass.
    pub flight: Option<Flight>,
    pub pending_shot: Option<PendingShot>,
}

impl CorneriaStage {
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

    /// The live background `map`'s slot.
    pub fn background_mut(&mut self, map: u8) -> &mut Option<Background> {
        let index = super::backgrounds::MAPS
            .iter()
            .position(|&m| m == map)
            .expect("a background map");
        &mut self.backgrounds[index]
    }
}
