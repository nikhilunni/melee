//! The original complete capture ends after Back Air entry's direct step.
use super::*;
use crate::scenario::Scenario;
use melee_ft::fighter::state::{AnimationPhase, InputPhase};
use melee_types::CommonMotionState as S;
use std::{io::BufRead, path::PathBuf};

fn root() -> PathBuf {
    std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    )
}
fn expect_guard(f: impl FnOnce(), name: &str) {
    let error = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).unwrap_err();
    let message = error
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| error.downcast_ref::<&str>().copied())
        .unwrap();
    assert!(message.contains(name), "{message}");
}

#[test]
fn complete_back_air_entry_keeps_raw_held_identity_and_refuses_later_callbacks() {
    const NAME: &str = "corpus_v3_fd_items_sword_nocont_e8f0de8c6_p2";
    let scenario = Scenario::load(&root().join(format!("harness/scenarios/{NAME}.toml"))).unwrap();
    assert_eq!(scenario.frames, 621);
    let raw = scenario.trace_path("tick.raw.jsonl");
    if !melee_test_support::require_files(
        scenario.required_files().into_iter().chain([raw.clone()]),
    ) {
        return;
    }
    let pads = crate::trace::pad_script(&scenario)
        .unwrap()
        .with_display_from(&raw)
        .unwrap();
    let mut simulation = TestSimulation::with_inputs(
        InitialState::from_savestate_traces(&scenario).unwrap(),
        pads,
    );
    let mut rows = 0;
    let mut aerial_entries = 0;
    let mut held_identity = None;
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
        let f = &simulation.state().fighters[1].0;
        let bytes = row["fighters"][1]["bytes"].as_str().unwrap();
        let word = |o: usize| u32::from_str_radix(&bytes[o * 2..o * 2 + 8], 16).unwrap();
        assert_eq!(
            u32::from(f.motion_state.action.0),
            word(0x10),
            "tick{frame} action"
        );
        assert_eq!(
            f.animation.frame.to_bits(),
            word(0x894),
            "tick{frame} frame"
        );
        assert_eq!(
            f.commands.variables[0] as u32,
            word(0x2200),
            "tick{frame} command0"
        );
        let raw_interrupt = word(0x2218) & 0x8000_0000 != 0;
        // The flag is owned by these consuming attack callbacks. Retail
        // Fall retains the departed aerial's bit, while reset_motion's
        // existing semantic representation normalizes its unused value.
        if (65..=69).contains(&f.motion_state.action.0) {
            assert_eq!(
                f.commands.allow_interrupt, raw_interrupt,
                "tick{frame} aerial interrupt"
            );
        }
        if frame == 336 {
            assert_eq!(f.motion_state.action, S::Fall.into());
            assert!(f.held_item.is_none());
            assert!(raw_interrupt, "retail retains the old aerial unlock");
            assert!(
                !f.commands.allow_interrupt,
                "unused Fall control is normalized"
            );
        }
        let visible =
            u8::from_str_radix(&bytes[0x221e * 2..0x221e * 2 + 2], 16).unwrap() & 0x10 != 0;
        assert_eq!(
            !f.commands.held_item_hidden, visible,
            "tick{frame} held visibility"
        );
        for (value, offset) in [
            (f.physics.position.x, 0xb0),
            (f.physics.position.y, 0xb4),
            (f.physics.position.z, 0xb8),
            (f.combat.hitlag_remaining, 0x195c),
        ] {
            assert_eq!(value.to_bits(), word(offset), "tick{frame} +{offset:x}");
        }
        let live = simulation
            .state()
            .items
            .iter()
            .filter(|i| !i.destroyed)
            .collect::<Vec<_>>();
        let raw_items = row["items"].as_array().unwrap();
        assert_eq!(live.len(), raw_items.len());
        for (item, raw_item) in live.iter().zip(raw_items) {
            let hex = raw_item["bytes"].as_str().unwrap();
            let iw = |o: usize| u32::from_str_radix(&hex[o * 2..o * 2 + 8], 16).unwrap();
            assert_eq!(item.kind as u32, iw(0x10));
            assert_eq!(u32::from(item.motion), iw(0x24));
            assert_eq!(item.owner.map(u64::from), raw_item["owner"].as_u64());
            assert_eq!(item.held, iw(0xdc8) & (1 << (31 - 0x13)) != 0);
            assert_eq!(item.life_timer.to_bits(), iw(0xd44), "tick{frame} life");
            for (value, offset) in [
                (item.position.x, 0x4c),
                (item.position.y, 0x50),
                (item.position.z, 0x54),
            ] {
                assert_eq!(value.to_bits(), iw(offset), "tick{frame} item pos");
            }
            // Pickup attachment belongs to the Sword under test. Fox's
            // held blaster uses its separate HELD_PART presentation path;
            // its xDC4 is not represented by ItemCore::holder_part.
            if item.held && item.kind == melee_types::ItemKind::Sword {
                assert_eq!(
                    u32::from(item.holder_part),
                    iw(0xdc4),
                    "tick{frame} Sword holder part"
                );
                let owner = usize::from(item.owner.expect("held Sword owner"));
                let owner_bytes = row["fighters"][owner]["bytes"].as_str().unwrap();
                let owner_word =
                    |o: usize| u32::from_str_radix(&owner_bytes[o * 2..o * 2 + 8], 16).unwrap();
                assert_eq!(iw(0x518), owner_word(0), "actual fighter GObj owner");
                let raw_gobj = u32::from_str_radix(
                    raw_item["gobj"].as_str().unwrap().trim_start_matches("0x"),
                    16,
                )
                .unwrap();
                assert_eq!(owner_word(0x1974), raw_gobj);
                assert_eq!(
                    simulation.state().fighters[owner].0.held_item.unwrap().item,
                    item.id
                );
                let identity = (raw_gobj, item.id, item.gobj);
                if let Some(previous) = held_identity {
                    assert_eq!(previous, identity);
                } else {
                    held_identity = Some(identity);
                }
            }
        }
        assert_eq!(f.held_item.is_some(), word(0x1974) != 0);
        if f.motion_state.action == S::AttackAirB.into() && f.held_item.is_some() {
            assert!(matches!(
                f.state_data,
                melee_ft::fighter::MotionData::Aerial {
                    held_on_entry: true,
                    ..
                }
            ));
            aerial_entries += 1;
            assert_eq!(frame, 620);
            assert_eq!(f.animation.frame, 1.0);
            assert_eq!(f.physics.ground_or_air, melee_types::GroundOrAir::Air);
            assert!(!f.commands.allow_interrupt);
            assert_eq!(word(0x2210), 0);
            assert!(
                !f.commands.grab_release
                    && !f.commands.throw_reverse
                    && !f.commands.rapid_jab_loop_end
            );
            assert!(f.commands.hitboxes.iter().all(Option::is_none));
            let sword = live.iter().find(|item| item.held).unwrap();
            assert_eq!(sword.holder_part, 40);
            assert_eq!(sword.life_timer, 1400.0);
            assert_eq!(
                sword.position,
                hsd_types::Vec3::new(0.0, f32::from_bits(0x40e0a4a9), 0.0)
            );
        }
        rows += 1;
    }
    assert_eq!((rows, aerial_entries), (621, 1));
    assert!(held_identity.is_some());
    let state = simulation.state();
    // Direct callbacks distinguish IASA from Animation. They refuse even
    // while interrupt is locked, and before animation playback/command work.
    for animation in [true, false] {
        let mut fighter = state.fighters[1].clone();
        let mut map = state.map.clone();
        let mut rng = state.rng;
        let frame = fighter.0.animation.frame.to_bits();
        let remainder = fighter.0.animation.remainder.to_bits();
        let timer = fighter.0.commands.timer.to_bits();
        let facing = fighter.0.physics.facing.to_bits();
        let instruction = fighter.0.commands.instruction;
        let hitlag = fighter.0.combat.hitlag_remaining.to_bits();
        let effects = fighter.0.effects.clone();
        let row = melee_ft::fighter::state::COMMON[67];
        expect_guard(
            || {
                if animation {
                    (row.anim)(
                        &mut fighter.0,
                        AnimationPhase {
                            assets: &state.assets.fighters[1],
                            map: &mut map,
                            rng: &mut rng,
                        },
                    )
                    .unwrap();
                } else {
                    (row.iasa)(
                        &mut fighter.0,
                        InputPhase {
                            assets: &state.assets.fighters[1],
                        },
                    );
                }
            },
            if animation {
                "ftCo_AttackAir_Anim"
            } else {
                "ftCo_AttackAir_IASA"
            },
        );
        assert_eq!(fighter.0.animation.frame.to_bits(), frame);
        assert_eq!(fighter.0.animation.remainder.to_bits(), remainder);
        assert_eq!(fighter.0.commands.timer.to_bits(), timer);
        assert_eq!(fighter.0.physics.facing.to_bits(), facing);
        assert_eq!(fighter.0.commands.instruction, instruction);
        assert_eq!(fighter.0.combat.hitlag_remaining.to_bits(), hitlag);
        assert_eq!(fighter.0.effects, effects);
    }
    // The simulator's next actual animation callback also remains refused;
    // this is not a claim that every partial tick is rolled back.
    expect_guard(
        || {
            simulation.tick_without_snapshot().unwrap();
        },
        "held Sword aerial entry continuation",
    );
}

