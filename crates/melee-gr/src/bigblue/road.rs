//! The road (map 0x22, grBigBlue_801E613C / 801E61C4): twelve segments
//! (grBb_TrackEntries), two drawn at a time, the current one and the next
//! placed at its end. Each tick the road's JObj scrolls left by
//! yakumono_param x6C and turns toward the floor under the stage's centre;
//! once the current segment's end passes x -300 (scaled) the next becomes
//! current and another is drawn (grBigBlue_801EB4AC). The loop segment
//! (11) switches its collision joints as the road turns over.
use super::{set_translate_x, set_translate_z, Body, Parameters, Scene, NO_FLOOR};
use gekko_math::fma::{fmadd, fmadds, fmsub, fnmsubs};
use hsd_types::Vec3;

/// `grBb_TrackEntry`: a segment's bone, the bones at its start and end,
/// and (filled at creation) the end's offset from the start.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segment {
    pub bone: usize,
    pub start: usize,
    pub end: usize,
    pub delta: Vec3,
}

const fn segment(bone: usize, start: usize, end: usize) -> Segment {
    Segment {
        bone,
        start,
        end,
        delta: Vec3::ZERO,
    }
}

/// grBb_TrackEntries (grbigblue.c:2316).
pub const SEGMENTS: [Segment; 12] = [
    segment(4, 6, 5),
    segment(19, 21, 20),
    segment(7, 9, 8),
    segment(10, 12, 11),
    segment(44, 46, 45),
    segment(1, 3, 2),
    segment(32, 34, 33),
    segment(35, 37, 36),
    segment(38, 40, 39),
    segment(41, 43, 42),
    segment(29, 31, 30),
    segment(13, 18, 17),
];
/// The loop (grBb_TrackEntries[11]).
pub const LOOP: usize = 11;
/// grBigBlue_801EB004's first segment, and the next.
const FIRST: usize = 4;
const SECOND: usize = 0;
/// The loop's entry, climb, top and exit joints (grBigBlue_801EBAF8).
const LOOP_ENTRY: [i32; 3] = [0x29, 0x2A, 0x2B];
const LOOP_CLIMB: [i32; 3] = [0x2C, 0x2D, 0x2E];
const LOOP_EXIT: [i32; 3] = [0x2F, 0x30, 0x31];

/// 801EB004's 1000 and 10 (the road's first position) and 801EBAF8's 300
/// (the switch), 20 and 120 (the floor searches), all times the scale.
const START_X: f32 = -1000.0;
const START_Y: f32 = 10.0;
const SWITCH_X: f32 = -300.0;
const FOLLOW_SEARCH: f32 = 20.0;
const FALL_SEARCH: f32 = 120.0;
/// 10 degrees (0.17453292): the road is level enough to switch segments.
const LEVEL: f32 = 0.174_532_92;
/// 0.2 degrees (0.0034906585): a turn too small to ease.
const SMALL_TURN: f32 = 0.003_490_658_5;
/// The loop's quarter turns (-0.5235988 and -2.7925267 radians).
const LOOP_QUARTER: f32 = -std::f32::consts::FRAC_PI_6;
const LOOP_HALF: f32 = -2.792_526_7;
/// A segment below this depth may not start the loop (801EB4AC).
const LOOP_DEPTH: f32 = -700.0;

/// grBigBlue_RoadVars (gp+C4..gp+FC) and the bitfield at gp+C4.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Road {
    /// `b0`: set by grBigBlue_801E613C, cleared when the countdown ends
    /// (fn_801E6124, deferred through Ground_801C10B8); until then only
    /// segments 0, 1 and 4 are drawn.
    pub starting: bool,
    /// grBb_TrackEntries with the creation's deltas.
    pub segments: [Segment; 12],
    /// `prev_lane` (7 bits; 0x7F before the first switch), `cur_lane`,
    /// `next_lane`.
    pub previous: u8,
    pub current: u8,
    pub next: u8,
    /// `b1`: the road is falling, its floor lost.
    pub falling: bool,
    /// `b2`: the loop has turned past level with its end ahead.
    pub looping: bool,
    /// `nibble_hi`: the loop's collision stage (0..4).
    pub loop_stage: u8,
    /// gp+C8, gp+D4: where the road's JObj is and was.
    pub position: Vec3,
    pub previous_position: Vec3,
    /// gp+E0: the road's last motion before it fell.
    pub drift: Vec3,
    /// gp+EC: the fall's vertical speed.
    pub fall_speed: f32,
    /// gp+F0: -1 climbing back up, 1 dipping back down, 0 free.
    pub direction: i16,
    /// gp+F8: the road's turn.
    pub rotation: f32,
}

