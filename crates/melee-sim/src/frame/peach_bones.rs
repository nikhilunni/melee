//! Tick-aligned local SRT oracle. No rendered Peach matrix capture exists.
use super::*;
use crate::scenario::Scenario;
use melee_ft::fighter::Fighter;
use serde_json::Value;
use std::{fs, path::Path};

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
fn replay(name: &str, ticks: usize) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let scenario = Scenario::load(&root.join(format!("harness/scenarios/{name}.toml"))).unwrap();
    let bones_path = scenario.trace_path("bones.jsonl");
    if !melee_test_support::require_files(
        scenario
            .required_files()
            .into_iter()
            .chain([bones_path.clone()]),
    ) {
        return;
    }
    let initial = InitialState::from_savestate_traces(&scenario).unwrap();
    let peach = &initial.fighters[0];
    let _ = peach.character.get::<ft_peach::init::Peach>();
    assert_eq!(peach.animation.parts.len(), 114);
    let dynamic_bones: usize = peach.dynamics.iter().map(|set| set.bones.len()).sum();
    assert_eq!(dynamic_bones, 45);
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
        assert_eq!(row["state"].as_object().unwrap().len(), (114 + 73) * 22);
        simulation.tick().unwrap();
        let runtime = &simulation.runtime;
        for (player, fighter) in runtime.state.fighters.iter().enumerate() {
            words +=
                crate::scene_fighter::with_fighter!(fighter, |f| compare(f, row, player, tick));
        }
    }
    eprintln!("{name}: 114 Peach bones ({dynamic_bones} dynamic), 73 Fox bones, {ticks} ticks, {words} SRT words, 0 matrix words (no rendered capture)");
}
#[test]
fn start_peach_bones_130() {
    replay("start_fd_peach", 130);
}
#[test]
fn idle_peach_bones_8() {
    replay("idle_fd_peach", 8);
}
