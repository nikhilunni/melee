//! `melee-sim sweep`: every recorded scenario's gate in one process, across
//! the machine's cores, as a TSV; and the comparison of two sweeps.
//!
//! One row per scenario, sorted by name: `name`, `PASS`/`FAIL` (fighters,
//! items, seed), `EXACT`/`FAULT`/`NOTCMP`/`-` (the particle system against
//! its dump: exact, diverged, no comparable dump, or not reached), and a
//! detail. The columns are those of the wave scripts' `sweep.sh`, so sweeps
//! from either compare.
use crate::{scenario::Scenario, trace};
use anyhow::{Context, Result};
use std::{
    cell::RefCell,
    fmt::Write as _,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
};

/// One scenario's sweep result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub name: String,
    pub base: String,
    pub particles: String,
    pub detail: String,
}

impl Row {
    pub fn tsv(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}",
            self.name, self.base, self.particles, self.detail
        )
    }

    fn parse(line: &str) -> Option<Row> {
        let mut fields = line.splitn(4, '\t');
        Some(Row {
            name: fields.next()?.to_owned(),
            base: fields.next()?.to_owned(),
            particles: fields.next()?.to_owned(),
            detail: fields.next().unwrap_or("").to_owned(),
        })
    }
}

/// The scenarios under `dir` whose names contain any of `filters` (all of
/// them without filters), sorted by name.
pub fn scenarios(dir: &Path, filters: &[String]) -> Result<Vec<PathBuf>> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .map(|entry| entry.map(|e| e.path()))
        .collect::<std::io::Result<_>>()?;
    paths.retain(|path| {
        path.extension().is_some_and(|e| e == "toml")
            && (filters.is_empty() || filters.iter().any(|f| name_of(path).contains(f.as_str())))
    });
    paths.sort();
    Ok(paths)
}

fn name_of(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

thread_local! {
    /// The last panic on this thread, as `at <file:line>: <message>`.
    static LAST_PANIC: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Gate every scenario with `jobs` worker threads; `progress` is called
/// after each one with the number done. A panicking gate is a `FAIL` row
/// with the panic's location, as `melee-sim gate` would print it.
pub fn run(
    paths: &[PathBuf],
    jobs: usize,
    particles: bool,
    progress: impl Fn(usize) + Sync,
) -> Vec<Row> {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|info| {
        let message = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_default();
        let at = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_default();
        LAST_PANIC.set(Some(format!("panicked at {at}: {message}")));
    }));
    let next = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let rows = Mutex::new(Vec::with_capacity(paths.len()));
    std::thread::scope(|scope| {
        for _ in 0..jobs.clamp(1, paths.len().max(1)) {
            scope.spawn(|| {
                while let Some(path) = paths.get(next.fetch_add(1, Ordering::Relaxed)) {
                    let row = gate_one(path, particles);
                    rows.lock().unwrap().push(row);
                    progress(done.fetch_add(1, Ordering::Relaxed) + 1);
                }
            });
        }
    });
    std::panic::set_hook(hook);
    let mut rows = rows.into_inner().unwrap();
    rows.sort_by(|a, b| a.name.cmp(&b.name));
    rows
}

fn gate_one(path: &Path, particles: bool) -> Row {
    let name = name_of(path);
    let outcome = std::panic::catch_unwind(|| -> Result<Option<String>> {
        let scenario = Scenario::load(path)?;
        Ok(if particles {
            Some(trace::gate_items_and_particles(&scenario)?.summary())
        } else {
            trace::gate_items(&scenario)?;
            None
        })
    });
    let row = |base: &str, particles: &str, detail: String| Row {
        name: name.clone(),
        base: base.into(),
        particles: particles.into(),
        detail: detail.replace(['\t', '\n'], " "),
    };
    match outcome {
        Ok(Ok(Some(summary))) if summary.contains(" 0 divergences") => row("PASS", "EXACT", summary),
        Ok(Ok(Some(summary))) => row("PASS", "NOTCMP", summary),
        Ok(Ok(None)) => row("PASS", "-", String::new()),
        Ok(Err(error)) => {
            let text = format!("{error:#}");
            if text.contains("phase particles") {
                let line = text
                    .lines()
                    .find(|l| l.contains("retail bank"))
                    .unwrap_or_else(|| text.lines().next().unwrap_or_default());
                row("PASS", "FAULT", line.to_owned())
            } else {
                row("FAIL", "-", text.lines().take(2).collect::<Vec<_>>().join(" "))
            }
        }
        Err(_) => {
            let panic = LAST_PANIC.take().unwrap_or_else(|| "panicked".into());
            row("FAIL", "-", panic)
        }
    }
}

