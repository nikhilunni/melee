//! The canonical 49-key scene snapshot and strict M3 comparison.
use crate::{
    frame::Simulation,
    initial_state::InitialState,
    inputs::PadScript,
    scenario::Scenario,
    schema::{Schema, SchemaCoverage},
};
use anyhow::{ensure, Result};
use melee_diff::{first_divergence, read_trace, Record};
use std::{
    collections::BTreeSet,
    fs::File,
    io::{BufRead, Write},
    path::Path,
};

pub fn check_schema(record: &Record) -> Result<()> {
    let schema = Schema::fighter_hand();
    let coverage = SchemaCoverage::new(&schema, &[]);
    let mut required = BTreeSet::from(["rng.seed".to_string()]);
    for player in 0..2 {
        let prefix = format!("p{player}.");
        let paths = record
            .state
            .keys()
            .filter_map(|key| key.strip_prefix(&prefix));
        let report = coverage.check_paths(paths);
        ensure!(report.is_ok() && report.extra.is_empty(), "{report}");
        required.extend(
            coverage
                .required()
                .into_iter()
                .map(|key| format!("{prefix}{key}")),
        );
    }
    ensure!(
        record.state.keys().cloned().collect::<BTreeSet<_>>() == required && required.len() == 49,
        "scene must emit exactly the 49 schema keys"
    );
    Ok(())
}
/// The per-tick pads recorded beside the scenario's expected trace. A
/// scripted scenario must carry them; a neutral one may predate the capture.
pub fn pad_script(scenario: &Scenario) -> Result<PadScript> {
    if scenario.is_cold() {
        if !scenario.replay_inputs.is_empty() {
            return PadScript::from_replay_inputs(
                &scenario.replay_inputs,
                scenario.frames as usize,
                &scenario.fighters.iter().map(|f| f.slot).collect::<Vec<_>>(),
            );
        }
        return Ok(PadScript::neutral(scenario.frames as usize));
    }
    let path = scenario.expected_path();
    let script = PadScript::from_expected_trace(&path, scenario.is_scripted())?;
    ensure!(
        script.len() as u64 == scenario.frames,
        "expected trace length {} differs from scenario {}",
        script.len(),
        scenario.frames
    );
    Ok(script)
}
/// A scripted simulation whose display passes follow another capture run.
pub fn simulation_displayed_as(scenario: &Scenario, capture: &Path) -> Result<Simulation> {
    let pads = pad_script(scenario)?.with_display_from(capture)?;
    Ok(Simulation::with_inputs(
        if scenario.is_cold() {
            InitialState::from_parameters(scenario)?
        } else {
            InitialState::from_savestate_traces(scenario)?
        },
        pads,
    ))
}
pub(crate) fn simulation(scenario: &Scenario) -> Result<Simulation> {
    let pads = pad_script(scenario)?;
    Ok(Simulation::with_inputs(
        if scenario.is_cold() {
            InitialState::from_parameters(scenario)?
        } else {
            InitialState::from_savestate_traces(scenario)?
        },
        pads,
    ))
}
pub fn write_run(scenario: &Scenario, mut out: impl Write) -> Result<()> {
    let mut simulation = simulation(scenario)?;
    if !scenario.replay_inputs.is_empty() {
        simulation.tick()?;
    }
    for frame in 0..scenario.frames {
        let mut record = simulation.tick()?;
        record.frame = frame;
        check_schema(&record)?;
        serde_json::to_writer(&mut out, &record)?;
        writeln!(out)?;
    }
    out.flush()?;
    Ok(())
}
/// Run `scenario`'s tick-clock inputs from `reference`'s imported savestate
/// (a recording of the same boundary), with item state beside each record:
/// a port-only search for inputs worth recording. Nothing is compared.
pub fn write_dry_run(scenario: &Scenario, reference: &Scenario, mut out: impl Write) -> Result<()> {
    ensure!(
        scenario.input_clock.as_deref() == Some("tick"),
        "a dry run needs tick-clock inputs"
    );
    ensure!(
        scenario.savestate == reference.savestate,
        "the reference must start from the same savestate"
    );
    let pads = PadScript::from_tick_schedule(&scenario.inputs, scenario.frames as usize)?
        .with_display_from(&reference.expected_path())?;
    let mut simulation =
        Simulation::with_inputs(InitialState::from_savestate_traces(reference)?, pads);
    for frame in 0..scenario.frames {
        let mut record = simulation.tick()?;
        record.frame = frame;
        record.state.extend(simulation.item_snapshot(frame).state);
        serde_json::to_writer(&mut out, &record)?;
        writeln!(out)?;
    }
    out.flush()?;
    Ok(())
}
/// Unlike first_divergence alone, this also rejects extra keys/records.
pub fn gate(scenario: &Scenario) -> Result<()> {
    gate_with_recording(scenario, false, false)?;
    Ok(())
}

