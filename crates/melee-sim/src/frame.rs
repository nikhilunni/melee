//! Scene composition through HSD's real scheduler. Registrations, rather than
//! a sorted callback replay, preserve same-tick insertion/deferred destruction.
use crate::initial_state::InitialState;
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
    frame: u64,
    error: Option<anyhow::Error>,
    /// Diagnostic only: values observed around procs, never gameplay inputs.
    rng_writers: Vec<(String, u32)>,
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
                    FighterProc::Input => f.proc_input(assets, &PadSample::default()),
                    FighterProc::Update => f.proc_update(assets, &state.map, Vec3::ZERO),
                    FighterProc::Map => f.proc_map(&mut state.map),
                    FighterProc::Pose => f.proc_pose(&state.map),
                    FighterProc::Accessories => f.proc_accessories(),
                    FighterProc::HitboxPositions => f.proc_hitbox_positions(),
                    FighterProc::Grab => f.proc_grab(),
                    FighterProc::HitDetection => f.proc_hit_detection(),
                    FighterProc::ProcessHit => f.proc_process_hit(assets),
                    FighterProc::Dynamics => f.proc_dynamics(),
                    FighterProc::Camera => f.proc_camera(assets, 1.0),
                    FighterProc::PlayerMirror => f.proc_player_mirror(),
                }
            }
            Callback::Stage { map, address } => match address {
                // Registered Ground wrappers: lighting, disabled spawn manager,
                // fixed animation attachments, static collision, disabled rain.
                // M3.md documents the scope and the constructor's phase guard.
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
            Callback::ParticlesMain => state
                .particles
                .proc_main::<RetailTrig>(&mut state.rng, &mut DrawLog::default())?,
            Callback::ParticlesAux => state
                .particles
                .proc_aux::<RetailTrig>(&mut state.rng, &mut DrawLog::default())?,
        }
        Ok(())
    }
}
/// Owns all mutable simulation state. Once constructed, tick has no trace,
/// ledger, seed, or frame-specific input argument.
pub struct Simulation {
    world: World,
    runtime: Rc<RefCell<Runtime>>,
}
impl Simulation {
    pub fn new(state: InitialState) -> Self {
        let rows = registrations(&state.stage);
        let runtime = Rc::new(RefCell::new(Runtime {
            state,
            frame: 0,
            error: None,
            rng_writers: Vec::new(),
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
        self.runtime.borrow_mut().rng_writers.clear();
        self.world.run_procs();
        let mut runtime = self.runtime.borrow_mut();
        if let Some(error) = &runtime.error {
            anyhow::bail!("{error:#}");
        }
        let record = crate::trace::snapshot(&runtime.state, runtime.frame);
        runtime.frame += 1;
        Ok(record)
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
