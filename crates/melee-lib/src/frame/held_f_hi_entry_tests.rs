//! Complete common F/Hi entry captures; subsequent callbacks are refused.
use super::*;
use crate::scenario::Scenario;
use melee_ft::fighter::state::{AnimationPhase, InputPhase};
use melee_types::ItemKind;
use std::{io::BufRead, path::PathBuf};

fn root() -> PathBuf {
    std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    )
}
fn expect_guard(f: impl FnOnce(), target: &str) {
    let error = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).unwrap_err();
    let text = error
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| error.downcast_ref::<&str>().copied())
        .unwrap();
    assert!(text.contains(target), "{text}");
    assert!(
        text.contains("held Sword aerial entry continuation"),
        "{text}"
    );
}

#[test]
fn complete_forward_and_up_entries_preserve_raw_owned_state_and_refuse_callbacks() {
    for (name, frames, actor, action) in [
        ("corpus_v3_bf_items_sword_ea3ec9208_p1", 314, 1, 66),
        ("corpus_v3_fd_items_sword_e00f31913_p2", 1502, 1, 68),
        ("corpus_v3_fd_items_sword_e1502cb40_p0", 1116, 1, 66),
        ("corpus_v3_fd_items_sword_edb13a9c3_p2", 568, 1, 66),
        ("corpus_v3_fd_items_sword_edb4b01fd_p1", 1073, 1, 68),
        ("corpus_v3_fd_items_sword_ee92d4a97_p0", 834, 0, 66),
        ("corpus_v3_fd_items_sword_eecfcf1ea_p2", 776, 1, 68),
        ("corpus_v3_fd_items_sword_nocont_e05625e18_p2", 1384, 0, 68),
        ("corpus_v3_fd_items_sword_nocont_e89a89d0e_p2", 727, 0, 66),
        ("corpus_v3_fd_items_sword_nocont_ea4d5d9ec_p2", 880, 1, 66),
        ("corpus_v3_fd_items_sword_nocont_ee62c6106_p1", 1052, 0, 68),
        ("corpus_v3_fd_items_taru_e8d30e8ed_p1", 1160, 1, 68),
        ("corpus_v3_fd_items_taru_ea9c3e366_p1", 1247, 1, 68),
    ] {
        let scenario =
            Scenario::load(&root().join(format!("harness/scenarios/{name}.toml"))).unwrap();
        assert_eq!(scenario.frames, frames);
        let raw = scenario.trace_path("tick.raw.jsonl");
        assert!(melee_test_support::require_files(
            scenario.required_files().into_iter().chain([raw.clone()])
        ));
        let pads = crate::trace::pad_script(&scenario)
            .unwrap()
            .with_display_from(&raw)
            .unwrap();
        let mut simulation = TestSimulation::with_inputs(
            InitialState::from_savestate_traces(&scenario).unwrap(),
            pads,
        );
        let mut previous_identity: Option<(u32, u32, hsd_gobj::GObjAddress)> = None;
        let mut rows = 0;
        let mut entries = 0;
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
            let fighter = &state.fighters[actor].0;
            let bytes = row["fighters"][actor]["bytes"].as_str().unwrap();
            let word = |o: usize| u32::from_str_radix(&bytes[o * 2..o * 2 + 8], 16).unwrap();
            assert_eq!(
                u32::from(fighter.motion_state.action.0),
                word(0x10),
                "{name} tick{frame} action"
            );
            assert_eq!(
                fighter.animation.frame.to_bits(),
                word(0x894),
                "{name} tick{frame} frame"
            );
            assert_eq!(
                i32::from(fighter.physics.ground_or_air) as u32,
                word(0xe0),
                "{name} tick{frame} air"
            );
            for (v, o) in [
                (fighter.physics.position.x, 0xb0),
                (fighter.physics.position.y, 0xb4),
                (fighter.physics.position.z, 0xb8),
                (fighter.physics.self_velocity.x, 0x80),
                (fighter.physics.self_velocity.y, 0x84),
                (fighter.physics.self_velocity.z, 0x88),
                (fighter.physics.knockback_velocity.x, 0x8c),
                (fighter.physics.knockback_velocity.y, 0x90),
                (fighter.physics.knockback_velocity.z, 0x94),
                (fighter.physics.facing, 0x2c),
                (fighter.combat.hitlag_remaining, 0x195c),
            ] {
                assert_eq!(v.to_bits(), word(o), "{name} tick{frame} +{o:x}");
            }
            if (65..=69).contains(&fighter.motion_state.action.0) {
                assert_eq!(
                    fighter.commands.allow_interrupt,
                    word(0x2218) & 0x8000_0000 != 0,
                    "{name} tick{frame} active interrupt"
                );
                assert_eq!(
                    fighter.commands.variables[0] as u32,
                    word(0x2200),
                    "{name} tick{frame} active command0"
                );
            }
            assert_eq!(
                fighter.held_item.is_some(),
                word(0x1974) != 0,
                "{name} tick{frame} held handle presence"
            );
            let live = state
                .items
                .iter()
                .filter(|i| !i.destroyed)
                .collect::<Vec<_>>();
            let raw_items = row["items"].as_array().unwrap();
            assert_eq!(live.len(), raw_items.len());
            let mut identity = None;
            for (item, raw_item) in live.iter().zip(raw_items) {
                let hex = raw_item["bytes"].as_str().unwrap();
                let iw = |o: usize| u32::from_str_radix(&hex[o * 2..o * 2 + 8], 16).unwrap();
                assert_eq!(item.kind as u32, iw(0x10));
                assert_eq!(u32::from(item.motion), iw(0x24));
                assert_eq!(item.owner.map(u64::from), raw_item["owner"].as_u64());
                assert_eq!(item.held, iw(0xdc8) & (1 << 12) != 0);
                assert_eq!(item.life_timer.to_bits(), iw(0xd44));
                for (v, o) in [
                    (item.position.x, 0x4c),
                    (item.position.y, 0x50),
                    (item.position.z, 0x54),
                ] {
                    assert_eq!(v.to_bits(), iw(o), "{name} tick{frame} item+{o:x}");
                }
                if item.held && item.kind == ItemKind::Sword {
                    let owner = usize::from(item.owner.unwrap());
                    let owner_bytes = row["fighters"][owner]["bytes"].as_str().unwrap();
                    let ow =
                        |o: usize| u32::from_str_radix(&owner_bytes[o * 2..o * 2 + 8], 16).unwrap();
                    let raw_gobj = u32::from_str_radix(
                        raw_item["gobj"].as_str().unwrap().trim_start_matches("0x"),
                        16,
                    )
                    .unwrap();
                    assert_eq!(iw(0x518), ow(0));
                    assert_eq!(ow(0x1974), raw_gobj);
                    assert_eq!(state.fighters[owner].0.held_item.unwrap().item, item.id);
                    assert_eq!(u32::from(item.holder_part), iw(0xdc4));
                    assert_eq!(
                        item.holder_part,
                        state.assets.fighters[owner]
                            .bones
                            .model
                            .animation_translation
                    );
                    let visible = u8::from_str_radix(&owner_bytes[0x221e * 2..0x221e * 2 + 2], 16)
                        .unwrap()
                        & 0x10
                        != 0;
                    assert_eq!(!state.fighters[owner].0.commands.held_item_hidden, visible);
                    if owner == actor {
                        identity = Some((raw_gobj, item.id, item.gobj));
                    }
                }
            }
            if let (Some(old), Some(current)) = (previous_identity, identity) {
                if old.0 == current.0 {
                    assert_eq!(old, current, "{name} tick{frame} same held identity");
                }
            }
            if frame + 1 == frames as usize {
                entries += 1;
                assert_eq!(fighter.motion_state.action.0, action);
                assert_eq!(fighter.animation.frame, 1.0);
                assert_eq!(word(0x2210), 0);
                assert!(!fighter.commands.allow_interrupt);
                assert!(
                    !fighter.commands.grab_release
                        && !fighter.commands.throw_reverse
                        && !fighter.commands.rapid_jab_loop_end
                );
                assert_eq!(fighter.held_item.unwrap().kind, ItemKind::Sword);
                assert!(matches!(
                    fighter.state_data,
                    melee_ft::fighter::MotionData::Aerial {
                        held_on_entry: true,
                        ..
                    }
                ));
                assert!(identity.is_some());
                assert_eq!(
                    identity, previous_identity,
                    "entry preserves the held identity"
                );
            }
            previous_identity = identity;
            rows += 1;
        }
        assert_eq!((rows, entries), (frames as usize, 1));
        let state = simulation.state();
        for animation in [true, false] {
            let mut fighter = state.fighters[actor].0.clone();
            let mut map = state.map.clone();
            let mut rng = state.rng;
            let saved = (
                fighter.animation.frame.to_bits(),
                fighter.animation.remainder.to_bits(),
                fighter.physics.facing.to_bits(),
                fighter.combat.hitlag_remaining.to_bits(),
                format!("{:?}", fighter.commands),
                fighter.effects.clone(),
            );
            let row = melee_ft::fighter::state::COMMON[action as usize];
            expect_guard(
                || {
                    if animation {
                        (row.anim)(
                            &mut fighter,
                            AnimationPhase {
                                assets: &state.assets.fighters[actor],
                                map: &mut map,
                                rng: &mut rng,
                            },
                        )
                        .unwrap();
                    } else {
                        (row.iasa)(
                            &mut fighter,
                            InputPhase {
                                assets: &state.assets.fighters[actor],
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
            assert_eq!(
                (
                    fighter.animation.frame.to_bits(),
                    fighter.animation.remainder.to_bits(),
                    fighter.physics.facing.to_bits(),
                    fighter.combat.hitlag_remaining.to_bits(),
                    format!("{:?}", fighter.commands),
                    fighter.effects.clone()
                ),
                saved
            );
        }
        expect_guard(
            || {
                simulation.tick_without_snapshot().unwrap();
            },
            "ftCo_AttackAir_Anim",
        );
    }
}
