//! grRCruise_801FF168 (0x801FF168): creation order 3 (the camera path,
//! hidden), 3 again (the scroll controller), 0, 2, 5, 1, 6, 4.
use super::{
    parts::{Cloud, Swing, Tilt},
    Parameters,
};
use crate::castle::wind::Gusts;
use crate::last::procs::ProcRegistration;
use hsd_types::Vec3;

/// The camera path (the first map-3 GObj) and the scroll controller (the
/// second), which the scheduler and the models key apart.
pub const PATH_KEY: u8 = 3;
pub const SCROLL_KEY: u8 = 7;
pub const SHIP_MAP: u8 = 1;
pub const SWING_MAP: u8 = 4;
pub const SHIP_COPY_MAP: u8 = 5;
pub const FLAG_MAP: u8 = 6;
/// The keys in creation order.
pub const KEY_ORDER: [u8; 8] = [PATH_KEY, SCROLL_KEY, 0, 2, 5, 1, 6, 4];
/// The maps the scroll translates (grRCruise_801FFADC), in its order.
pub const SCROLLED: [u8; 5] = [2, 1, 5, 6, 4];
/// grRCruise_801FF6D4 (map 1): the parts, lb_800115F4, the wind,
/// Ground_801C2FE0.
pub const DYNAMICS_PROC: u32 = 0x801F_F6D4;
/// grRCruise_801FF168's mpLib_80057BC0(11).
pub const DISABLED_JOINT: i32 = 11;
/// grRCruise_801FF924's jobjs on the path model: the camera's centre,
/// eye and interest.
pub const CENTRE_BONE: usize = 1;
pub const EYE_BONE: usize = 2;
pub const INTEREST_BONE: usize = 3;
/// grRCruise_801FFADC: the scroll controller's model sits this far behind
/// (times the map scale); the projection is lifted by 10.
pub const BACKDROP_DEPTH: f32 = -350.0;
pub const LIFT: f32 = 10.0;
/// grRCruise_801FF7A4: map 6's bones that carry the players' spawn
/// points 0..3 (Ground_801C2D0C).
pub const SPAWN_BONES: [usize; 4] = [10, 11, 12, 13];
/// grRCruise_801FF5B4's lb_80011A50(&{1, 0, 0}, -1, 0.4, 0, 60 degrees).
pub const FIELD_STRENGTH: f32 = 0.4;

const fn ship(joint_index: i16, extra: i16) -> crate::desc::JointMapping {
    crate::desc::JointMapping {
        joint_index,
        target_index: SHIP_MAP as i16,
        extra,
    }
}
/// grRc_803E4DA8 (grRc_StageData.joints): collision joints on map 1's
/// bones. Ground_801C2ED0 binds them after the archive's rows, so a joint
/// both list keeps this bone.
pub const STAGE_BINDINGS: [crate::desc::JointMapping; 23] = [
    ship(0, 1),
    ship(1, 1),
    ship(2, 1),
    ship(3, 1),
    ship(4, 1),
    ship(11, 7),
    ship(10, 17),
    ship(6, 7),
    ship(7, 7),
    ship(8, 7),
    ship(9, 7),
    ship(12, 19),
    ship(13, 18),
    ship(14, 18),
    ship(15, 18),
    ship(16, 18),
    ship(17, 18),
    ship(18, 18),
    ship(19, 18),
    ship(20, 18),
    ship(21, 18),
    ship(22, 18),
    ship(23, 18),
];

/// Map 1's joints in Ground_801C2FE0's update order: the stage table's,
/// then the archive's not yet updated, each on the bone it was bound to
/// last (Ground_801C2ED0: the archive's rows, then the stage table's).
pub fn ship_bindings(archive: &[crate::desc::JointMapping]) -> Vec<crate::desc::JointMapping> {
    let mut rows = STAGE_BINDINGS.to_vec();
    rows.extend(
        archive
            .iter()
            .filter(|a| !STAGE_BINDINGS.iter().any(|s| s.joint_index == a.joint_index))
            .cloned(),
    );
    rows
}

/// Ground_801C32D4 (0x801C32D4) on map 1: the joint bound to `bone`, the
/// archive's row winning over the stage table's.
pub fn ship_joint_of(archive: &[crate::desc::JointMapping], bone: usize) -> Option<i32> {
    let found = |rows: &[crate::desc::JointMapping]| {
        rows.iter()
            .rev()
            .find(|b| b.extra as usize == bone)
            .map(|b| i32::from(b.joint_index))
    };
    found(archive).or_else(|| found(&STAGE_BINDINGS))
}

/// grRc_StageCallbacks' gobj procs by key.
pub fn map_callback(key: u8) -> (&'static str, u32) {
    match key {
        0 => ("grRCruise_801FF3E8", 0x801F_F3E8),
        1 => ("grRCruise_801FF6D4", DYNAMICS_PROC),
        2 => ("grRCruise_801FF43C", 0x801F_F43C),
        PATH_KEY | SCROLL_KEY => ("grRCruise_801FFADC", 0x801F_FADC),
        4 => ("grRCruise_80200154", 0x8020_0154),
        5 => ("grRCruise_801FF79C", 0x801F_F79C),
        6 => ("grRCruise_801FF8E4", 0x801F_F8E4),
        _ => unreachable!("Rainbow Cruise key {key}"),
    }
}

/// The model a key draws.
pub fn model(key: u8) -> u8 {
    if key == SCROLL_KEY {
        PATH_KEY
    } else {
        key
    }
}

/// grRCruise_801FFADC's projection of the camera path: where the eye's
/// line through the interest meets z = 0 (retail 0x801FFBA8 / 0x801FFBAC:
/// fnmsubs), less the stage's camera offset, lifted by 10.
pub fn project(eye: Vec3, interest: Vec3, camera_offset: Vec3) -> Vec3 {
    use gekko_math::fma::fnmsubs;
    let diff = Vec3::new(interest.x - eye.x, interest.y - eye.y, interest.z - eye.z);
    let ratio = eye.z / diff.z;
    let mut point = Vec3::new(fnmsubs(diff.x, ratio, eye.x), fnmsubs(diff.y, ratio, eye.y), 0.0);
    point.x -= camera_offset.x;
    point.y -= camera_offset.y;
    point.z -= camera_offset.z;
    point.y += LIFT;
    point
}

/// The stage's controllers.
#[derive(Clone, Debug)]
pub struct RCruiseStage {
    pub parameters: Parameters,
    /// The scroll controller's x10 (last projection) and x1C (its change:
    /// grRCruise_80201918, the stage's drift).
    pub scroll: Vec3,
    pub drift: Vec3,
    pub tilt: Tilt,
    pub clouds: [Cloud; 17],
    pub swings: [Swing; 3],
    /// The appearing blocks' states (`Map_Vanish.x0`).
    pub vanish: [u8; 20],
    /// x10: set until the deferred grRCruise_801FF444 runs.
    pub waiting: bool,
    pub gusts: Gusts,
    /// Map 1's collision joints ([`ship_bindings`]).
    pub ship_bindings: Vec<crate::desc::JointMapping>,
}

impl RCruiseStage {
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
        for key in KEY_ORDER {
            rows.push(row(1, 5, Some(key), "Ground_801C1CD0", 0x801C1CD0));
        }
        for key in KEY_ORDER {
            rows.push(row(4, 5, Some(key), "Ground_801C1D38", 0x801C1D38));
            let (name, address) = map_callback(key);
            rows.push(row(4, 5, Some(key), name, address));
        }
        rows.push(row(10, 5, None, "Ground_801C0C2C", 0x801C0C2C));
        rows
    }
}
