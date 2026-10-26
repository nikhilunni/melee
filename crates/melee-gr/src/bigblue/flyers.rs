//! The flyers over the road. The Falcon Flyer (map 0x23,
//! grBigBlue_801E8D64 / 801E93D8) carries the fighters at the start; when
//! its wait ends it calls the first small flyer and leaves to the right,
//! and once it is gone the three small flyers (map 0x20's bones 1, 6 and
//! 11, grBigBlue_801E6904 / 801E6C60) cross the stage in turn. They sink
//! and tilt under riders. After a few flyers the route platform (map 0x24,
//! grBigBlue_801E9F3C / 801EA05C) may cross, and after more the Falcon
//! Flyer comes back. The calls go through map 0x20's three words
//! (grBigBlue_801E8978 / 801E89DC).
use super::{
    add_rotation_z, add_translate_x, add_translate_y, along_slope, set_translate_y,
    slope_angle, to_int, zrandi, Body, Parameters, Scene, NO_FLOOR,
};
use crate::bigblue::cars::Cars;
use gekko_math::fma::{fmadds, fnmsubs};
use hsd_types::Vec3;

/// The small flyers.
pub const FLYERS: usize = 3;
/// Map 0x20's flyer bones (Ground_801C3FA4(gobj, 1 / 6 / 11)).
pub const FLYER_BONES: [usize; FLYERS] = [1, 6, 11];
/// grBb_803B8114: each flyer's half length (and lane speed) before the
/// ground's scale.
const FLYER_HALF: [f32; FLYERS] = [56.0, 40.0, 24.0];
/// The route platform's length and height before the scale (68, 52) and
/// the Falcon Flyer's half length and height (60, 140).
const ROUTE_LENGTH: f32 = 68.0;
const ROUTE_HEIGHT: f32 = 52.0;
const FALCON_HALF: f32 = 60.0;
const FALCON_HEIGHT: f32 = 140.0;
/// One degree, and its inverse (57.29578).
const DEGREE: f32 = 0.017_453_292;
const RADIAN: f32 = 57.295_78;
/// The floor search every flyer makes.
const SEARCH: f32 = 500.0;

/// One small flyer (`grBigBlue_GroundData`, 0x54 bytes from gp+E4).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flyer {
    /// +01: 0 parked, 1 waiting, 2 called, 3 flying.
    pub state: i8,
    /// +02: 1 to the right, -1 to the left.
    pub direction: i8,
    /// +04: the ticks to wait (also the sink's and rise's).
    pub wait: i32,
    /// +08: the height over the road.
    pub height: f32,
    /// +0C: the tilt toward the slope; +10 the height a rider found it at;
    /// +14 the height it heads for.
    pub tilt: f32,
    pub ridden_height: f32,
    pub target: f32,
    /// +1C: the side the riders weigh (1 right, -1 left); +20 the sway.
    pub sway_side: f32,
    pub sway: f32,
    /// +24 / +28: the riders' weight left and right of its middle.
    pub weight_left: f32,
    pub weight_right: f32,
    /// +2C / +30: the rider callbacks this tick and the last.
    pub riders: i32,
    pub previous_riders: i32,
    /// +34: 0 free, 1 sinking, 2 rising, 3 risen.
    pub phase: i32,
    /// +38: its position at the end of its last step.
    pub position: Vec3,
    /// +44: its speed (x) and rise (y).
    pub speed: f32,
    pub rise: f32,
    /// +50: the item it brought.
    pub item: Option<u32>,
}

impl Flyer {
    const PARKED: Self = Self {
        state: 0,
        direction: 0,
        wait: 0,
        height: 0.0,
        tilt: 0.0,
        ridden_height: 0.0,
        target: 0.0,
        sway_side: 0.0,
        sway: 0.0,
        weight_left: 0.0,
        weight_right: 0.0,
        riders: 0,
        previous_riders: 0,
        phase: 0,
        position: Vec3::ZERO,
        speed: 0.0,
        rise: 0.0,
        item: None,
    };
}

/// The Falcon Flyer's Ground (map 0x23, gp+C4..gp+D8).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Falcon {
    /// +C4: 0 away, 1 arriving, 2 waiting, 3 leaving.
    pub state: i8,
    /// +C8: its wait.
    pub wait: i32,
    /// +CC: the tilt it turns toward.
    pub tilt: f32,
    /// +D0: the height it heads for.
    pub target: f32,
    /// +D8: its speed.
    pub speed: f32,
}

