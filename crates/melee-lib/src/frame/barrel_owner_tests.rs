//! it_8027236C's fighter-owner transfer, checked against raw retail Item bytes.
use super::*;
use crate::scenario::Scenario;
use std::path::PathBuf;

fn bytes(row: &serde_json::Value) -> Vec<u8> {
    let text = row["bytes"].as_str().unwrap();
    text.as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
fn word(bytes: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

#[test]
fn article_hit_transfers_fighter_owner_and_resets_barrel_attack() {
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    );
    for (name, ticks) in [
        ("corpus_v3_bf_items_sword_e44c79d87_p2", 252),
        ("corpus_v3_fd_items_sword_e3b85eba6_p1", 282),
    ] {
        let scenario =
            Scenario::load(&root.join(format!("harness/scenarios/{name}.toml"))).unwrap();
        let raw_path = scenario.trace_path("tick.raw.jsonl");
        if !melee_test_support::require_files(
            scenario
                .required_files()
                .into_iter()
                .chain([raw_path.clone()]),
        ) {
            continue;
        }
        let raw = melee_test_support::trace::read_to_string(&raw_path).unwrap();
        assert_eq!(raw.lines().count(), ticks);
        let rows: Vec<serde_json::Value> = raw
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        let before = &rows[ticks - 2];
        let after = &rows[ticks - 1];
        let barrel = |row: &serde_json::Value| {
            bytes(
                row["items"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|item| item["kind"] == 2)
                    .unwrap(),
            )
        };
        assert_eq!(word(&barrel(before), 0x518), 0);
        assert_eq!(word(&barrel(before), 0x24), 0);
        let retail = barrel(after);
        let article = after["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["kind"] == 56)
            .unwrap();
        let article_bytes = bytes(article);
        let fighter = bytes(&after["fighters"][0]);
        let owner = word(&fighter, 0);
        assert_eq!(word(&article_bytes, 0x518), owner);
        assert_ne!(
            owner,
            u32::from_str_radix(
                article["gobj"].as_str().unwrap().trim_start_matches("0x"),
                16
            )
            .unwrap()
        );
        assert_eq!(word(&retail, 0x518), owner);
        assert_eq!(retail[0x20], fighter[0x61b]);
        assert_eq!(word(&retail, 0x24), 5);
        assert_eq!(word(&retail, 0xd88), 1);
        assert_eq!(
            u16::from_be_bytes(retail[0xd8c..0xd8e].try_into().unwrap()),
            0
        );
        assert_eq!(word(&retail, 0x5d4), 2);
        assert_eq!(f32::from_bits(word(&retail, 0x5e0)), 15.0);

        let mut simulation = TestSimulation::with_inputs(
            InitialState::from_savestate_traces(&scenario).unwrap(),
            crate::trace::pad_script(&scenario).unwrap(),
        );
        for _ in 0..ticks {
            simulation.tick().unwrap();
        }
        let item = simulation
            .state()
            .items
            .iter()
            .find(|item| item.kind == melee_types::ItemKind::Taru)
            .unwrap();
        assert_eq!(item.owner, Some(0));
        assert!(!item.owner_secondary);
        assert_eq!(item.motion, 5);
        assert!(item.stale_source.is_none());
        assert_eq!(item.stale_multiplier, 1.0);
        let hit = item.hitboxes[0].as_ref().unwrap();
        assert_eq!(hit.descriptor.damage, f32::from_bits(word(&retail, 0x5e0)));
        assert_eq!(hit.phase, melee_coll::hitbox::CapsulePhase::FirstPosition);
    }
}
