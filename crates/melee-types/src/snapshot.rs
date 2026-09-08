//! `Snapshot`: flatten game state into dotted paths for oracle comparison.
//!
//! The Dolphin side (`harness/decode.py`) reads raw struct bytes and emits a
//! flat map keyed by dotted path, e.g. `p0.cur_pos.x`, `p0.cpu.buttons`,
//! driven by `harness/schema/*.yaml`. The Rust side must produce the same
//! paths with the same value kinds. This module is the Rust half of that
//! contract and nothing more: a value enum that mirrors the decoder's value
//! kinds, a sink that receives `(path, value)` pairs, and a trait that types
//! implement to walk themselves into a sink.
//!
//! Layer 0: this crate has no dependencies beyond `hsd-types`, so the trace
//! format itself (JSON, `approx` decimals, ...) lives in `melee-diff`, which
//! converts [`SnapValue`] into its own `Value`.
//!
//! # Path composition
//!
//! A leaf (`f32`, `i32`, ...) emits itself at the empty path `""`. Aggregates
//! wrap the sink in a [`PrefixSink`] per field, and the prefix sink joins
//! `prefix` and `path` with `.`, dropping the separator when either side is
//! empty. So a `Vec3` field named `cur_pos` under player prefix `p0` becomes
//! `p0.cur_pos.x`, and an `f32` field named `percent` becomes `p0.percent`:
//!
//! ```
//! use hsd_types::Vec3;
//! use melee_types::snapshot::{PrefixSink, SnapValue, Snapshot, SnapshotSink};
//!
//! struct Fighter { cur_pos: Vec3, percent: f32 }
//!
//! impl Snapshot for Fighter {
//!     fn snapshot(&self, sink: &mut dyn SnapshotSink) {
//!         sink.field("cur_pos", &self.cur_pos);
//!         sink.field("percent", &self.percent);
//!     }
//! }
//!
//! let f = Fighter { cur_pos: Vec3::new(1.0, 2.0, 3.0), percent: 42.5 };
//! let mut out: Vec<(String, SnapValue)> = Vec::new();
//! f.snapshot(&mut PrefixSink::new(&mut out, "p0"));
//! assert_eq!(out[0], ("p0.cur_pos.x".to_string(), SnapValue::f32(1.0)));
//! assert_eq!(out[3], ("p0.percent".to_string(), SnapValue::f32(42.5)));
//! ```
//!
//! # Value kinds
//!
//! These mirror `harness/decode.py::_val` exactly: signed schema types
//! (`s8 s16 s32`) decode to `I64`, unsigned ones (`u8 u16 u32 ptr`) to
//! `U64`, floats carry their bit pattern. Emit the same kind the schema
//! declares for the field, even when the Rust type differs (a `bool` that
//! retail stores as `s32` must emit `I64(0|1)`, not `U64`).

use hsd_types::Vec3;

/// One emitted value. Floats are carried as raw bits so that a one-ULP
/// difference is visible and no formatting step can round it away.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SnapValue {
    /// Signed integer schema types: `s8 s16 s32`.
    I64(i64),
    /// Unsigned integer schema types: `u8 u16 u32 ptr`.
    U64(u64),
    /// `f32` bit pattern.
    F32(u32),
    /// `f64` bit pattern.
    F64(u64),
    /// Free text, for names and phase markers. Never used for numeric state.
    Str(String),
    /// Absent or not-applicable.
    Null,
}

impl SnapValue {
    /// The bit pattern of `x`.
    #[inline]
    pub fn f32(x: f32) -> Self {
        SnapValue::F32(x.to_bits())
    }

    /// The bit pattern of `x`.
    #[inline]
    pub fn f64(x: f64) -> Self {
        SnapValue::F64(x.to_bits())
    }
}

/// Receives `(path, value)` pairs from a [`Snapshot`] walk.
pub trait SnapshotSink {
    /// Record `v` at `path`. Paths are dotted and relative to whatever prefix
    /// the caller has already applied; a leaf emits at `""`.
    fn put(&mut self, path: &str, v: SnapValue);
}

