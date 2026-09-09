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
use serde_json::Value as Json;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

/// Controller ports on the GameCube.
pub const PORTS: usize = 4;

/// The pads every tick consumed, in scenario tick order.
#[derive(Debug, Clone, Default)]
pub struct PadScript {
    ticks: Vec<[PadSample; PORTS]>,
}

impl PadScript {
    /// All-neutral pads for `ticks` ticks (an idle scenario, or a trace
    /// recorded before the tracer captured pads).
    pub fn neutral(ticks: usize) -> Self {
        Self {
            ticks: vec![[PadSample::default(); PORTS]; ticks],
        }
    }

    /// Read the `inputs` beside each record of a decoded tick trace.
    ///
    /// `require`: fail if any record lacks `inputs` (scripted scenarios must
    /// carry them); otherwise records without them are neutral.
    pub fn from_expected_trace(path: &Path, require: bool) -> Result<Self> {
        let reader =
            BufReader::new(File::open(path).with_context(|| format!("opening {}", path.display()))?);
        let mut ticks = Vec::new();
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
        }
        Ok(Self { ticks })
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
        usize::try_from(tick)
            .ok()
            .and_then(|t| self.ticks.get(t))
            .map_or_else(PadSample::default, |pads| pads[port])
    }

    /// True if any tick has a non-neutral pad on any port.
    pub fn has_input(&self) -> bool {
        self.ticks
            .iter()
            .flatten()
            .any(|pad| *pad != PadSample::default())
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn pad_json(button: u32, stick_x_bits: u32) -> String {
        let f32_field = |bits: u32| serde_json::json!({"t": "f32", "v": {"bits": bits, "approx": 0}});
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
        let mut f = File::create(&path).unwrap();
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
