//! Retail `bl HSD_Randf` addresses, rather than return addresses.
//!
//! FD: an eligible generator with nonnegative emission rate draws at
//! `0x8039EF00` even when its rate is zero. For each emitted shape-8 particle,
//! nonzero/non-pi latitude range draws at `0x8039EB74`, then azimuth at
//! `0x8039EBCC`. Negative radius suppresses the radius draw. Immediate
//! interpretation reaches BA and draws R, G, B, A at the four sites below,
//! even if a channel's delta is zero. This is six draws per *emission*, not
//! per surviving particle. Masks and kind bit 0x800 pause both lifetimes
//! and RNG. Other supported scripts can draw at additional sites.
use gekko_math::rng::HsdRng;

pub const EMISSION_COUNT: u32 = 0x8039_EF00;
pub const SPHERE_LATITUDE: u32 = 0x8039_EB74;
pub const SPHERE_AZIMUTH: u32 = 0x8039_EBCC;
pub const PRIMARY_COLOR: [u32; 4] = [0x8039_B088, 0x8039_B0F4, 0x8039_B160, 0x8039_B1CC];
/// hsd_8039DAD4+0x5B4: negative-angle disc pre-loop, count >= 1.
pub const DISC_INITIAL_ANGLE: u32 = 0x8039_E088;
/// hsd_8039DAD4+0x710: disc/cone emission with nonnegative radius.
pub const DISC_RADIUS: u32 = 0x8039_E1E4;
/// hsd_8039DAD4+0x900: nonnegative-angle disc, modes other than 6/7.
pub const DISC_AZIMUTH: u32 = 0x8039_E3D4;
/// hsd_8039F05C+0x1F4: kind bit 0x100 clear and emission rate >= 0.
pub const INITIAL_EMISSION_COUNT: u32 = 0x8039_F250;
/// hsd_8039930C+0x1504: AC random size, including zero range.
pub const RANDOM_SIZE: u32 = 0x8039_A810;
/// hsd_8039930C+0x22D4: BD random target speed, including zero velocity.
pub const RANDOM_SPEED: u32 = 0x8039_B5E0;
/// hsd_8039930C+0x31EC/+0x3280: E4/E5 with low two mode bits == 3.
pub const RANDOM_TEXTURE_FLIP: [u32; 2] = [0x8039_C4F8, 0x8039_C58C];
/// hsd_8039930C+0x3564: ED with nonzero division count.
pub const DISCRETE_ROTATION: u32 = 0x8039_C870;
pub const FD_EMISSION: [u32; 6] = [
    SPHERE_LATITUDE,
    SPHERE_AZIMUTH,
    PRIMARY_COLOR[0],
    PRIMARY_COLOR[1],
    PRIMARY_COLOR[2],
    PRIMARY_COLOR[3],
];

/// Ordered observations for testing and comparison with the Dolphin ledger.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct DrawLog(pub Vec<u32>);
impl DrawLog {
    pub(crate) fn draw(&mut self, rng: &mut HsdRng, address: u32) -> f32 {
        self.0.push(address);
        rng.randf()
    }
}

impl Clone for DrawLog {
    fn clone(&self) -> Self {
        Self(hsd_types::storage::clone_vec(&self.0))
    }
}
