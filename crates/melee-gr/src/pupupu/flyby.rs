//! Dream Land's background Bronto Burt flyby, grOldPupupu_80210D10
//! (0x80210D10, groldpupupu.c:351-436).
//!
//! When the map-8 controller's timer runs out it picks one of ten flight
//! paths, sometimes a group of three, places the models just beyond the
//! camera's view and draws the next delay. The map-2 models it creates carry
//! no particle keys and their procs (grOldPupupu_80210C34) only delete them
//! when the animation ends, so the simulation keeps the placement as data
//! (`Flyby`) for presentation; the gameplay-visible effect is the RNG draws.
use super::{range, Pupupu};
use gekko_math::{fma, HsdRng};

/// grOp_803E67B0: one flight path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlightPath {
    /// Enters from the right edge (`1`) or the left edge (`-1`).
    pub from_right: bool,
    /// The path may fly as a group of three (one chance in five).
    pub may_group: bool,
    /// Ground_801C3FA4 joint index made visible on each spawned model.
    pub visible_joint: i16,
}

const fn path(side: i8, may_group: bool, visible_joint: i16) -> FlightPath {
    FlightPath {
        from_right: side == 1,
        may_group,
        visible_joint,
    }
}

/// grOp_803E67B0.
pub const FLIGHT_PATHS: [FlightPath; 10] = [
    path(-1, true, 1),
    path(1, true, 3),
    path(1, true, 5),
    path(-1, true, 7),
    path(-1, false, 9),
    path(1, false, 11),
    path(-1, false, 13),
    path(1, false, 15),
    path(-1, false, 17),
    path(1, false, 19),
];

/// A group this large when the one-in-five draw succeeds.
const GROUP_SIZE: usize = 3;
/// Models start this far past the camera edge (world units).
const EDGE_MARGIN: f32 = 50.0;
/// The camera edge is never taken closer to the centre than this.
const MIN_EDGE: f32 = 200.0;
/// Spacing between group members along the flight path.
const SPACING: f32 = 10.0;
/// Base flight height before the random offset.
const BASE_HEIGHT: f32 = 20.0;
/// Background depth, scaled by the map scale.
const DEPTH: f32 = -150.0;
/// Group members start their animation this many frames apart.
const FRAME_STAGGER: f32 = 40.0;

/// One flyby: what grOldPupupu_80210D10 decided before placing the models.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flyby {
    pub path: FlightPath,
    pub count: usize,
    /// Ground_801C0498: the map scale, which also scales the placement.
    pub scale: f32,
    pub y: f32,
    pub z: f32,
}

impl Flyby {
    /// Each model's world translation and animation start frame, given the
    /// camera's ground-plane view edges (Camera_800307D0, 0x800307D0).
    pub fn placements(&self, camera_left: f32, camera_right: f32) -> [(f32, f32, f32, f32); 3] {
        let count = self.count as f32;
        let (step, mut x) = if self.path.from_right {
            let right = if camera_right < MIN_EDGE {
                MIN_EDGE
            } else {
                camera_right
            };
            // retail 0x80210DF8 fadds, 0x80210E04 fnmsubs
            (-SPACING, fma::fnmsubs(count, -SPACING, EDGE_MARGIN + right))
        } else {
            let left = if camera_left > -MIN_EDGE {
                -MIN_EDGE
            } else {
                camera_left
            };
            // retail 0x80210E44 fsubs, 0x80210E50 fnmsubs
            (SPACING, fma::fnmsubs(count, SPACING, left - EDGE_MARGIN))
        };
        let mut out = [(0.0, 0.0, 0.0, 0.0); 3];
        for (i, slot) in out.iter_mut().enumerate().take(self.count) {
            // retail 0x80210EF8 fmuls; 0x8021105C fmuls (start frame)
            *slot = (self.scale * x, self.y, self.z, FRAME_STAGGER * i as f32);
            // retail 0x80211078 fadds
            x += step;
        }
        out
    }
}