/// The route platform's Ground (map 0x24, `grBigBlue_PlatformVars`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Route {
    /// +C4 (`x0`): 0 away, 1 called, 2 crossing; +C5 (`x1`) its side.
    pub state: i8,
    pub direction: i8,
    /// +C8 / +CC / +D0: the arrival wait and the two speed waits.
    pub wait: i32,
    pub speed_wait: i32,
    pub lift_wait: i32,
    /// +D4: its height over the road; +DC the height it heads for.
    pub height: f32,
    pub target: f32,
    /// +E4 / +E8: its horizontal and vertical speeds.
    pub speed: f32,
    pub lift: f32,
}

/// Map 0x20's Ground: the flyers and the calls between the objects.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flyers {
    /// gp+C4: the flyers launched since the Falcon Flyer last left.
    pub launched: u32,
    /// gp+C8 (`event_data[0]`): the small flyers may come.
    pub flyers_called: bool,
    /// gp+CC (`event_data[1]`): the Falcon Flyer is out (or called back).
    pub falcon_called: bool,
    /// gp+D0 (`event_data[2]`): the route platform is called.
    pub route_called: bool,
    pub flyers: [Flyer; FLYERS],
    pub falcon: Falcon,
    pub route: Route,
}

/// What a flyer's step reads of the cars (grBigBlue_801E8B84).
pub trait CarHeights {
    fn highest(&self, top: f32, bottom: f32, left: f32, right: f32) -> f32;
}
impl CarHeights for Cars {
    fn highest(&self, top: f32, bottom: f32, left: f32, right: f32) -> f32 {
        Cars::highest(self, top, bottom, left, right)
    }
}

/// The items a flyer may bring (grBigBlue_801E8A1C); with items off
/// nothing is drawn.
pub trait FlyerItems {
    /// `it_8026D324` over grBb_803B8120: whether any of its five kinds may
    /// spawn. Items on Big Blue are not witnessed.
    fn any_enabled(&self) -> bool;
}

impl Flyers {
    /// grBigBlue_801E6904 (map 0x20), grBigBlue_801E8D64 (map 0x23, the
    /// Falcon Flyer at `falcon_y` over the stage's middle) and
    /// grBigBlue_801E9F3C (map 0x24), in creation order. `scale_flyers`
    /// is map 0x20's bone scaling.
    pub fn new<S: Scene>(parameters: &Parameters, scene: &mut S) -> Self {
        let scale = scene.scale();
        for bone in 0..FLYERS {
            scene.set_scale_of(Body::Flyer(bone), scale);
        }
        Self {
            launched: 0,
            flyers_called: false,
            // grBigBlue_801E8D64.
            falcon_called: true,
            // grBigBlue_801E9F3C.
            route_called: false,
            flyers: [Flyer::PARKED; FLYERS],
            falcon: Falcon {
                state: 2,
                wait: to_int(parameters.falcon_wait),
                tilt: 0.0,
                target: 0.0,
                speed: 0.0,
            },
            route: Route {
                state: 0,
                direction: 0,
                wait: 0,
                speed_wait: 0,
                lift_wait: 0,
                height: 0.0,
                target: 0.0,
                speed: 0.0,
                lift: 0.0,
            },
        }
    }

    /// grBigBlue_801E8560: a rider on flyer `index` weighs on the side of
    /// its middle it stands, by its distance (beyond 2) times its weight
    /// over 1000 (`coll x50`, at most 1000).
    pub fn ridden<S: Scene>(&mut self, scene: &S, index: usize, at: Vec3, x50: i32) {
        let x50 = if x50 as f32 > 1000.0 { 1000 } else { x50 };
        let pos = scene.translation(Body::Flyer(index));
        let dx = pos.x - at.x;
        let dy = pos.y - at.y;
        // retail 0x801E8640: fmadds, then the inline sqrtf.
        let distance = gekko_math::msl::sqrtf(fmadds(dy, dy, dx * dx));
        if distance > 2.0 {
            let flyer = &mut self.flyers[index];
            let share = x50 as f32 / 1000.0;
            // retail 0x801E8728 / 0x801E8764: fmadds.
            if pos.x < at.x {
                flyer.weight_left = fmadds(distance, share, flyer.weight_left);
            } else {
                flyer.weight_right = fmadds(distance, share, flyer.weight_right);
            }
            flyer.riders += 1;
        }
    }

