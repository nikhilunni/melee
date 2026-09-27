//! Match-start music selection, Ground_801C24F8 (801C24F8), stage.c's
//! Stage_80225074 (80225074). Audio output still advances the shared RNG.
use gekko_math::HsdRng;

/// Ground_801C24F8: supported normal music-selection rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MusicRule {
    /// Rule 0: use the primary track unless music is explicitly forced.
    Primary,
    /// Rule 6: draw only when every character is unlocked.
    AllCharactersUnlocked,
}

/// StageParam music rows are data. FD uses the all-characters unlock rule.
#[derive(Clone, Copy, Debug)]
pub struct MusicParameters {
    pub rule: MusicRule,
    pub primary: i32,
    pub alternate: i32,
    pub alternate_chance: i16,
    /// StageParam xC: the Sudden Death track.
    pub sudden_death: i32,
}
/// Ground_801C24F8's track id that asks for a random character track.
const RANDOM_CHARACTER_TRACK: i32 = -2;
impl MusicParameters {
    /// Stage_80225074 under the Sudden Death rule (gm_8016B238): arg 0x11,
    /// so no unlock-rule draw, and StageParam xC. (Arg 0x12, after an
    /// alternate Versus track, would pick x10; a cold start has none.)
    pub fn select_sudden_death(self) -> i32 {
        assert_ne!(
            self.sudden_death, RANDOM_CHARACTER_TRACK,
            "Ground_801C24F8: random Sudden Death track"
        );
        self.sudden_death
    }
    /// ground.c:1398-1406; gm_80164ABC checks the eleven unlock bits.
    /// Retail 801C26AC calls HSD_Randi(100) before comparing the threshold.
    pub fn select(self, all_characters_unlocked: bool, rng: &mut HsdRng) -> i32 {
        if self.rule == MusicRule::AllCharactersUnlocked
            && all_characters_unlocked
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
            rule: MusicRule::AllCharactersUnlocked,
            primary: 1,
            alternate: 2,
            alternate_chance: 0,
            sudden_death: 3,
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

#[cfg(test)]
mod primary_tests {
    use super::*;
    #[test]
    fn story_primary_music_never_draws_even_when_unlocked() {
        for unlocked in [false, true] {
            let music = MusicParameters {
                rule: MusicRule::Primary,
                primary: 7,
                alternate: -1,
                alternate_chance: 100,
                sudden_death: 8,
            };
            let mut rng = HsdRng::new(17);
            assert_eq!(music.select(unlocked, &mut rng), 7);
            assert_eq!(rng.seed, 17);
        }
    }
}
