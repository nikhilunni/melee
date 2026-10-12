//! A fighter's snapshot as plain values, for tools that compare every tick.
//!
//! [`super::snapshot`] builds a `Record`: a map keyed by path, with a
//! `String` per key. A tool that reads every tick of a long match (the
//! replay runner) wants the same values without the map. A fighter's
//! `Snapshot` walk emits the same paths in the same order every time, so
//! the values alone, in emission order, are the snapshot; the paths are
//! read once ([`fighter_paths`]).
use crate::initial_state::InitialState;
use melee_types::snapshot::{SnapValue, Snapshot, SnapshotSink};

/// The most values one fighter's snapshot may emit
/// (`harness/schema/fighter.yaml` has 24 scalars).
const CAPACITY: usize = 32;

/// One fighter's snapshot values in emission order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FighterValues {
    len: u8,
    kinds: [Kind; CAPACITY],
    bits: [u64; CAPACITY],
}

/// A [`SnapValue`]'s kind; the fighter schema has no text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Null,
    I64,
    U64,
    F32,
    F64,
}

impl FighterValues {
    const EMPTY: Self = Self {
        len: 0,
        kinds: [Kind::Null; CAPACITY],
        bits: [0; CAPACITY],
    };

    pub(crate) fn of(fighter: &dyn Snapshot) -> Self {
        let mut values = Self::EMPTY;
        fighter.snapshot(&mut values);
        values
    }

    pub fn len(&self) -> usize {
        usize::from(self.len)
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The value emitted at `index` ([`fighter_paths`] names it).
    pub fn get(&self, index: usize) -> Option<SnapValue> {
        if index >= self.len() {
            return None;
        }
        let bits = self.bits[index];
        Some(match self.kinds[index] {
            Kind::Null => SnapValue::Null,
            Kind::I64 => SnapValue::I64(bits as i64),
            Kind::U64 => SnapValue::U64(bits),
            Kind::F32 => SnapValue::F32(bits as u32),
            Kind::F64 => SnapValue::F64(bits),
        })
    }
}

impl SnapshotSink for FighterValues {
    fn put(&mut self, _path: &str, value: SnapValue) {
        let index = self.len();
        assert!(
            index < CAPACITY,
            "a fighter snapshot emits more than {CAPACITY} values: raise CAPACITY"
        );
        (self.kinds[index], self.bits[index]) = match value {
            SnapValue::Null => (Kind::Null, 0),
            SnapValue::I64(v) => (Kind::I64, v as u64),
            SnapValue::U64(v) => (Kind::U64, v),
            SnapValue::F32(v) => (Kind::F32, u64::from(v)),
            SnapValue::F64(v) => (Kind::F64, v),
            SnapValue::Str(_) => unreachable!("the fighter schema has no text values"),
        };
        self.len += 1;
    }
}

/// The path of each value in a [`FighterValues`], without a player prefix
/// (`motion_id`, `cur_pos.x`, ...).
pub(crate) fn fighter_paths(state: &InitialState) -> Vec<String> {
    let mut entries: Vec<(String, SnapValue)> = Vec::new();
    if let Some(fighter) = state.fighters.first() {
        fighter.snapshot(&mut entries);
    }
    entries.into_iter().map(|(path, _)| path).collect()
}
