//! Big Blue (`gr/grbigblue.c`, GrBb.dat, StKind 0x18). The stage is a race
//! seen from above the track: map 0x22 is the road, twelve segments
//! (grBb_TrackEntries) that scroll left under the stage and climb, dip and
//! loop as the next one is drawn at random; map 0x21 carries four racing
//! cars (thirty collision bones, one per car model) that ride the road;
//! map 0x23 is the Falcon Flyer the fighters start on, which waits, leaves
//! and lets three small flyers (map 0x20) and the route platform (map
//! 0x24) cross the stage until it comes back. Maps 1 and 2 (the scenery)
//! follow the road. Map 0x1F holds the revival markers, map 0 the spawns.
pub mod cars;
pub mod flyers;
pub mod procs;
pub mod road;

use gekko_math::HsdRng;
use hsd_types::Vec3;

/// `-F32_MAX`: grBigBlue_801EC58C's "no floor" and the empty searches.
pub const NO_FLOOR: f32 = -f32::MAX;

/// `grBb_YakumonoParam` (grbigblue.static.h, GrBb.dat `yakumono_param`).
/// Offsets are the struct's; GrBb.dat's values in the comments.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parameters {
    /// +0x00 (-60): the road's resting height, and the cars' base.
    pub road_height: f32,
    /// +0x04 (2500) / +0x08 (0): the road's depths below and above which
    /// the next segment must climb or dip (grBigBlue_801EB4AC).
    pub road_low: f32,
    pub road_high: f32,
    /// +0x0C (2): a car bone's scale over the ground's.
    pub car_scale: f32,
    /// +0x10 (400) / +0x14 (1200): the wait between car arrivals.
    pub arrival_wait: [i32; 2],
    /// +0x18 (4): the cars on the road at the start (at most four).
    pub starting_cars: i32,
    /// +0x1C (440): the width of the cars' formation.
    pub formation_width: i32,
    /// +0x20 (60): the closest two cars come (grBigBlue_801EF424).
    pub car_gap: i32,
    /// +0x24 (0.9): a car's top speed; +0x28 (0.04) its acceleration.
    pub car_top_speed: f32,
    pub car_acceleration: f32,
    /// +0x2C (10): a car's height over the road.
    pub car_ride_height: f32,
    /// +0x30 (0) / +0x34 (1) / +0x38 (2): the bob's amplitude backing,
    /// cruising and accelerating; +0x3C (0.1) how fast it follows.
    pub bob_backing: f32,
    pub bob_cruising: f32,
    pub bob_accelerating: f32,
    pub bob_follow: f32,
    /// +0x40 (5): the bob's phase step, in degrees.
    pub bob_rate: f32,
    /// +0x44 (0.007): a fighter landing on a car pushes it down by this
    /// times its `coll x50` (fn_801EF60C).
    pub landing_push: f32,
    /// +0x48 (0.074) / +0x4C (7): the push's spring and its deepest sink.
    pub sink_spring: f32,
    pub sink_depth: f32,
    /// +0x50 (0.09): a riding car's tilt toward the road's slope;
    /// +0x54 (0.03) / +0x58 (-10): an airborne car's toward a pitch, in
    /// degrees.
    pub tilt_follow: f32,
    pub level_follow: f32,
    pub level_angle: f32,
    /// +0x5C (480) / +0x60 (960): a car's wait between manoeuvres
    /// (floats, converted with fctiwz).
    pub manoeuvre_wait: [f32; 2],
    /// +0x68 (50): how far past the blast zone a car arrives and leaves.
    pub car_margin: f32,
    /// +0x6C (10): the road's scroll per tick.
    pub road_scroll: f32,
    /// +0x70 (99999): the highest the road follows its floor.
    pub road_ceiling: f32,
    /// +0x74 (0.03125): the road's turn toward level.
    pub road_turn: f32,
    /// +0x78 (0.02) / +0x7C (6): the road's fall over a gap.
    pub road_fall: f32,
    pub road_drop: f32,
    /// +0x80 (0.03): an airborne car's gravity; +0x84 (2) its launch.
    pub car_gravity: f32,
    pub car_launch: f32,
    /// +0x88 (100) / +0x8C (500): a flyer's wait before it comes.
    pub flyer_wait: [i32; 2],
    /// +0x90 (40) / +0x94 (50): a flyer's height over the road.
    pub flyer_height: [f32; 2],
    /// +0x98 (0.14): a flyer's speed.
    pub flyer_speed: f32,
    /// +0x9C (2): a flyer's tilt toward the road's slope, in degrees per
    /// unit.
    pub flyer_tilt_follow: f32,
    /// +0xA0 (0.7) / +0xA4 (10) / +0xA8 (0.6): a flyer's sway under a
    /// fighter's weight, its limit, and the sink it brings.
    pub flyer_sway: f32,
    pub flyer_sway_limit: f32,
    pub flyer_sink: f32,
    /// +0xAC (0.6): a flyer's rise once nobody rides it.
    pub flyer_rise: f32,
    /// +0xB0 (5) / +0xB4 (15): the ticks before a flyer rises, and of its
    /// rise.
    pub flyer_rise_wait: i32,
    pub flyer_rise_ticks: i32,
    /// +0xB8 (6): one flyer in this many brings an item.
    pub flyer_item_chance: i32,
    /// +0xBC (8) / +0xC0 (2) / +0xC4 (30) / +0xC8 (1): a flyer's climb
    /// (divisor, limit) and descent toward its height.
    pub flyer_climb: [f32; 2],
    pub flyer_descent: [f32; 2],
    /// +0xCC (50): the Falcon Flyer's height over the road; +0xD0 (0.2)
    /// its speed; +0xD4 (1.5) its tilt toward the slope.
    pub falcon_height: f32,
    pub falcon_speed: f32,
    pub falcon_tilt_follow: f32,
    /// +0xD8 (900): the Falcon Flyer's wait (float, fctiwz).
    pub falcon_wait: f32,
    /// +0xDC (7) / +0xE0 (10): the flyers before it may return, and must.
    pub falcon_return: [i32; 2],
    /// +0xE4..+0xF0 (8, 2, 30, 1): its climb and descent.
    pub falcon_climb: [f32; 2],
    pub falcon_descent: [f32; 2],
    /// +0xF4 (45) / +0xF8 (50): the route platform's height over the road.
    pub route_height: [f32; 2],
    /// +0xFC (0.1) / +0x100 (0.5) and +0x104 (0.1) / +0x108 (1): its
    /// horizontal and vertical speeds.
    pub route_speed_x: [f32; 2],
    pub route_speed_y: [f32; 2],
    /// +0x10C (120) / +0x110 (180): its wait between speed changes.
    pub route_wait: [i32; 2],
    /// +0x11C (3) / +0x120 (6): the flyers before the route platform may
    /// come, and must.
    pub route_count: [i32; 2],
    /// +0x124..+0x130 (8, 1, 30, 1): its climb and descent.
    pub route_climb: [f32; 2],
    pub route_descent: [f32; 2],
    /// +0x134 (0, 60, 60) / +0x140 (0.8): map 0x1F's translation and scale.
    pub markers_translate: Vec3,
    pub markers_scale: f32,
}

