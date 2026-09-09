//! HSD's random number generator, from `src/sysdolphin/baselib/random.c`.
//!
//! A 32-bit linear congruential generator with the classic Microsoft
//! constants. Melee's entire nondeterminism flows through this one state
//! word, so a fixed seed plus a fixed input stream makes a match fully
//! reproducible. The retail global `seed` lives at `0x804D5F90` and is
//! initialised to 1.
//!
//! ```c
//! u32 seed = 1;
//! s32 HSD_Rand(void)  { seed = seed * 214013 + 2531011; return seed >> 16; }
//! f32 HSD_Randf(void) { seed = seed * 214013 + 2531011; return (f32)(seed >> 16) / (1 << 16); }
//! s32 HSD_Randi(s32 max) { return max * HSD_Rand() / (1 << 16); }
//! ```

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HsdRng {
    pub seed: u32,
}

impl Default for HsdRng {
    fn default() -> Self {
        Self { seed: 1 }
    }
}

impl HsdRng {
    pub const MULTIPLIER: u32 = 214013;
    pub const INCREMENT: u32 = 2531011;

    pub fn new(seed: u32) -> Self {
        Self { seed }
    }

    #[inline]
    fn step(&mut self) -> u32 {
        self.seed = self
            .seed
            .wrapping_mul(Self::MULTIPLIER)
            .wrapping_add(Self::INCREMENT);
        self.seed >> 16
    }

    /// `HSD_Rand`: returns a value in `0..=0xFFFF`.
    pub fn rand(&mut self) -> i32 {
        self.step() as i32
    }

    /// `HSD_Randf`: returns a value in `[0, 1)` with 16 bits of resolution.
    ///
    /// The division by 65536 is exact in single precision, so this is
    /// bit-exact without any special handling.
    pub fn randf(&mut self) -> f32 {
        (self.step() as f32) / 65536.0
    }

    /// `HSD_Randi`: returns a value in `0..max`.
    ///
    /// Note the C computes `max * rand` in 32-bit signed arithmetic before
    /// dividing. For `max` values Melee actually passes this never
    /// overflows, but we reproduce the wrapping to be safe.
    pub fn randi(&mut self, max: i32) -> i32 {
        max.wrapping_mul(self.rand()) / 65536
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_value_from_default_seed() {
        // 1 * 214013 + 2531011 = 2745024; 2745024 >> 16 = 41
        let mut r = HsdRng::default();
        assert_eq!(r.rand(), 41);
        assert_eq!(r.seed, 2_745_024);
    }

    #[test]
    fn matches_reference_loop() {
        let mut r = HsdRng::new(0xDEAD_BEEF);
        let mut s: u32 = 0xDEAD_BEEF;
        for _ in 0..10_000 {
            s = s.wrapping_mul(214013).wrapping_add(2531011);
            assert_eq!(r.rand() as u32, s >> 16);
        }
    }

    #[test]
    fn randf_is_in_unit_interval_and_exact() {
        let mut r = HsdRng::default();
        for _ in 0..10_000 {
            let before = r.seed;
            let f = r.randf();
            let expected =
                ((before.wrapping_mul(214013).wrapping_add(2531011)) >> 16) as f32 / 65536.0;
            assert_eq!(f.to_bits(), expected.to_bits());
            assert!((0.0..1.0).contains(&f));
        }
    }
}