impl Road {
    /// grBigBlue_801EB004 (0x801EB004), the deltas and the first two
    /// segments. The whole model is hidden and its JObj at unit scale; each
    /// segment bone takes the ground's scale before its ends are read.
    pub fn new<S: Scene>(parameters: &Parameters, scene: &mut S) -> Self {
        scene.set_hidden(Body::Road, true);
        scene.set_scale_of(Body::Road, 1.0);
        let scale = scene.scale();
        let mut segments = SEGMENTS;
        for segment in &mut segments {
            let bone = Body::RoadBone(segment.bone);
            scene.set_scale_of(bone, scale);
            let start = scene.world_position(Body::RoadBone(segment.start));
            let end = scene.world_position(Body::RoadBone(segment.end));
            segment.delta = melee_lb::vector::difference(end, start);
        }
        let zero = 0.0 * scale;
        let road = Self {
            starting: true,
            segments,
            previous: 0x7F,
            current: FIRST as u8,
            next: SECOND as u8,
            falling: false,
            looping: false,
            loop_stage: 0,
            position: Vec3::new(START_X * scale, START_Y * scale, zero),
            previous_position: Vec3::new(zero, zero, zero),
            drift: Vec3::ZERO,
            fall_speed: 0.0,
            direction: 0,
            rotation: zero,
        };
        let first = Body::RoadBone(segments[FIRST].bone);
        scene.set_hidden(first, false);
        let mut at = Vec3::new(0.0, parameters.road_height * scale, 0.0);
        scene.set_translation(first, at);
        let second = Body::RoadBone(segments[SECOND].bone);
        scene.set_hidden(second, false);
        let delta = segments[FIRST].delta;
        at = Vec3::new(at.x + delta.x, at.y + delta.y, at.z + delta.z);
        scene.set_translation(second, at);
        road
    }

