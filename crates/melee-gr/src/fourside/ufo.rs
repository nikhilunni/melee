//! The UFO (map 5, `grFourside_UfoVars`): every `ufo_wait` ticks it may
//! land over one of three buildings as a platform (collision joint 4 on
//! bone 1), framed by the camera while it descends, then stays and leaves.
use super::{wait, Parameters};
use gekko_math::HsdRng;
use hsd_types::Vec3;

/// The UFO's collision joint (grFs_803E3CE8).
pub const JOINT: i32 = 4;
/// The bone the camera frames (Ground_801C3FA4(gobj, 1)).
pub const BONE: usize = 1;
/// Animations per building: arrive, stay (looping), leave, unused.
const ANIMATIONS_PER_BUILDING: usize = 4;
/// The building the helicopter never blocks.
const FAR_BUILDING: u8 = 2;
/// Ticks the leaving UFO stays visible.
const LEAVE_FRAMES: i32 = 0x3C;
/// x1 before the first landing.
pub const NO_BUILDING: u8 = 0xFF;
/// grFourside_801F3B70: the framed subject's extents.
const SUBJECT_EXTENTS: [f32; 4] = [-50.0, 50.0, 15.0, -15.0];

/// `foursideUfo.x0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UfoPhase {
    Away = 0,
    Arriving = 1,
    Staying = 2,
    Leaving = 4,
}
impl UfoPhase {
    pub fn from_saved(value: u8) -> Option<Self> {
        Some(match value {
            0 => Self::Away,
            1 => Self::Arriving,
            2 => Self::Staying,
            4 => Self::Leaving,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Ufo {
    pub phase: UfoPhase,
    /// x1: the building of the current or last landing.
    pub building: u8,
    /// x2: waits since the last landing.
    pub tries: u8,
    /// x3: 1 on the tick the arrival first comes below the camera's top
    /// (its sound), counting on from there.
    pub sound_step: u8,
    /// x4.
    pub timer: i32,
    /// x8: fighters on the main building's roof since the last step
    /// (grFourside_801F30A0, collision joint 0's callback).
    pub roof_contacts: i32,
    /// xC: the camera subject while it arrives (Camera_80029020).
    pub subject: Option<melee_cm::Subject>,
}

/// The model and collision operations of one step, in C order.
pub trait UfoEngine {
    /// grAnime_801C8138(gobj, 5, index): attach and evaluate frame zero.
    fn select(&mut self, index: usize);
    /// mpLib_80055E9C(4), mpLib_80057424(4): the joint follows bone 1 as
    /// it is now (still hidden).
    fn bind_joint(&mut self);
    /// HSD_JObjClearFlagsAll / HSD_JObjSetFlagsAll(JOBJ_HIDDEN).
    fn set_hidden(&mut self, hidden: bool);
    /// mpJointListAdd(4) / mpLib_80057BC0(4).
    fn set_joint_enabled(&mut self, enabled: bool);
    /// grAnime_801C83D0(gobj, 0, 7).
    fn ended(&self) -> bool;
    /// grAnime_801C84A4(gobj, 0, 7).
    fn rewound(&self) -> bool;
    /// lb_8000B1CC(bone 1): its world translation.
    fn bone_position(&mut self) -> Vec3;
}

impl Ufo {
    /// grFourside_801F37FC (0x801F37FC): hidden, joint 4 disabled, no draw.
    pub fn create(parameters: &Parameters) -> Self {
        Self {
            phase: UfoPhase::Away,
            building: NO_BUILDING,
            tries: 0,
            sound_step: 0,
            timer: parameters.ufo_wait,
            roof_contacts: 0,
            subject: None,
        }
    }

    /// The helicopter's test of map 5 (`fourside2.x1 == 2 || x0 == 0`).
    pub fn away_or_far(&self) -> bool {
        self.building == FAR_BUILDING || self.phase == UfoPhase::Away
    }

    /// grFourside_801F3894 (0x801F3894) up to its Ground_801C2FE0.
    /// `heli_busy` is grFourside_801F3F10; `camera_top`
    /// Stage_GetCamBoundsTopOffset.
    pub fn tick(
        &mut self,
        parameters: &Parameters,
        heli_busy: impl FnOnce() -> bool,
        camera_top: f32,
        rng: &mut HsdRng,
        engine: &mut impl UfoEngine,
    ) {
        let p = parameters;
        // The building as the proc read it on entry (r29).
        let previous = self.building;
        match self.phase {
            UfoPhase::Away => {
                if self.timer <= 0 {
                    let mut odds = 0;
                    if self.roof_contacts != 0 {
                        odds = i32::from(p.ufo_standing_odds);
                    }
                    if i32::from(self.tries) >= i32::from(p.ufo_challenge) {
                        odds = i32::from(p.ufo_challenged_odds);
                    }
                    if odds != 0 && rng.randi(odds) == 0 {
                        self.tries = 0;
                        let building = loop {
                            let building = rng.randi(3) as u8;
                            if building != previous {
                                break building;
                            }
                        };
                        if building == FAR_BUILDING || !heli_busy() {
                            self.building = building;
                            engine.select(usize::from(building) * ANIMATIONS_PER_BUILDING);
                            engine.bind_joint();
                            engine.set_hidden(false);
                            engine.set_joint_enabled(true);
                            self.subject =
                                Some(melee_cm::Subject::new(melee_cm::SubjectState::Active));
                            self.sound_step = 0;
                            self.phase = UfoPhase::Arriving;
                        }
                    } else {
                        self.timer = p.ufo_wait;
                        self.tries = self.tries.wrapping_add(1);
                    }
                } else {
                    self.timer -= 1;
                }
            }
            UfoPhase::Arriving => {
                if engine.ended() {
                    self.timer = wait(p.ufo_stay_time, p.ufo_stay_time_add, rng);
                    engine.select(usize::from(previous) * ANIMATIONS_PER_BUILDING + 1);
                    // Camera_800290D4: the subject leaves the camera's list.
                    self.subject = None;
                    self.phase = UfoPhase::Staying;
                }
                self.frame(parameters, camera_top, engine);
                // Ground_801C53EC on the first framed tick: sound only.
                if self.sound_step == 1 {
                    self.sound_step += 1;
                }
            }
            UfoPhase::Staying => {
                if self.timer <= 0 {
                    if engine.rewound() {
                        engine.select(usize::from(previous) * ANIMATIONS_PER_BUILDING + 2);
                        self.phase = UfoPhase::Leaving;
                    }
                } else {
                    self.timer -= 1;
                }
            }
            UfoPhase::Leaving => {
                if self.timer >= LEAVE_FRAMES {
                    engine.set_hidden(true);
                    engine.set_joint_enabled(false);
                    self.timer = p.ufo_wait;
                    self.phase = UfoPhase::Away;
                } else {
                    self.timer += 1;
                }
            }
        }
    }

    /// The end of grFourside_801F3894, after Ground_801C2FE0.
    pub fn end_step(&mut self) {
        self.roof_contacts = 0;
    }

    /// grFourside_801F3B70 (0x801F3B70): frame the UFO once it is below the
    /// camera's top plus `ufo_cs_offs`.
    fn frame(&mut self, parameters: &Parameters, camera_top: f32, engine: &mut impl UfoEngine) {
        let position = engine.bone_position();
        let Some(subject) = &mut self.subject else {
            return;
        };
        // retail 0x801F3BA8: fadds.
        if position.y <= camera_top + parameters.ufo_camera_offset {
            if self.sound_step == 0 {
                self.sound_step = 1;
            }
            subject.position = position;
            subject.bone_position = position;
            let [left, right, top, bottom] = SUBJECT_EXTENTS;
            subject.target_extents.left = left;
            subject.target_extents.right = right;
            subject.target_extents.top = top;
            subject.target_extents.bottom = bottom;
        }
    }
}
