//! HSD_PadADConvert against retail: every pad recorded in these corpus traces
//! carries the virtual stick direction bits the game synthesized; the port
//! must derive the same bits from the same clamped sticks.
use std::{io::BufRead, path::PathBuf};

/// Generated matches with every stick direction and many diagonals.
const TRACES: [&str; 4] = [
    "corpus_v2_s0_e2a_p1",
    "corpus_v2_s1_e49_p2",
    "corpus_v3_s1_e2af099ca_p1",
    "corpus_v3_s1_e4068796b_p0",
];

fn trace(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../harness/traces")
        .join(format!("{name}.tick.expected.jsonl"))
}

#[test]
fn stick_direction_bits_match_retail() {
    if !melee_test_support::require_files(TRACES.map(trace)) {
        return;
    }
    let mut checked = 0;
    for name in TRACES {
        for line in melee_test_support::trace::open(&trace(name))
            .unwrap()
            .lines()
        {
            let row: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
            for port in ["p0", "p1"] {
                let pad = &row["inputs"][port];
                let byte = |key: &str| pad[key]["v"].as_i64().unwrap() as i8;
                let expected = pad["button"]["v"].as_u64().unwrap() as u32
                    & melee_ft::input::pad::STICK_DIRECTIONS;
                let actual = melee_ft::input::pad::stick_directions(
                    [byte("stickX"), byte("stickY")],
                    [byte("subStickX"), byte("subStickY")],
                );
                assert_eq!(
                    actual.0,
                    expected,
                    "{name} {port}: sticks ({}, {}) / ({}, {})",
                    byte("stickX"),
                    byte("stickY"),
                    byte("subStickX"),
                    byte("subStickY")
                );
                checked += 1;
            }
        }
    }
    assert!(checked > 10_000, "only {checked} pads checked");
}