/// Stage_GetCamBounds*Offset and Stage_GetBlastZone*Offset as the stage
/// reads them this tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Limits {
    pub camera_top: f32,
    pub camera_bottom: f32,
    pub camera_left: f32,
    pub camera_right: f32,
    pub blast_left: f32,
    pub blast_right: f32,
}

/// A JObj the stage moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Body {
    /// Map 0x20's bones 1, 6 and 11 (`xD4[i]`): the three flyers.
    Flyer(usize),
    /// Map 0x23's GObj JObj: the Falcon Flyer.
    Falcon,
    /// Map 0x24's GObj JObj: the route platform.
    Route,
    /// Map 0x21's bone `slot + 1` (`collision_jobjs[slot]`): a car.
    Car(usize),
    /// Map 0x22's GObj JObj: the road.
    Road,
    /// A bone of map 0x22: a road segment and what it carries.
    RoadBone(usize),
    /// Maps 1 and 2's GObj JObjs: the scenery the road carries.
    Scenery(u8),
}

/// What Big Blue's procs read and move outside their own state.
pub trait Scene {
    /// Ground_801C0498: the ground's scale (GrBb.dat: 0.5).
    fn scale(&self) -> f32;
    fn limits(&self) -> Limits;
    fn rng(&mut self) -> &mut HsdRng;
    /// grBigBlue_801EC58C (0x801EC58C): the highest floor of the road's
    /// joints (grBb_803B8134) under a vertical segment `half_height` above
    /// and below `at`, and that floor's normal; [`NO_FLOOR`] if none.
    fn floor(&mut self, at: Vec3, half_height: f32) -> (f32, Option<Vec3>);
    /// HSD_JObjGetTranslation.
    fn translation(&self, body: Body) -> Vec3;
    /// HSD_JObjSetTranslate.
    fn set_translation(&mut self, body: Body, translation: Vec3);
    /// HSD_JObjGetRotationZ / HSD_JObjSetRotationZ.
    fn rotation_z(&self, body: Body) -> f32;
    fn set_rotation_z(&mut self, body: Body, z: f32);
    /// HSD_JObjSetScale with the same scale on every axis.
    fn set_scale_of(&mut self, body: Body, scale: f32);
    /// HSD_JObjSetFlagsAll / HSD_JObjClearFlagsAll(JOBJ_HIDDEN).
    fn set_hidden(&mut self, body: Body, hidden: bool);
    /// lb_8000B1CC(bone, NULL, &out): a road bone's world position.
    fn world_position(&mut self, body: Body) -> Vec3;
    /// mpJointListAdd / mpLib_80057BC0 / mpLib_80057424 / mpLib_80058560.
    fn enable_joint(&mut self, joint: i32);
    fn disable_joint(&mut self, joint: i32);
    fn snapshot_joint(&mut self, joint: i32);
    fn stitch_joints(&mut self);
}

