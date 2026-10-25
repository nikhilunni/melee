//! A full retail capture and an actual cleanup probe before effect draining.
use super::*;
use crate::scenario::Scenario;
use std::{io::BufRead, path::PathBuf};

const CAPTURE: &str = "kirby_inhale_randall_pit_ys_fox4";

#[test]
fn pinned_death_pair_geometry_and_revivals_match_every_raw_row() {
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    );
    let scenario = Scenario::load(&root.join(format!("harness/scenarios/{CAPTURE}.toml"))).unwrap();
    let raw = scenario.trace_path("tick.raw.jsonl");
    if !melee_test_support::require_files(
        scenario.required_files().into_iter().chain([raw.clone()]),
    ) {
        return;
    }
    let pads = crate::trace::pad_script(&scenario)
        .unwrap()
        .with_display_from(&raw)
        .unwrap();
    let mut simulation = TestSimulation::with_inputs(
        InitialState::from_savestate_traces(&scenario).unwrap(),
        pads,
    );
    let mut rows = 0;
    for (frame, line) in melee_test_support::trace::open(&raw)
        .unwrap()
        .lines()
        .enumerate()
    {
        simulation
            .engine
            .set_display_pass(simulation.pads.display_pass(frame as u64));
        simulation.tick_without_snapshot().unwrap();
        let row: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
        assert_eq!(row["frame"], frame);
        for port in 0..2 {
            let hex = row["fighters"][port]["bytes"].as_str().unwrap();
            let word = |o: usize| u32::from_str_radix(&hex[o * 2..o * 2 + 8], 16).unwrap();
            let f = &simulation.state().fighters[port].0;
            assert!(
                !f.effect_state.dispatch_at_once,
                "tick{frame} p{port}: immediate effect context must not leak past its callback"
            );
            assert_eq!(
                u32::from(f.motion_state.action.0),
                word(0x10),
                "tick{frame} p{port} action"
            );
            let p = f.physics.position;
            for (value, offset) in [(p.x, 0xB0), (p.y, 0xB4), (p.z, 0xB8)] {
                assert_eq!(
                    value.to_bits(),
                    word(offset),
                    "tick{frame} p{port} position"
                );
            }
            assert_eq!(
                f.animation
                    .root_motion
                    .as_ref()
                    .is_some_and(|root| root.pinned),
                (word(0x2224) >> 8) & 0x20 != 0,
                "tick{frame} pinned"
            );
            assert_eq!(
                f.combat.grab.is_some(),
                word(0x1A58) != 0,
                "tick{frame} grab"
            );
            assert!(
                f.released_link.is_none(),
                "no late scene release tick{frame}"
            );
        }
        if frame == 1357 {
            cleanup_effect_probe(simulation.state().clone());
        }
        rows += 1;
    }
    assert_eq!(rows, 1500);
}

fn cleanup_effect_probe(mut state: InitialState) {
    use melee_ef::request::EffectRequest;
    let (captor, victim) = grab_pairs::pair(&mut state.fighters, 0, 1);
    let captor = &mut **captor.0;
    let victim = &mut **victim.0;
    assert!(victim.combat.thrown_pose.is_some());
    // Keep the actual captured pose/constraint but give the death selector a
    // different world position. Cleanup must use XRotN, then effects the newly
    // released position, rather than this pre-release selection position.
    let before = hsd_types::Vec3::new(0.0, state.assets.arena.bottom - 5.0, 0.0);
    victim.physics.position = before;
    victim
        .check_blast_zone_with_partner(
            &state.assets.fighters[1],
            &state.assets.arena,
            &mut state.rng,
            captor,
            &state.assets.fighters[0],
            &mut state.map,
        )
        .unwrap();
    let released = victim.physics.position;
    assert_ne!(released, before);
    assert_eq!(released.x.to_bits(), (-63.281_99_f32).to_bits());
    assert_eq!(released.y.to_bits(), (-92.980_32_f32).to_bits());
    assert_eq!(victim.motion_state.action.0, 0);
    assert_eq!(captor.motion_state.action.0, 29);
    assert_eq!(victim.fatal_action.0, 289);
    assert!(victim.combat.grab.is_none() && captor.combat.grab.is_none());
    assert!(victim.combat.thrown_pose.is_none());
    assert!(victim.released_link.is_none());
    let requests = victim.effects_after_items.iter().collect::<Vec<_>>();
    assert_eq!(requests.len(), 2);
    assert!(matches!(requests[0],EffectRequest::CameraQuake{position,..} if *position==released));
    assert!(matches!(requests[1],EffectRequest::Death{position,..} if *position==released));
    assert!(matches!(
        victim.effects.iter().next(),
        Some(EffectRequest::ArticlesFreed)
    ));
    assert!(matches!(
        victim.effects.iter().nth(1),
        Some(EffectRequest::DestroyOwned)
    ));
}

