//! Recovery SRT and typed move scratch supplement fighter/item/RNG gates.
use super::*;
use crate::scenario::Scenario;
use melee_ft::fighter::Fighter;
use serde_json::Value;
use std::{fs, path::Path};

const SCENARIOS: [&str; 5] = [
    "illusion_start_landing_fd_fox",
    "firefox_charge_landing_fd_fox",
    "firefox_ground_launch_fd_fox",
    "firefox_floor_rebound_fd_fox",
    "firefox_end_air_landing_fd_fox",
];

fn compare(fighter: &Fighter, expected: &Value, player: usize, tick: usize) -> usize {
    let mut compared = 0;
    for (bone, part) in fighter.animation.parts.iter().enumerate() {
        let joint = fighter.skeleton.get(part.joint);
        let rotation = [
            joint.rotate.x,
            joint.rotate.y,
            joint.rotate.z,
            joint.rotate.w,
        ];
        let scale = [joint.scale.x, joint.scale.y, joint.scale.z];
        let translate = [joint.translate.x, joint.translate.y, joint.translate.z];
        for (field, values) in [
            ("rotate", &rotation[..]),
            ("scale", &scale[..]),
            ("translate", &translate[..]),
        ] {
            for (index, value) in values.iter().enumerate() {
                // Same contract as start_fox_bones_130: unused Euler W is stack
                // data. Every quaternion component is compared when enabled.
                if field == "rotate"
                    && index == 3
                    && joint.flags & hsd_anim::jobj::JOBJ_USE_QUATERNION == 0
                {
                    continue;
                }
                let key = format!("p{player}.bone[{bone}].{field}[{index}]");
                let bits = expected["state"][&key]["v"]["bits"].as_u64().unwrap() as u32;
                assert_eq!(
                    value.to_bits(),
                    bits,
                    "tick {tick} {key}: actual {:08X}, expected {bits:08X}",
                    value.to_bits()
                );
                compared += 1;
            }
        }
    }
    compared
}

#[test]
fn recovery_collisions_match_all_local_bone_transforms() {
    for name in SCENARIOS {
        replay_bones(name, 150);
    }
}

