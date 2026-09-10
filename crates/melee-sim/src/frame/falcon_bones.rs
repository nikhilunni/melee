//! Tick-aligned local SRT oracle. No rendered CaptainFalcon matrix capture exists.
use super::*;
use crate::{scenario::Scenario, scene_fighter::SceneFighter};
use melee_ft::fighter::{CharacterCallbacks, Fighter};
use serde_json::Value;
use std::{fs, path::Path};

fn compare<C: CharacterCallbacks>(
    fighter: &Fighter<C>,
    expected: &Value,
    player: usize,
    tick: usize,
) -> usize {
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
fn replay(name: &str, ticks: usize) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let scenario = Scenario::load(&root.join(format!("harness/scenarios/{name}.toml"))).unwrap();
    let bones_path = scenario.trace_path("bones.jsonl");
    if let Some(missing) = scenario
        .required_files()
        .into_iter()
        .chain([bones_path.clone()])
        .find(|p| !p.is_file())
    {
        eprintln!(
            "skipping local Captain Falcon bones: {} absent",
            missing.display()
        );
        return;
    }
    let initial = InitialState::from_savestate_traces(&scenario).unwrap();
    let SceneFighter::CaptainFalcon(falcon) = &initial.fighters[0] else {
        panic!("CaptainFalcon slot")
    };
    assert_eq!(falcon.animation.parts.len(), 63);
    let dynamic_bones: usize = falcon.dynamics.iter().map(|set| set.bones.len()).sum();
    eprintln!(
        "Captain Falcon dynamic chains: {:?}",
        initial.assets.fighters[0]
            .dynamics
            .iter()
            .map(|s| (s.root, s.springs.len()))
            .collect::<Vec<_>>()
    );
    assert_eq!(dynamic_bones, 0);
    let rows: Vec<Value> = fs::read_to_string(bones_path)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(rows.len(), ticks);
    let mut simulation = Simulation::new(initial);
    let mut words = 0;
    for (tick, row) in rows.iter().enumerate() {
        assert_eq!(row["frame"].as_u64(), Some(tick as u64));
        assert_eq!(row["state"].as_object().unwrap().len(), (63 + 73) * 22);
        simulation.tick().unwrap();
        let runtime = simulation.runtime.borrow();
        for (player, fighter) in runtime.state.fighters.iter().enumerate() {
            words +=
                crate::scene_fighter::with_fighter!(fighter, |f| compare(f, row, player, tick));
        }
    }
    eprintln!("{name}: 63 Captain Falcon bones ({dynamic_bones} dynamic), 73 Fox bones, {ticks} ticks, {words} SRT words, 0 matrix words (no rendered capture)");
}
#[test]
fn start_falcon_bones_130() {
    replay("start_fd_falcon", 130);
}
#[test]
fn idle_falcon_bones_8() {
    replay("idle_fd_falcon", 8);
}

/// The idle save stops before the last particle is allocated. Check the entire
/// particle population after resuming, including tick zero, to independently
/// verify the imported stack operands and partial-generator completion.
#[test]
fn idle_falcon_partial_emission_particles_600() {
    use crate::initial_state::particles;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let scenario = Scenario::load(&root.join("harness/scenarios/idle_fd_falcon.toml")).unwrap();
    let path = scenario.trace_path("particles.jsonl");
    if let Some(missing) = scenario
        .required_files()
        .into_iter()
        .chain([path.clone()])
        .find(|p| !p.is_file())
    {
        eprintln!(
            "skipping local Falcon particles: {} absent",
            missing.display()
        );
        return;
    }
    let expected =
        melee_diff::read_trace(std::io::BufReader::new(fs::File::open(path).unwrap())).unwrap();
    assert_eq!(expected.len(), 600);
    let initial = InitialState::from_savestate_traces(&scenario).unwrap();
    // Whether the saved boundary interrupted a particle emission depends on the
    // recording (the 2026-09-09 savestate did, the re-recorded one does not);
    // the replay must match either way, and the emission must be consumed by
    // the first tick when present.
    eprintln!(
        "idle_fd_falcon boundary pending emission: {}",
        initial.pending_emission.is_some()
    );
    let mut simulation = Simulation::new(initial);
    let mut words = 0;
    for row in expected {
        simulation.tick().unwrap();
        let runtime = simulation.runtime.borrow();
        let state = &runtime.state;
        assert!(state.pending_emission.is_none());
        let actual = particles::snapshot(
            &state.particles,
            state.rng.seed,
            row.frame,
            &state.assets.particle_bank,
        );
        particles::assert_state(&row, &actual);
        words += row.state.len();
    }
    eprintln!("idle_fd_falcon particles: 600 ticks, {words} fields, 0 mismatches");
}