/// `HSD_JObjSetTranslateX` and its siblings, through [`Scene`].
pub(crate) fn set_translate_x<S: Scene>(scene: &mut S, body: Body, x: f32) {
    let mut t = scene.translation(body);
    t.x = x;
    scene.set_translation(body, t);
}
pub(crate) fn set_translate_y<S: Scene>(scene: &mut S, body: Body, y: f32) {
    let mut t = scene.translation(body);
    t.y = y;
    scene.set_translation(body, t);
}
pub(crate) fn set_translate_z<S: Scene>(scene: &mut S, body: Body, z: f32) {
    let mut t = scene.translation(body);
    t.z = z;
    scene.set_translation(body, t);
}
/// `HSD_JObjAddTranslationX` (`translate.x += x`, fadds).
pub(crate) fn add_translate_x<S: Scene>(scene: &mut S, body: Body, dx: f32) {
    let mut t = scene.translation(body);
    t.x += dx;
    scene.set_translation(body, t);
}
pub(crate) fn add_translate_y<S: Scene>(scene: &mut S, body: Body, dy: f32) {
    let mut t = scene.translation(body);
    t.y += dy;
    scene.set_translation(body, t);
}
/// `HSD_JObjAddRotationZ` (`rotate.z += z`, fadds).
pub(crate) fn add_rotation_z<S: Scene>(scene: &mut S, body: Body, dz: f32) {
    let z = scene.rotation_z(body);
    scene.set_rotation_z(body, z + dz);
}

/// `ZRANDI(n)` (gr/inlines.h): `HSD_Randi(n)`, or zero without a draw.
pub(crate) fn zrandi(rng: &mut HsdRng, n: i32) -> i32 {
    if n != 0 {
        rng.randi(n)
    } else {
        0
    }
}

/// `rand_range(a, b)` (gr/inlines.h): uniform between the two.
pub(crate) fn rand_range(rng: &mut HsdRng, a: i32, b: i32) -> i32 {
    if a > b {
        b + zrandi(rng, a - b)
    } else if a < b {
        a + zrandi(rng, b - a)
    } else {
        a
    }
}

/// `fctiwz`: a float parameter read as an int.
pub(crate) fn to_int(value: f32) -> i32 {
    value as i32
}

/// The quintic `lbVector_ApplyEulerRotation(&v, &(0, 0, z))` then
/// `lbVector_Add(&v, origin)`: `x` along a slope of angle `z` from
/// `origin`. The zero x and y turns still scale by the polynomial's cosine.
pub(crate) fn along_slope(x: f32, z: f32, origin: Vec3) -> Vec3 {
    use melee_lb::vector::{rotate_about, Axis};
    let mut v = Vec3::new(x, 0.0, 0.0);
    v = rotate_about(v, Axis::X, 0.0);
    v = rotate_about(v, Axis::Y, 0.0);
    v = rotate_about(v, Axis::Z, z);
    Vec3::new(v.x + origin.x, v.y + origin.y, v.z + origin.z)
}