impl Pupupu {
    /// grOldPupupu_80210D10 (0x80210D10): signed post-decrement; when the
    /// old value was negative, draw a flyby and the next delay.
    pub fn tick_background(&mut self, scale: f32, rng: &mut HsdRng) -> Option<Flyby> {
        let old = self.background_timer;
        self.background_timer = old.wrapping_sub(1);
        if old >= 0 {
            return None;
        }
        // retail 0x80210D68: HSD_Randi(10)
        let path = FLIGHT_PATHS[rng.randi(10) as usize];
        // retail 0x80210D88: HSD_Randi(5), only for paths that may group.
        let count = if path.may_group && rng.randi(5) == 0 {
            GROUP_SIZE
        } else {
            1
        };
        // retail 0x80210E54: HSD_Randf after Camera_800307D0 (which draws nothing).
        let random = rng.randf();
        let height = f32::from(self.parameters.background_height);
        // retail 0x80210E90 fmsubs, 0x80210EA8 fmadds, 0x80210EAC fmuls
        let y = scale * fma::fmadds(height, fma::fmsubs(2.0, random, 1.0), BASE_HEIGHT);
        // retail 0x80210E6C fmuls
        let z = DEPTH * scale;
        // retail 0x802110A4/0x802110C4: the next delay.
        self.background_timer = range(rng, self.parameters.background_delay.map(i32::from)) as i16;
        Some(Flyby {
            path,
            count,
            scale,
            y,
            z,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pupupu::Parameters;

    fn stage(timer: i16) -> Pupupu {
        let mut stage = Pupupu::initialize(
            Parameters {
                background_delay: [3000, 4000],
                background_height: 30,
                wind_delay: [600, 1200],
                wind_speed: 0.2,
                right_bounds: [-17.0, 76.0],
                left_bounds: [-18.0, -74.0],
                vertical_bounds: [40.0, -10.0],
                blink_delay: [180, 360],
            },
            &mut HsdRng::new(1),
        );
        stage.background_timer = timer;
        stage
    }

    #[test]
    fn timer_fires_only_after_passing_zero() {
        let mut stage = stage(0);
        let mut rng = HsdRng::new(7);
        assert_eq!(stage.tick_background(1.0, &mut rng), None);
        assert_eq!(stage.background_timer, -1);
        assert_eq!(rng.seed, 7);
        assert!(stage.tick_background(1.0, &mut rng).is_some());
        assert!((3000..4000).contains(&stage.background_timer));
    }

    #[test]
    fn draws_follow_retail_order() {
        for seed in 0..64_u32 {
            let mut rng = HsdRng::new(seed);
            let flyby = stage(-1).tick_background(1.0, &mut rng).unwrap();
            let mut expected = HsdRng::new(seed);
            let path = FLIGHT_PATHS[expected.randi(10) as usize];
            let count = if path.may_group && expected.randi(5) == 0 {
                3
            } else {
                1
            };
            expected.randf();
            expected.randi(1000);
            assert_eq!((flyby.path, flyby.count), (path, count));
            assert_eq!(rng.seed, expected.seed);
            assert_eq!(flyby.z.to_bits(), (-150.0_f32).to_bits());
            assert!((-10.0..=50.0).contains(&flyby.y));
        }
    }

    #[test]
    fn groups_start_past_the_camera_edge() {
        let flyby = |from_right| Flyby {
            path: FlightPath {
                from_right,
                may_group: true,
                visible_joint: 1,
            },
            count: 3,
            scale: 1.0,
            y: 0.0,
            z: -150.0,
        };
        let right = flyby(true).placements(-100.0, 100.0);
        assert_eq!([right[0].0, right[1].0, right[2].0], [280.0, 270.0, 260.0]);
        assert_eq!([right[0].3, right[1].3, right[2].3], [0.0, 40.0, 80.0]);
        let left = flyby(false).placements(-300.0, 100.0);
        assert_eq!([left[0].0, left[1].0, left[2].0], [-380.0, -370.0, -360.0]);
    }
}
