//! The rope bridge (grKg_803E188C, grKongo_801D6AFC / 801D7134) and its two
//! swinging ends (grKongo_801D77E0).
//!
//! Each of the fifteen planks is a map-10 JObj turned about x by `angle`.
//! A plank springs back toward its rest angle, pulls on its neighbours
//! past a slack, and a fighter landing on it kicks it down
//! (fn_801D7700). Its sag `37.8 * tan(-angle)` offsets its collision line.
use super::{Parameters, DEG_TO_RAD};
use gekko_math::fma::{fmadds, fnmsubs};

pub const PLANK_COUNT: usize = 15;
/// grKg_803E188C: each plank's map-10 descendant and rest angle.
const PLANKS: [(usize, f32); PLANK_COUNT] = [
    (8, 0.069_813_17),
    (9, 0.0),
    (0xA, -0.052_359_88),
    (0x17, -0.104_719_76),
    (0x18, -0.139_626_34),
    (0x19, -0.157_079_64),
    (0x1A, -0.209_439_52),
    (0x1B, -0.235_619_44),
    (0x1C, -0.183_259_58),
    (0x1D, -0.130_899_7),
    (0x1E, -0.078_539_82),
    (0x1F, -0.026_179_94),
    (0x20, 0.0),
    (7, 0.052_359_88),
    (0x2D, 0.104_719_76),
];
/// grKg_804D6984: the bridge's two end JObjs (descendants 0xB and 0x21),
/// tilted by the planks next to them (planks 2 and 12).
pub const END_JOINTS: [usize; 2] = [0xB, 0x21];
const END_PLANKS: [usize; 2] = [2, 12];
/// Map 10's swinging platform ends: gp+CC/D0 (descendants 0x13, 0xD) turn
/// with the left swing, gp+DC/E0 (0x2B, 0x28) with the right.
pub const SWING_JOINTS: [[usize; 2]; 2] = [[0x13, 0xD], [0x2B, 0x28]];
/// The bridge's collision joint and its first plank line (two lines a
/// plank, 0x28..0x46; the outer planks also move their outer neighbour).
pub const BRIDGE_COLLISION_JOINT: i32 = 4;
const FIRST_LINE: i32 = 0x28;
/// fn_801D7700: the bridge's end vertices, whose span splits into planks.
pub const BRIDGE_LEFT_VERTEX: i32 = 0x1D;
pub const BRIDGE_RIGHT_VERTEX: i32 = 0x1A;

/// One plank (`_struct_grKg_803E188C_0x18`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plank {
    /// +0x0: its map-10 descendant.
    pub joint: usize,
    /// +0x2 (s16): never set; a plank whose flag is set neither pulls nor
    /// is pulled.
    pub pinned: bool,
    /// +0x8.
    pub rest: f32,
    /// +0xC.
    pub angle: f32,
    /// +0x10.
    pub velocity: f32,
    /// +0x14: the sag its collision line takes.
    pub sag: f32,
}

/// The bridge: grKg_803E188C is static data, so the velocities and sags a
/// match ends with carry into the next one; a fresh boot starts at zero.
#[derive(Clone, Debug, PartialEq)]
pub struct Bridge {
    pub planks: [Plank; PLANK_COUNT],
}

/// What one bridge step writes to the model and the map.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BridgeStep {
    /// Each end JObj's rotation x and z.
    pub ends: [(f32, f32); 2],
    /// A jolt the end gives its swing (gp+C8 / gp+D8), if any.
    pub jolts: [Option<f32>; 2],
}

impl Default for Bridge {
    fn default() -> Self {
        Self {
            planks: PLANKS.map(|(joint, rest)| Plank {
                joint,
                pinned: false,
                rest,
                angle: 0.0,
                velocity: 0.0,
                sag: 0.0,
            }),
        }
    }
}

/// A clamp to [-limit, limit], testing the top first (Ground_ClampSymmetric).
fn clamp(value: f32, limit: f32) -> f32 {
    if value > limit {
        limit
    } else if value < -limit {
        -limit
    } else {
        value
    }
}

impl Bridge {
    /// grKongo_801D69B0 (0x801D69B0): every plank at its rest angle.
    pub fn reset_angles(&mut self) {
        for plank in &mut self.planks {
            plank.angle = plank.rest;
        }
    }

    /// grKongo_801D6AFC (0x801D6AFC): spring, damping and the neighbours'
    /// pull, one frame.
    fn settle(&mut self, p: &Parameters) {
        let planks = &mut self.planks;
        for plank in planks.iter_mut() {
            // 801D6B60: fmuls; 801D6B94: (b + v) in single, times 0.99 in
            // double, rounded.
            let limit = DEG_TO_RAD * p.plank_restore;
            let offset = plank.angle - plank.rest;
            let push = if offset > limit {
                -limit
            } else if offset < -limit {
                limit
            } else {
                0.0
            };
            plank.velocity = (0.99 * f64::from(push + plank.velocity)) as f32;
        }
        // 801D6C5C: a zeroed array is added in (x + 0.0 clears a -0.0).
        for plank in planks.iter_mut() {
            plank.velocity += 0.0;
        }
        for plank in planks.iter_mut() {
            plank.velocity = clamp(plank.velocity, DEG_TO_RAD * p.plank_speed_limit);
        }
        for plank in planks.iter_mut() {
            plank.angle += plank.velocity;
            plank.angle = clamp(plank.angle, DEG_TO_RAD * p.plank_limit);
        }
        let mut pull = [0.0f32; PLANK_COUNT];
        for i in 0..PLANK_COUNT {
            if planks[i].pinned {
                continue;
            }
            let mut tug = |other: usize| {
                if planks[other].pinned {
                    return;
                }
                // 801D6FB8..801D6FD4: fsubs, then fmadds x9C's pull.
                let slack = DEG_TO_RAD * p.plank_slack;
                let input = planks[other].angle - planks[i].angle;
                if input > slack {
                    pull[i] = fmadds(p.plank_pull, input - slack, pull[i]);
                } else if input < -slack {
                    pull[i] = fmadds(p.plank_pull, input + slack, pull[i]);
                }
            };
            if i != 0 {
                tug(i - 1);
            }
            if i != PLANK_COUNT - 1 {
                tug(i + 1);
            }
        }
        for (plank, pull) in planks.iter_mut().zip(pull) {
            if f64::from(pull) != 0.0 {
                plank.angle += pull;
            }
        }
    }