impl dyn SnapshotSink + '_ {
    /// Emit `value` under `name`, joining nested paths with `.`.
    ///
    /// This is how aggregates name their fields; see the module docs.
    pub fn field(&mut self, name: &str, value: &dyn Snapshot) {
        value.snapshot(&mut PrefixSink::new(self, name));
    }
}

/// Collects into a `Vec` in emission order. Used by tests and by coverage
/// checks that only care about the set of paths.
impl SnapshotSink for Vec<(String, SnapValue)> {
    fn put(&mut self, path: &str, v: SnapValue) {
        self.push((path.to_owned(), v));
    }
}

/// Adapter that prepends a prefix to every path before forwarding.
///
/// `PrefixSink::new(sink, "p0")` turns `cur_pos.x` into `p0.cur_pos.x`.
/// Prefix sinks nest: wrapping a `PrefixSink` in another composes the
/// prefixes in outer-to-inner order.
pub struct PrefixSink<'a> {
    inner: &'a mut dyn SnapshotSink,
    prefix: &'a str,
}

impl<'a> PrefixSink<'a> {
    /// Wrap `inner`, prepending `prefix` to every path.
    pub fn new(inner: &'a mut dyn SnapshotSink, prefix: &'a str) -> Self {
        PrefixSink { inner, prefix }
    }
}

impl SnapshotSink for PrefixSink<'_> {
    fn put(&mut self, path: &str, v: SnapValue) {
        match (self.prefix.is_empty(), path.is_empty()) {
            (true, _) => self.inner.put(path, v),
            (false, true) => self.inner.put(self.prefix, v),
            (false, false) => {
                let mut full = String::with_capacity(self.prefix.len() + 1 + path.len());
                full.push_str(self.prefix);
                full.push('.');
                full.push_str(path);
                self.inner.put(&full, v);
            }
        }
    }
}

/// Walk a value into a sink as `(path, value)` pairs.
///
/// Implementations for aggregates call `sink.field(name, &self.member)` once
/// per schema field, in header order, using the field names from
/// `harness/schema/`. Leaves call `sink.put("", value)`.
pub trait Snapshot {
    fn snapshot(&self, sink: &mut dyn SnapshotSink);
}

impl<T: Snapshot + ?Sized> Snapshot for &T {
    fn snapshot(&self, sink: &mut dyn SnapshotSink) {
        (**self).snapshot(sink)
    }
}

macro_rules! leaf {
    ($($t:ty => $conv:expr),* $(,)?) => {
        $(
            impl Snapshot for $t {
                #[inline]
                fn snapshot(&self, sink: &mut dyn SnapshotSink) {
                    let conv: fn($t) -> SnapValue = $conv;
                    sink.put("", conv(*self));
                }
            }
        )*
    };
}

leaf! {
    i8 => |x| SnapValue::I64(i64::from(x)),
    i16 => |x| SnapValue::I64(i64::from(x)),
    i32 => |x| SnapValue::I64(i64::from(x)),
    i64 => SnapValue::I64,
    u8 => |x| SnapValue::U64(u64::from(x)),
    u16 => |x| SnapValue::U64(u64::from(x)),
    u32 => |x| SnapValue::U64(u64::from(x)),
    u64 => SnapValue::U64,
    f32 => SnapValue::f32,
    f64 => SnapValue::f64,
}

/// Retail stores flags declared `bool`/`BOOL` as `s32` (see `PRIMITIVES` in
/// `harness/gen_schema.py`), so a `bool` emits `I64(0|1)`.
impl Snapshot for bool {
    fn snapshot(&self, sink: &mut dyn SnapshotSink) {
        sink.put("", SnapValue::I64(i64::from(*self)));
    }
}

