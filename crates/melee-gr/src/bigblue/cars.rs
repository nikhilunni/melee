//! The racing cars (map 0x21, grBigBlue_801E6364 / 801EF424). Four lanes
//! (`grBigBlue_CarLane`) drive thirty car bones: a lane holds one bone
//! while it is on the road (`ranks[slot] == 1`), and the free bones are
//! ranked to enter from the right (0) or the left (2). Each tick the
//! arrivals are managed (grBigBlue_801ECB50), every lane steps
//! (grBigBlue_801ED694: its manoeuvre, speed, bob and ride over the road),
//! then the lanes are pushed apart (801EF424's ten passes).
use super::{rand_range, to_int, zrandi, Body, Parameters, Scene, NO_FLOOR};
use gekko_math::fma::{fmadds, fmsubs, fnmsubs};
use hsd_types::Vec3;

/// grFZeroCar's thirty car bones.
pub const SLOTS: usize = 30;
/// `grBigBlue_CarVars.lanes`.
pub const LANES: usize = 4;
/// lbl_803E2DFC: the collision joint of each car bone.
pub const fn slot_joint(slot: usize) -> i32 {
    slot as i32 + 3
}

/// A lane's manoeuvre (`state`, six bits of gp+D4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Manoeuvre {
    /// 1: off the road; its bone hidden.
    Off,
    /// 2 / 3: pulling ahead to the right, dropping back to the left.
    Ahead,
    Behind,
    /// 4: holding its place in the formation.
    Hold,
    /// 5 / 6: arriving from the right / left, fading in.
    ArriveRight,
    ArriveLeft,
    /// 7 / 8: leaving to the right / left.
    LeaveRight,
    LeaveLeft,
    /// 9: fading out past the blast zone.
    Fade,
    /// 10: the one car slowing to the stage's middle.
    Pace,
}

impl Manoeuvre {
    pub fn from_state(state: u8) -> Self {
        match state {
            1 => Self::Off,
            2 => Self::Ahead,
            3 => Self::Behind,
            4 => Self::Hold,
            5 => Self::ArriveRight,
            6 => Self::ArriveLeft,
            7 => Self::LeaveRight,
            8 => Self::LeaveLeft,
            9 => Self::Fade,
            10 => Self::Pace,
            _ => unimplemented!("grbigblue.c: car lane state {state}"),
        }
    }
    pub fn state(self) -> u8 {
        match self {
            Self::Off => 1,
            Self::Ahead => 2,
            Self::Behind => 3,
            Self::Hold => 4,
            Self::ArriveRight => 5,
            Self::ArriveLeft => 6,
            Self::LeaveRight => 7,
            Self::LeaveLeft => 8,
            Self::Fade => 9,
            Self::Pace => 10,
        }
    }
    /// States 1, 7 and 8: a lane not in the formation.
    fn outside(self) -> bool {
        matches!(self, Self::Off | Self::LeaveRight | Self::LeaveLeft)
    }
}

/// `grBigBlue_CarLane` (0x40 bytes from gp+D4).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lane {
    pub manoeuvre: Manoeuvre,
    /// Bit 0x02 of gp+D4: off the road, falling.
    pub airborne: bool,
    /// `collision_slot`: the car bone.
    pub slot: usize,
    /// +04 / +08: the formation place and the offset from it.
    pub place: f32,
    pub offset: f32,
    /// +0C.
    pub position: Vec3,
    /// +18: the fade (only sound reads it).
    pub alpha: f32,
    /// +1C: the ticks to the next manoeuvre (also a pace's lead).
    pub wait: i32,
    /// +20 / +24: the sink a landing fighter gives and its depth.
    pub sink_speed: f32,
    pub sink: f32,
    /// +28 / +2C.
    pub velocity: f32,
    pub acceleration: f32,
    /// +30 / +34: the bob's phase and amplitude.
    pub phase: f32,
    pub bob: f32,
    /// +38: the vertical speed while airborne.
    pub vertical_speed: f32,
}

