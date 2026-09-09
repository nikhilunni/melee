//! The canonical 49-key scene snapshot and strict M3 comparison.
use crate::{
    frame::Simulation,
    initial_state::InitialState,
    inputs::PadScript,
    scenario::Scenario,
    schema::{Schema, SchemaCoverage},
};
use anyhow::{ensure, Result};
use melee_diff::{first_divergence, read_trace, Record, RecordSink};
use melee_types::snapshot::{PrefixSink, Snapshot, SnapshotSink};
use std::{
    collections::BTreeSet,
    fs::File,
    io::{BufReader, Write},
};

impl Snapshot for InitialState {
    fn snapshot(&self, sink: &mut dyn SnapshotSink) {
        sink.field("rng.seed", &self.rng.seed);
        for (player, fighter) in self.fighters.iter().enumerate() {
            fighter.snapshot(&mut PrefixSink::new(sink, &format!("p{player}")));
        }
    }
}
pub(crate) fn snapshot(state: &InitialState, frame: u64) -> Record {
    let mut sink = RecordSink::new(frame, "frame_end");
    state.snapshot(&mut sink);
    sink.finish()
}
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
fn simulation(scenario: &Scenario) -> Result<Simulation> {
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
/// Unlike first_divergence alone, this also rejects extra keys/records.
pub fn gate(scenario: &Scenario) -> Result<()> {
    let expected = read_trace(BufReader::new(File::open(scenario.expected_path())?))?;
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
    for expected in expected {
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
    }
    Ok(())
}