fn replay_bones(name: &str, frames: usize) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let scenario = Scenario::load(&root.join(format!("harness/scenarios/{name}.toml"))).unwrap();
    let path = scenario.trace_path("bones.jsonl");
    if !melee_test_support::require_files(
        scenario.required_files().into_iter().chain([path.clone()]),
    ) {
        return;
    }
    let initial = InitialState::from_savestate_traces(&scenario).unwrap();
    let mut simulation =
        super::TestSimulation::with_inputs(initial, crate::trace::pad_script(&scenario).unwrap());
    let bones = fs::read_to_string(path).unwrap();
    assert_eq!(bones.lines().count(), frames);
    for (tick, line) in bones.lines().enumerate() {
        let row: Value = serde_json::from_str(line).unwrap();
        assert_eq!(row["frame"].as_u64(), Some(tick as u64));
        simulation.tick().unwrap();
        for (player, fighter) in simulation.runtime.state.fighters.iter().enumerate() {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                compare(&fighter.0, &row, player, tick)
            }));
            assert!(
                result.is_ok(),
                "{name}: bone divergence at {tick}, player {player}"
            );
        }
    }
}
#[test]
fn common_input_bones_taunt_fd_fox() {
    replay_bones("taunt_fd_fox", 150);
}
#[test]
fn common_input_bones_taunt_fd_marth() {
    replay_bones("taunt_fd_marth", 150);
}
#[test]
fn common_input_bones_dash_escape_fd_fox() {
    replay_bones("dash_escape_fd_fox", 150);
}
#[test]
fn common_input_bones_dash_escape_fd_marth() {
    replay_bones("dash_escape_fd_marth", 150);
}
#[test]
fn common_input_bones_dash_shield_fd_fox() {
    replay_bones("dash_shield_fd_fox", 150);
}
#[test]
fn common_input_bones_dash_shield_fd_marth() {
    replay_bones("dash_shield_fd_marth", 150);
}
#[test]
fn common_input_bones_dash_taunt_fd_fox() {
    replay_bones("dash_taunt_fd_fox", 150);
}
#[test]
fn common_input_bones_dash_taunt_fd_marth() {
    replay_bones("dash_taunt_fd_marth", 150);
}
#[test]
fn common_input_bones_dash_late_shield_fd_fox() {
    replay_bones("dash_late_shield_fd_fox", 150);
}
#[test]
fn common_input_bones_dash_late_shield_fd_marth() {
    replay_bones("dash_late_shield_fd_marth", 150);
}
#[test]
fn common_input_bones_shield_grab_fd_fox() {
    replay_bones("shield_grab_fd_fox", 150);
}
#[test]
fn common_input_bones_shield_grab_fd_marth() {
    replay_bones("shield_grab_fd_marth", 150);
}
#[test]
fn common_input_bones_dash_late_shield_grab_fd_fox() {
    replay_bones("dash_late_shield_grab_fd_fox", 150);
}
#[test]
fn common_input_bones_dash_late_shield_grab_fd_marth() {
    replay_bones("dash_late_shield_grab_fd_marth", 150);
}
#[test]
fn common_input_bones_shield_cstick_jump_fd_fox() {
    replay_bones("shield_cstick_jump_fd_fox", 150);
}
#[test]
fn common_input_bones_shield_cstick_jump_fd_marth() {
    replay_bones("shield_cstick_jump_fd_marth", 150);
}
#[test]
fn common_input_bones_shield_delayed_power_fd_fox() {
    replay_bones("shield_delayed_power_fd_fox", 150);
}
#[test]
fn common_input_bones_shield_delayed_power_fd_marth() {
    replay_bones("shield_delayed_power_fd_marth", 150);
}

#[test]
fn common_input_bones_run_shield_fd_fox() {
    replay_bones("run_shield_fd_fox", 150);
}

#[test]
fn common_input_bones_run_shield_fd_marth() {
    replay_bones("run_shield_fd_marth", 150);
}

#[test]
fn common_input_bones_run_taunt_fd_fox() {
    replay_bones("run_taunt_fd_fox", 150);
}

#[test]
fn common_input_bones_run_taunt_fd_marth() {
    replay_bones("run_taunt_fd_marth", 150);
}

#[test]
fn common_input_bones_run_shield_grab_fd_fox() {
    replay_bones("run_shield_grab_fd_fox", 150);
}

#[test]
fn common_input_bones_run_shield_grab_fd_marth() {
    replay_bones("run_shield_grab_fd_marth", 150);
}

#[test]
fn capture_revival_bones_rebirth_timeout_fd_fox() {
    replay_bones("rebirth_timeout_fd_fox", 900);
}

#[test]
fn capture_revival_bones_rebirth_timeout_fd_marth() {
    replay_bones("rebirth_timeout_fd_marth", 900);
}

#[test]
fn capture_revival_bones_rebirth_shield_a_fd_fox() {
    replay_bones("rebirth_shield_a_fd_fox", 600);
}

#[test]
fn capture_revival_bones_rebirth_analog_shield_a_fd_fox() {
    replay_bones("rebirth_analog_shield_a_fd_fox", 600);
}

#[test]
fn capture_revival_bones_rebirth_held_shield_a_fd_fox() {
    replay_bones("rebirth_held_shield_a_fd_fox", 600);
}

#[test]
fn capture_revival_bones_grab_airborne_fd_foxmarth() {
    replay_bones("grab_airborne_fd_foxmarth", 600);
}

#[test]
fn capture_revival_bones_grab_airborne_fd_marthfox() {
    replay_bones("grab_airborne_fd_marthfox", 600);
}

#[test]
fn ledge_input_bones_ledge_cstick_attack_fd_fox() {
    replay_bones("ledge_cstick_attack_fd_fox", 300);
}

