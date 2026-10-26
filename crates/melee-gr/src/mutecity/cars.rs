//! Mute City's thirty cars: their places on the track (grMc_8049F4B8),
//! the pack order (grMc_8049F440), each tick's driving (grMuteCity_801F1328
//! and 801F106C) and their placement along the splines with what follows
//! from it (grMuteCity_801F1A34): sparks between close cars, the quake and
//! sounds as they pass the stage, their stage material items, smoke and
//! explosions from the damage fighters deal them.
use super::Parameters;
use crate::fzero::{orientation, CarJoints, CARS};
use gekko_math::{
    fma::{fmadd, fmadds},
    HsdRng,
};
use hsd_archive::desc::spline::Spline;
use hsd_types::{Mtx, Vec3};
use melee_lb::vector::normalize;

/// PSVECCrossProduct (paired single, unnormalized).
fn cross(a: Vec3, b: Vec3) -> Vec3 {
    let mut out = Vec3::ZERO;
    hsd_anim::mtx::vec_cross_product(&a, &b, &mut out);
    out
}

/// `x20` bits grMuteCity_801F1328 sets each tick.
pub mod flag {
    /// Some car is within 0.002 of the track behind this one.
    pub const CLOSE_BEHIND: u16 = 0x1;
    /// The faster or slower of a close pair.
    pub const SLOWER: u16 = 0x2;
    pub const FASTER: u16 = 0x4;
    /// The pair is within 30 of each other.
    pub const TOUCHING: u16 = 0x8;
    /// Among the five leaders / the five trailers.
    pub const LEADER: u16 = 0x10;
    pub const TRAILER: u16 = 0x20;
}

/// One car (`grMc_CarEntry`, 0x2C bytes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Car {
    /// x0: the distance travelled, never wrapped (the pack order's key).
    pub distance: f32,
    /// x4: the place on the track in [0, 1).
    pub place: f32,
    /// x8: the speed, in track per tick.
    pub speed: f32,
    /// xC: the lane between the two splines, 0 (right) to 1 (left).
    pub lane: f32,
    /// x10: the damage fighters dealt it.
    pub damage: f32,
    /// x14: its position last tick.
    pub position: Vec3,
    /// x20.
    pub flags: u16,
    /// x22 b0: exploded; b1: its passing sound played.
    pub wrecked: bool,
    pub passed: bool,
    /// x24: its stage material item while it is near the stage.
    pub item: Option<u32>,
    /// x28: whether a smoke generator (0x116 or 0x119) follows it.
    pub smoking: bool,
}

/// grMc_803E3B7C: the grid, two lanes per row.
const GRID: [(f32, f32); CARS] = {
    let mut grid = [(0.0, 0.0); CARS];
    let places = [
        0.9999, 0.9988, 0.9977, 0.9966, 0.9955, 0.9944, 0.9933, 0.9922, 0.9911, 0.9900,
        0.9889, 0.9878, 0.9867, 0.9856, 0.9845,
    ];
    let mut row = 0;
    while row < 15 {
        grid[2 * row] = (places[row], 0.37);
        grid[2 * row + 1] = (places[row], 0.87);
        row += 1;
    }
    grid
};

/// The cars and the pack order.
#[derive(Clone, Debug, PartialEq)]
pub struct Cars {
    pub cars: [Car; CARS],
    /// grMc_8049F440, behind the word before it: `order[0]` is the last
    /// word of grZe_8049F170 (Brinstar's data, zero unless Brinstar
    /// wrote it), which the insertion sort reads and swaps as index -1.
    pub order: [i32; CARS + 1],
    /// Map 2's xC4: ticks until the next pair of touching cars sparks.
    pub spark_cooldown: i16,
}

/// The track's two splines with the joints that carry them (map 0x1E's
/// bones 9 and 5, xE0 and xDC).
pub struct Splines<'a> {
    pub right: &'a Spline,
    pub right_matrix: Mtx,
    pub left: &'a Spline,
    pub left_matrix: Mtx,
}

