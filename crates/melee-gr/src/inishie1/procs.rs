//! grInishie1_801FA90C (0x801FA90C): Ground creation and callback order.
//! Maps 0 and 3 (setupStageCallbacks); map 2, the `?` block, is created
//! only with items on.
use super::{Brick, Lifts, Parameters};
use crate::last::procs::ProcRegistration;

pub const MAP_ORDER: [u8; 2] = [0, 3];
/// The stage: bricks, lifts, collision.
pub const STAGE_MAP: u8 = 3;
/// grInishie1_801FAB68 (map 3): bricks, lifts, music, lb_800115F4,
/// Ground_801C2FE0.
pub const DYNAMICS_PROC: u32 = 0x801F_AB68;
/// The lifts' collision joints (mpJointSetCb1(0x14 / 0x15, fn_801FC9AC)).
pub const LIFT_JOINTS: [i32; 2] = [0x14, 0x15];
/// The lifts' bones (grInishie1_801FC018: x108, x10C) and the bone whose
/// material grMaterial_801C8858 flags.
pub const LIFT_BONES: [usize; 2] = [0x1A, 0x1C];
/// grInishie1_801FAD84: the brick's material item's capsule
/// (grMaterial_801C8DE0(item, 0, -1, 0, 0, 1, 0, 6)).
pub const BRICK_HURT: ([[f32; 3]; 2], f32) = ([[0.0, -1.0, 0.0], [0.0, 1.0, 0.0]], 6.0);
/// grInishie1_801FBCEC: the breaking brick's effect (efSync_Spawn).
pub const BREAK_EFFECT: u16 = 0x442;
/// fn_801FBEB8: a ceiling contact faster than this breaks a brick
/// (`fabsf(delta_y) > 0.7`, compared in double).
pub const BREAK_SPEED: f64 = 0.7;

const fn binding(joint_index: i16, extra: i16) -> crate::desc::JointMapping {
    crate::desc::JointMapping {
        joint_index,
        target_index: STAGE_MAP as i16,
        extra,
    }
}
/// grI1_803E48C8 (grI1_StageData.joints): collision joint 13 on bone 3 and
/// 14..19 on their own bones, bound with the archive's (Ground_801C2ED0).
pub const STAGE_BINDINGS: [crate::desc::JointMapping; 7] = [
    binding(13, 3),
    binding(14, 14),
    binding(15, 15),
    binding(16, 16),
    binding(17, 17),
    binding(18, 18),
    binding(19, 19),
];
/// grI1_803E49B8: each brick's collision joint and bone, in brick order.
pub const BRICKS: [(i32, usize); 19] = [
    (13, 3),
    (14, 14),
    (15, 15),
    (16, 16),
    (17, 17),
    (18, 18),
    (19, 19),
    (11, 20),
    (12, 21),
    (1, 4),
    (2, 5),
    (3, 6),
    (4, 7),
    (5, 8),
    (6, 9),
    (7, 10),
    (8, 11),
    (9, 12),
    (10, 13),
];

/// grI1_StageCallbacks gobj procs.
pub fn map_callback(map: u8) -> (&'static str, u32) {
    match map {
        0 => ("grInishie1_801FAAD4", 0x801F_AAD4),
        3 => ("grInishie1_801FAB68", DYNAMICS_PROC),
        _ => unreachable!("Mushroom Kingdom map {map}"),
    }
}

/// The stage's controller (map 3's `GroundVars_inishie1`).
#[derive(Clone, Debug)]
pub struct Inishie1Stage {
    pub parameters: Parameters,
    pub bricks: [Brick; 19],
    /// Each brick's stage material item.
    pub items: [Option<u32>; 19],
    pub lifts: Lifts,
    /// xC6 / xC8: the `?` blocks' waits (0 with items off).
    pub hatena_waits: [i16; 2],
    /// xD8: the all-`?` cooldown.
    pub all_hatena_cooldown: i32,
    /// xC4 b0 of grInishie1_801FCB10: the hurry-up music was decided.
    pub music_decided: bool,
    pub lights: Vec<crate::battle::lights::Light>,
}

impl Inishie1Stage {
    /// The brick whose collision joint is `joint` (get_block_id).
    pub fn brick_of_joint(joint: i32) -> Option<usize> {
        BRICKS.iter().position(|&(j, _)| j == joint)
    }
    /// The brick whose material item is `item`.
    pub fn brick_of_item(&self, item: u32) -> Option<usize> {
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
