//! Complete terminal pickup entries; later ownership remains unavailable.
use super::*;
use crate::scenario::Scenario;
use melee_ft::fighter::{item_pickup::PickupCandidate, state::InputPhase, MotionData};
use std::{io::BufRead, path::PathBuf};

const CASES: [(&str, usize, usize); 5] = [
    ("corpus_v3_bf_items_sword_e5993e1bc_p0", 259, 1),
    ("corpus_v3_fd_items_sword_e5de865a8_p1", 257, 0),
    ("corpus_v3_fd_items_sword_e6af4a7bb_p2", 240, 0),
    ("corpus_v3_fd_items_sword_efc6a328f_p2", 241, 0),
    ("corpus_v3_fd_items_taru_e1895a33d_p0", 295, 1),
];

fn root() -> PathBuf {
    std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    )
}

fn expect_guard(f: impl FnOnce(), message: &str) {
    let error = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).unwrap_err();
    let text = error
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| error.downcast_ref::<&str>().copied())
        .unwrap();
    assert!(text.contains(message), "{text}");
}

#[test]
fn complete_heavy_entries_match_raw_and_refuse_next_animation() {
    for (name, ticks, actor) in CASES {
        let scenario =
            Scenario::load(&root().join(format!("harness/scenarios/{name}.toml"))).unwrap();
        let raw = scenario.trace_path("tick.raw.jsonl");
        assert!(melee_test_support::require_files(
            scenario.required_files().into_iter().chain([raw.clone()])
        ));
        assert_eq!(scenario.frames as usize, ticks);
        let pads = crate::trace::pad_script(&scenario)
            .unwrap()
            .with_display_from(&raw)
            .unwrap();
        let mut simulation = TestSimulation::with_inputs(
            InitialState::from_savestate_traces(&scenario).unwrap(),
            pads,
        );
        let mut entries = 0;
        let mut rows = 0;
        let mut before_entry_item = None;
        for (frame, line) in melee_test_support::trace::open(&raw)
            .unwrap()
            .lines()
            .enumerate()
        {
            let row: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
            assert_eq!(row["frame"], frame);
            simulation
                .engine
                .set_display_pass(simulation.pads.display_pass(frame as u64));
            simulation.tick_without_snapshot().unwrap();
            for port in 0..2 {
                let hex = row["fighters"][port]["bytes"].as_str().unwrap();
                let word = |o: usize| u32::from_str_radix(&hex[o * 2..o * 2 + 8], 16).unwrap();
                let f = &simulation.state().fighters[port].0;
                assert_eq!(
                    u32::from(f.motion_state.action.0),
                    word(0x10),
                    "{name} tick{frame} p{port}"
                );
                for (v, o) in [
                    (f.physics.position.x, 0xB0),
                    (f.physics.position.y, 0xB4),
                    (f.physics.position.z, 0xB8),
                ] {
                    assert_eq!(v.to_bits(), word(o), "{name} tick{frame} p{port} position");
                }
                if name == CASES[4].0 && port == 1 && frame >= 242 {
                    assert_eq!(
                        f.inherited_scratch_word().unwrap().to_bits(),
                        word(0x2344),
                        "{name} tick{frame} throw-to-Wait-to-HeavyGet scratch"
                    );
                }
                if word(0x10) != 93 {
                    continue;
                }
                entries += 1;
                assert_eq!((frame, port), (ticks - 1, actor));
                assert_eq!(f.motion_state.animation, 89);
                assert_eq!(f.animation.frame.to_bits(), word(0x894));
                assert_eq!(f.animation.speed.to_bits(), word(0x89C));
                let MotionData::ItemGet {
                    heavy: true,
                    retained_word,
                } = f.state_data
                else {
                    panic!("heavy pickup scratch");
                };
                assert_eq!(
                    retained_word.expect("retained mv+4").to_bits(),
                    word(0x2344),
                    "{name} tick{frame} p{port} retained scratch"
                );
                assert_eq!(f.physics.ground_or_air as u32, word(0xE0));
                assert_eq!(f.collision.data.floor.index as u32, word(0x83C));
                assert_eq!(word(0x2340), 1);
                assert_eq!(word(0x21DC), 0x8009_74C4);
                assert!(f.heavy_pickup_damage_armed);
                assert_eq!(word(0x2210), 0);
                assert!(
                    !f.commands.grab_release
                        && !f.commands.rapid_jab_loop_end
                        && !f.commands.throw_reverse
                        && !f.commands.throw_accessory
                        && !f.commands.move_cue
                );
                assert!(f.held_item.is_none());
                assert_eq!((word(0x1974), word(0x1978)), (0, 0));
                for (v, o) in [
                    (f.physics.self_velocity.x, 0x80),
                    (f.physics.self_velocity.y, 0x84),
                    (f.physics.self_velocity.z, 0x88),
                ] {
                    assert_eq!(v.to_bits(), word(o), "{name} terminal velocity");
                }
            }
            if frame + 2 >= ticks {
                let live: Vec<_> = simulation
                    .state()
                    .items
                    .iter()
                    .filter(|i| !i.destroyed)
                    .collect();
                assert_eq!(live.len(), 1);
                assert_eq!(row["items"].as_array().unwrap().len(), 1);
                let actual = live[0];
                let raw_item = &row["items"][0];
                let bytes = raw_item["bytes"].as_str().unwrap();
                let word = |o: usize| u32::from_str_radix(&bytes[o * 2..o * 2 + 8], 16).unwrap();
                assert_eq!(actual.kind as u32, word(0x10));
                assert_eq!(u32::from(actual.motion), word(0x24));
                assert!(actual.owner.is_none());
                assert_eq!(word(0x518), 0);
                let identity = (
                    raw_item["gobj"].as_str().unwrap().to_owned(),
                    actual.id,
                    actual.gobj,
                );
                if frame + 2 == ticks {
                    before_entry_item = Some(identity);
                } else {
                    // Internal identities model pointer equality, not retail
                    // numeric addresses. Neither identity changes at entry.
                    assert_eq!(before_entry_item.as_ref().unwrap(), &identity);
                }
            }
            rows += 1;
        }
        assert_eq!((rows, entries), (ticks, 1));
        let state = simulation.state();
        let mut fighter = state.fighters[actor].clone();
        let assets = &state.assets.fighters[actor];
        let row = melee_ft::fighter::state::COMMON[93];
        expect_guard(
            || (row.iasa)(&mut fighter.0, InputPhase { assets }),
            "ftpickupitem_IASA",
        );
        expect_guard(
            || {
                simulation.tick_without_snapshot().unwrap();
            },
            "HeavyGet continuation",
        );
    }
}

