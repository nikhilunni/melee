//! The gate's particle check: a per-tick digest of the port's particle
//! system against the retail particle dump (`<name>.particles.jsonl`).
//!
//! A dump is large (about 100 kB of JSON per tick), so the gate never holds
//! one. Each dump is read once, as a stream, into a small sidecar beside it
//! (`sidecar`). The gate then digests the port's particle system after
//! every tick (`digest`) and compares one number; only at a difference does
//! it read the dump's record of that tick, to name the first field.
//!
//! The digest covers every key of the dump except the AppSRT display caches
//! (`particles.appsrt[i]`): the generators in list order with their fields,
//! the sixteen particle lists in order with every particle's fields, the
//! pending list, the counts, the family counter and the seed.
//!
//! A dump is its own Dolphin run, so how it is used is decided per scenario
//! ([`ParticleCheck::open`]):
//!
//! - It is compared only if it is the game the tick trace recorded: equal
//!   seeds on every record, and every record its pass was asked for.
//! - particleSort runs in the display pass, so the order of a particle list
//!   follows the display cadence of the run, and each run has its own (VI
//!   timing). Where the dump's display clock differs from the tick trace's,
//!   the check steps a second simulation under the dump's clock beside the
//!   gate's and compares that one.
//! - A dump whose tracer recorded no display clock (before 2026-09-26) has
//!   an unknown cadence. Its lists are compared as bags: every particle's
//!   fields, but not its place in its list.
use crate::{frame::Simulation, inputs::PadScript, scenario::Scenario};
use anyhow::{bail, Context, Result};
use melee_diff::{Divergence, Record, Value};
use melee_lib::diagnostics::{particle_key, ParticleScope};
use std::{collections::BTreeMap, io::BufRead};

mod digest;
mod sidecar;

pub use digest::{record_digests, Digests, PortDigest};
use digest::{parse_key, APPSRT_PREFIX};
use sidecar::DumpTick;

/// What the gate does with a scenario's particle dump.
pub enum DumpUse {
    /// No dump was recorded: the scenario gates without particles.
    Absent,
    /// The dump is not the tick trace's game; it is not compared, and why.
    Inconsistent(String),
    /// Every record of the dump is compared.
    Compared(ParticleCheck),
}

impl DumpUse {
    /// One line for the gate's report.
    pub fn summary(&self) -> String {
        match self {
            DumpUse::Absent => "particles: no dump, not compared".into(),
            DumpUse::Inconsistent(why) => format!("particles: dump not compared: {why}"),
            DumpUse::Compared(check) => {
                let mut notes = Vec::from_iter(check.shortened.clone());
                if check.own_cadence.is_some() {
                    notes.push("a second run under the dump's display cadence".into());
                }
                if !check.ordered {
                    notes.push("no display clock in the dump: lists compared as bags".into());
                }
                let notes = match notes.is_empty() {
                    true => String::new(),
                    false => format!(" ({})", notes.join("; ")),
                };
                format!(
                    "particles: {} ticks, 0 divergences{notes}",
                    check.ticks.len()
                )
            }
        }
    }
}

/// The per-tick comparison of one scenario's run with its dump.
pub struct ParticleCheck {
    ticks: Vec<DumpTick>,
    port: PortDigest,
    /// Why fewer ticks than the scenario's are compared.
    shortened: Option<String>,
    /// The run under the dump's display clock, where that differs from the
    /// tick trace's; stepped once per compared tick.
    own_cadence: Option<Box<Simulation>>,
    /// Whether a particle's place in its list is compared: the dump
    /// recorded its display clock.
    ordered: bool,
}

/// A particle difference with both sides' records of the tick (without the
/// AppSRT display caches).
pub struct ParticleDivergence {
    pub first: Divergence,
    /// The differing object's bank, program and family id on both sides.
    pub object: String,
    pub expected: Record,
    pub actual: Record,
}