/// Extend the existing fighter gate with every recorded item key in list order.
pub fn gate_items(scenario: &Scenario) -> Result<()> {
    gate_with_recording(scenario, false, true)?;
    Ok(())
}

pub fn compared_keys(scenario: &Scenario) -> Result<usize> {
    let first = melee_trace_io::open(&scenario.expected_path())?
        .lines()
        .next()
        .transpose()?
        .ok_or_else(|| anyhow::anyhow!("empty expected trace"))?;
    let json: serde_json::Value = serde_json::from_str(&first)?;
    Ok(49
        + if json.get("items").is_some() {
            crate::trace_items::KEYS.len() + 1
        } else {
            0
        })
}

/// Record caller inputs during the exact gate loop; no output file is opened
/// until all comparisons succeed, so a failed import/gate preserves its target.
pub fn fixture_spawns(
    scenario: &Scenario,
    out: &std::path::Path,
    ticks: Option<u64>,
) -> Result<()> {
    let ticks = ticks.unwrap_or(scenario.frames);
    ensure!(
        ticks > 0 && ticks <= scenario.frames,
        "fixture ticks must be within the scenario"
    );
    let mut events = gate_with_recording(scenario, true, true)?;
    events.retain(|frame, _| *frame < ticks);
    let mut out = std::io::BufWriter::new(File::create(out)?);
    serde_json::to_writer_pretty(&mut out, &events)?;
    writeln!(out)?;
    out.flush()?;
    Ok(())
}

fn gate_with_recording(
    scenario: &Scenario,
    record_spawns: bool,
    compare_items: bool,
) -> Result<std::collections::BTreeMap<u64, Vec<serde_json::Value>>> {
    let expected = read_trace(melee_trace_io::open(&scenario.expected_path())?)?;
    let item_rows = if compare_items {
        melee_trace_io::open(&scenario.expected_path())?
            .lines()
            .enumerate()
            .map(|(frame, line)| {
                crate::trace_items::expected(
                    &serde_json::from_str::<serde_json::Value>(&line?)?,
                    frame as u64,
                )
            })
            .collect::<Result<Vec<_>>>()?
    } else {
        vec![None; expected.len()]
    };
    ensure!(
        expected.len() as u64 == scenario.frames,
        "expected trace length {} differs from scenario {}",
        expected.len(),
        scenario.frames
    );
    let mut simulation = simulation(scenario)?;
    if !scenario.replay_inputs.is_empty() {
        simulation.tick()?;
    }
    if record_spawns {
        simulation.enable_spawn_recording();
    }
    for (expected, expected_items) in expected.into_iter().zip(item_rows) {
        let mut actual = simulation.tick()?;
        if !scenario.replay_inputs.is_empty() {
            actual.frame -= 1;
        }
        check_schema(&expected)?;
        check_schema(&actual)?;
        if let Some(diff) = first_divergence([&expected], [&actual]) {
            anyhow::bail!(
                "{diff}\n{} ticks matched; RNG-writing procs this tick: {:?}",
                actual.frame,
                simulation.rng_writers()
            );
        }
        if let Some(expected_items) = expected_items {
            let actual_items = simulation.item_snapshot(actual.frame);
            if let Some(diff) = first_divergence([&expected_items], [&actual_items]) {
                anyhow::bail!("{diff}\n{} ticks matched (item list order)", actual.frame);
            }
        }
    }
    Ok(if record_spawns {
        let offset = u64::from(!scenario.replay_inputs.is_empty());
        simulation
            .finish_spawn_recording()
            .into_iter()
            .map(|(frame, events)| (frame - offset, events))
            .collect()
    } else {
        std::collections::BTreeMap::new()
    })
}

