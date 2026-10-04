//! Who is credited with a fighter's next fall (dmg.x18C4 / x18C8).
//!
//! Every hit names its source (ftColl_8007861C); a hit that leaves a
//! grounded fighter standing clears it (ftCommon_800804FC), and so does a
//! short countdown once the fighter is back in a neutral grounded motion
//! (Fighter_ChangeMotionState, Fighter_8006A360). A stock lost while a
//! source stands is that player's KO (ftCo_800D34E0 ->
//! Player_UpdateKOsBySlot), which the CPU reads back as its KO total
//! (gm_8016C75C).
use melee_coll::damage_log::HitCredit;
use melee_types::{CommonMotionState as S, GroundOrAir};

/// dmg.x18C4 (the source's player slot; retail's 6 is `None`) and x18C8
/// (the frames until it lapses; retail's -1 is `None`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KoSource {
    pub player: Option<u8>,
    pub countdown: Option<u32>,
}

impl KoSource {
    /// ftColl_8007861C (0x8007861C): a hit's source replaces the last one
    /// (0x80078680) and stops the countdown (0x80078684, 0x800786AC). A hit
    /// with no fighter behind it clears the source (0x800786B8) unless it
    /// is one retail keeps the earlier source through (its last argument).
    pub fn record(&mut self, credit: HitCredit) {
        match credit {
            HitCredit::Player(player) => self.player = Some(player),
            HitCredit::Nobody => self.player = None,
            HitCredit::Unchanged => {}
        }
        self.countdown = None;
    }

    /// The tail of ftColl_80078754 (0x80078790) and fighter.c:311: no
    /// source, not counting.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// ftCommon_800804FC (0x800804FC): a grounded fighter that took a hit
    /// without being launched owes its next fall to no one.
    pub fn clear_if_grounded(&mut self, ground_or_air: GroundOrAir) {
        if ground_or_air == GroundOrAir::Ground {
            self.clear();
        }
    }

    /// Fighter_ChangeMotionState (fighter.c:1183-1192): entering a motion
    /// with MotionState.x9_b1 on the ground starts the countdown at PlCo
    /// +814 frames (one when that is not positive), unless it is running.
    pub fn start_countdown(&mut self, frames: i32) {
        if self.countdown.is_none() {
            self.countdown = Some(if frames > 0 { frames as u32 } else { 1 });
        }
    }

    /// Fighter_8006A360 (fighter.c:1455-1462): the countdown; at zero the
    /// source lapses.
    pub fn tick(&mut self) {
        match self.countdown {
            None => {}
            Some(frames) if frames > 1 => self.countdown = Some(frames - 1),
            // `start_countdown` never stores zero, so this is the last frame.
            Some(_) => self.clear(),
        }
    }
}

/// MotionState.x9_b1 (bit 22 of the word at +8): the 28 common motions
/// that start the countdown, read from the retail table
/// (ftData_MotionStateList, 0x803C2800). The character tables' rows are
/// not modelled: in the decomp only Mr. Game & Watch's, Donkey Kong's,
/// Kirby's and Sandbag's spell the bit, none of them ported.
pub fn starts_countdown(state: S) -> bool {
    matches!(
        state,
        S::Wait
            | S::WalkSlow
            | S::WalkMiddle
            | S::WalkFast
            | S::Turn
            | S::Dash
            | S::KneeBend
            | S::SquatWait
            | S::Landing
            | S::LandingFallSpecial
            | S::LandingAirN
            | S::LandingAirF
            | S::LandingAirB
            | S::LandingAirHi
            | S::LandingAirLw
            | S::GuardOn
            | S::CaptureWaitHi
            | S::CaptureWaitLw
            | S::ShoulderedWait
            | S::CaptureWaitKoopa
            | S::CaptureDamageKoopaAir
            | S::CaptureWaitKirby
            | S::BarrelWait
            | S::HammerWait
            | S::HammerLanding
            | S::CaptureWaitMasterHand
            | S::CaptureWaitCrazyHand
            | S::Barrel
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hit_names_its_source_and_stops_the_countdown() {
        let cases = [
            (HitCredit::Player(2), Some(2)),
            (HitCredit::Nobody, None),
            (HitCredit::Unchanged, Some(1)),
        ];
        for (credit, player) in cases {
            let mut source = KoSource {
                player: Some(1),
                countdown: Some(40),
            };
            source.record(credit);
            assert_eq!(
                source,
                KoSource {
                    player,
                    countdown: None
                },
                "{credit:?}"
            );
        }
    }

    #[test]
    fn the_countdown_lapses_the_source_on_its_last_frame() {
        let mut source = KoSource {
            player: Some(3),
            countdown: None,
        };
        source.start_countdown(2);
        // A second neutral motion does not restart a running countdown.
        source.tick();
        source.start_countdown(60);
        assert_eq!(source.countdown, Some(1));
        assert_eq!(source.player, Some(3));
        source.tick();
        assert_eq!(source, KoSource::default());
    }

    #[test]
    fn a_non_positive_duration_counts_one_frame() {
        let mut source = KoSource {
            player: Some(0),
            countdown: None,
        };
        source.start_countdown(0);
        assert_eq!(source.countdown, Some(1));
    }
}