/// The `.done` marker of the dump's pass (tick_trace.py's summary).
#[derive(serde::Deserialize)]
struct PassSummary {
    ticks: u64,
    #[serde(default)]
    ended_early: bool,
}

/// Whether a display pass precedes record `tick` of a run with this clock,
/// by `PadScript::display_pass`'s rule.
fn dump_display_pass(ticks: &[DumpTick], tick: usize) -> bool {
    if tick == 0 {
        return true;
    }
    match (ticks[tick - 1].display_clock, ticks[tick].display_clock) {
        (Some(previous), Some(current)) => current != previous,
        _ => true,
    }
}

impl ParticleCheck {
    /// Decide how `scenario`'s dump is used by a gate that runs `pads` and
    /// expects `seeds` (the tick trace's end-of-tick seeds). `record.py`
    /// accepts a dump on the same terms: equal seeds on every record, and
    /// every record it was asked for.
    pub fn open(scenario: &Scenario, pads: &PadScript, seeds: &[u32]) -> Result<DumpUse> {
        let Some(mut ticks) = sidecar::dump_ticks(scenario)? else {
            return Ok(DumpUse::Absent);
        };
        ticks.truncate(seeds.len());
        if let Some(tick) = (0..ticks.len()).find(|&t| ticks[t].seed != seeds[t]) {
            return Ok(DumpUse::Inconsistent(format!(
                "its pass is not the tick trace's game: its seed differs from record {tick} \
                 ({:#010x}, the tick trace has {:#010x})",
                ticks[tick].seed, seeds[tick]
            )));
        }
        let mut shortened = None;
        if ticks.len() < seeds.len() {
            // A pass capped with `record.py --particle-ticks` is whole; one
            // whose match ended, or that Dolphin left, is not.
            let done = scenario.trace_path("particles.jsonl.done");
            let summary: Option<PassSummary> = std::fs::read_to_string(done)
                .ok()
                .and_then(|text| serde_json::from_str(&text).ok());
            match summary {
                Some(pass) if pass.ticks == ticks.len() as u64 && !pass.ended_early => {
                    shortened = Some(format!("the dump was capped at {} ticks", pass.ticks));
                }
                _ => {
                    return Ok(DumpUse::Inconsistent(format!(
                        "its pass wrote {} records of {}",
                        ticks.len(),
                        seeds.len()
                    )))
                }
            }
        }
        let ordered = ticks.iter().all(|tick| tick.display_clock.is_some());
        // The simulation runs one tick ahead of a cold replay's records.
        let lead = usize::from(!scenario.replay_inputs.is_empty());
        let same_cadence = (0..ticks.len())
            .all(|t| dump_display_pass(&ticks, t) == pads.display_pass((t + lead) as u64));
        let own_cadence = if same_cadence || !ordered {
            None
        } else {
            let clock = std::iter::repeat_n(None, lead)
                .chain(ticks.iter().map(|tick| tick.display_clock))
                .collect();
            let mut simulation =
                crate::trace::simulation_with(scenario, pads.clone().with_display_clock(clock))?;
            if lead != 0 {
                simulation.tick()?;
            }
            Some(Box::new(simulation))
        };
        Ok(DumpUse::Compared(ParticleCheck {
            ticks,
            port: PortDigest::default(),
            shortened,
            own_cadence,
            ordered,
        }))
    }

    /// Compare the dump's record `frame` with the port. Called once per
    /// tick, in order, when the gate's `simulation` has finished that tick.
    pub fn divergence(
        &mut self,
        scenario: &Scenario,
        frame: u64,
        simulation: &Simulation,
    ) -> Result<Option<ParticleDivergence>> {
        let Some(expected) = self.ticks.get(frame as usize) else {
            return Ok(None);
        };
        let simulation = match &mut self.own_cadence {
            Some(own) => {
                own.tick_without_snapshot()
                    .context("the run under the particle dump's display cadence")?;
                &**own
            }
            None => simulation,
        };
        let port = self.port.of(simulation);
        let equal = match self.ordered {
            true => port.ordered == expected.digests.ordered,
            false => port.unordered == expected.digests.unordered,
        };
        if equal {
            return Ok(None);
        }
        difference(scenario, frame, simulation, self.ordered).map(Some)
    }
}