/// What a car's step asks of the stage.
pub trait CarEngine {
    /// grLib_801C96F8(kind, 0, &position): a common-bank generator at a
    /// point (0x11A the sparks, 0xE3 the explosion).
    fn effect_at(&mut self, kind: u32, position: Vec3);
    /// grMuteCity_801F2AB0(kind, car joint): a common-bank generator that
    /// follows the car (0x116 smoke, 0x119 a wreck's).
    fn follow_car(&mut self, car: usize, kind: u32) -> bool;
    /// grLib_801C98A0(car joint): every generator on the car ends.
    fn end_car_effects(&mut self, car: usize);
    /// grMaterial_801C8CFC(0, 2, ...): the car's stage material item with
    /// its hurt capsule (grMaterial_801C8DE0) and hits on (801C8E08).
    fn create_item(&mut self, car: usize) -> Option<u32>;
    /// grMaterial_801C8CDC.
    fn destroy_item(&mut self, item: u32);
    /// Camera_RequestQuake(QuakeKind_Loop).
    fn quake(&mut self);
    /// Camera_GetTransformPosition's z.
    fn camera_z(&self) -> f32;
    fn rng(&mut self) -> &mut HsdRng;
    /// The car joints (grFZeroCar_801CAFBC's), which the step moves and
    /// the items and generators read.
    fn joints(&mut self) -> &mut CarJoints;
}

/// grMuteCity_801F094C's and 801F1A34's spline reads: past 0.93 and
/// before 0.01 the path is the straight line from 0.93 to 0.01, by
/// `t - 0.93` or `0.07 + t` over 0.08 of it.
const WRAP_START: f32 = 0.93;
const WRAP_END: f32 = 0.01;
const WRAP_OFFSET: f32 = 0.069_999_99;
const WRAP_SPAN: f32 = 0.079_999_99;

/// The wrapped read of `curve` (splGetSplinePoint or lbShadow_8000E9F0).
pub fn wrapped(curve: impl Fn(f32) -> Vec3, t: f32) -> Vec3 {
    let along = if t > WRAP_START {
        t - WRAP_START
    } else if t < WRAP_END {
        WRAP_OFFSET + t
    } else {
        return curve(t);
    };
    let from = curve(WRAP_START);
    let to = curve(WRAP_END);
    // lbVector_Sub, the fdivs, three fmuls, lbVector_Add.
    let ratio = along / WRAP_SPAN;
    let d = Vec3::new(to.x - from.x, to.y - from.y, to.z - from.z);
    let d = Vec3::new(d.x * ratio, d.y * ratio, d.z * ratio);
    Vec3::new(d.x + from.x, d.y + from.y, d.z + from.z)
}

pub fn point(spline: &Spline, t: f32) -> Vec3 {
    hsd_anim::spline::spline_point(spline, t).expect("a car spline parameter in [0, 1]")
}
pub fn tangent(spline: &Spline, t: f32) -> Vec3 {
    hsd_anim::spline::tangent(spline, t).expect("a car spline parameter in [0, 1]")
}

/// `lb_8000B1CC(jobj, &local, &out)` for a joint with a parent: a zero
/// offset reads the translation column, any other goes through
/// PSMTXMultVec.
pub fn to_world(matrix: &Mtx, local: Vec3) -> Vec3 {
    if local.x == 0.0 && local.y == 0.0 && local.z == 0.0 {
        return Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]);
    }
    let mut out = Vec3::ZERO;
    hsd_anim::mtx::mtx_mult_vec(matrix, &local, &mut out);
    out
}

/// A direction along a spline in the world: the tangent through the
/// joint's matrix (translation included), less the joint's origin,
/// normalized (lbVector_Normalize).
fn world_direction(matrix: &Mtx, tangent: Vec3) -> Vec3 {
    let tip = to_world(matrix, tangent);
    let origin = to_world(matrix, Vec3::ZERO);
    normalize(Vec3::new(tip.x - origin.x, tip.y - origin.y, tip.z - origin.z))
}

/// The pit lane (x4 in (0.827, 0.914)): a car there is placed and
/// oriented as at the start, and a wreck keeps rolling.
const PIT_START: f32 = 0.827;
const PIT_END: f32 = 0.914;
/// Within 0.2 of the track's middle under the stage a passing car quakes
/// the camera; within this box it is near the stage.
const PASSING_RANGE: f32 = 0.2;
const NEAR_HALF_WIDTH: f32 = 200.0;
const PASSING_DEPTH: (f32, f32) = (-1350.0, 50.0);
const ITEM_DEPTH: (f32, f32) = (-100.0, 50.0);
/// The distance within which a pair's sparks are heard.
const SPARK_HEARING: f32 = 300.0;
/// Ticks between sparks.
const SPARK_COOLDOWN: i16 = 7;
/// A wreck smokes only in front of the camera and within these depths.
const WRECK_DEPTH: (f64, f64) = (5000.0, -1500.0);
/// The common-bank effects.
pub const SPARK_EFFECT: u32 = 0x11A;
pub const EXPLOSION_EFFECT: u32 = 0xE3;
pub const SMOKE_EFFECT: u32 = 0x116;
pub const WRECK_EFFECT: u32 = 0x119;