impl Lane {
    const OFF: Self = Self {
        manoeuvre: Manoeuvre::Off,
        airborne: false,
        slot: 0,
        place: 0.0,
        offset: 0.0,
        position: Vec3::ZERO,
        alpha: 0.0,
        wait: 0,
        sink_speed: 0.0,
        sink: 0.0,
        velocity: 0.0,
        acceleration: 0.0,
        phase: 0.0,
        bob: 0.0,
        vertical_speed: 0.0,
    };
}

/// Car states 2 and 3 are never entered: grBigBlue_801EDF44 returns 1, 4,
/// 7, 8 or 9, grBigBlue_801ECB50 sets 4 and 10 and asks for 5 and 6, and
/// nothing else calls grBigBlue_801EE398.
const NO_AHEAD_OR_BEHIND: &str =
    "grbigblue.c:4014/4272: no caller enters car state 2 or 3";

/// A car bone's rank (`grBigBlue_CarVars.ranks`).
pub const RANK_RIGHT: u8 = 0;
pub const RANK_ON_ROAD: u8 = 1;
pub const RANK_LEFT: u8 = 2;

/// 801EC6C0's sound table (lbl_803E3010): only the draw matters.
const ENGINE_SOUNDS: i32 = 4;
/// A pace car keeps out of the middle 60 units (grBigBlue_FindClosestCar).
const PACE_CLEARANCE: f32 = 60.0;
/// 801EE398's pace lead and a leaving car's wait.
const PACE_LEAD: i32 = 20;
const LEAVE_WAIT: i32 = 1000;
/// The depth past which a lane is dropped (801EDF44).
const LOST_DEPTH: f32 = -2000.0;
/// The floor searches: an arrival's, an airborne car's, a riding car's.
const ARRIVAL_SEARCH: f32 = 1000.0;
const FALL_SEARCH: f32 = 500.0;
const RIDE_SEARCH: f32 = 20.0;
/// One degree (0.017453292) and a fade step (1/60).
const DEGREE: f32 = 0.017_453_292;
const FADE_STEP: f32 = 1.0 / 60.0;

/// grBigBlue_CarVars: the lanes, the ranks and the arrival timer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cars {
    pub lanes: [Lane; LANES],
    pub ranks: [u8; SLOTS],
    /// gp+D0 (s16).
    pub arrival_timer: i16,
}

/// The road's vertical motion this tick (map 0x22's gp+CC - gp+D8), which
/// an airborne car follows.
#[derive(Clone, Copy, Debug)]
pub struct RoadMotion {
    pub y: f32,
    pub previous_y: f32,
}

impl Cars {
    /// grBigBlue_801EC6C0 (0x801EC6C0) and the arrival timer
    /// (grBigBlue_801E6364's tail): every bone hidden and ranked at
    /// random, then the starting cars placed in a formation across the
    /// road, each on a bone no other holds.
    pub fn new<S: Scene>(parameters: &Parameters, scene: &mut S) -> Self {
        let mut cars = Self {
            lanes: [Lane::OFF; LANES],
            ranks: [0; SLOTS],
            arrival_timer: 0,
        };
        for slot in 0..SLOTS {
            scene.set_hidden(Body::Car(slot), true);
            cars.ranks[slot] = if scene.rng().randi(2) != 0 {
                RANK_RIGHT
            } else {
                RANK_LEFT
            };
        }
        // `if (x18 == 0) x18 = 1`.
        let starting = parameters.starting_cars.max(1);
        for car in 0..LANES {
            if (car as i32) < starting {
                cars.lanes[car].manoeuvre = Manoeuvre::Hold;
                let slot = loop {
                    let slot = scene.rng().randi(SLOTS as i32) as usize;
                    if cars.lanes[..car].iter().all(|lane| lane.slot != slot) {
                        break slot;
                    }
                };
                cars.start_lane(parameters, scene, car, slot);
            } else {
                cars.lanes[car].manoeuvre = Manoeuvre::Off;
            }
        }
        let [low, high] = parameters.arrival_wait;
        cars.arrival_timer = rand_range(scene.rng(), low, high) as i16;
        cars
    }

