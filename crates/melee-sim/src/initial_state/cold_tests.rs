use super::*;
use crate::{scene_fighter::with_fighter, scene_stage::SceneStage};
use melee_types::snapshot::SnapshotSink;

fn setup_snapshot(state: &InitialState) -> Record {
    let mut output = RecordSink::new(0, "setup");
    let sink: &mut dyn SnapshotSink = &mut output;
    state.snapshot(sink);
    for (p, fighter) in state.fighters.iter().enumerate() {
        let name = format!("p{p}");
        let mut prefix = PrefixSink::new(sink, &name);
        let sink: &mut dyn SnapshotSink = &mut prefix;
        with_fighter!(fighter, |f| {
            sink.field("cpu.attack_delay", &f.cpu.attack_delay);
            let melee_ft::fighter::MotionData::Entry(entry) = &f.state_data else {
                panic!("match-start fighter must be in Entry");
            };
            sink.field("entry.timer", &entry.timer);
            sink.field("entry.origin_y", &entry.origin_y);
            sink.field("entry.original_scale", &entry.original_scale);
            sink.field("entry.current_scale", &entry.current_scale);
            sink.field("entry.ecb.top", &entry.collision_box.top);
            sink.field("entry.ecb.bottom", &entry.collision_box.bottom);
            sink.field("entry.ecb.left.x", &entry.collision_box.left.x);
            sink.field("entry.ecb.left.y", &entry.collision_box.left.y);
            sink.field("entry.ecb.right.x", &entry.collision_box.right.x);
            sink.field("entry.ecb.right.y", &entry.collision_box.right.y);
        });
    }
    match &state.stage {
        SceneStage::FinalDestination(stage) => stage.ground.snapshot(sink),
        SceneStage::Battlefield(stage) => {
            sink.field("stage.phase", &(stage.phase as u8));
            sink.field("stage.timer", &stage.timer);
            sink.field("stage.current", &stage.current.map(i32::from).unwrap_or(-1));
            sink.field(
                "stage.previous",
                &stage.previous.map(i32::from).unwrap_or(-1),
            );
            sink.field("stage.light_count", &(stage.lights.len() as u32));
            for (i, light) in stage.lights.iter().enumerate() {
                let name = format!("stage.light[{i}]");
                let mut prefix = PrefixSink::new(sink, &name);
                let sink: &mut dyn SnapshotSink = &mut prefix;
                use melee_gr::battle::lights::Light;
                let color = match light {
                    Light::Point { .. } => unreachable!("Battlefield uses directional light"),
                    Light::Ambient { color } => {
                        sink.field("kind", &0u8);
                        color
                    }
                    Light::Directional {
                        color,
                        position,
                        shininess,
                    } => {
                        sink.field("kind", &1u8);
                        sink.field("position", position);
                        sink.field("shininess", shininess);
                        color
                    }
                };
                for (component, value) in color.iter().enumerate() {
                    sink.field(&format!("color[{component}]"), value);
                }
            }
        }
        SceneStage::Pupupu(stage) => {
            sink.field("stage.phase", &(stage.phase as u8));
            sink.field("stage.cycle", &stage.cycle);
            sink.field("stage.timer", &stage.timer);
            sink.field("stage.blink_timer", &stage.blink_timer);
            sink.field("stage.entering", &stage.entering);
            sink.field("stage.facing_right", &stage.facing_right);
            sink.field("stage.elapsed", &stage.elapsed);
            sink.field("stage.background_timer", &stage.background_timer);
            // xDC (wind) is inactive heap storage until the first map-7 proc.
        }
        SceneStage::Story(stage) => {
            sink.field("stage.puff_timer", &stage.puff_timer);
            sink.field("stage.shy_timer", &stage.shy_timer);
            sink.field("stage.previous_pattern", &stage.previous_pattern);
            sink.field("stage.spawn_count", &stage.spawn_count);
            sink.field("stage.occupied_frames", &stage.occupied_frames);
            sink.field("stage.shy_guys", &(stage.shy_guys.len() as u32));
        }
    }
    sink.field("scheduler.resume_s_link", &state.resume.s_link);
    output.finish()
}

