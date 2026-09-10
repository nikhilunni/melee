//! VI-to-scheduler alignment using only animation-frame and position bits.
//! Matrix values never choose a candidate; repeated VI samples may reuse a
//! tick, but alignment cannot travel backwards through an animation loop.
use std::collections::{BTreeMap, BTreeSet};

pub type PoseKey = [u32; 4];

pub struct TickPose {
    pub tick: usize,
    pub matrices: Vec<[u32; 12]>,
}

#[derive(Default)]
pub struct PoseTimeline {
    poses: BTreeMap<PoseKey, Vec<TickPose>>,
    last_inserted: Option<usize>,
    last_aligned: usize,
    covered_ticks: BTreeSet<usize>,
}

impl PoseTimeline {
    pub fn insert(&mut self, tick: usize, key: PoseKey, matrices: Vec<[u32; 12]>) {
        assert!(
            self.last_inserted.is_none_or(|last| tick > last),
            "poses must be recorded in tick order"
        );
        self.last_inserted = Some(tick);
        self.poses
            .entry(key)
            .or_default()
            .push(TickPose { tick, matrices });
    }

    pub fn is_empty(&self) -> bool {
        self.poses.is_empty()
    }

    /// Earliest chronological key match, including repeated observations of
    /// the same tick. Unmatched frames do not advance the cursor.
    pub fn align(&mut self, key: &PoseKey) -> Option<&TickPose> {
        let pose = self
            .poses
            .get(key)?
            .iter()
            .find(|pose| pose.tick >= self.last_aligned)?;
        self.last_aligned = pose.tick;
        Some(pose)
    }

    /// Count only distinct aligned ticks which compared non-dirty words.
    /// Repeated VI samples and all-dirty frames cannot satisfy a coverage gate.
    pub fn record_coverage(&mut self, tick: usize, words: usize) {
        assert_eq!(
            tick, self.last_aligned,
            "coverage must belong to the aligned pose"
        );
        if words > 0 {
            self.covered_ticks.insert(tick);
        }
    }

    pub fn covered_ticks(&self) -> usize {
        self.covered_ticks.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chronological_alignment_handles_loops_repeats_and_unmatched_frames() {
        let mut poses = PoseTimeline::default();
        let a = [1, 0, 0, 0];
        let b = [2, 0, 0, 0];
        poses.insert(0, a, vec![[10; 12]]);
        poses.insert(1, b, vec![[20; 12]]);
        poses.insert(2, a, vec![[30; 12]]);
        assert_eq!(poses.align(&a).unwrap().matrices, vec![[10; 12]]);
        assert_eq!(poses.align(&a).unwrap().tick, 0);
        assert_eq!(poses.align(&b).unwrap().tick, 1);
        assert!(poses.align(&[99, 0, 0, 0]).is_none());
        assert_eq!(poses.align(&a).unwrap().tick, 2);
        assert!(poses.align(&b).is_none());
    }

    #[test]
    fn signed_zero_is_not_an_alignment_match_and_repeats_do_not_add_coverage() {
        let mut poses = PoseTimeline::default();
        let key = [1, 0, 0, 0];
        poses.insert(0, key, vec![[0; 12]]);
        assert!(poses.align(&[1, (-0.0_f32).to_bits(), 0, 0]).is_none());
        poses.align(&key).unwrap();
        poses.record_coverage(0, 0);
        assert_eq!(poses.covered_ticks(), 0);
        poses.record_coverage(0, 12);
        poses.align(&key).unwrap();
        poses.record_coverage(0, 12);
        assert_eq!(poses.covered_ticks(), 1);
    }
}