    /// grBigBlue_801E6C60 (0x801E6C60): the small flyers' step.
    pub fn step_flyers<S: Scene, C: CarHeights, I: FlyerItems>(
        &mut self,
        parameters: &Parameters,
        scene: &mut S,
        cars: &C,
        items: &I,
    ) {
        for index in 0..FLYERS {
            let body = Body::Flyer(index);
            match self.flyers[index].state {
                0 => {
                    if self.flyers_called {
                        let [low, range] = parameters.flyer_wait;
                        let flyer = &mut self.flyers[index];
                        flyer.wait = low + zrandi(scene.rng(), range);
                        flyer.position = scene.translation(body);
                        flyer.speed = 0.0;
                        flyer.rise = 0.0;
                        flyer.sway_side = 0.0;
                        flyer.sway = 0.0;
                        flyer.state = 1;
                    }
                }
                1 if !self.flyers_called => self.flyers[index].state = 0,
                1 | 2 => self.call_flyer(parameters, scene, cars, items, index),
                3 => self.fly(parameters, scene, cars, index),
                state => unimplemented!("grBigBlue_801E6C60: flyer state {state}"),
            }
        }
    }

    /// 801E6C60's cases 1 and 2: wait, then launch from a side of the
    /// road.
    fn call_flyer<S: Scene, C: CarHeights, I: FlyerItems>(
        &mut self,
        parameters: &Parameters,
        scene: &mut S,
        cars: &C,
        items: &I,
        index: usize,
    ) {
        let limits = scene.limits();
        if self.flyers[index].wait > 0 {
            let mut active = 0;
            for (other, flyer) in self.flyers.iter().enumerate() {
                if other == index {
                    continue;
                }
                if flyer.state == 3 {
                    if (flyer.direction == 1 && flyer.position.x < limits.camera_right)
                        || (flyer.direction == -1 && flyer.position.x > limits.camera_left)
                    {
                        active += 1;
                    }
                } else if flyer.state == 1 && flyer.wait == 0 {
                    active += 1;
                }
            }
            let flyer = &mut self.flyers[index];
            if active <= 1 {
                flyer.wait = 0;
            } else {
                flyer.wait -= 1;
            }
            return;
        }
        let scale = scene.scale();
        let edge = 10.0 + limits.blast_right;
        let mut pos = Vec3::new(edge, 0.0, 0.0);
        let mut other = Vec3::new(-(10.0 + limits.blast_right), 0.0, 0.0);
        let (right_y, _) = scene.floor(pos, SEARCH);
        let (left_y, _) = scene.floor(other, SEARCH);
        if right_y == NO_FLOOR && left_y == NO_FLOOR {
            return;
        }
        let [low, high] = parameters.flyer_height;
        let mut height = low;
        let range = gekko_math::msl::fabsf(high - low);
        height += zrandi(scene.rng(), to_int(range)) as f32;
        self.flyers[index].height = height;
        pos.y = right_y + height;
        other.y = left_y + height;
        let mut direction = side(scene, right_y, left_y);
        if self.falcon_called {
            direction = 1;
            pos.x = -(10.0 + limits.blast_right);
        } else if direction == 1 {
            pos = other;
        }
        let reach = 2.0 * (FLYER_HALF[0] * scale);
        let mut tries = 0;
        let mut blocked;
        loop {
            blocked = self.flyer_near(scene, Some(Body::Flyer(index)), pos, false, reach, 25.0);
            if !blocked {
                blocked = self.route_near(scene, pos, false, reach, 25.0);
            }
            if !blocked {
                // grBigBlue_801E8D04: under the highest car in camera.
                let bound =
                    cars.highest(limits.camera_top, limits.camera_bottom, limits.camera_left, limits.camera_right);
                if pos.y <= bound {
                    blocked = true;
                }
            }
            if blocked {
                tries += 1;
                pos.y += 25.0;
                if tries < 3 {
                    continue;
                }
            }
            break;
        }
        if blocked || pos.y == NO_FLOOR {
            pos.y = 10.0 + limits.camera_top;
        }
        let body = Body::Flyer(index);
        scene.set_translation(body, pos);
        let flyer = &mut self.flyers[index];
        flyer.target = pos.y;
        flyer.position = pos;
        flyer.direction = direction;
        flyer.speed = parameters.flyer_speed * direction as f32;
        flyer.rise = 0.0;
        flyer.sway_side = 0.0;
        flyer.sway = 0.0;
        flyer.phase = 0;
        flyer.riders = 0;
        flyer.state = 3;
        scene.set_hidden(body, false);
        flyer.item = None;
        if zrandi(scene.rng(), parameters.flyer_item_chance) == 0 && items.any_enabled() {
            unimplemented!("grBigBlue_801E8A1C (grbigblue.c:1320): a flyer's item");
        }
        self.launched += 1;
        let count = self.launched as i32;
        if !self.route_called {
            let [first, last] = parameters.route_count;
            if count >= first && count <= last && (count == last || scene.rng().randi(2) != 0) {
                self.route_called = true;
            }
        }
        if !self.falcon_called {
            let [first, last] = parameters.falcon_return;
            if count >= first
                && direction == 1
                && (count >= last || scene.rng().randi(2) != 0)
            {
                self.flyers_called = false;
                self.falcon_called = true;
                self.launched = 0;
            }
        }
    }

