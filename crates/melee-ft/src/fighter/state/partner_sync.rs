//! MotionState.x9_b0 (bit 23 of the packed move word), which
//! Fighter_ChangeMotionState copies to Fighter.x2225_b3 (fighter.c:1200).
//! Only the Ice Climbers' partner reads it: in such a motion Nana counts as
//! in step with her player (ftCo_800B0CA8) and her position is drawn toward
//! the one she replays (ftCo_800B0AF4).
use super::{ActionId, COMMON_COUNT};
use crate::fighter::Fighter;

/// ftData_MotionStateList (0x803C2800, 341 rows of 0x20 bytes): the rows
/// whose x8 word has bit 23 set, one bit per row, read from the retail DOL.
const COMMON_ROWS: [u64; 6] = [
    0xFFFF_F23F_FFFF_C000,
    0xFF0F_F0FF_D000_07FF,
    0x007C_3FFF_CFFF_FFFF,
    0x0010_0000_0000_0000,
    0x01F8_0100_0000_0000,
    0x0000_0000_0000_0000,
];

/// Whether retail's table row for `action` sets x9_b0; character rows come
/// from the character's `SPECIAL_PARTNER_SYNC`.
pub fn partner_sync(fighter: &Fighter, action: ActionId) -> bool {
    let index = usize::from(action.0);
    if index < COMMON_COUNT {
        COMMON_ROWS[index / 64] & (1 << (index % 64)) != 0
    } else {
        fighter
            .character
            .table()
            .special_partner_sync
            .get(index - COMMON_COUNT)
            .copied()
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn common_rows_match_the_retail_table() {
        let set = |index: usize| COMMON_ROWS[index / 64] & (1 << (index % 64)) != 0;
        // Wait and the walk/dash/run family (14..37) are; Entry (322) is not.
        assert!((14..=37).all(set));
        assert!(!set(13) && !set(38) && !set(322));
        // Landing (42) is not, its special variants (44..) are.
        assert!(!set(42) && set(44));
        assert_eq!(COMMON_ROWS.iter().map(|w| w.count_ones()).sum::<u32>(), 140);
    }
}