/// The dump's record of `frame`, read as a stream up to it.
fn dump_record(scenario: &Scenario, frame: u64) -> Result<Record> {
    let path = scenario.trace_path("particles.jsonl");
    let line = melee_trace_io::open(&path)?
        .lines()
        .filter(|line| line.as_ref().map_or(true, |l| !l.trim().is_empty()))
        .nth(frame as usize)
        .with_context(|| format!("{}: no record {frame}", path.display()))??;
    Ok(serde_json::from_str(&line)?)
}

/// Drop the keys the digests leave out.
fn compared_keys(mut record: Record) -> Record {
    record
        .state
        .retain(|key, _| !key.starts_with(APPSRT_PREFIX));
    record
}

/// A differing key, the object it belongs to on the port's side when that
/// is not the key's own, and both values.
struct Difference {
    path: String,
    port_object: Option<String>,
    expected: Option<Value>,
    actual: Option<Value>,
}

/// The first differing key of two records: the dump's keys in order, then
/// a key only the port holds.
fn first_differing_key<'a>(
    mut expected: impl Iterator<Item = (&'a String, &'a Value)>,
    actual: &BTreeMap<String, Value>,
    mut actual_keys: impl Iterator<Item = &'a String>,
    expected_state: &BTreeMap<String, Value>,
) -> Option<Difference> {
    let path = expected
        .find(|(key, value)| actual.get(*key) != Some(value))
        .map(|(key, _)| key)
        .or_else(|| actual_keys.find(|key| !expected_state.contains_key(*key)))?;
    Some(Difference {
        path: path.clone(),
        port_object: None,
        expected: expected_state.get(path).cloned(),
        actual: actual.get(path).cloned(),
    })
}

/// A record's particles: list, place in the list, then each field's name
/// and component.
type Particles<'a> = BTreeMap<usize, BTreeMap<usize, BTreeMap<(&'a str, Option<usize>), &'a Value>>>;

fn particles(record: &Record) -> Particles<'_> {
    let mut lists = Particles::new();
    for (key, value) in &record.state {
        if let Some((ParticleScope::Particle { link, index }, name, component)) = parse_key(key) {
            lists
                .entry(link)
                .or_default()
                .entry(index)
                .or_default()
                .insert((name, component), value);
        }
    }
    lists
}

fn is_particle_key(key: &str) -> bool {
    matches!(
        parse_key(key),
        Some((ParticleScope::Particle { .. }, _, _))
    )
}

/// The first difference of two records whose particle lists are bags:
/// a key that is not a particle's, else the dump's first particle that no
/// particle of the port's list equals, against the port's nearest one.
fn unordered_difference(expected: &Record, actual: &Record) -> Option<Difference> {
    let other = first_differing_key(
        expected.state.iter().filter(|(key, _)| !is_particle_key(key)),
        &actual.state,
        actual.state.keys().filter(|key| !is_particle_key(key)),
        &expected.state,
    );
    if other.is_some() {
        return other;
    }
    let (expected_lists, mut actual_lists) = (particles(expected), particles(actual));
    for (link, list) in &expected_lists {
        let mut pool: Vec<_> = actual_lists
            .remove(link)
            .unwrap_or_default()
            .into_iter()
            .collect();
        for (index, fields) in list {
            if let Some(found) = pool.iter().position(|(_, other)| other == fields) {
                pool.swap_remove(found);
                continue;
            }
            let shared = |other: &BTreeMap<(&str, Option<usize>), &Value>| {
                fields
                    .iter()
                    .filter(|(field, value)| other.get(*field) == Some(*value))
                    .count()
            };
            let nearest = pool.iter().max_by_key(|(_, other)| shared(other));
            let field = fields
                .iter()
                .find(|(field, value)| {
                    nearest.and_then(|(_, other)| other.get(*field)) != Some(*value)
                })
                .map(|(field, _)| *field)?;
            let scope = ParticleScope::Particle {
                link: *link,
                index: *index,
            };
            return Some(Difference {
                path: particle_key(scope, field.0, field.1),
                port_object: nearest
                    .map(|(place, _)| format!("particles.link[{link}].particle[{place}]")),
                expected: fields.get(&field).map(|value| (*value).clone()),
                actual: nearest
                    .and_then(|(_, other)| other.get(&field))
                    .map(|value| (*value).clone()),
            });
        }
    }
    None
}