    /// 801E6C60's case 3: a flyer crosses, sinking and tilting under its
    /// riders, following the road's height, until past the far side.
    fn fly<S: Scene, C: CarHeights>(&mut self, parameters: &Parameters, scene: &mut S, cars: &C, index: usize) {
        let scale = scene.scale();
        let limits = scene.limits();
        let body = Body::Flyer(index);
        let mut pos = scene.translation(body);
        let (surface, normal) = scene.floor(pos, SEARCH);
        let slope = slope_angle(normal);
        let half = FLYER_HALF[index] * scale;
        let direction = self.flyers[index].direction;
        let ahead = along_slope(direction as f32 * half, slope, pos);
        let _behind = along_slope((-i32::from(direction)) as f32 * half, slope, pos);
        let flyer = self.flyers[index];
        if flyer.riders == 0 || flyer.phase == 3 {
            if flyer.previous_riders == 0 && flyer.riders != 0 {
                let flyer = &mut self.flyers[index];
                flyer.ridden_height = pos.y;
                flyer.wait = parameters.flyer_rise_wait;
                flyer.phase = 1;
            }
            self.flyers[index].tilt = slope;
            tilt_toward(scene, body, slope, parameters.flyer_tilt_follow);
        } else {
            if flyer.previous_riders == 0 {
                let flyer = &mut self.flyers[index];
                flyer.ridden_height = pos.y;
                flyer.wait = parameters.flyer_rise_wait;
                flyer.phase = 1;
            }
            let side: f32 = if flyer.weight_left < flyer.weight_right { 1.0 } else { -1.0 };
            {
                let flyer = &mut self.flyers[index];
                if flyer.sway_side != side {
                    flyer.wait = parameters.flyer_rise_wait;
                    flyer.phase = 1;
                }
                flyer.sway_side = side;
            }
            let flyer = self.flyers[index];
            if flyer.phase == 2 {
                pos.y += flyer.rise;
                self.flyers[index].tilt = slope;
                tilt_toward(scene, body, slope, parameters.flyer_tilt_follow);
                if pos.y >= flyer.ridden_height {
                    let flyer = &mut self.flyers[index];
                    flyer.rise = 0.0;
                    flyer.sway = 0.0;
                    flyer.sway_side = 0.0;
                    flyer.phase = 3;
                }
            } else if flyer.wait <= 0 {
                let flyer = &mut self.flyers[index];
                flyer.rise = parameters.flyer_rise;
                flyer.wait = parameters.flyer_rise_ticks;
                flyer.phase = 2;
            } else {
                let flyer = &mut self.flyers[index];
                if flyer.weight_left < flyer.weight_right {
                    flyer.sway = parameters.flyer_sway * (flyer.weight_right - flyer.weight_left);
                    if flyer.sway >= parameters.flyer_sway_limit {
                        flyer.sway = parameters.flyer_sway_limit;
                    }
                    pos.y -= parameters.flyer_sink;
                } else {
                    flyer.sway = -parameters.flyer_sway * (flyer.weight_left - flyer.weight_right);
                    if flyer.sway <= -parameters.flyer_sway_limit {
                        flyer.sway = -parameters.flyer_sway_limit;
                    }
                    pos.y += parameters.flyer_sink;
                }
                add_rotation_z(scene, body, DEGREE * flyer.sway);
                let degrees = RADIAN * scene.rotation_z(body);
                if pos.y < surface + parameters.flyer_height[0] {
                    if degrees > 30.0 {
                        scene.set_rotation_z(body, std::f32::consts::FRAC_PI_6);
                    }
                    if degrees < -30.0 {
                        scene.set_rotation_z(body, -std::f32::consts::FRAC_PI_6);
                    }
                }
                if degrees > 65.0 {
                    scene.set_rotation_z(body, 1.134_464);
                }
                if degrees < -65.0 {
                    scene.set_rotation_z(body, -1.134_464);
                }
                self.flyers[index].wait -= 1;
            }
        }
        let height = self.flyers[index].height;
        let target = self.follow_height(
            scene,
            cars,
            body,
            pos,
            ahead.y,
            height,
            half,
            [10.0 + half, 16.5],
            self.flyers[index].target,
        );
        self.flyers[index].target = target;
        pos.y += vertical_step(pos.y, target, parameters.flyer_climb, parameters.flyer_descent);
        let speed = self.flyers[index].speed;
        add_translate_x(scene, body, speed);
        set_translate_y(scene, body, pos.y);
        let flyer = &mut self.flyers[index];
        flyer.position = pos;
        flyer.previous_riders = flyer.riders;
        flyer.riders = 0;
        flyer.weight_right = 0.0;
        flyer.weight_left = 0.0;
        let x = scene.translation(body).x;
        let edge = (10.0 + limits.blast_right) - 50.0;
        if (flyer.speed < 0.0 && x < -edge) || (flyer.speed > 0.0 && x > edge) {
            scene.set_hidden(body, true);
            scene.set_rotation_z(body, 0.0);
            let flyer = &mut self.flyers[index];
            flyer.speed = 0.0;
            flyer.rise = 0.0;
            flyer.sway_side = 0.0;
            flyer.sway = 0.0;
            flyer.tilt = 0.0;
            flyer.phase = 0;
            flyer.riders = 0;
            let [low, range] = parameters.flyer_wait;
            flyer.wait = (low + zrandi(scene.rng(), range)) / 2;
            flyer.state = 1;
        }
    }

