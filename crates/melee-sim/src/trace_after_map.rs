//! Retail's sample at the end of each fighter's map proc, beside the tick's
//! end state.
//!
//! The tick trace reads fighters when the scheduler tick completes. A value
//! a later proc of the same tick overwrites is invisible there: a captured
//! fighter rides its moving floor in Fighter_procUpdate (0x8006BE48), stands
//! there through its map proc, and is pinned back to its captor by the
//! accessory proc. Slippi before 3.4.0 reads Post Frame at the end of the
//! map proc (Fighter_8006C27C, hook 0x8006C5D8), so such a state decides
//! whether a replay matches.
//!
//! A scenario with `after_map = true` records each fighter's struct when
//! its map proc returns (`harness/dolphin/tick_trace.py`, which explains
//! the hook); `decode.py` writes it beside the tick record as `after_map`,
//! with the same keys as `state`. Fighter `N`'s `pN.*` keys are present on
//! the ticks its map proc ran.
use anyhow::{Context, Result};
use melee_diff::{first_divergence, Record};

/// The comparison's `phase`, shown in a divergence.
const PHASE: &str = "after_map";

/// One tick's retail after-map sample from its expected-trace line.
pub fn expected(json: &serde_json::Value, frame: u64) -> Result<Record> {
    let state = json.get("after_map").with_context(|| {
        format!(
            "tick {frame} has no after-map sample: record the scenario again with \
             `after_map = true`"
        )
    })?;
    Ok(Record {
        frame,
        phase: PHASE.into(),
        state: serde_json::from_value(state.clone()).context("after_map state")?,
    })
}

/// The port's after-map record (`Simulation::after_map_record`) as the keys
/// retail sampled this tick.
pub fn actual(after_map: &Record, expected: &Record) -> Record {
    Record {
        frame: expected.frame,
        phase: PHASE.into(),
        state: after_map
            .state
            .iter()
            .filter(|(key, _)| expected.state.contains_key(*key))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
    }
}

/// The first key on which the port's after-map record leaves retail's.
pub fn divergence(expected: &Record, after_map: &Record) -> Option<String> {
    first_divergence([expected], [&actual(after_map, expected)]).map(|diff| {
        format!("{diff}\n(at the end of the fighter's map proc, retail 0x8006C5D8)")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use melee_diff::Value;

    fn record(values: &[(&str, u64)]) -> Record {
        Record {
            frame: 3,
            phase: "frame_end".into(),
            state: values
                .iter()
                .map(|(key, value)| (key.to_string(), Value::UInt(*value)))
                .collect(),
        }
    }

    #[test]
    fn only_the_fighters_retail_sampled_are_compared() {
        let line = serde_json::json!({"after_map": {"p1.motion_id": {"t": "u", "v": 226}}});
        let retail = expected(&line, 3).unwrap();
        // Fighter 0's map proc did not reach its end on retail this tick.
        let port = record(&[("p0.motion_id", 14), ("p1.motion_id", 226)]);
        assert_eq!(divergence(&retail, &port), None);
        let port = record(&[("p0.motion_id", 14), ("p1.motion_id", 227)]);
        let diff = divergence(&retail, &port).unwrap();
        assert!(diff.contains("p1.motion_id") && diff.contains("0x8006C5D8"), "{diff}");
    }

    #[test]
    fn a_recording_without_the_sample_is_an_error() {
        let error = expected(&serde_json::json!({"state": {}}), 7).unwrap_err();
        assert!(error.to_string().contains("after_map = true"), "{error}");
    }
}
