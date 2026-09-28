//! Pokemon Stadium, gr/grpstadium.c (NTSC 1.02): the jumbotron (map 1) and
//! the transformation controller (map 2), which swaps the base arena (map 5)
//! for the fire (3), grass (4), rock (6) or water (9) form and back.
pub mod procs;
pub mod screen;
pub mod transform;

pub use screen::{Screen, ScreenMode, ScreenPlayers};
pub use transform::{Form, Transformation};

use gekko_math::HsdRng;

/// `grStadium_801D1E20` (0x801D1E20): one audience camera flash per this
/// many ticks on average.
const FLASH_CHANCE: i32 = 200;
/// Bank-30 particle of the audience flash.
pub const FLASH_PARTICLE: u32 = 0x7530;
/// The flash's position before map scaling: a stand behind the arena.
const FLASH_Y: f32 = -100.0;
const FLASH_Z: f32 = -660.0;
const FLASH_MIN_OFFSET: i32 = 100;
const FLASH_OFFSET_RANGE: i32 = 200;

/// `grPStadium_YakumonoParam` (GrPs.dat `yakumono_param`).
#[derive(Clone, Debug, PartialEq)]
pub struct Parameters {
    /// +0/+4: ticks spent in the base form, `range` endpoints.
    pub base_frames: [i32; 2],
    /// +8/+C: ticks spent in a transformed form.
    pub form_frames: [i32; 2],
    /// +10: ticks between the announcement and the arena sinking.
    pub announce_delay: i32,
    /// +14: ticks for the old form to sink and the new one to rise.
    pub sink_frames: i32,
    /// +18: ticks the old form stays sunk before the new one is created.
    pub sunk_frames: i32,
    /// +20: the match-info screen (and pictures 15/16).
    pub info_frames: i32,
    /// +24: "Player N Defeated".
    pub defeat_frames: i32,
    /// +28: a transformation announcement.
    pub announce_frames: i32,
    /// +2C: the standings.
    pub standings_frames: i32,
    /// +30/+34: a player close-up.
    pub player_camera_frames: [i32; 2],
    /// +38/+3C: the stage camera feed.
    pub stage_camera_frames: [i32; 2],
    /// +48..+4E.
    pub mode_weights: ModeWeights,
    /// +50: choices between standings.
    pub standings_interval: i16,
}

/// Weights `grStadium_801D2A60` draws the next screen mode with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModeWeights {
    /// +48.
    pub player_camera: i16,
    /// +4A.
    pub match_info: i16,
    /// +4C.
    pub stage_camera: i16,
    /// +4E: mode 15.
    pub picture: i16,
}

#[derive(Clone, Debug)]
pub struct Stadium {
    pub parameters: Parameters,
    pub screen: Screen,
    pub transformation: Transformation,
}

impl Stadium {
    /// `grStadium_OnInit` (0x801D10F8 for maps 0, 1, 2): the screen blanks
    /// without drawing; map 2 draws the first base-form duration.
    pub fn initialize(parameters: Parameters, rng: &mut HsdRng) -> Self {
        let screen = Screen::new(&parameters, rng);
        let transformation = Transformation::new(&parameters, rng);
        Self {
            parameters,
            screen,
            transformation,
        }
    }

    /// `Ground_801C0FB8` (0x801C0FB8) at the countdown's end runs the
    /// deferred callbacks newest first: fn_801D13C8 releases the controller,
    /// fn_801D11E4 creates the screen's (inactive) camera subject.
    pub fn start(&mut self) {
        self.transformation.waiting_for_start = false;
        self.screen.subject_active = Some(false);
    }

    /// `grStadium_801D1E20` (0x801D1E20): the audience flash, before
    /// map scaling. No fused arithmetic: 801D1EA0 fsubs (int conversion),
    /// 801D1EA4 fadds; the caller's scale is three fmuls.
    pub fn audience_flash(rng: &mut HsdRng) -> Option<hsd_types::Vec3> {
        if rng.randi(FLASH_CHANCE) != 0 {
            return None;
        }
        let side = rng.randi(2);
        let offset = rng.randi(FLASH_OFFSET_RANGE);
        let x = 0.0 + ((side * 2 - 1) * (offset + FLASH_MIN_OFFSET)) as f32;
        Some(hsd_types::Vec3::new(x, FLASH_Y, FLASH_Z))
    }
}