/// The first differing field between the dump and the port at `frame`.
fn difference(
    scenario: &Scenario,
    frame: u64,
    simulation: &Simulation,
    ordered: bool,
) -> Result<ParticleDivergence> {
    let expected = compared_keys(dump_record(scenario, frame)?);
    let actual = compared_keys(simulation.particle_snapshot(frame));
    let found = match ordered {
        true => first_differing_key(
            expected.state.iter(),
            &actual.state,
            actual.state.keys(),
            &expected.state,
        ),
        false => unordered_difference(&expected, &actual),
    };
    let Some(found) = found else {
        bail!(
            "tick {frame}: the particle digests differ but no field does ({:x?} for the dump's \
             record, {:x?} for the port's)",
            record_digests(&expected)?,
            record_digests(&actual)?
        );
    };
    let object = describe(&found, &expected, &actual);
    Ok(ParticleDivergence {
        first: Divergence {
            frame,
            phase: "particles".into(),
            path: found.path,
            expected: found.expected,
            actual: found.actual,
        },
        object,
        expected,
        actual,
    })
}

/// What a differing particle key belongs to, for a fault list: the
/// generator's or particle's bank, program and family id on both sides.
fn describe(found: &Difference, expected: &Record, actual: &Record) -> String {
    let Some(end) = found.path.rfind("].") else {
        return "(no generator or particle: a count or the seed)".into();
    };
    let object = &found.path[..=end];
    let port_object = found.port_object.as_deref().unwrap_or(object);
    let side = |record: &Record, object: &str| {
        let field = |name: &str| match record.state.get(&format!("{object}.{name}")) {
            Some(Value::UInt(value)) => value.to_string(),
            Some(Value::Null) => "null".into(),
            _ => "-".into(),
        };
        format!(
            "bank {} program {} id {}",
            field("bank"),
            field("program_kind"),
            field("id")
        )
    };
    let nearest = match &found.port_object {
        Some(place) => format!(" (the nearest of its list, {place})"),
        None => String::new(),
    };
    format!(
        "{object}: retail {}, port {}{nearest}",
        side(expected, object),
        side(actual, port_object)
    )
}

/// A record's generators and particle lists in order, one line each: every
/// entry is `program.family id`, with the bank before a colon where it is
/// not the common bank 0, and a particle's remaining life in brackets.
fn lists(record: &Record) -> String {
    let entry = |object: &str, life: bool| {
        let field = |name: &str| match record.state.get(&format!("{object}.{name}")) {
            Some(Value::UInt(value)) => value.to_string(),
            _ => "-".into(),
        };
        let bank = match field("bank").as_str() {
            "0" => String::new(),
            bank => format!("{bank}:"),
        };
        let life = match life {
            true => format!("[{}]", field("life")),
            false => String::new(),
        };
        format!("{bank}{}.{}{life}", field("program_kind"), field("id"))
    };
    let count = |key: &str| match record.state.get(key) {
        Some(Value::UInt(count)) => *count,
        _ => 0,
    };
    let mut text = String::from("    generators:");
    for index in 0..count("particles.generator_count") {
        text.push(' ');
        text.push_str(&entry(&format!("particles.generator[{index}]"), false));
    }
    for link in 0..16 {
        let particles = count(&format!("particles.link[{link}].count"));
        if particles == 0 {
            continue;
        }
        text.push_str(&format!("\n    link {link}:"));
        for index in 0..particles {
            text.push(' ');
            text.push_str(&entry(
                &format!("particles.link[{link}].particle[{index}]"),
                true,
            ));
        }
    }
    text
}

