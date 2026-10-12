//! Where a replay's time goes: `MELEE_REPLAY_COST=1` prints one line per
//! replay to stderr with the time and simulated ticks of each part of the
//! runner (`docs/PERF.md`, "Replay runner"). Counters only; no result reads
//! them.
use std::cell::RefCell;
use std::time::{Duration, Instant};

/// The parts of one replay's run. A part's time excludes the parts timed
/// inside it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    /// Reading and parsing the `.slp`.
    Parse,
    /// The recording as a trace of expected records.
    Expected,
    /// Building a cold scenario (pads, boundary seed).
    Scenario,
    /// Loading archives and constructing a match, with its first tick.
    Start,
    /// The stage code's dry pass.
    StageProbe,
    /// The screen code's dry pass.
    ScreenProbe,
    /// The UCF version's dry pass.
    FixProbe,
    /// The comparison run.
    Compare,
}

const PARTS: [Part; 8] = [
    Part::Parse,
    Part::Expected,
    Part::Scenario,
    Part::Start,
    Part::StageProbe,
    Part::ScreenProbe,
    Part::FixProbe,
    Part::Compare,
];

#[derive(Default)]
struct Cost {
    enabled: bool,
    /// Time, entries and simulated ticks per part.
    parts: [(Duration, u32, u64); PARTS.len()],
    /// The parts being timed, innermost last, each with the time its inner
    /// parts took.
    open: Vec<(Part, Instant, Duration)>,
}

thread_local! {
    static COST: RefCell<Cost> = RefCell::new(Cost {
        enabled: std::env::var_os("MELEE_REPLAY_COST").is_some(),
        ..Cost::default()
    });
}

/// Times a part until dropped.
pub struct Timed(bool);

pub fn time(part: Part) -> Timed {
    COST.with_borrow_mut(|cost| {
        if cost.enabled {
            cost.open.push((part, Instant::now(), Duration::ZERO));
        }
        Timed(cost.enabled)
    })
}

impl Drop for Timed {
    fn drop(&mut self) {
        if !self.0 {
            return;
        }
        COST.with_borrow_mut(|cost| {
            let Some((part, since, inner)) = cost.open.pop() else {
                return;
            };
            let whole = since.elapsed();
            let entry = &mut cost.parts[part as usize];
            entry.0 += whole.saturating_sub(inner);
            entry.1 += 1;
            if let Some(outer) = cost.open.last_mut() {
                outer.2 += whole;
            }
        });
    }
}

/// Count ticks simulated for the innermost part being timed.
pub fn ticks(count: u64) {
    COST.with_borrow_mut(|cost| {
        if let Some(&(part, ..)) = cost.open.last() {
            cost.parts[part as usize].2 += count;
        }
    });
}

/// Print this thread's counters for one replay and clear them.
pub fn report(name: &str, frames: usize) {
    COST.with_borrow_mut(|cost| {
        if !cost.enabled {
            return;
        }
        let mut line = format!("cost\t{name}\tframes={frames}");
        for part in PARTS {
            let (time, entries, ticks) = cost.parts[part as usize];
            line += &format!(
                "\t{part:?}={:.1}ms/{entries}/{ticks}",
                time.as_secs_f64() * 1e3
            );
        }
        eprintln!("{line}");
        cost.parts = Default::default();
        cost.open.clear();
    });
}
