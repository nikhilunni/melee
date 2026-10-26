//! grGreatBay_801F4240 (0x801F4240): Ground creation and callback order.
//! Maps 0, 3, 4, 2, 1, 9 (whose on_init makes the giants, maps 5..8) and
//! 10 (Tingle's platform, whose proc runs at s_link 5).
use super::{turtle::Turtle, FloatFloor, Parameters};
use crate::{desc::JointMapping, last::procs::ProcRegistration};

/// grGreatBay_801F4300's calls, in order.
pub const CREATION_ORDER: [u8; 7] = [0, 3, 4, 2, 1, 9, 10];
/// The GObjs in creation order: the moon's on_init makes the giants
/// between its GObj and its gobj proc.
pub const GOBJ_ORDER: [u8; 11] = [0, 3, 4, 2, 1, 9, 5, 6, 7, 8, 10];
pub const BACKGROUND_MAP: u8 = 3;
pub const WATER_MAP: u8 = 4;
/// The lab and its floating platform.
pub const STAGE_MAP: u8 = 2;
pub const TURTLE_MAP: u8 = 1;
pub const MOON_MAP: u8 = 9;
pub const GIANT_MAPS: [u8; 4] = [5, 6, 7, 8];
/// Tingle's platform: beyond GrGb.dat's ten models, an identity JObj.
pub const TINGLE_MAP: u8 = 10;
/// grGreatBay_801F4658 (map 2): the platform, Ground_801C2FE0, lb_800115F4.
pub const DYNAMICS_PROC: u32 = 0x801F_4658;
/// The floating platform's collision joint and bone.
pub const FLOAT_JOINT: i32 = 0;
pub const FLOAT_BONE: usize = 49;
/// Tingle's platform's collision joint.
pub const TINGLE_JOINT: i32 = 5;

const fn joint(joint_index: i16, target_index: i16, extra: i16) -> JointMapping {
    JointMapping {
        joint_index,
        target_index,
        extra,
    }
}
/// grGb_803E3E60 (grGb_StageData.joints).
pub const STAGE_JOINTS: [JointMapping; 6] = [
    joint(0, 2, 49),
    joint(1, 1, 2),
    joint(2, 1, 3),
    joint(4, 1, 34),
    joint(3, 1, 38),
    joint(5, 10, 0),
];
/// grGb_803E4048: the turtle's shell vertices that follow its bones
/// (grGreatBay_801F660C, mpVtxSetPos).
pub const SHELL_VERTICES: [(i32, usize); 7] = [
    (32, 20),
    (23, 18),
    (22, 15),
    (29, 16),
    (28, 17),
    (27, 19),
    (39, 21),
];

/// grGb_StageCallbacks gobj procs and their s_link.
pub fn map_callback(map: u8) -> (&'static str, u32, u8) {
    match map {
        0 => ("grGreatBay_801F4438", 0x801F_4438, 4),
        TURTLE_MAP => ("grGreatBay_801F499C", 0x801F_499C, 4),
        STAGE_MAP => ("grGreatBay_801F4658", DYNAMICS_PROC, 4),
        BACKGROUND_MAP => ("grGreatBay_801F44A8", 0x801F_44A8, 4),
        WATER_MAP => ("grGreatBay_801F4518", 0x801F_4518, 4),
        5..=8 => ("grGreatBay_801F59F8", 0x801F_59F8, 4),
        MOON_MAP => ("grGreatBay_801F5AF8", 0x801F_5AF8, 4),
        TINGLE_MAP => ("grGreatBay_801F5600", 0x801F_5600, 5),
        _ => unreachable!("Great Bay map {map}"),
    }
}

/// The moon (map 9, `grGreatBay_GroundVars2`): x10 counts down to its
/// fall.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Moon {
    pub timer: i16,
    /// x12 b0..b3: the giants' sounds started (Ground_801C3F20 > 0).
    pub started: [bool; 4],
}

/// Tingle's platform (map 10, `grGreatBay_GroundVars4`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TinglePlatform {
    /// xC4 / xC8: Tingle's motion state now and at the last step.
    pub motion: i32,
    pub last_motion: i32,
    /// xCC: the balloon popped (its effect and sound made).
    pub popped: bool,
    /// xD0: the splash made.
    pub splashed: bool,
    /// xD4: the last rider's ledge flags.
    pub ledge: u32,
    /// xD8 / xDC: riders this frame and last.
    pub riders: i32,
    pub last_riders: i32,
    /// xE0: the riders' weight (x50 / 100 each).
    pub weight: f32,
    /// xE4: where the platform stands (Tingle's position, raised).
    pub position: hsd_types::Vec3,
    /// xF0: Tingle's item.
    pub item: Option<u32>,
    /// Tingle fell past -50 this step: once his teardown runs he is made
    /// again.
    pub respawn: bool,
}

/// The stage's controllers.
#[derive(Clone, Debug)]
pub struct GreatBayStage {
    pub parameters: Parameters,
    pub float_floor: FloatFloor,
    /// bone 49's world x after the last step, for the landings until the next
    /// (lb_8000B1CC in grGreatBay_801F60C4).
    pub float_x: f32,
    pub turtle: Turtle,
    pub moon: Moon,
    pub tingle: TinglePlatform,
    /// Tingle's creation draws (it_802ECA70 at the stage's creation: the
    /// facing coin, then his place and wait), until the item pool makes him.
    pub tingle_draws: Option<(f32, i32, i32)>,
    /// fn_801F4520 ran: the water's fighter device is registered.
    pub water: bool,
    /// xC8: the turtle's wake generator.
    pub wake: Option<usize>,
    /// gm_8016AE80() != -1: the turtle's items would be drawn.
    pub items_on: bool,
    pub lights: Vec<crate::battle::lights::Light>,
}

impl GreatBayStage {
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
        for map in GOBJ_ORDER {
            rows.push(row(1, 5, Some(map), "Ground_801C1CD0", 0x801C1CD0));
        }
        for map in GOBJ_ORDER {
            let (name, address, s_link) = map_callback(map);
            rows.push(row(4, 5, Some(map), "Ground_801C1D38", 0x801C1D38));
            rows.push(row(s_link, 5, Some(map), name, address));
        }
        rows.push(row(10, 5, None, "Ground_801C0C2C", 0x801C0C2C));
        rows
    }
}
