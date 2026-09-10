//! State-callback replays; m4_gate separately verifies the complete scene/RNG.
mod fighter_support;
use fighter_support::replay::replay_state_callbacks;

#[test]
fn squat_fox_state_callbacks() {
    replay_state_callbacks("squat", "ledger");
}
#[test]
fn turn_fox_state_callbacks() {
    replay_state_callbacks("turn", "ledger");
}
#[test]
fn walk_fox_state_callbacks() {
    replay_state_callbacks("walk", "ledger");
}

#[test]
fn dash_fox_state_callbacks() {
    replay_state_callbacks("dash", "ledger");
}

#[test]
fn jump_fox_state_callbacks() {
    replay_state_callbacks("jump", "ledger");
}

/// Exercise both release decisions through real input and animation callbacks.
/// The supplied retail scenario independently validates the short-hop trajectory.
#[test]
fn holding_jump_full_hops_and_release_then_repress_stays_a_short_hop() {
    use fighter_support::{harness, json_lines, raw, Fixture};
    use gekko_math::HsdRng;
    use hsd_types::Vec3;
    use melee_ft::{
        fighter::MotionData,
        input::{Buttons, PadSample, Stick},
    };
    use melee_types::CommonMotionState as S;
    let path = harness().join("traces/jump_fd_fox.tick.raw.jsonl");
    if !melee_test_support::require_files([&path]) {
        return;
    }
    let Some(fixture) = Fixture::load() else {
        return;
    };
    let boundary = raw(&json_lines(&path)[0], 0);
    for stick_jump in [false, true] {
        for release in [false, true] {
            let mut fighter = fixture.import(&boundary);
            let held = if stick_jump {
                PadSample {
                    stick: Stick { x: 0.0, y: 1.0 },
                    ..Default::default()
                }
            } else {
                PadSample {
                    buttons: Buttons::X,
                    ..Default::default()
                }
            };
            let neutral = PadSample::default();
            let mut rng = HsdRng::new(1);
            fighter.proc_input(&fixture.assets, &held);
            assert_eq!(fighter.motion_state.id, S::KneeBend);
            for tick in 1..=3 {
                fighter.proc_anim(&fixture.assets, &mut rng).unwrap();
                if fighter.motion_state.id == S::JumpF {
                    break;
                }
                fighter.proc_input(
                    &fixture.assets,
                    if release && tick == 1 {
                        &neutral
                    } else {
                        &held
                    },
                );
                fighter.proc_update(&fixture.assets, &fixture.map, Vec3::ZERO);
            }
            assert_eq!(fighter.motion_state.id, S::JumpF);
            let MotionData::Jump(jump) = &fighter.state_data else {
                panic!("jump scratch missing")
            };
            assert_eq!(jump.short_hop, release);
            assert!(!jump.physics_started);
            let attrs = &fixture.assets.attributes;
            let launch_speed = if release {
                attrs.jumping.hop_v_initial_velocity
            } else {
                attrs.jumping.jump_v_initial_velocity
            };
            assert_eq!(
                fighter.physics.self_velocity.y.to_bits(),
                launch_speed.to_bits()
            );
            fighter.proc_update(&fixture.assets, &fixture.map, Vec3::ZERO);
            assert_eq!(
                fighter.physics.self_velocity.y.to_bits(),
                launch_speed.to_bits(),
                "first Jump Phys skips gravity"
            );
            fighter.proc_update(&fixture.assets, &fixture.map, Vec3::ZERO);
            assert_eq!(
                fighter.physics.self_velocity.y.to_bits(),
                (launch_speed - attrs.air.gravity).to_bits()
            );
        }
    }
}

#[test]
fn shield_fox_state_callbacks() {
    replay_state_callbacks("shield", "ledger");
}

#[test]
fn spotdodge_fox_state_callbacks() {
    replay_state_callbacks("spotdodge", "ledger");
}

#[test]
fn roll_fox_state_callbacks() {
    replay_state_callbacks("roll", "ledger");
}

#[test]
fn airdodge_fox_state_callbacks() {
    replay_state_callbacks("airdodge", "ledger");
}

#[test]
fn wavedash_fox_state_callbacks() {
    replay_state_callbacks("wavedash", "ledger");
}

#[test]
fn ledge_fox_state_callbacks() {
    replay_state_callbacks("ledge", "ledger");
}

/// Follow the recorded approach using only its initial boundary and pad inputs,
/// then vary the grab gates. A one-frame cooldown expires before collision.
#[test]
fn ledge_grab_respects_cooldown_down_input_and_disable_flag() {
    use fighter_support::{harness, json_lines, raw, replay::recorded_pad, Fixture};
    use gekko_math::HsdRng;
    use hsd_types::Vec3;
    use melee_types::CommonMotionState as S;
    let path = harness().join("traces/ledge_fd_fox.tick.raw.jsonl");
    let pads = harness().join("traces/ledge_fd_fox.tick.expected.jsonl");
    if !melee_test_support::require_files([&path, &pads]) {
        return;
    }
    let Some(mut fixture) = Fixture::load() else {
        return;
    };
    let raw_trace = json_lines(&path);
    let boundary = raw(&raw_trace[0], 0);
    let catch_tick = raw_trace
        .iter()
        .position(|row| fighter_support::word(&raw(row, 0), 0x10) == S::CliffCatch as u32)
        .expect("recording reaches CliffCatch");
    let initial_facing = fighter_support::word(&boundary, 0x2C);
    let pads = json_lines(&pads);
    for (cooldown, down, disabled, catches) in [
        (0, false, false, true),
        (1, false, false, true),
        (2, false, false, false),
        (0, true, false, false),
        (0, false, true, false),
    ] {
        let mut fighter = fixture.import(&boundary);
        let mut rng = HsdRng::new(1);
        for (tick, pad) in pads.iter().enumerate().take(catch_tick + 1).skip(1) {
            fighter.proc_anim(&fixture.assets, &mut rng).unwrap();
            fighter.proc_input(&fixture.assets, &recorded_pad(pad, 0));
            if tick == catch_tick {
                fighter.status.ledge_cooldown = cooldown;
            }
            fighter.proc_update(&fixture.assets, &fixture.map, Vec3::ZERO);
            if tick == catch_tick {
                fighter.status.ledge_grab_disabled = disabled;
                if down {
                    fighter.input.current.stick.y = -fixture.assets.ledge.grab_down_threshold;
                }
            }
            fighter
                .proc_map_with_assets(&fixture.assets, &mut fixture.map, &mut rng)
                .unwrap();
            fighter.resolve_graphics_commands(&fixture.assets, &mut rng);
        }
        assert_eq!(
            fighter.motion_state.id,
            if catches { S::CliffCatch } else { S::JumpB }
        );
        assert_eq!(fighter.physics.facing.to_bits(), initial_facing);
        assert_eq!(fighter.status.ledge_cooldown, (cooldown - 1).max(0));
    }
}

#[test]
fn turnrun_fox_state_callbacks() {
    replay_state_callbacks("turnrun", "ledger");
}

#[test]
fn walkfast_fox_state_callbacks() {
    replay_state_callbacks("walkfast", "ledger");
}

#[test]
fn ledgeclimb_fox_state_callbacks() {
    replay_state_callbacks("ledgeclimb", "ledger");
}

#[test]
fn ledgeescape_fox_state_callbacks() {
    replay_state_callbacks("ledgeescape", "ledger");
}
