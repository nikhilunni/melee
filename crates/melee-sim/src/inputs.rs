//! Per-tick controller input for a scenario.
//!
//! A scripted scenario's TOML `inputs` schedule drives Dolphin at VI frames;
//! what the game actually consumed each tick is `HSD_PadGameStatus`, which
//! the tick tracer records beside each tick (`inputs.pN` in the expected
//! trace, decoded by `harness/decode.py`). The port replays that array, so
//! VI-to-tick alignment never has to be modelled. Inputs are inputs, not
//! state: nothing here is compared by the gate.
use anyhow::{bail, Context, Result};
use melee_ft::input::{Buttons, PadSample, Stick};
use melee_lib::{ExternalEvents, StageRead};
use serde_json::Value as Json;
use std::{io::BufRead, path::Path};

/// Controller ports on the GameCube.
pub const PORTS: usize = 4;

/// The pads every tick consumed, in scenario tick order.
#[derive(Debug, Clone, Default)]
pub struct PadScript {
    ticks: Vec<[PadSample; PORTS]>,
    /// psFrameNum at each tick's end, when the tracer recorded it: one step
    /// per particle display pass.
    display_clock: Vec<Option<u64>>,
    /// Each tick's external events. Empty: the port's default policies
    /// (schedules and Slippi replays); a retail trace fills every tick.
    events: Vec<ExternalEvents>,
}

impl PadScript {
    /// Slippi frame -123 is the first full scheduler pass. Simulation tick
    /// zero completes the cold setup/reset boundary, so prepend one pad row.
    pub fn from_replay_inputs(
        inputs: &[slp::cold::ControllerFrame],
        frames: usize,
        ports: &[u8],
    ) -> Result<Self> {
        let mut script = Self::neutral(frames + 1);
        let mut seen = std::collections::BTreeSet::new();
        for input in inputs {
            let frame = usize::try_from(input.frame)?;
            anyhow::ensure!(
                frame < frames && ports.contains(&input.port),
                "replay input frame/port outside match"
            );
            anyhow::ensure!(
                seen.insert((frame, input.port)),
                "duplicate replay input frame {frame} port {}",
                input.port
            );
            script.ticks[frame + 1][usize::from(input.port)] = replay_pad(input)
                .with_context(|| format!("replay tick {frame} port {}", input.port))?;
        }
        anyhow::ensure!(
            seen.len() == frames * ports.len(),
            "missing replay input records"
        );
        Ok(script)
    }
    /// A tick-clock schedule (`input_clock = "tick"`): each step's raw
    /// PADStatus from its tick until the port's next step, with HSD's
    /// virtual stick directions. Dry runs search inputs with it before a
    /// recording exists; gates always replay the recorded pads instead.
    pub fn from_tick_schedule(steps: &[crate::scenario::InputStep], ticks: usize) -> Result<Self> {
        let mut script = Self::neutral(ticks);
        let mut steps: Vec<_> = steps.iter().collect();
        steps.sort_by_key(|step| step.frame);
        let mut current = [PadSample::default(); PORTS];
        let mut next = 0;
        for (tick, pads) in script.ticks.iter_mut().enumerate() {
            while next < steps.len() && steps[next].frame as usize == tick {
                let step = steps[next];
                let port = usize::from(step.port);
                anyhow::ensure!(port < PORTS, "input step port {port}");
                current[port] = scheduled_pad(&step.raw)?;
                next += 1;
            }
            *pads = current;
        }
        Ok(script)
    }

    /// All-neutral pads for `ticks` ticks (an idle scenario, or a trace
    /// recorded before the tracer captured pads).
    pub fn neutral(ticks: usize) -> Self {
        Self {
            ticks: vec![[PadSample::default(); PORTS]; ticks],
            display_clock: vec![None; ticks],
            events: Vec::new(),
        }
    }

