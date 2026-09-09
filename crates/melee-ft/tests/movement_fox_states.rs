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
    if !path.exists() {
        eprintln!("skipping: local jump boundary absent");
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