    /// grBigBlue_801EC6C0_inline: a starting car's place in the formation
    /// (0x801EC8A8 / 0x801EC958 fmadds), its ride height (0x801EC8CC).
    fn start_lane<S: Scene>(&mut self, parameters: &Parameters, scene: &mut S, car: usize, slot: usize) {
        let scale = scene.scale();
        let lane = &mut self.lanes[car];
        lane.slot = slot;
        lane.airborne = false;
        let place = formation_place(parameters, scale, car as i32 + 1, parameters.starting_cars + 1);
        lane.position.x = place;
        lane.position.y = fmadds(parameters.road_height, scale, parameters.car_ride_height * scale);
        lane.position.z = 0.0;
        lane.place = formation_place(parameters, scale, car as i32 + 1, parameters.starting_cars + 1);
        lane.offset = 0.0;
        lane.sink_speed = 0.0;
        lane.sink = 0.0;
        lane.velocity = 0.0;
        lane.acceleration = 0.0;
        lane.phase = (2.0 * std::f64::consts::PI * f64::from(scene.rng().randf())) as f32;
        lane.bob = parameters.bob_cruising * scale;
        lane.vertical_speed = 0.0;
        let [low, high] = parameters.manoeuvre_wait.map(to_int);
        lane.wait = rand_range(scene.rng(), low, high);
        scene.rng().randi(ENGINE_SOUNDS);
        lane.alpha = 1.0;
        scene.set_hidden(Body::Car(slot), false);
        scene.set_translation(Body::Car(slot), lane.position);
        self.ranks[slot] = RANK_ON_ROAD;
    }

    /// grBigBlue_801EF424 (0x801EF424): arrivals, each lane's step, then
    /// the lanes pushed at least `car_gap` apart.
    pub fn step<S: Scene>(&mut self, parameters: &Parameters, scene: &mut S, road: RoadMotion) {
        self.manage_arrivals(parameters, scene);
        for lane in 0..LANES {
            self.step_lane(parameters, scene, road, lane);
        }
        let scale = scene.scale();
        let mut changed = true;
        let mut pass = 0;
        while pass < 10 && changed {
            changed = false;
            for i in 0..LANES {
                if self.lanes[i].manoeuvre == Manoeuvre::Off {
                    continue;
                }
                for j in 0..LANES {
                    if i == j || self.lanes[j].manoeuvre == Manoeuvre::Off {
                        continue;
                    }
                    let mut diff = self.lanes[i].position.x - self.lanes[j].position.x;
                    let distance = if diff < 0.0 { -diff } else { diff };
                    let gap = scale * parameters.car_gap as f32;
                    if distance < gap {
                        let push = if diff > 0.0 { gap } else { -gap };
                        diff -= push;
                        changed = true;
                        diff = (f64::from(diff) * 0.5) as f32;
                        self.lanes[i].offset -= diff;
                        self.lanes[i].position.x -= diff;
                        self.lanes[j].offset += diff;
                        self.lanes[j].position.x += diff;
                    }
                }
            }
            pass += 1;
        }
    }

    /// fn_801EF60C (0x801EF60C): a fighter landing on a car's floor sinks
    /// it by its `coll x50` times yakumono_param x44.
    pub fn landed(&mut self, parameters: &Parameters, joint: i32, x50: f32) {
        let slot = (joint - 3) as usize;
        assert!(slot < SLOTS, "grbigblue.c:3801: car joint {joint}");
        for lane in &mut self.lanes {
            if lane.slot == slot {
                lane.sink_speed = -x50 * parameters.landing_push;
            }
        }
    }

    fn count_ranks(&self, rank: u8) -> i32 {
        self.ranks.iter().filter(|&&r| r == rank).count() as i32
    }

    /// The `pick`-th bone (from zero) of a rank, as the retail loops walk.
    fn nth_rank(&self, rank: u8, pick: i32) -> Option<usize> {
        let mut pick = pick;
        for (slot, &r) in self.ranks.iter().enumerate() {
            if r == rank {
                pick -= 1;
                if pick < 0 {
                    return Some(slot);
                }
            }
        }
        None
    }

