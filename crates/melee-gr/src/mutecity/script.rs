//! grMuteCity_801F04B8 (0x801F04B8): map 0x1E's command script, read
//! against the frame of its route animation (grMc_803E34E0).

/// One `grMc_CmdEntry`: run `command` once the route reaches `frame`
/// (-1 runs with the entry before it).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Entry {
    pub frame: i32,
    pub command: Command,
}

/// The script's commands (grMuteCity_801F04B8's switch on `cmd`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    /// 0, 10: no case.
    Nothing,
    /// 1: pause the script `ticks` ticks (xC6); the script stops here.
    Wait(i16),
    /// 2: the route stops: map 0x1E's joints and the whole platform play at
    /// rate 0, map 0x1E's bone 0x11 and the platform's bone 6 at rate 1.
    StopRoute,
    /// 3: map 0x1E's joints and the platform play on.
    ResumeRoute,
    /// 4 / 5: mpLib_80057BC0 / mpJointListAdd on a collision joint.
    RemoveJoint(i32),
    AddJoint(i32),
    /// 6 / 7: grMc_804D69D4, the track's item and bury callbacks' gate.
    TrackCallbacks(bool),
    /// 8: the track's floor (joint 4) leaves, its lines stop following.
    LowerTrack,
    /// 9: the track's floor joins, its splines' search starts at `start`.
    RaiseTrack(f32),
    /// 11 / 12: hide or show a platform bone and its subtree.
    HidePlatformBone(usize),
    ShowPlatformBone(usize),
    /// 13..16: one camera limit (scaled by the ground's scale) and, past
    /// the stage's own limit, the blast zone beyond it.
    CameraTop(f32),
    CameraBottom(f32),
    CameraLeft(f32),
    CameraRight(f32),
    /// 17: the camera limits and blast zone back to the stage's.
    RestoreCamera,
    /// 18 / 19: xD0 b1, which of the passing cars' sound sets plays.
    PassingSounds(bool),
    /// 20: the hazard sign (un_802FD604) for `ticks`, and its sound.
    Warning(i16),
    /// 21: map 0x1E's color overlay (grMaterial_801C9604, x0).
    CityOverlay,
    /// 22: the platform's tilt target (map 0x1D xCC).
    PlatformTilt(f32),
    /// 23: lbAudioAx_80024D78(param ? 2 : 1).
    Audio,
    /// 24: map 0x1D's color overlay and the point light.
    PlatformOverlay,
    /// 25: lines 0x32 and 0x34's material (0xC on, 0 off).
    TrackMaterial(bool),
    /// 26: xD0 b23, the shape of the track's end lines.
    TrackEnds(u8),
}

use Command::*;
const fn at(frame: i32, command: Command) -> Entry {
    Entry { frame, command }
}