impl Cars {
    /// grMuteCity_801F0F4C (0x801F0F4C): the grid; the pack order in car
    /// order. `order[0]` (the word before the table) is not written.
    pub fn new(word_before_order: i32) -> Self {
        let mut cars = [Car {
            distance: 0.0,
            place: 0.0,
            speed: 0.0,
            lane: 0.0,
            damage: 0.0,
            position: Vec3::ZERO,
            flags: 0,
            wrecked: false,
            passed: false,
            item: None,
            smoking: false,
        }; CARS];
        let mut order = [0; CARS + 1];
        order[0] = word_before_order;
        for (i, car) in cars.iter_mut().enumerate() {
            let (mut place, lane) = GRID[i];
            // Compared in double; no grid place leaves [0, 1].
            if f64::from(place) > 1.0 {
                place -= 1.0;
            } else if f64::from(place) < 0.0 {
                place += 1.0;
            }
            car.distance = place;
            car.place = place;
            car.lane = lane;
            order[i + 1] = i as i32;
        }
        Self {
            cars,
            order,
            spark_cooldown: 0,
        }
    }

    /// `grMc_8049F4B8[order[slot]]`: `slot` -1 is the word before the
    /// table, which only ever holds car indices once a sort wrote it.
    fn ordered(&self, slot: i32) -> usize {
        let car = self.order[(slot + 1) as usize];
        assert!(
            (0..CARS as i32).contains(&car),
            "grMuteCity_801F1328: pack order word {car} outside grMc_8049F4B8"
        );
        car as usize
    }

    /// grMuteCity_801F1328 (0x801F1328): sort the pack by distance (an
    /// insertion sort that also compares slot 0 with the word before the
    /// table), mark close pairs, leaders and trailers, then drive each car.
    pub fn plan(&mut self, parameters: &Parameters, rng: &mut HsdRng) {
        for i in 1..CARS as i32 {
            // 801F1360: i + 1 comparisons, down to slot 0 against slot -1.
            let mut j = i;
            while j >= 0 {
                let here = self.ordered(j);
                let before = self.ordered(j - 1);
                if self.cars[here].distance > self.cars[before].distance {
                    self.order[(j + 1) as usize] = before as i32;
                    self.order[j as usize] = here as i32;
                    j -= 1;
                } else {
                    break;
                }
            }
        }
        for slot in 0..CARS as i32 {
            let car = self.ordered(slot);
            self.cars[car].flags = 0;
        }
        for slot in 0..CARS as i32 {
            let me = self.ordered(slot);
            if self.cars[me].wrecked {
                continue;
            }
            for k in 1..CARS as i32 {
                let mut other_slot = slot + k;
                if other_slot >= CARS as i32 {
                    other_slot -= CARS as i32;
                }
                let other = self.ordered(other_slot);
                if self.cars[other].wrecked {
                    continue;
                }
                let mine = self.cars[me].place;
                let theirs = self.cars[other].place;
                let gap = if mine > theirs {
                    mine - theirs
                } else {
                    1.0 + (mine - theirs)
                };
                if f64::from(gap) > 0.002 {
                    break;
                }
                self.cars[other].flags |= flag::CLOSE_BEHIND;
                let (a, b) = (self.cars[me].position, self.cars[other].position);
                let (dx, dy, dz) = (a.x - b.x, a.y - b.y, a.z - b.z);
                // 801F1564..74: three fmuls, then z^2 + (x^2 + y^2).
                if dz * dz + (dx * dx + dy * dy) < 900.0 {
                    self.cars[me].flags |= flag::TOUCHING;
                    self.cars[other].flags |= flag::TOUCHING;
                }
                if self.cars[me].lane > self.cars[other].lane {
                    self.cars[me].flags |= flag::FASTER;
                    self.cars[other].flags |= flag::SLOWER;
                } else {
                    self.cars[me].flags |= flag::SLOWER;
                    self.cars[other].flags |= flag::FASTER;
                }
            }
        }
        for slot in 0..5 {
            let car = self.ordered(slot);
            self.cars[car].flags |= flag::LEADER;
        }
        for slot in 25..CARS as i32 {
            let car = self.ordered(slot);
            self.cars[car].flags |= flag::TRAILER;
        }
        for car in 0..CARS {
            self.drive(car, parameters, rng);
        }
    }