    /// grBigBlue_801ECB50 (0x801ECB50).
    fn manage_arrivals<S: Scene>(&mut self, parameters: &Parameters, scene: &mut S) {
        let mut free = self.count_ranks(RANK_RIGHT);
        let mut reserved = self.count_ranks(RANK_LEFT);
        // Keep both ranks stocked.
        if free == 0 {
            let pick = zrandi(scene.rng(), reserved);
            if let Some(slot) = self.nth_rank(RANK_LEFT, pick) {
                self.ranks[slot] = RANK_RIGHT;
                free += 1;
            }
        } else if reserved == 0 {
            let pick = zrandi(scene.rng(), free);
            if let Some(slot) = self.nth_rank(RANK_RIGHT, pick) {
                self.ranks[slot] = RANK_LEFT;
                reserved += 1;
            }
        }
        let active = self
            .lanes
            .iter()
            .filter(|lane| !lane.manoeuvre.outside())
            .count();
        if active == 1 {
            self.arrival_timer = self.arrival_timer.wrapping_sub(1);
        }
        // grBigBlue_FindClosestCar: the formation car nearest the middle
        // (beyond 60) paces, unless one does or one is within 60.
        let mut pacing = false;
        let mut closest = None;
        let mut closest_distance = f32::MAX;
        for (index, lane) in self.lanes.iter().enumerate() {
            if lane.manoeuvre == Manoeuvre::Pace {
                pacing = true;
                break;
            }
            if !lane.manoeuvre.outside() {
                let x = lane.position.x;
                let distance = if x < 0.0 { -x } else { x };
                if distance > PACE_CLEARANCE {
                    if closest_distance > distance {
                        closest_distance = distance;
                        closest = Some(index);
                    }
                } else {
                    pacing = true;
                    break;
                }
            }
        }
        if !pacing {
            if let Some(pace) = closest {
                self.lanes[pace].manoeuvre = Manoeuvre::Pace;
                let x = self.lanes[pace].position.x;
                for lane in &mut self.lanes {
                    let leaving_behind = (lane.manoeuvre == Manoeuvre::LeaveRight
                        && lane.position.x < x)
                        || (lane.manoeuvre == Manoeuvre::LeaveLeft && lane.position.x > x);
                    if leaving_behind {
                        lane.manoeuvre = Manoeuvre::Hold;
                    }
                }
            }
        }
        let timer = self.arrival_timer;
        self.arrival_timer = timer.wrapping_sub(1);
        if timer >= 0 {
            return;
        }
        let Some(idle) = self
            .lanes
            .iter()
            .position(|lane| lane.manoeuvre == Manoeuvre::Off)
        else {
            return;
        };
        let leaving_right = self
            .lanes
            .iter()
            .filter(|lane| lane.manoeuvre == Manoeuvre::LeaveRight)
            .count();
        let leaving_left = self
            .lanes
            .iter()
            .filter(|lane| lane.manoeuvre == Manoeuvre::LeaveLeft)
            .count();
        let direction = if leaving_right == 0 && leaving_left == 0 {
            if scene.rng().randi(2) != 0 {
                1
            } else {
                -1
            }
        } else if leaving_right != 0 {
            -1
        } else {
            1
        };
        let (rank, count, status) = if direction == 1 {
            (RANK_RIGHT, free, Manoeuvre::ArriveRight)
        } else {
            (RANK_LEFT, reserved, Manoeuvre::ArriveLeft)
        };
        let pick = zrandi(scene.rng(), count);
        if self.nth_rank(rank, pick).is_some() && self.change(parameters, scene, idle, status) {
            let [low, high] = parameters.arrival_wait;
            self.arrival_timer = rand_range(scene.rng(), low, high) as i16;
        }
    }

