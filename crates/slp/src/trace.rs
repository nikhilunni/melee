//! Render a replay as a canonical trace: one `frame_end` record per frame.
//!
//! Paths follow `harness/schema/fighter.yaml`, prefixed `p{port}` where
//! `port` is the 0-based controller port (the Dolphin decoder uses the
//! fighter-list index instead; they coincide when ports are filled from 1
//! without gaps). Only fields that Slippi's Post Frame Update carries are
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
//! Climbers follower) has no schema path and is not emitted.
//!
//! `rng.seed` in a `frame_end` record is the seed after the frame finished.
//! Slippi records the seed at the *start* of each frame (Frame Start, or the
//! first Pre Frame Update before 2.2.0), so the record for frame `n` carries
//! the start-of-frame seed of frame `n + 1`; the final frame has none. This
//! assumes the game draws no random numbers between the end-of-frame dump
//! and the next frame's Frame Start hook.

use crate::{harness_frame, Replay};
use melee_diff::{Record, Value};
use std::collections::BTreeMap;

pub const PHASE_FRAME_END: &str = "frame_end";

pub fn to_trace(replay: &Replay) -> Vec<Record> {
    let mut out = Vec::with_capacity(replay.frames.len());
    let mut iter = replay.frames.values().peekable();
    while let Some(frame) = iter.next() {
        let Some(hf) = harness_frame(frame.number) else {
            continue;
        };
        let mut state = BTreeMap::new();

        // Seed at end of this frame = seed at start of the next frame, when
        // the next frame really is the successor.
        if let Some(next) = iter.peek() {
            if next.number == frame.number.wrapping_add(1) {
                if let Some(seed) = next.start_seed() {
                    state.insert("rng.seed".to_string(), Value::UInt(seed as u64));
                }
            }
        }

        for (port, pf) in frame.ports.iter().enumerate() {
            let Some(post) = &pf.leader.post else {
                continue;
            };
            let p = format!("p{port}");
            let mut put = |name: &str, v: Value| {
                state.insert(format!("{p}.{name}"), v);
            };
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

        out.push(Record {
            frame: hf,
            phase: PHASE_FRAME_END.to_string(),
            state,
        });
    }
    out
}