/// Emits `.x`, `.y`, `.z`, matching the decoder's `vec3` expansion.
impl Snapshot for Vec3 {
    fn snapshot(&self, sink: &mut dyn SnapshotSink) {
        sink.put("x", SnapValue::f32(self.x));
        sink.put("y", SnapValue::f32(self.y));
        sink.put("z", SnapValue::f32(self.z));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect(v: &dyn Snapshot, prefix: &str) -> Vec<(String, SnapValue)> {
        let mut out = Vec::new();
        v.snapshot(&mut PrefixSink::new(&mut out, prefix));
        out
    }

    #[test]
    fn leaves_emit_at_prefix() {
        assert_eq!(
            collect(&1.5f32, "a"),
            vec![("a".into(), SnapValue::F32(0x3FC0_0000))]
        );
        assert_eq!(collect(&-7i8, "b"), vec![("b".into(), SnapValue::I64(-7))]);
        assert_eq!(
            collect(&200u8, "c"),
            vec![("c".into(), SnapValue::U64(200))]
        );
        assert_eq!(collect(&true, "d"), vec![("d".into(), SnapValue::I64(1))]);
        assert_eq!(
            collect(&2.0f64, "e"),
            vec![("e".into(), SnapValue::F64(0x4000_0000_0000_0000))]
        );
        // No prefix at all: a bare leaf lands at "".
        assert_eq!(collect(&3i32, ""), vec![("".into(), SnapValue::I64(3))]);
    }

    #[test]
    fn f32_keeps_bits() {
        // NaN payloads and signed zero survive; a decimal round trip would not
        // preserve them.
        let nan = f32::from_bits(0x7FC0_1234);
        assert_eq!(SnapValue::f32(nan), SnapValue::F32(0x7FC0_1234));
        assert_ne!(SnapValue::f32(-0.0), SnapValue::f32(0.0));
    }

    #[test]
    fn vec3_expands_xyz() {
        let v = Vec3::new(1.0, 2.0, 3.0);
        let got = collect(&v, "p0.cur_pos");
        assert_eq!(
            got,
            vec![
                ("p0.cur_pos.x".into(), SnapValue::f32(1.0)),
                ("p0.cur_pos.y".into(), SnapValue::f32(2.0)),
                ("p0.cur_pos.z".into(), SnapValue::f32(3.0)),
            ]
        );
    }

    struct Inner {
        buttons: u32,
        lstick_x: i8,
    }

    impl Snapshot for Inner {
        fn snapshot(&self, sink: &mut dyn SnapshotSink) {
            sink.field("buttons", &self.buttons);
            sink.field("lstick_x", &self.lstick_x);
        }
    }

    struct Outer {
        kind: i32,
        cur_pos: Vec3,
        cpu: Inner,
    }

    impl Snapshot for Outer {
        fn snapshot(&self, sink: &mut dyn SnapshotSink) {
            sink.field("kind", &self.kind);
            sink.field("cur_pos", &self.cur_pos);
            sink.field("cpu", &self.cpu);
        }
    }

    #[test]
    fn nested_prefixes_compose() {
        let o = Outer {
            kind: 1,
            cur_pos: Vec3::new(0.0, -1.0, 0.5),
            cpu: Inner {
                buttons: 0x100,
                lstick_x: -80,
            },
        };
        let paths: Vec<String> = collect(&o, "p1").into_iter().map(|(p, _)| p).collect();
        assert_eq!(
            paths,
            [
                "p1.kind",
                "p1.cur_pos.x",
                "p1.cur_pos.y",
                "p1.cur_pos.z",
                "p1.cpu.buttons",
                "p1.cpu.lstick_x"
            ]
        );
        // Without an outer prefix the same walk yields schema-relative paths.
        let bare: Vec<String> = collect(&o, "").into_iter().map(|(p, _)| p).collect();
        assert_eq!(bare[0], "kind");
        assert_eq!(bare[4], "cpu.buttons");
    }

    #[test]
    fn reference_forwards() {
        let x = 5u16;
        let r = &x;
        assert_eq!(collect(&r, "n"), vec![("n".into(), SnapValue::U64(5))]);
    }
}
