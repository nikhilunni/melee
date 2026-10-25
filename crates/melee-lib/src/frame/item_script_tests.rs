//! Item_80268E5C retains the active script through Barrel's rolling drop.
use super::*;
use crate::scenario::Scenario;
use std::{io::BufRead, path::PathBuf};

#[test]
fn rolling_barrel_drop_keeps_the_raw_command_program_timer_and_delayed_variable() {
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    );
    for (name, ticks, roll, drop) in [
        ("corpus_v3_bf_items_sword_e5993e1bc_p2", 398, 365, 375),
        ("corpus_v3_fd_items_taru_e17844051_p1", 369, 334, 353),
    ] {
        let scenario =
            Scenario::load(&root.join(format!("harness/scenarios/{name}.toml"))).unwrap();
        let raw = scenario.trace_path("tick.raw.jsonl");
        if !melee_test_support::require_files(
            scenario.required_files().into_iter().chain([raw.clone()]),
        ) {
            continue;
        }
        assert_eq!(scenario.frames, ticks);
        let pads = crate::trace::pad_script(&scenario)
            .unwrap()
            .with_display_from(&raw)
            .unwrap();
        let mut simulation = TestSimulation::with_inputs(
            InitialState::from_savestate_traces(&scenario).unwrap(),
            pads,
        );
        let mut rows = 0;
        let mut command_pointer = None;
        let mut continued_rows = 0;
        for (frame, line) in melee_test_support::trace::open(&raw)
            .unwrap()
            .lines()
            .enumerate()
        {
            simulation
                .engine
                .set_display_pass(simulation.pads.display_pass(frame as u64));
            simulation.tick_without_snapshot().unwrap();
            let row: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
            assert_eq!(row["frame"], frame);
            rows += 1;
            let Some(retail) = row["items"]
                .as_array()
                .unwrap()
                .iter()
                .find(|item| item["kind"] == 2)
            else {
                continue;
            };
            let hex = retail["bytes"].as_str().unwrap();
            let word =
                |offset: usize| u32::from_str_radix(&hex[offset * 2..offset * 2 + 8], 16).unwrap();
            let item = simulation
                .state()
                .items
                .iter()
                .find(|item| item.kind == melee_types::ItemKind::Taru)
                .unwrap();
            assert_eq!(u32::from(item.motion), word(0x24), "{name} tick {frame}");
            assert_eq!(
                item.script.instruction.is_some(),
                word(0x52c) != 0,
                "{name} tick {frame}: command pointer"
            );
            if frame >= roll {
                assert_eq!(
                    item.command_variables[0],
                    word(0xdac),
                    "{name} tick {frame}: delayed variable"
                );
                assert_eq!(
                    item.script.timer.to_bits(),
                    word(0x524),
                    "{name} tick {frame}: timer"
                );
                assert_eq!(item.script.return_stack.len(), word(0x530) as usize);
                if frame < ticks as usize - 1 {
                    let first = *command_pointer.get_or_insert(word(0x52c));
                    assert_ne!(first, 0);
                    assert_eq!(word(0x52c), first, "retail keeps the rolling continuation");
                    assert_eq!(item.script_source_motion, Some(5));
                }
            }
            if (drop..ticks as usize - 1).contains(&frame) {
                assert_eq!(item.motion, 3);
                assert_eq!(item.command_variables[0], 0);
                continued_rows += 1;
            }
        }
        assert_eq!(rows, ticks as usize);
        assert_eq!(continued_rows, ticks as usize - 1 - drop);
        let item = simulation
            .state()
            .items
            .iter()
            .find(|item| item.kind == melee_types::ItemKind::Taru)
            .unwrap();
        assert_eq!(item.command_variables[0], 1);
        assert_eq!(item.script.instruction, None);
    }
}
