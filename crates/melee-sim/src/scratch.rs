//! The stale motion-scratch words against retail (`melee-sim scratch-diff`).
//!
//! Retail's per-state variables share one union that a motion change does
//! not clear, and Yoshi's egg shield reads two of its words back as
//! countdowns long after another state wrote them
//! (`melee_ft::fighter::scratch_tail`). Every tick trace holds each
//! fighter's memory, so the port's image of those words can be checked on
//! every tick of every recording.
use crate::scenario::Scenario;
use anyhow::Result;
use std::collections::BTreeMap;
use std::io::BufRead;

/// Fighter offsets of the compared words: mv+0x14 and mv+0x18.
const WORDS: [usize; 2] = [0x2354, 0x2358];
/// Fighter offset of the motion state id.
const MOTION: usize = 0x10;
/// Fighter offset of the kind.
const KIND: usize = 0x4;

pub struct Report {
    pub differences: Vec<String>,
    pub summary: String,
}

/// Whether a word still holds a countdown after one tick of
/// ftCo_80093BC0 (`x -= 1; if (x < 0) clear`): a float of at least 1.0, or
/// a NaN, which never compares below zero.
fn counts_down(bits: u32) -> bool {
    let value = f32::from_bits(bits);
    value.is_nan() || value >= 1.0
}

/// The big-endian word at `offset` of each fighter image in one trace line.
fn fighter_words(line: &str, offsets: &[usize]) -> Vec<Vec<u32>> {
    const KEY: &str = "\"bytes\": \"";
    let mut fighters = Vec::new();
    let mut rest = line;
    while let Some(at) = rest.find(KEY) {
        let hex = &rest[at + KEY.len()..];
        let end = hex.find('"').unwrap_or(hex.len());
        let hex = &hex[..end];
        let words = offsets
            .iter()
            .map(|&offset| {
                hex.get(offset * 2..offset * 2 + 8)
                    .and_then(|text| u32::from_str_radix(text, 16).ok())
                    .unwrap_or(0)
            })
            .collect();
        fighters.push(words);
        rest = &rest[at + KEY.len() + end..];
    }
    fighters
}

/// Each tick's image of the two words against the recording, for fighters
/// of `kind` (retail's internal id) or all of them. A difference that
/// changes whether the word counts down is listed; the others are counted,
/// or listed too with `every_bit`.
/// After a difference the image takes retail's words, so each line names
/// the state that wrote (or failed to write) them.
pub fn diff(scenario: &Scenario, kind: Option<u32>, every_bit: bool) -> Result<Report> {
    let path = scenario.trace_path("tick.raw.jsonl");
    let mut simulation = crate::trace::simulation_displayed_as(scenario, &path)?;
    let mut differences = Vec::new();
    // (retail motion, word) -> ticks compared, differing bits, differing class.
    let mut by_state: BTreeMap<(u32, usize), [usize; 3]> = BTreeMap::new();
    let offsets = [KIND, MOTION, WORDS[0], WORDS[1]];
    for (tick, line) in melee_trace_io::open(&path)?.lines().enumerate() {
        let line = line?;
        if simulation.tick_without_snapshot().is_err() {
            differences.push(format!("tick {tick}: the port stopped"));
            break;
        }
        let port = simulation.fighter_scratch_tails();
        for (index, retail) in fighter_words(&line, &offsets).iter().enumerate() {
            let Some(&image) = port.get(index) else {
                continue;
            };
            if let Some(kind) = kind {
                if kind != retail[0] {
                    continue;
                }
            }
            if image != [retail[2], retail[3]] {
                // Report a difference where it starts, not on every tick
                // the stale word then lives.
                simulation.set_fighter_scratch_tail(index, [retail[2], retail[3]]);
            }
            for word in 0..2 {
                let counts = by_state.entry((retail[1], word)).or_default();
                counts[0] += 1;
                let (expected, actual) = (retail[2 + word], image[word]);
                if expected == actual {
                    continue;
                }
                counts[1] += 1;
                let countdown = counts_down(expected) != counts_down(actual)
                    || (counts_down(expected) && expected != actual);
                if countdown {
                    counts[2] += 1;
                }
                if countdown || every_bit {
                    differences.push(format!(
                        "tick {tick}: fighter {index} (kind {}) motion {} mv+0x{:X}: retail {expected:08X} ({}), port {actual:08X} ({})",
                        retail[0],
                        retail[1],
                        0x14 + 4 * word,
                        f32::from_bits(expected),
                        f32::from_bits(actual),
                    ));
                }
            }
        }
    }
    let mut summary = String::new();
    let mut countdowns = 0;
    for ((motion, word), [ticks, bits, class]) in &by_state {
        countdowns += class;
        if *bits != 0 {
            summary.push_str(&format!(
                "motion {motion} mv+0x{:X}: {ticks} ticks, {bits} with other bits, {class} with another countdown\n",
                0x14 + 4 * word
            ));
        }
    }
    summary.push_str(&format!("{countdowns} countdown differences"));
    Ok(Report {
        differences,
        summary,
    })
}