impl ParticleDivergence {
    /// Both sides' generator and particle lists in order.
    pub fn lists(&self) -> String {
        format!(
            "retail:\n{}\nport:\n{}",
            lists(&self.expected),
            lists(&self.actual)
        )
    }
}

impl std::fmt::Display for ParticleDivergence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}\n{}", self.first, self.object)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(fields: &[(&str, Value)]) -> Record {
        Record {
            frame: 0,
            phase: "particles".into(),
            state: fields
                .iter()
                .map(|(key, value)| (key.to_string(), value.clone()))
                .collect(),
        }
    }

    #[test]
    fn bags_differ_at_the_first_particle_without_an_equal_in_its_list() {
        let retail = record(&[
            ("particles.link[0].count", Value::UInt(2)),
            ("particles.link[0].particle[0].life", Value::UInt(4)),
            ("particles.link[0].particle[0].size", Value::f32(1.0)),
            ("particles.link[0].particle[1].life", Value::UInt(19)),
            ("particles.link[0].particle[1].size", Value::f32(2.0)),
        ]);
        let swapped = record(&[
            ("particles.link[0].count", Value::UInt(2)),
            ("particles.link[0].particle[0].life", Value::UInt(19)),
            ("particles.link[0].particle[0].size", Value::f32(2.0)),
            ("particles.link[0].particle[1].life", Value::UInt(4)),
            ("particles.link[0].particle[1].size", Value::f32(1.0)),
        ]);
        assert!(unordered_difference(&retail, &swapped).is_none());
        assert_eq!(
            record_digests(&retail).unwrap().unordered,
            record_digests(&swapped).unwrap().unordered
        );

        let mut resized = swapped.clone();
        resized.state.insert(
            "particles.link[0].particle[0].size".into(),
            Value::f32(2.5),
        );
        let found = unordered_difference(&retail, &resized).unwrap();
        assert_eq!(found.path, "particles.link[0].particle[1].size");
        assert_eq!(
            found.port_object.as_deref(),
            Some("particles.link[0].particle[0]")
        );
        assert_eq!(found.expected, Some(Value::f32(2.0)));
        assert_eq!(found.actual, Some(Value::f32(2.5)));
    }

    #[test]
    fn bags_still_compare_every_key_that_is_not_a_particles() {
        let retail = record(&[
            ("particles.generator[0].count", Value::f32(0.5)),
            ("particles.link[0].count", Value::UInt(0)),
        ]);
        let port = record(&[
            ("particles.generator[0].count", Value::f32(0.75)),
            ("particles.link[0].count", Value::UInt(0)),
        ]);
        let found = unordered_difference(&retail, &port).unwrap();
        assert_eq!(found.path, "particles.generator[0].count");
    }

    #[test]
    fn an_ordered_difference_names_a_key_only_the_port_holds() {
        let retail = record(&[("rng.seed", Value::UInt(1))]);
        let port = record(&[
            ("rng.seed", Value::UInt(1)),
            ("particles.generator[0].aux.mode", Value::UInt(6)),
        ]);
        let found = first_differing_key(
            retail.state.iter(),
            &port.state,
            port.state.keys(),
            &retail.state,
        )
        .unwrap();
        assert_eq!(found.path, "particles.generator[0].aux.mode");
        assert_eq!(found.expected, None);
    }
}