    /// The height a flyer heads for (801E6C60, 801E93D8 and 801EA05C's
    /// shared shape): over the highest car under it, else the road, but
    /// over a flyer or the route platform it would land on; `ahead_y` when
    /// there is no road.
    #[allow(clippy::too_many_arguments)]
    fn follow_height<S: Scene, C: CarHeights>(
        &self,
        scene: &mut S,
        cars: &C,
        body: Body,
        pos: Vec3,
        ahead_y: f32,
        height: f32,
        half: f32,
        probe: [f32; 2],
        current: f32,
    ) -> f32 {
        let limits = scene.limits();
        let left = pos.x - (20.0 + half);
        let right = pos.x + (20.0 + half);
        let cars_y = cars.highest(limits.camera_top, limits.camera_bottom, left, right);
        let (road_y, _) = scene.floor(pos, SEARCH);
        let (found, landing) = self.landing(scene, Some(body), pos, probe[0], probe[1]);
        if found == 0 || (found == 1 && pos.y < landing) {
            if cars_y <= road_y {
                if road_y == NO_FLOOR {
                    ahead_y
                } else {
                    road_y + height
                }
            } else {
                cars_y + height
            }
        } else if found == 1 {
            pos.y + (pos.y - landing)
        } else {
            current
        }
    }

