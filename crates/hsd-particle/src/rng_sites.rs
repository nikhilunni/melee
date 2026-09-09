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
pub const FD_EMISSION: [u32; 6] = [
    SPHERE_LATITUDE,
    SPHERE_AZIMUTH,
    PRIMARY_COLOR[0],
    PRIMARY_COLOR[1],
    PRIMARY_COLOR[2],
    PRIMARY_COLOR[3],
];

/// Ordered observations for testing and comparison with the Dolphin ledger.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DrawLog(pub Vec<u32>);
impl DrawLog {
    pub(crate) fn draw(&mut self, rng: &mut HsdRng, address: u32) -> f32 {
        self.0.push(address);
        rng.randf()
    }
}
