//! Bridge from `melee_types::snapshot` to the trace format.
//!
//! `melee-types` is a layer-0 crate and cannot know about JSON or the
//! `approx` decimal that [`Value`] carries for humans. This module converts
//! its [`SnapValue`] into [`Value`] and offers two ways to turn a `Snapshot`
//! walk into a [`Record`]:
//!
//! - [`Record::from_sink`] takes the entries a `Vec<(String, SnapValue)>`
//!   sink collected.
//! - [`RecordSink`] implements `SnapshotSink` directly and builds the
//!   `Record` as values arrive, so the simulator can do
//!   `fighter.snapshot(&mut PrefixSink::new(&mut sink, "p0"))` per player and
//!   then `sink.finish()`.

use crate::{Record, Value};
use melee_types::snapshot::{SnapValue, SnapshotSink};
use std::collections::BTreeMap;

impl From<SnapValue> for Value {
    fn from(v: SnapValue) -> Value {
        match v {
            SnapValue::I64(i) => Value::Int(i),
            SnapValue::U64(u) => Value::UInt(u),
            SnapValue::F32(bits) => Value::f32(f32::from_bits(bits)),
            SnapValue::F64(bits) => Value::f64(f64::from_bits(bits)),
            SnapValue::Str(s) => Value::Str(s),
            SnapValue::Null => Value::Null,
        }
    }
}

impl Record {
    /// Build a record from the `(path, value)` pairs a sink collected.
    ///
    /// Later entries win when a path repeats, mirroring how the decoder's
    /// dict assignment behaves; a `Snapshot` impl should never repeat a path.
    pub fn from_sink<I, P>(frame: u64, phase: impl Into<String>, entries: I) -> Record
    where
        I: IntoIterator<Item = (P, SnapValue)>,
        P: Into<String>,
    {
        Record {
            frame,
            phase: phase.into(),
            state: entries
                .into_iter()
                .map(|(p, v)| (p.into(), Value::from(v)))
                .collect(),
        }
    }
}

/// A `SnapshotSink` that accumulates straight into a [`Record`].
#[derive(Debug, Clone)]
pub struct RecordSink {
    frame: u64,
    phase: String,
    state: BTreeMap<String, Value>,
}

impl RecordSink {
    pub fn new(frame: u64, phase: impl Into<String>) -> Self {
        RecordSink {
            frame,
            phase: phase.into(),
            state: BTreeMap::new(),
        }
    }

    /// Number of paths recorded so far.
    pub fn len(&self) -> usize {
        self.state.len()
    }

    pub fn is_empty(&self) -> bool {
        self.state.is_empty()
    }

    pub fn finish(self) -> Record {
        Record {
            frame: self.frame,
            phase: self.phase,
            state: self.state,
        }
    }
}

impl SnapshotSink for RecordSink {
    fn put(&mut self, path: &str, v: SnapValue) {
        self.state.insert(path.to_owned(), Value::from(v));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hsd_types::Vec3;
    use melee_types::snapshot::{PrefixSink, Snapshot};

    #[test]
    fn value_conversion_keeps_bits_and_kinds() {
        assert_eq!(Value::from(SnapValue::I64(-3)), Value::Int(-3));
        assert_eq!(Value::from(SnapValue::U64(7)), Value::UInt(7));
        assert_eq!(Value::from(SnapValue::f32(1.5)), Value::f32(1.5));
        assert_eq!(Value::from(SnapValue::f64(-0.0)), Value::f64(-0.0));
        assert_eq!(
            Value::from(SnapValue::Str("a".into())),
            Value::Str("a".into())
        );
        assert_eq!(Value::from(SnapValue::Null), Value::Null);
        // A NaN payload passes through untouched.
        match Value::from(SnapValue::F32(0x7FC0_1234)) {
            Value::F32 { bits, .. } => assert_eq!(bits, 0x7FC0_1234),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn from_sink_builds_record() {
        let mut sink: Vec<(String, SnapValue)> = Vec::new();
        Vec3::new(1.0, 2.0, 3.0).snapshot(&mut PrefixSink::new(&mut sink, "p0.cur_pos"));
        7u8.snapshot(&mut PrefixSink::new(&mut sink, "p0.jumps_used"));
        let rec = Record::from_sink(12, "frame_end", sink);
        assert_eq!(rec.frame, 12);
        assert_eq!(rec.phase, "frame_end");
        assert_eq!(rec.state.len(), 4);
        assert_eq!(rec.state["p0.cur_pos.y"], Value::f32(2.0));
        assert_eq!(rec.state["p0.jumps_used"], Value::UInt(7));
    }

    #[test]
    fn record_sink_matches_from_sink_and_diff() {
        let mut direct = RecordSink::new(3, "frame_end");
        let mut collected: Vec<(String, SnapValue)> = Vec::new();
        for (i, v) in [1.0f32, -2.5].iter().enumerate() {
            let prefix = format!("p{i}.percent");
            v.snapshot(&mut PrefixSink::new(&mut direct, &prefix));
            v.snapshot(&mut PrefixSink::new(&mut collected, &prefix));
        }
        assert_eq!(direct.len(), 2);
        let a = direct.finish();
        let b = Record::from_sink(3, "frame_end", collected);
        assert_eq!(a, b);
        assert_eq!(
            crate::first_divergence(std::slice::from_ref(&a), &[b]),
            None
        );

        // And the record round-trips through the JSONL format the oracle writes.
        let s = serde_json::to_string(&a).unwrap();
        let back: Record = serde_json::from_str(&s).unwrap();
        assert_eq!(a, back);
    }
}