    /// grBigBlue_801E93D8 (0x801E93D8): the Falcon Flyer's step.
    pub fn step_falcon<S: Scene, C: CarHeights>(&mut self, parameters: &Parameters, scene: &mut S, cars: &C) {
        let scale = scene.scale();
        let limits = scene.limits();
        let body = Body::Falcon;
        let mut pos = scene.translation(body);
        let (_, normal) = scene.floor(pos, SEARCH);
        let slope = slope_angle(normal);
        let ahead = along_slope(50.0, slope, pos);
        let _behind = along_slope(-50.0, slope, pos);
        match self.falcon.state {
            0 => {
                if self.falcon_called {
                    let out = self.flyers.iter().filter(|f| f.state != 0).count();
                    if out <= 1 {
                        pos = Vec3::new(limits.blast_left - 50.0, 0.0, 0.0);
                        let (floor, _) = scene.floor(pos, SEARCH);
                        if floor != NO_FLOOR {
                            pos.y = floor + parameters.falcon_height;
                            let height = FALCON_HEIGHT * scale;
                            let reach = 2.0 * (FALCON_HALF * scale);
                            let mut blocked = self.flyer_near(scene, None, pos, true, reach, height);
                            if !blocked {
                                blocked = self.route_near(scene, pos, true, reach, 25.0);
                            }
                            if !blocked {
                                let bound = cars.highest(
                                    limits.camera_top,
                                    limits.camera_bottom,
                                    limits.camera_left,
                                    limits.camera_right,
                                );
                                if pos.y <= bound {
                                    blocked = true;
                                }
                            }
                            if blocked {
                                pos.y = 30.0 + limits.camera_top;
                            }
                            scene.set_translation(body, pos);
                            self.falcon.target = pos.y;
                            self.falcon.speed = parameters.falcon_speed;
                            scene.set_hidden(body, false);
                            self.falcon.state = 1;
                        }
                    }
                }
                return;
            }
            1 => {
                if pos.x > 0.0 {
                    self.falcon.speed = 0.0;
                    self.falcon.wait = to_int(parameters.falcon_wait);
                    self.falcon.state = 2;
                } else {
                    let height = FALCON_HEIGHT * scale;
                    // retail 0x801E980C / 0x801E9844: fmadds.
                    let reach = fmadds(FALCON_HALF, scale, 30.0);
                    let blocked = self.flyer_near(scene, None, pos, true, reach, height) || {
                        let reach = fmadds(FALCON_HALF, scale, 30.0);
                        let height = FALCON_HEIGHT * scale;
                        self.route_near(scene, pos, true, reach, height)
                    };
                    self.falcon.speed = if blocked { 0.0 } else { parameters.falcon_speed };
                }
            }
            2 => {
                let wait = self.falcon.wait;
                if wait <= 0 {
                    self.falcon.speed = parameters.falcon_speed;
                    self.falcon.state = 3;
                    if let Some(flyer) = self.flyers.iter_mut().find(|f| f.state == 0) {
                        flyer.state = 2;
                        flyer.wait = 0;
                    }
                } else {
                    self.falcon.wait = wait - 1;
                }
            }
            3 => {
                if pos.x > 50.0 + limits.blast_right {
                    scene.set_hidden(body, true);
                    self.falcon.speed = 0.0;
                    self.falcon_called = false;
                    self.flyers_called = true;
                    self.falcon.tilt = 0.0;
                    scene.set_rotation_z(body, 0.0);
                    self.falcon.state = 0;
                }
            }
            state => unimplemented!("grBigBlue_801E93D8: Falcon Flyer state {state}"),
        }
        self.falcon.tilt = slope;
        tilt_toward(scene, body, slope, parameters.falcon_tilt_follow);
        let bound = cars.highest(limits.camera_top, limits.camera_bottom, limits.camera_left, limits.camera_right);
        let (floor, _) = scene.floor(pos, SEARCH);
        self.falcon.target = if bound <= floor {
            if floor == NO_FLOOR {
                ahead.y
            } else {
                floor + parameters.falcon_height
            }
        } else {
            bound + parameters.falcon_height
        };
        pos.y += vertical_step(pos.y, self.falcon.target, parameters.falcon_climb, parameters.falcon_descent);
        add_translate_x(scene, body, self.falcon.speed);
        set_translate_y(scene, body, pos.y);
    }

