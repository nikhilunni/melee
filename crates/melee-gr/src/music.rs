//! Match-start music selection, Ground_801C24F8 (801C24F8), stage.c's
//! Stage_80225074 (80225074). Audio output still advances the shared RNG.
use gekko_math::HsdRng;

/// Ground_801C24F8: supported normal music-selection rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MusicRule {
    /// Rule 0: use the primary track unless music is explicitly forced.
    Primary,
    /// Rules 3 and 4: draw only when this character's unlock bit
    /// (gm_IsCKindUnlocked: lbl_803B78C8's index) is set.
    CharacterUnlocked(u8),
    /// Rule 6: draw only when every character is unlocked.
    AllCharactersUnlocked,
}
/// The unlockable characters' bits in the save data's mask
/// (gmMainLib_GetUnlockedCharactersBitmaskPtr).
pub const ALL_UNLOCKABLE_CHARACTERS: u16 = (1 << 11) - 1;
/// lbl_803B78C8: Marth's unlock index (rule 3).
pub const MARTH_UNLOCK_BIT: u8 = 2;
/// lbl_803B78C8: Young Link's unlock index (rule 4).
pub const YOUNG_LINK_UNLOCK_BIT: u8 = 6;

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
    /// ground.c:1398-1406; gm_80164ABC checks the eleven unlock bits,
    /// gm_IsCKindUnlocked one of them. `unlocks` is the save data's mask.
    /// Retail 801C26AC calls HSD_Randi(100) before comparing the threshold.
    pub fn select(self, unlocks: u16, rng: &mut HsdRng) -> i32 {
        let unlocked = match self.rule {
            MusicRule::Primary => false,
            MusicRule::CharacterUnlocked(bit) => unlocks & (1 << bit) != 0,
            MusicRule::AllCharactersUnlocked => {
                unlocks & ALL_UNLOCKABLE_CHARACTERS == ALL_UNLOCKABLE_CHARACTERS
            }
        };
        if unlocked
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
        assert_eq!(music.select(0, &mut rng), 1);
        assert_eq!(rng.seed, 17);
        let mut expected = rng;
        expected.randi(100);
        assert_eq!(music.select(ALL_UNLOCKABLE_CHARACTERS, &mut rng), 1);
        assert_eq!(rng.seed, expected.seed);
    }
}

#[cfg(test)]
mod primary_tests {
    use super::*;
    #[test]
    fn story_primary_music_never_draws_even_when_unlocked() {
        for unlocked in [0, ALL_UNLOCKABLE_CHARACTERS] {
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

#[cfg(test)]
mod character_tests {
    use super::*;
    #[test]
    fn a_character_rule_draws_only_with_that_character_unlocked() {
        let music = MusicParameters {
            rule: MusicRule::CharacterUnlocked(MARTH_UNLOCK_BIT),
            primary: 1,
            alternate: 2,
            alternate_chance: 100,
            sudden_death: 3,
        };
        let mut rng = HsdRng::new(17);
        assert_eq!(music.select(!(1 << MARTH_UNLOCK_BIT), &mut rng), 1);
        assert_eq!(rng.seed, 17);
        assert_eq!(music.select(1 << MARTH_UNLOCK_BIT, &mut rng), 2);
        assert_ne!(rng.seed, 17);
    }
}