    /// Read the `inputs` beside each record of a decoded tick trace.
    ///
    /// `require`: fail if any record lacks `inputs` (scripted scenarios must
    /// carry them); otherwise records without them are neutral.
    pub fn from_expected_trace(path: &Path, require: bool) -> Result<Self> {
        let reader =
            melee_trace_io::open(path).with_context(|| format!("opening {}", path.display()))?;
        let mut ticks = Vec::new();
        let mut display_clock = Vec::new();
        let mut events = Vec::new();
        for (index, line) in reader.lines().enumerate() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let record: Json = serde_json::from_str(&line)
                .with_context(|| format!("{}: line {}", path.display(), index + 1))?;
            let pads = match record.get("inputs") {
                Some(inputs) => parse_ports(inputs)
                    .with_context(|| format!("{}: record {index} inputs", path.display()))?,
                None if require => bail!(
                    "{}: record {index} has no `inputs`; re-record with the pad-capturing tick tracer",
                    path.display()
                ),
                None => [PadSample::default(); PORTS],
            };
            ticks.push(pads);
            display_clock.push(display_clock_of(&record));
            events.push(
                recorded_events(&record)
                    .with_context(|| format!("{}: record {index} events", path.display()))?,
            );
        }
        Ok(Self {
            ticks,
            display_clock,
            events,
        })
    }

    pub fn len(&self) -> usize {
        self.ticks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ticks.is_empty()
    }

    /// The pad `port` consumed during tick `tick`. Past the end of the
    /// script the pad is neutral.
    pub fn sample(&self, tick: u64, port: usize) -> PadSample {
        self.samples(tick)[port]
    }

    /// Every port's pad consumed during tick `tick`; neutral past the end.
    pub fn samples(&self, tick: u64) -> [PadSample; PORTS] {
        usize::try_from(tick)
            .ok()
            .and_then(|t| self.ticks.get(t))
            .copied()
            .unwrap_or_default()
    }

    /// Take display passes from another capture of the same inputs (the RNG
    /// ledger or particle dump run): each Dolphin run has its own VI timing,
    /// and particle order follows the run being compared.
    pub fn with_display_from(mut self, path: &Path) -> Result<Self> {
        let reader =
            melee_trace_io::open(path).with_context(|| format!("opening {}", path.display()))?;
        self.display_clock.clear();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let record: Json = serde_json::from_str(&line)?;
            self.display_clock.push(display_clock_of(&record));
        }
        Ok(self)
    }

    /// Share another script's display clock (read once with
    /// [`Self::with_display_from`]) instead of re-reading its trace.
    pub fn with_display_of(mut self, other: &Self) -> Self {
        self.display_clock.clone_from(&other.display_clock);
        self
    }

    /// Whether retail rendered (and so re-sorted particle lists) between the
    /// previous tick's end and this tick's: the recorded display clock moved.
    /// Traces without one (and live play) render every tick.
    pub fn display_pass(&self, tick: u64) -> bool {
        let Some(t) = usize::try_from(tick).ok().filter(|&t| t > 0) else {
            return true;
        };
        match (self.display_clock.get(t - 1), self.display_clock.get(t)) {
            (Some(Some(previous)), Some(Some(current))) => current != previous,
            _ => true,
        }
    }

    /// The external events of tick `tick`: the recorded ones for a retail
    /// trace, the port's defaults otherwise.
    pub fn events(&self, tick: u64) -> ExternalEvents {
        if self.events.is_empty() {
            return ExternalEvents::default();
        }
        usize::try_from(tick)
            .ok()
            .and_then(|t| self.events.get(t))
            .copied()
            .unwrap_or(ExternalEvents {
                stage_read: StageRead::Unrecorded,
            })
    }

    /// True if any tick has a non-neutral pad on any port.
    pub fn has_input(&self) -> bool {
        self.ticks
            .iter()
            .flatten()
            .any(|pad| *pad != PadSample::default())
    }
}

/// A retail tick's external events (`events` in the decoded trace,
/// harness/decode.py). A trace without them recorded no stage reads: a
/// replay fails closed if the stage polls one.
fn recorded_events(record: &Json) -> Result<ExternalEvents> {
    let Some(events) = record.get("events") else {
        return Ok(ExternalEvents {
            stage_read: StageRead::Unrecorded,
        });
    };
    let completed = events
        .get("stage_read_completed")
        .and_then(Json::as_bool)
        .context("stage_read_completed")?;
    Ok(ExternalEvents {
        stage_read: if completed {
            StageRead::Completed
        } else {
            StageRead::InFlight
        },
    })
}

/// psFrameNum (psdisp.c:1857), recorded by the tick tracer since
/// 2026-09-26. Older traces render every tick: their VI counts are not a
/// reliable proxy for display passes.
fn display_clock_of(record: &Json) -> Option<u64> {
    record.get("ps_frame").and_then(Json::as_u64)
}

/// Reconstruct the pad subset consumed by the human input proc. With old
/// recordings, dead-zoned stick values cannot recover the original HSD pad;
/// they do reproduce the ordinary fighter deadzone result. See docs/SLIPPI.md.
pub fn replay_pad(input: &slp::cold::ControllerFrame) -> Result<PadSample> {
    anyhow::ensure!(
        input
            .stick
            .iter()
            .chain(&input.cstick)
            .all(|x| (-1.0..=1.0).contains(x))
            && input.triggers.iter().all(|x| (0.0..=1.0).contains(x)),
        "unusable Slippi stick/physical trigger fields (no guessed replacement)"
    );
    let stick = |raw: Option<[i8; 2]>, processed: [f32; 2]| {
        raw.map_or(
            Stick {
                x: processed[0],
                y: processed[1],
            },
            |[x, y]| melee_ft::input::pad::normalize_stick(x, y),
        )
    };
    Ok(PadSample {
        // Fighter-generated bit 31 (shield) and Z->A must be recomputed.
        // The HSD stick-direction bits are preserved from processed buttons.
        buttons: Buttons(
            u32::from(input.buttons_physical) | (input.buttons_processed & 0x7fff_0000),
        ),
        stick: stick(input.raw_stick, input.stick),
        cstick: stick(input.raw_cstick, input.cstick),
        left_trigger: input.triggers[0],
        right_trigger: input.triggers[1],
    })
}

