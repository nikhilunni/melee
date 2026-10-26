//! Princess Peach's Castle (`gr/grcastle.c`, GrCs.dat, StKind 0x04).
//!
//! Map 3 is the castle roof and the stage's controller: it keeps the flags'
//! wind (a directional dynamics field whose direction and strength wander,
//! drawing every tick), counts down to the next switch and to the next
//! Bullet Bill. Map 6 holds the two side platforms, raised by fighters
//! standing on them and lowered when they leave. Maps 18..20 are the three
//! switches, which appear on a bone of the roof or of a platform; a press
//! fills in one of the block sets (maps 5, 7 and 17) for a while. Maps
//! 8..16 are the Bullet Bill's nine flights; each launches the cannon
//! (map 2) partway and ends in the explosion (map 1). The castle's moat is
//! a fighter device (ftCo_800C06E8 type 6) that splashes and buoys.
pub mod bills;
pub mod platforms;
pub mod procs;
pub mod switches;
pub mod wind;

use hsd_types::Vec3;

/// One Bullet Bill flight's row (`grCastleParams_Entry`, yakumono_param
/// +0x5C, stride 0x14).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flight {
    /// +0x0: ticks from the launch until the cannon appears.
    pub cannon_delay: i32,
    /// +0x4: the launch sound's pan (Ground_801C5694).
    pub sound: f32,
    /// +0x8: the cannon's rotation.
    pub cannon_rotation: Vec3,
}

/// `grCastle_YakumonoParam` (grcastle.c, GrCs.dat `yakumono_param`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parameters {
    /// +0x0 / +0x2: the wait before the next switch, `rand_range(x2, x0)`.
    pub switch_wait: [i32; 2],
    /// +0x4: the freeze a press puts on its fighter (ftLib_80086A4C).
    pub switch_freeze: i32,
    /// +0x6: the last switch's weight is divided by this.
    pub switch_repeat_divisor: i32,
    /// +0x8 / +0xA: the first Bullet Bill's wait, `x8 + HSD_Randi(xA)`.
    pub first_bill_wait: [i32; 2],
    /// +0xC / +0xE: the next one's.
    pub next_bill_wait: [i32; 2],
    /// +0x10 / +0x14: the platforms' top and bottom heights.
    pub platform_top: f32,
    pub platform_bottom: f32,
    /// +0x18: nonzero starts the platforms at the bottom.
    pub platform_starts_low: f32,
    /// +0x20 / +0x24 / +0x28: rising, `weight * x20 + x24` up to x28.
    pub rise: [f32; 3],
    /// +0x2C / +0x30 / +0x34: sinking, the same.
    pub sink: [f32; 3],
    /// +0x38: ticks with weight before a platform rises.
    pub rise_delay: f32,
    /// +0x3C: ticks without weight before it sinks.
    pub sink_delay: f32,
    /// +0x40 / +0x42: how long a block set stays, `rand_range(x42, x40)`.
    pub blocks_time: [i32; 2],
    /// +0x44: the ticks it blinks before it goes.
    pub blocks_blink: i32,
    /// +0x48 / +0x4C: the blocks' growth and shrinking per tick.
    pub block_grow: f32,
    pub block_shrink: f32,
    /// +0x50: the launch speed of a `?` block's item.
    pub item_launch: f32,
    /// +0x54: the ticks a pressed switch lingers after its animation.
    pub switch_linger: i32,
    /// +0x58: the ticks between a flight's end and its explosion.
    pub bill_fuse: i32,
    /// +0x5C: the nine flights.
    pub flights: [Flight; 9],
    /// +0x110: the cannon's height above the Bill's second bone.
    pub cannon_lift: f32,
    /// +0x118..+0x124: the Bill's camera subject extents.
    pub bill_extents: [f32; 4],
    /// +0x12C: the cannon's four animation lengths (-1 for the last).
    pub cannon_waits: [i32; 4],
    /// +0x134..+0x140: the explosion's camera subject extents.
    pub explosion_extents: [f32; 4],
}

/// `rand_range(a, b)` (gr/inlines.h): equal ends draw nothing.
pub fn rand_range(a: i32, b: i32, rng: &mut gekko_math::HsdRng) -> i32 {
    crate::rand::range(rng, [a, b])
}

/// `range != 0 ? HSD_Randi(range) : 0`.
pub fn randi_or_zero(range: i32, rng: &mut gekko_math::HsdRng) -> i32 {
    if range != 0 {
        rng.randi(range)
    } else {
        0
    }
}

/// The moat (grCastle_801D09B8, the fighter device ftCo_800C06E8 type 6).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Moat {
    /// `14.0f * Ground_801C0498()`.
    pub level: f32,
}

impl Moat {
    pub fn new(map_scale: f32) -> Self {
        Self {
            level: procs::MOAT_LEVEL * map_scale,
        }
    }

    /// grCastle_801D09B8 (0x801D09B8): below the surface a falling fighter
    /// (pos_delta.y below -0.5) is buoyed by 0.1; crossing it downward this
    /// tick splashes at the surface, scaled by the fighter's camera size
    /// over 10 (grCastle_801D0A9C).
    pub fn sample(
        &self,
        current: Vec3,
        previous: Vec3,
        delta: Vec3,
        size: f32,
    ) -> (Vec3, Option<crate::garden::Splash>) {
        if current.y >= self.level {
            return (Vec3::ZERO, None);
        }
        let splash = (previous.y > self.level).then(|| crate::garden::Splash {
            position: Vec3::new(current.x, self.level, current.z),
            scale: size / 10.0,
            castle: true,
        });
        let lift = if delta.y < -0.5 {
            Vec3::new(0.0, 0.1, 0.0)
        } else {
            Vec3::ZERO
        };
        (lift, splash)
    }
}

/// grCastle_801D0FF0 (0x801D0FF0): the camera eye's lowest height is this
/// times the map scale (Camera_8002AF68).
pub const MIN_EYE_HEIGHT: f32 = 50.0;
