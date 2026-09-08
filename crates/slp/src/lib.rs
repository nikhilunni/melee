//! Slippi `.slp` replay parser and converters to the harness formats.
//!
//! A replay is a UBJSON container ([`ubjson`]) whose `raw` member is a byte
//! stream of events ([`event`]). [`Replay::parse`] walks that stream and
//! groups events by frame. [`to_scenario`] renders a harness scenario TOML
//! (see `harness/scenarios/idle_fd_fox.toml`) with the replay's per-frame
//! inputs, and [`to_trace`] renders `melee_diff::Record`s with the paths from
//! `harness/schema/fighter.yaml` that a replay can supply.
//!
//! Field offsets follow project-slippi/slippi-wiki `SPEC.md`. Fields are
//! parsed by payload size: a field is present if the event's payload (as
//! declared in the Event Payloads event) is long enough to contain it, which
//! is what the spec's backwards-compatibility rule guarantees. The Game
//! Start version is kept for reporting but does not gate parsing.
//!
//! # Frame numbering
//!
//! Slippi numbers frames from [`SLIPPI_FIRST_FRAME`] (-123); frame 0 is when
//! the match timer starts. The harness numbers frames from 0 at the first
//! simulated frame, so harness frame `k` is Slippi frame `k + SLIPPI_FIRST_FRAME`
//! ([`harness_frame`]). Slippi's -123 is the first frame the game engine
//! runs for the match (the "GO!" countdown), which is the frame the
//! harness savestates are recorded at.

pub mod event;
pub mod ids;
mod scenario;
mod trace;
pub mod ubjson;

use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;

pub use event::{
    FrameBookend, FrameStart, GameEnd, GameStart, ItemUpdate, PlayerStart, PlayerType, PostFrame,
    PreFrame, Version,
};
pub use scenario::{to_scenario, to_scenario_named};
pub use trace::to_trace;
pub use ubjson::Ubj;

/// The Slippi frame number of the first frame of a match.
pub const SLIPPI_FIRST_FRAME: i32 = -123;

/// Map a Slippi frame number to the harness's 0-based frame index.
/// Returns `None` for frames before [`SLIPPI_FIRST_FRAME`] (which the spec
/// says cannot occur).
pub fn harness_frame(slippi_frame: i32) -> Option<u64> {
    u64::try_from(i64::from(slippi_frame) - i64::from(SLIPPI_FIRST_FRAME)).ok()
}

/// Per-port data for one frame. Ice Climbers produce a `follower` (Nana)
/// alongside the `leader`; every other character only has a `leader`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PortFrame {
    pub leader: PlayerFrame,
    pub follower: PlayerFrame,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PlayerFrame {
    pub pre: Option<PreFrame>,
    pub post: Option<PostFrame>,
}

impl PlayerFrame {
    pub fn is_empty(&self) -> bool {
        self.pre.is_none() && self.post.is_none()
    }
}

/// Everything recorded for one frame number. On rollback (online) replays a
/// frame can be transmitted more than once; the last transmission wins,
/// which is the finalised state.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Frame {
    pub number: i32,
    /// Present from Slippi 2.2.0.
    pub start: Option<FrameStart>,
    pub ports: [PortFrame; 4],
    /// Present from 3.0.0. Cleared and refilled if the frame is rolled back.
    pub items: Vec<ItemUpdate>,
    /// Present from 3.0.0.
    pub bookend: Option<FrameBookend>,
}