fn parse_ports(inputs: &Json) -> Result<[PadSample; PORTS]> {
    let mut pads = [PadSample::default(); PORTS];
    for (port, pad) in pads.iter_mut().enumerate() {
        let fields = inputs
            .get(format!("p{port}"))
            .with_context(|| format!("missing port {port}"))?;
        *pad = parse_pad(fields)?;
    }
    Ok(pads)
}

/// `HSD_PadStatus` fields the fighter reads (Fighter_Spaghetti_8006AD10):
/// the button word and the normalized sticks/triggers, by bit pattern.
fn parse_pad(fields: &Json) -> Result<PadSample> {
    let uint = |name: &str| -> Result<u64> {
        fields[name]["v"]
            .as_u64()
            .with_context(|| format!("field {name} is not an unsigned integer"))
    };
    let float = |name: &str| -> Result<f32> {
        let bits = fields[name]["v"]["bits"]
            .as_u64()
            .with_context(|| format!("field {name} has no f32 bits"))?;
        Ok(f32::from_bits(u32::try_from(bits)?))
    };
    Ok(PadSample {
        buttons: Buttons(u32::try_from(uint("button")?)?),
        stick: Stick {
            x: float("nml_stickX")?,
            y: float("nml_stickY")?,
        },
        cstick: Stick {
            x: float("nml_subStickX")?,
            y: float("nml_subStickY")?,
        },
        left_trigger: float("nml_analogL")?,
        right_trigger: float("nml_analogR")?,
    })
}

/// One tick-clock step's raw PADStatus as HSD_PadGameStatus carries it.
fn scheduled_pad(raw: &toml::Table) -> Result<PadSample> {
    let value = |key: &str| -> Result<i64> {
        raw.get(key).map_or(Ok(0), |v| {
            v.as_integer().with_context(|| format!("raw {key}"))
        })
    };
    let byte = |key: &str| -> Result<i8> { Ok(i8::try_from(value(key)?)?) };
    Ok(PadSample::from_origin_adjusted(
        Buttons(u32::try_from(value("button")?)?),
        [byte("stickX")?, byte("stickY")?],
        [byte("substickX")?, byte("substickY")?],
        [
            u8::try_from(value("triggerL")?)?,
            u8::try_from(value("triggerR")?)?,
        ],
    )
    .with_stick_directions())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn pad_json(button: u32, stick_x_bits: u32) -> String {
        let f32_field =
            |bits: u32| serde_json::json!({"t": "f32", "v": {"bits": bits, "approx": 0}});
        let port = |b: u32, x: u32| {
            serde_json::json!({
                "button": {"t": "u", "v": b},
                "nml_stickX": f32_field(x),
                "nml_stickY": f32_field(0),
                "nml_subStickX": f32_field(0),
                "nml_subStickY": f32_field(0),
                "nml_analogL": f32_field(0),
                "nml_analogR": f32_field(0),
            })
        };
        serde_json::json!({
            "frame": 0,
            "phase": "frame_end",
            "state": {},
            "inputs": {"p0": port(button, stick_x_bits), "p1": port(0, 0), "p2": port(0, 0), "p3": port(0, 0)},
        })
        .to_string()
    }

    #[test]
    fn reads_pads_by_bit_pattern_and_defaults_missing_records_to_neutral() {
        let dir = std::env::temp_dir().join(format!("melee-sim-inputs-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "{}", pad_json(0x100, 0x3F80_0000)).unwrap();
        writeln!(f, r#"{{"frame":1,"phase":"frame_end","state":{{}}}}"#).unwrap();
        drop(f);
        let script = PadScript::from_expected_trace(&path, false).unwrap();
        assert_eq!(script.len(), 2);
        let pad = script.sample(0, 0);
        assert_eq!(pad.buttons, Buttons::A);
        assert_eq!(pad.stick.x.to_bits(), 0x3F80_0000);
        assert_eq!(script.sample(0, 1), PadSample::default());
        assert_eq!(script.sample(1, 0), PadSample::default());
        assert_eq!(script.sample(99, 0), PadSample::default());
        assert!(script.has_input());
        assert!(PadScript::from_expected_trace(&path, true).is_err());
        assert!(!PadScript::neutral(3).has_input());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
