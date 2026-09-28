//! `melee-sim triage`: one report at a scenario's first divergence.
//!
//! The gate stops at the first differing key. A bug hunt then wants the rest
//! of the picture at that tick: every differing key, what both fighters were
//! doing just before, the items, who wrote the RNG, where the particle RNG
//! call sites part, which generators exist, and any bone mismatch. Each
//! section reuses the tool that already answers it; sections whose capture is
//! missing (no ledger, no bone dump) say so and are skipped.
use crate::{scenario::Scenario, trace};
use anyhow::Result;
use melee_diff::{first_divergence, read_trace, Record, Value};
use std::collections::VecDeque;
use std::fmt::Write as _;

/// Ticks of motion history printed before the divergence.
const HISTORY: usize = 20;
/// Differing keys listed at the divergence tick.
const KEY_LIMIT: usize = 40;
/// Particle call sites printed on each side of the first differing one.
const SITE_CONTEXT: usize = 6;

/// Where and how a scenario first leaves retail.
struct Divergence {
    tick: u64,
    /// The gate's own message (first differing key, or the port's error).
    summary: String,
    expected: Option<Record>,
    actual: Option<Record>,
    expected_items: Option<Record>,
    actual_items: Option<Record>,
    rng_writers: String,
    history: VecDeque<(u64, Record, Record)>,
}

/// The full triage report, or a one-line pass.
pub fn triage(scenario: &Scenario) -> Result<String> {
    let Some(d) = find_divergence(scenario)? else {
        return Ok(format!(
            "{}: {} ticks, no divergence\n",
            scenario.name, scenario.frames
        ));
    };
    let mut out = String::new();
    writeln!(out, "== {} diverges at tick {}", scenario.name, d.tick)?;
    writeln!(out, "{}", d.summary.trim_end())?;
    if !d.rng_writers.is_empty() {
        writeln!(out, "RNG-writing procs this tick: {}", d.rng_writers)?;
    }
    section(&mut out, "Differing keys");
    if let (Some(e), Some(a)) = (&d.expected, &d.actual) {
        write_key_diff(&mut out, e, a)?;
    }
    if let (Some(e), Some(a)) = (&d.expected_items, &d.actual_items) {
        write_key_diff(&mut out, e, a)?;
    }
    section(&mut out, "Motion history (expected / actual; changes only)");
    write_history(&mut out, &d.history)?;
    section(&mut out, "Items (expected / actual)");
    for (label, record) in [("retail", &d.expected_items), ("port", &d.actual_items)] {
        match record {
            Some(r) => writeln!(out, "{label:>6}: {}", item_summary(r))?,
            None => writeln!(out, "{label:>6}: (no item capture)")?,
        }
    }
    section(&mut out, "Particle RNG call sites");
    let from = d.tick.saturating_sub(1);
    match trace::particle_site_diff(scenario, from, d.tick, "ledger") {
        Ok(lines) if lines.is_empty() => writeln!(out, "identical through tick {}", d.tick)?,
        Ok(lines) => {
            for line in &lines {
                writeln!(out, "{}", site_summary(line))?;
            }
        }
        Err(e) => writeln!(out, "skipped: {e:#}")?,
    }
    section(&mut out, "Particle generators (from up to 10 ticks before)");
    let early = d.tick.saturating_sub(10);
    match trace::particle_state_diff_with(scenario, early, d.tick, false, true) {
        Ok(lines) if lines.is_empty() => writeln!(out, "identical through tick {}", d.tick)?,
        Ok(lines) => {
            // The first two differing ticks, each a key line plus two lists.
            for line in lines
                .iter()
                .filter(|l| !l.ends_with("differing ticks"))
                .take(6)
            {
                writeln!(out, "{line}")?;
            }
        }
        Err(e) => writeln!(out, "skipped: {e:#}")?,
    }
    section(&mut out, "Bones");
    match crate::bones::bones_diff(scenario, 12, Some(d.tick)) {
        Ok(lines) if lines.is_empty() => writeln!(out, "identical at tick {}", d.tick)?,
        Ok(lines) => {
            for line in lines.iter().take(24) {
                writeln!(out, "{line}")?;
            }
        }
        Err(e) => writeln!(out, "skipped: {e:#}")?,
    }
    Ok(out)
}