impl Frame {
    /// The RNG seed at the start of this frame: Frame Start if present,
    /// otherwise the earliest port's Pre Frame seed (0.1.0 files).
    pub fn start_seed(&self) -> Option<u32> {
        if let Some(s) = &self.start {
            return Some(s.random_seed);
        }
        self.ports
            .iter()
            .find_map(|p| p.leader.pre.as_ref().map(|pre| pre.random_seed))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Replay {
    pub start: GameStart,
    pub end: Option<GameEnd>,
    pub frames: BTreeMap<i32, Frame>,
    /// The `metadata` UBJSON object, if the file was finalised.
    pub metadata: Option<Ubj>,
    /// True if the event stream ended mid-event or the `raw` length was
    /// still 0 (file written while the game was in progress).
    pub incomplete: bool,
    /// Command bytes seen in the stream that this parser does not decode
    /// (skipped by their declared payload size).
    pub skipped_commands: Vec<u8>,
}

impl Replay {
    pub fn parse(bytes: &[u8]) -> Result<Replay> {
        let container = ubjson::split_container(bytes)?;
        let raw = container.raw;
        let mut incomplete = container.raw_len_was_zero;

        // Event Payloads (0x35): command byte, payload size, then (cmd, u16 size) pairs.
        if raw.first() != Some(&event::CMD_EVENT_PAYLOADS) {
            bail!("raw stream does not start with Event Payloads (0x35)");
        }
        let table_len = *raw.get(1).context("truncated Event Payloads")? as usize;
        if table_len < 1 || !(table_len - 1).is_multiple_of(3) || raw.len() < 1 + table_len {
            bail!("malformed Event Payloads: size {table_len}");
        }
        let mut sizes = [None::<usize>; 256];
        for i in 0..(table_len - 1) / 3 {
            let cmd = raw[2 + 3 * i];
            let size = u16::from_be_bytes([raw[3 + 3 * i], raw[4 + 3 * i]]) as usize;
            sizes[cmd as usize] = Some(size);
        }

        let mut pos = 1 + table_len;
        let mut start = None;
        let mut end = None;
        let mut frames: BTreeMap<i32, Frame> = BTreeMap::new();
        let mut skipped_commands = Vec::new();

        while pos < raw.len() {
            let cmd = raw[pos];
            let Some(size) = sizes[cmd as usize] else {
                if container.raw_len_was_zero {
                    // Unfinalised file: we have run off the end of the stream
                    // into whatever follows. Stop quietly.
                    break;
                }
                bail!("unknown command byte 0x{cmd:02X} at raw offset {pos}");
            };
            let Some(ev) = raw.get(pos..pos + 1 + size) else {
                incomplete = true;
                break;
            };
            pos += 1 + size;

            match cmd {
                event::CMD_GAME_START => start = Some(GameStart::parse(ev)?),
                event::CMD_PRE_FRAME => {
                    let pre = PreFrame::parse(ev)?;
                    let slot = slot_mut(&mut frames, pre.frame, pre.player_index, pre.is_follower)?;
                    slot.pre = Some(pre);
                }
                event::CMD_POST_FRAME => {
                    let post = PostFrame::parse(ev)?;
                    let slot =
                        slot_mut(&mut frames, post.frame, post.player_index, post.is_follower)?;
                    slot.post = Some(post);
                }
                event::CMD_GAME_END => end = Some(GameEnd::parse(ev)?),
                event::CMD_FRAME_START => {
                    let fs = FrameStart::parse(ev)?;
                    let f = frame_mut(&mut frames, fs.frame);
                    // A Frame Start marks a (re)transmission of this frame;
                    // drop the previous attempt's per-frame item list.
                    f.items.clear();
                    f.start = Some(fs);
                }
                event::CMD_ITEM_UPDATE => {
                    let it = ItemUpdate::parse(ev)?;
                    frame_mut(&mut frames, it.frame).items.push(it);
                }
                event::CMD_FRAME_BOOKEND => {
                    let b = FrameBookend::parse(ev)?;
                    frame_mut(&mut frames, b.frame).bookend = Some(b);
                }
                other => {
                    if !skipped_commands.contains(&other) {
                        skipped_commands.push(other);
                    }
                }
            }
        }

        let start = start.context("no Game Start (0x36) event")?;
        Ok(Replay {
            start,
            end,
            frames,
            metadata: container.metadata,
            incomplete,
            skipped_commands,
        })
    }

    pub fn version(&self) -> Version {
        self.start.version
    }

    pub fn first_frame(&self) -> Option<i32> {
        self.frames.keys().next().copied()
    }

    pub fn last_frame(&self) -> Option<i32> {
        self.frames.keys().next_back().copied()
    }

    /// `metadata.lastFrame`, if the file carries it.
    pub fn metadata_last_frame(&self) -> Option<i32> {
        self.metadata
            .as_ref()?
            .get("lastFrame")?
            .as_i64()
            .and_then(|v| i32::try_from(v).ok())
    }

    /// The seed the match starts with: the start-of-frame seed of the first
    /// frame, falling back to the Game Start seed for streams with no frames.
    pub fn initial_seed(&self) -> u32 {
        self.frames
            .values()
            .next()
            .and_then(Frame::start_seed)
            .unwrap_or(self.start.random_seed)
    }
}

fn frame_mut(frames: &mut BTreeMap<i32, Frame>, number: i32) -> &mut Frame {
    frames.entry(number).or_insert_with(|| Frame {
        number,
        ..Frame::default()
    })
}

fn slot_mut(
    frames: &mut BTreeMap<i32, Frame>,
    number: i32,
    player_index: u8,
    is_follower: bool,
) -> Result<&mut PlayerFrame> {
    if player_index > 3 {
        bail!("frame {number}: player index {player_index} out of range");
    }
    let port = &mut frame_mut(frames, number).ports[player_index as usize];
    Ok(if is_follower {
        &mut port.follower
    } else {
        &mut port.leader
    })
}
