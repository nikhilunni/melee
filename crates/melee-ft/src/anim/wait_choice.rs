//! Idle submotion selection (`ft/ftwaitanim.c`).
use gekko_math::rng::HsdRng;

/// One `WaitStruct` in `ftData.x24`: motion id and cumulative weight increment.
/// The on-disc sentinel is (-1, -1); callers may omit it from the slice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WaitEntry {
    pub motion: i32,
    pub weight: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WaitChoice {
    pub motion: i32,
    pub draws: usize,
}

/// `ftCo_8008A7A8` (0x8008A7A8); the draw is `bl HSD_Randi` at
/// 0x8008A8BC. Roll 1..=100 against cumulative weights. Only Wait1 (2)
/// and its item variant (31) may repeat. Every rejected repeat draws again.
/// Invalid probability tables assert in retail; no fallback motion or retry cap.
pub fn choose_wait_animation(
    rng: &mut HsdRng,
    table: &[WaitEntry],
    current_motion: i32,
) -> WaitChoice {
    let mut draws = 0;
    loop {
        let roll = rng.randi(100) + 1;
        draws += 1;
        let mut cumulative: i32 = 0;
        let motion = table
            .iter()
            .take_while(|entry| entry.motion != -1)
            .find_map(|entry| {
                cumulative = cumulative.wrapping_add(entry.weight);
                (roll <= cumulative).then_some(entry.motion)
            })
            .expect("wait anim data illegal: weights do not cover the draw");
        if matches!(current_motion, 2 | 31) || current_motion != motion {
            return WaitChoice { motion, draws };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const FOX: [WaitEntry; 3] = [
        WaitEntry {
            motion: 2,
            weight: 70,
        },
        WaitEntry {
            motion: 3,
            weight: 30,
        },
        WaitEntry {
            motion: -1,
            weight: -1,
        },
    ];

    #[test]
    fn fixed_seed_choices_and_draw_counts() {
        // Hand LCG calculations: next = (214013 * seed + 2531011) mod 2^32;
        // roll = floor(100 * (next >> 16) / 65536) + 1.
        // seed 1: next=2745024, upper=41, roll=1.
        // DEAD_BEEF: C7A1_DDF6, upper=51105, roll=78 -> Wait2.
        // Repeating Wait2 rejects 78; next=4218_5CE1, upper=16920,
        // roll=26 -> Wait1. No third draw is allowed.
        for (seed, current, motion, draws, after) in [
            (0, 2, 2, 1, 0x0026_9ec3),
            (1, 2, 2, 1, 0x0029_e2c0),
            (12345, 2, 2, 1, 0x9da0_3218),
            (0xdead_beef, 2, 3, 1, 0xc7a1_ddf6),
            (0xdead_beef, 3, 2, 2, 0x4218_5ce1),
            (20000, 3, 2, 2, 0x4567_4d9a), // rolls 100, 28
            (0xffff_ffff, 3, 2, 1, 0x0023_5ac6),
        ] {
            let mut rng = HsdRng::new(seed);
            assert_eq!(
                choose_wait_animation(&mut rng, &FOX, current),
                WaitChoice { motion, draws },
                "seed {seed:08x}, current {current}"
            );
            assert_eq!(rng.seed, after);
        }
    }

    #[test]
    fn cumulative_boundary_and_item_wait_repeat() {
        // Invert the LCG to obtain upper words at the 70/71 boundary.
        // floor(100*45875/65536)+1=70; upper=45876 yields 71.
        for (upper, expected) in [(45875u32, 2), (45876, 3)] {
            let after = upper << 16;
            let before = after.wrapping_sub(2531011).wrapping_mul(0xb9b3_3155);
            let mut rng = HsdRng::new(before);
            assert_eq!(choose_wait_animation(&mut rng, &FOX, 2).motion, expected);
            assert_eq!(rng.seed, after);
        }
        let mut rng = HsdRng::new(1);
        assert_eq!(
            choose_wait_animation(
                &mut rng,
                &[WaitEntry {
                    motion: 31,
                    weight: 100
                }],
                31
            ),
            WaitChoice {
                motion: 31,
                draws: 1
            }
        );
    }
}