    /// grMuteCity_801F106C (0x801F106C): one car's speed and lane.
    fn drive(&mut self, index: usize, p: &Parameters, rng: &mut HsdRng) {
        let car = &mut self.cars[index];
        let flags = car.flags;
        if car.wrecked {
            if car.place > PIT_START && car.place < PIT_END {
                if car.speed < 0.001 {
                    car.speed = 0.001;
                }
            } else {
                car.speed *= 0.95;
                if car.speed < 0.000_01 {
                    car.speed = 0.0;
                }
            }
            return;
        }
        if flags & flag::CLOSE_BEHIND != 0 {
            if flags & flag::TOUCHING != 0 {
                car.speed -= p.braking;
            } else {
                match rng.randi(4) {
                    0 => car.speed -= p.braking,
                    1 | 2 => {
                        if flags & flag::FASTER != 0 {
                            car.lane += p.lane_step;
                            if f64::from(car.lane) > 0.9 {
                                car.lane = 0.9;
                            }
                        } else {
                            car.lane -= p.lane_step;
                            if f64::from(car.lane) < 0.1 {
                                car.lane = 0.1;
                            }
                        }
                    }
                    _ => {}
                }
            }
        } else {
            car.speed += p.acceleration;
            if car.lane > 0.7 + p.lane_centering {
                car.lane -= p.lane_centering;
            } else if car.lane < 0.3 - p.lane_centering {
                car.lane += p.lane_centering;
            }
        }
        if flags & flag::TOUCHING != 0 {
            if flags & flag::FASTER != 0 {
                car.lane += p.lane_step;
                if f64::from(car.lane) > 1.0 {
                    car.lane = 1.0;
                }
            } else {
                car.lane -= p.lane_step;
                if f64::from(car.lane) < 0.0 {
                    car.lane = 0.0;
                }
            }
        }
        let top = if flags & flag::LEADER != 0 {
            p.leader_top_speed
        } else if flags & flag::TRAILER != 0 {
            p.trailer_top_speed
        } else {
            p.top_speed
        };
        if car.speed > top {
            car.speed = top;
        } else if f64::from(car.speed) < 0.0 {
            car.speed = 0.0;
        }
    }

