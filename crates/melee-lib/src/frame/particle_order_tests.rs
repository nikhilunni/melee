//! An explicit callback teardown precedes a motion change's pending-effect flush.
use super::*;
use crate::scenario::Scenario;
use melee_ef::{request::EffectRequest, EffectTiming};
use std::{io::BufRead, path::PathBuf};

const NAME: &str = "corpus_v3_ys_ganondorf_fox4_e281b1a7a_p1";
fn scenario() -> Scenario {
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    );
    Scenario::load(&root.join(format!("harness/scenarios/{NAME}.toml"))).unwrap()
}

fn sealed_after_teardown(sound_only: bool, later_teardown: bool) {
    let mut state = InitialState::from_savestate_traces(&scenario()).unwrap();
    let program = state
        .assets
        .common_particle_bank
        .descriptor(225)
        .unwrap()
        .program
        .clone();
    let f = &mut state.fighters[0].0;
    f.commands.graphics.clear();
    f.commands.landing_effects.clear();
    f.commands.sound_draws.clear();
    f.commands.sound_draws_before_seal = None;
    f.commands.graphics_before_seal = 0;
    f.effects = Default::default();
    // Actual598 shape: Animation already queued0x414, then Map destroys
    // existing effects before ChangeMotionState seals that pending request.
    f.effects.push(EffectRequest::DestroyOwned);
    f.effects.push(EffectRequest::Attached {
        id: 0x414,
        bone: 67,
    });
    f.effects.resolve_pending(|_| hsd_types::Mtx::IDENTITY);
    if later_teardown {
        f.effects.push(EffectRequest::DestroyOwned);
    }
    if sound_only {
        f.commands.sound_draws_before_seal = Some(0);
        f.commands
            .sound_draws
            .push((melee_ft::fighter::commands::SoundDraw::Smash, 0));
        resolve_issued_sounds(
            f,
            0,
            &state.assets,
            &mut state.effects,
            &mut state.particles,
            &mut state.rng,
            true,
        )
        .unwrap();
    } else {
        // No unresolved graphics exist; the new landing command alone
        // triggers the explicit sealed-motion drain.
        f.commands.landing_effects.push((0x407, 0));
        resolve_issued_graphics(
            f,
            0,
            &state.assets,
            &mut state.effects,
            &mut state.particles,
            &mut state.rng,
        )
        .unwrap();
    }
    assert!(f.commands.graphics.is_empty());
    assert_eq!(
        state
            .particles
            .generators
            .iter()
            .filter(|g| g.descriptor.program == program)
            .count(),
        1
    );
    assert_eq!(
        f.effects
            .iter()
            .filter(|r| matches!(r, EffectRequest::DestroyOwned))
            .count(),
        usize::from(later_teardown)
    );
    state
        .effects
        .flush::<melee_ft::fighter::RetailTrig>(
            EffectTiming::Immediate,
            0,
            &mut f.core,
            &state.assets.common_particle_bank,
            &state.assets.particle_bank,
            &mut state.particles,
            &mut state.rng,
        )
        .unwrap();
    assert_eq!(
        state
            .particles
            .generators
            .iter()
            .any(|g| g.descriptor.program == program),
        !later_teardown
    );
}

#[test]
fn already_resolved_graphics_survive_the_preceding_teardown_at_landing() {
    sealed_after_teardown(false, false);
}
#[test]
fn sound_only_motion_flush_keeps_the_preceding_teardown_order() {
    sealed_after_teardown(true, false);
}
#[test]
fn teardown_after_the_last_seal_is_not_run_early() {
    for sound_only in [false, true] {
        sealed_after_teardown(sound_only, true);
    }
}

