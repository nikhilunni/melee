//! Input capture belongs to consumers, outside the deterministic simulation core.
mod config;
pub use config::Config;
use melee_lib::{
    diagnostics, Buttons, ControllerState, GameAssets, Inputs, Match, MatchConfig, Stick,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::OpenOptions,
    io::{BufReader, BufWriter, Read, Write},
    path::Path,
};

/// Thirty minutes of 60 Hz inputs (about 12 MiB). Stop before overflow; never
/// silently discard the cold-start prefix required to reproduce a failure.
pub const MAX_TICKS: usize = 30 * 60 * 60;
const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
const FORMAT_VERSION: u32 = 1;

/// Float bit patterns preserve signed zero and every normalized analog value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sample(pub [[u32; 7]; 4]);
impl From<Inputs> for Sample {
    fn from(inputs: Inputs) -> Self {
        Self(inputs.0.map(|p| {
            [
                p.buttons.0,
                p.stick.x.to_bits(),
                p.stick.y.to_bits(),
                p.cstick.x.to_bits(),
                p.cstick.y.to_bits(),
                p.left_trigger.to_bits(),
                p.right_trigger.to_bits(),
            ]
        }))
    }
}
impl Sample {
    pub fn inputs(self) -> Inputs {
        Inputs(self.0.map(|p| ControllerState {
            buttons: Buttons(p[0]),
            stick: Stick {
                x: f32::from_bits(p[1]),
                y: f32::from_bits(p[2]),
            },
            cstick: Stick {
                x: f32::from_bits(p[3]),
                y: f32::from_bits(p[4]),
            },
            left_trigger: f32::from_bits(p[5]),
            right_trigger: f32::from_bits(p[6]),
        }))
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fault {
    /// One-based attempted tick. Successful prefix has attempt - 1 inputs.
    pub attempt: usize,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recording {
    version: u32,
    pub config: Config,
    asset_fingerprint: u64,
    pub fault: Option<Fault>,
    samples: Vec<Sample>,
}
impl Recording {
    pub fn new(config: &MatchConfig, assets: &GameAssets) -> Self {
        Self {
            version: FORMAT_VERSION,
            config: config.into(),
            asset_fingerprint: diagnostics::asset_fingerprint(assets),
            fault: None,
            samples: Vec::with_capacity(MAX_TICKS),
        }
    }
    pub fn samples(&self) -> &[Sample] {
        &self.samples
    }
    pub fn is_full(&self) -> bool {
        self.samples.len() == MAX_TICKS
    }
    /// Called before Match::step, so even a failing input is retained.
    pub fn push(&mut self, inputs: Inputs) -> Result<(), &'static str> {
        if self.fault.is_some() {
            return Err("recording is faulted; reset before stepping");
        }
        if self.is_full() {
            return Err("30-minute replay capacity reached; save and restart the match");
        }
        self.samples.push(inputs.into());
        Ok(())
    }
    pub fn fail(&mut self, message: String) {
        self.fault = Some(Fault {
            attempt: self.samples.len(),
            message,
        });
    }
    pub fn reset(&mut self) {
        self.samples.clear();
        self.fault = None;
    }
    pub fn write(&self, writer: impl Write) -> Result<(), String> {
        serde_json::to_writer(writer, self).map_err(|e| e.to_string())
    }
    /// Exclusive creation avoids overwriting another playtest's evidence.
    pub fn save_new(&self, path: &Path) -> Result<(), String> {
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        let mut writer = BufWriter::new(file);
        self.write(&mut writer)?;
        writer.flush().map_err(|e| e.to_string())
    }
    /// Native save panels may authorize replacement. Write a sibling first so
    /// a failed export never truncates the destination's existing evidence.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let temporary =
            path.with_file_name(format!(".melee-replay-{}-{stamp}.part", std::process::id()));
        self.save_new(&temporary)?;
        let result = std::fs::rename(&temporary, path).map_err(|e| e.to_string());
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
        result
    }
    pub fn read(reader: impl Read) -> Result<Self, String> {
        let mut bytes = Vec::new();
        reader
            .take(MAX_FILE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() as u64 > MAX_FILE_BYTES {
            return Err("replay exceeds file size limit".into());
        }
        let mut result: Self = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        if result.version != FORMAT_VERSION {
            return Err("unsupported replay version".into());
        }
        if result.samples.len() > MAX_TICKS {
            return Err("replay exceeds tick limit".into());
        }
        if let Some(fault) = &result.fault {
            if fault.attempt == 0 || fault.attempt != result.samples.len() {
                return Err("fault must identify the final attempted tick".into());
            }
        }
        result.config.decode()?;
        result
            .samples
            .reserve_exact(MAX_TICKS - result.samples.len());
        Ok(result)
    }
    pub fn load(path: &Path) -> Result<Self, String> {
        Self::read(BufReader::new(
            std::fs::File::open(path).map_err(|e| e.to_string())?,
        ))
    }
    /// Replay has no rendering dependency. A fixed recording remains useful
    /// after its old fault is fixed; success does not require the old error.
    pub fn replay(&self, assets: &GameAssets) -> Result<Match, Fault> {
        let start_error = |message| Fault {
            attempt: 0,
            message,
        };
        if diagnostics::asset_fingerprint(assets) != self.asset_fingerprint {
            return Err(start_error(
                "replay asset fingerprint differs from loaded data".into(),
            ));
        }
        let config = self.config.decode().map_err(start_error)?;
        let mut game = Match::new(assets, config).map_err(|e| start_error(e.to_string()))?;
        for (index, sample) in self.samples.iter().enumerate() {
            game.step(&sample.inputs()).map_err(|e| Fault {
                attempt: index + 1,
                message: e.to_string(),
            })?;
        }
        Ok(game)
    }
}