    /// grBigBlue_801ED694 (0x801ED694): one lane's tick.
    fn step_lane<S: Scene>(&mut self, parameters: &Parameters, scene: &mut S, road: RoadMotion, index: usize) {
        if let Some(status) = self.next_manoeuvre(parameters, scene, index) {
            self.change(parameters, scene, index, status);
        }
        if !self.accelerate(parameters, scene, index) {
            return;
        }
        let scale = scene.scale();
        let body = Body::Car(self.lanes[index].slot);
        let mut tilt = scene.rotation_z(body);
        // The formation place this lane steers toward.
        let target = if matches!(
            self.lanes[index].manoeuvre,
            Manoeuvre::LeaveRight | Manoeuvre::LeaveLeft
        ) {
            self.lanes[index].place
        } else {
            let mut active = 0;
            let mut behind = 0;
            for (other, lane) in self.lanes.iter().enumerate() {
                if !lane.manoeuvre.outside() {
                    active += 1;
                    if other != index && lane.position.x < self.lanes[index].position.x {
                        behind += 1;
                    }
                }
            }
            formation_place(parameters, scale, behind + 1, active + 1)
        };
        let lane = &mut self.lanes[index];
        let current = lane.place;
        let diff = target - current;
        lane.place = if diff < -0.5 {
            current - 0.5
        } else if diff > 0.5 {
            lane.place + 0.5
        } else {
            target
        };
        lane.velocity += lane.acceleration;
        let top = parameters.car_top_speed * scale;
        if lane.velocity > top {
            lane.velocity = parameters.car_top_speed * scale;
        } else if lane.velocity < -(parameters.car_top_speed * scale) {
            lane.velocity = -(parameters.car_top_speed * scale);
        }
        lane.offset += lane.velocity;
        lane.position.x = lane.place + lane.offset;
        if !lane.airborne {
            // retail 0x801EDA24: fmadds.
            lane.sink_speed = fmadds(parameters.sink_spring, scale, lane.sink_speed);
            lane.sink += lane.sink_speed;
            if lane.sink > 0.0 {
                lane.sink = 0.0;
                lane.sink_speed = 0.0;
            } else if lane.sink < -(parameters.sink_depth * scale) {
                lane.sink = -(parameters.sink_depth * scale);
            }
        } else {
            lane.sink = 0.0;
            lane.sink_speed = 0.0;
        }
        let bob = if lane.airborne {
            0.0
        } else if lane.velocity > 0.0 {
            bob_amplitude(parameters, scale, lane.velocity, parameters.bob_accelerating)
        } else if lane.velocity < 0.0 {
            bob_amplitude(parameters, scale, lane.velocity, parameters.bob_backing)
        } else {
            parameters.bob_cruising * scale
        };
        // retail 0x801EDB94 / 0x801EDBAC: fmadds.
        lane.bob = fmadds(parameters.bob_follow, bob - lane.bob, lane.bob);
        lane.phase = fmadds(DEGREE, parameters.bob_rate, lane.phase);
        if f64::from(lane.phase) > std::f64::consts::TAU {
            lane.phase = (f64::from(lane.phase) - std::f64::consts::TAU) as f32;
        }
        let swing = lane.bob * gekko_math::msl::sinf(lane.phase);
        // retail 0x801EDC00: fnmsubs.
        let resting =
            (fnmsubs(parameters.car_ride_height, scale, lane.position.y) - lane.sink) - swing;
        let position = lane.position;
        let (floor, normal);
        let mut landed = false;
        if lane.airborne {
            (floor, normal) = scene.floor(position, FALL_SEARCH);
            let lane = &mut self.lanes[index];
            if floor != NO_FLOOR && floor > resting && lane.vertical_speed < 0.0 {
                lane.airborne = false;
                landed = true;
            } else {
                // retail 0x801EDC8C / 0x801EDCAC: fnmsubs.
                lane.vertical_speed = if floor != NO_FLOOR {
                    fnmsubs(3.0, parameters.car_gravity * scale, lane.vertical_speed)
                } else {
                    fnmsubs(parameters.car_gravity, scale, lane.vertical_speed)
                };
                lane.position.y += lane.vertical_speed;
                lane.position.y += road.y - road.previous_y;
            }
        } else {
            (floor, normal) = scene.floor(position, RIDE_SEARCH * scale);
            let lane = &mut self.lanes[index];
            if floor != NO_FLOOR {
                // retail 0x801EDD58: fmadds.
                lane.position.y =
                    swing + (lane.sink + fmadds(parameters.car_ride_height, scale, floor));
            } else {
                lane.airborne = true;
                let launch = parameters.car_launch * scale;
                lane.vertical_speed = gekko_math::msl::sinf(tilt) * launch;
            }
        }
        let _ = landed;
        let lane = &mut self.lanes[index];
        if floor != NO_FLOOR && !lane.airborne {
            let angle = super::slope_angle(normal);
            // retail 0x801EDDD4: fmadds.
            tilt = fmadds(parameters.tilt_follow, angle - tilt, tilt);
        } else {
            // retail 0x801EDDEC: fmsubs.
            tilt += parameters.level_follow * fmsubs(DEGREE, parameters.level_angle, tilt);
        }
        scene.set_translation(body, lane.position);
        scene.set_rotation_z(body, tilt);
    }