fn verify(name: &str) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let cold = Scenario::load(&root.join(format!("harness/scenarios/{name}_cold.toml"))).unwrap();
    let saved = Scenario::load(&root.join(format!("harness/scenarios/{name}.toml"))).unwrap();
    if cold
        .required_files()
        .iter()
        .chain(saved.required_files().iter())
        .any(|p| !p.exists())
    {
        eprintln!("skipping {name}: local assets/captures absent");
        return;
    }
    let constructed = InitialState::from_parameters(&cold).unwrap();
    let imported = InitialState::from_savestate_traces(&saved).unwrap();
    let actual = setup_snapshot(&constructed);
    let expected = setup_snapshot(&imported);
    if let Some(diff) = first_divergence([&expected], [&actual]) {
        panic!("{name} initial snapshot: {diff}");
    }
    assert_eq!(actual.state.len(), expected.state.len());
    eprintln!("{name}: {} initial scene fields match", actual.state.len());
    let banks = BTreeMap::from([
        (0, constructed.assets.common_particle_bank.clone()),
        (30, constructed.assets.particle_bank.clone()),
    ]);
    particles::assert_state(
        &particles::snapshot(&imported.particles, imported.rng.seed, 0, &banks),
        &particles::snapshot(&constructed.particles, constructed.rng.seed, 0, &banks),
    );
    // Attachment matrices live in capture metadata, outside the particle
    // Snapshot fields. Compare their bits too; never supply them to setup.
    for (actual, expected) in constructed
        .particles
        .generators
        .iter()
        .zip(&imported.particles.generators)
    {
        let words = |matrix: Option<Mtx>| matrix.map(|m| m.0.map(|row| row.map(f32::to_bits)));
        assert_eq!(
            words(actual.joint_matrix),
            words(expected.joint_matrix),
            "{name} generator attachment"
        );
        assert_eq!(actual.attachment_id, expected.attachment_id);
    }
    crate::trace::gate(&cold).unwrap();
}
use std::collections::BTreeMap;

#[test]
fn start_fd_fox_cold_600() {
    verify("start_fd_fox");
}
#[test]
fn start_fd_marth_cold_600() {
    verify("start_fd_marth");
}
#[test]
fn start_fd_falco_cold_600() {
    verify("start_fd_falco");
}
#[test]
fn start_bf_fox_cold_600() {
    verify("start_bf_fox");
}

/// Construction and `run` must work when the root has DAT assets only:
/// neither a comparison trace nor any saved-boundary capture can be read.
#[cfg(unix)]
#[test]
fn cold_run_reads_only_dat_assets() {
    struct Scratch(std::path::PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).expect("remove test scratch directory");
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let assets = root.join("harness/roms/files");
    if !assets.is_dir() {
        eprintln!("skipping: owned DAT assets absent");
        return;
    }
    let scratch = Scratch(std::env::temp_dir().join(format!(
        "melee-cold-assets-{}-{}",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(),
    )));
    std::fs::create_dir_all(scratch.0.join("harness/roms")).unwrap();
    std::os::unix::fs::symlink(
        assets.canonicalize().unwrap(),
        scratch.0.join("harness/roms/files"),
    )
    .unwrap();
    for name in [
        "start_fd_fox",
        "start_fd_marth",
        "start_fd_falco",
        "start_bf_fox",
        "start_ys_fox",
        "start_dl_fox",
    ] {
        let mut scenario =
            Scenario::load(&root.join(format!("harness/scenarios/{name}_cold.toml"))).unwrap();
        if scenario
            .required_files()
            .iter()
            .filter(|p| p.starts_with(scenario.assets_path()))
            .any(|p| !p.exists())
        {
            eprintln!("skipping {name}: required DAT assets absent");
            continue;
        }
        scenario.root = scratch.0.clone();
        assert!(!scenario.expected_path().exists());
        assert!(!scenario.root.join("harness/traces").exists());
        crate::trace::write_run(&scenario, std::io::sink()).unwrap();
    }
}

#[test]
fn start_ys_fox_cold_600() {
    verify("start_ys_fox");
}

#[test]
fn start_dl_fox_cold_600() {
    verify("start_dl_fox");
}