    /// grKongo_801D7134 (0x801D7134) up to its JObj writes: settle, each
    /// plank's sag, and the end JObjs' tilt.
    pub fn step(&mut self, p: &Parameters, end_rotation_z: [f32; 2]) -> BridgeStep {
        self.settle(p);
        for plank in &mut self.planks {
            // 801D717C: fmul by 37.8 in double, frsp.
            plank.sag = (37.8 * f64::from(gekko_math::msl::tanf(-plank.angle))) as f32;
        }
        let mut step = BridgeStep {
            ends: [(0.0, 0.0); 2],
            jolts: [None; 2],
        };
        for (side, &i) in END_PLANKS.iter().enumerate() {
            let planks = &self.planks;
            // 801D71A4..801D7200: (sag_i - sag_(i-1)) / 6 + (sag_(i+1) -
            // sag_i) / 6 in single; times 0.5 then pi/4 in double.
            let left = (planks[i].sag - planks[i - 1].sag) / 6.0;
            let right = (planks[i + 1].sag - planks[i].sag) / 6.0;
            let mut angle =
                (std::f64::consts::FRAC_PI_4 * (0.5 * f64::from(left + right))) as f32;
            let tilt = planks[i].angle
                * ((DEG_TO_RAD * p.end_tilt) / (DEG_TO_RAD * p.plank_limit) - 1.0);
            angle = clamp(angle, DEG_TO_RAD * p.end_angle_limit);
            step.ends[side] = (tilt, angle);
            let change = angle - end_rotation_z[side];
            let size = if change < 0.0 { -change } else { change };
            if size > DEG_TO_RAD * p.end_jolt {
                step.jolts[side] = Some(-change);
            }
        }
        step
    }

    /// grKongo_801D7134's tail: the lines' vertical offsets, plank by
    /// plank, as `(line, sag)` (mpLib_80056758(line, 0, sag, 0, sag)).
    pub fn line_offsets(&self) -> impl Iterator<Item = (i32, f32)> + '_ {
        self.planks.iter().enumerate().flat_map(|(i, plank)| {
            let line = FIRST_LINE + 2 * i as i32;
            let outer = match i {
                0 => Some(line - 1),
                14 => Some(line + 1),
                _ => None,
            };
            std::iter::once((line, plank.sag)).chain(outer.map(|l| (l, plank.sag)))
        })
    }

    /// fn_801D7700 (0x801D7700): a fighter or stage enemy (collision type 1
    /// or 3) landing (ground kind 1) at `x` kicks the plank under it.
    /// retail 0x801D776C..0x801D77C4: fsubs, fdivs by 15, fdivs, fctiwz,
    /// then fmadds of x0A4 in degrees.
    pub fn land(&mut self, p: &Parameters, x: f32, left: f32, right: f32) {
        let segment = (right - left) / 15.0;
        let ratio = (x - left) / segment;
        let index = gekko_math::msl::fctiwz(ratio);
        let index = if index < 0 {
            0
        } else if index as u32 >= PLANK_COUNT as u32 {
            PLANK_COUNT - 1
        } else {
            index as usize
        };
        let plank = &mut self.planks[index];
        plank.velocity = fmadds(DEG_TO_RAD, p.plank_kick, plank.velocity);
    }
}

/// One swinging platform end (gp+C4/C8 or gp+D4/D8).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Swing {
    pub angle: f32,
    pub velocity: f32,
}

impl Swing {
    /// grKongo_801D77E0 (0x801D77E0): damp toward rest, clamp, settle.
    pub fn step(&mut self, p: &Parameters) {
        let damping = DEG_TO_RAD * p.swing_damping;
        if self.angle > 0.0 {
            let step = if self.velocity > 0.0 {
                damping
            } else {
                damping * 0.5
            };
            self.velocity -= step;
        } else if self.angle < 0.0 {
            let step = if self.velocity < 0.0 {
                damping
            } else {
                damping * 0.5
            };
            self.velocity += step;
        }
        self.angle += self.velocity;
        // 801D78D4: fnmsubs, x0B8 - k * x0AC, then times k.
        let limit = DEG_TO_RAD * fnmsubs(DEG_TO_RAD, p.end_angle_limit, p.swing_limit);
        if self.angle > limit {
            self.angle = limit;
            self.velocity = 0.0;
        } else if self.angle < -limit {
            self.angle = -limit;
            self.velocity = 0.0;
        } else {
            let size = if self.angle < 0.0 { -self.angle } else { self.angle };
            let speed = if self.velocity < 0.0 {
                -self.velocity
            } else {
                self.velocity
            };
            if size < damping && speed < damping {
                self.angle = 0.0;
                self.velocity = 0.0;
            }
        }
    }
}
