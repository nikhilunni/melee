//! grYorster_80201FA4 (0x80201FA4): Ground creation and callback order.
//! Maps 0 and 1 (grYorster_8020203C); both have a gobj proc.
use super::{Block, Parameters, BLOCK_BONES};
use crate::last::procs::ProcRegistration;

pub const MAP_ORDER: [u8; 2] = [0, 1];
/// The stage itself: collision, the blocks, the dynamics fields.
pub const STAGE_MAP: u8 = 1;
/// grYorster_80202254 (map 1): the blocks, lb_800115F4, Ground_801C2FE0.
pub const DYNAMICS_PROC: u32 = 0x8020_2254;
/// grYorster_802021AC: bones 23 and 26 loop animation 3.
pub const LOOP_BONES: [usize; 2] = [23, 26];
pub const LOOP_ANIMATION: usize = 3;
/// grYorster_802021AC: map 1's model animation.
pub const STAGE_ANIMATION: usize = 4;
/// grMaterial_801C8DE0(item, 0, 0, 0, 0, 0, 0, 5.0): a block's hurt
/// capsule, a sphere on its bone.
pub const BLOCK_HURT_RADIUS: f32 = 5.0;

/// grYt_StageCallbacks gobj procs.
pub fn map_callback(map: u8) -> (&'static str, u32) {
    match map {
        0 => ("grYorster_80202158", 0x8020_2158),
        1 => ("grYorster_80202254", DYNAMICS_PROC),
        _ => unreachable!("Yoshi's Island map {map}"),
    }
}

/// The stage's controller (map 1's `GroundVars_yorster`).
#[derive(Clone, Debug)]
pub struct YorsterStage {
    pub parameters: Parameters,
    /// xC4: set by grYorster_802021AC until the deferred
    /// grYorster_80202160 creates the blocks at the stage's start.
    pub waiting: bool,
    pub blocks: [Block; 9],
    /// Each block's stage material item (`elements[i].x1C`).
    pub items: [Option<u32>; 9],
    /// Each block's collision joint (Ground_801C32D4(1, bone)).
    pub joints: [i32; 9],
    pub lights: Vec<crate::battle::lights::Light>,
}

impl YorsterStage {
    pub fn new(parameters: Parameters, joints: [i32; 9]) -> Self {
        Self {
            parameters,
            waiting: true,
            blocks: BLOCK_BONES.map(Block::new),
            items: [None; 9],
            joints,
            lights: Vec::new(),
        }
    }
    /// The block whose collision joint is `joint`.
    pub fn block_of_joint(&self, joint: i32) -> Option<usize> {
        self.joints.iter().position(|&j| j == joint)
    }
    /// The block whose material item is `item`.
    pub fn block_of_item(&self, item: u32) -> Option<usize> {
        self.items.iter().position(|&i| i == Some(item))
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
