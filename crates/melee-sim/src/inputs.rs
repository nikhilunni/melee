//! Per-tick controller input for a scenario.
//!
//! A scripted scenario's TOML `inputs` schedule drives Dolphin at VI frames;
//! what the game actually consumed each tick is `HSD_PadGameStatus`, which
//! the tick tracer records beside each tick (`inputs.pN` in the expected
//! trace, decoded by `harness/decode.py`). The port replays that array, so
//! VI-to-tick alignment never has to be modelled. Inputs are inputs, not
//! state: nothing here is compared by the gate.
use anyhow::{bail, ensure, Context, Result};
use melee_ft::input::{Buttons, PadQueueX, PadSample, Stick};
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
    /// Each tick's raw `PADStatus.stickX` per port, when the source carries
    /// it (a tick-clock schedule, a Slippi replay's raw joystick X). Empty:
    /// the engine derives it from the normalized pads.
    raw_stick_x: Vec<[i8; PORTS]>,
    /// The pad-queue bytes each recorded tick's UCF code would read
    /// (`pad_queue_x` in the tick trace). Empty for older recordings.
    pad_queue_x: Vec<[PadQueueX; PORTS]>,
}

/// The tick-trace keys a pad script reads. Deserializing only these lets
/// serde skip the rest of each record (most of it) without building values.
/// A present `events` or `ps_frame`, `null` included, is `Some`, as
/// `Json::get` would find it.
#[derive(serde::Deserialize)]
struct ScriptRecord {
    #[serde(default)]
    inputs: Option<TracePorts>,
    #[serde(default, deserialize_with = "present")]
    events: Option<Json>,
    #[serde(default, deserialize_with = "present")]
    ps_frame: Option<Json>,
    /// Per port, [queue entry qread-1, qread-3] `stickX` (tick_trace.py).
    #[serde(default)]
    pad_queue_x: Option<[[i8; 2]; PORTS]>,
}

