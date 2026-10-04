//! Color timers in `hsd_8039930C` (particle.c, retail 0x8039930C).
use crate::{program::Cursor, rng_sites::DrawLog, Error};
use gekko_math::{msl::fctiwz, rng::HsdRng};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorTrack {
    pub current: [u8; 4],
    pub target: [u8; 4],
    pub duration: u16,
    pub remaining: u16,
}
impl ColorTrack {
    pub fn new(current: [u8; 4]) -> Self {
        // memset initializes targets to zero, independently of current color.
        Self {
            current,
            target: [0; 4],
            duration: 0,
            remaining: 0,
        }
    }
    /// getColorPrimEnv (psdisp.c): read the interpolated display color without
    /// materializing it into the simulation's countdown endpoints.
    pub fn display_color(&self) -> [u8; 4] {
        let mut color = self.current;
        display_bytes(
            &self.current,
            &self.target,
            self.duration,
            self.remaining,
            &mut color,
        );
        color
    }
    pub(crate) fn tick(&mut self) {
        if self.duration != 0 {
            self.remaining = self.remaining.wrapping_sub(1);
            if self.remaining == 0 {
                self.duration = 0;
                self.current = self.target;
            }
        }
    }
    fn materialize(&mut self) {
        self.current = self.display_color();
    }
    pub(crate) fn setup(&mut self, mask: u8, pc: &mut Cursor<'_>) -> Result<(), Error> {
        self.materialize();
        self.duration = pc.timer()?;
        self.target = self.current;
        for channel in 0..4 {
            if mask & (1 << channel) != 0 {
                self.target[channel] = pc.byte()?;
            }
        }
        self.restart();
        Ok(())
    }
    fn restart(&mut self) {
        if self.duration == 0 {
            self.current = self.target;
        }
        self.remaining = self.duration;
    }
    /// E0, particle.c:2190-2349. One random offset per channel is shared
    /// by both tracks; materialize each old interpolation before restarting.
    pub(crate) fn random_dual(
        primary: &mut Self,
        environment: &mut Self,
        pc: &mut Cursor<'_>,
        rng: &mut HsdRng,
        draws: &mut DrawLog,
    ) -> Result<(), Error> {
        primary.materialize();
        environment.materialize();
        for (channel, site) in [0x8039_BB28, 0x8039_BBE4, 0x8039_BCA0, 0x8039_BD5C]
            .into_iter()
            .enumerate()
        {
            let random = draws.draw(rng, site);
            let delta = i32::from(pc.byte()? as i8) << 1;
            // retail 0x8039BB60/BB6C/BBB0 and subsequent channels:
            // fmuls then separate fadds for each target; no contraction.
            let offset = delta as f32 * random;
            for track in [&mut *primary, &mut *environment] {
                track.target[channel] =
                    fctiwz((f32::from(track.target[channel]) + offset).clamp(0.0, 255.0)) as u8;
            }
        }
        primary.restart();
        environment.restart();
        Ok(())
    }
    /// E9, particle.c:2341-2534 (retail 0x8039BE54..0x8039C3BC): after both
    /// tracks materialize, a flags byte (bits 0..3 the channels, 0x10 the
    /// primary track, 0x20 the environment) and a step count. R, G and B
    /// share one draw, quantized to `steps + 1` levels when steps is
    /// nonzero; alpha takes its own draw, always quantized.
    pub(crate) fn random_dual_stepped(
        primary: &mut Self,
        environment: &mut Self,
        pc: &mut Cursor<'_>,
        rng: &mut HsdRng,
        draws: &mut DrawLog,
    ) -> Result<(), Error> {
        primary.materialize();
        environment.materialize();
        let flags = pc.byte()?;
        let steps = i32::from(pc.byte()?);
        let tracks = [flags & 0x10 != 0, flags & 0x20 != 0];
        // retail 0x8039BFC0 fmuls, 0x8039BFCC fctiwz, 0x8039BFEC fdivs.
        let scale = if steps != 0 {
            let draw = draws.draw(rng, 0x8039_BF98);
            fctiwz((steps + 1) as f32 * draw) as f32 / steps as f32
        } else {
            draws.draw(rng, 0x8039_BFF4)
        };
        let mut apply = |channel: usize, offset: f32| {
            for (track, enabled) in [&mut *primary, &mut *environment].into_iter().zip(tracks) {
                if enabled {
                    // retail 0x8039C048 and siblings: fadds, then the
                    // clamp to [0, 255] and fctiwz.
                    let value = f32::from(track.target[channel]) + offset;
                    track.target[channel] = fctiwz(value.clamp(0.0, 255.0)) as u8;
                }
            }
        };
        for channel in 0..3 {
            if flags & (1 << channel) != 0 {
                let delta = i32::from(pc.byte()? as i8) << 1;
                // retail 0x8039C028 / C0F8 / C1C8: fmuls.
                apply(channel, scale * delta as f32);
            }
        }
        if flags & 0x08 != 0 {
            let draw = draws.draw(rng, 0x8039_C270);
            let delta = i32::from(pc.byte()? as i8) << 1;
            let level = fctiwz((steps + 1) as f32 * draw);
            // retail 0x8039C2EC fmuls, 0x8039C2F0 fdivs.
            apply(3, delta as f32 * level as f32 / steps as f32);
        }
        primary.restart();
        environment.restart();
        Ok(())
    }
    pub(crate) fn random_delta(
        &mut self,
        pc: &mut Cursor<'_>,
        rng: &mut HsdRng,
        draws: &mut DrawLog,
        sites: [u32; 4],
    ) -> Result<(), Error> {
        self.materialize();
        for (channel, site) in sites.into_iter().enumerate() {
            let random = draws.draw(rng, site);
            let delta = i32::from(pc.byte()? as i8) << 1;
            // BA retail 0x8039B0C0/0x8039B0C8 and +0x6C per channel:
            // separate fmuls/fadds. BB uses the same unfused instruction shape.
            let offset = delta as f32 * random;
            let value = f32::from(self.target[channel]) + offset;
            self.target[channel] = fctiwz(value.clamp(0.0, 255.0)) as u8;
        }
        self.restart();
        Ok(())
    }
}

/// Shared psdisp fixed-point interpolation for colors and alpha-test references.
pub(crate) fn display_bytes(
    current: &[u8],
    target: &[u8],
    duration: u16,
    remaining: u16,
    output: &mut [u8],
) {
    if duration == 0 {
        output.copy_from_slice(current);
        return;
    }
    let scale = (i32::from(remaining) << 16) / i32::from(duration);
    for ((out, &current), &target) in output.iter_mut().zip(current).zip(target) {
        let delta = scale.wrapping_mul(i32::from(current) - i32::from(target));
        *out = ((i32::from(target) << 16).wrapping_add(delta) >> 16) as u8;
    }
}

#[cfg(test)]
mod display_tests {
    use super::*;
    #[test]
    fn display_color_interpolates_without_changing_endpoints() {
        let track = ColorTrack {
            current: [255, 0, 128, 255],
            target: [0, 255, 64, 0],
            duration: 4,
            remaining: 1,
        };
        let original = track.clone();
        assert_eq!(track.display_color(), [63, 191, 80, 63]);
        assert_eq!(track, original);
    }
}
