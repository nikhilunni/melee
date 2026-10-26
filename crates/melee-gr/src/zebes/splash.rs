//! The acid's splashes along the parts' undersides: grZakoGenerator_801CA394
//! / 801CA43C (grzakogenerator.c) with Brinstar's segments and callbacks.
//! Each tick, every segment the acid's surface crosses draws (its chance
//! is 1.0, so it always passes) and splashes where it crosses
//! (grZebes_801DCB64 / 801DCBB0: grLib_801C96F8(0xBA or 0xBB, 0)).
use gekko_math::{fma::fmadds, HsdRng};
use hsd_types::Vec3;

/// grZe_803E1B90: map 6's segments, on its bone 14.
pub const LEFT_SEGMENTS: [[Vec3; 2]; 10] = [
    [Vec3::new(-40.5, -10.0, 13.0), Vec3::new(-40.5, -24.0, 13.0)],
    [Vec3::new(-5.5, -13.0, 23.5), Vec3::new(-5.5, -27.0, 23.5)],
    [Vec3::new(-5.5, 0.5, 0.0), Vec3::new(-5.5, -10.0, 0.0)],
    [Vec3::new(-8.0, 4.5, 32.0), Vec3::new(-8.0, -14.5, 32.0)],
    [Vec3::new(-34.0, 0.5, 24.0), Vec3::new(-34.0, -17.5, 24.0)],
    [Vec3::new(-23.5, -12.0, 5.5), Vec3::new(-23.5, -23.0, 5.5)],
    [Vec3::new(6.5, 4.0, 18.0), Vec3::new(6.5, -10.5, 18.0)],
    [Vec3::new(10.0, 0.5, 9.5), Vec3::new(10.0, -10.0, 9.5)],
    [Vec3::new(-21.5, 2.0, 15.5), Vec3::new(-21.5, -16.0, 15.5)],
    [Vec3::new(-59.0, 27.5, 6.0), Vec3::new(-59.0, 17.0, 6.0)],
];
/// grZe_803E1C80: map 7's segments, on its bone 1.
pub const RIGHT_SEGMENTS: [[Vec3; 2]; 4] = [
    [Vec3::new(42.5, -11.0, 0.0), Vec3::new(42.5, -25.0, 0.0)],
    [Vec3::new(30.0, 1.0, 11.0), Vec3::new(30.0, -17.0, 11.0)],
    [Vec3::new(45.0, -5.0, 7.5), Vec3::new(45.0, -14.0, 7.5)],
    [Vec3::new(56.0, 15.0, 6.0), Vec3::new(56.0, 4.5, 6.0)],
];
/// grZebes_801DCB64 / 801DCBB0: segments from these on splash 0xBB, the
/// rest 0xBA (common bank).
pub const LEFT_WIDE_FROM: usize = 9;
pub const RIGHT_WIDE_FROM: usize = 3;
pub const SPLASH: u32 = 0xBA;
pub const WIDE_SPLASH: u32 = 0xBB;

/// Below this the segment counts as level and the crossing is drawn.
const LEVEL: f32 = 0.0001;

/// grZakoGenerator_801CA43C (0x801CA43C) over segments already in world
/// space (lb_8000B1CC of each end, all before the first test), with its
/// chance 1.0: `splash(position, index)` for every crossing. Returns the
/// crossings.
pub fn splash_crossings(
    segments: &[[Vec3; 2]],
    height: f32,
    rng: &mut HsdRng,
    mut splash: impl FnMut(Vec3, usize, &mut HsdRng),
) -> usize {
    const CHANCE: f32 = 1.0;
    let mut count = 0;
    for (index, &[start, end]) in segments.iter().enumerate() {
        if (start.y > height && end.y > height) || (start.y < height && end.y < height) {
            continue;
        }
        if rng.randf() >= CHANCE {
            continue;
        }
        let rise = end.y - start.y;
        let t = if (if rise < 0.0 { -rise } else { rise }) > LEVEL {
            (height - start.y) / rise
        } else {
            rng.randf()
        };
        // retail 0x801CA560 / 0x801CA578 (or 0x801CA594 / 0x801CA5AC):
        // fmadds.
        let position = Vec3::new(
            fmadds(t, end.x - start.x, start.x),
            height,
            fmadds(t, end.z - start.z, start.z),
        );
        splash(position, index, rng);
        count += 1;
    }
    count
}