    /// grBigBlue_801EBAF8 (0x801EBAF8): the road's step, then its JObj and
    /// the scenery's (maps 2 and 1) placed.
    pub fn step<S: Scene>(&mut self, parameters: &Parameters, scene: &mut S) {
        let scale = scene.scale();
        let center = Vec3::new(0.0, parameters.road_height * scale, 0.0);
        let previous = self.position;
        // The segment read below is the one current before any switch.
        let entry = self.segments[self.current as usize];
        let end = scene.world_position(Body::RoadBone(entry.end));
        if gekko_math::msl::fabsf(self.rotation) < LEVEL && end.x < SWITCH_X * scale {
            self.switch_segment(parameters, scene);
        }
        if gekko_math::msl::fabsf(self.rotation) < LEVEL {
            if end.x < 0.0 {
                if self.looping {
                    self.looping = false;
                }
            } else if !self.looping && self.current as usize == LOOP {
                self.looping = true;
            }
        }
        if self.current as usize == LOOP {
            self.step_loop(scene);
        }
        let search = if self.falling { FALL_SEARCH } else { FOLLOW_SEARCH };
        let (mut floor, normal) = scene.floor(center, search * scale);
        let target;
        let turn;
        if floor != NO_FLOOR && (!self.falling || floor > center.y) {
            // x70 is 99999 and the search spans 120 (scaled) about the
            // stage's centre: neither clamp can act.
            let ceiling = parameters.road_ceiling;
            if floor > ceiling {
                floor = ceiling;
            } else if floor < -ceiling {
                floor = -ceiling;
            }
            target = Vec3::new(center.x, floor, center.z);
            turn = if self.current as usize == LOOP && self.looping {
                -super::slope_angle(normal)
            } else {
                -self.rotation
            };
            self.falling = false;
        } else {
            if !self.falling {
                self.drift = melee_lb::vector::difference(self.previous_position, self.position);
                let drop = parameters.road_drop * scale;
                self.fall_speed = gekko_math::msl::sinf(self.rotation) * drop;
                self.falling = true;
            }
            // retail 0x801EBED4 / 0x801EBEF0: fnmsubs.
            self.fall_speed = if floor != NO_FLOOR {
                fnmsubs(3.0, parameters.road_fall * scale, self.fall_speed)
            } else {
                fnmsubs(parameters.road_fall, scale, self.fall_speed)
            };
            target = Vec3::new(center.x, center.y + self.fall_speed, center.z);
            turn = -self.rotation;
        }
        let mut angular = turn;
        if turn > 0.0 {
            unimplemented!("grbigblue.c:2822: the road turning back up (a positive turn)");
        } else if gekko_math::msl::fabsf(turn) > SMALL_TURN {
            angular *= parameters.road_turn;
        }
        let mut v = Vec3::new(
            self.position.x - target.x,
            self.position.y - target.y,
            self.position.z - target.z,
        );
        if angular != 0.0 {
            let s = f64::from(gekko_math::msl::sinf(angular));
            let c = f64::from(gekko_math::msl::cosf(angular));
            let (x, y) = (f64::from(v.x), f64::from(v.y));
            // retail 0x801EBFAC / 0x801EBFB0: double fmsub / fmadd.
            v.x = fmsub(x, c, y * s) as f32;
            v.y = fmadd(x, s, y * c) as f32;
        }
        // lbVector_Add(&v, &center): `v` is the road's position from here.
        let placed = Vec3::new(v.x + center.x, v.y + center.y, v.z + center.z);
        self.position = placed;
        let old = self.rotation;
        self.rotation = old + angular;
        if !self.looping && old > 0.0 && self.rotation <= 0.0 {
            self.rotation = 0.0;
        }
        if f64::from(self.rotation) > std::f64::consts::PI {
            // The turn is never positive (a positive one is zeroed above), so
            // the rotation only falls; the +TAU wrap below leaves it under pi.
            unreachable!("grbigblue.c:2865: the road's rotation past pi");
        } else if f64::from(self.rotation) < -std::f64::consts::PI {
            self.rotation = (f64::from(self.rotation) + std::f64::consts::TAU) as f32;
        }
        // retail 0x801EC070: fnmsubs.
        self.position.x = fnmsubs(parameters.road_scroll, scale, self.position.x);
        let correction = 0.6 * ((entry.delta.z / entry.delta.x) * (parameters.road_scroll * scale));
        if end.z > correction {
            self.position.z -= correction;
        } else if end.z < -correction {
            self.position.z += correction;
        } else {
            self.position.z -= end.z;
        }
        scene.set_translation(Body::Road, self.position);
        scene.set_rotation_z(Body::Road, self.rotation);
        // lbVector_Rotate(&v, 4, -rotation), keep y, rotate back.
        use melee_lb::vector::{rotate_about, Axis};
        let mut carry = rotate_about(placed, Axis::Z, -self.rotation);
        carry.z = 0.0;
        carry.x = 0.0;
        carry = rotate_about(carry, Axis::Z, self.rotation);
        for map in [2, 1] {
            scene.set_translation(Body::Scenery(map), carry);
            scene.set_rotation_z(Body::Scenery(map), self.rotation);
        }
        self.previous_position = previous;
    }

    /// fn_801E6124 (0x801E6124): the countdown is over.
    pub fn start(&mut self) {
        self.starting = false;
    }

    /// 801EBAF8's loop stages: the climb's joints replace the entry's as the
    /// road turns past a quarter, the exit's past a half.
    fn step_loop<S: Scene>(&mut self, scene: &mut S) {
        match self.loop_stage {
            0 => self.loop_stage = 1,
            1 => {
                for joint in LOOP_ENTRY {
                    scene.enable_joint(joint);
                }
                // 0x30 twice (0x801EBCAC, 0x801EBCB4); 0x31 stays.
                for joint in [0x2C, 0x2D, 0x2E, 0x2F, 0x30, 0x30] {
                    scene.disable_joint(joint);
                }
                self.loop_stage = 2;
            }
            2 if self.rotation < LOOP_QUARTER => {
                for joint in LOOP_CLIMB {
                    scene.enable_joint(joint);
                }
                self.loop_stage = 3;
            }
            3 if self.rotation < LOOP_HALF => {
                for joint in LOOP_ENTRY {
                    scene.disable_joint(joint);
                }
                for joint in LOOP_EXIT {
                    scene.enable_joint(joint);
                }
                self.loop_stage = 4;
            }
            _ => {}
        }
    }

