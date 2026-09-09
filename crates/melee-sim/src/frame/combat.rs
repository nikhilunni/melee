//! Raw fighter scratch assertions supplement M5's 49-key gate. Only the initial
//! state and pad samples drive simulation; subsequent retail bytes are assertions.
use super::*;
use crate::{scenario::Scenario, scene_fighter::SceneFighter};
use melee_ft::fighter::{hitbox::CapsulePhase, CharacterCallbacks, Fighter, MotionData};
use std::{fs, path::Path};

fn word(bytes: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap())
}
fn vector(v: Vec3, bytes: &[u8], offset: usize) {
    for (axis, value) in [v.x, v.y, v.z].into_iter().enumerate() {
        assert_eq!(
            value.to_bits(),
            word(bytes, offset + axis * 4),
            "vector {offset:#x} axis {axis}"
        );
    }
}
fn compare<C: CharacterCallbacks>(f: &Fighter<C>, bytes: &[u8]) {
    assert_eq!(
        f.combat.hitlag_remaining.to_bits(),
        word(bytes, 0x195c),
        "hitlag countdown"
    );
    if let MotionData::Damage(damage) = &f.state_data {
        assert_eq!(
            damage.hitstun.to_bits(),
            word(bytes, 0x2340),
            "hitstun countdown"
        );
    }
    for (id, hit) in f.commands.hitboxes.iter().enumerate() {
        let offset = 0x914 + id * 0x138;
        let Some(hit) = hit else {
            assert_eq!(word(bytes, offset), 0, "disabled hitbox {id}");
            continue;
        };
        let phase = match hit.phase {
            CapsulePhase::Enabled => 1,
            CapsulePhase::FirstPosition => 2,
            CapsulePhase::Sweeping => 3,
        };
        assert_eq!(phase, word(bytes, offset), "hitbox phase");
        assert_eq!(
            hit.descriptor.damage.to_bits(),
            word(bytes, offset + 12),
            "hitbox damage"
        );
        assert_eq!(
            hit.descriptor.radius.to_bits(),
            word(bytes, offset + 0x1c),
            "hitbox radius"
        );
        vector(hit.descriptor.offset, bytes, offset + 0x10);
        vector(hit.position, bytes, offset + 0x4c);
        vector(hit.previous_position, bytes, offset + 0x58);
    }
}
#[test]
fn jab_hitboxes_hitlag_and_hitstun_match_retail_scratch() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let scenario = Scenario::load(&root.join("harness/scenarios/jab_fd_marth.toml")).unwrap();
    let path = scenario.trace_path("tick.raw.jsonl");
    if let Some(missing) = scenario
        .required_files()
        .into_iter()
        .chain([path.clone()])
        .find(|p| !p.is_file())
    {
        eprintln!("skipping jab scratch: {} absent", missing.display());
        return;
    }
    let mut simulation = Simulation::with_inputs(
        InitialState::from_savestate_traces(&scenario).unwrap(),
        crate::trace::pad_script(&scenario).unwrap(),
    );
    let raw = fs::read_to_string(path).unwrap();
    assert_eq!(raw.lines().count(), 300);
    for (tick, line) in raw.lines().enumerate() {
        let row: serde_json::Value = serde_json::from_str(line).unwrap();
        simulation.tick().unwrap();
        let runtime = simulation.runtime.borrow();
        for slot in 0..2 {
            let bytes: Vec<u8> = row["fighters"][slot]["bytes"]
                .as_str()
                .unwrap()
                .as_bytes()
                .chunks_exact(2)
                .map(|s| u8::from_str_radix(std::str::from_utf8(s).unwrap(), 16).unwrap())
                .collect();
            assert_eq!(bytes[12], slot as u8, "slot order tick {tick}");
            crate::scene_fighter::with_fighter!(&runtime.state.fighters[slot], |f| compare(
                f, &bytes
            ));
        }
    }
}
