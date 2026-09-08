//! Render a replay as a harness scenario (TOML), in the shape of
//! `harness/scenarios/idle_fd_fox.toml` plus a top-level `[[inputs]]` list.
//!
//! Frames in the output are harness frames (see `crate::harness_frame`).
//! One `[[inputs]]` entry is written per frame per human-controlled port,
//! carrying the physical controller state Slippi recorded in Pre Frame
//! Update, so a scripted controller can replay it.

use crate::ids;
use crate::{harness_frame, Replay, SLIPPI_FIRST_FRAME};
use std::fmt::Write;

/// TOML float: Rust's `Debug` output is valid TOML except for NaN/inf spellings.
fn toml_f32(x: f32) -> String {
    if x.is_nan() {
        "nan".into()
    } else if x.is_infinite() {
        if x > 0.0 { "inf" } else { "-inf" }.into()
    } else {
        format!("{x:?}")
    }
}

fn toml_str(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => write!(out, "\\u{:04X}", c as u32).unwrap(),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Render `replay` as scenario TOML. `name` becomes the scenario's `name`.
pub fn to_scenario_named(replay: &Replay, name: &str) -> String {
    let mut s = String::new();
    let gs = &replay.start;
    let first = replay.first_frame().unwrap_or(SLIPPI_FIRST_FRAME);
    let last = replay.last_frame().unwrap_or(SLIPPI_FIRST_FRAME - 1);
    let frames = (i64::from(last) - i64::from(first) + 1).max(0);

    let _ = writeln!(
        s,
        "# Generated from a Slippi replay (Slippi {}).",
        gs.version
    );
    let _ = writeln!(
        s,
        "# Harness frame 0 is Slippi frame {first}; Slippi's first match frame is {SLIPPI_FIRST_FRAME}."
    );
    let _ = writeln!(s, "name = {}", toml_str(name));
    let _ = writeln!(s, "frames = {frames}");
    let _ = writeln!(s, "# RNG seed at the start of the first frame.");
    let _ = writeln!(s, "seed = {}", replay.initial_seed());
    let _ = writeln!(
        s,
        "# RNG seed Slippi recorded in Game Start, before the match loaded."
    );
    let _ = writeln!(s, "game_start_seed = {}", gs.random_seed);
    match ids::stage_name(gs.stage) {
        Some(n) => {
            let _ = writeln!(s, "stage = {}", toml_str(n));
        }
        None => {
            let _ = writeln!(s, "stage = \"Unknown\"");
        }
    }
    let _ = writeln!(s, "stage_id = {}", gs.stage);
    let _ = writeln!(s, "slippi_version = {}", toml_str(&gs.version.to_string()));
    let _ = writeln!(s, "slippi_first_frame = {first}");
    if gs.is_teams {
        let _ = writeln!(s, "teams = true");
    }
    if let Some(pal) = gs.pal {
        let _ = writeln!(s, "pal = {pal}");
    }
    if gs.is_online() {
        let _ = writeln!(s, "online = true");
    }

    for p in gs.players.iter().filter(|p| p.is_present()) {
        let _ = writeln!(s);
        let _ = writeln!(s, "[[fighters]]");
        let _ = writeln!(s, "slot = {}", p.port);
        match ids::external_character_name(p.character) {
            Some(n) => {
                let _ = writeln!(s, "kind = {}", toml_str(n));
            }
            None => {
                let _ = writeln!(s, "kind = \"Unknown\"");
            }
        }
        let _ = writeln!(s, "character_id = {}", p.character);
        let controller = match p.player_type {
            crate::PlayerType::Cpu => "cpu",
            crate::PlayerType::Demo => "demo",
            _ => "scripted",
        };
        let _ = writeln!(s, "controller = {}", toml_str(controller));
        if p.player_type == crate::PlayerType::Cpu {
            let _ = writeln!(s, "cpu_level = {}", p.cpu_level);
        }
        let _ = writeln!(s, "costume = {}", p.costume);
        let _ = writeln!(s, "stocks = {}", p.stock_start_count);
        if gs.is_teams {
            let _ = writeln!(s, "team = {}", p.team_id);
        }
        if let Some(d) = p.dashback_fix {
            let _ = writeln!(s, "dashback_fix = {d}");
        }
        if let Some(d) = p.shield_drop_fix {
            let _ = writeln!(s, "shield_drop_fix = {d}");
        }
    }

    let _ = writeln!(s);
    let _ = writeln!(
        s,
        "# Per-frame controller state from Pre Frame Update, one entry per human port."
    );
    let _ = writeln!(
        s,
        "# `buttons` are physical button names; `buttons_raw` is the physical bitfield,"
    );
    let _ = writeln!(
        s,
        "# `buttons_processed` the game's processed bitfield. Sticks are processed"
    );
    let _ = writeln!(
        s,
        "# analog values in [-1, 1]; `trigger` is the processed trigger in [0, 1] and"
    );
    let _ = writeln!(
        s,
        "# `triggers` the physical [L, R] pair (unreliable in some recordings)."
    );
    for frame in replay.frames.values() {
        let Some(hf) = harness_frame(frame.number) else {
            continue;
        };
        for (port, pf) in frame.ports.iter().enumerate() {
            if gs.players[port].player_type != crate::PlayerType::Human {
                continue;
            }
            let Some(pre) = &pf.leader.pre else { continue };
            let _ = writeln!(s, "[[inputs]]");
            let _ = writeln!(s, "frame = {hf}");
            let _ = writeln!(s, "port = {port}");
            let names = ids::button_names(pre.buttons_physical);
            let _ = write!(s, "buttons = [");
            for (i, n) in names.iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                s.push_str(&toml_str(n));
            }
            let _ = writeln!(s, "]");
            let _ = writeln!(s, "buttons_raw = 0x{:04X}", pre.buttons_physical);
            let _ = writeln!(s, "buttons_processed = 0x{:08X}", pre.buttons_processed);
            let _ = writeln!(
                s,
                "stick = [{}, {}]",
                toml_f32(pre.joystick_x),
                toml_f32(pre.joystick_y)
            );
            let _ = writeln!(
                s,
                "cstick = [{}, {}]",
                toml_f32(pre.cstick_x),
                toml_f32(pre.cstick_y)
            );
            let _ = writeln!(s, "trigger = {}", toml_f32(pre.trigger));
            let _ = writeln!(
                s,
                "triggers = [{}, {}]",
                toml_f32(pre.physical_l_trigger),
                toml_f32(pre.physical_r_trigger)
            );
            if let (Some(x), Some(y)) = (pre.raw_joystick_x, pre.raw_joystick_y) {
                let _ = writeln!(s, "stick_raw = [{x}, {y}]");
            }
            if let (Some(x), Some(y)) = (pre.raw_cstick_x, pre.raw_cstick_y) {
                let _ = writeln!(s, "cstick_raw = [{x}, {y}]");
            }
        }
    }
    s
}

/// Render `replay` as scenario TOML with a generic name.
pub fn to_scenario(replay: &Replay) -> String {
    to_scenario_named(replay, "slippi_replay")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floats_are_valid_toml() {
        assert_eq!(toml_f32(1.0), "1.0");
        assert_eq!(toml_f32(-0.6875), "-0.6875");
        assert_eq!(toml_f32(f32::NAN), "nan");
        assert_eq!(toml_f32(f32::NEG_INFINITY), "-inf");
    }

    #[test]
    fn strings_are_escaped() {
        assert_eq!(toml_str("a\"b\\c"), "\"a\\\"b\\\\c\"");
    }
}
