//! Ordinary contacts discard earlier phantom logs even when a shell holds.
use super::*;
use crate::scenario::Scenario;
use std::{io::BufRead, path::PathBuf};

fn root() -> PathBuf {
    std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    )
}

#[test]
fn stone_mixed_contacts_keep_raw_shell_timer_hitlag_and_nudge_through_timeout() {
    for (name, ticks, contact) in [
        ("kirby_stone_phantom_fd_fox4", 360, 276),
        ("corpus_v3_ys_kirby_fox4_e45211ed5_p0", 699, 513),
        ("corpus_v3_ys_kirby_fox4_ee769e727_p2", 1505, usize::MAX),
        (
            "falco_shine_phantom_crouch_start_fd_falco_fox4",
            322,
            usize::MAX,
        ),
    ] {
        let scenario =
            Scenario::load(&root().join(format!("harness/scenarios/{name}.toml"))).unwrap();
        let raw = scenario.trace_path("tick.raw.jsonl");
        if !melee_test_support::require_files(
            scenario.required_files().into_iter().chain([raw.clone()]),
        ) {
            return;
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
        let mut count = 0;
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
            for port in 0..2 {
                let hex = row["fighters"][port]["bytes"].as_str().unwrap();
                let word = |o: usize| u32::from_str_radix(&hex[o * 2..o * 2 + 8], 16).unwrap();
                let f = &simulation.state().fighters[port].0;
                assert_eq!(
                    f.combat.hitlag_remaining.to_bits(),
                    word(0x195c),
                    "{name} tick{frame} p{port} hitlag"
                );
                assert_eq!(
                    f.combat.phantom_lockout.to_bits(),
                    word(0x189c),
                    "{name} tick{frame} p{port} phantom lockout"
                );
                assert_eq!(
                    f.combat.phantom_knockback.to_bits(),
                    word(0x18a0),
                    "{name} tick{frame} p{port} phantom knockback"
                );
                assert_eq!(u32::from(f.motion_state.action.0), word(0x10));
                for (value, offset) in [
                    (f.physics.player_nudge.x, 0xf8),
                    (f.physics.player_nudge.y, 0xfc),
                ] {
                    assert_eq!(
                        value.to_bits(),
                        word(offset),
                        "{name} tick{frame} p{port} nudge"
                    );
                }
                for (value, offset) in [
                    (f.physics.position.x, 0xb0),
                    (f.physics.position.y, 0xb4),
                    (f.physics.position.z, 0xb8),
                ] {
                    assert_eq!(
                        value.to_bits(),
                        word(offset),
                        "{name} tick{frame} p{port} position"
                    );
                }
                if port == 0 && (393..=398).contains(&f.motion_state.action.0) {
                    assert_eq!(
                        f.combat.shell_health.to_bits(),
                        word(0x1834),
                        "{name} tick{frame} shell"
                    );
                    let timer = (word(0x2340) >> 16) as u16 as i16;
                    assert_eq!(
                        f.character.get::<ft_kirby::init::Kirby>().stone.timer,
                        timer,
                        "{name} tick{frame} timer"
                    );
                }
            }
            if frame == contact {
                let victim = &simulation.state().fighters[0].0;
                let attacker = &simulation.state().fighters[1].0;
                assert!(victim.combat.phantom_log.is_empty());
                assert_eq!(victim.combat.hitlag_remaining, 0.0);
                for hit in attacker.commands.hitboxes.iter().flatten() {
                    assert!(hit.victims.contains(&victim.spawn_number));
                    assert!(hit.phantom_victims.contains(victim.spawn_number));
                }
                collision_controls(simulation.state().clone());
            }
            count += 1;
        }
        assert_eq!(count, ticks as usize);
    }
}

/// Reuse the actual contact pose, and isolate the two hitboxes. The first
/// contact must really append a phantom; clearing a synthetic log alone
/// would not exercise collision classification or retained victim histories.
fn collision_controls(state: InitialState) {
    for invincible in [false, true] {
        let mut state = state.clone();
        let (victim, attacker) = grab_pairs::pair(&mut state.fighters, 0, 1);
        let victim = &mut **victim.0;
        let attacker = &mut **attacker.0;
        victim.core.begin_hit_detection();
        victim.combat.shell_active = true;
        victim.combat.shell_health = 38.0;
        for hit in attacker.commands.hitboxes.iter_mut().flatten() {
            hit.victims.clear();
            hit.phantom_victims = Default::default();
        }
        // Keep active group members resident so the phantom's group-history
        // propagation is real; only their fighter-contact eligibility pauses.
        attacker.commands.hitboxes[1]
            .as_mut()
            .unwrap()
            .hits_fighters = false;
        attacker.commands.hitboxes[2]
            .as_mut()
            .unwrap()
            .hits_fighters = false;
        melee_ft::fighter::damage::detect_hit(
            victim,
            attacker,
            &state.assets.fighters[0],
            true,
            None,
            &mut state.rng,
        );
        assert!(
            !victim.combat.phantom_log.is_empty(),
            "actual first hitbox is a phantom"
        );
        assert_eq!(
            victim.combat.shell_health, 38.0,
            "phantom alone does not immediately damage shell"
        );
        assert!(victim.combat.hit_log.is_empty());
        assert!(!attacker.commands.hitboxes[0]
            .as_ref()
            .unwrap()
            .victims
            .contains(&victim.spawn_number));
        assert!(attacker.commands.hitboxes[0]
            .as_ref()
            .unwrap()
            .phantom_victims
            .contains(victim.spawn_number));
        // The earlier phantom remains valid until a subsequent ordinary
        // contact; the same probe also covers the invincibility early return.
        attacker.commands.hitboxes[1]
            .as_mut()
            .unwrap()
            .hits_fighters = true;
        let histories = attacker
            .commands
            .hitboxes
            .iter()
            .flatten()
            .map(|hit| hit.phantom_victims.clone())
            .collect::<Vec<_>>();
        if invincible {
            victim.status.revival_invincibility = 1;
        }
        melee_ft::fighter::damage::detect_hit(
            victim,
            attacker,
            &state.assets.fighters[0],
            true,
            None,
            &mut state.rng,
        );
        assert!(victim.combat.phantom_log.is_empty());
        assert!(victim.combat.hit_log.is_empty());
        assert!(victim.combat.shell_active);
        if invincible {
            assert_eq!(victim.combat.shell_health, 38.0);
        } else {
            assert!(victim.combat.shell_health < 38.0);
        }
        for (hit, history) in attacker.commands.hitboxes.iter().flatten().zip(histories) {
            assert!(hit.victims.contains(&victim.spawn_number));
            assert_eq!(
                hit.phantom_victims, history,
                "ordinary contact preserves existing history"
            );
            assert!(hit.phantom_victims.contains(victim.spawn_number));
        }
    }
}
