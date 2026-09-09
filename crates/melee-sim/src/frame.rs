//! Scene composition through HSD's real scheduler. Registrations, rather than
//! a sorted callback replay, preserve same-tick insertion/deferred destruction.
use crate::{initial_state::InitialState, inputs::PadScript};
use anyhow::{ensure, Result};
use hsd_gobj::{World, WorldConfig};
use hsd_particle::rng_sites::DrawLog;
use hsd_types::Vec3;
use melee_diff::Record;
use melee_ft::{
    fighter::{FighterProc, RetailTrig},
    input::PadSample,
};
use melee_gr::last::{AnimationStatus, FinalDestination};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Callback {
    Stage { map: Option<u8>, address: u32 },
    Fighter { player: usize, proc: FighterProc },
    Effects,
    ParticlesMain,
    ParticlesAux,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Registration {
    s_link: u8,
    p_link: u8,
    priority: u8,
    /// Local object identity within a p_link; maps and fighters use list order.
    object: u8,
    callback: Callback,
}
fn registrations(stage: &FinalDestination) -> Vec<Registration> {
    let mut rows: Vec<_> = stage
        .proc_table()
        .into_iter()
        .map(|r| Registration {
            s_link: r.s_link,
            p_link: r.p_link,
            priority: r.p_priority,
            object: r.map_id.unwrap_or(u8::MAX),
            callback: Callback::Stage {
                map: r.map_id,
                address: r.address,
            },
        })
        .collect();
    for player in 0..2 {
        rows.extend(FighterProc::ALL.map(|proc| Registration {
            s_link: proc.s_link(),
            p_link: 8,
            priority: 0,
            object: player as u8,
            callback: Callback::Fighter { player, proc },
        }));
    }
    rows.extend([
        Registration {
            s_link: 15,
            p_link: 11,
            priority: 0,
            object: 1,
            callback: Callback::Effects,
        },
        Registration {
            s_link: 15,
            p_link: 11,
            priority: 1,
            object: 0,
            callback: Callback::ParticlesMain,
        },
        Registration {
            s_link: 15,
            p_link: 12,
            priority: 1,
            object: 0,
            callback: Callback::ParticlesAux,
        },
    ]);
    rows
}
fn register(
    world: &mut World,
    rows: &[Registration],
    callback: impl FnMut(&mut World, Registration) + 'static,
) {
    let callback = Rc::new(RefCell::new(callback));
    let mut objects = BTreeMap::new();
    for &row in rows {
        let object = *objects
            .entry((row.p_link, row.object))
            .or_insert_with(|| world.create(0, row.p_link, row.priority));
        let callback = Rc::clone(&callback);
        world.add_proc(object, row.s_link, move |world, _| {
            (callback.borrow_mut())(world, row)
        });
    }
}
struct Runtime {
    state: InitialState,
    /// The pad each port consumed per tick: scenario input, never state.
    pads: PadScript,
    frame: u64,
    error: Option<anyhow::Error>,
    /// Diagnostic only: values observed around procs, never gameplay inputs.
    rng_writers: Vec<(String, u32)>,
    particle_draws: DrawLog,
}
impl Runtime {
    fn dispatch(&mut self, row: Registration) -> Result<()> {
        // A savestate can stop inside a tick. All earlier procs already ran.
        // ProcessHit is idempotent on the asserted idle path; completing it
        // rebuilds derived collision caches before the remaining phases.
        if self.frame == 0 && row.s_link < self.state.resume_s_link {
            return Ok(());
        }
        let state = &mut self.state;
        let state_pads = &self.pads;
        match row.callback {
            Callback::Fighter { player, proc } => {
                let f = &mut state.fighters[player];
                let assets = &state.assets.fighter;
                match proc {
                    FighterProc::Status => f.proc_status(),
                    FighterProc::Animation => {
                        f.proc_anim(assets, &mut state.rng)
                            .map_err(|e| anyhow::anyhow!("{e}"))?;
                    }
                    FighterProc::CpuGate => f.proc_cpu_gate(),
                    FighterProc::Input => {
                        // HSD_PadGameStatus[fp->x618_player_id]: in a Vs match
                        // the human slot's port is its player index.
                        let pad: PadSample = state_pads.sample(self.frame, player);
                        f.proc_input(assets, &pad)
                    }
                    FighterProc::Update => f.proc_update(assets, &state.map, Vec3::ZERO),
                    FighterProc::Map => {
                        f.proc_map_with_assets(assets, &mut state.map, &mut state.rng)
                            .map_err(|e| anyhow::anyhow!("{e}"))?;
                    }
                    FighterProc::Pose => f.proc_pose(&state.map),
                    FighterProc::Accessories => f.proc_accessories(),
                    FighterProc::HitboxPositions => {
                        state.effects.flush(
                            player,
                            f,
                            &state.assets.effects,
                            &state.assets.common_particle_bank,
                            &mut state.particles,
                            &mut state.rng,
                        )?;
                        f.proc_hitbox_positions();
                    }
                    FighterProc::Grab => f.proc_grab(),
                    FighterProc::HitDetection => f.proc_hit_detection(),
                    FighterProc::ProcessHit => f.proc_process_hit(assets),
                    FighterProc::Dynamics => f.proc_dynamics_with_map(&mut state.map),
                    FighterProc::Camera => f.proc_camera(assets, 1.0),
                    FighterProc::PlayerMirror => f.proc_player_mirror(),
                }
                f.resolve_graphics_commands(assets, &mut state.rng);
            }
            Callback::Stage { map, address } => match address {
                // Registered Ground wrappers: lighting, disabled spawn manager,
                // fixed animation attachments, static collision, disabled rain.
                // M3.md documents the scope and the constructor's phase guard.
                0x801C1CD0 if map == Some(4) => {
                    if let Some(animation) = &mut state.stage_animation {
                        for event in animation.tick::<RetailTrig>() {
                            let mut request =
                                hsd_particle::system::SpawnRequest::new(event.bank, event.kind, 0);
                            request.joint = Some((event.joint, event.matrix));
                            state.particles.spawn::<RetailTrig>(
                                &state.assets.particle_bank,
                                request,
                                &mut state.rng,
                                &mut self.particle_draws,
                            )?;
                        }
                    }
                }
                0x801C461C | 0x801CADBC | 0x801C1CD0 | 0x801C1D38 | 0x801C0C2C => {}
                _ => {
                    state.stage.run_stage_proc(
                        map.expect("stage callback map"),
                        &AnimationStatus::default(),
                        &mut state.rng,
                    );
                    ensure!(
                        state.stage.actions.is_empty(),
                        "FD animation/creation transition outside imported M3 interval: {:?}",
                        state.stage.actions
                    );
                }
            },
            Callback::Effects => state.effects.tick(
                &mut state.fighters,
                &state.assets.common_particle_bank,
                &mut state.particles,
                &mut state.rng,
            )?,
            Callback::ParticlesMain => state
                .particles
                .proc_main::<RetailTrig>(&mut state.rng, &mut self.particle_draws)?,
            Callback::ParticlesAux => state
                .particles
                .proc_aux::<RetailTrig>(&mut state.rng, &mut self.particle_draws)?,
        }
        self.particle_draws.0.append(&mut state.effects.draws.0);
        Ok(())
    }
}
/// Owns all mutable simulation state. Once constructed, tick has no trace,
/// ledger, seed, or frame-specific argument: the input script is fixed up front.
pub struct Simulation {
    world: World,
    runtime: Rc<RefCell<Runtime>>,
}
impl Simulation {
    /// A simulation with neutral pads on every port.
    pub fn new(state: InitialState) -> Self {
        Self::with_inputs(state, PadScript::default())
    }
    pub fn with_inputs(state: InitialState, pads: PadScript) -> Self {
        let rows = registrations(&state.stage);
        let runtime = Rc::new(RefCell::new(Runtime {
            state,
            pads,
            frame: 0,
            error: None,
            rng_writers: Vec::new(),
            particle_draws: DrawLog::default(),
        }));
        let shared = Rc::clone(&runtime);
        let mut world = World::new(WorldConfig::MELEE);
        register(&mut world, &rows, move |_, row| {
            let mut runtime = shared.borrow_mut();
            if runtime.error.is_some() {
                return;
            }
            let seed = runtime.state.rng.seed;
            if let Err(error) = runtime.dispatch(row) {
                runtime.error =
                    Some(error.context(format!("frame {} proc {:?}", runtime.frame, row)));
            }
            if seed != runtime.state.rng.seed {
                let seed = runtime.state.rng.seed;
                runtime
                    .rng_writers
                    .push((format!("{:?}", row.callback), seed));
            }
        });
        Self { world, runtime }
    }
    /// Complete the imported partial tick first, then one full scheduler pass
    /// per call. Errors poison the simulation: a partial tick cannot be retried.
    pub fn tick(&mut self) -> Result<Record> {
        ensure!(
            self.runtime.borrow().error.is_none(),
            "simulation is poisoned by a prior tick error"
        );
        {
            let mut runtime = self.runtime.borrow_mut();
            runtime.rng_writers.clear();
            runtime.particle_draws.0.clear();
            if runtime.frame != 0 {
                // particleSort (psdisp.c:0x8039FC70), between observations.
                runtime.state.particles.sort_for_display(7);
            }
        }
        self.world.run_procs();
        let mut runtime = self.runtime.borrow_mut();
        if let Some(error) = &runtime.error {
            anyhow::bail!("{error:#}");
        }
        let record = crate::trace::snapshot(&runtime.state, runtime.frame);
        runtime.frame += 1;
        Ok(record)
    }
    /// Ordered particle branch sites observed in the last completed tick.
    pub fn particle_rng_sites(&self) -> Vec<u32> {
        self.runtime.borrow().particle_draws.0.clone()
    }
    pub fn rng_writers(&self) -> Vec<(String, u32)> {
        self.runtime.borrow().rng_writers.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use melee_gr::{ground::Ground, last::MatchMode};
    fn stage() -> FinalDestination {
        let mut ground = Ground::controller([0; 3]);
        ground.start();
        ground.live_maps[..9].fill(true);
        FinalDestination {
            ground,
            mode: MatchMode::Versus,
            actions: vec![],
        }
    }
    #[test]
    fn m3_proc_order() {
        let rows = registrations(&stage());
        let calls = Rc::new(RefCell::new(Vec::new()));
        let output = Rc::clone(&calls);
        let mut world = World::new(WorldConfig::MELEE);
        register(&mut world, &rows, move |_, row| {
            output
                .borrow_mut()
                .push((row.s_link, row.p_link, row.object, row.callback))
        });
        world.run_procs();
        // Independent transcription of M3_PLAN §2, substituting the FD map
        // creation/callback table for the old Story map IDs. No item instances.
        let mut expected = Vec::new();
        let mut stage_row = |s, p, map: Option<u8>, address| {
            expected.push((s, p, map.unwrap_or(255), Callback::Stage { map, address }))
        };
        stage_row(0, 3, None, 0x801C461C);
        stage_row(0, 4, None, 0x801CADBC);
        for phase in [0, 1, 2, 3, 4, 6, 7, 8, 9, 10, 12, 13, 14, 15, 16, 18, 22] {
            if phase == 1 {
                for map in 0..9 {
                    expected.push((
                        1,
                        5,
                        map,
                        Callback::Stage {
                            map: Some(map),
                            address: 0x801C1CD0,
                        },
                    ));
                }
            }
            if phase == 4 {
                for (map, address) in [
                    0x8021A914, 0x8021A968, 0x8021A9A4, 0x8021AAB0, 0x8021AB80, 0x8021ABD4,
                    0x8021AC28, 0x8021ADD0, 0x8021B28C,
                ]
                .into_iter()
                .enumerate()
                {
                    for address in [0x801C1D38, address] {
                        expected.push((
                            4,
                            5,
                            map as u8,
                            Callback::Stage {
                                map: Some(map as u8),
                                address,
                            },
                        ));
                    }
                }
            }
            if phase == 10 {
                expected.push((
                    10,
                    5,
                    255,
                    Callback::Stage {
                        map: None,
                        address: 0x801C0C2C,
                    },
                ));
                continue;
            }
            if phase == 15 {
                expected.push((15, 11, 1, Callback::Effects));
                expected.push((15, 11, 0, Callback::ParticlesMain));
                expected.push((15, 12, 0, Callback::ParticlesAux));
                continue;
            }
            let proc = match phase {
                0 => FighterProc::Status,
                1 => FighterProc::Animation,
                2 => FighterProc::CpuGate,
                3 => FighterProc::Input,
                4 => FighterProc::Update,
                6 => FighterProc::Map,
                7 => FighterProc::Pose,
                8 => FighterProc::Accessories,
                9 => FighterProc::HitboxPositions,
                12 => FighterProc::Grab,
                13 => FighterProc::HitDetection,
                14 => FighterProc::ProcessHit,
                16 => FighterProc::Dynamics,
                18 => FighterProc::Camera,
                22 => FighterProc::PlayerMirror,
                _ => unreachable!(),
            };
            for player in 0..2 {
                expected.push((phase, 8, player as u8, Callback::Fighter { player, proc }));
            }
        }
        assert_eq!(*calls.borrow(), expected);
    }
    #[test]
    fn m3_same_tick_item_insertion() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let output = Rc::clone(&calls);
        let mut world = World::new(WorldConfig::MELEE);
        let row = Registration {
            s_link: 4,
            p_link: 8,
            priority: 0,
            object: 0,
            callback: Callback::Fighter {
                player: 0,
                proc: FighterProc::Update,
            },
        };
        let mut spawned = false;
        register(&mut world, &[row], move |world, _| {
            output.borrow_mut().push("fighter");
            if spawned {
                return;
            }
            spawned = true;
            let item = world.create(0, 9, 0);
            for (phase, name) in [(0, "item status"), (4, "item update"), (5, "item map")] {
                let output = Rc::clone(&output);
                world.add_proc(item, phase, move |_, _| output.borrow_mut().push(name));
            }
        });
        world.run_procs();
        assert_eq!(*calls.borrow(), ["fighter", "item update", "item map"]);
        calls.borrow_mut().clear();
        world.run_procs();
        assert_eq!(
            *calls.borrow(),
            ["item status", "fighter", "item update", "item map"]
        );
    }
}

#[cfg(test)]
mod start_tests {
    use super::*;
    use crate::{initial_state::particles, scenario::Scenario};
    use std::{collections::BTreeSet, fs::File, io::BufReader, path::Path};
    #[test]
    fn start_effect_matrices_and_particle_state() {
        let scenario = Scenario::load(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../harness/scenarios/start_fd_fox.toml"),
        )
        .unwrap();
        let path = scenario.trace_path("particles.jsonl");
        if let Some(missing) = scenario
            .required_files()
            .into_iter()
            .chain([path.clone()])
            .find(|p| !p.is_file())
        {
            eprintln!(
                "skipping start matrices/particles: {} absent",
                missing.display()
            );
            return;
        }
        let expected = melee_diff::read_trace(BufReader::new(File::open(path).unwrap())).unwrap();
        let changes: Vec<(u64, usize, [u32; 12])> = serde_json::from_str(include_str!(
            "../../hsd-particle/tests/support/start_fd_joints.json"
        ))
        .unwrap();
        let initial = InitialState::from_savestate_traces(&scenario).unwrap();
        // The ledger is verification only. Tick zero has no draws; its seed
        // therefore is also the seed before tick zero's draws.
        use std::io::BufRead;
        let line = BufReader::new(File::open(scenario.trace_path("ledger600.raw.jsonl")).unwrap())
            .lines()
            .next()
            .unwrap()
            .unwrap();
        let ledger: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert!(ledger["rng_draws"].as_array().unwrap().is_empty());
        assert_eq!(ledger["seed"].as_u64(), Some(u64::from(initial.rng.seed)));
        assert_eq!(initial.rng.seed, 0xCC51_A0A5);
        let mut simulation = Simulation::new(initial);
        assert_eq!(expected.len(), 600);
        let mut attachments = BTreeSet::new();
        let mut checked = 0;
        let mut fields = 0;
        for expected in expected {
            simulation.tick().unwrap();
            let runtime = simulation.runtime.borrow();
            let state = &runtime.state;
            attachments.extend(
                state
                    .particles
                    .generators
                    .iter()
                    .filter_map(|g| g.attachment_id),
            );
            let mut matrices = state.effects.matrices();
            for generator in &state.particles.generators {
                if let Some(id) = generator
                    .attachment_id
                    .filter(|&id| id < crate::effects::FIRST_EFFECT_JOINT)
                {
                    matrices.insert(id, generator.joint_matrix.unwrap());
                }
            }
            let ordered: Vec<_> = attachments.iter().copied().collect();
            for &(_, normalized, words) in changes.iter().filter(|row| row.0 == expected.frame) {
                let id = ordered[normalized];
                let matrix = matrices.get(&id).unwrap();
                let actual: Vec<_> = matrix.0.iter().flatten().map(|f| f.to_bits()).collect();
                assert_eq!(
                    actual, words,
                    "effect matrix tick {} fixture joint {normalized} owned {id}",
                    expected.frame
                );
                checked += 12;
            }
            struct Banks<'a>(&'a crate::assets::Assets);
            impl particles::Banks for Banks<'_> {
                fn bank(&self, id: u8) -> &hsd_particle::bank::ParticleBank {
                    match id {
                        0 => &self.0.common_particle_bank,
                        30 => &self.0.particle_bank,
                        _ => panic!("bank {id}"),
                    }
                }
            }
            let actual = particles::snapshot(
                &state.particles,
                state.rng.seed,
                expected.frame,
                &Banks(&state.assets),
            );
            if let Some(diff) = melee_diff::first_divergence([&expected], [&actual]) {
                panic!("{diff}");
            }
            fields += expected.state.len();
        }
        assert_eq!(checked, changes.len() * 12);
        eprintln!("{checked} captured matrix words and {fields} particle fields matched");
    }
}
