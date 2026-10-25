//! Registered original scripts retain the pair; malformed state still fails.
use super::*;
use crate::scenario::Scenario;
use melee_ft::fighter::{state::AnimationPhase, ActionId, MotionEntryFlags};
use melee_types::CommonMotionState;
use std::path::PathBuf;

#[test]
fn giant_punch_live_pairs_survive_seek_clear_on_exit_and_reject_missing_source() {
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    );
    let mut controls = 0;
    for (name, partial) in [
        ("donkey_punch_partial_fd_fox4", [372, 377]),
        ("kirby_copy_giantpunch_fd_donkeykong4", [458, 463]),
    ] {
        let scenario =
            Scenario::load(&root.join(format!("harness/scenarios/{name}.toml"))).unwrap();
        assert!(melee_test_support::require_files(scenario.required_files()));
        let pads = crate::trace::pad_script(&scenario)
            .unwrap()
            .with_display_from(&scenario.trace_path("tick.raw.jsonl"))
            .unwrap();
        let mut sim = TestSimulation::with_inputs(
            InitialState::from_savestate_traces(&scenario).unwrap(),
            pads,
        );
        let mut paired_rows = 0;
        for frame in 0..scenario.frames {
            sim.engine.set_display_pass(sim.pads.display_pass(frame));
            sim.tick_without_snapshot().unwrap();
            let state = sim.state();
            let fighter = &state.fighters[0].0;
            if !partial.contains(&fighter.motion_state.action.0) {
                continue;
            }
            assert_eq!(
                fighter.commands.hitboxes[0].is_some(),
                fighter.commands.hitboxes[1].is_some()
            );
            assert!(!fighter.commands.revived_hitboxes.iter().any(|v| *v));
            if fighter.commands.hitboxes[0].is_none() {
                continue;
            }
            paired_rows += 1;
            if name != "donkey_punch_partial_fd_fox4" || controls != 0 {
                continue;
            }
            let assets = &state.assets.fighters[0];
            let mut converted = fighter.clone();
            let before = format!("{:?}", converted.commands.hitboxes);
            // The actual punch ground/air flags: SkipHit and UpdateCmd.
            converted
                .change_motion_state_with_flags(
                    ActionId(377),
                    assets,
                    MotionEntryFlags(0x0C4C_5080 | (1 << 1) | (1 << 2) | (1 << 3)),
                    fighter.animation.frame,
                    1.0,
                )
                .unwrap();
            assert_eq!(format!("{:?}", converted.commands.hitboxes), before);
            converted
                .change_motion_state(CommonMotionState::Wait.into(), assets)
                .unwrap();
            assert!(converted.commands.hitboxes.iter().all(Option::is_none));
            assert!(!converted.commands.revived_hitboxes.iter().any(|v| *v));

            for scaled in [false, true] {
                let mut malformed = fighter.clone();
                // Explicit caller-corrupted/custom state, not an asset path.
                malformed.commands.script.instruction = None;
                malformed
                    .character
                    .get_mut::<ft_donkey::init::DonkeyKong>()
                    .giant_punch
                    .base_damage = [-1; 2];
                if scaled {
                    malformed.player.scale = 2.0;
                } else {
                    malformed.commands.disable_hitbox(1);
                }
                let mut map = state.map.clone();
                let mut rng = state.rng;
                let callback = malformed.motion_row.anim;
                let error = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    callback(
                        &mut malformed,
                        AnimationPhase {
                            assets,
                            map: &mut map,
                            rng: &mut rng,
                        },
                    )
                    .unwrap();
                }))
                .unwrap_err();
                let message = error
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .or_else(|| error.downcast_ref::<&str>().copied())
                    .unwrap();
                assert!(
                    message.contains(if scaled {
                        "scaled fighter"
                    } else {
                        "paired-hitbox invariant"
                    }),
                    "{message}"
                );
            }
            controls += 1;
        }
        assert!(paired_rows > 0, "{name} must actually scale a live pair");
    }
    assert_eq!(controls, 1);
}
