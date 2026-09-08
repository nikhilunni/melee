//! Canonical trace format and divergence detection.
//!
//! A trace is JSON Lines. Each line is one [`Record`]: a frame number, a
//! phase name marking where inside the frame the snapshot was taken, and a
//! flat map from dotted field path to value. Both the Dolphin oracle
//! (`harness/decode.py`) and the Rust port emit this format, driven by the
//! same schema files under `harness/schema/`.
//!
//! Floats are compared by bit pattern. A one-ULP difference is a real bug.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

mod snapshot;
pub use snapshot::RecordSink;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Record {
    pub frame: u64,
    pub phase: String,
    pub state: BTreeMap<String, Value>,
}

/// Values are stored so that float comparison is exact: an `f32` is carried
/// as its bit pattern alongside a human-readable decimal.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t", content = "v")]
pub enum Value {
    #[serde(rename = "i")]
    Int(i64),
    #[serde(rename = "u")]
    UInt(u64),
    #[serde(rename = "f32")]
    F32 { bits: u32, approx: f64 },
    #[serde(rename = "f64")]
    F64 { bits: u64, approx: f64 },
    #[serde(rename = "s")]
    Str(String),
    #[serde(rename = "null")]
    Null,
}

/// Floats compare by bit pattern only. The `approx` field is a human-readable
/// convenience and may lose its last digit on a JSON round trip, so it must
/// never participate in equality.
impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::UInt(a), Value::UInt(b)) => a == b,
            (Value::F32 { bits: a, .. }, Value::F32 { bits: b, .. }) => a == b,
            (Value::F64 { bits: a, .. }, Value::F64 { bits: b, .. }) => a == b,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Null, Value::Null) => true,
            _ => false,
        }
    }
}

impl Value {
    pub fn f32(x: f32) -> Self {
        Value::F32 { bits: x.to_bits(), approx: x as f64 }
    }
    pub fn f64(x: f64) -> Self {
        Value::F64 { bits: x.to_bits(), approx: x }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Divergence {
    pub frame: u64,
    pub phase: String,
    pub path: String,
    pub expected: Option<Value>,
    pub actual: Option<Value>,
}

impl std::fmt::Display for Divergence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "first divergence at frame {} phase {}", self.frame, self.phase)?;
        writeln!(f, "  field:    {}", self.path)?;
        writeln!(f, "  expected: {}", fmt_val(&self.expected))?;
        write!(f, "  actual:   {}", fmt_val(&self.actual))
    }
}

fn fmt_val(v: &Option<Value>) -> String {
    match v {
        None => "<missing>".into(),
        Some(Value::Int(i)) => i.to_string(),
        Some(Value::UInt(u)) => format!("{u} (0x{u:X})"),
        Some(Value::F32 { bits, approx }) => format!("{approx} (0x{bits:08X})"),
        Some(Value::F64 { bits, approx }) => format!("{approx} (0x{bits:016X})"),
        Some(Value::Str(s)) => format!("{s:?}"),
        Some(Value::Null) => "null".into(),
    }
}

/// Compare two traces record by record. Returns the first divergence, or
/// `None` if `actual` matches `expected` for the full length of `expected`.
///
/// `actual` being longer than `expected` is not a divergence; `actual` being
/// shorter is reported as a missing record.
pub fn first_divergence<'a, E, A>(expected: E, actual: A) -> Option<Divergence>
where
    E: IntoIterator<Item = &'a Record>,
    A: IntoIterator<Item = &'a Record>,
{
    let mut actual = actual.into_iter();
    for exp in expected {
        let Some(act) = actual.next() else {
            return Some(Divergence {
                frame: exp.frame,
                phase: exp.phase.clone(),
                path: "<record>".into(),
                expected: Some(Value::Str(format!("frame {} phase {}", exp.frame, exp.phase))),
                actual: None,
            });
        };
        if exp.frame != act.frame || exp.phase != act.phase {
            return Some(Divergence {
                frame: exp.frame,
                phase: exp.phase.clone(),
                path: "<record>".into(),
                expected: Some(Value::Str(format!("frame {} phase {}", exp.frame, exp.phase))),
                actual: Some(Value::Str(format!("frame {} phase {}", act.frame, act.phase))),
            });
        }
        for (path, ev) in &exp.state {
            match act.state.get(path) {
                Some(av) if av == ev => {}
                other => {
                    return Some(Divergence {
                        frame: exp.frame,
                        phase: exp.phase.clone(),
                        path: path.clone(),
                        expected: Some(ev.clone()),
                        actual: other.cloned(),
                    })
                }
            }
        }
    }
    None
}

pub fn read_trace(reader: impl std::io::BufRead) -> anyhow::Result<Vec<Record>> {
    let mut out = Vec::new();
    for (i, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let rec: Record = serde_json::from_str(&line)
            .map_err(|e| anyhow::anyhow!("line {}: {e}", i + 1))?;
        out.push(rec);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(frame: u64, phase: &str, kv: &[(&str, Value)]) -> Record {
        Record {
            frame,
            phase: phase.into(),
            state: kv.iter().map(|(k, v)| (k.to_string(), v.clone())).collect(),
        }
    }

    #[test]
    fn identical_traces_have_no_divergence() {
        let a = vec![rec(0, "post_fighter", &[("p0.pos.x", Value::f32(1.5))])];
        assert_eq!(first_divergence(&a, &a), None);
    }

    #[test]
    fn one_ulp_is_a_divergence() {
        let x = 1.5f32;
        let y = f32::from_bits(x.to_bits() + 1);
        let e = vec![rec(3, "post_fighter", &[("p0.pos.x", Value::f32(x))])];
        let a = vec![rec(3, "post_fighter", &[("p0.pos.x", Value::f32(y))])];
        let d = first_divergence(&e, &a).expect("should diverge");
        assert_eq!(d.frame, 3);
        assert_eq!(d.path, "p0.pos.x");
    }

    #[test]
    fn truncated_actual_is_reported() {
        let e = vec![rec(0, "a", &[]), rec(1, "a", &[])];
        let a = vec![rec(0, "a", &[])];
        let d = first_divergence(&e, &a).unwrap();
        assert_eq!(d.frame, 1);
        assert_eq!(d.actual, None);
    }

    #[test]
    fn float_equality_ignores_approx() {
        let a = Value::F32 { bits: 0x41D5_28A9, approx: 26.644920349121094 };
        let b = Value::F32 { bits: 0x41D5_28A9, approx: 26.644920349121097 };
        assert_eq!(a, b);
        let c = Value::F32 { bits: 0x41D5_28AA, approx: 26.644920349121094 };
        assert_ne!(a, c);
    }

    #[test]
    fn roundtrips_through_json() {
        let r = rec(7, "post_input", &[("seed", Value::UInt(2745024)), ("p0.pos.y", Value::f32(-3.25))]);
        let s = serde_json::to_string(&r).unwrap();
        let back: Record = serde_json::from_str(&s).unwrap();
        assert_eq!(r, back);
    }
}
