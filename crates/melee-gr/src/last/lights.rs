//! FD fog/background colour transitions; GX writes become owned RGB data.
use gekko_math::HsdRng;
pub type Rgb = [u8; 3];
const FLICKER_CHANNEL: u8 = 30;
const MIN_FLICKER_FRAMES: i32 = 60;

#[derive(Clone, Debug, PartialEq)]
pub struct ColorFade {
    pub target: [f32; 3],
    pub current: [f32; 3],
    pub remaining: f32,
    pub complete: bool,
}
impl Default for ColorFade {
    fn default() -> Self {
        // C leaves the floats uninitialized until begin(); complete gates them.
        Self {
            target: [0.0; 3],
            current: [0.0; 3],
            remaining: 0.0,
            complete: true,
        }
    }
}
impl ColorFade {
    /// `grLast_8021C40C` (grlast.c), retail 0x8021C40C.
    pub fn begin(&mut self, fog: Rgb, target: Rgb, frames: f32) {
        self.target = target.map(f32::from);
        self.current = fog.map(f32::from);
        self.remaining = frames;
        self.complete = false;
    }
    /// `grLast_8021C500` (grlast.c), retail 0x8021C500.
    pub fn tick(&mut self, fog: &mut Rgb) {
        if self.complete {
            return;
        }
        self.remaining -= 1.0;
        if self.remaining > 0.0 {
            for (current, target) in self.current.iter_mut().zip(self.target) {
                // retail 0x8021C544..0x8021C584: fsubs, fdivs, fadds; no fusion.
                *current += (target - *current) / self.remaining;
            }
            *fog = self.current.map(|v| v as u8);
        } else {
            *fog = self.target.map(|v| v as u8);
            self.complete = true;
        }
    }
}

/// The retry predicate in `grLast_8021B2E8`, retail 0x8021B524..0x8021B564.
/// Preserve the repeated green comparison: retail never compares old blue.
pub fn reject_flicker(candidate: Rgb, old: Rgb) -> bool {
    candidate == [FLICKER_CHANNEL; 3] || (candidate[0] == old[0] && candidate[1] == old[1])
}

/// Phase 15 of `grLast_8021B2E8`, retail 0x8021B4E0.
/// Three draws per candidate, then one duration draw. No artificial retry cap.
pub fn choose_flicker(old: Rgb, rng: &mut HsdRng) -> (Rgb, f32, u32) {
    let mut draws = 0;
    loop {
        let candidate = std::array::from_fn(|_| rng.randi(2) as u8 * FLICKER_CHANNEL);
        draws += 3;
        if !reject_flicker(candidate, old) {
            let frames = rng.randi(MIN_FLICKER_FRAMES) + MIN_FLICKER_FRAMES;
            return (candidate, frames as f32, draws + 1);
        }
    }
}

/// `grLast_8021C640`, retail 0x8021C640: nine environment palette writes.
pub fn dark_palette() -> [Rgb; 9] {
    [[12, 6, 40]; 9]
}
/// `grLast_8021C6AC`, retail 0x8021C6AC. Order: 052C,05D4,0544,058C,
/// 05EC,05A4,055C,05BC,0574 (Ground.c setters).
pub fn bright_palette() -> [Rgb; 9] {
    [
        [140, 180, 190],
        [150, 180, 190],
        [140, 180, 190],
        [110, 120, 140],
        [0, 0, 0],
        [110, 120, 140],
        [115, 160, 145],
        [135, 165, 160],
        [115, 160, 145],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retry_ignores_blue_and_rejects_white() {
        assert!(reject_flicker([0, 30, 30], [0, 30, 0]));
        assert!(reject_flicker([30; 3], [0; 3]));
        assert!(!reject_flicker([30, 0, 0], [0; 3]));
    }
    #[test]
    fn fade_decrements_before_dividing_and_finishes_one_tick_after_reaching_target() {
        let mut fade = ColorFade::default();
        let mut fog = [0; 3];
        fade.begin(fog, [30, 60, 90], 3.0);
        fade.tick(&mut fog);
        assert_eq!(fog, [15, 30, 45]);
        fade.tick(&mut fog);
        assert_eq!(fog, [30, 60, 90]);
        assert!(!fade.complete);
        fade.tick(&mut fog);
        assert!(fade.complete);
        fade.tick(&mut fog);
        assert_eq!(fade.remaining, 0.0);
    }
    #[test]
    fn seed_one_accepts_green_with_four_draws() {
        let mut rng = HsdRng::new(1);
        let (color, duration, draws) = choose_flicker([0; 3], &mut rng);
        // First candidate draws are 41, 51235, 6334: [0,30,0] is accepted.
        assert_eq!(color, [0, 30, 0]);
        assert_eq!(draws, 4);
        assert_eq!(duration, 114.0);
    }
}