fn present<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Json>, D::Error> {
    serde::Deserialize::deserialize(d).map(Some)
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
        script.raw_stick_x = vec![[0; PORTS]; frames + 1];
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
            let pad = replay_pad(input)
                .with_context(|| format!("replay tick {frame} port {}", input.port))?;
            // Pre Frame raw joystick X is the SDK ring byte UCF reads; older
            // replays without it fall back to the normalized stick.
            script.raw_stick_x[frame + 1][usize::from(input.port)] = input
                .raw_stick
                .map_or_else(|| pad.raw_stick_x(), |[x, _]| x);
            script.ticks[frame + 1][usize::from(input.port)] = pad;
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
        let mut current_raw_x = [0; PORTS];
        let mut next = 0;
        script.raw_stick_x = vec![[0; PORTS]; ticks];
        for (tick, (pads, raw_x)) in script
            .ticks
            .iter_mut()
            .zip(script.raw_stick_x.iter_mut())
            .enumerate()
        {
            while next < steps.len() && steps[next].frame as usize == tick {
                let step = steps[next];
                let port = usize::from(step.port);
                anyhow::ensure!(port < PORTS, "input step port {port}");
                current[port] = scheduled_pad(&step.raw)?;
                current_raw_x[port] = scheduled_raw_stick_x(&step.raw)?;
                next += 1;
            }
            *pads = current;
            *raw_x = current_raw_x;
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
            raw_stick_x: Vec::new(),
            pad_queue_x: Vec::new(),
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
        let mut pad_queue_x = Vec::new();
        for (index, line) in reader.lines().enumerate() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let record: ScriptRecord = serde_json::from_str(&line)
                .with_context(|| format!("{}: line {}", path.display(), index + 1))?;
            let pads = match &record.inputs {
                Some(inputs) => parse_ports(inputs)
                    .with_context(|| format!("{}: record {index} inputs", path.display()))?,
                None if require => bail!(
                    "{}: record {index} has no `inputs`; re-record with the pad-capturing tick tracer",
                    path.display()
                ),
                None => [PadSample::default(); PORTS],
            };
            ticks.push(pads);
            if let Some(queue) = record.pad_queue_x {
                ensure!(
                    pad_queue_x.len() + 1 == ticks.len(),
                    "{}: record {index} has `pad_queue_x` but earlier records lack it",
                    path.display()
                );
                pad_queue_x.push(queue.map(|[current, two_ticks_ago]| PadQueueX {
                    current,
                    two_ticks_ago,
                }));
            }
            display_clock.push(display_clock_of(record.ps_frame.as_ref()));
            events.push(
                recorded_events(record.events.as_ref())
                    .with_context(|| format!("{}: record {index} events", path.display()))?,
            );
        }
        ensure!(
            pad_queue_x.is_empty() || pad_queue_x.len() == ticks.len(),
            "{}: `pad_queue_x` on some records only",
            path.display()
        );
        Ok(Self {
            raw_stick_x: pad_queue_x
                .iter()
                .map(|queue| queue.map(|q| q.current))
                .collect(),
            ticks,
            display_clock,
            events,
            pad_queue_x,
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

    /// Tick `tick`'s raw `PADStatus.stickX` per port, when the source
    /// carries it (neutral past the end of such a source).
    pub fn raw_stick_x(&self, tick: u64) -> Option<[i8; PORTS]> {
        if self.raw_stick_x.is_empty() {
            return None;
        }
        Some(
            usize::try_from(tick)
                .ok()
                .and_then(|t| self.raw_stick_x.get(t))
                .copied()
                .unwrap_or_default(),
        )
    }

    /// Tick `tick`'s pad-queue bytes as the retail recording read them.
    pub fn recorded_pad_queue_x(&self, tick: u64) -> Option<[PadQueueX; PORTS]> {
        usize::try_from(tick)
            .ok()
            .and_then(|t| self.pad_queue_x.get(t))
            .copied()
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
    /// and particle order follows the run being compared. A capture that
    /// recorded no display clock (ledgers before `ps_frame`) keeps the
    /// script's own.
    pub fn with_display_from(mut self, path: &Path) -> Result<Self> {
        let reader =
            melee_trace_io::open(path).with_context(|| format!("opening {}", path.display()))?;
        let mut clock = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let record: ScriptRecord = serde_json::from_str(&line)?;
            clock.push(display_clock_of(record.ps_frame.as_ref()));
        }
        if clock.iter().any(Option::is_some) {
            self.display_clock = clock;
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
fn recorded_events(events: Option<&Json>) -> Result<ExternalEvents> {
    let Some(events) = events else {
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
fn display_clock_of(ps_frame: Option<&Json>) -> Option<u64> {
    ps_frame.and_then(Json::as_u64)
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

/// A tick record's `inputs`: each port's decoded `HSD_PadStatus`.
#[derive(serde::Deserialize)]
struct TracePorts {
    p0: Option<TracePad>,
    p1: Option<TracePad>,
    p2: Option<TracePad>,
    p3: Option<TracePad>,
}

/// The `HSD_PadStatus` fields a pad script reads; serde skips the others.
#[derive(serde::Deserialize)]
#[allow(non_snake_case)]
struct TracePad {
    button: Option<UIntField>,
    nml_stickX: Option<FloatField>,
    nml_stickY: Option<FloatField>,
    nml_subStickX: Option<FloatField>,
    nml_subStickY: Option<FloatField>,
    nml_analogL: Option<FloatField>,
    nml_analogR: Option<FloatField>,
}

/// A decoded unsigned field, `{"t": "u", "v": value}`.
#[derive(serde::Deserialize)]
struct UIntField {
    v: u64,
}

/// A decoded f32 field, `{"t": "f32", "v": {"bits": .., "approx": ..}}`.
#[derive(serde::Deserialize)]
struct FloatField {
    v: FloatBits,
}

#[derive(serde::Deserialize)]
struct FloatBits {
    bits: u64,
}

fn parse_ports(inputs: &TracePorts) -> Result<[PadSample; PORTS]> {
    let mut pads = [PadSample::default(); PORTS];
    let ports = [&inputs.p0, &inputs.p1, &inputs.p2, &inputs.p3];
    for (port, (pad, fields)) in pads.iter_mut().zip(ports).enumerate() {
        let fields = fields
            .as_ref()
            .with_context(|| format!("missing port {port}"))?;
        *pad = parse_pad(fields)?;
    }
    Ok(pads)
}

/// `HSD_PadStatus` fields the fighter reads (Fighter_Spaghetti_8006AD10):
/// the button word and the normalized sticks/triggers, by bit pattern.
fn parse_pad(fields: &TracePad) -> Result<PadSample> {
    let uint = |name: &str, field: &Option<UIntField>| -> Result<u64> {
        field
            .as_ref()
            .map(|f| f.v)
            .with_context(|| format!("field {name} is not an unsigned integer"))
    };
    let float = |name: &str, field: &Option<FloatField>| -> Result<f32> {
        let bits = field
            .as_ref()
            .map(|f| f.v.bits)
            .with_context(|| format!("field {name} has no f32 bits"))?;
        Ok(f32::from_bits(u32::try_from(bits)?))
    };
    Ok(PadSample {
        buttons: Buttons(u32::try_from(uint("button", &fields.button)?)?),
        stick: Stick {
            x: float("nml_stickX", &fields.nml_stickX)?,
            y: float("nml_stickY", &fields.nml_stickY)?,
        },
        cstick: Stick {
            x: float("nml_subStickX", &fields.nml_subStickX)?,
            y: float("nml_subStickY", &fields.nml_subStickY)?,
        },
        left_trigger: float("nml_analogL", &fields.nml_analogL)?,
        right_trigger: float("nml_analogR", &fields.nml_analogR)?,
    })
}

/// One tick-clock step's raw `PADStatus.stickX` (the pad queue's byte).
fn scheduled_raw_stick_x(raw: &toml::Table) -> Result<i8> {
    raw.get("stickX").map_or(Ok(0), |v| {
        let value = v.as_integer().context("raw stickX")?;
        Ok(i8::try_from(value)?)
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
        assert_eq!(script.raw_stick_x(0), None);
        assert_eq!(script.recorded_pad_queue_x(0), None);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn recorded_pad_queue_bytes_feed_the_raw_stick() {
        let dir = std::env::temp_dir().join(format!("melee-sim-queue-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();
        for queue in [
            "[[-80,0],[0,0],[0,0],[0,0]]",
            "[[127,-3],[1,2],[0,0],[0,0]]",
        ] {
            let mut record: Json = serde_json::from_str(&pad_json(0, 0)).unwrap();
            record["pad_queue_x"] = serde_json::from_str(queue).unwrap();
            writeln!(f, "{record}").unwrap();
        }
        drop(f);
        let script = PadScript::from_expected_trace(&path, true).unwrap();
        assert_eq!(script.raw_stick_x(1), Some([127, 1, 0, 0]));
        let queue = script.recorded_pad_queue_x(1).unwrap();
        assert_eq!(
            (queue[0], queue[1]),
            (
                PadQueueX {
                    current: 127,
                    two_ticks_ago: -3
                },
                PadQueueX {
                    current: 1,
                    two_ticks_ago: 2
                }
            )
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn tick_schedules_carry_the_unclamped_raw_stick() {
        let step = |frame: u64, x: i64| crate::scenario::InputStep {
            frame,
            port: 0,
            buttons: toml::Table::new(),
            raw: toml::toml! { stickX = x },
        };
        let script = PadScript::from_tick_schedule(&[step(1, 127), step(3, -40)], 5).unwrap();
        let raw: Vec<i8> = (0..5).map(|t| script.raw_stick_x(t).unwrap()[0]).collect();
        assert_eq!(raw, [0, 127, 127, -40, -40]);
        // HSD clamps 127 to the 80-unit circle; the queue keeps the byte.
        assert_eq!(script.sample(1, 0).raw_stick_x(), 80);
    }
}
