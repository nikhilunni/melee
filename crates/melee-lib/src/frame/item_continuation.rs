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

/// The recorded tick-495 bounce spark resolves its root before the REST pitch.
#[test]
fn party_ball_settle_flushes_joint_effect_before_rest_pitch() {
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    );
    let scenario =
        Scenario::load(&root.join("harness/scenarios/start_fd_items_kusudama.toml")).unwrap();
    if !melee_test_support::require_files(scenario.required_files()) {
        return;
    }
    let mut simulation = Simulation::new(InitialState::from_savestate_traces(&scenario).unwrap());
    for _ in 0..495 {
        simulation.tick_without_snapshot().unwrap();
    }
    let ball = simulation
        .runtime
        .state
        .items
        .iter()
        .find(|i| i.kind == ItemKind::Kusudama)
        .unwrap();
    assert_eq!(ball.motion, 2);
    assert_eq!(ball.rotation.x, -std::f32::consts::FRAC_PI_2);
    simulation.tick_without_snapshot().unwrap();
    let ball = simulation
        .runtime
        .state
        .items
        .iter()
        .find(|i| i.kind == ItemKind::Kusudama)
        .unwrap();
    assert_eq!(ball.motion, 0);
    assert_eq!(
        ball.rotation.x,
        -std::f32::consts::FRAC_PI_2,
        "REST tail ran after the flush"
    );
    // Retail capture, not simulator-derived: generator 1, type 0 at tick 495.
    // Applying the pitch before draining the JointEffect moves this point.
    assert!(
        simulation
            .runtime
            .state
            .particles
            .generators
            .iter()
            .any(|g| g.position.map(f32::to_bits) == [75.0f32.to_bits(), 0x3f2c_d360, 0]),
        "bounce spark retained the pre-pitch root matrix"
    );
    let life = ball.life_timer;
    for _ in 496..600 {
        simulation.tick_without_snapshot().unwrap();
    }
    let ball = simulation
        .runtime
        .state
        .items
        .iter()
        .find(|i| i.kind == ItemKind::Kusudama)
        .unwrap();
    assert_eq!(ball.motion, 0);
    assert_eq!(
        ball.life_timer,
        life - 104.0,
        "common lifetime keeps ticking at rest"
    );
    let id = ball.id;
    // A perturbed root still eases on the next REST animation callback.
    simulation
        .runtime
        .state
        .items
        .get_mut(id)
        .unwrap()
        .rotation
        .x = 0.0;
    simulation.tick_without_snapshot().unwrap();
    assert_eq!(
        simulation
            .runtime
            .state
            .items
            .iter()
            .find(|item| item.id == id)
            .unwrap()
            .rotation
            .x
            .to_bits(),
        (-0.08726646f32).to_bits()
    );
}

#[test]
fn party_ball_settle_draws_spread_before_flushing_the_pre_pitch_root() {
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    );
    let scenario =
        Scenario::load(&root.join("harness/scenarios/start_fd_items_kusudama.toml")).unwrap();
    let ledger = scenario.trace_path("ledger600.raw.jsonl");
    if !melee_test_support::require_files(
        scenario
            .required_files()
            .into_iter()
            .chain([ledger.clone()]),
    ) {
        return;
    }
    let mut simulation = super::TestSimulation::with_inputs(
        InitialState::from_savestate_traces(&scenario).unwrap(),
        crate::trace::pad_script(&scenario)
            .unwrap()
            .with_display_from(&ledger)
            .unwrap(),
    );
    for _ in 0..495 {
        simulation.tick_without_snapshot().unwrap();
    }
    let runtime = &mut simulation.runtime;
    let id = runtime
        .state
        .items
        .iter()
        .find(|i| i.kind == ItemKind::Kusudama)
        .unwrap()
        .id;
    // Retail ledger tick494 end, then tick495's three request-side draws at
    // 80278A30/54/78, before the first generator/interpreter draw.
    assert_eq!(runtime.state.rng.seed, 0x6b99_22fb);
    runtime.dispatch_item(id, 4).unwrap();
    runtime.dispatch_item(id, 5).unwrap();
    assert_eq!(runtime.state.rng.seed, 0xc6ea_cedc);
    let ball = runtime.state.items.get_mut(id).unwrap();
    assert_eq!(ball.motion, 0);
    assert_eq!(ball.rotation.x.to_bits(), 0);
    assert_eq!(ball.events.len(), 2);
    assert!(matches!(
        ball.events[0],
        melee_it::ItemEvent::JointEffect { id: 0x405, .. }
    ));
    assert!(matches!(
        ball.events[1],
        melee_it::ItemEvent::Resume {
            past_hitbox_refresh: false,
            efasync_immediate: false,
            ..
        }
    ));
    assert!(runtime
        .drain_item_events_to_release(false)
        .unwrap()
        .is_none());
    let ball = runtime.state.items.get_mut(id).unwrap();
    assert_eq!(ball.rotation.x, -std::f32::consts::FRAC_PI_2);
    assert_eq!(
        (ball.past_hitbox_refresh, ball.efasync_immediate),
        (false, false)
    );
}