    /// grBigBlue_801EDF44 (0x801EDF44): the manoeuvre a lane changes to
    /// this tick, if any.
    fn next_manoeuvre<S: Scene>(&mut self, parameters: &Parameters, scene: &mut S, index: usize) -> Option<Manoeuvre> {
        let scale = scene.scale();
        let limits = scene.limits();
        let lane = self.lanes[index];
        let status = match lane.manoeuvre {
            Manoeuvre::Off => None,
            Manoeuvre::Fade => (lane.alpha == 0.0).then_some(Manoeuvre::Off),
            Manoeuvre::LeaveRight => {
                // retail 0x801EDFE0: fmadds.
                let edge = fmadds(parameters.car_margin, scale, limits.blast_right);
                (lane.position.x > edge).then(|| leave(lane.alpha))
            }
            Manoeuvre::LeaveLeft => {
                // retail 0x801EE028: fnmsubs.
                let edge = fnmsubs(parameters.car_margin, scale, limits.blast_left);
                (lane.position.x < edge).then(|| leave(lane.alpha))
            }
            _ => {
                let mut result = None;
                let waited = self.lanes[index].wait;
                self.lanes[index].wait = waited.wrapping_sub(1);
                if waited < 0 {
                    while result.is_none() {
                        result = self.draw_manoeuvre(scene, index);
                    }
                }
                let x = self.lanes[index].position.x;
                for (other, lane) in self.lanes.iter().enumerate() {
                    if other == index {
                        continue;
                    }
                    if lane.manoeuvre == Manoeuvre::LeaveRight && lane.position.x < x {
                        result = Some(Manoeuvre::LeaveRight);
                        continue;
                    }
                    if lane.manoeuvre == Manoeuvre::LeaveLeft && lane.position.x > x {
                        result = Some(Manoeuvre::LeaveLeft);
                    }
                }
                if self.lanes[index].position.y < LOST_DEPTH {
                    result = Some(Manoeuvre::Off);
                }
                result
            }
        };
        status
    }

    /// 801EDF44's draw: three in four hold; otherwise leave toward the
    /// side with fewer cars ahead of the pace car, or draw again.
    fn draw_manoeuvre<S: Scene>(&self, scene: &mut S, index: usize) -> Option<Manoeuvre> {
        if scene.rng().randi(4) != 3 {
            return Some(Manoeuvre::Hold);
        }
        let x = self.lanes[index].position.x;
        let (mut right, mut left, mut right_pace, mut left_pace) = (0, 0, 0, 0);
        for (other, lane) in self.lanes.iter().enumerate() {
            if other == index || lane.manoeuvre.outside() {
                continue;
            }
            let pace = lane.manoeuvre == Manoeuvre::Pace;
            if lane.position.x > x {
                if pace {
                    right_pace += 1;
                } else {
                    right += 1;
                }
            } else if lane.position.x < x {
                if pace {
                    left_pace += 1;
                } else {
                    left += 1;
                }
            }
        }
        if right != 0 {
            if left != 0 {
                if right_pace != 0 {
                    if left_pace != 0 {
                        return None;
                    }
                    Some(Manoeuvre::LeaveLeft)
                } else if left_pace != 0 || scene.rng().randi(2) != 0 {
                    // The coin is tossed only with no pace car either side.
                    Some(Manoeuvre::LeaveRight)
                } else {
                    Some(Manoeuvre::LeaveLeft)
                }
            } else {
                Some(Manoeuvre::LeaveLeft)
            }
        } else if left != 0 {
            Some(Manoeuvre::LeaveRight)
        } else {
            None
        }
    }