    /// grMuteCity_801F1A34 (0x801F1A34): every car one step along the
    /// track and into place, then what its place and damage cause.
    /// `track_middle` is half the track's two search parameters (map
    /// 0x1E's xD4 and xD8, 0x801F1AB8 fadds then fmuls).
    pub fn step(
        &mut self,
        parameters: &Parameters,
        splines: &Splines<'_>,
        track_middle: f32,
        engine: &mut impl CarEngine,
    ) {
        let mut sounds = 0;
        if self.spark_cooldown != 0 {
            self.spark_cooldown -= 1;
        }
        for index in 0..CARS {
            let car = &mut self.cars[index];
            car.place += car.speed;
            car.distance += car.speed;
            if car.place > 1.0 {
                car.place -= 1.0;
            }
            let mut t = car.place;
            let right = to_world(
                &splines.right_matrix,
                wrapped(|u| point(splines.right, u), t),
            );
            let left = to_world(&splines.left_matrix, wrapped(|u| point(splines.left, u), t));
            let side = normalize(Vec3::new(left.x - right.x, left.y - right.y, left.z - right.z));
            if t > PIT_START && t < PIT_END {
                t = 0.0;
            }
            let along_right =
                world_direction(&splines.right_matrix, wrapped(|u| tangent(splines.right, u), t));
            let along_left =
                world_direction(&splines.left_matrix, wrapped(|u| tangent(splines.left, u), t));
            let forward = normalize(Vec3::new(
                along_right.x + along_left.x,
                along_right.y + along_left.y,
                along_right.z + along_left.z,
            ));
            let up = normalize(cross(forward, side));
            let forward = normalize(cross(side, up));
            // 801F1FBC..801F2004: fmadds across the lanes, then a double
            // fmadd of 3 along the up vector, rounded once.
            let lane = car.lane;
            let place = |r: f32, l: f32, u: f32| -> f32 {
                fmadd(3.0, f64::from(u), f64::from(fmadds(l - r, lane, r))) as f32
            };
            let position = Vec3::new(
                place(right.x, left.x, up.x),
                place(right.y, left.y, up.y),
                place(right.z, left.z, up.z),
            );
            car.position = position;
            engine.joints().set_translate(index, position);
            engine.joints().set_rotation(index, orientation(forward, side, up));
            self.after_placing(index, position, parameters, &mut sounds, track_middle, engine);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn after_placing(
        &mut self,
        index: usize,
        position: Vec3,
        parameters: &Parameters,
        sounds: &mut i32,
        track_middle: f32,
        engine: &mut impl CarEngine,
    ) {
        let flags = self.cars[index].flags;
        if flags & flag::TOUCHING != 0 && flags & flag::SLOWER != 0 && self.spark_cooldown == 0 {
            engine.effect_at(SPARK_EFFECT, position);
            if *sounds < 1 {
                if position.z < 0.0 {
                    // 801F22B4..C0: x^2 + y^2, then fmadds of z^2; the
                    // inline sqrtf.
                    let squared = fmadds(position.z, position.z, position.x * position.x + position.y * position.y);
                    let distance = gekko_math::msl::sqrtf(squared);
                    if distance < SPARK_HEARING {
                        // Ground_801C5440 picks one of three sounds;
                        // Ground_801C5630 its volume.
                        engine.rng().randi(3);
                        *sounds += 1;
                    }
                }
                self.spark_cooldown = SPARK_COOLDOWN;
            }
        }
        self.damage_effects(index, position, parameters, engine);
        let car = &mut self.cars[index];
        if car.wrecked {
            let z = f64::from(position.z);
            if position.z > engine.camera_z() || z > WRECK_DEPTH.0 || z < WRECK_DEPTH.1 {
                if car.smoking {
                    engine.end_car_effects(index);
                    car.smoking = false;
                }
            } else if !car.smoking {
                car.smoking = engine.follow_car(index, WRECK_EFFECT);
            }
            return;
        }
        let gap = if track_middle < car.place {
            (1.0 + track_middle) - car.place
        } else {
            track_middle - car.place
        };
        let in_box = |depth: (f32, f32)| {
            -NEAR_HALF_WIDTH < position.x
                && position.x < NEAR_HALF_WIDTH
                && -NEAR_HALF_WIDTH < position.y
                && position.y < NEAR_HALF_WIDTH
                && f64::from(depth.0) < f64::from(position.z)
                && position.z < depth.1
        };
        if f64::from(gap) < f64::from(PASSING_RANGE) && f64::from(gap) > f64::from(-PASSING_RANGE) {
            if in_box(PASSING_DEPTH) {
                engine.quake();
                if !car.passed {
                    // Ground_801C53EC plays one of three sounds.
                    engine.rng().randi(3);
                    car.passed = true;
                }
            } else if car.passed {
                car.passed = false;
            }
        }
        let car = &mut self.cars[index];
        if in_box(ITEM_DEPTH) {
            if !car.wrecked && car.item.is_none() {
                car.item = engine.create_item(index);
            }
        } else if let Some(item) = car.item.take() {
            engine.destroy_item(item);
        }
    }

    /// The smoke past x2C damage and the explosion past x30.
    fn damage_effects(
        &mut self,
        index: usize,
        position: Vec3,
        parameters: &Parameters,
        engine: &mut impl CarEngine,
    ) {
        let car = &mut self.cars[index];
        if car.damage > parameters.explode_damage {
            if car.wrecked {
                return;
            }
            engine.end_car_effects(index);
            car.smoking = false;
            engine.effect_at(EXPLOSION_EFFECT, position);
            // The wreck's model (Ground_801C13D0(5, 0)) turned by
            // 1.0471976 * HSD_Randf() - 0.5235988 (801F243C fmsubs), a
            // translation the next store overwrites.
            engine.rng().randf();
            if let Some(item) = car.item.take() {
                engine.destroy_item(item);
            }
            car.wrecked = true;
        } else if car.damage > parameters.smoke_damage && !car.smoking {
            car.smoking = engine.follow_car(index, SMOKE_EFFECT);
        }
    }

    /// grMuteCity_801F173C .. 801F19F4 (grMc_803E3C6C[car]): a car's item
    /// took `damage` (its yaku.x18 callback).
    pub fn take_damage(&mut self, car: usize, damage: f32) {
        self.cars[car].damage += damage;
    }

    pub fn car_of_item(&self, item: u32) -> Option<usize> {
        self.cars.iter().position(|c| c.item == Some(item))
    }
}