#[test]
fn whole_700_raw_clock_and_landing_generator_lifetime() {
    let scenario = scenario();
    assert_eq!(scenario.frames, 700);
    let raw = scenario.trace_path("tick.raw.jsonl");
    let pads = crate::trace::pad_script(&scenario)
        .unwrap()
        .with_display_from(&raw)
        .unwrap();
    let mut simulation = TestSimulation::with_inputs(
        InitialState::from_savestate_traces(&scenario).unwrap(),
        pads,
    );
    let meta = scenario.trace_path("particles.jsonl.meta.jsonl");
    let mut meta_rows = melee_test_support::trace::open(&meta).unwrap().lines();
    let mut count = 0;
    for (frame, line) in melee_test_support::trace::open(&raw)
        .unwrap()
        .lines()
        .enumerate()
    {
        let row: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
        assert_eq!(row["frame"], frame);
        simulation
            .engine
            .set_display_pass(simulation.pads.display_pass(frame as u64));
        simulation.tick_without_snapshot().unwrap();
        assert_eq!(
            u64::from(simulation.state().rng.seed),
            row["seed"].as_u64().unwrap(),
            "tick{frame}"
        );
        for (port, raw) in row["fighters"].as_array().unwrap().iter().enumerate() {
            let bytes = raw["bytes"].as_str().unwrap();
            let word = |o: usize| u32::from_str_radix(&bytes[o * 2..o * 2 + 8], 16).unwrap();
            let f = &simulation.state().fighters[port].0;
            assert_eq!(
                u32::from(f.motion_state.action.0),
                word(0x10),
                "tick{frame} p{port}"
            );
            for (actual, offset) in [
                (f.physics.position.x, 0xb0),
                (f.physics.position.y, 0xb4),
                (f.physics.position.z, 0xb8),
            ] {
                assert_eq!(actual.to_bits(), word(offset), "tick{frame} p{port}");
            }
        }
        let metadata: serde_json::Value =
            serde_json::from_str(&meta_rows.next().unwrap().unwrap()).unwrap();
        assert_eq!(metadata["frame"], frame);
        for clock in ["tick", "ps_frame", "seed"] {
            assert_eq!(
                metadata[clock], row[clock],
                "same recorded display/seed owner tick{frame}"
            );
        }
        let raw_generators = metadata["particles"]["generators"].as_array().unwrap();
        let particles = &simulation.state().particles;
        assert_eq!(
            particles.generators.len(),
            raw_generators.len(),
            "tick{frame}"
        );
        assert_eq!(
            u64::from(particles.family_counter),
            metadata["particles"]["globals"]["lbl_804D6368"]
                .as_u64()
                .unwrap()
        );
        for (actual, raw) in particles.generators.iter().zip(raw_generators) {
            let fields = &raw["fields"];
            for (actual, field) in [
                (u64::from(actual.family_id), "id"),
                (u64::from(actual.bank), "bank"),
                (u64::from(actual.link), "link"),
                (u64::from(actual.flags), "type"),
                (u64::from(actual.children), "child_count"),
                (u64::from(actual.remaining_life), "generator_life"),
            ] {
                assert_eq!(
                    actual,
                    fields[field].as_u64().unwrap(),
                    "tick{frame} {field}"
                );
            }
            assert_eq!(
                actual.attachment_id.is_some(),
                fields["jobj"].as_u64().unwrap() != 0,
                "tick{frame} joint owner presence"
            );
        }
        if [598, 599].contains(&frame) {
            let g = simulation
                .state()
                .particles
                .generators
                .iter()
                .find(|g| g.family_id == 724)
                .expect("retail newborn survives explicit prior teardown");
            assert_eq!(
                g.descriptor.program,
                simulation
                    .state()
                    .assets
                    .common_particle_bank
                    .descriptor(225)
                    .unwrap()
                    .program
            );
            assert_eq!(g.remaining_life, (607 - frame) as u16);
            assert_eq!(g.children, 0);
            assert_eq!(
                simulation.state().particles.family_counter,
                if frame == 598 { 725 } else { 726 }
            );
        }
        count += 1;
    }
    assert_eq!(count, 700);
    assert!(meta_rows.next().is_none());
}