    /// grBigBlue_801EE398 (0x801EE398): enter a manoeuvre; false when an
    /// arrival found no floor or no free bone (or for the fade).
    fn change<S: Scene>(&mut self, parameters: &Parameters, scene: &mut S, index: usize, status: Manoeuvre) -> bool {
        let scale = scene.scale();
        match status {
            Manoeuvre::Off => {
                let lane = &mut self.lanes[index];
                scene.set_hidden(Body::Car(lane.slot), true);
                self.ranks[lane.slot] = if lane.position.x > 0.0 {
                    RANK_RIGHT
                } else {
                    RANK_LEFT
                };
                lane.manoeuvre = status;
                true
            }
            Manoeuvre::Pace => {
                self.lanes[index].wait = PACE_LEAD;
                self.lanes[index].manoeuvre = status;
                true
            }
            Manoeuvre::Ahead | Manoeuvre::Behind => unreachable!("{NO_AHEAD_OR_BEHIND}"),
            Manoeuvre::Hold => {
                let [low, high] = parameters.manoeuvre_wait.map(to_int);
                self.lanes[index].wait = rand_range(scene.rng(), low, high);
                self.lanes[index].manoeuvre = status;
                true
            }
            Manoeuvre::ArriveRight | Manoeuvre::ArriveLeft => {
                let limits = scene.limits();
                // retail 0x801EE540 fmadds / 0x801EEA04 fnmsubs.
                let x = if status == Manoeuvre::ArriveRight {
                    fmadds(parameters.car_margin, scale, limits.blast_right)
                } else {
                    fnmsubs(parameters.car_margin, scale, limits.blast_left)
                };
                let (floor, _) = scene.floor(Vec3::new(x, 0.0, 0.0), ARRIVAL_SEARCH);
                if floor == NO_FLOOR {
                    return false;
                }
                let count = self.count_ranks(RANK_RIGHT);
                if count == 0 {
                    return false;
                }
                let pick = zrandi(scene.rng(), count);
                let slot = self.nth_rank(RANK_RIGHT, pick).unwrap_or(SLOTS);
                let lane = &mut self.lanes[index];
                lane.slot = slot;
                lane.airborne = false;
                lane.position.x = x;
                // retail 0x801EE7F4 / 0x801EECB8: fmadds.
                lane.position.y = fmadds(parameters.car_ride_height, scale, floor);
                lane.position.z = 0.0;
                lane.place = x;
                lane.offset = 0.0;
                lane.sink_speed = 0.0;
                lane.sink = 0.0;
                lane.velocity = 0.0;
                lane.acceleration = 0.0;
                lane.phase = (std::f64::consts::TAU * f64::from(scene.rng().randf())) as f32;
                lane.bob = parameters.bob_cruising * scale;
                lane.vertical_speed = 0.0;
                let [low, high] = parameters.manoeuvre_wait.map(to_int);
                lane.wait = rand_range(scene.rng(), low, high);
                assert!(slot < SLOTS, "grBigBlue_801EE398: no free car bone");
                self.ranks[slot] = RANK_ON_ROAD;
                let lane = &mut self.lanes[index];
                lane.alpha = 0.0;
                scene.rng().randi(ENGINE_SOUNDS);
                scene.set_hidden(Body::Car(slot), false);
                scene.set_translation(Body::Car(slot), lane.position);
                lane.manoeuvre = status;
                true
            }
            Manoeuvre::LeaveRight | Manoeuvre::LeaveLeft => {
                self.lanes[index].wait = LEAVE_WAIT;
                self.lanes[index].manoeuvre = status;
                true
            }
            Manoeuvre::Fade => {
                self.lanes[index].manoeuvre = status;
                false
            }
        }
    }

