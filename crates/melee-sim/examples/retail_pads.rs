//! Which replays can be played back on retail with the console's pads: run
//! `replay_retail_pads::reconstruct` over every leader's Pre Frame of each
//! replay and print the ones with a pad that cannot be reproduced (the
//! first such tick), with failed polls or with frozen ticks whose stick is
//! no pad's, then the totals.
//!
//!   cargo run -q --release -p melee-sim --example retail_pads -- <dir or .slp>...
use melee_sim::replay_retail_pads::{reconstruct, Reading, INPUT_RELEASE_FRAME};
use std::path::{Path, PathBuf};

fn replays(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_dir() {
        let mut entries: Vec<_> = std::fs::read_dir(path)
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.path())
            .collect();
        entries.sort();
        for entry in entries {
            replays(&entry, out);
        }
    } else if path.extension().is_some_and(|ext| ext == "slp") {
        out.push(path.to_path_buf());
    }
}

fn main() -> anyhow::Result<()> {
    let mut paths = Vec::new();
    for arg in std::env::args().skip(1) {
        replays(Path::new(&arg), &mut paths);
    }
    let (mut clean, mut noted, mut refused, mut unparsed) = (0, 0, 0, 0);
    for path in &paths {
        let Ok(replay) = std::fs::read(path)
            .map_err(anyhow::Error::from)
            .and_then(|bytes| slp::Replay::parse(&bytes))
        else {
            unparsed += 1;
            continue;
        };
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let (mut failed_polls, mut live_failed_polls, mut frozen) = (Vec::new(), 0, 0);
        let mut error = None;
        let mut previous: [Option<slp::cold::ControllerFrame>; 4] = [None, None, None, None];
        'frames: for frame in replay.frames.values() {
            for port in replay.leader_ports() {
                let Some(pre) = &frame.ports[port].leader.pre else {
                    previous[port] = None;
                    continue;
                };
                let input = slp::cold::ControllerFrame::from_pre(pre)?;
                let before = previous[port]
                    .as_ref()
                    .filter(|before| before.frame + 1 == input.frame);
                match reconstruct(&input, before) {
                    Ok(pad) => match pad.reading {
                        Reading::Read => {}
                        Reading::TransferError => {
                            failed_polls.push(input.frame + 1);
                            live_failed_polls += usize::from(input.frame >= INPUT_RELEASE_FRAME);
                        }
                        Reading::Frozen => frozen += 1,
                    },
                    Err(e) => {
                        error = Some(format!("{e:.160} (action {})", pre.action_state));
                        break 'frames;
                    }
                }
                previous[port] = Some(input);
            }
        }
        if let Some(error) = &error {
            refused += 1;
            println!("refused  {name}: {error}");
        } else if failed_polls.is_empty() && frozen == 0 {
            clean += 1;
        } else {
            noted += 1;
            failed_polls.truncate(12);
            println!(
                "noted    {name}: failed polls at ticks {failed_polls:?} ({live_failed_polls} \
                 after GO), {frozen} frozen ticks with a stick no pad gives"
            );
        }
    }
    println!(
        "{} replays: {clean} read on every tick, {noted} with failed polls or frozen sticks, \
         {refused} refused, {unparsed} unparsed",
        paths.len()
    );
    Ok(())
}