#[test]
fn ledge_input_bones_ledge_cstick_attack_fd_marth() {
    replay_bones("ledge_cstick_attack_fd_marth", 300);
}

#[test]
fn ledge_input_bones_ledge_cstick_escape_fd_fox() {
    replay_bones("ledge_cstick_escape_fd_fox", 300);
}

#[test]
fn ledge_input_bones_ledge_cstick_escape_fd_marth() {
    replay_bones("ledge_cstick_escape_fd_marth", 300);
}

#[test]
fn ledge_input_bones_ledge_cstick_drop_fd_fox() {
    replay_bones("ledge_cstick_drop_fd_fox", 300);
}

#[test]
fn ledge_input_bones_ledge_cstick_drop_fd_marth() {
    replay_bones("ledge_cstick_drop_fd_marth", 300);
}

#[test]
fn ledge_input_bones_ledge_cstick_priority_fd_fox() {
    replay_bones("ledge_cstick_priority_fd_fox", 300);
}

#[test]
fn ledge_input_bones_ledge_cstick_priority_fd_marth() {
    replay_bones("ledge_cstick_priority_fd_marth", 300);
}

#[test]
fn ledge_input_bones_ledge_timeout_fd_fox() {
    replay_bones("ledge_timeout_fd_fox", 1200);
}

#[test]
fn ledge_input_bones_ledge_timeout_fd_marth() {
    replay_bones("ledge_timeout_fd_marth", 1200);
}

#[test]
fn fire_contact_bones_firefox_charge_hit_fd_marth() {
    replay_bones("firefox_charge_hit_fd_marth", 300);
}

#[test]
fn fire_contact_bones_firefox_travel_hit_fd_marth() {
    replay_bones("firefox_travel_hit_fd_marth", 300);
}

#[test]
fn fire_contact_particle_simulation_fields_match_retail() {
    use crate::initial_state::particles;
    struct Banks<'a>(&'a InitialState);
    impl particles::Banks for Banks<'_> {
        fn bank(&self, id: u8) -> &hsd_particle::bank::ParticleBank {
            match id {
                0 => &self.0.assets.common_particle_bank,
                30 => &self.0.assets.particle_bank,
                _ => self.0.particles.bank(id).expect("loaded effect bank"),
            }
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for name in ["firefox_charge_hit_fd_marth", "firefox_travel_hit_fd_marth"] {
        let scenario =
            Scenario::load(&root.join(format!("harness/scenarios/{name}.toml"))).unwrap();
        let path = scenario.trace_path("particles.jsonl");
        if !melee_test_support::require_files(
            scenario.required_files().into_iter().chain([path.clone()]),
        ) {
            return;
        }
        let initial = InitialState::from_savestate_traces(&scenario).unwrap();
        let mut simulation = super::TestSimulation::with_inputs(
            initial,
            crate::trace::pad_script(&scenario).unwrap(),
        );
        let expected =
            melee_diff::read_trace(std::io::BufReader::new(fs::File::open(path).unwrap())).unwrap();
        assert_eq!(expected.len(), 300);
        for mut record in expected {
            simulation.tick().unwrap();
            let state = &simulation.runtime.state;
            let mut actual = particles::snapshot(
                &state.particles,
                state.rng.seed,
                record.frame,
                &Banks(state),
            );
            // psDispSubAppSRT owns camera-dependent caches and the display
            // frame tag. Compare authored SRT and lifetimes here; psdisp's
            // separate reference oracles cover those renderer caches.
            let simulation_field = |key: &String, _: &mut melee_diff::Value| {
                !key.starts_with("particles.appsrt[")
                    || [".translation[", ".rotation[", ".scale[", ".use_count"]
                        .iter()
                        .any(|field| key.contains(field))
            };
            actual.state.retain(simulation_field);
            record.state.retain(simulation_field);
            if let Some(diff) = melee_diff::first_divergence([&record], [&actual]) {
                panic!("{name}: {diff}");
            }
        }
    }
}