fn section(out: &mut String, title: &str) {
    out.push_str(&format!("\n-- {title}\n"));
}

/// The gate loop, keeping a short motion history and stopping at the first
/// divergence (fighter keys, then items) or port error.
fn find_divergence(scenario: &Scenario) -> Result<Option<Divergence>> {
    let expected = read_trace(melee_trace_io::open(&scenario.expected_path())?)?;
    let expected_items: Vec<Option<Record>> = {
        use std::io::BufRead as _;
        melee_trace_io::open(&scenario.expected_path())?
            .lines()
            .enumerate()
            .map(|(frame, line)| {
                crate::trace_items::expected(
                    &serde_json::from_str::<serde_json::Value>(&line?)?,
                    frame as u64,
                )
            })
            .collect::<Result<_>>()?
    };
    let mut simulation = trace::simulation(scenario)?;
    if !scenario.replay_inputs.is_empty() {
        simulation.tick()?;
    }
    let mut history = VecDeque::with_capacity(HISTORY + 1);
    for (exp, exp_items) in expected.into_iter().zip(expected_items) {
        let tick = exp.frame;
        let mut act = match simulation.tick() {
            Ok(record) => record,
            Err(e) => {
                return Ok(Some(Divergence {
                    tick,
                    summary: format!("port error: {e:#}"),
                    expected: Some(exp),
                    actual: None,
                    expected_items: exp_items,
                    actual_items: None,
                    rng_writers: String::new(),
                    history,
                }))
            }
        };
        if !scenario.replay_inputs.is_empty() {
            act.frame -= 1;
        }
        let act_items = exp_items
            .as_ref()
            .map(|_| simulation.item_snapshot(act.frame));
        let fighter_diff = first_divergence([&exp], [&act]);
        let item_diff = match (&exp_items, &act_items) {
            (Some(e), Some(a)) => first_divergence([e], [a]),
            _ => None,
        };
        if let Some(diff) = fighter_diff.or(item_diff) {
            return Ok(Some(Divergence {
                tick,
                summary: diff.to_string(),
                rng_writers: format!("{:?}", simulation.rng_writers()),
                expected: Some(exp),
                actual: Some(act),
                expected_items: exp_items,
                actual_items: act_items,
                history,
            }));
        }
        if history.len() == HISTORY {
            history.pop_front();
        }
        history.push_back((tick, exp, act));
    }
    Ok(None)
}

fn show(value: Option<&Value>) -> String {
    match value {
        None => "-".into(),
        Some(Value::F32 { bits, approx }) => format!("{approx} ({bits:#010X})"),
        Some(Value::F64 { approx, .. }) => format!("{approx}"),
        Some(Value::Int(v)) => v.to_string(),
        Some(Value::UInt(v)) => v.to_string(),
        Some(Value::Str(s)) => s.clone(),
        Some(Value::Null) => "null".into(),
    }
}

fn write_key_diff(out: &mut String, expected: &Record, actual: &Record) -> Result<()> {
    let keys: std::collections::BTreeSet<&String> =
        expected.state.keys().chain(actual.state.keys()).collect();
    let differing: Vec<_> = keys
        .into_iter()
        .filter(|k| expected.state.get(*k) != actual.state.get(*k))
        .collect();
    for key in differing.iter().take(KEY_LIMIT) {
        writeln!(
            out,
            "{key:<32} retail {:<28} port {}",
            show(expected.state.get(*key)),
            show(actual.state.get(*key))
        )?;
    }
    if differing.len() > KEY_LIMIT {
        writeln!(out, "... {} more", differing.len() - KEY_LIMIT)?;
    }
    Ok(())
}

