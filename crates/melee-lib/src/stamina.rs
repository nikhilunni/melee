//! A Stamina match's scene object (gmstamina.c): a fighter knocked out at
//! 0 HP slows the game to 0.4 for 100 ticks, and 100 ticks after at most
//! one player (team) is left the match ends.

/// gm_804975F8 (gmstamina.c), the state fn_801B9850's GObj proc keeps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct StaminaRules {
    /// done_counter: ticks since at most one player (team) was left.
    done: u16,
    /// eliminated: by slot; an empty slot counts as eliminated.
    eliminated: [bool; 4],
    /// slomo_counter: by slot, the ticks of the slowdown a knockout began.
    slow_motion: [u16; 4],
}

/// gm_801B97C4's gm_SetGameSpeed(0.4F).
const KNOCKOUT_SPEED: f32 = 0.4;
/// fn_801B96E8's limits for both counters.
const COUNT: u16 = 100;
/// gm_804975F8's address in the boundary savestate.
const RULES_ADDRESS: u32 = 0x8049_75F8;

impl StaminaRules {
    /// fn_801B9850 (the match start, `on_match_start`): an empty slot is out.
    pub(crate) fn starting(occupied: [bool; 4]) -> Self {
        Self {
            done: 0,
            eliminated: occupied.map(|occupied| !occupied),
            slow_motion: [0; 4],
        }
    }

    /// gm_804975F8 as a savestate holds it (big-endian, 0x1C bytes): the
    /// u16 counter, the four `bool`s (MSL's int) from +4, the four u16
    /// slow-motion counters from +0x14.
    pub(crate) fn saved(bytes: &[u8]) -> Self {
        let half = |at: usize| u16::from_be_bytes([bytes[at], bytes[at + 1]]);
        let word = |at: usize| {
            u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
        };
        Self {
            done: half(0),
            eliminated: std::array::from_fn(|slot| word(4 + 4 * slot) != 0),
            slow_motion: std::array::from_fn(|slot| half(0x14 + 2 * slot)),
        }
    }

    pub(crate) const SAVED_RANGE: (u32, usize) = (RULES_ADDRESS, 0x1C);

    /// gm_801B97C4 (gm_80167470 in a Stamina match): `slot`'s own fighter
    /// (not a partner, x221F_b4) was knocked out. Returns the game speed
    /// to request, if any.
    pub(crate) fn knocked_out(&mut self, slot: usize) -> Option<f32> {
        if slot >= 4 {
            return None;
        }
        let first = !self.eliminated[slot];
        self.eliminated[slot] = true;
        first.then(|| {
            self.slow_motion[slot] = 1;
            KNOCKOUT_SPEED
        })
    }

    /// fn_801B96E8, the proc (s_link 0x15). `players` gives each slot's
    /// presence, stocks and team. Returns the game speed to request and
    /// whether the match ends (gm_8016B33C(5), gm_8016B328).
    pub(crate) fn tick(&mut self, players: [Option<(u8, Option<u8>)>; 4]) -> (Option<f32>, bool) {
        let mut speed = None;
        for (slot, player) in players.iter().enumerate() {
            if player.is_none() {
                continue;
            }
            let counter = &mut self.slow_motion[slot];
            if *counter > 0 && *counter < COUNT {
                *counter += 1;
            } else if *counter == COUNT {
                *counter = 0;
                speed = Some(1.0);
            }
        }
        let mut ends = false;
        if self.sides_left(players) <= 1 {
            self.done += 1;
            if self.done > COUNT {
                speed = Some(1.0);
                ends = true;
            }
        }
        (speed, ends)
    }

    /// gm_801B9600: the players (teams) not eliminated; a player out of
    /// stocks is eliminated here too.
    fn sides_left(&mut self, players: [Option<(u8, Option<u8>)>; 4]) -> usize {
        let mut count = 0;
        for slot in 0..4 {
            if let Some((stocks, _)) = players[slot] {
                if stocks == 0 {
                    self.eliminated[slot] = true;
                }
            }
            if self.eliminated[slot] {
                continue;
            }
            let team = players[slot].and_then(|(_, team)| team);
            // gmMainLib_804D3EE0's is_teams: a team counts once, at its
            // first member left in slot order.
            let counted_before = team.is_some()
                && (0..slot).any(|earlier| {
                    !self.eliminated[earlier] && players[earlier].and_then(|(_, t)| t) == team
                });
            if !counted_before {
                count += 1;
            }
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_knockout_slows_for_a_hundred_ticks_and_ends_the_match_after_a_hundred_and_one() {
        let mut rules = StaminaRules::starting([true, true, false, false]);
        let players = [Some((1, None)), Some((1, None)), None, None];
        assert_eq!(rules.knocked_out(1), Some(KNOCKOUT_SPEED));
        assert_eq!(rules.knocked_out(1), None);
        let mut events = Vec::new();
        for tick in 0..=100 {
            let (speed, ends) = rules.tick(players);
            if speed.is_some() || ends {
                events.push((tick, speed, ends));
            }
        }
        assert_eq!(events, [(99, Some(1.0), false), (100, Some(1.0), true)]);
    }
}