#[test]
fn pickup_entry_clears_every_throw_flag_and_common_damage_owner_replaces_character() {
    let scenario =
        Scenario::load(&root().join(format!("harness/scenarios/{}.toml", CASES[1].0))).unwrap();
    let state = InitialState::from_savestate_traces(&scenario).unwrap();
    for heavy in [false, true] {
        let mut fighter = state.fighters[0].clone();
        let f = &mut **fighter.0;
        let assets = &state.assets.fighters[0];
        f.commands.grab_release = true;
        f.commands.rapid_jab_loop_end = true;
        f.commands.throw_reverse = true;
        f.commands.throw_accessory = true;
        f.commands.move_cue = true;
        let position = hsd_types::Vec2::new(f.physics.position.x, f.physics.position.y);
        f.pickup_candidates.offer(std::iter::once(PickupCandidate {
            item: 1,
            kind: if heavy {
                melee_types::ItemKind::Box
            } else {
                melee_types::ItemKind::Capsule
            },
            position,
            range: hsd_types::Vec2::new(100.0, 100.0),
            heavy,
            consumable: false,
            use_kind: 0,
            hand_hold_kind: 0,
            damage_multiplier: 1.0,
        }));
        assert!(f.try_item_pickup(assets).unwrap());
        assert_eq!(f.motion_state.action.0, if heavy { 93 } else { 92 });
        assert!(
            !f.commands.grab_release
                && !f.commands.rapid_jab_loop_end
                && !f.commands.throw_reverse
                && !f.commands.throw_accessory
                && !f.commands.move_cue
        );
        if !heavy {
            continue;
        }
        let fox = f.character.get_mut::<ft_fox::init::Fox>();
        fox.special_neutral.callbacks_armed = true;
        fox.special_neutral.blaster_present = true;
        f.interrupt_actions(assets);
        assert!(
            f.character
                .get::<ft_fox::init::Fox>()
                .special_neutral
                .blaster_present
        );
        f.change_motion_state(melee_ft::fighter::state::ActionId(14), assets)
            .unwrap();
        let fox = f.character.get_mut::<ft_fox::init::Fox>();
        fox.special_neutral.callbacks_armed = true;
        fox.special_neutral.blaster_present = true;
        f.interrupt_actions(assets);
        assert!(
            !f.character
                .get::<ft_fox::init::Fox>()
                .special_neutral
                .blaster_present
        );
        assert!(f.try_item_pickup(assets).unwrap());
        f.held_item = Some(melee_ft::fighter::item_pickup::HeldItem {
            item: 1,
            kind: melee_types::ItemKind::Taru,
            heavy: true,
            hand_hold_kind: 0,
            use_kind: 0,
            damage_multiplier: 1.0,
        });
        expect_guard(|| f.interrupt_actions(assets), "ftCo_800974C4");
        f.held_item = None;
        f.player.scale = 2.0;
        expect_guard(
            || {
                f.try_item_pickup(assets).unwrap();
            },
            "scaled HeavyGet",
        );
    }
}