fn fighter_line(record: &Record, port: usize) -> String {
    let get = |k: &str| show(record.state.get(&format!("p{port}.{k}")));
    let short = |k: &str| {
        record
            .state
            .get(&format!("p{port}.{k}"))
            .and_then(|v| match v {
                Value::F32 { approx, .. } => Some(format!("{approx:.2}")),
                _ => None,
            })
            .unwrap_or_else(|| "-".into())
    };
    format!(
        "p{port} {}@{} ({}, {})",
        get("motion_id"),
        short("cur_anim_frame"),
        short("cur_pos.x"),
        short("cur_pos.y")
    )
}

fn write_history(out: &mut String, history: &VecDeque<(u64, Record, Record)>) -> Result<()> {
    let motions = |r: &Record| {
        (0..2)
            .map(|p| r.state.get(&format!("p{p}.motion_id")).cloned())
            .collect::<Vec<_>>()
    };
    let mut previous = None;
    for (tick, exp, act) in history {
        let now = (motions(exp), motions(act));
        if previous.as_ref() == Some(&now) {
            continue;
        }
        writeln!(
            out,
            "{tick:>6} retail {} | {}",
            fighter_line(exp, 0),
            fighter_line(exp, 1)
        )?;
        if now.0 != now.1 {
            writeln!(
                out,
                "{:>6} port   {} | {}",
                "",
                fighter_line(act, 0),
                fighter_line(act, 1)
            )?;
        }
        previous = Some(now);
    }
    Ok(())
}

fn item_summary(record: &Record) -> String {
    let count = match record.state.get("items.count") {
        Some(Value::UInt(n)) => *n as usize,
        _ => 0,
    };
    if count == 0 {
        return "none".into();
    }
    (0..count)
        .map(|i| {
            let get = |k: &str| show(record.state.get(&format!("items.{i}.{k}")));
            let pos = |k: &str| match record.state.get(&format!("items.{i}.{k}")) {
                Some(Value::F32 { approx, .. }) => format!("{approx:.1}"),
                _ => "-".into(),
            };
            format!(
                "#{i} kind {} motion {} owner {} ({}, {}) hitbox {}",
                get("kind"),
                get("motion_id"),
                get("owner"),
                pos("pos.x"),
                pos("pos.y"),
                get("hitbox0.state")
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// `tick N: port [..] retail [..]` compressed to lengths and a window around
/// the first differing call site.
fn site_summary(line: &str) -> String {
    let parse = |tag: &str| -> Option<Vec<&str>> {
        let start = line.find(&format!("{tag} ["))? + tag.len() + 2;
        let end = start + line[start..].find(']')?;
        Some(line[start..end].split_whitespace().collect())
    };
    let (Some(port), Some(retail)) = (parse("port"), parse("retail")) else {
        return line.to_string();
    };
    let head = line.split(':').next().unwrap_or("");
    let first = port
        .iter()
        .zip(&retail)
        .position(|(a, b)| a != b)
        .unwrap_or(port.len().min(retail.len()));
    let window = |sites: &[&str]| {
        let lo = first.saturating_sub(SITE_CONTEXT);
        let hi = (first + SITE_CONTEXT).min(sites.len());
        sites[lo..hi].join(" ")
    };
    format!(
        "{head}: {} port / {} retail draws; first difference at #{first}\n    port   … {}\n    retail … {}",
        port.len(),
        retail.len(),
        window(&port),
        window(&retail)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn site_summaries_point_at_the_first_differing_call() {
        let line = "tick 7: port [A B C D] retail [A B X D E]";
        let summary = site_summary(line);
        assert!(summary.starts_with("tick 7: 4 port / 5 retail draws; first difference at #2"));
        assert!(summary.contains("port   … A B C D"));
        assert!(summary.contains("retail … A B X D E"));
    }
}
