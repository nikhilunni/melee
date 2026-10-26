//! The map's island lists as a savestate holds them.
use super::{saved_pose::SavedPose, word};
use anyhow::{ensure, Result};

/// mpIsland_80458E88: +0 floors, +4 ceilings, +8 and +C the last static
/// node of each.
const ISLANDS: u32 = 0x8045_8E88;
/// mp_UnkStruct0 +24: the island's first line.
const FIRST_LINE: u32 = 0x24;

/// The static island lists in the order retail saved them. A stage's
/// joint restores (`restore_collision`) each end in mpIsland_8005B334,
/// which reverses the static lists, though retail's saved lists already
/// include those updates. The CPU walks the lists in order: ftCo_800A3908
/// and ftCo_800A4038 take the first island that qualifies (Brinstar,
/// `corpus_v3_w27s_start_brin_fox_cpu9fox4_edb2b114a_p2` tick 723).
pub(super) fn restore_order(saved: &SavedPose, map: &mut melee_mp::CollMap) -> Result<()> {
    let list = |head_offset: u32, tail_offset: u32| -> Vec<i16> {
        let tail = word(saved.bytes(ISLANDS + tail_offset, 4), 0);
        let mut lines = Vec::new();
        let mut node = word(saved.bytes(ISLANDS + head_offset, 4), 0);
        while tail != 0 && node != 0 {
            let raw = saved.bytes(node + FIRST_LINE, 2);
            lines.push(i16::from_be_bytes([raw[0], raw[1]]));
            if node == tail {
                break;
            }
            node = word(saved.bytes(node, 4), 0);
        }
        lines
    };
    let (floors, ceilings) = (list(0, 8), list(4, 0xC));
    ensure!(
        map.restore_static_island_order(&floors, &ceilings),
        "the saved islands (floors {floors:?}, ceilings {ceilings:?}) are not the map's"
    );
    Ok(())
}
