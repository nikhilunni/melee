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
        if self.duration == 0 {
            return self.current;
        }
        let scale = (i32::from(self.remaining) << 16) / i32::from(self.duration);
        std::array::from_fn(|i| {
            let delta = scale.wrapping_mul(i32::from(self.current[i]) - i32::from(self.target[i]));
            ((i32::from(self.target[i]) << 16).wrapping_add(delta) >> 16) as u8
        })
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
