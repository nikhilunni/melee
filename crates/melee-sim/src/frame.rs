//! Scene composition through HSD's real scheduler. Registrations, rather than
//! a sorted callback replay, preserve same-tick insertion/deferred destruction.
use crate::initial_state::scheduler_resume::{Continuation, ProcKey};
use crate::initial_state::stage;
use crate::scene_stage::SceneStage;
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
use std::collections::BTreeMap;
#[cfg(test)]
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Callback {
    Item { id: u32, phase: u8 },
    Countdown,
    Stage { map: Option<u8>, address: u32 },
    Fighter { player: usize, proc: FighterProc },
    Interface { player: usize },
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
impl Registration {
    fn key(self) -> ProcKey {
        let callback = match self.callback {
            Callback::Item { phase, .. } => match phase {
                0 => 0x802693E4,
                1 => 0x80269528,
                4 => 0x802697D4,
                5 => 0x80269978,
                9 => 0x80269A9C,
                11 => 0x80269B60,
                12 => 0x80269BE4,
                13 => 0x80269C5C,
                14 => 0x8026A294,
                16 => 0x8026A788,
                _ => unreachable!(),
            },
            Callback::Fighter { proc, .. } => match proc {
                FighterProc::Status => 0x8006_A1BC,
                FighterProc::Animation => 0x8006_A360,
                FighterProc::CpuGate => 0x8006_ABA0,
                FighterProc::Input => 0x8006_AD10,
                FighterProc::Update => 0x8006_B82C,
                FighterProc::Map => 0x8006_C27C,
                FighterProc::Pose => 0x8006_C5F4,
                FighterProc::Accessories => 0x8006_C624,
                FighterProc::HitboxPositions => 0x8006_C80C,
                FighterProc::Grab => 0x8006_CA5C,
                FighterProc::HitDetection => 0x8006_CB94,
                FighterProc::ProcessHit => 0x8006_D1EC,
                FighterProc::Dynamics => 0x8006_D9AC,
                FighterProc::Camera => 0x8006_D9EC,
                FighterProc::PlayerMirror => 0x8006_DA4C,
            },
            Callback::Stage { address, .. } => address,
            Callback::Interface { .. } => 0x802F_9410,
            Callback::ParticlesMain => 0x8005_C9A4,
            // These identities are only used when resuming their own s_link.
            Callback::ParticlesAux => 0x8005_C9D0,
            Callback::Effects => 0x8005_BC50,
            // Cold-only composite callback has no single retail proc identity.
            Callback::Countdown => 0,
        };
        ProcKey {
            p_link: self.p_link,
            object: if matches!(self.p_link, 5 | 8 | 15) {
                self.object
            } else {
                u8::MAX
            },
            callback,
        }
    }
}
fn registrations(stage: &SceneStage) -> Vec<Registration> {
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
        rows.push(Registration {
            s_link: 17,
            p_link: 15,
            priority: 0,
            object: player as u8,
            callback: Callback::Interface { player },
        });
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
/// Every imported cursor must identify exactly one modeled registration.
/// Otherwise an unported callback could be silently omitted on tick zero.
pub(crate) fn validate_saved_resume(
    resume: &crate::initial_state::scheduler_resume::SchedulerResume,
    stage: &SceneStage,
) -> Result<()> {
    if let Some((key, _)) = resume.current {
        ensure!(
            registrations(stage)
                .iter()
                .filter(|row| row.s_link == resume.s_link && row.key() == key)
                .count()
                == 1,
            "saved cursor does not identify one modeled callback: {key:?}"
        );
    }
    Ok(())
}
#[cfg(test)]
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
    item_objects: crate::scene_items::Objects,
    state: InitialState,
    /// The pad each port consumed per tick: scenario input, never state.
    pads: PadScript,
    frame: u64,
    error: Option<anyhow::Error>,
    /// Diagnostic only: values observed around procs, never gameplay inputs.
    rng_writers: Vec<(Option<Callback>, u32)>,
    particle_draws: DrawLog,
    interface: [melee_if::PercentDisplay; 2],
    match_finished: bool,
}
impl Runtime {
    fn dispatch_item(&mut self, id: u32, phase: u8) -> Result<()> {
        use crate::scene_items::SceneItems;
        let state = &mut self.state;
        let Some(item) = state.items.get_mut(id) else {
            return Ok(());
        };
        let kind = item.kind;
        let owner_slot = item.owner;
        let owner = owner_slot.and_then(|slot| {
            let index = state.fighters.iter().position(|fighter| {
                crate::scene_fighter::with_fighter!(fighter, |f| f.player.id == slot)
            })?;
            Some(crate::scene_fighter::with_fighter!(
                &mut state.fighters[index],
                |f| f.item_owner(&state.assets.fighters[index])
            ))
        });
        match phase {
            0 => {}
            1 => {
                state
                    .items
                    .animate::<SceneItems>(id, state.assets.items.get(kind), owner.as_ref())
            }
            4 => state.items.physics::<SceneItems>(id, owner.as_ref()),
            5 => {
                let contact = state.items.stage_contact(id, &mut state.map);
                state.items.collide::<SceneItems>(id, contact);
            }
            9 => {
                let item = state.items.get_mut(id).unwrap();
                if let melee_it::ItemScratch::Held(held) = &mut item.scratch {
                    if held.shot_pending {
                        held.shot_pending = false;
                        let slot = owner_slot.expect("blaster owner");
                        let index = state
                            .fighters
                            .iter()
                            .position(|fighter| {
                                crate::scene_fighter::with_fighter!(fighter, |f| f.player.id
                                    == slot)
                            })
                            .expect("live blaster owner");
                        let (position, angle) =
                            crate::scene_fighter::with_fighter!(&mut state.fighters[index], |f| f
                                .item_muzzle(&state.assets.fighters[index]))
                            .expect("blaster muzzle callback");
                        state.effects.spawn_blaster_muzzle::<RetailTrig>(
                            usize::from(slot),
                            position,
                            angle,
                            &state.assets.common_particle_bank,
                            &mut state.particles,
                            &mut state.rng,
                        )?;
                    }
                }
            }
            11 => state.items.get_mut(id).unwrap().update_hitboxes(),
            14 => state.items.process_events::<SceneItems>(id),
            12 | 13 | 16 => {}
            _ => unreachable!(),
        }
        Ok(())
    }
    fn dispatch(&mut self, row: Registration, world: &mut World) -> Result<()> {
        // fn_8016CFE0 -> gm_801A4634(4); gm_803DA888[4] freezes gameplay procs.
        const ELIMINATION_PAUSE_MASK: u64 = 0x800FFA;
        if self.match_finished
            && ELIMINATION_PAUSE_MASK & (1u64 << row.p_link) != 0
            && !matches!(row.callback, Callback::Effects)
        {
            return Ok(());
        }
        let continuation = if self.frame == 0 {
            self.state.resume.action(row.s_link, row.key())
        } else {
            Continuation::Invoke
        };
        if continuation == Continuation::Complete {
            return Ok(());
        }
        let state = &mut self.state;
        let state_pads = &self.pads;
        match row.callback {
            Callback::Item { id, phase } => {
                self.dispatch_item(id, phase)?;
            }
            Callback::Countdown => {
                if state
                    .countdown
                    .as_mut()
                    .is_some_and(|countdown| countdown.tick())
                {
                    for fighter in &mut state.fighters {
                        crate::scene_fighter::with_fighter!(fighter, |f| f.status.input_frozen =
                            false);
                    }
                    state.countdown = None;
                }
            }
            Callback::Fighter { player, proc } => {
                if proc == FighterProc::Animation {
                    use crate::scene_fighter::with_fighter;
                    let victim = with_fighter!(&state.fighters[player], |f| f.combat.combo.victim);
                    let expired = victim.is_some_and(|id| state.fighters.iter().any(|other| {
                        with_fighter!(other, |f| f.spawn_number == id && f.combat.combo.grace == 0 && !matches!(&f.state_data, melee_ft::fighter::MotionData::Damage(d) if d.hitstun > 0.0))
                    }));
                    if expired {
                        with_fighter!(&mut state.fighters[player], |f| f.combat.combo.victim =
                            None);
                    }
                }
                grab_pairs::constrain(state, player);
                if proc == FighterProc::Grab {
                    grab_pairs::select(state, player)?;
                }
                if proc == FighterProc::Update {
                    grab_pairs::align(state, player);
                }
                let assets = &state.assets;
                if proc == FighterProc::HitDetection {
                    use crate::scene_fighter::with_fighter;
                    for other in 0..state.fighters.len() {
                        if player == other {
                            continue;
                        }
                        let (victim, attacker) = if player < other {
                            let (left, right) = state.fighters.split_at_mut(other);
                            (&mut left[player], &mut right[0])
                        } else {
                            let (left, right) = state.fighters.split_at_mut(player);
                            (&mut right[0], &mut left[other])
                        };
                        with_fighter!(victim, |v| with_fighter!(attacker, |a| {
                            melee_ft::fighter::damage::detect_hit(v, a, &assets.fighters[player])
                        }));
                    }
                    for item in state.items.iter_mut() {
                        let hit = with_fighter!(&mut state.fighters[player], |f| {
                            f.core.detect_item_hit(item, &assets.fighters[player])
                        });
                        if let Some(damage) = hit {
                            // ftColl_80077C60 records contact; Item_8026A294 runs the callback at link 14.
                            item.record_damage_dealt(damage);
                        }
                    }
                }
                crate::scene_fighter::with_fighter!(&mut state.fighters[player], |f| {
                    if let Continuation::Animation {
                        restart,
                        configuring,
                    } = continuation
                    {
                        f.resume_wait_animation(
                            &assets.fighters[player],
                            &mut state.rng,
                            restart,
                            configuring,
                        )
                        .map_err(|e| anyhow::anyhow!("{e}"))?;
                        Ok(())
                    } else if let Continuation::Collision { in_sweep } = continuation {
                        melee_ft::collision::ground::resume_wait(
                            &mut f.core.physics,
                            &mut f.core.collision,
                            &mut state.map,
                            &mut f.core.skeleton,
                            f.core.animation.root,
                            in_sweep,
                        );
                        Ok(())
                    } else {
                        dispatch_fighter(
                            f,
                            proc,
                            player,
                            self.frame,
                            state_pads,
                            assets,
                            &mut state.map,
                            &mut state.effects,
                            &mut state.particles,
                            &mut state.rng,
                            match &state.stage {
                                SceneStage::Pupupu(stage) => stage.wind_at(f.physics.position),
                                _ => Vec3::ZERO,
                            },
                        )
                    }
                })?;
                if proc == FighterProc::Animation {
                    crate::scene_fighter::with_fighter!(&mut state.fighters[player], |f| {
                        if matches!(
                            f.state_data,
                            melee_ft::fighter::MotionData::Life(
                                melee_ft::fighter::life::LifeState::AwaitingRespawn
                            )
                        ) {
                            f.reset_for_revival(
                                &state.assets.fighters[player],
                                &state.assets.arena,
                                melee_ft::fighter::SpawnContext {
                                    map: &mut state.map,
                                    rng: &mut state.rng,
                                    counter: &mut state.spawn_counter,
                                },
                            )
                            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
                        }
                        Ok::<(), anyhow::Error>(())
                    })?;
                }
                if proc == FighterProc::Input {
                    grab_pairs::throw_input(state, player)?;
                }
                if proc == FighterProc::Accessories {
                    crate::scene_fighter::with_fighter!(&mut state.fighters[player], |f| {
                        f.character_accessory(&state.assets.fighters[player]);
                        f.update_revival_platform();
                        if f.motion_state.id == melee_types::CommonMotionState::ThrownB {
                            f.thrown_accessory(&state.assets.fighters[player]);
                        }
                    });
                }
                if proc == FighterProc::Animation {
                    grab_pairs::sync_wait(state, player)?;
                    grab_pairs::release(state, player)?;
                    let assets = &state.assets;
                    use crate::scene_fighter::with_fighter;
                    let bodies = std::array::from_fn::<_, 2, _>(|slot| {
                        with_fighter!(&state.fighters[slot], |f| f
                            .overlap_body(&assets.fighters[slot]))
                    });
                    let nudge = melee_coll::overlap::nudge(
                        player,
                        &bodies,
                        &assets.fighters[player].overlap,
                        |floor| [state.map.line_next(floor), state.map.line_prev(floor)],
                    );
                    with_fighter!(&mut state.fighters[player], |f| f.physics.player_nudge =
                        nudge);
                }
            }
            Callback::Stage { map, address } => match address {
                // Registered Ground wrappers: lighting, disabled spawn manager,
                // fixed animation attachments, static collision, disabled rain.
                // M3.md documents the scope and the constructor's phase guard.
                0x801C1CD0 => {
                    let map = map.expect("animation map");
                    if let Some(animation) = state.stage_animations.get_mut(&map) {
                        for event in animation.tick::<RetailTrig>() {
                            let mut request =
                                hsd_particle::system::SpawnRequest::new(event.bank, event.kind, 0);
                            request.joint = Some((stage::joint_id(map, event.joint), event.matrix));
                            state.effects.events.spawn(&request, false, false);
                            state.particles.spawn::<RetailTrig>(
                                &state.assets.particle_bank,
                                request,
                                &mut state.rng,
                                &mut self.particle_draws,
                            )?;
                        }
                        animation.for_each_matrix(|joint, matrix| {
                            state
                                .effects
                                .events
                                .update_joint(stage::joint_id(map, joint), matrix);
                            state
                                .particles
                                .update_joint(stage::joint_id(map, joint), matrix);
                        });
                    }
                    state.map.finish_ground_animation();
                }
                0x801C461C | 0x801CADBC | 0x801C1D38 | 0x801C0C2C => {}
                _ => {
                    let map_id = map.expect("stage callback map");
                    // Ground_801C2FE0 also runs from FD's controller. Even static
                    // transforms advance the collision epoch and select remapped sweeps.
                    if matches!(state.stage, SceneStage::Story(_)) || address == 0x8021_AAB0 {
                        let animation = state.stage_animations.get_mut(&map_id).unwrap();
                        let bindings =
                            &state.assets.stage_desc.models[map_id as usize].joint_mappings;
                        if address == 0x8021_AAB0 {
                            // grLast_804D4968: collision joint 0 belongs to map 3, root bone.
                            const FD_COLLISION_BINDINGS: [melee_gr::desc::JointMapping; 1] =
                                [melee_gr::desc::JointMapping {
                                    joint_index: 0,
                                    target_index: 3,
                                    extra: 0,
                                }];
                            animation.update_collision(&mut state.map, &FD_COLLISION_BINDINGS);
                        }
                        animation.update_collision(&mut state.map, bindings);
                    }
                    if continuation == Continuation::GroundCollision {
                        // grLast's controller already advanced; finish only its collision tail.
                        return Ok(());
                    }
                    if matches!(state.stage, SceneStage::Pupupu(_)) {
                        crate::scene_stage::pupupu::run_proc(
                            state,
                            map_id,
                            &mut self.particle_draws,
                        )?;
                    } else if state.stage.run_stage_proc(map_id, &mut state.rng)? {
                        // grLib_801C97DC (0x801C97DC): detached puff at the
                        // current world position of archive descendant 1.
                        let matrix =
                            state.stage_animations.get_mut(&map_id).unwrap().matrices()[1].1;
                        let mut request = hsd_particle::system::SpawnRequest::new(
                            0,
                            melee_gr::story::PUFF_PARTICLE,
                            0,
                        );
                        request.joint = Some((stage::joint_id(map_id, 1), matrix));
                        state.effects.events.spawn(&request, true, false);
                        let id = state.particles.spawn::<RetailTrig>(
                            &state.assets.common_particle_bank,
                            request,
                            &mut state.rng,
                            &mut self.particle_draws,
                        )?;
                        state.particles.pending_generators.push(id);
                    }
                }
            },
            Callback::Interface { player } => {
                use crate::scene_fighter::with_fighter;
                let percent = with_fighter!(&state.fighters[player], |f| f.physics.percent);
                let dead =
                    crate::scene_fighter::with_fighter!(&state.fighters[player], |f| matches!(
                        f.state_data,
                        melee_ft::fighter::MotionData::Life(
                            melee_ft::fighter::life::LifeState::Dead { .. }
                        )
                    ));
                self.interface[player].set_dead(dead, &mut state.rng);
                self.interface[player].tick(percent, &mut state.rng);
                if let Some(display) = &mut state.stock_displays[player] {
                    let stocks = with_fighter!(&state.fighters[player], |f| f.player.stocks);
                    for position in display.tick(stocks) {
                        let mut request = hsd_particle::system::SpawnRequest::new(0, 0xF7, 1);
                        request.position = [position.x, position.y, position.z];
                        state.effects.events.spawn(&request, false, true);
                        state.particles.spawn::<RetailTrig>(
                            &state.assets.common_particle_bank,
                            request,
                            &mut state.rng,
                            &mut self.particle_draws,
                        )?;
                    }
                }
            }
            Callback::Effects => state
                .effects
                .tick_with_pause::<melee_ft::fighter::RetailTrig>(
                    self.match_finished,
                    |player, bone| state.fighters[player].bone_matrix(bone),
                    &state.assets.common_particle_bank,
                    &mut state.particles,
                    &mut state.rng,
                )?,
            Callback::ParticlesMain => {
                if let Some(pending) = state.pending_emission.take() {
                    pending.finish(
                        &mut state.particles,
                        &mut state.rng,
                        &mut self.particle_draws,
                    )?;
                } else {
                    state
                        .particles
                        .proc_main::<RetailTrig>(&mut state.rng, &mut self.particle_draws)?;
                }
            }
            Callback::ParticlesAux => state
                .particles
                .proc_aux::<RetailTrig>(&mut state.rng, &mut self.particle_draws)?,
        }
        let state = &mut self.state;
        for fighter in &mut state.fighters {
            crate::scene_fighter::with_fighter!(fighter, |f| {
                while !f.item_requests.is_empty() {
                    let request = f.item_requests.remove(0);
                    crate::scene_items::request(
                        &mut state.items,
                        &state.assets.items,
                        &mut state.map,
                        world,
                        &mut self.item_objects,
                        request,
                    );
                }
            });
        }
        // Item SFX use the same headless request sink as ft_PlaySFX.
        for item in state.items.iter_mut() {
            while !item.sound_requests.is_empty() {
                let id = item.sound_requests.remove(0);
                if let Some(fighter) = state.fighters.iter_mut().find(|fighter| {
                    crate::scene_fighter::with_fighter!(fighter, |f| Some(f.player.id)
                        == item.owner)
                }) {
                    crate::scene_fighter::with_fighter!(fighter, |f| {
                        f.commands.footstep_sounds.push(
                            melee_ft::fighter::commands::FootstepSound {
                                channel: melee_ft::fighter::commands::SoundChannel::Ordinary,
                                id,
                                volume: 127,
                                pan: 64,
                            },
                        );
                    });
                }
            }
        }
        for item in state
            .items
            .iter()
            .filter(|item| item.destroyed && matches!(item.scratch, melee_it::ItemScratch::Held(_)))
        {
            if let Some(owner) = item.owner {
                state
                    .effects
                    .destroy_blaster_muzzles(usize::from(owner), &mut state.particles);
            }
        }
        crate::scene_items::cleanup(&mut state.items, world, &mut self.item_objects);
        self.particle_draws.0.append(&mut state.effects.draws.0);
        Ok(())
    }
}
/// Owns all mutable simulation state. Once constructed, tick has no trace,
/// ledger, seed, or frame-specific argument: the input script is fixed up front.
pub struct Simulation {
    world: World,
    // Unique initialization-time storage keeps cold-start call stacks small.
    // The scheduler borrows this owner; no reference counts or runtime borrows.
    runtime: Box<Runtime>,
    registrations: Vec<Registration>,
}
impl Simulation {
    pub fn item_snapshot(&self, frame: u64) -> Record {
        crate::trace_items::actual(&self.runtime.state.items, frame)
    }
    /// A simulation with neutral pads on every port.
    pub fn new(state: InitialState) -> Self {
        Self::with_inputs(state, PadScript::default())
    }
    pub fn with_inputs(mut state: InitialState, pads: PadScript) -> Self {
        // Prepare ordinary combat storage before ticks. These reserve budgets
        // do not change retail allocation limits or particle failure behavior.
        const GENERATOR_RESERVE: usize = 128;
        const PARTICLES_PER_LINK_RESERVE: usize = 256;
        state
            .particles
            .generators
            .reserve(GENERATOR_RESERVE.saturating_sub(state.particles.generators.len()));
        state
            .particles
            .pending_generators
            .reserve(GENERATOR_RESERVE);
        for link in &mut state.particles.particles {
            link.reserve(PARTICLES_PER_LINK_RESERVE.saturating_sub(link.len()));
        }
        let mut rows = registrations(&state.stage);
        if state.countdown.is_some() {
            rows.push(Registration {
                s_link: 0,
                p_link: 14,
                priority: 0,
                object: 0,
                callback: Callback::Countdown,
            });
        }
        use crate::scene_fighter::with_fighter;
        let interface = std::array::from_fn(|player| {
            melee_if::PercentDisplay::new(with_fighter!(&state.fighters[player], |f| f
                .physics
                .percent))
        });
        let runtime = Box::new(Runtime {
            item_objects: Default::default(),
            state,
            interface,
            match_finished: false,
            pads,
            frame: 0,
            error: None,
            // At most one writer per registered proc, plus match-start music.
            rng_writers: Vec::with_capacity(rows.len() + 1 + melee_it::ITEM_CAPACITY * 10),
            particle_draws: DrawLog(Vec::with_capacity(4096)),
        });
        let mut world = World::new(WorldConfig::MELEE);
        let mut objects = BTreeMap::new();
        for (index, row) in rows.iter().enumerate() {
            let object = *objects
                .entry((row.p_link, row.object))
                .or_insert_with(|| world.create(0, row.p_link, row.priority));
            world.add_tagged_proc(object, row.s_link, index);
        }
        crate::scene_items::prepare_scheduler(&mut world);
        Self {
            world,
            runtime,
            registrations: rows,
        }
    }
    /// Complete the imported partial tick first, then one full scheduler pass
    /// per call. Errors poison the simulation: a partial tick cannot be retried.
    pub fn tick(&mut self) -> Result<Record> {
        self.tick_without_snapshot()?;
        let runtime = &self.runtime;
        Ok(crate::trace::snapshot(&runtime.state, runtime.frame - 1))
    }
    /// Advance the same scheduler without constructing a diagnostic trace record.
    /// Use this for headless throughput and allocation measurements.
    pub fn tick_without_snapshot(&mut self) -> Result<()> {
        ensure!(
            self.runtime.error.is_none(),
            "simulation is poisoned by a prior tick error"
        );
        {
            let runtime = self.runtime.as_mut();
            let frame = runtime.frame;
            runtime.state.effects.events.begin_tick(frame);
            runtime.rng_writers.clear();
            runtime.particle_draws.0.clear();
            runtime.state.effects.direct_draws.clear();
            if let Some((music, unlocked)) = runtime.state.pending_music.take() {
                runtime.state.selected_music = Some(music.select(unlocked, &mut runtime.state.rng));
                let seed = runtime.state.rng.seed;
                runtime.rng_writers.push((None, seed));
            }
            if runtime.frame != 0 {
                // particleSort (psdisp.c:0x8039FC70), between observations.
                runtime.state.particles.sort_for_display(7);
            }
        }
        let runtime = self.runtime.as_mut();
        // gm_GetFFAOutcome (8016BF74): the supported two-player stock match ends
        // when only one player retains stocks. The pause takes effect next tick.
        runtime.match_finished |=
            runtime.state.fighters.iter().any(|fighter| {
                crate::scene_fighter::with_fighter!(fighter, |f| f.player.stocks == 0)
            });
        let rows = &self.registrations;
        self.world.run_procs_with(|world, _, index| {
            let row = if index >= crate::scene_items::TAG_BASE {
                let (id, phase) = crate::scene_items::decode_tag(index);
                Registration {
                    s_link: phase,
                    p_link: 9,
                    priority: 0,
                    object: u8::MAX,
                    callback: Callback::Item { id, phase },
                }
            } else {
                rows[index]
            };
            if runtime.error.is_some() {
                return;
            }
            let seed = runtime.state.rng.seed;
            if let Err(error) = runtime.dispatch(row, world) {
                runtime.error =
                    Some(error.context(format!("frame {} proc {:?}", runtime.frame, row)));
            }
            if seed != runtime.state.rng.seed {
                runtime
                    .rng_writers
                    .push((Some(row.callback), runtime.state.rng.seed));
            }
        });
        let runtime = self.runtime.as_mut();
        if let Some(error) = &runtime.error {
            anyhow::bail!("{error:#}");
        }
        runtime.frame += 1;
        Ok(())
    }
    pub(crate) fn enable_spawn_recording(&mut self) {
        self.runtime.state.effects.events.enable();
    }
    pub(crate) fn finish_spawn_recording(&mut self) -> BTreeMap<u64, Vec<serde_json::Value>> {
        self.runtime.state.effects.events.finish()
    }
    /// Ordered particle branch sites observed in the last completed tick.
    pub fn particle_rng_sites(&self) -> Vec<u32> {
        self.runtime.particle_draws.0.clone()
    }
    /// Ordered direct hit-effect RNG sites, separate from particle interpreter sites.
    pub fn effect_rng_sites(&self) -> Vec<u32> {
        self.runtime
            .state
            .effects
            .direct_draws
            .iter()
            .copied()
            .collect()
    }
    pub fn rng_writers(&self) -> Vec<(String, u32)> {
        self.runtime
            .rng_writers
            .iter()
            .map(|&(callback, seed)| {
                let name = callback.map_or_else(
                    || "match-start music".into(),
                    |callback| format!("{callback:?}"),
                );
                (name, seed)
            })
            .collect()
    }
}

#[allow(clippy::too_many_arguments)] // Borrow each subsystem independently while dispatching a concrete fighter.
fn dispatch_fighter<C: melee_ft::fighter::CharacterCallbacks>(
    f: &mut melee_ft::fighter::Fighter<C>,
    proc: FighterProc,
    player: usize,
    frame: u64,
    state_pads: &PadScript,
    scene_assets: &crate::assets::Assets,
    map: &mut melee_mp::CollMap,
    effects: &mut melee_ef::Effects,
    particles: &mut hsd_particle::system::ParticleSystem,
    rng: &mut gekko_math::HsdRng,
    wind: Vec3,
) -> Result<()> {
    let assets = &scene_assets.fighters[player];
    match proc {
        FighterProc::Status => f.proc_status(),
        FighterProc::Animation => {
            f.proc_anim(assets, rng)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
        }
        FighterProc::CpuGate => f.proc_cpu_gate(),
        FighterProc::Input => {
            // HSD_PadGameStatus[fp->x618_player_id]: in a Vs match
            // the human slot's port is its player index.
            let pad: PadSample = state_pads.sample(frame, usize::from(f.player.id));
            f.proc_input(assets, &pad)
        }
        FighterProc::Update => {
            f.proc_update(assets, map, wind);
            f.check_blast_zone(assets, &scene_assets.arena)
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        }
        FighterProc::Map => {
            f.proc_map_with_assets(assets, map, rng)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
        }
        FighterProc::Pose => f.proc_pose(map),
        FighterProc::Accessories => f.proc_accessories(),
        FighterProc::HitboxPositions => {
            effects.flush::<melee_ft::fighter::RetailTrig>(
                melee_ef::EffectTiming::Deferred,
                player,
                &mut f.core,
                &scene_assets.common_particle_bank,
                particles,
                rng,
            )?;
            f.proc_hitbox_positions();
        }
        FighterProc::Grab => f.proc_grab(),
        FighterProc::HitDetection => f.proc_hit_detection(),
        FighterProc::ProcessHit => f.proc_process_hit(assets),
        FighterProc::Dynamics => f.proc_dynamics_with_map(map),
        FighterProc::Camera => f.proc_camera_with_map(assets, 1.0, map),
        FighterProc::PlayerMirror => f.proc_player_mirror(),
    }
    // ftAction_80071CCC -> ft_800889F4 (80088A18): one Randi per smash voice.
    for _ in 0..std::mem::take(&mut f.commands.smash_sound_requests) {
        if !assets.smash_sounds.is_empty() {
            let id = assets.smash_sounds[rng.randi(assets.smash_sounds.len() as i32) as usize];
            f.commands
                .footstep_sounds
                .push(melee_ft::fighter::commands::FootstepSound {
                    channel: melee_ft::fighter::commands::SoundChannel::Action,
                    id,
                    volume: 127,
                    pan: 64,
                });
        }
    }
    f.resolve_graphics_commands(assets, rng);
    effects.flush::<melee_ft::fighter::RetailTrig>(
        melee_ef::EffectTiming::Immediate,
        player,
        &mut f.core,
        &scene_assets.common_particle_bank,
        particles,
        rng,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use melee_gr::{
        ground::Ground,
        last::{FinalDestination, MatchMode},
    };
    fn stage() -> SceneStage {
        let mut ground = Ground::controller([0; 3]);
        ground.start();
        ground.live_maps[..9].fill(true);
        SceneStage::FinalDestination(Box::new(FinalDestination {
            ground,
            mode: MatchMode::Versus,
            actions: vec![],
        }))
    }
    #[test]
    fn saved_cursor_must_resolve_to_a_modeled_owner() {
        let mut resume =
            crate::initial_state::scheduler_resume::SchedulerResume::between_ticks(false);
        resume.s_link = 1;
        let key = ProcKey {
            p_link: 8,
            object: 0,
            callback: 0x8006_A360,
        };
        resume.current = Some((key, Continuation::Invoke));
        assert!(validate_saved_resume(&resume, &stage()).is_ok());
        resume.current = Some((ProcKey { object: 9, ..key }, Continuation::Invoke));
        assert!(validate_saved_resume(&resume, &stage()).is_err());
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
        for phase in [
            0, 1, 2, 3, 4, 6, 7, 8, 9, 10, 12, 13, 14, 15, 16, 17, 18, 22,
        ] {
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
            if phase == 17 {
                // ifStatus_802F5B48 -> ifStatus_802F4EDC, after particles.
                for player in 0..2 {
                    expected.push((17, 15, player as u8, Callback::Interface { player }));
                }
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
    use std::{
        fs::File,
        io::{BufRead, BufReader},
        path::Path,
    };
    #[test]
    fn start_effect_matrices_and_particle_state() {
        let scenario = Scenario::load(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../harness/scenarios/start_fd_fox.toml"),
        )
        .unwrap();
        let path = scenario.trace_path("particles.jsonl");
        if !melee_test_support::require_files(
            scenario.required_files().into_iter().chain([path.clone()]),
        ) {
            return;
        }
        let expected = melee_diff::read_trace(BufReader::new(File::open(path).unwrap())).unwrap();
        let metadata: Vec<serde_json::Value> =
            BufReader::new(File::open(scenario.trace_path("particles.jsonl.meta.jsonl")).unwrap())
                .lines()
                .map(|line| serde_json::from_str(&line.unwrap()).unwrap())
                .collect();
        assert_eq!(metadata.len(), expected.len());
        let initial = InitialState::from_savestate_traces(&scenario).unwrap();
        let sidecar: serde_json::Value = serde_json::from_reader(
            File::open(scenario.savestate_path().with_extension("sav.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            u64::from(initial.rng.seed),
            sidecar["seed"].as_u64().unwrap()
        );
        let mut simulation = Simulation::new(initial);
        assert_eq!(expected.len(), 600);
        let mut checked = 0;
        let mut fields = 0;
        for expected in expected {
            simulation.tick().unwrap();
            let runtime = &simulation.runtime;
            let state = &runtime.state;
            let mut matrices = state.effects.matrices();
            for generator in &state.particles.generators {
                if let Some(id) = generator
                    .attachment_id
                    .filter(|&id| id < melee_ef::FIRST_EFFECT_JOINT)
                {
                    matrices.insert(id, generator.joint_matrix.unwrap());
                }
            }
            let meta = &metadata[expected.frame as usize]["particles"];
            for (generator, captured) in state
                .particles
                .generators
                .iter()
                .zip(meta["generators"].as_array().unwrap())
            {
                let Some(matrix) = generator.attachment_id.and_then(|id| matrices.get(&id)) else {
                    continue;
                };
                let pointer = &captured["fields"]["jobj"];
                let joint = meta["joints"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|j| &j["pointer"] == pointer)
                    .unwrap();
                let words: Vec<_> = joint["fields"]["matrix"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|w| w.as_u64().unwrap() as u32)
                    .collect();
                let actual: Vec<_> = matrix.0.iter().flatten().map(|f| f.to_bits()).collect();
                assert_eq!(actual, words, "effect matrix tick {}", expected.frame);
                checked += words.len();
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
        assert!(checked > 0, "live attached matrix coverage");
        eprintln!("{checked} captured matrix words and {fields} particle fields matched");
    }
}

#[cfg(test)]
mod marth_bones;

#[cfg(test)]
mod fall_states;

#[cfg(test)]
mod combat;
#[cfg(test)]
mod falco_bones;

#[cfg(test)]
mod falcon_bones;

#[cfg(test)]
mod peach_bones;

#[cfg(test)]
mod puff_bones;
#[cfg(test)]
mod puff_state;
#[cfg(test)]
mod yoshi_bones;

mod grab_pairs;
#[cfg(test)]
mod yoshi_state;

pub mod rendered_pose;