    /// grBigBlue_801EA05C (0x801EA05C): the route platform's step.
    pub fn step_route<S: Scene, C: CarHeights>(&mut self, parameters: &Parameters, scene: &mut S, cars: &C) {
        let scale = scene.scale();
        let limits = scene.limits();
        let body = Body::Route;
        let mut pos = scene.translation(body);
        let (_, normal) = scene.floor(pos, SEARCH);
        let slope = slope_angle(normal);
        let half = (ROUTE_LENGTH * scale) * 0.5;
        let ahead = along_slope(half, slope, pos);
        let _behind = along_slope(-((ROUTE_LENGTH * scale) * 0.5), slope, pos);
        match self.route.state {
            0 => {
                if self.route_called {
                    self.route.wait = 0;
                    self.route.lift_wait = 0;
                    self.route.speed_wait = 0;
                    self.route.state = 1;
                }
            }
            1 => {
                let wait = self.route.wait;
                if wait > 0 {
                    self.route.wait = wait - 1;
                } else {
                    let mut pos = Vec3::new(limits.blast_right, 0.0, 0.0);
                    let mut other = Vec3::new(-limits.blast_right, 0.0, 0.0);
                    let (right_y, _) = scene.floor(pos, SEARCH);
                    let (left_y, _) = scene.floor(other, SEARCH);
                    let [low, high] = parameters.route_height;
                    self.route.height = low;
                    let range = gekko_math::msl::fabsf(high - low);
                    self.route.height += zrandi(scene.rng(), to_int(range)) as f32;
                    pos.y = right_y + self.route.height;
                    other.y = left_y + self.route.height;
                    if right_y != NO_FLOOR || left_y != NO_FLOOR {
                        self.route.direction = side(scene, right_y, left_y);
                        if self.route.direction == 1 {
                            pos = other;
                        }
                        let height = ROUTE_HEIGHT * scale;
                        let reach = 2.0 * (ROUTE_LENGTH * scale);
                        let mut blocked = self.flyer_near(scene, Some(body), pos, false, reach, height);
                        if !blocked {
                            let bound = cars.highest(
                                limits.camera_top,
                                limits.camera_bottom,
                                limits.camera_left,
                                limits.camera_right,
                            );
                            if pos.y <= bound {
                                blocked = true;
                            }
                        }
                        if !blocked {
                            scene.set_translation(body, pos);
                            self.route.target = pos.y;
                            self.route.lift = 0.0;
                            self.route.speed = 0.0;
                            scene.set_hidden(body, false);
                            self.route.state = 2;
                        }
                    }
                }
            }
            2 => {
                // retail 0x801EA54C / 0x801EA578: fmadds.
                let reach = fmadds(ROUTE_LENGTH * scale, 0.5, 20.0);
                let left = pos.x - reach;
                let right = pos.x + fmadds(ROUTE_LENGTH * scale, 0.5, 20.0);
                let bound = cars.highest(limits.camera_top, limits.camera_bottom, left, right);
                let (surface, _) = scene.floor(pos, SEARCH);
                // retail 0x801EA5C8 / 0x801EA5EC: fmadds.
                let probe_y = fmadds(ROUTE_HEIGHT * scale, 0.5, 4.0);
                let probe_x = fmadds(ROUTE_LENGTH * scale, 0.5, 10.0);
                let (found, landing) = self.landing(scene, Some(body), pos, probe_x, probe_y);
                if found == 0 || (found == 1 && pos.y < landing) {
                    self.route.target = if bound <= surface {
                        if surface == NO_FLOOR {
                            ahead.y
                        } else {
                            surface + self.route.height
                        }
                    } else {
                        bound + self.route.height
                    };
                } else if found == 1 {
                    self.route.target = pos.y + (pos.y - landing);
                }
                pos.y += vertical_step(pos.y, self.route.target, parameters.route_climb, parameters.route_descent);
                set_translate_y(scene, body, pos.y);
                let [low, high] = parameters.route_wait;
                if self.route.speed_wait <= 0 {
                    let range = (high - low).abs();
                    self.route.speed_wait = zrandi(scene.rng(), range) + low;
                    let speed = route_speed(scene, parameters.route_speed_x);
                    self.route.speed = speed * self.route.direction as f32;
                } else {
                    self.route.speed_wait -= 1;
                }
                if self.route.lift_wait <= 0 {
                    let range = (high - low).abs();
                    self.route.lift_wait = zrandi(scene.rng(), range) + low;
                    let mut lift = route_speed(scene, parameters.route_speed_y);
                    if scene.rng().randi(2) != 0 {
                        lift *= -1.0;
                    }
                    self.route.lift = lift;
                } else {
                    self.route.lift_wait -= 1;
                }
                add_translate_x(scene, body, self.route.speed);
                add_translate_y(scene, body, self.route.lift);
                let x = scene.translation(body).x;
                if (self.route.direction == -1 && x < -(limits.blast_right - 50.0))
                    || (self.route.direction == 1 && x > limits.blast_right - 50.0)
                {
                    scene.set_hidden(body, true);
                    self.route.lift = 0.0;
                    self.route.speed = 0.0;
                    self.route_called = false;
                    self.route.state = 0;
                }
            }
            state => unimplemented!("grBigBlue_801EA05C: route platform state {state}"),
        }
    }

    /// grBigBlue_801E8794 (0x801E8794): a flying flyer (not `exclude`)
    /// within `reach` across and `height` up or down of `pos`, or (with
    /// `leftward`) any flying to the left.
    fn flyer_near<S: Scene>(
        &self,
        scene: &S,
        exclude: Option<Body>,
        pos: Vec3,
        leftward: bool,
        reach: f32,
        height: f32,
    ) -> bool {
        for (index, flyer) in self.flyers.iter().enumerate() {
            if exclude == Some(Body::Flyer(index)) || flyer.state != 3 {
                continue;
            }
            let at = scene.translation(Body::Flyer(index));
            let dx = at.x - pos.x;
            let dx = if dx < 0.0 { -(at.x - pos.x) } else { dx };
            if dx < reach {
                let dy = at.y - pos.y;
                let dy = if dy < 0.0 { -(at.y - pos.y) } else { dy };
                if dy < height {
                    return true;
                }
            }
            if leftward && flyer.direction == -1 {
                return true;
            }
        }
        false
    }

    /// grBigBlue_801EAB50 (0x801EAB50): the crossing route platform within
    /// `reach` and `height` of `pos`, or (with `leftward`) crossing left.
    fn route_near<S: Scene>(&self, scene: &S, pos: Vec3, leftward: bool, reach: f32, height: f32) -> bool {
        let mut result = false;
        if self.route.state == 2 {
            let at = scene.translation(Body::Route);
            let dx = at.x - pos.x;
            let dx = if dx < 0.0 { -(at.x - pos.x) } else { dx };
            if dx < reach {
                let dy = at.y - pos.y;
                let dy = if dy < 0.0 { -(at.y - pos.y) } else { dy };
                if dy < height {
                    result = true;
                }
            }
            if leftward && self.route.direction == -1 {
                result = true;
            }
        }
        result
    }