    /// grBigBlue_801EB4AC (0x801EB4AC): the current segment hides, the next
    /// becomes current (the road's JObj takes its x and z), and another is
    /// drawn and placed at its end.
    fn switch_segment<S: Scene>(&mut self, parameters: &Parameters, scene: &mut S) {
        let scale = scene.scale();
        scene.set_hidden(Body::RoadBone(self.segments[self.current as usize].bone), true);
        self.previous = self.current;
        self.current = self.next;
        let current = Body::RoadBone(self.segments[self.next as usize].bone);
        let at = scene.world_position(current);
        self.position.x = at.x;
        set_translate_x(scene, current, 0.0);
        self.position.z = at.z;
        set_translate_z(scene, current, 0.0);
        match self.direction {
            1 => {
                if self.position.y < 0.25 * (parameters.road_high * scale) {
                    self.direction = 0;
                }
            }
            -1 => {
                if self.position.y > 0.25 * -(parameters.road_low * scale) {
                    self.direction = 0;
                }
            }
            _ => {
                if self.position.y < -(parameters.road_low * scale) {
                    self.direction = -1;
                } else if self.position.y > parameters.road_high * scale {
                    unimplemented!("grbigblue.c:2488: the road above yakumono_param x8 (climbing only)");
                }
            }
        }
        let mut attempts = 0;
        let pick = loop {
            attempts += 1;
            if attempts > 1001 {
                unimplemented!("grbigblue.c:2414: no road segment fits in a thousand draws");
            }
            let pick = scene.rng().randi(12) as usize;
            let candidate = self.segments[pick];
            if pick == LOOP && self.position.y < LOOP_DEPTH {
                continue;
            }
            if self.current as usize == pick {
                continue;
            }
            if self.starting {
                // The road's first switch comes 22 ticks after GO in every
                // match: its motion reads only its own joints, no draw or
                // player, so fn_801E6124 has always run by then.
                unreachable!("grbigblue.c:2520: a road segment drawn before the countdown ends");
            }
            if self.direction == 1 && candidate.delta.y < 0.0 {
                continue;
            }
            if self.direction == -1 && candidate.delta.y > 0.0 {
                continue;
            }
            // retail 0x801EB7E0: fmadds.
            let reach = (at.y + self.segments[self.current as usize].delta.y) + candidate.delta.y;
            if reach < fmadds(parameters.road_high, scale, self.position.y) {
                continue;
            }
            break pick;
        };
        self.next = pick as u8;
        let mut at = scene.translation(current);
        let next = Body::RoadBone(self.segments[pick].bone);
        scene.set_hidden(next, false);
        let delta = self.segments[self.current as usize].delta;
        at = Vec3::new(at.x + delta.x, at.y + delta.y, at.z + delta.z);
        scene.set_translation(next, at);
        if pick == LOOP {
            for joint in LOOP_ENTRY {
                scene.enable_joint(joint);
            }
            for joint in [0x2C, 0x2D, 0x2E, 0x2F, 0x30, 0x31] {
                scene.disable_joint(joint);
            }
            self.loop_stage = 0;
        }
        for &joint in arriving_joints(pick) {
            scene.snapshot_joint(joint);
        }
        scene.stitch_joints();
    }
}

/// 801EB4AC's switch: the drawn segment's joints take their positions as
/// their previous ones (mpLib_80057424).
fn arriving_joints(segment: usize) -> &'static [i32] {
    match segment {
        0 => &[35],
        1 => &[50],
        2 => &[38, 39, 40],
        3 => &[36, 37],
        4 => &[64, 65],
        5 => &[33],
        6 => &[52, 53, 54],
        7 => &[55, 56, 57],
        8 => &[58, 59, 60],
        9 => &[61, 62, 63],
        10 => &[51],
        11 => &[41, 42, 43, 44, 45, 46, 47, 48, 49],
        _ => unreachable!("road segment {segment}"),
    }
}
