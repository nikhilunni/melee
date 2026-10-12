//! ftKb_Init_MotionStateTable's move ids as the stale-move queue sees them.
use ft_kirby::{SPECIAL_MOVES, SPECIAL_ROW_COUNT};
use melee_types::combat::StaleMove as M;

const FIRST_ACTION: usize = 341;

fn move_of(action: usize) -> Option<M> {
    SPECIAL_MOVES[action - FIRST_ACTION]
}

#[test]
fn every_row_past_the_midair_jumps_has_a_move_id() {
    for action in FIRST_ACTION..FIRST_ACTION + SPECIAL_ROW_COUNT {
        assert_eq!(move_of(action).is_some(), action > 350, "row {action}");
    }
}

#[test]
fn each_copied_special_stales_under_its_own_id() {
    for (action, expected) in [
        (399, M::CopiedMario),
        (406, M::CopiedLink),
        (415, M::CopiedYoshi),
        // The copied Egg Lay's two SpecialNCapture2 rows carry the Inhale's id.
        (416, M::SpecialNeutral),
        (417, M::SpecialNeutral),
        (418, M::CopiedYoshi),
        (423, M::CopiedFox),
        (428, M::CopiedFox),
        (443, M::CopiedKoopa),
        (519, M::CopiedCLink),
        (520, M::CopiedFalco),
        (525, M::CopiedFalco),
        (538, M::CopiedGkoops),
        (543, M::CopiedGkoops),
    ] {
        assert_eq!(move_of(action), Some(expected), "row {action}");
    }
    // 0x16..=0x2F: twenty-six ids over the rows from 399 on.
    let mut distinct: Vec<M> = Vec::new();
    for action in 399..FIRST_ACTION + SPECIAL_ROW_COUNT {
        let id = move_of(action).unwrap();
        if id != M::SpecialNeutral && !distinct.contains(&id) {
            distinct.push(id);
        }
    }
    assert_eq!(distinct.len(), 26);
}