    /// grBigBlue_801EEF00 (0x801EEF00): a lane's acceleration for its
    /// manoeuvre; false for a lane off the road.
    fn accelerate<S: Scene>(&mut self, parameters: &Parameters, scene: &mut S, index: usize) -> bool {
        let scale = scene.scale();
        let lane = &mut self.lanes[index];
        let push = || parameters.car_acceleration * scale;
        // How many ticks of full acceleration the current speed is.
        let speed_ticks = |velocity: f32| {
            let speed = if velocity < 0.0 { -velocity } else { velocity };
            speed / (parameters.car_acceleration * scale)
        };
        match lane.manoeuvre {
            Manoeuvre::Off => return false,
            Manoeuvre::Pace => {
                let ticks = speed_ticks(lane.velocity);
                let slow = lane.wait as f32 > ticks;
                lane.acceleration = if (lane.position.x > 0.0) == slow { -push() } else { push() };
            }
            // grBigBlue_801EEF00's cases 2 and 3 (grbigblue.c:4272-4315).
            Manoeuvre::Ahead | Manoeuvre::Behind => unreachable!("{NO_AHEAD_OR_BEHIND}"),
            Manoeuvre::Hold => {
                let target = lane.place;
                let ticks = speed_ticks(lane.velocity);
                let x = lane.position.x;
                lane.acceleration = if target > x {
                    if lane.velocity < 0.0 {
                        push()
                    } else {
                        // retail 0x801EF24C fmadds / 0x801EF258 fnmsubs: the
                        // stop's end if it braked now.
                        let brake = ticks * (0.5 * push());
                        let stop = fnmsubs(ticks, brake, fmadds(lane.velocity, ticks, x));
                        if stop < target {
                            push()
                        } else {
                            -push()
                        }
                    }
                } else if lane.velocity > 0.0 {
                    -push()
                } else {
                    let brake = ticks * (0.5 * -push());
                    let stop = fnmsubs(ticks, brake, fmadds(lane.velocity, ticks, x));
                    if stop < target {
                        push()
                    } else {
                        -push()
                    }
                };
            }
            Manoeuvre::ArriveRight | Manoeuvre::ArriveLeft => {
                lane.acceleration = 0.0;
                lane.alpha += FADE_STEP;
                if lane.alpha > 1.0 {
                    lane.alpha = 1.0;
                }
            }
            Manoeuvre::LeaveRight => lane.acceleration = push(),
            Manoeuvre::LeaveLeft => lane.acceleration = -push(),
            Manoeuvre::Fade => {
                lane.alpha -= FADE_STEP;
                if lane.alpha < 0.0 {
                    lane.alpha = 0.0;
                }
            }
        }
        true
    }

    /// grBigBlue_801E8B84 (0x801E8B84): the highest car (on the road)
    /// strictly inside the camera's height and the x window, or
    /// [`NO_FLOOR`].
    pub fn highest(&self, top: f32, bottom: f32, left: f32, right: f32) -> f32 {
        let mut result = NO_FLOOR;
        for lane in &self.lanes {
            if lane.manoeuvre == Manoeuvre::Off {
                continue;
            }
            let p = lane.position;
            if p.y < top && p.y > bottom && p.x < right && p.x > left && p.y > result {
                result = p.y;
            }
        }
        result
    }
}

/// 801EDF44: a leaving car past the edge fades if it is still visible.
fn leave(alpha: f32) -> Manoeuvre {
    if alpha != 0.0 {
        Manoeuvre::Fade
    } else {
        Manoeuvre::Off
    }
}

/// A formation place: `rank * (width * scale / (count))` plus half the
/// width back (801EC6C0 and 801ED694's fmadds with 0.5).
fn formation_place(parameters: &Parameters, scale: f32, rank: i32, count: i32) -> f32 {
    let width = parameters.formation_width as f32 * scale;
    let step = rank as f32 * (width / count as f32);
    fmadds(0.5, -(parameters.formation_width as f32 * scale), step)
}

/// 801ED694's bob amplitude for a moving car: the cruising amplitude
/// moved toward `extreme` with the speed (0x801EDAF0 fmsubs, 0x801EDB08
/// fmadds).
fn bob_amplitude(parameters: &Parameters, scale: f32, velocity: f32, extreme: f32) -> f32 {
    let top = parameters.car_top_speed * scale;
    let cruising = parameters.bob_cruising * scale;
    let slope = fmsubs(extreme, scale, cruising) / top;
    fmadds(parameters.bob_cruising, scale, velocity * slope)
}
