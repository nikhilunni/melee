//! Additional raw item fields distinguish reflection from an ordinary laser hit.
use super::*;
use crate::scenario::Scenario;
use std::{collections::BTreeMap, path::Path};

fn word(bytes: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

#[test]
fn laser_reflection_damage_history_and_original_identity() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for name in [
        "laser_reflect_fresh_fd_marth",
        "laser_reflect_return_boundary_fd_marth",
        "laser_reflect_stale_fd_marth",
        "laser_reflect_delayed_timed_fd_marth",
    ] {
        let scenario =
            Scenario::load(&root.join(format!("harness/scenarios/{name}.toml"))).unwrap();
        let path = scenario.trace_path("tick.raw.jsonl");
        if !melee_test_support::require_files(
            scenario.required_files().into_iter().chain([path.clone()]),
        ) {
            return;
        }
        let mut simulation = TestSimulation::with_inputs(
            InitialState::from_savestate_traces(&scenario).unwrap(),
            crate::trace::pad_script(&scenario).unwrap(),
        );
        let mut identities = BTreeMap::new();
        let mut compared_histories = 0;
        for (tick, line) in melee_test_support::trace::read_to_string(&path)
            .unwrap()
            .lines()
            .enumerate()
        {
            let row: serde_json::Value = serde_json::from_str(line).unwrap();
            simulation.tick().unwrap();
            let state = &simulation.runtime.state;
            let items = row["items"].as_array().unwrap();
            assert_eq!(
                state.items.iter().count(),
                items.len(),
                "{name} tick {tick}"
            );
            for (item, expected) in state.items.iter().zip(items) {
                if item.kind != melee_types::ItemKind::FoxLaser {
                    continue;
                }
                let bytes: Vec<u8> = expected["bytes"]
                    .as_str()
                    .unwrap()
                    .as_bytes()
                    .chunks_exact(2)
                    .map(|s| u8::from_str_radix(std::str::from_utf8(s).unwrap(), 16).unwrap())
                    .collect();
                assert_eq!(
                    item.owner.map(u64::from),
                    expected["owner"].as_u64(),
                    "{name} tick {tick} owner"
                );
                let identity = (
                    item.stale_source,
                    word(&bytes, 0xD88),
                    u16::from_be_bytes(bytes[0xD8C..0xD8E].try_into().unwrap()),
                );
                assert_eq!(
                    *identities.entry(item.id).or_insert(identity),
                    identity,
                    "{name} tick {tick} original attack identity"
                );
                assert!(item.pending_reflection.is_none());
                assert_eq!(word(&bytes, 0xC64), 0, "reflection consumed at link14");
                assert_eq!(item.reflection_direction.to_bits(), word(&bytes, 0xC68));
                for (id, hit) in item.hitboxes.iter().enumerate() {
                    let Some(hit) = hit else { continue };
                    let offset = 0x5D4 + id * 0x13C;
                    assert_eq!(
                        hit.knockback_damage,
                        word(&bytes, offset + 8),
                        "{name} tick {tick} base damage"
                    );
                    assert_eq!(
                        hit.descriptor.damage.to_bits(),
                        word(&bytes, offset + 12),
                        "{name} tick {tick} working damage"
                    );
                    let mut history = Vec::new();
                    for slot in 0..12 {
                        let victim = word(&bytes, offset + 0x74 + slot * 8);
                        if victim == 0 {
                            continue;
                        }
                        let player = row["fighters"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .position(|f| {
                                u32::from_str_radix(
                                    f["base"].as_str().unwrap().trim_start_matches("0x"),
                                    16,
                                )
                                .unwrap()
                                    == victim
                            })
                            .expect("reflection victim is a fighter");
                        history.push((
                            state.fighters[player].spawn_number,
                            word(&bytes, offset + 0x78 + slot * 8),
                        ));
                    }
                    let actual: Vec<_> = item.reflection_history[id]
                        .iter()
                        .map(|h| (h.victim, u32::from(h.remaining)))
                        .collect();
                    assert_eq!(actual, history, "{name} tick {tick} rehit history");
                    compared_histories += history.len();
                }
            }
        }
        assert!(compared_histories > 0, "{name} must actually reflect");
    }
}
