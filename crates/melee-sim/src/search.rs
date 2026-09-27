//! `melee-sim search`: find tick-clock inputs that make the port reach a goal.
//!
//! A spec (TOML) lists input edits to vary and a goal over trace keys:
//!
//! ```toml
//! max_ticks = 400        # stop a candidate at this tick (default: the scenario's frames)
//! margin = 40            # ticks kept after the goal in the written scenario
//! limit = 5              # stop after this many successes
//!
//! [[vary]]
//! name = "jump"
//! port = 1
//! at = [120, 140]        # inclusive tick range
//! raw = { button = 1024 }
//! hold = 1               # release (neutral pad) after 1 tick; omit to persist
//!
//! [[vary]]
//! port = 1
//! after = "jump"         # relative to another edit's tick
//! offset = [2, 12]
//! raw = [{ button = 256, stickY = -127 }, { button = 256, stickY = 127 }]
//! hold = 2
//!
//! [goal]
//! reach = ["p1.motion_id == JumpF", "p0.motion_id == 82"]   # in order
//! avoid = ["p1.motion_id == DeadDown"]
//! ```
//!
//! Edits merge after the scenario's own inputs (a same-tick edit wins). The
//! search simulates each shared prefix once and clones the match at every
//! branch point, so candidates cost only the ticks after their first edit.
//! The port is the oracle here: a found recipe still needs a retail recording.
use crate::{
    frame::Simulation,
    initial_state::InitialState,
    inputs::PadScript,
    scenario::{InputStep, Scenario},
};
use anyhow::{bail, ensure, Context, Result};
use melee_diff::{Record, Value};
use serde::Deserialize;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    #[serde(default)]
    vary: Vec<Vary>,
    goal: Goal,
    max_ticks: Option<u64>,
    #[serde(default = "default_margin")]
    margin: u64,
    #[serde(default = "default_limit")]
    limit: usize,
}
fn default_margin() -> u64 {
    40
}
fn default_limit() -> usize {
    1
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Vary {
    name: Option<String>,
    port: u8,
    at: Option<[u64; 2]>,
    after: Option<String>,
    offset: Option<[i64; 2]>,
    raw: RawChoice,
    hold: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawChoice {
    One(toml::Table),
    Many(Vec<toml::Table>),
}
impl RawChoice {
    fn options(&self) -> Vec<&toml::Table> {
        match self {
            Self::One(t) => vec![t],
            Self::Many(v) => v.iter().collect(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Goal {
    reach: Vec<String>,
    #[serde(default)]
    avoid: Vec<String>,
}

/// `<key> <op> <value>`: the value is a number or a common motion name.
#[derive(Clone, Debug)]
struct Condition {
    key: String,
    op: Op,
    value: f64,
}
#[derive(Clone, Copy, Debug)]
enum Op {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}
impl Condition {
    fn parse(text: &str) -> Result<Self> {
        let parts: Vec<&str> = text.split_whitespace().collect();
        let [key, op, value] = parts[..] else {
            bail!("condition {text:?}: expected `<key> <op> <value>`");
        };
        let op = match op {
            "==" => Op::Eq,
            "!=" => Op::Ne,
            "<" => Op::Lt,
            "<=" => Op::Le,
            ">" => Op::Gt,
            ">=" => Op::Ge,
            _ => bail!("condition {text:?}: unknown operator {op}"),
        };
        let value = match value.parse::<f64>() {
            Ok(v) => v,
            Err(_) => melee_types::CommonMotionState::ALL
                .iter()
                .find(|m| format!("{m:?}") == value)
                .map(|&m| f64::from(i32::from(m)))
                .with_context(|| {
                    format!("condition {text:?}: {value} is not a number or motion")
                })?,
        };
        Ok(Self {
            key: key.to_string(),
            op,
            value,
        })
    }
    fn holds(&self, record: &Record) -> bool {
        let Some(actual) = record.state.get(&self.key).and_then(number) else {
            return false;
        };
        match self.op {
            Op::Eq => actual == self.value,
            Op::Ne => actual != self.value,
            Op::Lt => actual < self.value,
            Op::Le => actual <= self.value,
            Op::Gt => actual > self.value,
            Op::Ge => actual >= self.value,
        }
    }
}
fn number(value: &Value) -> Option<f64> {
    match value {
        Value::Int(v) => Some(*v as f64),
        Value::UInt(v) => Some(*v as f64),
        Value::F32 { approx, .. } | Value::F64 { approx, .. } => Some(*approx),
        Value::Str(_) | Value::Null => None,
    }
}

/// One chosen input edit: a pad from `tick`, released after `hold` ticks.
#[derive(Clone, Debug)]
pub struct Edit {
    pub tick: u64,
    pub port: u8,
    pub raw: toml::Table,
    pub hold: Option<u64>,
}
impl Edit {
    fn steps(&self) -> Vec<InputStep> {
        let mut steps = vec![InputStep {
            frame: self.tick,
            port: self.port,
            buttons: toml::Table::new(),
            raw: self.raw.clone(),
        }];
        if let Some(hold) = self.hold {
            steps.push(InputStep {
                frame: self.tick + hold,
                port: self.port,
                buttons: toml::Table::new(),
                raw: toml::Table::new(),
            });
        }
        steps
    }
}

/// A candidate that reached every goal condition, with the tick of each.
#[derive(Clone, Debug)]
pub struct Found {
    pub edits: Vec<Edit>,
    pub reached: Vec<u64>,
}

struct Search<'a> {
    spec: &'a Spec,
    base: &'a [InputStep],
    reach: Vec<Condition>,
    avoid: Vec<Condition>,
    needs_items: bool,
    max_ticks: u64,
    display: PadScript,
    found: Mutex<Vec<Found>>,
    tried: std::sync::atomic::AtomicUsize,
    done: AtomicBool,
}

impl Search<'_> {
    fn pads(&self, edits: &[Edit]) -> Result<PadScript> {
        let mut steps: Vec<InputStep> = self.base.to_vec();
        for edit in edits {
            steps.extend(edit.steps());
        }
        Ok(
            PadScript::from_tick_schedule(&steps, self.max_ticks as usize)?
                .with_display_of(&self.display),
        )
    }
    /// The earliest tick an unchosen edit can take, given the chosen ones.
    fn lower_bound(&self, level: usize, chosen: &[Edit]) -> u64 {
        let vary = &self.spec.vary[level];
        if let Some([lo, _]) = vary.at {
            return lo;
        }
        let parent = self.parent(vary).expect("validated");
        let base = chosen
            .get(parent)
            .map_or_else(|| self.lower_bound(parent, chosen), |e| e.tick);
        (base as i64 + vary.offset.unwrap_or([0, 0])[0]).max(0) as u64
    }
    fn range(&self, level: usize, chosen: &[Edit]) -> (u64, u64) {
        let vary = &self.spec.vary[level];
        if let Some([lo, hi]) = vary.at {
            return (lo, hi);
        }
        let parent = chosen[self.parent(vary).expect("validated")].tick as i64;
        let [lo, hi] = vary.offset.unwrap_or([0, 0]);
        ((parent + lo).max(0) as u64, (parent + hi).max(0) as u64)
    }
    fn parent(&self, vary: &Vary) -> Option<usize> {
        let name = vary.after.as_ref()?;
        self.spec
            .vary
            .iter()
            .position(|v| v.name.as_deref() == Some(name))
    }

    /// Advance `sim` to the earliest remaining edit, then branch on each
    /// choice for edit `level`.
    fn branch(&self, level: usize, mut sim: Simulation, chosen: Vec<Edit>) -> Result<()> {
        if self.done.load(Ordering::Relaxed) {
            return Ok(());
        }
        if level == self.spec.vary.len() {
            return self.finish(sim, chosen);
        }
        let until = (level..self.spec.vary.len())
            .map(|l| self.lower_bound(l, &chosen))
            .min()
            .unwrap_or(self.max_ticks);
        while sim.frame() < until.min(self.max_ticks) {
            sim.tick_without_snapshot()?;
        }
        let (lo, hi) = self.range(level, &chosen);
        let vary = &self.spec.vary[level];
        for tick in lo.max(sim.frame())..=hi.min(self.max_ticks) {
            for raw in vary.raw.options() {
                let mut edits = chosen.clone();
                edits.push(Edit {
                    tick,
                    port: vary.port,
                    raw: raw.clone(),
                    hold: vary.hold,
                });
                let mut child = sim.clone();
                child.set_pads(self.pads(&edits)?);
                self.branch(level + 1, child, edits)?;
            }
        }
        Ok(())
    }

    fn finish(&self, mut sim: Simulation, edits: Vec<Edit>) -> Result<()> {
        self.tried.fetch_add(1, Ordering::Relaxed);
        let mut reached = Vec::new();
        while sim.frame() < self.max_ticks && reached.len() < self.reach.len() {
            let frame = sim.frame();
            let mut record = match sim.tick() {
                Ok(record) => record,
                // A port fault ends the candidate; report it as a finding.
                Err(e) => {
                    eprintln!("candidate {edits:?} faulted at {frame}: {e}");
                    return Ok(());
                }
            };
            if self.needs_items {
                record.state.extend(sim.item_snapshot(frame).state);
            }
            if self.avoid.iter().any(|c| c.holds(&record)) {
                return Ok(());
            }
            while reached.len() < self.reach.len() && self.reach[reached.len()].holds(&record) {
                reached.push(frame);
            }
        }
        if reached.len() == self.reach.len() {
            let mut found = self.found.lock().unwrap();
            found.push(Found { edits, reached });
            if found.len() >= self.spec.limit {
                self.done.store(true, Ordering::Relaxed);
            }
        }
        Ok(())
    }
}

/// Run a search from `scenario`'s savestate and inputs. Returns successes,
/// earliest goal completion first, and the number of candidates simulated.
pub fn search(
    scenario: &Scenario,
    reference: &Scenario,
    spec: &Spec,
    jobs: usize,
) -> Result<(Vec<Found>, usize)> {
    ensure!(
        scenario.input_clock.as_deref() == Some("tick"),
        "search needs tick-clock inputs"
    );
    ensure!(
        scenario.savestate == reference.savestate,
        "the reference must start from the same savestate"
    );
    ensure!(
        !spec.goal.reach.is_empty(),
        "the goal needs a reach condition"
    );
    for (i, vary) in spec.vary.iter().enumerate() {
        ensure!(
            vary.at.is_some() != vary.after.is_some(),
            "vary #{i}: give exactly one of `at` or `after`"
        );
        if let Some(after) = &vary.after {
            let parent = spec
                .vary
                .iter()
                .position(|v| v.name.as_deref() == Some(after));
            ensure!(
                parent.is_some_and(|p| p < i),
                "vary #{i}: `after = {after:?}` must name an earlier edit"
            );
        }
    }
    let reach: Vec<_> = spec
        .goal
        .reach
        .iter()
        .map(|c| Condition::parse(c))
        .collect::<Result<_>>()?;
    let avoid: Vec<_> = spec
        .goal
        .avoid
        .iter()
        .map(|c| Condition::parse(c))
        .collect::<Result<_>>()?;
    let needs_items = reach
        .iter()
        .chain(&avoid)
        .any(|c| c.key.starts_with("items."));
    let max_ticks = spec.max_ticks.unwrap_or(scenario.frames);
    let display = PadScript::neutral(0).with_display_from(&reference.expected_path())?;
    let search = Search {
        spec,
        base: &scenario.inputs,
        reach,
        avoid,
        needs_items,
        max_ticks,
        display,
        found: Mutex::new(Vec::new()),
        tried: Default::default(),
        done: AtomicBool::new(false),
    };
    let root = Simulation::with_inputs(
        InitialState::from_savestate_traces(reference)?,
        search.pads(&[])?,
    );
    if spec.vary.is_empty() {
        search.finish(root, Vec::new())?;
    } else {
        // Advance the shared prefix once, then spread the first edit's
        // choices over worker threads; deeper levels branch by cloning.
        let mut root = root;
        let until = search.lower_bound(0, &[]).min(max_ticks);
        while root.frame() < until {
            root.tick_without_snapshot()?;
        }
        let (lo, hi) = search.range(0, &[]);
        let vary = &spec.vary[0];
        let choices: Vec<(u64, &toml::Table)> = (lo.max(root.frame())..=hi.min(max_ticks))
            .flat_map(|t| vary.raw.options().into_iter().map(move |r| (t, r)))
            .collect();
        let next = std::sync::atomic::AtomicUsize::new(0);
        let errors = Mutex::new(Vec::new());
        std::thread::scope(|scope| {
            for _ in 0..jobs.max(1) {
                scope.spawn(|| loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(&(tick, raw)) = choices.get(i) else {
                        break;
                    };
                    let edits = vec![Edit {
                        tick,
                        port: vary.port,
                        raw: raw.clone(),
                        hold: vary.hold,
                    }];
                    let result = search.pads(&edits).and_then(|pads| {
                        let mut child = root.clone();
                        child.set_pads(pads);
                        search.branch(1, child, edits)
                    });
                    if let Err(e) = result {
                        errors.lock().unwrap().push(e);
                    }
                });
            }
        });
        if let Some(e) = errors.into_inner().unwrap().into_iter().next() {
            return Err(e);
        }
    }
    let tried = search.tried.load(Ordering::Relaxed);
    let mut found = search.found.into_inner().unwrap();
    found.sort_by_key(|f| {
        (
            f.reached.last().copied(),
            f.edits.iter().map(|e| e.tick).collect::<Vec<_>>(),
        )
    });
    Ok((found, tried))
}

/// A raw pad as the repo's scenarios write it: `{ button = 256, stickX = -127 }`.
pub fn inline_raw(raw: &toml::Table) -> String {
    if raw.is_empty() {
        return "{  }".into();
    }
    let fields: Vec<String> = raw.iter().map(|(k, v)| format!("{k} = {v}")).collect();
    format!("{{ {} }}", fields.join(", "))
}

/// The scenario text with the found edits merged into its `inputs = [...]`
/// block (kept inline and in tick order), `frames` set to the goal plus
/// `margin`, the name replaced if asked, and a comment naming the search.
/// Everything else, comments included, is kept as written.
pub fn write_scenario(
    scenario_text: &str,
    found: &Found,
    spec: &Spec,
    name: Option<&str>,
) -> Result<String> {
    let scenario: toml::Table = toml::from_str(scenario_text)?;
    let mut steps: Vec<(u64, u8, String)> = Vec::new();
    for step in scenario
        .get("inputs")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
    {
        let frame = step
            .get("frame")
            .and_then(|f| f.as_integer())
            .context("input frame")? as u64;
        let port = step.get("port").and_then(|p| p.as_integer()).unwrap_or(0) as u8;
        let raw = step
            .get("raw")
            .and_then(|r| r.as_table())
            .cloned()
            .unwrap_or_default();
        steps.push((frame, port, inline_raw(&raw)));
    }
    for edit in &found.edits {
        for step in edit.steps() {
            steps.push((step.frame, step.port, inline_raw(&step.raw)));
        }
    }
    // Stable: a same-tick edit stays after the base step it overrides.
    steps.sort_by_key(|s| s.0);
    let block: String = steps
        .iter()
        .map(|(frame, port, raw)| {
            format!("  {{ frame = {frame}, port = {port}, buttons = {{  }}, raw = {raw} }},\n")
        })
        .collect();
    let end = found.reached.last().copied().unwrap_or(0) + spec.margin;
    let mut out = String::new();
    let mut lines = scenario_text.lines();
    let mut wrote_note = false;
    while let Some(line) = lines.next() {
        if !wrote_note && !line.starts_with('#') {
            out += &format!(
                "# melee-sim search: reached [{}] at ticks {:?}.\n",
                spec.goal.reach.join("; "),
                found.reached
            );
            wrote_note = true;
        }
        if line.starts_with("frames =") {
            out += &format!("frames = {end}\n");
        } else if let (Some(name), true) = (name, line.starts_with("name =")) {
            out += &format!("name = \"{name}\"\n");
        } else if line.starts_with("inputs = [") {
            out += "inputs = [\n";
            out += &block;
            out += "]\n";
            if !line.trim_end().ends_with(']') {
                for rest in lines.by_ref() {
                    if rest.trim() == "]" {
                        break;
                    }
                }
            }
        } else {
            out += line;
            out.push('\n');
        }
    }
    ensure!(
        out.contains("inputs = ["),
        "the base scenario needs an `inputs = [...]` block"
    );
    toml::from_str::<toml::Table>(&out).context("written scenario does not parse")?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(motion: i64) -> Record {
        let mut record = Record {
            frame: 0,
            phase: "frame_end".into(),
            state: Default::default(),
        };
        record
            .state
            .insert("p1.motion_id".into(), Value::Int(motion));
        record
    }

    #[test]
    fn conditions_take_numbers_or_common_motion_names() {
        let named = Condition::parse("p1.motion_id == KneeBend").unwrap();
        assert!(named.holds(&record(24)));
        assert!(!named.holds(&record(25)));
        assert!(Condition::parse("p1.motion_id >= 341")
            .unwrap()
            .holds(&record(356)));
        assert!(!Condition::parse("p1.missing == 0")
            .unwrap()
            .holds(&record(0)));
        assert!(Condition::parse("p1.motion_id == NotAMotion").is_err());
        assert!(Condition::parse("p1.motion_id ~ 3").is_err());
    }

    #[test]
    fn written_scenarios_merge_edits_inline_and_keep_comments() {
        let base = "# A comment.\nname = \"base\"\ninput_clock = \"tick\"\nframes = 50\nstage = \"FinalDestination\"\ninputs = [\n  { frame = 5, port = 0, buttons = {  }, raw = { stickX = 80 } },\n]\nfighters = []\n";
        let spec: Spec = toml::from_str("[goal]\nreach = [\"p0.motion_id == 20\"]").unwrap();
        let found = Found {
            edits: vec![Edit {
                tick: 3,
                port: 1,
                raw: toml::from_str("button = 256").unwrap(),
                hold: Some(2),
            }],
            reached: vec![30],
        };
        let text = write_scenario(base, &found, &spec, Some("found")).unwrap();
        assert!(text.starts_with(
            "# A comment.\n# melee-sim search: reached [p0.motion_id == 20] at ticks [30].\n"
        ));
        assert!(text.contains("name = \"found\"\n"));
        assert!(text.contains("frames = 70\n"));
        assert!(text.contains(concat!(
            "inputs = [\n",
            "  { frame = 3, port = 1, buttons = {  }, raw = { button = 256 } },\n",
            "  { frame = 5, port = 0, buttons = {  }, raw = { stickX = 80 } },\n",
            "  { frame = 5, port = 1, buttons = {  }, raw = {  } },\n",
            "]\n"
        )));
    }
}