#[test]
fn pinned_victim_clean_xrot_cache_matches_the_partial_bone_capture() {
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    );
    let scenario = Scenario::load(&root.join(format!("harness/scenarios/{CAPTURE}.toml"))).unwrap();
    let bones = scenario.trace_path("bones.jsonl");
    let raw = scenario.trace_path("bones.raw.jsonl");
    if !melee_test_support::require_files(
        scenario
            .required_files()
            .into_iter()
            .chain([bones.clone(), raw.clone()]),
    ) {
        return;
    }
    let text = melee_test_support::trace::read_to_string(&bones).unwrap();
    let rows = text
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 15, "only the1345..1359 bone window");
    let pads = crate::trace::pad_script(&scenario)
        .unwrap()
        .with_display_from(&raw)
        .unwrap();
    let mut simulation = TestSimulation::with_inputs(
        InitialState::from_savestate_traces(&scenario).unwrap(),
        pads,
    );
    let index = usize::from(
        simulation.state().assets.fighters[1]
            .parts
            .joint(melee_types::FtPart::XRotN)
            .unwrap(),
    );
    for frame in 0..1360 {
        simulation
            .engine
            .set_display_pass(simulation.pads.display_pass(frame as u64));
        simulation.tick_without_snapshot().unwrap();
        if frame < 1345 {
            continue;
        }
        if frame == 1357 {
            external_snapshot_sampler_probe(simulation.state().clone());
        }
        let row = &rows[frame - 1345];
        assert_eq!(row["frame"], frame);
        assert!(!row["dirty_bones"]["p1"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v.as_u64() == Some(index as u64)));
        let poses = simulation.engine.joint_poses();
        // Raw JObj DFS skips Kirby's empty conditional-part slots; the
        // port's diagnostic vector retains them. ftParts_SetupParts (800743E0)
        // advances the part index without advancing the live JObj for each
        // NULL slot. PlCo's conditional table identifies empty parts7..10,
        // 15,16,28..34 here; removing those gives these independent mappings.
        // These nine post-death reports must compare semantic joints.
        for (retail, port) in [(10, 14), (14, 20), (27, 40)] {
            let key = format!("p0.bone[{retail}].rotate[2]");
            assert_eq!(
                u64::from(poses[0][port].rotate[2].to_bits()),
                row["state"][key]["v"]["bits"].as_u64().unwrap(),
                "tick{frame} mapped Kirby rotate{retail}"
            );
        }
        for (retail, port) in [(36, 49), (42, 55)] {
            for axis in 0..3 {
                let key = format!("p0.bone[{retail}].scale[{axis}]");
                assert_eq!(
                    u64::from(poses[0][port].scale[axis].to_bits()),
                    row["state"][key]["v"]["bits"].as_u64().unwrap(),
                    "tick{frame} mapped Kirby scale{retail}/{axis}"
                );
            }
        }
        let actual = &poses[1][index];
        assert!(!actual.world_dirty, "tick{frame} XRotN cache must be clean");
        for (word, value) in actual.world.iter().flatten().enumerate() {
            let key = format!("p1.bone[{index}].mtx[{word}]");
            assert_eq!(
                u64::from(value.to_bits()),
                row["state"][key]["v"]["bits"].as_u64().unwrap(),
                "tick{frame} XRotN word{word}"
            );
        }
    }
}

fn external_snapshot_sampler_probe(mut state: InitialState) {
    let (captor, victim) = grab_pairs::pair(&mut state.fighters, 0, 1);
    let captor = &mut **captor.0;
    let victim = &mut **victim.0;
    let owner_part = usize::from(
        state.assets.fighters[1]
            .parts
            .joint(melee_types::FtPart::XRotN)
            .unwrap(),
    );
    let anchor_part = usize::from(
        state.assets.fighters[0]
            .parts
            .joint(melee_types::FtPart::TransN2)
            .unwrap(),
    );
    let owner = victim.animation.parts[owner_part].joint;
    let anchor = captor.animation.parts[anchor_part].joint;
    assert!(!victim.skeleton.mtx_is_dirty(owner));
    let cached = victim.skeleton.get(owner).mtx;
    captor.skeleton.set_mtx_dirty(anchor);
    melee_ft::fighter::grab_throw::refresh_constraint(
        &mut victim.core,
        &mut captor.core,
        &state.assets.fighters[1],
        &state.assets.fighters[0],
    );
    assert!(
        captor.skeleton.mtx_is_dirty(anchor),
        "a clean owner must not demand its external anchor"
    );
    assert_eq!(victim.skeleton.get(owner).mtx, cached);
    victim.skeleton.check_depend(owner);
    melee_ft::fighter::grab_throw::refresh_constraint(
        &mut victim.core,
        &mut captor.core,
        &state.assets.fighters[1],
        &state.assets.fighters[0],
    );
    assert!(
        !captor.skeleton.mtx_is_dirty(anchor),
        "an owner rebuild must resolve its current anchor"
    );
}
