//! grMuteCity_801EFC6C (0x801EFC6C): Ground creation and callback order.
//! Map 0, then map 0x1E, whose on_init (grMuteCity_801EFDF8) creates the
//! platform (0x1D) and the cars (2) before its own proc is registered.
use super::{cars::Cars, script::Script, track::Ends, Parameters};
use crate::fzero::CarJoints;
use crate::last::procs::ProcRegistration;
use hsd_archive::desc::spline::Spline;
use hsd_types::Vec3;

/// GObj creation order.
pub const MAP_ORDER: [u8; 4] = [0, CITY_MAP, PLATFORM_MAP, CAR_MAP];
/// The city, the track and the route (grMuteCity_801EFDF8).
pub const CITY_MAP: u8 = 0x1E;
/// The platform (grMuteCity_801F01B4).
pub const PLATFORM_MAP: u8 = 0x1D;
/// The cars (grMuteCity_801F044C).
pub const CAR_MAP: u8 = 2;
/// grMuteCity_801F0120 (map 0x1E): the script, the track, the cars,
/// lb_800115F4.
pub const DYNAMICS_PROC: u32 = 0x801F_0120;
pub const PLATFORM_PROC: u32 = 0x801F_0290;
pub const CAR_PROC: u32 = 0x801F_04B4;
/// Map 0x1E's bones: the right and left car splines (xE0, xDC), the
/// subtree whose rewind restarts the script (lb_8000B134), the subtree
/// that keeps playing while the route stops.
pub const RIGHT_SPLINE_BONE: usize = 9;
pub const LEFT_SPLINE_BONE: usize = 5;
pub const REWIND_BONE: usize = 0x1A;
pub const ROUTE_LIGHTS_BONE: usize = 0x11;
/// The platform's tilting bone (map 0x1D xC8) and its animation.
pub const TILT_BONE: usize = 6;
pub const TILT_ANIMATION: usize = 1;
/// grAnime_801C775C(gobj, 0, 7, 0.0, 3600.0): both models loop the route.
pub const ROUTE_FRAMES: f32 = 3600.0;
/// grMc_803E30B0 (grMc_StageData.joints): collision joints 6, 7 and 8 on
/// the platform's bones 6, 8 and 9, besides GrMc.dat's own bindings.
pub const PLATFORM_BINDINGS: [crate::desc::JointMapping; 3] = [
    binding(6, 6),
    binding(7, 8),
    binding(8, 9),
];
const fn binding(joint_index: i16, extra: i16) -> crate::desc::JointMapping {
    crate::desc::JointMapping {
        joint_index,
        target_index: PLATFORM_MAP as i16,
        extra,
    }
}
/// grMuteCity_801EFDF8's initial search parameter for both splines.
pub const TRACK_START: f32 = 0.932_775;
/// The tilt's step toward its target (grMuteCity_801F0290).
const TILT_STEP: f32 = 0.001;

/// grMc_StageCallbacks gobj procs (map 0's row is empty).
pub fn map_callback(map: u8) -> Option<(&'static str, u32)> {
    match map {
        0 => None,
        CITY_MAP => Some(("grMuteCity_801F0120", DYNAMICS_PROC)),
        PLATFORM_MAP => Some(("grMuteCity_801F0290", PLATFORM_PROC)),
        CAR_MAP => Some(("grMuteCity_801F04B4", CAR_PROC)),
        _ => unreachable!("Mute City map {map}"),
    }
}

/// The camera and blast-zone limits relative to the camera's centre, as
/// map 0x1E read them at creation (x114..x130).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Limits {
    pub camera: melee_cm::Rect,
    pub blast: melee_cm::Rect,
}

/// The stage's controllers: map 0x1E's Ground (`grMuteCity_GroundVars`),
/// map 0x1D's (`grMuteCity_GroundVars2`), the cars' tables and grMc_804D69D4.
#[derive(Clone)]
pub struct MuteCityStage {
    pub parameters: Parameters,
    /// Map 0x1E's bones 9 and 5's HSD_Splines (`u.spline`).
    pub splines: std::sync::Arc<[Spline; 2]>,
    pub script: Script,
    /// xD0 b0: the track's floor follows the splines.
    pub track_raised: bool,
    /// xD0 b1.
    pub passing_sounds: bool,
    /// xD0 b23.
    pub ends: Ends,
    /// xD8 and xD4: where the right and left splines last crossed.
    pub right_parameter: f32,
    pub left_parameter: f32,
    /// xE4 and xF0: those crossings.
    pub right_point: Vec3,
    pub left_point: Vec3,
    pub limits: Limits,
    /// grMc_804D69D4 (commands 6 and 7).
    pub track_callbacks: bool,
    /// Map 0x1D's xCC (target) and xD0 (current tilt factor).
    pub tilt_target: f32,
    pub tilt: f32,
    pub cars: Cars,
    pub car_joints: CarJoints,
    /// yakumono_param x8 and xC: the track's end lines' hit and its
    /// floor's (grMuteCity_801F2BBC).
    pub hazards: [melee_types::combat::LineHazard; 2],
}

impl MuteCityStage {
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
        // Ground_GetStageGObj registers each map's s_link 1 and 4 procs;
        // map 0x1E's own follows the maps its on_init made.
        for map in MAP_ORDER {
            rows.push(row(1, 5, Some(map), "Ground_801C1CD0", 0x801C1CD0));
            rows.push(row(4, 5, Some(map), "Ground_801C1D38", 0x801C1D38));
            if map != CITY_MAP {
                if let Some((name, address)) = map_callback(map) {
                    rows.push(row(4, 5, Some(map), name, address));
                }
            }
        }
        let (name, address) = map_callback(CITY_MAP).unwrap();
        rows.push(row(4, 5, Some(CITY_MAP), name, address));
        rows.push(row(10, 5, None, "Ground_801C0C2C", 0x801C0C2C));
        rows
    }

    /// grMuteCity_801F0290's tilt factor step: toward the target by at
    /// most 0.001.
    pub fn step_tilt(&mut self) -> f32 {
        let target = self.tilt_target;
        let current = self.tilt;
        let diff = target - current;
        if diff > 0.0 {
            if diff < TILT_STEP {
                self.tilt = target;
            } else {
                self.tilt += TILT_STEP;
            }
        } else if diff < 0.0 {
            if diff > -TILT_STEP {
                self.tilt = target;
            } else {
                self.tilt = current - TILT_STEP;
            }
        } else {
            self.tilt = target;
        }
        self.tilt
    }

    /// grMuteCity_801F2BBC (0x801F2BBC), the stage's on_touch_line: while
    /// grMc_804D69D4 is set, the end lines 0x31 and 0x35 take x8's hit and
    /// lines 0x32..0x34 xC's (`(u32) (line - 0x32) <= 2`).
    pub fn line_hazard(&self, line: i32) -> Option<melee_types::combat::LineHazard> {
        if !self.track_callbacks {
            return None;
        }
        if line == 0x31 || line == 0x35 {
            return Some(self.hazards[0]);
        }
        if (line.wrapping_sub(0x32) as u32) <= 2 {
            return Some(self.hazards[1]);
        }
        None
    }

    /// `0.5f * (xD4 + xD8)` (0x801F1AB8 fadds, 0x801F1ABC fmuls).
    pub fn track_middle(&self) -> f32 {
        0.5 * (self.left_parameter + self.right_parameter)
    }
}