/// Catching into an existing aerial preserves its entry ownership. Entering
/// another aerial with that same hand must publish fresh ownership even
/// though the airtime-wide catch lock remains set.
#[test]
fn existing_aerial_catch_preserves_entry_context_but_reentry_owns_current_hand() {
    const NAME: &str = "corpus_v3_fd_items_sword_e0144d061_p1";
    let scenario = Scenario::load(&root().join(format!("harness/scenarios/{NAME}.toml"))).unwrap();
    assert_eq!(scenario.frames, 1344);
    let raw = scenario.trace_path("tick.raw.jsonl");
    let pads = crate::trace::pad_script(&scenario)
        .unwrap()
        .with_display_from(&raw)
        .unwrap();
    let mut simulation = TestSimulation::with_inputs(
        InitialState::from_savestate_traces(&scenario).unwrap(),
        pads,
    );
    let mut rows = 0;
    let mut caught_rows = 0;
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
        let state = simulation.state();
        let fighter = &state.fighters[1].0;
        let bytes = row["fighters"][1]["bytes"].as_str().unwrap();
        let word = |o: usize| u32::from_str_radix(&bytes[o * 2..o * 2 + 8], 16).unwrap();
        assert_eq!(
            u32::from(fighter.motion_state.action.0),
            word(0x10),
            "tick{frame} action"
        );
        if frame == 1293 || frame == 1294 {
            caught_rows += 1;
            assert_eq!(fighter.motion_state.action, S::AttackAirB.into());
            assert_eq!(
                fighter.held_item.unwrap().kind,
                melee_types::ItemKind::Sword
            );
            assert!(fighter.item_catch_locked);
            assert!(fighter.commands.allow_interrupt);
            assert!(word(0x2218) & 0x8000_0000 != 0);
            assert!(matches!(
                fighter.state_data,
                melee_ft::fighter::MotionData::Aerial {
                    held_on_entry: false,
                    ..
                }
            ));
            for action in [S::AttackAirF, S::AttackAirB, S::AttackAirHi] {
                let mut entered = fighter.clone();
                assert!(matches!(
                    entered.state_data,
                    melee_ft::fighter::MotionData::Aerial {
                        held_on_entry: false,
                        ..
                    }
                ));
                melee_ft::fighter::attack::aerial::enter_action(
                    &mut entered,
                    &state.assets.fighters[1],
                    action.into(),
                )
                .unwrap();
                assert!(
                    entered.item_catch_locked,
                    "airtime lock survives the new entry"
                );
                assert!(matches!(
                    entered.state_data,
                    melee_ft::fighter::MotionData::Aerial {
                        held_on_entry: true,
                        ..
                    }
                ));
                let mut map = state.map.clone();
                let mut rng = state.rng;
                let entry_frame = entered.animation.frame.to_bits();
                expect_guard(
                    || {
                        (melee_ft::fighter::state::COMMON[action as usize].anim)(
                            &mut entered,
                            AnimationPhase {
                                assets: &state.assets.fighters[1],
                                map: &mut map,
                                rng: &mut rng,
                            },
                        )
                        .unwrap();
                    },
                    "held Sword aerial entry continuation",
                );
                assert_eq!(entered.animation.frame.to_bits(), entry_frame);
                expect_guard(
                    || {
                        (melee_ft::fighter::state::COMMON[action as usize].iasa)(
                            &mut entered,
                            InputPhase {
                                assets: &state.assets.fighters[1],
                            },
                        );
                    },
                    "held Sword aerial entry continuation",
                );
            }
        }
        rows += 1;
    }
    assert_eq!((rows, caught_rows), (1344, 2));
}
