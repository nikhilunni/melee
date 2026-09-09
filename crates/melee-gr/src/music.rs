//! Match-start music selection, Ground_801C24F8 (801C24F8), stage.c's
//! Stage_80225074 (80225074). Audio output still advances the shared RNG.
use gekko_math::HsdRng;

/// StageParam music rows are data. FD uses the all-characters unlock rule.
#[derive(Clone, Copy, Debug)]
pub struct MusicParameters {
    pub primary: i32,
    pub alternate: i32,
    pub alternate_chance: i16,
}
impl MusicParameters {
    /// ground.c:1398-1406; gm_80164ABC checks the eleven unlock bits.
    /// Retail 801C26AC calls HSD_Randi(100) before comparing the threshold.
    pub fn select(self, all_characters_unlocked: bool, rng: &mut HsdRng) -> i32 {
        if all_characters_unlocked
            && i32::from(self.alternate_chance) > rng.randi(100)
            && self.alternate != -1
        {
            self.alternate
        } else {
            self.primary
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn locked_music_does_not_draw_and_unlocked_music_draws_even_at_zero_chance() {
        let music = MusicParameters {
            primary: 1,
            alternate: 2,
            alternate_chance: 0,
        };
        let mut rng = HsdRng::new(17);
        assert_eq!(music.select(false, &mut rng), 1);
        assert_eq!(rng.seed, 17);
        let mut expected = rng;
        expected.randi(100);
        assert_eq!(music.select(true, &mut rng), 1);
        assert_eq!(rng.seed, expected.seed);
    }
}