/// Particle RNG call sites per tick, port versus the retail ledger
/// (`<name>.ledger.raw.jsonl`), for ticks `from..=to`: one line per tick
/// where they differ.
pub fn particle_site_diff(scenario: &Scenario, from: u64, to: u64) -> Result<Vec<String>> {
    let ledger_path = scenario.trace_path("ledger.raw.jsonl");
    let ledger = melee_trace_io::read_to_string(&ledger_path)?;
    let mut simulation = simulation_displayed_as(scenario, &ledger_path)?;
    let mut report = Vec::new();
    for (tick, line) in ledger.lines().enumerate().take(to as usize + 1) {
        let row: serde_json::Value = serde_json::from_str(line)?;
        let expected: Vec<u32> = row["rng_draws"]
            .as_array()
            .map(|draws| {
                draws
                    .iter()
                    .filter_map(|draw| draw["lr"].as_u64())
                    .map(|lr| lr as u32 - 4)
                    .filter(|site| (0x8039_8f8c..0x8039_f6cc).contains(site))
                    .collect()
            })
            .unwrap_or_default();
        simulation.tick_without_snapshot()?;
        let actual = simulation.particle_rng_sites();
        if tick as u64 >= from && actual != expected {
            let hex = |sites: &[u32]| {
                sites
                    .iter()
                    .map(|s| format!("{s:08X}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            report.push(format!(
                "tick {tick}: port [{}] retail [{}]",
                hex(&actual),
                hex(&expected)
            ));
        }
    }
    Ok(report)
}

/// The first differing particle-system field per tick against the retail
/// particle dump (`<name>.particles.jsonl`), for ticks `from..=to`, apart
/// from the AppSRT display caches.
pub fn particle_state_diff(scenario: &Scenario, from: u64, to: u64) -> Result<Vec<String>> {
    let path = scenario.trace_path("particles.jsonl");
    let expected = read_trace(melee_trace_io::open(&path)?)?;
    // The dump's per-tick VI frames live in its metadata sidecar.
    let meta = scenario.trace_path("particles.jsonl.meta.jsonl");
    let mut simulation = simulation_displayed_as(scenario, &meta)?;
    let mut report = Vec::new();
    for record in expected.iter().take_while(|r| r.frame <= to) {
        simulation.tick_without_snapshot()?;
        if record.frame < from {
            continue;
        }
        let actual = simulation.particle_snapshot(record.frame);
        // AppSRT display caches need the camera and render schedule.
        let mismatch = record
            .state
            .iter()
            .filter(|(key, _)| !key.starts_with("particles.appsrt["))
            .find(|(key, value)| actual.state.get(*key) != Some(value));
        if let Some((key, value)) = mismatch {
            report.push(format!(
                "tick {}: {key}: retail {value:?}, port {:?}",
                record.frame,
                actual.state.get(key)
            ));
            if std::env::var_os("MELEE_PARTICLE_ALL").is_some() {
                for (key, value) in record.state.iter() {
                    if actual.state.get(key) != Some(value) {
                        report.push(format!(
                            "    {key}: retail {value:?}, port {:?}",
                            actual.state.get(key)
                        ));
                    }
                }
            }
            if std::env::var_os("MELEE_PARTICLE_LISTS").is_some() {
                let list = |state: &std::collections::BTreeMap<String, melee_diff::Value>| {
                    (0..64)
                        .map_while(|i| {
                            let kind =
                                state.get(&format!("particles.generator[{i}].program_kind"))?;
                            let id = state.get(&format!("particles.generator[{i}].id"));
                            Some(format!("{kind:?}/{id:?}"))
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                report.push(format!("  retail: {}", list(&record.state)));
                report.push(format!("  port:   {}", list(&actual.state)));
            }
        }
    }
    Ok(report)
}
