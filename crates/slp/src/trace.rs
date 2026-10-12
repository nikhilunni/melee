//! Render a replay as a canonical trace: one `frame_end` record per frame.
//!
//! Paths follow `harness/schema/fighter.yaml`, prefixed by the contiguous
//! leader index in ascending active-port order. `player_id` retains the
//! original controller slot, including gaps. Only Post Frame fields are
//! emitted; a field absent from an old replay is simply omitted, which
//! `melee_diff::first_divergence` treats as "not checked" when the replay is
//! the expected side.
//!
//! Field mapping (Post Frame Update -> Fighter):
//! - internal character id      -> `kind` (fp+0x4, s32)
//! - player index               -> `player_id` (fp+0xC, u8)
//! - action state id            -> `motion_id` (fp+0x10, s32)
//! - facing direction           -> `facing_dir` (fp+0x2C)
//! - x/y position               -> `cur_pos.x` / `cur_pos.y` (fp+0xB0)
//! - percent                    -> `percent` (fp+0x1830)
//! - ground/air state (2.0.0+)  -> `ground_or_air` (fp+0xE0; 0 ground, 1 air)
//! - action state frame counter -> `cur_anim_frame` (fp+0x894), 0.2.0+
//! - self-induced air x / y speed (3.5.0+) -> `self_vel.x` / `self_vel.y` (fp+0x80)
//! - attack-based x / y speed (3.5.0+)     -> `kb_vel.x` / `kb_vel.y` (fp+0x8C)
//!
//! `jumps_used` is deliberately not emitted: Slippi records jumps *remaining*
//! and the conversion needs the character's max jump count. Nana (the Ice
//! Climbers follower) is emitted under `pN.follower.`, N being her leader's
//! index; consumers map it to her own fighter.
//!
//! `rng.seed` in a `frame_end` record is the seed after the frame finished.
//! Use adjacent Frame Start(n+1), never Pre Frame(n+1): the latter runs
//! after fighter animation can draw RNG. The final frame, gaps, and versions
//! before Frame Start omit this key. docs/SLIPPI.md records the ledger proof
//! and the limits of the tick-end/next-start relation.

use crate::{harness_frame, Replay};
use melee_diff::{Record, Value};
use std::collections::BTreeMap;

pub const PHASE_FRAME_END: &str = "frame_end";

pub fn to_trace(replay: &Replay) -> Vec<Record> {
    let mut out = Vec::with_capacity(replay.frames.len());
    let mut iter = replay.frames.values().peekable();
    while let Some(frame) = iter.next() {
        out.extend(frame_record(replay, frame, iter.peek().copied()));
    }
    out
}

/// One frame's record of [`to_trace`]; `next` is the frame recorded after
/// it. `None` for a frame before the match's first.
pub fn frame_record(
    replay: &Replay,
    frame: &crate::Frame,
    next: Option<&crate::Frame>,
) -> Option<Record> {
    let mut state = BTreeMap::new();
    let hf = frame_values(replay, frame, next, |key, value| {
        state.insert(key.to_string(), value);
    })?;
    Some(Record {
        frame: hf,
        phase: PHASE_FRAME_END.to_string(),
        state,
    })
}

/// Where one value of a frame's record belongs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameKey {
    /// `rng.seed`.
    Seed,
    /// `pN.<path>`, or `pN.follower.<path>` for Nana: a fighter schema path
    /// of the Nth leader.
    Field {
        leader: usize,
        follower: bool,
        path: &'static str,
    },
}

impl std::fmt::Display for FrameKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Seed => write!(f, "rng.seed"),
            Self::Field {
                leader,
                follower: false,
                path,
            } => write!(f, "p{leader}.{path}"),
            Self::Field {
                leader,
                follower: true,
                path,
            } => write!(f, "p{leader}.follower.{path}"),
        }
    }
}

/// [`frame_record`]'s values without the record: `put` takes each key and
/// value. Returns the harness frame; `None` (and no values) for a frame
/// before the match's first.
pub fn frame_values(
    replay: &Replay,
    frame: &crate::Frame,
    next: Option<&crate::Frame>,
    mut put: impl FnMut(FrameKey, Value),
) -> Option<u64> {
    let hf = harness_frame(frame.number)?;

    // Seed at end of this frame = seed at start of the next frame, when
    // the next frame really is the successor.
    if let Some(next) = next {
        if next.number == frame.number.wrapping_add(1) {
            if let Some(seed) = next.scheduler_start_seed() {
                put(FrameKey::Seed, Value::UInt(seed as u64));
            }
        }
    }

    for (leader, port) in replay.leader_ports().enumerate() {
        let pf = &frame.ports[port];
        let members = [
            (pf.leader.post.as_ref(), false),
            (pf.follower.post.as_ref(), true),
        ];
        for (post, follower) in members {
            let Some(post) = post else {
                continue;
            };
            put_post(post, |path, value| {
                let key = FrameKey::Field {
                    leader,
                    follower,
                    path,
                };
                put(key, value)
            });
        }
    }
    Some(hf)
}

fn put_post(post: &crate::PostFrame, mut put: impl FnMut(&'static str, Value)) {
    put("kind", Value::Int(post.internal_character as i64));
    put("player_id", Value::UInt(post.player_index as u64));
    put("motion_id", Value::Int(post.action_state as i64));
    put("facing_dir", Value::f32(post.facing_direction));
    put("cur_pos.x", Value::f32(post.position_x));
    put("cur_pos.y", Value::f32(post.position_y));
    put("percent", Value::f32(post.percent));
    if let Some(air) = post.airborne {
        put("ground_or_air", Value::Int(air as i64));
    }
    if let Some(f) = post.action_state_frame {
        put("cur_anim_frame", Value::f32(f));
    }
    if let (Some(x), Some(y)) = (post.self_air_speed_x, post.self_speed_y) {
        put("self_vel.x", Value::f32(x));
        put("self_vel.y", Value::f32(y));
    }
    if let (Some(x), Some(y)) = (post.attack_speed_x, post.attack_speed_y) {
        put("kb_vel.x", Value::f32(x));
        put("kb_vel.y", Value::f32(y));
    }
}
