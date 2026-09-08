//! Bit-for-bit replay of captured `frsqrte` / `fres` pairs against
//! `gekko_math::estimate`.
//!
//! Two sources:
//! - `tests/data/*_pairs.txt`: the committed fixture (see `tests/data/NOTICE`),
//!   always run.
//! - `harness/traces/{frsqrte_probe,fres64_probe,fres_probe}.jsonl` and the
//!   `jit/` copies: the full captures, run when present (they are gitignored
//!   and only exist on a machine that has run `harness/gekko_probe/run.sh`).
//!   The JIT capture quiets signalling-NaN inputs where the interpreter
//!   passes them through; that one known difference is tolerated for NaN
//!   inputs only.

use std::path::{Path, PathBuf};

use gekko_math::estimate::{fres, fres_bits, frsqrte_bits};

fn data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data")
}

fn traces_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/traces")
}

/// `<hex> <hex>` per line.
fn load_pairs_txt(path: &Path) -> Vec<(u64, u64)> {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let mut it = l.split_whitespace();
            let i = u64::from_str_radix(it.next().unwrap(), 16).unwrap();
            let o = u64::from_str_radix(it.next().unwrap(), 16).unwrap();
            (i, o)
        })
        .collect()
}

/// `{"input_bits": "0x..", "output_bits": "0x.."}` per line, parsed without a
/// JSON dependency (layer 0 stays dependency-free).
fn load_pairs_jsonl(path: &Path) -> Vec<(u64, u64)> {
    fn field(line: &str, key: &str) -> u64 {
        let at = line
            .find(key)
            .unwrap_or_else(|| panic!("no {key} in {line}"));
        let rest = &line[at + key.len()..];
        let start = rest.find("0x").unwrap() + 2;
        let end = rest[start..].find('"').unwrap() + start;
        u64::from_str_radix(&rest[start..end], 16).unwrap()
    }
    let text = std::fs::read_to_string(path).unwrap();
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| (field(l, "\"input_bits\""), field(l, "\"output_bits\"")))
        .collect()
}

#[derive(Clone, Copy)]
enum Op {
    Frsqrte,
    Fres64,
    Fres32,
}

impl Op {
    fn apply(self, input: u64) -> u64 {
        match self {
            Op::Frsqrte => frsqrte_bits(input),
            Op::Fres64 => fres_bits(input),
            Op::Fres32 => u64::from(fres(f32::from_bits(input as u32)).to_bits()),
        }
    }
    fn is_nan_input(self, input: u64) -> bool {
        match self {
            Op::Frsqrte | Op::Fres64 => {
                (input >> 52) & 0x7FF == 0x7FF && input & ((1 << 52) - 1) != 0
            }
            Op::Fres32 => (input >> 23) & 0xFF == 0xFF && input & 0x7F_FFFF != 0,
        }
    }
    fn quiet_bit(self) -> u64 {
        match self {
            Op::Frsqrte | Op::Fres64 => 1 << 51,
            Op::Fres32 => 1 << 22,
        }
    }
    fn width(self) -> usize {
        match self {
            Op::Frsqrte | Op::Fres64 => 16,
            Op::Fres32 => 8,
        }
    }
}

/// Replays every pair; returns the number checked. `jit` tolerates the
/// quiet-bit difference on NaN inputs.
fn replay(label: &str, op: Op, pairs: &[(u64, u64)], jit: bool) -> usize {
    let mut bad = Vec::new();
    for &(i, want) in pairs {
        let got = op.apply(i);
        if got == want {
            continue;
        }
        if jit && op.is_nan_input(i) && (got | op.quiet_bit()) == want {
            continue;
        }
        if bad.len() < 10 {
            let w = op.width();
            bad.push(format!("  in={i:0w$x} got={got:0w$x} want={want:0w$x}"));
        }
    }
    eprintln!("{label}: {} pairs replayed", pairs.len());
    assert!(
        bad.is_empty(),
        "{label}: model does not reproduce captured pairs:\n{}",
        bad.join("\n")
    );
    pairs.len()
}

#[test]
fn committed_fixture_replays_bit_for_bit() {
    let d = data_dir();
    let n1 = replay(
        "frsqrte fixture",
        Op::Frsqrte,
        &load_pairs_txt(&d.join("frsqrte_pairs.txt")),
        false,
    );
    let n2 = replay(
        "fres64 fixture",
        Op::Fres64,
        &load_pairs_txt(&d.join("fres64_pairs.txt")),
        false,
    );
    let n3 = replay(
        "fres32 fixture",
        Op::Fres32,
        &load_pairs_txt(&d.join("fres32_pairs.txt")),
        false,
    );
    assert!(
        n1 >= 3000 && n2 >= 3000 && n3 >= 3000,
        "fixture unexpectedly small"
    );
}

#[test]
fn full_captures_replay_bit_for_bit_if_present() {
    let files = [
        ("frsqrte_probe.jsonl", Op::Frsqrte),
        ("fres64_probe.jsonl", Op::Fres64),
        ("fres_probe.jsonl", Op::Fres32),
    ];
    let mut any = false;
    for (sub, jit) in [("", false), ("jit", true)] {
        let dir = traces_dir().join(sub);
        for (name, op) in files {
            let path = dir.join(name);
            if !path.exists() {
                continue;
            }
            any = true;
            let pairs = load_pairs_jsonl(&path);
            assert!(pairs.len() > 10_000, "{} looks truncated", path.display());
            replay(&format!("{sub}/{name}"), op, &pairs, jit);
        }
    }
    if !any {
        eprintln!("no harness/traces/*_probe.jsonl captures present; skipping full replay");
    }
}