#[test]
fn throw_scratch_tracks_frozen_flag_and_landing_before_retention() {
    for name in ["kirby_throw_f_fd_fox4", "kirby_throw_b_fd_fox4"] {
        let scenario =
            Scenario::load(&root().join(format!("harness/scenarios/{name}.toml"))).unwrap();
        let raw = scenario.trace_path("tick.raw.jsonl");
        let pads = crate::trace::pad_script(&scenario)
            .unwrap()
            .with_display_from(&raw)
            .unwrap();
        let mut simulation = TestSimulation::with_inputs(
            InitialState::from_savestate_traces(&scenario).unwrap(),
            pads,
        );
        let mut saw_zero = false;
        let mut saw_one = false;
        let mut saw_landing_after_one = false;
        let mut rows = 0;
        for (frame, line) in melee_test_support::trace::open(&raw)
            .unwrap()
            .lines()
            .enumerate()
        {
            let row: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
            assert_eq!(row["frame"], frame);
            simulation
                .engine
                .set_display_pass(simulation.pads.display_pass(frame as u64));
            simulation.tick_without_snapshot().unwrap();
            let f = &simulation.state().fighters[0].0;
            if matches!(f.state_data, MotionData::Throw) {
                let hex = row["fighters"][0]["bytes"].as_str().unwrap();
                let word = |o: usize| u32::from_str_radix(&hex[o * 2..o * 2 + 8], 16).unwrap();
                assert_eq!(
                    f.inherited_scratch_word().unwrap().to_bits(),
                    word(0x2344),
                    "{name} tick{frame} throw word"
                );
                assert_eq!(u32::from(f.combat.throw_flight.landed), word(0x2348));
                saw_zero |= word(0x2344) == 0;
                saw_one |= word(0x2344) == 1;
                saw_landing_after_one |= saw_one && word(0x2344) == 0 && word(0x2348) == 1;
            }
            rows += 1;
        }
        assert_eq!(rows, scenario.frames as usize);
        assert!(
            saw_zero && saw_one && saw_landing_after_one,
            "{name}: real 0→1→0 landing"
        );
    }
}
