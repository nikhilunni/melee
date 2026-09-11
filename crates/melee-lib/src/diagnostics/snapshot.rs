use crate::initial_state::InitialState;
use melee_diff::{Record, RecordSink};
use melee_types::snapshot::{PrefixSink, Snapshot, SnapshotSink};
impl Snapshot for InitialState {
    fn snapshot(&self, sink: &mut dyn SnapshotSink) {
        sink.field("rng.seed", &self.rng.seed);
        for (player, fighter) in self.fighters.iter().enumerate() {
            fighter.snapshot(&mut PrefixSink::new(sink, &format!("p{player}")));
        }
    }
}
pub fn snapshot(state: &InitialState, frame: u64) -> Record {
    let mut sink = RecordSink::new(frame, "frame_end");
    state.snapshot(&mut sink);
    sink.finish()
}