/// `atan2f(-normal.x, normal.y)` of a floor normal, or of straight up when
/// there is no floor: the slope a body tilts toward.
pub(crate) fn slope_angle(normal: Option<Vec3>) -> f32 {
    let n = normal.unwrap_or(Vec3::new(0.0, 1.0, 0.0));
    melee_lb::trigf::atan2f(-n.x, n.y)
}

/// GrBb.dat's yakumono_param (tests/real_bigblue.rs reads it back), for
/// the creation's draw count where the archive is not at hand.
pub const RETAIL_PARAMETERS: Parameters = Parameters {
    road_height: -60.0,
    road_low: 2500.0,
    road_high: 0.0,
    car_scale: 2.0,
    arrival_wait: [400, 1200],
    starting_cars: 4,
    formation_width: 440,
    car_gap: 60,
    car_top_speed: 0.9,
    car_acceleration: 0.04,
    car_ride_height: 10.0,
    bob_backing: 0.0,
    bob_cruising: 1.0,
    bob_accelerating: 2.0,
    bob_follow: 0.1,
    bob_rate: 5.0,
    landing_push: 0.007,
    sink_spring: 0.074,
    sink_depth: 7.0,
    tilt_follow: 0.09,
    level_follow: 0.03,
    level_angle: -10.0,
    manoeuvre_wait: [480.0, 960.0],
    car_margin: 50.0,
    road_scroll: 10.0,
    road_ceiling: 99999.0,
    road_turn: 0.03125,
    road_fall: 0.02,
    road_drop: 6.0,
    car_gravity: 0.03,
    car_launch: 2.0,
    flyer_wait: [100, 500],
    flyer_height: [40.0, 50.0],
    flyer_speed: 0.14,
    flyer_tilt_follow: 2.0,
    flyer_sway: 0.7,
    flyer_sway_limit: 10.0,
    flyer_sink: 0.6,
    flyer_rise: 0.6,
    flyer_rise_wait: 5,
    flyer_rise_ticks: 15,
    flyer_item_chance: 6,
    flyer_climb: [8.0, 2.0],
    flyer_descent: [30.0, 1.0],
    falcon_height: 50.0,
    falcon_speed: 0.2,
    falcon_tilt_follow: 1.5,
    falcon_wait: 900.0,
    falcon_return: [7, 10],
    falcon_climb: [8.0, 2.0],
    falcon_descent: [30.0, 1.0],
    route_height: [45.0, 50.0],
    route_speed_x: [0.1, 0.5],
    route_speed_y: [0.1, 1.0],
    route_wait: [120, 180],
    route_count: [3, 6],
    route_climb: [8.0, 1.0],
    route_descent: [30.0, 1.0],
    markers_translate: Vec3::new(0.0, 60.0, 60.0),
    markers_scale: 0.8,
};

/// A scene that only draws: the creation's random draws without its
/// models.
struct DrawProbe<'a> {
    rng: &'a mut HsdRng,
}
impl Scene for DrawProbe<'_> {
    fn scale(&self) -> f32 {
        0.5
    }
    fn limits(&self) -> Limits {
        unreachable!("the creation reads no limits")
    }
    fn rng(&mut self) -> &mut HsdRng {
        self.rng
    }
    fn floor(&mut self, _: Vec3, _: f32) -> (f32, Option<Vec3>) {
        unreachable!("the cars' creation searches no floor")
    }
    fn translation(&self, _: Body) -> Vec3 {
        Vec3::ZERO
    }
    fn set_translation(&mut self, _: Body, _: Vec3) {}
    fn rotation_z(&self, _: Body) -> f32 {
        0.0
    }
    fn set_rotation_z(&mut self, _: Body, _: f32) {}
    fn set_scale_of(&mut self, _: Body, _: f32) {}
    fn set_hidden(&mut self, _: Body, _: bool) {}
    fn world_position(&mut self, _: Body) -> Vec3 {
        Vec3::ZERO
    }
    fn enable_joint(&mut self, _: i32) {}
    fn disable_joint(&mut self, _: i32) {}
    fn snapshot_joint(&mut self, _: i32) {}
    fn stitch_joints(&mut self) {}
}

/// grBigBlue_801E57C0's draws, all in the cars' creation (801EC6C0 and the
/// arrival timer): `rng` after them. Their count depends on the draws (a
/// car bone already held is drawn again).
pub fn after_creation_draws(mut rng: HsdRng) -> HsdRng {
    cars::Cars::new(&RETAIL_PARAMETERS, &mut DrawProbe { rng: &mut rng });
    rng
}
