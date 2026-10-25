//! A synchronous item's deferred effects retain the scheduler's issuing phase.
use super::*;
use crate::scenario::Scenario;
use melee_coll::hitbox::CapsulePhase;
use melee_types::ItemKind;
use std::path::{Path, PathBuf};

#[test]
fn deferred_blast_retains_early_or_late_hitbox_phase_and_restores_drain_context() {
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    );
    let scenario =
        Scenario::load(&root.join("harness/scenarios/start_fd_items_sword.toml")).unwrap();
    if !melee_test_support::require_files(scenario.required_files()) {
        return;
    }
    let mut simulation = Simulation::new(InitialState::from_savestate_traces(&scenario).unwrap());
    for _ in 0..600 {
        simulation.tick_without_snapshot().unwrap();
        if simulation
            .runtime
            .state
            .items
            .iter()
            .any(|item| item.kind == ItemKind::Box)
        {
            break;
        }
    }
    let runtime = &mut simulation.runtime;
    let fixture = runtime
        .state
        .items
        .iter()
        .find(|item| item.kind == ItemKind::Box)
        .expect("the recorded 600-tick boundary contains a Box")
        .clone();
    for (late, immediate) in [(false, false), (true, false), (false, true), (true, true)] {
        let item = runtime.state.items.get_mut(fixture.id).unwrap();
        *item = fixture.clone();
        item.events.clear();
        item.queued_events.clear();
        item.position = Vec3::new(12.0, 30.0, 0.0);
        item.past_hitbox_refresh = late;
        item.efasync_immediate = immediate;
        // itBox_TryOpen resumes at step 0 after the boards' synchronous draws.
        item.defer_callback(0);
        // The deferred drain must use the issuer, not the drain's context.
        item.past_hitbox_refresh = !late;
        item.efasync_immediate = !immediate;
        // Actual pre-roll seed from e7254fba4_p2 tick 577, ledger LR8028636C.
        runtime.state.rng = gekko_math::HsdRng::new(0x8920_A4CC);
        assert!(runtime
            .drain_item_events_to_release(false)
            .unwrap()
            .is_none());
        let assets = runtime.state.assets.items.get(ItemKind::Box);
        let item = runtime.state.items.get_mut(fixture.id).unwrap();
        assert_eq!(item.motion, 6, "the witnessed roll breaks the Box empty");
        assert_eq!(
            (item.past_hitbox_refresh, item.efasync_immediate),
            (!late, !immediate)
        );
        let hit = item.hitboxes[0].as_ref().expect("blast capsule");
        if late {
            // retail 8027951C/2C: current proc s_link > 11 calls it_8027129C.
            assert_eq!(hit.phase, CapsulePhase::FirstPosition);
            assert_eq!(hit.position, item.position);
            assert_eq!(hit.previous_position, hit.position);
        } else {
            assert_eq!(hit.phase, CapsulePhase::Enabled);
            // An early caller leaves the normal link-11 refresh responsible.
            item.update_hitboxes(assets);
            let hit = item.hitboxes[0].as_ref().unwrap();
            assert_eq!(hit.phase, CapsulePhase::FirstPosition);
            assert_eq!(hit.position, item.position);
            assert_eq!(hit.previous_position, hit.position);
        }
    }
}