    /// grBigBlue_801EACE8 (0x801EACE8): the flyers and the route platform
    /// overlapping `point`'s window (`half_x` across, `half_y` up and
    /// down): 1 with the height of the last one in the window closer than
    /// any before (retail keeps the height, not the distance, as the best),
    /// 2 with the highest one below, else 0.
    fn landing<S: Scene>(
        &self,
        scene: &S,
        exclude: Option<Body>,
        point: Vec3,
        half_x: f32,
        half_y: f32,
    ) -> (i32, f32) {
        let scale = scene.scale();
        let left_bound = point.x - half_x;
        let right_bound = point.x + half_x;
        let top_bound = point.y + half_y;
        let bottom_bound = point.y - half_y;
        let mut in_range = f32::MAX;
        let mut below = NO_FLOOR;
        let mut consider = |left: f32, right: f32, y: f32| {
            if (right > left_bound && right < right_bound) || (left < right_bound && left > left_bound) {
                if y > bottom_bound && y < top_bound {
                    let d = point.y - y;
                    let d = if d < 0.0 { -d } else { d };
                    if d < in_range {
                        in_range = y;
                    }
                } else if point.y > y && y > below {
                    below = y;
                }
            }
        };
        for (index, flyer) in self.flyers.iter().enumerate() {
            let body = Body::Flyer(index);
            if exclude == Some(body) || flyer.state != 3 {
                continue;
            }
            let at = scene.translation(body);
            let half = FLYER_HALF[index] * scale;
            consider(at.x - half, at.x + half, at.y);
        }
        if exclude != Some(Body::Route) && self.route.state == 2 {
            let at = scene.translation(Body::Route);
            // retail 0x801EAF0C fnmsubs / 0x801EAF24 fmadds.
            let left = fnmsubs(ROUTE_LENGTH * scale, 0.5, at.x);
            let right = fmadds(ROUTE_LENGTH * scale, 0.5, at.x);
            consider(left, right, at.y);
        }
        if in_range != f32::MAX {
            (1, in_range)
        } else if below != NO_FLOOR {
            (2, below)
        } else {
            (0, 0.0)
        }
    }
}

/// The side a flyer enters from: the side without a road, a coin toss when
/// both sides are within 80 of each other, else the lower side.
fn side<S: Scene>(scene: &mut S, right_y: f32, left_y: f32) -> i8 {
    if left_y == NO_FLOOR {
        -1
    } else if right_y == NO_FLOOR {
        1
    } else {
        let d = right_y - left_y;
        let d = if d < 0.0 { -d } else { d };
        if d < 80.0 {
            if scene.rng().randi(2) != 0 {
                1
            } else {
                -1
            }
        } else if right_y < left_y {
            -1
        } else {
            1
        }
    }
}

/// A flyer's tilt toward `target` by `follow` degrees per radian of
/// difference, stopping at it (801E6C60 / 801E93D8: fmuls, then
/// HSD_JObjAddRotationZ's fadds).
fn tilt_toward<S: Scene>(scene: &mut S, body: Body, target: f32, follow: f32) {
    let rotation = scene.rotation_z(body);
    let below = rotation < target;
    add_rotation_z(scene, body, DEGREE * (follow * (target - rotation)));
    let rotation = scene.rotation_z(body);
    if (below && rotation >= target) || (!below && rotation <= target) {
        scene.set_rotation_z(body, target);
    }
}

/// The vertical step toward a target height: none within 0.5, else the
/// difference over the climb (or descent) divisor, at most its limit.
fn vertical_step(y: f32, target: f32, climb: [f32; 2], descent: [f32; 2]) -> f32 {
    let d = y - target;
    let d = if d < 0.0 { -d } else { d };
    if d < 0.5 {
        0.0
    } else if y < target {
        let v = (target - y) / climb[0];
        if v > climb[1] {
            climb[1]
        } else {
            v
        }
    } else {
        let v = (target - y) / descent[0];
        if v < -descent[1] {
            -descent[1]
        } else {
            v
        }
    }
}

/// 801EA05C's speed draw: tenths between the two bounds (zero draws the
/// lower bound).
fn route_speed<S: Scene>(scene: &mut S, bounds: [f32; 2]) -> f32 {
    let range = (bounds[1] - bounds[0]) / 0.1;
    let mut speed = zrandi(scene.rng(), to_int(range)) as f32;
    speed *= 0.1;
    if speed == 0.0 {
        speed = bounds[0];
    }
    speed
}