/// grMc_803E34E0: 132 entries (xC4 wraps at 0x84).
pub const SCRIPT: [Entry; 132] = [
    at(-1, HidePlatformBone(2)),
    at(-1, HidePlatformBone(3)),
    at(-1, HidePlatformBone(6)),
    at(-1, HidePlatformBone(4)),
    at(-1, HidePlatformBone(7)),
    at(-1, RemoveJoint(0)),
    at(-1, RemoveJoint(1)),
    at(-1, RemoveJoint(6)),
    at(-1, RemoveJoint(2)),
    at(-1, RemoveJoint(5)),
    at(-1, RemoveJoint(7)),
    at(-1, RemoveJoint(8)),
    at(-1, Nothing),
    at(-1, RaiseTrack(0.932_775)),
    at(-1, PassingSounds(true)),
    at(-1, TrackCallbacks(true)),
    at(-1, RestoreCamera),
    at(-1, CameraTop(90.0)),
    at(180, RestoreCamera),
    at(384, PassingSounds(false)),
    at(-1, TrackCallbacks(false)),
    at(-1, Nothing),
    at(475, StopRoute),
    at(475, Wait(340)),
    at(-1, Warning(120)),
    at(-1, Wait(60)),
    at(-1, ResumeRoute),
    at(530, CityOverlay),
    at(567, PassingSounds(true)),
    at(-1, TrackCallbacks(true)),
    at(708, TrackEnds(1)),
    at(800, PassingSounds(false)),
    at(-1, TrackCallbacks(false)),
    at(-1, Nothing),
    at(-1, ShowPlatformBone(2)),
    at(-1, AddJoint(0)),
    at(-1, AddJoint(7)),
    at(-1, AddJoint(8)),
    at(-1, CameraLeft(-270.0)),
    at(-1, CameraRight(270.0)),
    at(890, StopRoute),
    at(890, Wait(340)),
    at(-1, Warning(120)),
    at(-1, Wait(60)),
    at(-1, ResumeRoute),
    at(950, CityOverlay),
    at(980, PassingSounds(true)),
    at(-1, RemoveJoint(0)),
    at(-1, RemoveJoint(7)),
    at(-1, RemoveJoint(8)),
    at(-1, HidePlatformBone(2)),
    at(-1, TrackCallbacks(true)),
    at(-1, RestoreCamera),
    at(1000, CameraTop(40.0)),
    at(-1, CameraBottom(0.0)),
    at(-1, CameraLeft(-50.0)),
    at(-1, CameraRight(50.0)),
    at(1080, Audio),
    at(-1, PlatformOverlay),
    at(708, TrackEnds(0)),
    at(1130, PassingSounds(false)),
    at(-1, Audio),
    at(-1, PlatformOverlay),
    at(-1, TrackCallbacks(false)),
    at(-1, RestoreCamera),
    at(-1, CameraBottom(-30.0)),
    at(-1, Nothing),
    at(-1, ShowPlatformBone(7)),
    at(-1, AddJoint(5)),
    at(1223, StopRoute),
    at(1223, Wait(340)),
    at(-1, Warning(120)),
    at(-1, Wait(60)),
    at(-1, ResumeRoute),
    at(1280, CityOverlay),
    at(1312, PassingSounds(true)),
    at(-1, TrackEnds(2)),
    at(-1, HidePlatformBone(7)),
    at(-1, RemoveJoint(5)),
    at(-1, RestoreCamera),
    at(-1, TrackCallbacks(true)),
    at(1404, PassingSounds(false)),
    at(-1, TrackCallbacks(false)),
    at(-1, Nothing),
    at(-1, ShowPlatformBone(3)),
    at(-1, AddJoint(1)),
    at(1495, StopRoute),
    at(-1, TrackMaterial(true)),
    at(1495, Wait(340)),
    at(-1, Warning(120)),
    at(-1, Wait(60)),
    at(-1, ResumeRoute),
    at(1550, CityOverlay),
    at(1585, PassingSounds(true)),
    at(-1, TrackCallbacks(true)),
    at(-1, TrackMaterial(false)),
    at(-1, HidePlatformBone(3)),
    at(-1, RemoveJoint(1)),
    at(2045, PassingSounds(false)),
    at(-1, TrackCallbacks(false)),
    at(-1, Nothing),
    at(-1, ShowPlatformBone(6)),
    at(-1, AddJoint(6)),
    at(-1, PlatformTilt(1.0)),
    at(2135, StopRoute),
    at(2135, Wait(340)),
    at(-1, Warning(120)),
    at(-1, Wait(60)),
    at(-1, ResumeRoute),
    at(-1, PlatformTilt(0.0)),
    at(2190, CityOverlay),
    at(2225, PassingSounds(true)),
    at(-1, TrackCallbacks(true)),
    at(-1, HidePlatformBone(6)),
    at(-1, RemoveJoint(6)),
    at(2273, LowerTrack),
    at(3290, RaiseTrack(0.864_45)),
    at(3300, CameraTop(90.0)),
    at(3390, PassingSounds(false)),
    at(-1, TrackCallbacks(false)),
    at(-1, Nothing),
    at(-1, ShowPlatformBone(4)),
    at(-1, AddJoint(2)),
    at(3510, StopRoute),
    at(3510, Wait(340)),
    at(-1, Warning(120)),
    at(-1, Wait(60)),
    at(-1, ResumeRoute),
    at(3570, CityOverlay),
    at(3599, PassingSounds(true)),
    at(-1, TrackCallbacks(true)),
    at(9999, Nothing),
];

/// map 0x1E's script cursor: xC4 (the entry) and xC6 (the pause).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Script {
    pub entry: i16,
    pub wait: i16,
}

/// What a command asks of the stage.
pub trait ScriptEngine {
    /// `HSD_JObjReqAnimAll(platform, 1.0)`: the route rewound, the platform
    /// back to frame one.
    fn restart_platform(&mut self);
    fn run(&mut self, command: Command);
}

impl Script {
    /// grMuteCity_801F04B8: `frame` is the route's (Ground_801C3F20, read
    /// first), `rewound` whether any animation from map 0x1E's bone 0x1A on
    /// wrapped this tick (lb_8000B134).
    pub fn step(&mut self, frame: f32, rewound: bool, engine: &mut impl ScriptEngine) {
        if self.wait > 0 {
            self.wait -= 1;
            return;
        }
        if rewound {
            self.entry = 0;
            engine.restart_platform();
        }
        // 801F08F8..801F0924: the entry's frame converted to float, the
        // comparison `frame >= entry` (fcmpo, cror).
        while frame >= SCRIPT[self.entry as usize].frame as f32 {
            let command = SCRIPT[self.entry as usize].command;
            if let Wait(ticks) = command {
                self.wait = ticks;
            } else {
                engine.run(command);
            }
            self.entry += 1;
            if self.entry as u32 == 0x84 {
                self.entry = 0;
            }
            if matches!(command, Wait(_)) {
                break;
            }
        }
    }
}