/// Counts per (base, particles) status, as `sort | uniq -c` would print them.
pub fn summary(rows: &[Row]) -> String {
    let mut counts = std::collections::BTreeMap::new();
    for row in rows {
        *counts
            .entry((row.base.as_str(), row.particles.as_str()))
            .or_insert(0) += 1;
    }
    let mut out = String::new();
    for ((base, particles), count) in counts {
        let _ = writeln!(out, "{count:5} {base}\t{particles}");
    }
    out
}

/// Read a sweep TSV.
pub fn read(path: &Path) -> Result<Vec<Row>> {
    std::fs::read_to_string(path)
        .with_context(|| format!("reading {}", path.display()))?
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| Row::parse(l).with_context(|| format!("bad sweep row: {l}")))
        .collect()
}

/// Scenarios in both sweeps whose status changed. A regression is a scenario
/// that passed (or was particle-exact) before and no longer is.
#[derive(Debug, Default)]
pub struct Comparison {
    pub regressions: Vec<String>,
    pub fixed: Vec<String>,
}

pub fn compare(before: &[Row], after: &[Row]) -> Comparison {
    let before: std::collections::BTreeMap<&str, &Row> =
        before.iter().map(|r| (r.name.as_str(), r)).collect();
    let mut comparison = Comparison::default();
    for now in after {
        let Some(then) = before.get(now.name.as_str()) else {
            continue;
        };
        if then.base == "PASS" && now.base != "PASS" {
            comparison
                .regressions
                .push(format!("BASE REGRESSION\t{}\t{}", now.name, now.detail));
        } else if then.particles == "EXACT" && now.particles != "EXACT" {
            comparison
                .regressions
                .push(format!("PARTICLE REGRESSION\t{}\t{}", now.name, now.detail));
        } else if then.base != "PASS" && now.base == "PASS" {
            comparison.fixed.push(format!("base fixed\t{}", now.name));
        } else if then.particles != "EXACT" && now.particles == "EXACT" {
            comparison.fixed.push(format!("particles fixed\t{}", now.name));
        }
    }
    comparison
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(name: &str, base: &str, particles: &str) -> Row {
        Row {
            name: name.into(),
            base: base.into(),
            particles: particles.into(),
            detail: String::new(),
        }
    }

    #[test]
    fn comparison_reports_what_got_worse_and_what_got_fixed() {
        let before = [
            row("a", "PASS", "EXACT"),
            row("b", "PASS", "EXACT"),
            row("c", "FAIL", "-"),
            row("d", "PASS", "FAULT"),
            row("e", "PASS", "NOTCMP"),
        ];
        let after = [
            row("a", "FAIL", "-"),
            row("b", "PASS", "FAULT"),
            row("c", "PASS", "FAULT"),
            row("d", "PASS", "EXACT"),
            row("e", "PASS", "NOTCMP"),
            row("new", "FAIL", "-"),
        ];
        let comparison = compare(&before, &after);
        assert_eq!(
            comparison.regressions,
            ["BASE REGRESSION\ta\t", "PARTICLE REGRESSION\tb\t"]
        );
        assert_eq!(comparison.fixed, ["base fixed\tc", "particles fixed\td"]);
    }

    #[test]
    fn rows_round_trip_through_tsv() {
        let r = Row {
            detail: "particles: 300 ticks, 0 divergences".into(),
            ..row("x", "PASS", "EXACT")
        };
        assert_eq!(Row::parse(&r.tsv()), Some(r));
    }
}
