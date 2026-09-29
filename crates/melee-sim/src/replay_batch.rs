//! Replays a Slippi corpus in parallel and groups the first stops by cause.
//!
//! Each replay runs through [`crate::replay::run`] on its own thread. The
//! grouping key names what stopped it: an unsupported setup, a missing setup
//! fact, an unported boundary (the `unimplemented!` text), or a divergence
//! (the diverging field and the recorded actions of both leaders). Replays
//! that stop the same way are usually the same bug, so the largest groups
//! are the next work.
use crate::replay::{Report, Setup, Stop};
use std::collections::BTreeMap;
use std::io::Write;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

pub struct Outcome {
    pub path: PathBuf,
    pub report: Result<Report, String>,
}

impl Outcome {
    /// Frames matched and recorded (zero for a replay that failed to run).
    pub fn frames(&self) -> (usize, usize) {
        self.report
            .as_ref()
            .map_or((0, 0), |report| (report.matched, report.frames))
    }

    /// The grouping key: replays with the same key are likely the same fault.
    pub fn category(&self) -> String {
        let report = match &self.report {
            Ok(report) => report,
            Err(error) => return format!("error: {}", first_line(error)),
        };
        match &report.stop {
            Stop::Complete => "complete".into(),
            Stop::Unsupported(reasons) => format!("unsupported: {}", reasons.join("; ")),
            Stop::NeedsSetup(reason) => format!("needs setup: {}", first_line(reason)),
            Stop::UnavailableInput { .. } => "input unavailable".into(),
            Stop::Unported { reason, .. } => format!(
                "unported: {}",
                reason.strip_prefix("not implemented: ").unwrap_or(reason)
            ),
            Stop::Diverged(diff) => format!(
                "diverged: {} [{}]",
                telling_field(&report.differing).unwrap_or(&diff.path),
                report.context
            ),
        }
    }
}

/// The differing field that best names the fault, without its player
/// prefix: a motion change explains position and frame differences, a
/// ground/air change explains position, and so on.
fn telling_field(differing: &[String]) -> Option<&str> {
    const ORDER: [&str; 6] = [
        "motion_id",
        "ground_or_air",
        "percent",
        "cur_pos",
        "facing_dir",
        "cur_anim_frame",
    ];
    let field = |key: &String| {
        let rest = key.split_once('.').map_or(key.as_str(), |(_, f)| f);
        rest.strip_prefix("follower.").unwrap_or(rest).to_string()
    };
    let fields: Vec<String> = differing.iter().map(field).collect();
    ORDER
        .iter()
        .find_map(|name| fields.iter().position(|f| f.starts_with(name)))
        .or((!fields.is_empty()).then_some(0))
        .map(|index| {
            let key = &differing[index];
            let rest = key.split_once('.').map_or(key.as_str(), |(_, f)| f);
            rest.split('.').next().unwrap_or(rest)
        })
}

fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or("")
}

/// Every `.slp` under the given files and directories, sorted.
pub fn collect(inputs: &[PathBuf]) -> std::io::Result<Vec<PathBuf>> {
    fn walk(path: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
        if path.is_dir() {
            for entry in std::fs::read_dir(path)? {
                walk(&entry?.path(), out)?;
            }
        } else if path.extension().is_some_and(|ext| ext == "slp") {
            out.push(path.to_path_buf());
        }
        Ok(())
    }
    let mut out = Vec::new();
    for input in inputs {
        walk(input, &mut out)?;
    }
    out.sort();
    Ok(out)
}

/// Run every replay on `jobs` threads. A panic outside the replay runner's
/// own `unimplemented!` handling becomes that replay's error, not the batch's.
pub fn run(paths: &[PathBuf], root: &Path, setup: Setup, jobs: usize) -> Vec<Outcome> {
    let next = AtomicUsize::new(0);
    let results: Mutex<Vec<Option<Outcome>>> = Mutex::new((0..paths.len()).map(|_| None).collect());
    std::thread::scope(|scope| {
        for _ in 0..jobs.max(1) {
            scope.spawn(|| loop {
                let index = next.fetch_add(1, Ordering::Relaxed);
                let Some(path) = paths.get(index) else { break };
                let report = catch_unwind(AssertUnwindSafe(|| {
                    crate::replay::run_file(path, root, setup)
                }))
                .map_err(|payload| {
                    let message = payload
                        .downcast_ref::<String>()
                        .cloned()
                        .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                        .unwrap_or_else(|| "unknown panic".into());
                    format!("panic: {message}")
                })
                .and_then(|result| result.map_err(|error| format!("{error:#}")));
                eprintln!(
                    "[{}/{}] {}",
                    index + 1,
                    paths.len(),
                    path.file_name().unwrap_or_default().to_string_lossy()
                );
                results.lock().unwrap()[index] = Some(Outcome {
                    path: path.clone(),
                    report,
                });
            });
        }
    });
    results
        .into_inner()
        .unwrap()
        .into_iter()
        .map(|outcome| outcome.expect("every replay ran"))
        .collect()
}

/// The per-replay lines, then the groups by size with an example each.
pub fn summarize(outcomes: &[Outcome], out: &mut dyn Write) -> std::io::Result<()> {
    let mut groups: BTreeMap<String, Vec<&Outcome>> = BTreeMap::new();
    for outcome in outcomes {
        groups.entry(outcome.category()).or_default().push(outcome);
    }
    let mut groups: Vec<_> = groups.into_iter().collect();
    groups.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| a.0.cmp(&b.0)));
    let (matched, frames) = outcomes.iter().fold((0, 0), |(m, f), outcome| {
        let (om, of) = outcome.frames();
        (m + om, f + of)
    });
    let complete = outcomes
        .iter()
        .filter(|o| {
            matches!(
                o.report,
                Ok(Report {
                    stop: Stop::Complete,
                    ..
                })
            )
        })
        .count();
    writeln!(
        out,
        "{} replays: {complete} complete; {matched} / {frames} frames matched ({:.1}%)",
        outcomes.len(),
        100.0 * matched as f64 / frames.max(1) as f64
    )?;
    writeln!(out, "\nBy first stop, largest group first:")?;
    for (category, members) in &groups {
        let example = members
            .iter()
            .min_by_key(|o| o.frames().0)
            .expect("nonempty group");
        let stop_tick = example.frames().0;
        writeln!(
            out,
            "{:5}  {category}\n       e.g. {} (matched {stop_tick})",
            members.len(),
            example.path.display()
        )?;
    }
    Ok(())
}

/// One JSON object per replay, for later tooling.
pub fn write_jsonl(outcomes: &[Outcome], out: &mut dyn Write) -> std::io::Result<()> {
    for outcome in outcomes {
        let (matched, frames) = outcome.frames();
        let (detail, context) = match &outcome.report {
            Ok(report) => (report.to_string(), report.context.clone()),
            Err(error) => (error.clone(), String::new()),
        };
        let line = serde_json::json!({
            "path": outcome.path,
            "matched": matched,
            "frames": frames,
            "category": outcome.category(),
            "context": context,
            "detail": detail,
        });
        writeln!(out, "{line}")?;
    }
    Ok(())
}
