//! Scene composition through HSD's real scheduler. Registrations, rather than
//! a sorted callback replay, preserve same-tick insertion/deferred destruction.
use crate::banner::BannerKind;
use crate::initial_state::scheduler_resume::{Continuation, ProcKey};
use crate::initial_state::stage;
use crate::initial_state::InitialState;
use crate::scene_stage::SceneStage;
use anyhow::{ensure, Result};
use hsd_gobj::{TaggedWorld as World, WorldConfig};
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
    Item {
        id: u32,
        phase: u8,
    },
    Banner,
    Stage {
        map: Option<u8>,
        address: u32,
    },
    Fighter {
        player: usize,
        proc: FighterProc,
    },
    Interface {
        player: usize,
    },
    Effects,
    ParticlesMain,
    ParticlesAux,
    /// fn_8002F360: the camera gobj's mode proc (Camera_8002B3D4).
    Camera,
    /// grLib_801C9C40 for every playing quake model.
    Quakes,
    /// fn_8026C88C: the random item spawner (it_8026D018's GObj).
    ItemSpawner,
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
            Callback::Camera => 0x8002_F360,
            Callback::Quakes => 0x801C_9C40,
            Callback::ItemSpawner => 0x8026_C88C,
            // Cold-only composite callback has no single retail proc identity.
            Callback::Banner => 0,
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
/// Scheduled procs for `stage`, `players` players' HUDs and `fighters`
/// fighter GObjs (a player's partner, such as Nana, is a fighter of its own).
fn registrations(stage: &SceneStage, players: usize, fighters: usize) -> Vec<Registration> {
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
    for player in 0..players {
        rows.push(Registration {
            s_link: 17,
            p_link: 15,
            priority: 0,
            object: player as u8,
            callback: Callback::Interface { player },
        });
    }
    for player in 0..fighters {
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
        // Camera_Create: GObj_Create(0x10, 0x12, 0), proc at s_link 0x12.
        Registration {
            s_link: 18,
            p_link: 18,
            priority: 0,
            object: 0,
            callback: Callback::Camera,
        },
        // grLib_801C9CEC's quake gobjs (p_link 18, priority = kind), s_link 1.
        Registration {
            s_link: 1,
            p_link: 18,
            priority: 2,
            object: 1,
            callback: Callback::Quakes,
        },
    ]);
    rows
}
/// Every imported cursor must identify exactly one modeled registration.
/// Otherwise an unported callback could be silently omitted on tick zero.
pub(crate) fn validate_saved_resume(
    resume: &crate::initial_state::scheduler_resume::SchedulerResume,
    stage: &SceneStage,
    players: usize,
    fighters: usize,
) -> Result<()> {
    if let Some((key, _)) = resume.current {
        ensure!(
            registrations(stage, players, fighters)
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
    world: &mut hsd_gobj::World,
    rows: &[Registration],
    callback: impl FnMut(&mut hsd_gobj::World, Registration) + 'static,
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
    radial_forces: melee_lb::radial_force::RadialForces,
    item_objects: crate::scene_items::Objects,
    /// Registration rows (index, p_link, s_link) whose GObjs
    /// Ground_801C0FB8 creates at stage start.
    stage_start_rows: Vec<(usize, u8, u8)>,
    stage_objects: crate::scene_stage::StageObjects,
    state: InitialState,
    /// The pad each port consumed per tick: scenario input, never state.
    pads: [PadSample; 4],
    /// This tick's raw `PADStatus` sticks per port, when the caller knows
    /// them (a tick-clock schedule, a Slippi replay, a recording); otherwise
    /// the bytes behind each pad's normalized sticks.
    raw_sticks: Option<[melee_ft::input::RawSticks; 4]>,
    /// This tick's queue bytes exactly as a retail recording read them;
    /// overrides the modelled 0.74/0.8 view.
    recorded_pad_queue_x: Option<[melee_ft::input::PadQueueX; 4]>,
    /// HSD_PadLibData.queue as controller-fix codes read it: each port's
    /// raw stick X one and two ticks back (queue entries qread-2, qread-3),
    /// one poll consumed per tick.
    raw_stick_x_history: [[i8; 2]; 4],
    /// The pad-queue bytes this tick's 0.74/0.8 codes read.
    pad_queue_x: [melee_ft::input::PadQueueX; 4],
    /// The sticks of the queue entry this tick consumed (0.84 reads them).
    consumed_sticks: [melee_ft::input::RawSticks; 4],
    /// Diagnostic only: each fighter's snapshot at the end of its map proc
    /// (Fighter_8006C27C) this tick, when requested.
    after_map: Option<Vec<(usize, Record)>>,
    /// Diagnostic only: the same sample as plain values, each fighter's own
    /// (`Simulation::capture_after_map_values`).
    after_map_values: Option<Vec<(usize, crate::diagnostics::FighterValues)>>,
    /// This tick's external events: inputs like the pads, never state.
    external: crate::ExternalEvents,
    /// What the last tick consumed from `external`.
    consumed: crate::ConsumedEvents,
    frame: u64,
    error: Option<std::sync::Arc<str>>,
    /// Diagnostic only: values observed around procs, never gameplay inputs.
    rng_writers: Vec<(Option<Callback>, u32)>,
    particle_draws: DrawLog,
    /// Each player's damage display (ifStatus), in player order.
    interface: Vec<melee_if::PercentDisplay>,
    match_finished: bool,
    /// Each port's Start button on the previous tick, for its press.
    start_held: [bool; 4],
    /// Scenario input: retail rendered (and so ran particleSort) between the
    /// previous tick and this one. Live play renders every tick; a recording
    /// that ran behind executes several ticks per VI frame and renders once.
    display_pass: DisplayPass,
}
/// What ran between the previous tick and the next one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayPass {
    /// The display pass (HSD_GObj_80390FC0).
    Rendered,
    /// Nothing: the two ticks ran in one pass of gm_801A4D34's loop.
    Skipped,
    /// A Slippi Online rollback ran both ticks in its loop
    /// (Online/Core/LoopEngineForRollback.asm, 0x801A5014): no display
    /// pass, but the code calls Camera_8002A4AC and then ftLib_80086A8C for
    /// every fighter before it branches back to the tick body (0x801A4DE4).
    RollbackLoop,
}
impl Clone for Runtime {
    fn clone(&self) -> Self {
        Self {
            radial_forces: self.radial_forces.clone(),
            item_objects: self.item_objects.clone(),
            stage_start_rows: self.stage_start_rows.clone(),
            stage_objects: self.stage_objects,
            state: self.state.clone(),
            pads: self.pads,
            raw_sticks: self.raw_sticks,
            recorded_pad_queue_x: self.recorded_pad_queue_x,
            raw_stick_x_history: self.raw_stick_x_history,
            pad_queue_x: self.pad_queue_x,
            consumed_sticks: self.consumed_sticks,
            after_map: self.after_map.clone(),
            after_map_values: self
                .after_map_values
                .as_ref()
                .map(hsd_types::storage::clone_vec),
            external: self.external,
            consumed: self.consumed,
            frame: self.frame,
            error: self.error.clone(),
            rng_writers: hsd_types::storage::clone_vec(&self.rng_writers),
            particle_draws: self.particle_draws.clone(),
            interface: self.interface.clone(),
            match_finished: self.match_finished,
            start_held: self.start_held,
            display_pass: self.display_pass,
        }
    }
}

impl Runtime {
    /// HSD_PadRenewMasterStatus dequeues one raw sample per tick: this
    /// tick's queue view and the history the next ticks will read.
    fn consume_pad_queue(&mut self) {
        self.consumed_sticks = self
            .raw_sticks
            .unwrap_or_else(|| self.pads.map(|pad| pad.raw_sticks()));
        for (port, sticks) in self.consumed_sticks.iter().enumerate() {
            let current = sticks.stick[0];
            let [previous, two_ticks_ago] = self.raw_stick_x_history[port];
            self.pad_queue_x[port] = melee_ft::input::PadQueueX {
                current,
                two_ticks_ago,
            };
            self.raw_stick_x_history[port] = [current, previous];
        }
        if let Some(recorded) = self.recorded_pad_queue_x {
            self.pad_queue_x = recorded;
        }
    }
    /// fn_8026C88C (8026C88C), the random item spawner's proc: when its
    /// countdown runs out, a kind from the pick table (it_8026C75C) drops
    /// in at a random item marker (it_8026CB3C: Stage_80224FDC, and no
    /// solid stage there) in a puff (efSync_Spawn 0x420); then the next
    /// interval. db_AreItemSpawnsEnabled is on in a retail match.
    fn spawn_random_item(&mut self, world: &mut World) -> Result<()> {
        let state = &mut self.state;
        let Some(spawner) = state.item_spawner.as_mut() else {
            return Ok(());
        };
        if !spawner.count_down() {
            return Ok(());
        }
        // it_8026C75C: Poke Balls switched off, or Item_804A0C64.x1C at its
        // limit (ItCo +C).
        let poke_balls = state
            .items
            .iter()
            .filter(|item| item.kind == melee_types::ItemKind::MBall)
            .count();
        let poke_ball_limit = state.items.common().hold_limits[9].unwrap_or(0);
        let barred = !spawner.switched_on(melee_types::ItemKind::MBall)
            || poke_balls >= poke_ball_limit;
        if barred && poke_balls > 0 {
            unimplemented!("it_8026C75C: Item_804A0C64.x1C counts Poke Balls (item.c:408)");
        }
        let kind = spawner.items.pick(barred, &mut state.rng);
        if let Some(kind) = kind {
            // it_8026CB3C: a marker, flattened onto the stage plane, outside
            // solid stage (mpColl_8004D024).
            let position = melee_gr::bomb_rain::random_item_marker(
                &state.assets.drop_markers,
                &mut state.rng,
            )
            .map(|mut position| {
                position.z = 0.0;
                position
            })
            .filter(|position| !state.map.is_point_inside_solid(position));
            if let Some(position) = position {
                // it_8026B684 -> ftLib_800864A8.
                let facing = crate::scene_items::facing_toward_fighters(
                    position,
                    &mut state.fighters,
                    &mut state.rng,
                );
                let spawned = crate::scene_items::spawn_random_item(
                    &mut state.items,
                    &mut state.effects.gobjs,
                    &state.assets.items,
                    &mut state.map,
                    world,
                    &mut self.item_objects,
                    kind,
                    position,
                    facing,
                    &mut state.rng,
                );
                if let Some(id) = spawned {
                    // efSync_Spawn(0x420, item, &pos); it_80274ED8 counts
                    // the spawn (it_804D6D0C, which nothing in a match reads).
                    state
                        .items
                        .get_mut(id)
                        .expect("the new item")
                        .events
                        .push(melee_it::ItemEvent::Effect { id: 0x420, position });
                }
                self.drain_item_events(false, world)?;
            }
        }
        let state = &mut self.state;
        let intervals = state.items.common().random_items.intervals;
        state
            .item_spawner
            .as_mut()
            .expect("the spawner")
            .restart(&intervals, &mut state.rng);
        Ok(())
    }
    /// Item requests made during a proc, in request order: effects spawn and
    /// draw now; efAsync requests below s_link 9 wait for the item's link 9
    /// flush (Item_80269A9C -> efAsync_QueueFlush), which `flush` performs.
    fn drain_item_events(&mut self, flush: bool, world: &mut World) -> Result<()> {
        // A container's release creates items, which the walk over the
        // pool cannot: it stops there, lets the items out, and goes on.
        while let Some((container, release)) = self.drain_item_events_to_release(flush)? {
            self.release_contents(container, release, world)?;
        }
        Ok(())
    }
    /// [`Self::drain_item_events`] up to a container's release, which it
    /// returns with the container.
    fn drain_item_events_to_release(
        &mut self,
        flush: bool,
    ) -> Result<Option<(u32, melee_it::ItemEvent)>> {
        let state = &mut self.state;
        for item in state.items.iter_mut() {
            if flush {
                while !item.queued_events.is_empty() {
                    let event = item.queued_events.remove(0);
                    item.events.push(event);
                }
            }
            while !item.events.is_empty() {
                match item.events.remove(0) {
                    release @ (melee_it::ItemEvent::Release { .. }
                    | melee_it::ItemEvent::CrateContents { .. }) => {
                        return Ok(Some((item.id, release)));
                    }
                    melee_it::ItemEvent::Resume {
                        step,
                        past_hitbox_refresh,
                        efasync_immediate,
                    } => {
                        // What the callback pushes now precedes what it
                        // had pushed after this point.
                        let mut rest = melee_types::fixed::FixedVec::<melee_it::ItemEvent, 8>::default();
                        while !item.events.is_empty() {
                            rest.push(item.events.remove(0));
                        }
                        let logic = <crate::scene_items::SceneItems as melee_it::ItemDispatch>::logic(
                            item.kind,
                        );
                        let previous = (item.past_hitbox_refresh, item.efasync_immediate);
                        item.past_hitbox_refresh = past_hitbox_refresh;
                        item.efasync_immediate = efasync_immediate;
                        (logic.resumed)(item, step, state.assets.items.get(item.kind), &mut state.rng);
                        (item.past_hitbox_refresh, item.efasync_immediate) = previous;
                        while !rest.is_empty() {
                            item.events.push(rest.remove(0));
                        }
                    }
                    melee_it::ItemEvent::Effect {
                        id: melee_ef::BOX_FRAGMENTS_REQUEST,
                        position,
                    } => {
                        state.effects.spawn_box_fragments::<RetailTrig>(
                            item.id,
                            position,
                            &state.assets.common_particle_bank,
                            &mut state.particles,
                            &mut state.rng,
                        )?;
                    }
                    melee_it::ItemEvent::HitSpark { position, damage } => {
                        state.effects.spawn_item_hit_spark::<RetailTrig>(
                            item.id,
                            position,
                            damage,
                            &state.assets.common_particle_bank,
                            &mut state.particles,
                            &mut state.rng,
                        )?;
                    }
                    melee_it::ItemEvent::DestroyEffect { id, root } => {
                        // it_80278800's lb_8000B1CC reads the root JObj's
                        // world translation: its last HSD_JObjSetTranslate,
                        // not item->pos (Link's arrow), or for a held item
                        // not dirtied since the display pass, its holder's
                        // part as drawn (Ness's yo-yo).
                        let held_root = item.held.then_some(item.held_display_root).flatten();
                        // it_80278800: zero offset and range, but the three
                        // spread draws still happen.
                        for _ in 0..3 {
                            state.rng.randf();
                        }
                        state.effects.spawn_positional::<RetailTrig>(
                            id,
                            root.or(held_root).unwrap_or(item.root_translation),
                            &state.assets.common_particle_bank,
                            &mut state.particles,
                            &mut state.rng,
                        )?;
                    }
                    melee_it::ItemEvent::OwnEffect { id } => {
                        let mut matrix = hsd_types::Mtx::default();
                        hsd_anim::mtx::hsd_mtx_srt(
                            &mut matrix,
                            &item.model_scale,
                            &item.rotation,
                            &item.position,
                            None,
                        );
                        let matrix = (<crate::scene_items::SceneItems as melee_it::ItemDispatch>::logic(
                            item.kind,
                        )
                        .effect_joint_matrix)(
                            item, state.assets.items.get(item.kind), matrix
                        );
                        state.effects.spawn_item_generators::<RetailTrig>(
                            id,
                            item.id,
                            matrix,
                            item.model_scale.y,
                            &state.assets.common_particle_bank,
                            &mut state.particles,
                            &mut state.rng,
                        )?;
                    }
                    melee_it::ItemEvent::FollowingEffect { id, offset } => {
                        state.effects.spawn_item_following_generator::<RetailTrig>(
                            id,
                            item.id,
                            offset,
                            &state.assets.common_particle_bank,
                            &mut state.particles,
                            &mut state.rng,
                        )?;
                    }
                    melee_it::ItemEvent::DestroyEffects => {
                        // A generator kept for its particles goes on reading
                        // the JObj, whose translation is the root's.
                        let mut matrix = hsd_types::Mtx::default();
                        hsd_anim::mtx::hsd_mtx_srt(
                            &mut matrix,
                            &item.model_scale,
                            &item.rotation,
                            &item.root_translation,
                            None,
                        );
                        let matrix = (<crate::scene_items::SceneItems as melee_it::ItemDispatch>::logic(
                            item.kind,
                        )
                        .effect_joint_matrix)(
                            item, state.assets.items.get(item.kind), matrix
                        );
                        state
                            .effects
                            .update_item_joint(item.id, matrix, &mut state.particles);
                        // efLib_DestroyAll: the item's models, then its JObj.
                        state
                            .effects
                            .destroy_item_models(item.id, &mut state.particles);
                        state
                            .effects
                            .expire_item_joint(item.id, &mut state.particles);
                    }
                    melee_it::ItemEvent::JointParticle {
                        bank,
                        id,
                        joint,
                        position,
                        rotation,
                        scale,
                    } => {
                        let root = melee_it::pose::RootSrt {
                            translate: position,
                            rotate: rotation,
                            scale,
                        };
                        let matrix = if joint == 0 {
                            let mut matrix = hsd_types::Mtx::default();
                            hsd_anim::mtx::hsd_mtx_srt(
                                &mut matrix,
                                &root.scale,
                                &root.rotate,
                                &root.translate,
                                None,
                            );
                            matrix
                        } else {
                            // efLib_SpawnParticleEffect attaches the
                            // generator to the joint whose track fired.
                            anyhow::ensure!(
                                !crate::article_pose::ArticlePoses::poses_kind(
                                    &state.assets.items.article_skeletons,
                                    item.kind
                                ),
                                "a held article's DPtcl key below its root"
                            );
                            state
                                .assets
                                .items
                                .get(item.kind)
                                .pose
                                .as_ref()
                                .expect("a DPtcl key below the root without a sampled pose")
                                .bone_matrix(item.article_state, item.pose_steps, joint, root)
                        };
                        state.effects.spawn_item_particle::<RetailTrig>(
                            bank,
                            id,
                            item.id,
                            (joint != 0).then_some(joint),
                            matrix,
                            &state.assets.common_particle_bank,
                            &mut state.particles,
                            &mut state.rng,
                        )?;
                    }
                    melee_it::ItemEvent::ChildEffect { id, facing } => {
                        // HSD_JObjGetChild(root): the model's second joint.
                        const CHILD: usize = 1;
                        let matrix = crate::scene_items::free_item_bone_matrix(
                            &state.assets.items,
                            item,
                            CHILD,
                        );
                        state
                            .effects
                            .spawn_item_bone_generator_facing::<RetailTrig>(
                                id,
                                item.id,
                                CHILD,
                                matrix,
                                facing,
                                &mut state.particles,
                                &mut state.rng,
                            )?;
                    }
                    melee_it::ItemEvent::BoneGenerator { id, bone } => {
                        let matrix = article_bone_matrix(
                            &mut state.fighters,
                            &mut state.article_poses,
                            &state.assets.items.article_skeletons,
                            &state.assets.items,
                            item,
                            bone,
                            self.frame,
                        );
                        state.effects.spawn_item_bone_generator::<RetailTrig>(
                            id,
                            item.id,
                            bone,
                            matrix,
                            &mut state.particles,
                            &mut state.rng,
                        )?;
                    }
                    melee_it::ItemEvent::BoneModel { id, bone } => {
                        let matrix = article_bone_matrix(
                            &mut state.fighters,
                            &mut state.article_poses,
                            &state.assets.items.article_skeletons,
                            &state.assets.items,
                            item,
                            bone,
                            self.frame,
                        );
                        state.effects.spawn_kirby_hammer_model::<RetailTrig>(
                            id,
                            item.id,
                            bone,
                            matrix,
                            &state.assets.common_particle_bank,
                            &mut state.particles,
                            &mut state.rng,
                        )?;
                    }
                    melee_it::ItemEvent::SlashSpark { position } => {
                        state.effects.spawn_item_slash_spark::<RetailTrig>(
                            item.id,
                            position,
                            &state.assets.common_particle_bank,
                            &mut state.particles,
                            &mut state.rng,
                        )?;
                    }
                    melee_it::ItemEvent::IceSpark { position, facing } => {
                        state.effects.spawn_ice_hit_spark::<RetailTrig>(
                            position,
                            facing,
                            &state.assets.common_particle_bank,
                            &mut state.particles,
                            &mut state.rng,
                        )?;
                    }
                    melee_it::ItemEvent::CoinSpark { position } => {
                        state.effects.spawn_positional::<RetailTrig>(
                            state.effects.coin_hit_request(),
                            position,
                            &state.assets.common_particle_bank,
                            &mut state.particles,
                            &mut state.rng,
                        )?;
                    }
                    melee_it::ItemEvent::Effect { id, position } => {
                        state.effects.spawn_positional::<RetailTrig>(
                            id,
                            position,
                            &state.assets.common_particle_bank,
                            &mut state.particles,
                            &mut state.rng,
                        )?;
                    }
                    event @ (melee_it::ItemEvent::ScriptEffect(_)
                    | melee_it::ItemEvent::ScriptEffectNow(_)) => {
                        let (graphics, now) = match event {
                            melee_it::ItemEvent::ScriptEffect(graphics) => (graphics, false),
                            melee_it::ItemEvent::ScriptEffectNow(graphics) => (graphics, true),
                            _ => unreachable!(),
                        };
                        // it_80278800: the offset's random spread, x then y then z.
                        let mut offset = graphics.offset;
                        let range = graphics.range;
                        offset.x += 2.0 * range.x * (state.rng.randf() - 0.5);
                        offset.y += 2.0 * range.y * (state.rng.randf() - 0.5);
                        offset.z += 2.0 * range.z * (state.rng.randf() - 0.5);
                        // efAsync_Spawn below s_link 9 queues on the item.
                        let event = match graphics.id {
                            // block_680 / 6B4 / 6E8: efAsync EF_SPAWN_CAMERA_SHAKE.
                            0x513..=0x515 => melee_it::ItemEvent::Quake {
                                kind: graphics.id - 0x511,
                                joint: graphics.bone,
                                offset,
                            },
                            // block_4A8: efAsync kind 2 (the bounce spark).
                            0x405 => melee_it::ItemEvent::JointEffect {
                                id: graphics.id,
                                joint: graphics.bone,
                                offset,
                            },
                            id => anyhow::bail!("it_80278800: item effect {id:#x}"),
                        };
                        if now {
                            // Link 9 and later: created now, ahead of what
                            // the same callback spawns next (a bursting
                            // egg's quake, then its shell).
                            item.events.insert(0, event);
                        } else {
                            item.queued_events.push(event);
                        }
                    }
                    melee_it::ItemEvent::JointEffect { id, joint, offset } => {
                        ensure!(joint == 0, "item effect at joint {joint}");
                        // lb_8000B1CC(jobj, offset): the root's world matrix.
                        let mut matrix = hsd_types::Mtx::default();
                        hsd_anim::mtx::hsd_mtx_srt(
                            &mut matrix,
                            &item.model_scale,
                            &item.rotation,
                            &item.position,
                            None,
                        );
                        let mut position = Vec3::ZERO;
                        hsd_anim::mtx::mtx_mult_vec(&matrix, &offset, &mut position);
                        state.effects.spawn_positional::<RetailTrig>(
                            id,
                            position,
                            &state.assets.common_particle_bank,
                            &mut state.particles,
                            &mut state.rng,
                        )?;
                    }
                    melee_it::ItemEvent::RootEffect { id, parameter } => {
                        // lb_8000B1CC(jobj, NULL): the root's world translation.
                        state.effects.spawn_item_root::<RetailTrig>(
                            item.id,
                            id,
                            item.position,
                            parameter,
                            &state.assets.common_particle_bank,
                            &mut state.particles,
                            &mut state.rng,
                        )?;
                    }
                    melee_it::ItemEvent::Quake {
                        kind,
                        joint,
                        offset,
                    } => {
                        ensure!(joint == 0, "item quake at joint {joint}");
                        // lb_8000B1CC(jobj, offset): the root's world matrix.
                        let mut matrix = hsd_types::Mtx::default();
                        hsd_anim::mtx::hsd_mtx_srt(
                            &mut matrix,
                            &item.model_scale,
                            &item.rotation,
                            &item.position,
                            None,
                        );
                        let mut _position = Vec3::ZERO;
                        hsd_anim::mtx::mtx_mult_vec(&matrix, &offset, &mut _position);
                        let kind = match kind {
                            2 => melee_cm::QuakeKind::Small,
                            3 => melee_cm::QuakeKind::Medium,
                            4 => melee_cm::QuakeKind::Large,
                            _ => unreachable!(),
                        };
                        state
                            .quakes
                            .request(&mut state.camera, kind, &mut state.effects.gobjs);
                    }
                    melee_it::ItemEvent::Gust {
                        center,
                        frames,
                        strength,
                        decay,
                        phase_step,
                    } => {
                        self.radial_forces
                            .insert(melee_lb::radial_force::RadialImpulse {
                                center,
                                frames,
                                strength,
                                decay,
                                phase_step,
                            });
                    }
                    melee_it::ItemEvent::BoneGust {
                        bone,
                        frames,
                        strength,
                        decay,
                        phase_step,
                    } => {
                        // lb_8000B1CC(bone, NULL): the bone's world translation.
                        let matrix = article_bone_matrix(
                            &mut state.fighters,
                            &mut state.article_poses,
                            &state.assets.items.article_skeletons,
                            &state.assets.items,
                            item,
                            bone,
                            self.frame,
                        );
                        self.radial_forces
                            .insert(melee_lb::radial_force::RadialImpulse {
                                center: Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]),
                                frames,
                                strength,
                                decay,
                                phase_step,
                            });
                    }
                }
            }
        }
        Ok(None)
    }

    /// it_8026F3D4 (8026F3D4): a container lets out `count` items, each a
    /// draw from the contents table (it_8026C75C on it_804A0E50), created
    /// at the container (it_8026F5C8) and launched (it_8026F53C). A draw
    /// that finds nothing ends the release. ftLib_800874CC's tally of what
    /// came out is the owner's match statistics only.
    fn release_contents(
        &mut self,
        container: u32,
        release: melee_it::ItemEvent,
        world: &mut World,
    ) -> Result<()> {
        let state = &mut self.state;
        let (count, same_kind, position, previous_position) = match release {
            melee_it::ItemEvent::Release {
                count,
                same_kind,
                position,
                previous_position,
            } => (count, same_kind, position, previous_position),
            melee_it::ItemEvent::CrateContents {
                roll,
                position,
                previous_position,
            } => {
                // it_8026F6BC (8026F6BC): under the hold-kind 6 limit, one
                // crate in ItCo +124 lets out a monster instead.
                let common = state.items.common().random_items;
                let monsters = state.items.iter().filter(|item| item.hold_kind == 6).count();
                if (monsters as i32) < common.monster_limit
                    && state.rng.randi(common.monster_odds) == 0
                {
                    unimplemented!("it_8026F6BC: a monster from a crate (it_8026C65C on it_804A0E60)");
                }
                let Some(melee_it::CrateRoll {
                    weights: [one, two, three],
                    same_kind_odds,
                }) = roll
                else {
                    // it_802886C4: a barrel lets out one.
                    return self.release_contents(
                        container,
                        melee_it::ItemEvent::Release {
                            count: 1,
                            same_kind: false,
                            position,
                            previous_position,
                        },
                        world,
                    );
                };
                // it_80286248: the same-kind roll, then one, two or three.
                let same_kind = state.rng.randi(same_kind_odds) == 0;
                let two_or_fewer = one + two;
                let roll = state.rng.randi(three + two_or_fewer);
                let count = if roll < one {
                    1
                } else if roll < two_or_fewer {
                    2
                } else {
                    3
                };
                (count, same_kind, position, previous_position)
            }
            _ => unreachable!("a release event"),
        };
        let Some(spawner) = state.item_spawner else {
            // it_8026C75C on an empty table: x8 is zero without a spawner.
            return Ok(());
        };
        let attack = state.items.get_mut(container).expect("the container").stale_source;
        let mut kind = None;
        for _ in 0..count {
            let poke_balls = state
                .items
                .iter()
                .filter(|item| item.kind == melee_types::ItemKind::MBall)
                .count();
            let limit = state.items.common().hold_limits[9].unwrap_or(0);
            let barred = !spawner.switched_on(melee_types::ItemKind::MBall) || poke_balls >= limit;
            if barred && poke_balls > 0 {
                unimplemented!("it_8026C75C: Item_804A0C64.x1C counts Poke Balls (item.c:408)");
            }
            // it_8026F3D4: with arg3 bit 0, the first draw's kind repeats.
            if !same_kind || kind.is_none() {
                kind = spawner.contents.pick(barred, &mut state.rng);
            }
            let Some(kind) = kind else {
                break;
            };
            if kind == melee_types::ItemKind::MBall {
                // it_8026F3D4's it_8026C704 test (itdrop.c:63).
                unimplemented!("it_8026F3D4: a Poke Ball from a container (it_8026C704)");
            }
            // it_8026F5C8: facing from the stage-plane position.
            let facing = crate::scene_items::facing_toward_fighters(
                previous_position,
                &mut state.fighters,
                &mut state.rng,
            );
            let spawned = crate::scene_items::spawn_released_item(
                &mut state.items,
                &mut state.effects.gobjs,
                &state.assets.items,
                &mut state.map,
                world,
                &mut self.item_objects,
                crate::scene_items::Released {
                    kind,
                    position,
                    previous_position,
                    facing,
                    attack,
                },
                &mut state.rng,
            );
            if let Some(id) = spawned {
                // it_8026F53C: x54 * (2 * (r - 0.5)) sideways (separate
                // fmuls, 8026F578 / 8026F57C), the kind's x18 upward.
                let common = state.items.common().random_items;
                let random = state.rng.randf();
                let item = state.items.get_mut(id).expect("the released item");
                let rise = state.assets.items.get(kind).launch_vertical_velocity;
                item.velocity = Vec3::new(common.release_spread * (2.0 * (random - 0.5)), rise, 0.0);
                // it_80274ED8: the spawn count nothing in a match reads.
            }
        }
        Ok(())
    }

    /// Requests between linked articles made during this proc, in request
    /// order (see [`melee_it::LinkRequest`]). Returns an article that spawned
    /// another from the proc that ended it (Din's Fire's explosion).
    fn deliver_item_links(&mut self, s_link: u8, world: &mut World) -> Result<Option<u32>> {
        use crate::scene_items::SceneItems;
        use melee_it::{ItemDispatch, LinkMessage, LinkTarget};
        let state = &mut self.state;
        let mut ended_spawner = None;
        loop {
            let pending = state
                .items
                .iter()
                .find(|item| !item.link_requests.is_empty())
                .map(|item| item.id);
            let Some(sender) = pending else {
                break;
            };
            let request = state.items.get_mut(sender).unwrap().link_requests.remove(0);
            let target = match request.target {
                LinkTarget::Item(target) => target,
                LinkTarget::Spawn(mut spawn) => {
                    // it_8027B0C4: the parent fighter's current attack, or
                    // the spawning article's own when the spawn names it as
                    // the second parent (0x8027B158). An article spawned
                    // with no parent (a PK Thunder segment of a head that
                    // left its creator, it_802AC43C(NULL)) has none.
                    let secondary = state.items.get_mut(sender).unwrap().owner_secondary;
                    let sender_attack = state.items.get_mut(sender).unwrap().stale_source;
                    let sender_group = state.items.get_mut(sender).unwrap().hit_group;
                    let index = spawn.owner.map(|owner| {
                        crate::scene_items::owner_index(&state.fighters, Some(owner), secondary)
                            .expect("linked article owner")
                    });
                    let (stale_multiplier, owner_secondary) = match index {
                        Some(index) => {
                            let fighter = &state.fighters[index];
                            spawn.stale_source = if spawn.attack_from_spawner {
                                sender_attack
                            } else {
                                fighter.combat.stale.attack()
                            };
                            (
                                fighter.combat.stale.multiplier_for(
                                    spawn.stale_source.map(|attack| attack.move_id),
                                    &state.assets.fighters[index].stale_weights,
                                ),
                                fighter.player.secondary,
                            )
                        }
                        None => (1.0, false),
                    };
                    let id = crate::scene_items::request(
                        &mut state.items,
                        &mut state.effects.gobjs,
                        &state.assets.items,
                        &mut state.map,
                        world,
                        &mut self.item_objects,
                        melee_it::ItemRequest::Spawn(spawn),
                        crate::scene_items::RequestOwner {
                            slot: spawn.owner,
                            secondary: owner_secondary,
                            held_item: None,
                            after_hitbox_refresh: s_link > 11,
                            efasync_immediate: s_link >= 9,
                            stale_multiplier,
                            // An article's own spawn runs no procs at spawn.
                            bounds: None,
                        },
                        &mut state.rng,
                    )
                    .unwrap_or_else(|| unimplemented!("it_802B4224: Item_80268B18 found no room"));
                    state.items.get_mut(id).unwrap().partner = Some(sender);
                    if sender_group != 0 {
                        // it_802AC43C: SpawnItem.x40 is the head's xAC4.
                        state.items.join_hit_group(id, sender_group);
                    }
                    let spawner = state.items.get_mut(sender).unwrap();
                    spawner.partner = Some(id);
                    if spawner.destroyed {
                        ended_spawner = Some(sender);
                    }
                    id
                }
            };
            let Some(item) = state.items.get_mut(target) else {
                continue;
            };
            if request.message == LinkMessage::Unlinked {
                item.partner = None;
                continue;
            }
            if request.message == LinkMessage::Spawned {
                continue;
            }
            let assets = state.assets.items.get(item.kind);
            let receive = SceneItems::logic(item.kind).link_received;
            item.destroyed |= receive(item, request.message, assets);
        }
        Ok(ended_spawner)
    }

    /// An item that tells its owner of its end inside the proc that ended
    /// it (`ItemLogic::NOTICE_AT_ONCE`). From the ending callback itself,
    /// what the notice does to the owner's effects (ftNs_SpecialHiStopGFX's
    /// efLib_DestroyAll) happens before the item's own Item_8026A8EC
    /// events (`after_teardown` false); from that function's `destroyed`
    /// callback (item.c:1997) it follows them.
    fn notify_owner_at_once(&mut self, id: u32, after_teardown: bool) -> Result<()> {
        use crate::scene_items::SceneItems;
        use melee_it::ItemDispatch;
        let state = &mut self.state;
        let Some(item) = state.items.get_mut(id) else {
            return Ok(());
        };
        let logic = SceneItems::logic(item.kind);
        if !item.destroyed
            || item.owner_notified
            || !logic.notice_at_once
            || (!after_teardown && !(logic.notice_precedes_teardown)(item))
        {
            return Ok(());
        }
        crate::scene_items::article_destroyed(&mut state.fighters, item);
        item.owner_notified = true;
        let (owner, secondary) = (item.owner, item.owner_secondary);
        for (slot, fighter) in state.fighters.iter_mut().enumerate() {
            crate::scene_fighter::with_fighter!(fighter, |f| {
                if Some(f.player.id) == owner && f.player.secondary == secondary {
                    state.effects.flush::<RetailTrig>(
                        melee_ef::EffectTiming::Immediate,
                        slot,
                        &mut f.core,
                        &state.assets.common_particle_bank,
                        &state.assets.particle_bank,
                        &mut state.particles,
                        &mut state.rng,
                    )?;
                }
            });
        }
        Ok(())
    }
    fn dispatch_item(&mut self, id: u32, phase: u8) -> Result<()> {
        use crate::scene_items::SceneItems;
        let state = &mut self.state;
        let Some(item) = state.items.get_mut(id) else {
            return Ok(());
        };
        let kind = item.kind;
        let owner_slot = item.owner;
        let owner_secondary = item.owner_secondary;
        let attack = item.stale_source;
        let reflected_owner = item.pending_reflection.map(|p| {
            if p.preserve_owner {
                (item.owner, owner_secondary)
            } else {
                (Some(p.owner), false)
            }
        });
        let stale_for = |owner: Option<(Option<u8>, bool)>| {
            owner
                .and_then(|(slot, secondary)| {
                    crate::scene_items::owner_index(&state.fighters, slot, secondary)
                })
                .map_or(1.0, |i| {
                    state.fighters[i].combat.stale.multiplier_for(
                        attack.map(|a| a.move_id),
                        &state.assets.fighters[i].stale_weights,
                    )
                })
        };
        let current_stale = stale_for(Some((owner_slot, owner_secondary)));
        let reflected_stale = stale_for(reflected_owner);
        item.stale_multiplier = current_stale;
        // The fighter the kind's callbacks read: the owner, or the creator
        // a kind keeps past a reflection (`ItemLogic::tracker`).
        let tracker = (<SceneItems as melee_it::ItemDispatch>::logic(kind).tracker)(item);
        let owner = tracker.and_then(|(slot, secondary)| {
            let index = crate::scene_items::owner_index(&state.fighters, Some(slot), secondary)?;
            Some(crate::scene_fighter::with_fighter!(
                &mut state.fighters[index],
                |f| f.item_owner(&state.assets.fighters[index])
            ))
        });
        // The fighter whose shield the item reads in this proc: the one it
        // is stuck to, or for the hit-shield callback the one it struck
        // (xCF4).
        if matches!(phase, 1 | 4 | 14) {
            let watched = state.items.get_mut(id).and_then(|item| {
                if phase == 14 {
                    item.pending_shield_owner.or(item.shield_anchor)
                } else {
                    item.shield_anchor
                }
            });
            if let Some(slot) = watched {
                let view = crate::scene_items::owner_index(&state.fighters, Some(slot), false)
                    .map(|index| {
                        crate::scene_fighter::with_fighter!(&mut state.fighters[index], |f| f
                            .shield_view(&state.assets.fighters[index]))
                    });
                if let Some(item) = state.items.get_mut(id) {
                    item.shield_view = view;
                }
            }
        }
        match phase {
            0 => state.items.advance_hitlag(id),
            1 => {
                // fp->item_gobj: the fighter holding this item in hand. Character
                // articles held at spawn (the Blaster) are not item_gobj.
                let held_part = state
                    .items
                    .get_mut(id)
                    .and_then(|item| item.held.then_some(item.holder_part));
                let holder_index = state
                    .fighters
                    .iter()
                    .position(|f| f.0.core.held_item.is_some_and(|held| held.item == id));
                // A thrown holder's XRotN follows its captor's TransN2 through
                // a robj constraint retail evaluates when the hand's matrix is
                // rebuilt; the captor may have moved since the holder's last
                // proc refreshed it.
                if let Some(index) = holder_index {
                    grab_pairs::constrain(state, index);
                }
                let partner =
                    crate::scene_items::partner_view(&state.items, &state.assets.items, id);
                let holder = match (holder_index, held_part) {
                    (Some(index), Some(part)) => Some(
                        state.fighters[index]
                            .0
                            .core
                            .item_holder(part, &state.assets.fighters[index]),
                    ),
                    _ => None,
                };
                let rng = std::cell::Cell::new(state.rng);
                state.items.animate::<SceneItems>(
                    id,
                    state.assets.items.get(kind),
                    owner.as_ref(),
                    holder,
                    &mut state.map,
                    partner,
                    &rng,
                );
                state.rng = rng.get();
                // Item_8026A848 -> ftCommon_8007E6DC: the hand lets go.
                let released = state.items.get_mut(id).is_none_or(|item| !item.held);
                if let (Some(index), true) = (holder_index, released) {
                    state.fighters[index]
                        .0
                        .core
                        .release_held_item(id, &state.assets.fighters[index]);
                }
            }
            4 => {
                // Item_80269CA0: a kind whose phys_cb is its owner's work
                // (OWNER_PHYSICS), skipped in hitlag and while frozen as the
                // item's own callback is.
                if <SceneItems as melee_it::ItemDispatch>::logic(kind).owner_physics {
                    let runs = state
                        .items
                        .get_mut(id)
                        .is_some_and(|item| !item.frozen && !item.in_hitlag);
                    let index = crate::scene_items::owner_index(
                        &state.fighters,
                        owner_slot,
                        owner_secondary,
                    );
                    if let (true, Some(index)) = (runs, index) {
                        crate::scene_fighter::with_fighter!(&mut state.fighters[index], |f| f
                            .article_physics(&state.assets.fighters[index], &mut state.map));
                    }
                }
                let rng = std::cell::Cell::new(state.rng);
                let mut targets = melee_it::LockOnTargets::default();
                if <SceneItems as melee_it::ItemDispatch>::logic(kind).locks_on {
                    lock_on_targets(state, &mut targets);
                }
                state.items.physics::<SceneItems>(
                    id,
                    owner.as_ref(),
                    &targets,
                    &item_bounds(&state.assets),
                    state.assets.items.get(kind),
                    &state.map,
                    &rng,
                );
                state.rng = rng.get();
            }
            5 => {
                let contact = state.items.stage_contact(id, &mut state.map);
                let rng = std::cell::Cell::new(state.rng);
                state.items.collide::<SceneItems>(
                    id,
                    owner.as_ref(),
                    contact,
                    &mut state.map,
                    &item_bounds(&state.assets),
                    state.assets.items.get(kind),
                    Some(&rng),
                );
                state.rng = rng.get();
            }
            9 => {
                let item = state.items.get_mut(id).unwrap();
                // Item_80269A9C: a framed item's camera subject follows it.
                let position = item.position;
                if let Some(subject) = item.camera.as_mut() {
                    subject.position = position;
                    subject.bone_position = position;
                }
                // Item_80269A9C: hitlag skips the accessory callback.
                let in_hitlag = item.in_hitlag;
                let logic = <SceneItems as melee_it::ItemDispatch>::logic(kind);
                if logic.owner_accessory && !in_hitlag && !item.frozen {
                    // The kind's on_accessory is its owner's work.
                    let index = crate::scene_items::owner_index(
                        &state.fighters,
                        item.owner,
                        item.owner_secondary,
                    );
                    if let Some(index) = index {
                        let motion = crate::scene_fighter::with_fighter!(
                            &mut state.fighters[index],
                            |f| f.article_accessory(&state.assets.fighters[index], &mut state.map)
                        );
                        if let Some(motion) = motion {
                            let item = state.items.get_mut(id).unwrap();
                            (logic.control)(
                                item,
                                melee_it::ItemControl::Motion(motion),
                                state.assets.items.get(kind),
                            );
                        }
                    }
                }
                let item = state.items.get_mut(id).unwrap();
                let blast = if in_hitlag {
                    None
                } else {
                    (<SceneItems as melee_it::ItemDispatch>::logic(kind).accessory)(
                        item,
                        owner.as_ref(),
                        state.assets.items.get(kind),
                    )
                };
                if let Some(blast) = blast {
                    crate::scene_items::owner_blast(state, &blast);
                }
                let item = state.items.get_mut(id).unwrap();
                if let melee_it::ItemScratch::Held(held) = &mut item.scratch {
                    if held.shot_pending && !in_hitlag {
                        held.shot_pending = false;
                        let model = held.muzzle_model;
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
                            model,
                            position,
                            angle,
                            &state.assets.common_particle_bank,
                            &mut state.particles,
                            &mut state.rng,
                        )?;
                    }
                }
                // Item_80269A9C's tail: it_80272280, then it_802722B0's
                // sample of every fighter's cur_pos (ftCo_80098634) by the
                // first item in the list.
                state.items.sample_fighters_for_push(
                    id,
                    state.fighters.iter().map(|f| f.0.core.physics.position),
                );
            }
            11 => {
                let item = state.items.get_mut(id).unwrap();
                item.update_hitboxes(state.assets.items.get(kind));
                item.decay_reflection_history();
            }
            14 => {
                // ftLib_800866DC of the reflector, as a reflected callback
                // reads it inside this proc (it_802A20E8).
                let reflector = state
                    .items
                    .get_mut(id)
                    .and_then(|item| item.pending_reflection)
                    .map(|pending| pending.owner);
                if let Some(reflector) = reflector {
                    if let Some(index) = state.fighters.iter().position(|fighter| {
                        crate::scene_fighter::with_fighter!(fighter, |f| f.player.id == reflector)
                    }) {
                        let position =
                            crate::scene_fighter::with_fighter!(&mut state.fighters[index], |f| f
                                .core
                                .camera_bone_position());
                        if let Some(pending) = state
                            .items
                            .get_mut(id)
                            .and_then(|item| item.pending_reflection.as_mut())
                        {
                            pending.reflector_position = position;
                        }
                    }
                }
                let rng = std::cell::Cell::new(state.rng);
                state.items.process_events_with_stale::<SceneItems>(
                    id,
                    reflected_stale,
                    state.assets.items.get(kind),
                    &rng,
                );
                state.rng = rng.get();
            }
            13 => self.detect_item_hurts(id),
            12 | 16 => {}
            _ => unreachable!(),
        }
        // An article's call into a fighter from inside its proc.
        let request = self
            .state
            .items
            .get_mut(id)
            .and_then(|item| item.owner_request.take());
        if let Some((slot, request)) = request {
            deliver_owner_request(&mut self.state, id, kind, slot, request);
        }
        // it_8026FCF8: hitboxes this proc's scripts started in a hit group.
        self.state.items.resolve_group_histories();
        Ok(())
    }
    /// Item_80269C5C (item link 13): it_802703E8 lands fighter hitboxes on
    /// the item's hurt capsules, fighters in list order, then it_802706D0
    /// other items' hitboxes; it_80270E30 resolves the log.
    fn detect_item_hurts(&mut self, id: u32) {
        let state = &mut self.state;
        let Some(item) = state.items.get_mut(id) else {
            return;
        };
        let assets = state.assets.items.get(item.kind);
        let capsules = item.hurt_capsules(assets);
        let mut log = melee_it::hurt::ItemHitLog::default();
        // it_802703E8 returns at once without hurt capsules; it_802706D0
        // still meets other items' hitboxes with this item's.
        let owner =
            crate::scene_items::owner_index(&state.fighters, item.owner, item.owner_secondary);
        let owner_spawn = owner.map(|index| state.fighters[index].spawn_number);
        // x20_team_id: the owner's team (U8_MAX without one).
        let item_team = owner.and_then(|index| state.fighters[index].player.team);
        if !capsules.is_empty() {
            let teams: [Option<u8>; MAX_FIGHTERS] = std::array::from_fn(|index| {
                state.fighters.get(index)
                    .and_then(|fighter| thrown_hitbox_team(&state.fighters, fighter))
            });
            for (fighter, team) in state.fighters.iter_mut().zip(teams) {
                // itcoll.c:425-433: a team battle without friendly fire
                // spares the item's team (a thrown fighter counts as its
                // thrower's) until the owner lets it go (xDCE b1).
                let spared = fighter.player.team.is_some()
                    && !fighter.player.friendly_fire
                    && team.is_some()
                    && team == item_team;
                let hits = crate::scene_fighter::with_fighter!(fighter, |f| {
                    f.strike_item(item, &capsules, owner_spawn, spared)
                });
                for hit in hits {
                    log.push(hit);
                }
            }
        }
        let teams = ItemTeams::of(&state.fighters);
        item_hits_by_items(&mut state.items, id, &capsules, &teams, &mut log);
        if !log.is_empty() {
            let constants = state.items.common().knockback;
            let item = state.items.get_mut(id).expect("struck item");
            item.resolve_hits(&log, &constants, state.assets.items.get(item.kind));
        }
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
        // Each of a player's fighters sees the player's stale-move table as
        // the last proc left it.
        partner_fighters::share_stale_tables(&mut self.state);
        let state = &mut self.state;
        let state_pads = &self.pads;
        // An event callback that spawns an article and ends its own item
        // (the PK Fire bolt's it_2725_Logic23_DmgDealt): Item_8026A8EC's
        // efLib_DestroyAll (item.c:1991) and then its destroy effect follow
        // the new article's creation, so the article's generators are
        // inserted before that walk moves the insertion cursor.
        let mut spawner_teardown: Option<(u32, [Option<melee_it::ItemEvent>; 2])> = None;
        match row.callback {
            Callback::Item { id, phase } => {
                self.dispatch_item(id, phase)?;
                self.notify_owner_at_once(id, false)?;
                if let Some(item) = self.state.items.get_mut(id) {
                    let spawns = item
                        .link_requests
                        .iter()
                        .any(|r| matches!(r.target, melee_it::LinkTarget::Spawn(_)));
                    if spawns && item.destroyed {
                        let mut held = [None, None];
                        let walk = item
                            .events
                            .iter()
                            .enumerate()
                            .filter(|(_, e)| matches!(e, melee_it::ItemEvent::DestroyEffects))
                            .map(|(index, _)| index)
                            .last();
                        if let Some(index) = walk {
                            held[0] = Some(item.events.remove(index));
                        }
                        let effect = item
                            .events
                            .iter()
                            .position(|e| matches!(e, melee_it::ItemEvent::DestroyEffect { .. }));
                        if let Some(index) = effect {
                            held[1] = Some(item.events.remove(index));
                        }
                        spawner_teardown = Some((id, held));
                    }
                }
                self.drain_item_events(phase == 9, world)?;
                self.notify_owner_at_once(id, true)?;
            }
            Callback::Banner => {
                // if_802F73C4: the element's start callback runs on its first
                // step (x12.x2), before the animation.
                if let Some(kind) = state.banner.as_mut().and_then(|b| b.announce()) {
                    match kind {
                        // fn_8016B7B4(idx): Ground_801C1154 is empty.
                        BannerKind::Countdown => crate::scene_stage::stadium::show(
                            state,
                            melee_gr::stadium::ScreenMode::CountdownStart,
                        ),
                        BannerKind::SuddenDeathCountdown => crate::scene_stage::stadium::show(
                            state,
                            melee_gr::stadium::ScreenMode::SuddenDeathStart,
                        ),
                        BannerKind::Go => {}
                    }
                }
                let finished = state.banner.as_mut().is_some_and(|banner| banner.tick());
                if finished {
                    match state.banner.as_ref().unwrap().kind {
                        BannerKind::Countdown | BannerKind::SuddenDeathCountdown => {
                            // fn_8016B7F8: ftLib_800868A4 releases every input freeze.
                            for fighter in &mut state.fighters {
                                crate::scene_fighter::with_fighter!(fighter, |f| f
                                    .status
                                    .input_frozen =
                                    false);
                            }
                            // Ground_801C0FB8 invokes the stage's deferred start callback
                            // when Versus releases the countdown (grLast_8021A9AC).
                            if let SceneStage::FinalDestination(stage) = &mut state.stage {
                                stage.ground.start();
                                // Ground_801C0FB8: GObj_Create(stage, p_link, 0)
                                // and its proc, one per deferred row.
                                for &(index, p_link, s_link) in &self.stage_start_rows {
                                    let object = world.create(0, p_link, 0);
                                    world.add_tagged_proc(object, s_link, index);
                                }
                            }
                            if let SceneStage::Stadium(stage) = &mut state.stage {
                                stage.start(state.assets.stage_desc.parameters.map_scale);
                            }
                            // grOldKongo_8020F4E8: ftCo_800C0764 registers the
                            // barrel as a catch device.
                            if let SceneStage::OldKongo(stage) = &mut state.stage {
                                stage.catch_enabled = true;
                            }
                            // grGarden_80202D9C: the river's device.
                            if let SceneStage::Garden(stage) = &mut state.stage {
                                stage.start();
                            }
                            // grKongo_801D52FC (the barrel's catch device),
                            // then the deferred fn_801D542C (the bridge's
                            // landing callback, the first event wait).
                            if let SceneStage::Kongo(stage) = &mut state.stage {
                                stage.catch_active = true;
                                state.map.record_joint_events(
                                    melee_gr::kongo::planks::BRIDGE_COLLISION_JOINT,
                                    true,
                                );
                                stage.start(&mut state.rng);
                            }
                            // grStadium_801D4040.
                            crate::scene_stage::stadium::show(
                                state,
                                melee_gr::stadium::ScreenMode::CountdownEnd,
                            );
                            let gobjs = &mut state.effects.gobjs;
                            // Ground_801C0FB8: the stage's on_start, then
                            // Ground_801C0C2C's own GObj (ground.c:708).
                            for &class in state.stage.start_gobjs() {
                                gobjs.create(class);
                            }
                            // ifStatus_802F6EA4(4, ..., fn_8016B784): GO.
                            // Its GObj joins this s_link 0 pass after the
                            // countdown's, so it takes its first step now.
                            let mut go = state.go_banner.take().expect("preloaded GO banner");
                            go.gobj = Some(gobjs.create(hsd_gobj::GObjClass::Hud));
                            go.start();
                            assert!(!go.tick(), "GO banner ended on its first step");
                            // if_802F73C4 destroys the countdown's GObj,
                            // which is freed once this proc returns.
                            let countdown = state.banner.replace(go).expect("the countdown");
                            let address = countdown.gobj.unwrap_or_else(|| gobjs.outside());
                            gobjs.destroy(hsd_gobj::GObjClass::Hud, address);
                        }
                        BannerKind::Go => {
                            // fn_8016B784: the HUD, and with it the match clock.
                            state.clock.hud_enabled = true;
                            let go = state.banner.take().expect("the GO banner");
                            let gobjs = &mut state.effects.gobjs;
                            let address = go.gobj.unwrap_or_else(|| gobjs.outside());
                            gobjs.destroy(hsd_gobj::GObjClass::Hud, address);
                            // grStadium_801D4150.
                            crate::scene_stage::stadium::show(
                                state,
                                melee_gr::stadium::ScreenMode::MatchInfo,
                            );
                        }
                    }
                }
            }
            Callback::Camera => {
                // cm_804D6468 runs newest first: the framed items' subjects
                // (item.c foobar3), newest first; a stage subject created at
                // the stage's start; then the fighters' subjects in reverse
                // creation order.
                let stage_subject = match &mut state.stage {
                    SceneStage::Stadium(stage) => stage.screen.subject.as_mut(),
                    _ => None,
                };
                let items = state.items.iter_mut().rev().filter_map(|i| i.camera.as_mut());
                let fighters = state.fighters.iter_mut().rev().map(|f| &mut f.0.camera);
                let camera = &mut state.camera;
                let stage_camera = &state.assets.stage_camera;
                with_subjects(items.chain(stage_subject).chain(fighters), |subjects| {
                    camera.update_standard(subjects, stage_camera)
                });
                state
                    .quakes
                    .retire_loop(&state.camera, &mut state.effects.gobjs);
                let unzoomed = state.camera.zoom() == 1.0;
                for fighter in &mut state.fighters {
                    fighter.0.offscreen.camera_unzoomed = unzoomed;
                }
            }
            Callback::Quakes => state
                .quakes
                .animate(&mut state.camera, &mut state.effects.gobjs),
            Callback::ItemSpawner => self.spawn_random_item(world)?,
            Callback::Fighter { player, proc } => {
                if proc == FighterProc::Animation {
                    use crate::scene_fighter::with_fighter;
                    // ftColl_800764DC (800764DC), from Fighter_procAnim out of
                    // hitlag (fighter.c:1686): the repeated-hit victim is
                    // forgotten once it is out of hitstun (x221C_b6, retail
                    // 0x80076504: lbz 0x221C; extrwi. 1, 30) with its grace
                    // frames run out (x2098, 0x80076510). The bit, not the
                    // damage state's timer: a tech or a landing out of
                    // hitstun leaves a timer that no longer counts down.
                    let (victim, runs) = with_fighter!(&state.fighters[player], |f| (
                        f.combat.combo.victim,
                        !f.status.disabled && !f.in_hitlag()
                    ));
                    let expired = runs
                        && victim.is_some_and(|id| {
                            state.fighters.iter().any(|other| {
                                with_fighter!(other, |f| f.spawn_number == id
                                    && f.combat.combo.grace == 0
                                    && !f.status.in_hitstun)
                            })
                        });
                    if expired {
                        with_fighter!(&mut state.fighters[player], |f| f.combat.combo.victim =
                            None);
                    }
                }
                grab_pairs::constrain(state, player);
                if proc == FighterProc::Animation {
                    grab_pairs::offer_carrier_ground(state, player);
                    if state.offscreen_damage_by_camera_limits {
                        // BrawlOffscreenDamage.asm answers Fighter_8006A360's
                        // question (0x8006A880) from the fighter as its
                        // animation proc finds it.
                        let limits = &state.assets.stage_camera;
                        let f = &mut state.fighters[player].0;
                        f.offscreen.magnified = f.outside_camera_limits(limits);
                    }
                }
                if proc == FighterProc::Grab {
                    // Fighter_UnkProcessGrab_8006CA5C: the catch devices
                    // (ftColl_8007BA0C) before the fighter's own grab.
                    if !state.fighters[player].0.status.disabled {
                        crate::scene_stage::oldkongo::catch(state, player)?;
                    }
                    // Fighter_UnkProcessGrab_8006CA5C: ftColl_8007BA0C's
                    // catch devices come first.
                    crate::scene_stage::kongo::barrel_catch(state, player)?;
                    grab_pairs::select(state, player)?;
                }
                if proc == FighterProc::Accessories {
                    crate::scene_stage::kongo::barrel_accessory(state, player);
                }
                // Falcon Dive's accessory4 reads the held fighter's position;
                // so does the Inhale's pull from its animation callback
                // (ftCo_800BD19C).
                if matches!(
                    proc,
                    FighterProc::HitboxPositions | FighterProc::Animation
                ) {
                    grab_pairs::offer_partner_position(state, player);
                }
                if proc == FighterProc::Update {
                    grab_pairs::align(state, player)?;
                }
                if proc == FighterProc::Map {
                    grab_pairs::map_capture(state, player)?;
                }
                // ft_80082E3C reads the other fighters' ledges from the map
                // proc and from a tether article's climb (accessory2).
                if matches!(proc, FighterProc::Map | FighterProc::Accessories) {
                    offer_ledge_holders(state, player);
                }
                if proc == FighterProc::Accessories {
                    offer_nearest_fighter(state, player);
                }
                if proc == FighterProc::Animation {
                    let slot = &state.fighters[player].player;
                    let report = crate::scene_items::owner_report(
                        &state.items,
                        &state.assets.items,
                        slot.id,
                        slot.secondary,
                    );
                    crate::scene_fighter::with_fighter!(&mut state.fighters[player], |f| f
                        .core
                        .owned_article =
                        report);
                }
                if proc == FighterProc::Input {
                    // What controller-fix Gecko code reads this tick: the
                    // fighter's port (fp+0x618) setting, pad-queue bytes and
                    // 0.84's ring for the port.
                    let port = usize::from(state.fighters[player].player.id);
                    let hardware = melee_ft::input::HardwareInput {
                        fix: state.controller_fixes[port],
                        queue_x: self.pad_queue_x[port],
                        raw: self.consumed_sticks[port],
                        buffer: state.pad_buffers[port],
                        dween_previous_x: state.dween_previous_x[port],
                        partner_turn: None,
                    };
                    crate::scene_fighter::with_fighter!(&mut state.fighters[player], |f| f
                        .core
                        .input
                        .hardware =
                        hardware);
                }
                // ftpickupitem_800942A0 runs from input and animation callbacks.
                let offers_items = matches!(proc, FighterProc::Input | FighterProc::Animation);
                if offers_items {
                    let candidates =
                        crate::scene_items::pickup_candidates(&state.items, &state.assets.items);
                    crate::scene_fighter::with_fighter!(&mut state.fighters[player], |f| f
                        .core
                        .pickup_candidates
                        .offer(candidates));
                }
                // x2219_b5 before this proc, for the grab-pair hitlag link.
                let was_in_hitlag = grab_pairs::in_hitlag(state, player);
                if proc == FighterProc::ProcessHit {
                    grab_pairs::linked_hit(state, player)?;
                    // Player_80033BB8 (ftCo_800C318C's sleep timer): stock
                    // standings, ties share a rank.
                    use crate::scene_fighter::with_fighter;
                    let stocks = with_fighter!(&state.fighters[player], |f| f.player.stocks);
                    let rank = state
                        .fighters
                        .iter()
                        .filter(|f| {
                            with_fighter!(f, |f| !f.player.secondary && f.player.stocks > stocks)
                        })
                        .count() as u8;
                    with_fighter!(&mut state.fighters[player], |f| f.core.standing_rank = rank);
                }
                // The partner as this proc's callbacks will read it.
                partner_fighters::observe(state, player, proc);
                let assets = &state.assets;
                // Fighter_8006CB94: nothing while x221F_b3 or x2219_b1 is set.
                if proc == FighterProc::HitDetection
                    && !state.fighters[player].0.status.disabled
                    && !state.fighters[player].0.out_of_play()
                {
                    use crate::scene_fighter::with_fighter;
                    // Fighter_8006CB94 -> ftColl_800765E0: fresh damage logs.
                    with_fighter!(&mut state.fighters[player], |f| f
                        .core
                        .begin_hit_detection());
                    for other in 0..state.fighters.len() {
                        if player == other {
                            continue;
                        }
                        partner_fighters::take_partner_stale_table(
                            &mut state.fighters,
                            &assets.fighters,
                            other,
                        );
                        // grabber_unk1 / x1064 owner: the fighter throwing `other`.
                        let thrower =
                            state.fighters[other].commands.thrown_by.map(|spawn| {
                                let thrower = state
                                    .fighters
                                    .iter()
                                    .find(|f| f.spawn_number == spawn)
                                    .expect("the thrower");
                                melee_ft::fighter::damage::Thrower {
                                    player: thrower.player.id,
                                    team: thrower.player.team,
                                    position_x: thrower.physics.position.x,
                                }
                            });
                        let (victim, attacker) = if player < other {
                            let (left, right) = state.fighters.split_at_mut(other);
                            (&mut left[player], &mut right[0])
                        } else {
                            let (left, right) = state.fighters.split_at_mut(player);
                            (&mut right[0], &mut left[other])
                        };
                        with_fighter!(victim, |v| with_fighter!(attacker, |a| {
                            melee_ft::fighter::damage::detect_hit(
                                v,
                                a,
                                &assets.fighters[player],
                                player < other,
                                thrower,
                                &mut state.rng,
                            )
                        }));
                    }
                    state.items.resolve_group_histories();
                    for index in 0..state.items.len() {
                        let (common, item) = state.items.common_and_item_mut(index);
                        let owner = crate::scene_items::owner_index(
                            &state.fighters,
                            item.owner,
                            item.owner_secondary,
                        );
                        let mark = melee_it::ItemPool::group_victim_mark(item);
                        // ftcoll.c:2032-2036: x20_team_id, the owner's team.
                        let item_team = owner.and_then(|index| state.fighters[index].player.team);
                        let spared = state.fighters[player].player.spares(item_team);
                        let hit = with_fighter!(&mut state.fighters[player], |f| {
                            f.detect_item_hit(item, &assets.fighters[player], common, spared)
                        });
                        let item_id = item.id;
                        if let Some(mark) = &mark {
                            // it_8026FAC4 / it_8026FC00: the group shares it.
                            state.items.share_group_victims(item_id, mark);
                        }
                        let item = state.items.common_and_item_mut(index).1;
                        if let Some(contact) = hit {
                            if contact.logged_damage {
                                // plStale_UpdateStaleMovesFromItem: not when
                                // the item strikes its own owner.
                                if let (Some(owner), Some(attack)) =
                                    (owner.filter(|&o| o != player), item.stale_source)
                                {
                                    partner_fighters::take_partner_stale_table(
                                        &mut state.fighters,
                                        &assets.fighters,
                                        owner,
                                    );
                                    state.fighters[owner]
                                        .core
                                        .credit_item_hit(attack, &assets.fighters[owner]);
                                }
                                // ftColl_80078998 -> ftColl_8007646C: the owner's
                                // repeated-hit count takes the item's attack id.
                                if let Some(owner) = owner.filter(|&o| o != player) {
                                    let victim = state.fighters[player].spawn_number;
                                    let attack = item.stale_source.map(|a| a.move_id);
                                    let combo = &assets.fighters[owner].combo;
                                    state.fighters[owner]
                                        .combat
                                        .combo
                                        .record(victim, attack, combo);
                                }
                            }
                            // ftColl_80077C60 records contact; Item_8026A294 runs the callback at link 14.
                            item.record_damage_dealt(contact.damage);
                        }
                    }
                    // ftColl_8007AB48 / ftColl_8007AB80: resolve both logs.
                    with_fighter!(&mut state.fighters[player], |f| {
                        f.core.resolve_hit_logs(&assets.fighters[player])
                    });
                }
                // Fighter_8006ABA0: the CPU proc runs ftCo_800B3900 for a
                // fighter whose input the CPU supplies (Nana).
                let cpu_driven = proc == FighterProc::CpuGate
                    && continuation == Continuation::Invoke
                    && state.fighters[player].0.cpu_driven();
                if cpu_driven {
                    crate::cpu::think(state, player);
                } else {
                    crate::scene_fighter::with_fighter!(&mut state.fighters[player], |f| {
                        let f: &mut melee_ft::fighter::Fighter = f;
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
                            // Each fighter samples the device after integrating
                            // its velocity (ftColl_GetWindOffsetVec).
                            let wind = match &state.stage {
                                SceneStage::Pupupu(stage) => stage.wind(),
                                SceneStage::Garden(stage) => stage.water().map_or(
                                    melee_gr::wind::Wind::CALM,
                                    |water| {
                                        melee_gr::wind::Wind::water(
                                            water,
                                            f.camera.target_extents.radius,
                                        )
                                    },
                                ),
                                _ => melee_gr::wind::Wind::CALM,
                            };
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
                                &mut self.radial_forces,
                                wind,
                            )
                        }
                    })?;
                    partner_fighters::act(state, player)?;
                    if proc == FighterProc::Input {
                        crate::scene_stage::kongo::barrel_fire_request(state, player);
                    }
                }
                if proc == FighterProc::Map {
                    // fn_800DD568 / fn_800DD5EC: a thrower's landing resumes
                    // the fighter it holds within its collision callback.
                    grab_pairs::thrown_request(state, player);
                    if let Some(snapshots) = self.after_map.as_mut() {
                        snapshots.push((
                            player,
                            crate::diagnostics::snapshot(state, self.frame),
                        ));
                    }
                    if let Some(values) = self.after_map_values.as_mut() {
                        values.push((
                            player,
                            crate::diagnostics::FighterValues::of(&state.fighters[player]),
                        ));
                    }
                }
                // Fighter_8006C80C's accessory4 may hand the player to its
                // partner, whose own s_link 9 proc still runs this tick when
                // it follows in the fighter list.
                if proc == FighterProc::HitboxPositions {
                    transformation::perform(state, player)?;
                }
                if offers_items {
                    crate::scene_fighter::with_fighter!(&mut state.fighters[player], |f| f
                        .core
                        .pickup_candidates
                        .withdraw());
                }
                if matches!(
                    proc,
                    FighterProc::HitboxPositions | FighterProc::Animation
                ) {
                    grab_pairs::withdraw_partner_position(state, player);
                }
                if proc == FighterProc::ProcessHit {
                    grab_pairs::release_captor(state, player)?;
                    grab_pairs::hold_partner_hitlag(state, player, was_in_hitlag)?;
                }
                if proc == FighterProc::Status {
                    grab_pairs::release_partner_hitlag(state, player, was_in_hitlag)?;
                }
                if matches!(proc, FighterProc::Map | FighterProc::Accessories) {
                    crate::scene_fighter::with_fighter!(&mut state.fighters[player], |f| f
                        .core
                        .ledge_holders
                        .withdraw());
                }
                if proc == FighterProc::Input {
                    let port = usize::from(state.fighters[player].player.id);
                    let hardware = &state.fighters[player].0.input.hardware;
                    state.pad_buffers[port] = hardware.buffer;
                    state.dween_previous_x[port] = hardware.dween_previous_x;
                    partner_turn(state, player);
                    // BarrelWait's IASA calls Ground_801C4DD0 at once: the
                    // barrel's own proc follows at s_link 4.
                    crate::scene_stage::oldkongo::relay_fire_request(state, player);
                }
                // Camera_RequestQuake runs inside the proc that calls it,
                // whichever fighter asked: a thrown fighter's release hit
                // lands in its thrower's animation proc, ahead of this
                // tick's quake procs (s_link 1, p_link 18), which then
                // step the new model at once.
                for fighter in &mut state.fighters {
                    if let Some(kind) = fighter.0.quake_request.take() {
                        state.quakes.request_from_fighter(&mut state.camera, kind);
                    }
                }
                crate::quake::request_effect_quakes(state);
                if let Some(link) = state.fighters[player].0.released_link.take() {
                    let partner = match link {
                        melee_ft::fighter::grab::GrabLink::Holding { victim, .. } => victim,
                        melee_ft::fighter::grab::GrabLink::Captured { captor } => captor,
                    };
                    let index = state
                        .fighters
                        .iter()
                        .position(|f| f.0.spawn_number == partner)
                        .expect("linked partner");
                    grab_pairs::release_from_dead_partner(state, player, index)?;
                }
                partner_fighters::share_fall(state, player);
                if proc == FighterProc::Animation {
                    partner_fighters::complete_death(state, player)?;
                }
                if proc == FighterProc::Input {
                    if let Some(victim) = grab_pairs::throw_input(state, player)? {
                        // ftCo_800DD4B0 changes the captor's motion, then
                        // the victim's (ftCo_800DE3FC); each
                        // Fighter_ChangeMotionState flushes that fighter's
                        // efAsync queue (fighter.c:951) inside the captor's
                        // input proc, ahead of the next fighter's procs.
                        //
                        // ftCo_800DD398 also runs the captor's new script
                        // there (ftAnim_8006EBA4, retail 0x800DD3FC), before
                        // the victim's entry (0x800DD418): the throw's voice
                        // draw (ft_800889F4, retail 0x80088A18) and its
                        // graphics offsets come before the stage's procs
                        // and the next fighter's, not at the captor's next
                        // proc boundary.
                        for member in [player, victim] {
                            let f = &mut state.fighters[member].0;
                            resolve_issued_sounds(
                                f,
                                member,
                                &state.assets,
                                &mut state.effects,
                                &mut state.particles,
                                &mut state.rng,
                                true,
                            )?;
                            f.resolve_terrain_footsteps(&mut state.rng);
                            resolve_issued_graphics(
                                f,
                                member,
                                &state.assets,
                                &mut state.effects,
                                &mut state.particles,
                                &mut state.rng,
                            )?;
                            state.effects.flush::<RetailTrig>(
                                melee_ef::EffectTiming::Immediate,
                                member,
                                &mut f.core,
                                &state.assets.common_particle_bank,
                                &state.assets.particle_bank,
                                &mut state.particles,
                                &mut state.rng,
                            )?;
                        }
                    }
                }
                // A cargo carrier's Anim, IASA and Coll callbacks change the
                // carried fighter's motion too; the shouldered fighter's
                // Anim may end the carry for both. Each
                // Fighter_ChangeMotionState flushes that fighter's efAsync
                // queue then (fighter.c:951).
                if matches!(
                    proc,
                    FighterProc::Animation | FighterProc::Input | FighterProc::Map
                ) {
                    let mut moved = grab_pairs::cargo(state, player)?;
                    if proc == FighterProc::Animation {
                        if let Some(carrier) = grab_pairs::shoulder_escape(state, player)? {
                            moved = Some(carrier);
                        }
                    }
                    if let Some(other) = moved {
                        for member in [player, other] {
                            crate::scene_fighter::with_fighter!(
                                &mut state.fighters[member],
                                |f| state.effects.flush::<RetailTrig>(
                                    melee_ef::EffectTiming::Immediate,
                                    member,
                                    &mut f.core,
                                    &state.assets.common_particle_bank,
                                    &state.assets.particle_bank,
                                    &mut state.particles,
                                    &mut state.rng,
                                )
                            )?;
                        }
                    }
                }
                if matches!(
                    proc,
                    FighterProc::Animation | FighterProc::Input | FighterProc::Map
                ) {
                    // The Koopa Klaw: the captor's callback changes its
                    // victim's motion (ftCo_800BC9C8, ftCo_800BCDE0,
                    // ftCo_800BCE64), and the victim's spent
                    // hold timer both fighters' (ftCo_800DA698,
                    // ftCo_CaptureCut_Enter). Each Fighter_ChangeMotionState
                    // flushes that fighter's efAsync queue then.
                    let mut members = [None; 2];
                    if let Some(victim) = grab_pairs::koopa_captor_request(state, player)? {
                        members = [Some(player), Some(victim)];
                    }
                    // A grounded hold losing its floor (fn_800DA004 /
                    // fn_800DA440 / fn_800DD684): both fall, in the order
                    // the callback changes them.
                    if proc == FighterProc::Map {
                        if let Some(changed) = grab_pairs::hold_floor_lost(state, player)? {
                            members = changed;
                        }
                    }
                    if proc == FighterProc::Animation {
                        if let Some(captor) = grab_pairs::koopa_escape(state, player)? {
                            members = [Some(captor), Some(player)];
                        }
                    }
                    for member in members.into_iter().flatten() {
                        crate::scene_fighter::with_fighter!(&mut state.fighters[member], |f| {
                            state.effects.flush::<RetailTrig>(
                                melee_ef::EffectTiming::Immediate,
                                member,
                                &mut f.core,
                                &state.assets.common_particle_bank,
                                &state.assets.particle_bank,
                                &mut state.particles,
                                &mut state.rng,
                            )
                        })?;
                    }
                }
                if matches!(proc, FighterProc::Animation | FighterProc::Input) {
                    // The Inhale: Kirby's animation callback changes the
                    // motion of the fighter in his mouth (ftCo_800BD620,
                    // ftCo_800BDB58, ftCo_800BE000), and that fighter's IASA
                    // Kirby's (ftKb_SpecialS_800F6BB0, ftCo_800DA698). Each
                    // Fighter_ChangeMotionState flushes that fighter's
                    // efAsync queue then.
                    let mut members = [None; 2];
                    if proc == FighterProc::Animation {
                        if let Some(victim) = grab_pairs::mouth_request(state, player)? {
                            members = [Some(victim), None];
                        }
                    } else if let Some((captor, changes)) = grab_pairs::mouth_hold(state, player)? {
                        members = [
                            changes.victim.then_some(player),
                            changes.captor.then_some(captor),
                        ];
                    }
                    for member in members.into_iter().flatten() {
                        crate::scene_fighter::with_fighter!(&mut state.fighters[member], |f| {
                            state.effects.flush::<RetailTrig>(
                                melee_ef::EffectTiming::Immediate,
                                member,
                                &mut f.core,
                                &state.assets.common_particle_bank,
                                &state.assets.particle_bank,
                                &mut state.particles,
                                &mut state.rng,
                            )
                        })?;
                    }
                }
                if proc == FighterProc::ProcessHit {
                    credit_phantom_source(state, player);
                }
                if proc == FighterProc::Accessories {
                    // BarrelWait's accessory1 (ftCo_8009EC44).
                    crate::scene_stage::oldkongo::ride(state, player);
                    // ft_80082E3C: a tether's climb asks who holds the ledge.
                    offer_ledge_holders(state, player);
                    crate::scene_fighter::with_fighter!(&mut state.fighters[player], |f| f
                        .character_proc_accessories(
                            &state.assets.fighters[player],
                            &mut state.map,
                            &mut state.rng
                        ))
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                    {
                        // The article's step may change the fighter's
                        // motion (a tether's catch, hop or let-go:
                        // ftCo_8009B390's ftAnim_8006EBA4 at 0x8009B3F0).
                        // The new script's frame-0 graphics commands reach
                        // efAsync_Spawn inside this s_link 8 proc
                        // (Fighter_CallAcessoryCallbacks_8006C624), so they
                        // queue (efAsync_Spawn, 0x80067948: s_link < 9) and the link 9
                        // flush creates them newest first.
                        let f = &mut state.fighters[player].0;
                        resolve_issued_sounds(
                            f,
                            player,
                            &state.assets,
                            &mut state.effects,
                            &mut state.particles,
                            &mut state.rng,
                            true,
                        )?;
                        f.resolve_terrain_footsteps(&mut state.rng);
                        resolve_issued_graphics(
                            f,
                            player,
                            &state.assets,
                            &mut state.effects,
                            &mut state.particles,
                            &mut state.rng,
                        )?;
                    }
                    crate::scene_fighter::with_fighter!(&mut state.fighters[player], |f| {
                        f.core.ledge_holders.withdraw();
                        // efSync_Spawn from the article's step (a tether
                        // meeting a wall) spawns at once.
                        state.effects.flush::<RetailTrig>(
                            melee_ef::EffectTiming::Immediate,
                            player,
                            &mut f.core,
                            &state.assets.common_particle_bank,
                            &state.assets.particle_bank,
                            &mut state.particles,
                            &mut state.rng,
                        )
                    })?;
                    if let Some(victim) = grab_pairs::accessory(state, player)? {
                        // ftCo_800DC920 changes the captor, then the victim;
                        // each Fighter_ChangeMotionState flushes efAsync then.
                        for member in [player, victim] {
                            crate::scene_fighter::with_fighter!(
                                &mut state.fighters[member],
                                |f| state.effects.flush::<RetailTrig>(
                                    melee_ef::EffectTiming::Immediate,
                                    member,
                                    &mut f.core,
                                    &state.assets.common_particle_bank,
                                    &state.assets.particle_bank,
                                    &mut state.particles,
                                    &mut state.rng,
                                )
                            )?;
                        }
                    }
                    // ftCo_800BD39C (accessory1 on the way into Kirby's mouth).
                    grab_pairs::mouth_pull(state, player);
                    crate::scene_fighter::with_fighter!(&mut state.fighters[player], |f| {
                        // fn_800D54A4 / fn_800D55B4 (accessory1 while reviving).
                        if f.reviving() {
                            f.match_partner_height();
                        }
                        f.update_revival_platform();
                        // Fighter_CallAcessoryCallbacks_8006C624: hitlag
                        // (x2219_b5) runs accessory3 instead of accessory1.
                        // accessory1 ftCo_800DE508 / ftCo_800DB464 belongs to a
                        // victim; a constrained captor follows with accessory4.
                        if f.combat.thrown_pose.is_some()
                            && !f.in_hitlag()
                            && matches!(
                                f.combat.grab,
                                Some(melee_ft::fighter::grab::GrabLink::Captured { .. })
                            )
                        {
                            f.thrown_accessory(&state.assets.fighters[player]);
                        }
                    });
                }
                if proc == FighterProc::Animation {
                    grab_pairs::swallow(state, player)?;
                    grab_pairs::escape(state, player)?;
                    grab_pairs::sync_wait(state, player)?;
                    grab_pairs::release(state, player)?;
                    grab_pairs::special_throw_release(state, player)?;
                    // ftCo_800DD724's freeze follows its release
                    // (ftCo_Throw.c:269-293).
                    grab_pairs::thrown_request(state, player);
                    if grab_pairs::thrown_escape(state, player)? {
                        // fn_800DC070 / ftCo_CaptureCut_Enter: the thrown
                        // fighter's Fighter_ChangeMotionState flushes its
                        // efAsync queue (fighter.c:951).
                        crate::scene_fighter::with_fighter!(&mut state.fighters[player], |f| {
                            state.effects.flush::<RetailTrig>(
                                melee_ef::EffectTiming::Immediate,
                                player,
                                &mut f.core,
                                &state.assets.common_particle_bank,
                                &state.assets.particle_bank,
                                &mut state.particles,
                                &mut state.rng,
                            )
                        })?;
                    }
                    let assets = &state.assets;
                    use crate::scene_fighter::with_fighter;
                    let mut bodies = [melee_coll::overlap::OverlapBody::default(); MAX_FIGHTERS];
                    for (slot, fighter) in state.fighters.iter().enumerate() {
                        bodies[slot] =
                            with_fighter!(fighter, |f| f.overlap_body(&assets.fighters[slot]));
                    }
                    let owner = crate::scene_fighter::with_fighter!(&state.fighters[player], |f| f
                        .player
                        .id);
                    let owner = state
                        .fighters
                        .iter()
                        .position(|f| f.0.player.id == owner && !f.0.player.secondary)
                        .expect("the player's own fighter");
                    let nudge = melee_coll::overlap::nudge(
                        player,
                        owner,
                        &bodies[..state.fighters.len()],
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
                                state.assets.stage_key_bank(event.bank),
                                request,
                                &mut state.rng,
                                &mut self.particle_draws,
                            )?;
                        }
                        crate::scene_stage::publish_joint_matrices(
                            animation,
                            map,
                            &mut state.particles,
                            &mut state.effects.events,
                        );
                    }
                    if let Some(animation) = state.stage_animations.get_mut(&map) {
                        if let Some(script) = animation.overlay_script {
                            animation
                                .overlay
                                .tick(&state.assets.stage_desc.material_scripts[script]);
                        }
                    }
                    state.map.finish_ground_animation();
                }
                0x801C0C2C => {
                    // Player_LoadPlayerCoords: the slot's mirror, written at
                    // s_link 22 (Fighter_8006DA4C) and frozen while disabled.
                    let players = std::array::from_fn(|slot| {
                        state.fighters.get(slot).map(|fighter| {
                            crate::scene_fighter::with_fighter!(fighter, |f| f.player_position)
                        })
                    });
                    if let Some(position) = state
                        .clock
                        .sudden_death
                        .then(|| {
                            state.bomb_rain.drop_position(
                                state.clock.frame_count,
                                state.assets.stage_desc.kind,
                                &players,
                                state.assets.stage_camera.blast_top(),
                                &state.assets.drop_markers,
                                &mut state.rng,
                            )
                        })
                        .flatten()
                    {
                        // it_8026BE84 (BobOmbRain kind 6) -> it_8027D670: the
                        // facing is drawn before the item exists.
                        let facing = crate::scene_items::facing_toward_fighters(
                            position,
                            &mut state.fighters,
                            &mut state.rng,
                        );
                        crate::scene_items::spawn_rain_bomb(
                            &mut state.items,
                            &mut state.effects.gobjs,
                            &state.assets.items,
                            &mut state.map,
                            world,
                            &mut self.item_objects,
                            position,
                            facing,
                            &mut state.rng,
                        );
                        self.drain_item_events(false, world)?;
                    }
                }
                0x801C461C | 0x801CADBC | 0x801C1D38 => {}
                _ => {
                    let map_id = map.expect("stage callback map");
                    // grStory_801E3334: the Shy Guy spawner (grStory_801E3418)
                    // runs before the map's collision update.
                    if address == STORY_SHY_GUY_PROC
                        && continuation != Continuation::GroundCollision
                    {
                        let SceneStage::Story(story) = &mut state.stage else {
                            unreachable!("grStory_801E3334 outside Yoshi's Story")
                        };
                        let live = state.items.iter().any(|item| {
                            item.kind == melee_types::ItemKind::Heiho && !item.destroyed
                        });
                        story.tick_shy_guys(&mut state.rng, live, |rng, spawn| {
                            // it_8027B5B0 -> it_8026B684: the facing is drawn
                            // before the item exists.
                            let facing = crate::scene_items::facing_toward_fighters(
                                spawn.position,
                                &mut state.fighters,
                                rng,
                            );
                            crate::scene_items::spawn_shy_guy(
                                &mut state.items,
                                &mut state.effects.gobjs,
                                &state.assets.items,
                                &mut state.map,
                                world,
                                &mut self.item_objects,
                                spawn,
                                facing,
                                rng,
                            );
                        });
                    }
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
                    // lb_800115F4 (dynamics fields' decay and expiry) ends
                    // grLast_8021AAB0, grBattle_GObj6_Callback2 (0x8021A174),
                    // grStory_801E3334 and grIzumi_801CC0D4, and is all of
                    // grOldPupupu_80210BC0; it also ends grOldYoshi_8020EA64.
                    if address == 0x8021_AAB0
                        || address == 0x8021_A174
                        || address == 0x8021_0BC0
                        || address == STORY_SHY_GUY_PROC
                        || address == 0x801C_C0D4
                        || address == melee_gr::oldyoshi::procs::DYNAMICS_PROC
                        || address == melee_gr::oldkongo::procs::DYNAMICS_PROC
                        || address == 0x8020_2E48
                        || address == 0x801D_557C
                    {
                        self.radial_forces.tick();
                        // lb_80011ABC reads the state this tick left.
                        let wind = self.radial_forces.wind_state();
                        for fighter in &mut state.fighters {
                            crate::scene_fighter::with_fighter!(fighter, |f| f.stage_wind = wind);
                        }
                    }
                    if continuation == Continuation::GroundCollision {
                        // grLast's controller already advanced; finish only its collision tail.
                        return Ok(());
                    }
                    if matches!(state.stage, SceneStage::FinalDestination(_)) {
                        crate::scene_stage::last::run_proc(
                            state,
                            map_id,
                            &mut self.particle_draws,
                            world,
                            &mut self.stage_objects,
                        )?;
                    } else if matches!(state.stage, SceneStage::Battlefield(_)) {
                        crate::scene_stage::battle::run_proc(
                            state,
                            map_id,
                            &mut self.particle_draws,
                            world,
                            &mut self.stage_objects,
                        )?;
                    } else if matches!(state.stage, SceneStage::Izumi(_)) {
                        crate::scene_stage::izumi::run_proc(state, map_id)?;
                    } else if matches!(state.stage, SceneStage::OldKongo(_)) {
                        if let Some(shot) = crate::scene_stage::oldkongo::run_proc(state, map_id)? {
                            crate::scene_stage::oldkongo::fire(state, shot)?;
                        }
                    } else if matches!(state.stage, SceneStage::OldYoshi(_)) {
                        crate::scene_stage::oldyoshi::run_proc(
                            state,
                            map_id,
                            &mut self.particle_draws,
                        )?;
                    } else if matches!(state.stage, SceneStage::Kongo(_)) {
                        crate::scene_stage::kongo::run_proc(
                            state,
                            map_id,
                            &mut self.particle_draws,
                            world,
                            &mut self.stage_objects,
                        )?;
                    } else if matches!(state.stage, SceneStage::Garden(_)) {
                        crate::scene_stage::garden::run_proc(
                            state,
                            map_id,
                            &mut self.particle_draws,
                        )?;
                    } else if matches!(state.stage, SceneStage::Stadium(_)) {
                        crate::scene_stage::stadium::run_proc(
                            state,
                            map_id,
                            &mut self.particle_draws,
                            &mut self.radial_forces,
                            world,
                            &mut self.stage_objects,
                            &self.external,
                            &mut self.consumed,
                        )?;
                    } else if matches!(state.stage, SceneStage::Pupupu(_)) {
                        crate::scene_stage::pupupu::run_proc(
                            state,
                            map_id,
                            &mut self.particle_draws,
                            &mut self.radial_forces,
                        )?;
                    } else if address != STORY_SHY_GUY_PROC
                        && state.stage.run_stage_proc(map_id, &mut state.rng)?
                    {
                        // grLib_801C97DC (0x801C97DC): detached puff at the
                        // current world position of archive descendant 1.
                        let matrix = state
                            .stage_animations
                            .get_mut(&map_id)
                            .unwrap()
                            .joint_matrix(1);
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
                // The HUD follows the player's own fighter (Player_GetEntity).
                let own = state.player_fighter_index(player);
                let percent = with_fighter!(&state.fighters[own], |f| f.physics.percent);
                let dead = crate::scene_fighter::with_fighter!(&state.fighters[own], |f| matches!(
                    &f.state_data,
                    melee_ft::fighter::MotionData::Life(life) if life.stock_lost()
                ));
                self.interface[player].set_dead(dead, &mut state.rng);
                self.interface[player].tick(percent, &mut state.rng);
                if let Some(display) = &mut state.stock_displays[player] {
                    let stocks = with_fighter!(&state.fighters[own], |f| f.player.stocks);
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
            Callback::Effects => {
                // efLib_Cb_AccumOffset_FromParams reads followed items'
                // root JObjs (lb_8000B1CC).
                let items = &state.items;
                state.effects.update_item_roots(|id| {
                    items
                        .iter()
                        .find(|i| i.id == id)
                        .map(|i| i.root_translation)
                });
                // A model on a posed article's bone follows it.
                let mut index = 0;
                loop {
                    let Some((id, bone)) = state.effects.item_bone_models().nth(index) else {
                        break;
                    };
                    index += 1;
                    let Some(item) = state.items.iter().find(|i| i.id == id) else {
                        continue;
                    };
                    let matrix = article_bone_matrix(
                        &mut state.fighters,
                        &mut state.article_poses,
                        &state.assets.items.article_skeletons,
                        &state.assets.items,
                        item,
                        bone,
                        self.frame,
                    );
                    state.effects.place_item_bone_models(id, bone, &matrix);
                }
                state
                    .effects
                    .tick_with_pause::<melee_ft::fighter::RetailTrig>(
                        self.match_finished,
                        &mut crate::scene_fighter::FighterJointSource(&mut state.fighters[..]),
                        &state.assets.common_particle_bank,
                        &mut state.particles,
                        &mut state.rng,
                    )?
            }
            Callback::ParticlesMain => {
                // hsd_8039D214's HSD_JObjSetupMatrix leaves the followed
                // bones (and the display pass every joint) clean.
                for item in state.items.iter_mut() {
                    item.joints_dirty = false;
                }
                // Generators on a posed article's bones read their matrices.
                let mut index = 0;
                loop {
                    let Some((id, bone)) = state.effects.followed_item_bones().nth(index) else {
                        break;
                    };
                    let followed = state.effects.followed_item_bones().count();
                    let Some(item) = state.items.iter().find(|i| i.id == id) else {
                        // The item is gone: nothing reads its bones.
                        state.effects.forget_item_bone(id, bone);
                        continue;
                    };
                    let matrix = article_bone_matrix(
                        &mut state.fighters,
                        &mut state.article_poses,
                        &state.assets.items.article_skeletons,
                        &state.assets.items,
                        item,
                        bone,
                        self.frame,
                    );
                    state
                        .effects
                        .update_item_bone(id, bone, matrix, &mut state.particles);
                    // A bone nothing follows any more left the list.
                    if state.effects.followed_item_bones().count() == followed {
                        index += 1;
                    }
                }
                // Generators on an item's JObj read its current matrix.
                for item in state.items.iter() {
                    if state.effects.follows_item(item.id) {
                        let mut matrix = hsd_types::Mtx::default();
                        hsd_anim::mtx::hsd_mtx_srt(
                            &mut matrix,
                            &item.model_scale,
                            &item.rotation,
                            &item.position,
                            None,
                        );
                        let matrix = (<crate::scene_items::SceneItems as melee_it::ItemDispatch>::logic(
                            item.kind,
                        )
                        .effect_joint_matrix)(
                            item, state.assets.items.get(item.kind), matrix
                        );
                        state
                            .effects
                            .update_item_joint(item.id, matrix, &mut state.particles);
                    }
                }
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
        for (slot, fighter) in state.fighters.iter_mut().enumerate() {
            crate::scene_fighter::with_fighter!(fighter, |f| {
                f.apply_pending_pickup_pose(&state.assets.fighters[slot]);
                while !f.item_requests.is_empty() {
                    let mut request = f.item_requests.remove(0);
                    match &mut request {
                        melee_it::ItemRequest::Spawn(spawn)
                        | melee_it::ItemRequest::SpawnInGroup(spawn)
                        | melee_it::ItemRequest::SpawnHeld(spawn)
                        | melee_it::ItemRequest::SpawnInHand { spawn, .. }
                        | melee_it::ItemRequest::SpawnChain { spawn, .. }
                        | melee_it::ItemRequest::SpawnLaser { spawn, .. } => {
                            // A spawner that starts a new attack instance in
                            // the same call (ftKp_SpecialLw_80134ACC) names
                            // the instance its article took.
                            if spawn.stale_source.is_none() {
                                spawn.stale_source = f.combat.stale.attack()
                            }
                        }
                        melee_it::ItemRequest::Control { .. }
                        | melee_it::ItemRequest::NewHitGroup
                        | melee_it::ItemRequest::PickUp { .. }
                        | melee_it::ItemRequest::Throw { .. }
                        | melee_it::ItemRequest::Drop { .. }
                        | melee_it::ItemRequest::Destroy { .. }
                        | melee_it::ItemRequest::Stow { .. }
                        | melee_it::ItemRequest::DropArticle { .. }
                        | melee_it::ItemRequest::Launch { .. } => {}
                    }
                    let reads_owner = match &request {
                        melee_it::ItemRequest::SpawnHeld(_) => true,
                        melee_it::ItemRequest::SpawnInHand { spawn, .. } => {
                            <crate::scene_items::SceneItems as melee_it::ItemDispatch>::logic(
                                spawn.kind,
                            )
                            .pickup_reads_owner
                        }
                        _ => false,
                    };
                    let owner =
                        reads_owner.then(|| f.item_owner(&state.assets.fighters[slot]));
                    let (hold, catch_item) = match request {
                        melee_it::ItemRequest::SpawnInHand {
                            hold, catch_item, ..
                        } => (hold, catch_item),
                        _ => (false, false),
                    };
                    let spawned = crate::scene_items::request(
                        &mut state.items,
                        &mut state.effects.gobjs,
                        &state.assets.items,
                        &mut state.map,
                        world,
                        &mut self.item_objects,
                        request,
                        crate::scene_items::RequestOwner {
                            slot: Some(f.player.id),
                            secondary: f.player.secondary,
                            held_item: owner.as_ref(),
                            after_hitbox_refresh: row.s_link > 11,
                            efasync_immediate: row.s_link >= 9,
                            stale_multiplier: f
                                .combat
                                .stale
                                .multiplier(&state.assets.fighters[slot].stale_weights),
                            bounds: Some(&item_bounds(&state.assets)),
                        },
                        &mut state.rng,
                    );
                    // The requests remove articles in retail's order.
                    state.items.note_destroyed();
                    // Item_8026A8EC's DestroyItemInline: a destroyed held
                    // item lets go of the hand (ftCommon_8007E6DC).
                    if let Some(held) = f.held_item {
                        if !state
                            .items
                            .iter()
                            .any(|i| i.id == held.item && !i.destroyed)
                        {
                            f.core
                                .release_held_item(held.item, &state.assets.fighters[slot]);
                        }
                    }
                    if let Some(id) = spawned.filter(|_| hold) {
                        let kind = state.items.iter().find(|i| i.id == id).unwrap().kind;
                        let item = state.assets.items.get(kind);
                        f.hold_spawned_item(
                            melee_ft::fighter::item_pickup::HeldItem {
                                item: id,
                                kind,
                                heavy: item.heavy,
                                hand_hold_kind: item.hand_hold_kind,
                                use_kind: item.use_kind,
                                damage_multiplier: item.collision_damage_multiplier,
                            },
                            catch_item,
                            &state.assets.fighters[slot],
                        );
                    }
                }
            });
        }
        let ended_spawner = self.deliver_item_links(row.s_link, world)?;
        let state = &mut self.state;
        // A new article's own efSync_Spawn (it_802BE2E8, it_802BD248's
        // it_80272C08) belongs to its spawner's proc, as does an article
        // the proc destroyed at once (it_802B5974's efLib_DestroyAll).
        if state.items.iter().any(|item| {
            item.events.iter().any(|e| {
                matches!(
                    e,
                    melee_it::ItemEvent::OwnEffect { .. }
                        | melee_it::ItemEvent::FollowingEffect { .. }
                        | melee_it::ItemEvent::Effect { .. }
                        | melee_it::ItemEvent::JointParticle { .. }
                        | melee_it::ItemEvent::DestroyEffects
                        // efAsync_Spawn from a fighter's link 9 proc
                        // (efasync.c:1458, retail 800679B0's s_link test)
                        // dispatches before the next fighter's link 9 flush.
                        | melee_it::ItemEvent::ChildEffect { .. }
                )
            })
        }) {
            self.drain_item_events(false, world)?;
        }
        if let Some((id, held)) = spawner_teardown {
            if let Some(item) = self.state.items.get_mut(id) {
                for event in held.into_iter().flatten() {
                    item.events.push(event);
                }
            }
            self.drain_item_events(false, world)?;
        }
        let state = &mut self.state;
        // The spawner's own end (Item_8026A8EC's efLib_DestroyAll) follows
        // the article its proc created: the walk leaves the generator
        // insertion cursor at the list's tail, past the new generators.
        if let Some(spawner) = ended_spawner {
            state
                .effects
                .destroy_item_models(spawner, &mut state.particles);
            state.effects.expire_item_joint(spawner, &mut state.particles);
        }
        // Item SFX use the same headless request sink as ft_PlaySFX.
        for item in state.items.iter_mut() {
            while !item.sound_requests.is_empty() {
                let id = item.sound_requests.remove(0);
                if let Some(fighter) = crate::scene_items::owner_index(
                    &state.fighters,
                    item.owner,
                    item.owner_secondary,
                )
                .map(|index| &mut state.fighters[index])
                {
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
        // Where the proc's flush marked retail's Item_8026A8EC calls, the
        // frees below go back to that point and what the flush did after
        // it follows them.
        let spliced = state.effects.gobjs.marked();
        if spliced && state.items.iter().any(|item| item.destroyed) {
            state.effects.gobjs.rewind();
        } else {
            state.effects.gobjs.release();
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
        // The kinds' Destroyed callbacks (e.g. itPeachParasol_Logic60_Destroyed)
        // release their owner's references. Each item goes in the order
        // Item_8026A8EC was called: its models, then its own GObjs.
        while let Some(destroyed) = state.items.next_destroyed() {
            let item = state
                .items
                .iter()
                .find(|item| item.id == destroyed)
                .expect("a destroyed item");
            crate::scene_items::article_destroyed(&mut state.fighters, item);
            // Generators kept on a torn-down article's bones. JObjReleaseChild
            // leaves each bone without a parent: a dirty one is rebuilt from
            // its local SRT alone at its next setup; a clean one (no proc
            // touched the model since the particle pass set it up, as when
            // its owner's KO tears it down at s_link 4) keeps its last world
            // matrix.
            let mut index = 0;
            loop {
                let Some((id, bone)) = state.effects.followed_item_bones().nth(index) else {
                    break;
                };
                let followed = state.effects.followed_item_bones().count();
                if id == item.id && item.joints_dirty {
                    let matrix = state
                        .article_poses
                        .detached_bone_matrix(id, bone)
                        .or_else(|| {
                            crate::scene_items::free_item_detached_bone_matrix(
                                &state.assets.items,
                                item,
                                bone,
                            )
                        });
                    if let Some(matrix) = matrix {
                        state
                            .effects
                            .update_item_bone(id, bone, matrix, &mut state.particles);
                    }
                }
                // A bone nothing follows any more left the list.
                if state.effects.followed_item_bones().count() == followed {
                    index += 1;
                }
            }
            // Item_8026A8EC frees the JObj its generators follow, unless its
            // proc already did (ItemEvent::DestroyEffects).
            if !item.effects_destroyed {
                state
                    .effects
                    .destroy_item_models(item.id, &mut state.particles);
                state
                    .effects
                    .expire_item_joint(item.id, &mut state.particles);
            }
            state.effects.forget_item(item.id);
            crate::scene_items::cleanup(
                &mut state.items,
                &mut state.effects.gobjs,
                world,
                &mut self.item_objects,
                destroyed,
            );
        }
        if spliced {
            let renamed = state.effects.gobjs.replay();
            if !renamed.is_empty() {
                state.effects.rename_gobjs(&renamed);
                state.quakes.rename_gobjs(&renamed);
                state.items.rename_gobjs(&renamed);
            }
        }
        // A hit's quake (ftCo_8008DCE0's Camera_RequestQuake) precedes the
        // models its damage entry creates after it: Bury's mound, the ice
        // block.
        state.quakes.create_fighter_gobjs(&mut state.effects.gobjs);
        // Effects a proc spawned after the items it created (their creation
        // particles came first) and after the articles it removed
        // (Item_8026A8EC frees an article's models and GObjs when the proc
        // calls it: a KO's quake and blast follow ftCo_800D331C).
        for (slot, fighter) in state.fighters.iter_mut().enumerate() {
            crate::scene_fighter::with_fighter!(fighter, |f| {
                if !f.core.effects_after_items.is_empty() {
                    for request in f.core.effects_after_items.take_all() {
                        f.core.effects.push(request);
                    }
                    state.effects.flush::<RetailTrig>(
                        melee_ef::EffectTiming::Immediate,
                        slot,
                        &mut f.core,
                        &state.assets.common_particle_bank,
                        &state.assets.particle_bank,
                        &mut state.particles,
                        &mut state.rng,
                    )?;
                }
            });
        }
        crate::quake::request_effect_quakes(state);
        // A request made after the proc's own were taken: a throw's release
        // hit lands on its victim in the thrower's animation proc
        // (`grab_pairs::release`), ahead of this tick's quake procs.
        for fighter in &mut state.fighters {
            if let Some(kind) = fighter.0.quake_request.take() {
                state.quakes.request_from_fighter(&mut state.camera, kind);
            }
        }
        state.quakes.create_fighter_gobjs(&mut state.effects.gobjs);
        let items = &state.items;
        state
            .article_poses
            .retain(|id| items.iter().any(|item| item.id == id));
        self.particle_draws.0.append(&mut state.effects.draws.0);
        Ok(())
    }
}
/// Owns all mutable simulation state. Once constructed, tick has no trace,
/// ledger, seed, or frame-specific argument: the input script is fixed up front.
/// One joint as the retail bone dump records it: the local transform and
/// the cached world matrix.
#[derive(Clone, Copy, Debug)]
pub struct JointPose {
    pub rotate: [f32; 4],
    pub scale: [f32; 3],
    pub translate: [f32; 3],
    /// JOBJ_USE_QUATERNION; otherwise `rotate[3]` is unused stack data.
    pub quaternion: bool,
    /// `mtx`, row-major: the world matrix as last built, which HSD rebuilds
    /// only on demand (`HSD_JObjSetupMatrix`).
    pub world: [[f32; 4]; 3],
    /// JOBJ_MTX_DIRTY: `world` is stale, waiting for the next demand.
    pub world_dirty: bool,
}

#[derive(Clone)]
pub struct Simulation {
    world: World,
    // Unique initialization-time storage keeps cold-start call stacks small.
    // The scheduler borrows this owner; no reference counts or runtime borrows.
    runtime: Box<Runtime>,
    registrations: Vec<Registration>,
}
impl Simulation {
    pub(crate) fn complete_setup(&mut self) -> Result<()> {
        self.tick_without_snapshot()?;
        self.runtime.frame = 0;
        self.runtime.state.resume =
            crate::initial_state::scheduler_resume::SchedulerResume::between_ticks(false);
        Ok(())
    }
    pub(crate) fn state(&self) -> &InitialState {
        &self.runtime.state
    }
    /// Read-only, for presentation: whether the Ground GObj with this
    /// scheduler key (a map id, or a stage's extra key) currently exists.
    pub(crate) fn stage_object_live(&self, key: u8) -> bool {
        self.runtime
            .stage_objects
            .get(usize::from(key))
            .is_some_and(Option::is_some)
    }
    pub(crate) fn is_faulted(&self) -> bool {
        self.runtime.error.is_some()
    }
    pub(crate) fn poison(&mut self, error: &str) {
        self.runtime.error = Some(error.into());
    }
    pub fn set_inputs(&mut self, pads: [PadSample; 4]) {
        self.runtime.pads = pads;
    }
    /// The next tick's raw `PADStatus` sticks per port, before HSD clamps
    /// them (held until replaced). `None` derives them from each pad's
    /// normalized sticks, exact inside the 80-unit circle. Only
    /// controller-fix codes read raw bytes.
    pub fn set_raw_sticks(&mut self, raw: Option<[melee_ft::input::RawSticks; 4]>) {
        self.runtime.raw_sticks = raw;
    }
    /// Record each fighter's state at the end of its map proc
    /// (Fighter_8006C27C, s_link 6) every tick: where Slippi before 3.4.0
    /// sampled Post Frame (hook 0x8006C5D8).
    pub fn capture_after_map(&mut self, enabled: bool) {
        self.runtime.after_map = enabled.then(Vec::new);
    }
    /// The last tick's record as each fighter stood after its map proc:
    /// fighter `N`'s `pN.*` keys from its own snapshot, the rest from the
    /// tick's end (`end`).
    pub fn after_map_record(&self, end: &Record) -> Option<Record> {
        let snapshots = self.runtime.after_map.as_ref()?;
        let mut record = end.clone();
        for (fighter, snapshot) in snapshots {
            let prefix = format!("p{fighter}.");
            for (key, value) in &snapshot.state {
                if key.starts_with(&prefix) {
                    record.state.insert(key.clone(), value.clone());
                }
            }
        }
        Some(record)
    }
    /// [`Self::capture_after_map`] as plain values: each fighter's own
    /// snapshot values at the end of its map proc, with no record built.
    pub fn capture_after_map_values(&mut self, enabled: bool) {
        // One sample per fighter and tick; room for a second keeps a
        // repeated map proc from allocating.
        let capacity = 2 * self.runtime.state.fighters.len();
        self.runtime.after_map_values = enabled.then(|| Vec::with_capacity(capacity));
    }
    /// The last tick's after-map samples in the order the map procs ran:
    /// (fighter index, its values). `None` unless
    /// [`Self::capture_after_map_values`] is on.
    pub fn after_map_values(&self) -> Option<&[(usize, crate::diagnostics::FighterValues)]> {
        self.runtime.after_map_values.as_deref()
    }
    /// How many fighters the match holds (`pN` in a snapshot record).
    pub fn fighter_count(&self) -> usize {
        self.runtime.state.fighters.len()
    }
    /// Fighter `index`'s snapshot values now: what [`Self::tick`]'s record
    /// holds under `p{index}.`, in the order of [`Self::fighter_paths`].
    pub fn fighter_values(&self, index: usize) -> crate::diagnostics::FighterValues {
        crate::diagnostics::FighterValues::of(&self.runtime.state.fighters[index])
    }
    /// Each fighter's image of retail's motion scratch words at mv+0x14 and
    /// mv+0x18 (`melee_ft::fighter::scratch_tail`), in fighter order, for
    /// `melee-sim scratch-diff`.
    pub fn fighter_scratch_tails(&self) -> Vec<[u32; 2]> {
        self.runtime
            .state
            .fighters
            .iter()
            .map(|fighter| fighter.0.scratch_tail())
            .collect()
    }
    /// Each fighter's second motion scratch word (mv+4, fp+0x2344) as the
    /// port models it (`Fighter::inherited_scratch_word`), `None` where it
    /// does not: `melee-sim scratch-diff --word` checks it against retail.
    pub fn fighter_scratch_words(&self) -> Vec<Option<f32>> {
        self.runtime
            .state
            .fighters
            .iter()
            .map(|fighter| fighter.0.inherited_scratch_word())
            .collect()
    }
    /// Replace fighter `index`'s image of those words with a recording's,
    /// so `melee-sim scratch-diff` names each difference where it starts.
    pub fn set_fighter_scratch_tail(&mut self, index: usize, words: [u32; 2]) {
        self.runtime.state.fighters[index].0.scratch_tail_image = words;
    }
    /// The path of each of a fighter's snapshot values, without its `pN.`.
    pub fn fighter_paths(&self) -> Vec<String> {
        crate::diagnostics::fighter_paths(&self.runtime.state)
    }
    /// The random seed now (`rng.seed` in a snapshot record).
    pub fn seed(&self) -> u32 {
        self.runtime.state.rng.seed
    }
    /// Replace each port's controller fix (a dry run or search borrowing
    /// another scenario's recorded boundary state).
    pub fn set_controller_fixes(&mut self, fixes: [melee_ft::input::ControllerFix; 4]) {
        self.runtime.state.controller_fixes = fixes;
    }
    /// The next tick's pad-queue bytes as a retail recording read them
    /// (held until replaced); `None` models the queue instead.
    pub fn set_recorded_pad_queue_x(&mut self, queue: Option<[melee_ft::input::PadQueueX; 4]>) {
        self.runtime.recorded_pad_queue_x = queue;
    }
    /// The next tick's external events (held until replaced).
    pub fn set_external_events(&mut self, events: crate::ExternalEvents) {
        self.runtime.external = events;
    }
    /// What the last tick consumed from its external events.
    pub fn consumed_events(&self) -> crate::ConsumedEvents {
        self.runtime.consumed
    }
    /// Whether a display pass (particleSort) precedes the next tick.
    pub fn set_display_pass(&mut self, rendered: bool) {
        self.set_display(if rendered {
            DisplayPass::Rendered
        } else {
            DisplayPass::Skipped
        });
    }
    /// What precedes the next tick in place of a display pass.
    pub fn set_display(&mut self, display: DisplayPass) {
        self.runtime.display_pass = display;
    }
    pub fn frame(&self) -> u64 {
        self.runtime.frame
    }
    pub fn item_snapshot(&self, frame: u64) -> Record {
        crate::diagnostics::item_snapshot(&self.runtime.state.items, frame)
    }
    /// Each item's GObj address identity and kind, in item-list order, for
    /// `melee-sim gobj-diff`.
    pub fn item_gobjs(&self) -> Vec<(u32, i32)> {
        self.runtime
            .state
            .items
            .iter()
            .map(|item| (item.gobj.index(), i32::from(item.kind)))
            .collect()
    }
    /// The GObj creations and destructions since the last call, in order:
    /// whether created, the classifier, the address identity.
    pub fn take_gobj_events(&mut self) -> Vec<(bool, u16, u32)> {
        self.runtime
            .state
            .effects
            .gobjs
            .take_events()
            .map(|event| {
                (
                    event.created,
                    event.class.classifier(),
                    event.address.index(),
                )
            })
            .collect()
    }
    /// The stage's camera description (stage_info.cam_info).
    pub fn stage_camera(&self) -> melee_cm::StageCamera {
        self.runtime.state.assets.stage_camera
    }
    /// Pokemon Stadium's jumbotron between two ticks: its mode, when that
    /// began and when its timer ends it. `None` on every other stage. For
    /// the replay runner's lag-frame inference; the tick keeps nothing
    /// for it.
    pub fn stadium_screen(&self) -> Option<melee_gr::stadium::Showing> {
        match &self.runtime.state.stage {
            SceneStage::Stadium(stage) => Some(stage.screen.showing(&stage.parameters)),
            _ => None,
        }
    }
    /// The particle system in the retail particle-dump format, for
    /// `melee-sim particles-diff`.
    pub fn particle_snapshot(&self, frame: u64) -> Record {
        let mut state = std::collections::BTreeMap::new();
        self.write_particle_fields(&mut state);
        Record {
            frame,
            phase: "particles".into(),
            state,
        }
    }
    /// Every field of the retail particle dump into `sink`: what
    /// [`Self::particle_snapshot`] holds, without spelling a record.
    pub fn write_particle_fields(&self, sink: &mut impl crate::diagnostics::ParticleFieldSink) {
        struct LiveBanks<'a>(&'a InitialState);
        impl crate::initial_state::particles::Banks for LiveBanks<'_> {
            fn bank(&self, id: u8) -> &hsd_particle::bank::ParticleBank {
                match id {
                    0 => &self.0.assets.common_particle_bank,
                    30 => &self.0.assets.particle_bank,
                    _ => self.0.particles.bank(id).expect("loaded particle bank"),
                }
            }
        }
        let state = &self.runtime.state;
        crate::initial_state::particles::write_fields(
            &state.particles,
            state.rng.seed,
            &LiveBanks(state),
            sink,
        );
    }
    /// Every fighter's joints in part order, for bone-dump oracles.
    pub fn joint_poses(&self) -> Vec<Vec<JointPose>> {
        self.runtime
            .state
            .fighters
            .iter()
            .map(|fighter| {
                crate::scene_fighter::with_fighter!(fighter, |f| f
                    .animation
                    .parts
                    .iter()
                    .map(|part| {
                        let joint = f.skeleton.get(part.joint);
                        JointPose {
                            rotate: [
                                joint.rotate.x,
                                joint.rotate.y,
                                joint.rotate.z,
                                joint.rotate.w,
                            ],
                            scale: [joint.scale.x, joint.scale.y, joint.scale.z],
                            translate: [joint.translate.x, joint.translate.y, joint.translate.z],
                            quaternion: joint.flags & hsd_anim::jobj::JOBJ_USE_QUATERNION != 0,
                            world: joint.mtx.0,
                            world_dirty: joint.flags & hsd_anim::jobj::JOBJ_MTX_DIRTY != 0,
                        }
                    })
                    .collect())
            })
            .collect()
    }
    /// A simulation with neutral pads on every port.
    pub fn new(state: InitialState) -> Self {
        Self::with_inputs(state, [PadSample::default(); 4])
    }
    pub fn with_inputs(mut state: InitialState, pads: [PadSample; 4]) -> Self {
        // Prepare ordinary combat storage before ticks. These reserve budgets
        // do not change retail allocation limits or particle failure behavior.
        // Sized from random two-player matches on every stage
        // (tests/allocation.rs): link 0 carries most fighter and stage
        // particles and passed 512 there; no other link passed 128.
        const GENERATOR_RESERVE: usize = 256;
        const PARTICLES_LINK_0_RESERVE: usize = 1024;
        const PARTICLES_PER_LINK_RESERVE: usize = 256;
        state
            .particles
            .generators
            .reserve(GENERATOR_RESERVE.saturating_sub(state.particles.generators.len()));
        state
            .particles
            .pending_generators
            .reserve(GENERATOR_RESERVE);
        for (index, link) in state.particles.particles.iter_mut().enumerate() {
            let reserve = if index == 0 {
                PARTICLES_LINK_0_RESERVE
            } else {
                PARTICLES_PER_LINK_RESERVE
            };
            link.reserve(reserve.saturating_sub(link.len()));
        }
        let players = state.fighters.iter().filter(|f| !f.0.player.secondary).count();
        let mut rows = registrations(&state.stage, players, state.fighters.len());
        if state.banner.is_some() {
            rows.push(Registration {
                s_link: 0,
                p_link: 14,
                priority: 0,
                object: 0,
                callback: Callback::Banner,
            });
        }
        if state.item_spawner.is_some() {
            // it_8026D018: GObj_Create(5, 7, 0), its proc at s_link 0.
            rows.push(Registration {
                s_link: 0,
                p_link: 7,
                priority: 0,
                object: 0,
                callback: Callback::ItemSpawner,
            });
        }
        // Ground_801C0FB8's GObjs join the scheduler when the stage starts.
        let start_rows: Vec<(usize, u8, u8)> = state
            .stage
            .start_proc_table()
            .into_iter()
            .map(|r| {
                rows.push(Registration {
                    s_link: r.s_link,
                    p_link: r.p_link,
                    priority: r.p_priority,
                    object: u8::MAX,
                    callback: Callback::Stage {
                        map: None,
                        address: r.address,
                    },
                });
                (rows.len() - 1, r.p_link, r.s_link)
            })
            .collect();
        use crate::scene_fighter::with_fighter;
        let players = state.fighters.iter().filter(|f| !f.0.player.secondary).count();
        let interface = (0..players)
            .map(|player| {
                melee_if::PercentDisplay::new(with_fighter!(state.player_fighter(player), |f| f
                    .physics
                    .percent))
            })
            .collect();
        let mut runtime = Box::new(Runtime {
            radial_forces: Default::default(),
            item_objects: Default::default(),
            stage_objects: Default::default(),
            stage_start_rows: start_rows.clone(),
            state,
            interface,
            match_finished: false,
            start_held: [false; 4],
            display_pass: DisplayPass::Rendered,
            pads,
            raw_sticks: None,
            recorded_pad_queue_x: None,
            raw_stick_x_history: Default::default(),
            pad_queue_x: Default::default(),
            consumed_sticks: Default::default(),
            after_map: None,
            after_map_values: None,
            external: Default::default(),
            consumed: Default::default(),
            frame: 0,
            error: None,
            // At most one writer per registered proc, plus match-start music.
            rng_writers: Vec::with_capacity(rows.len() + 1 + melee_it::ITEM_CAPACITY * 10),
            particle_draws: DrawLog(Vec::with_capacity(4096)),
        });
        let mut world = World::new(WorldConfig::MELEE);
        let mut objects = BTreeMap::new();
        for (index, row) in rows.iter().enumerate() {
            if start_rows.iter().any(|&(start, ..)| start == index) {
                continue;
            }
            let object = *objects
                .entry((row.p_link, row.object))
                .or_insert_with(|| world.create(0, row.p_link, row.priority));
            if let Callback::Stage { map: Some(map), .. } = row.callback {
                runtime.stage_objects[usize::from(map)] = Some(object);
            }
            world.add_tagged_proc(object, row.s_link, index);
        }
        crate::scene_items::prepare_scheduler(&mut world);
        // A stage's material item exists from its creation.
        crate::scene_items::register_existing(
            &runtime.state.items,
            &mut world,
            &mut runtime.item_objects,
        );
        world.reserve_removals();
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
        Ok(crate::diagnostics::snapshot(
            &runtime.state,
            runtime.frame - 1,
        ))
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
            // Audio and controller output belong to one completed frame. The
            // headless sink retires them before the next frame's callbacks.
            for fighter in &mut runtime.state.fighters {
                fighter.0.commands.footstep_sounds.clear();
                fighter.0.commands.rumble_requests.clear();
            }
            runtime.state.effects.events.begin_tick(frame);
            runtime.consume_pad_queue();
            if let Some(snapshots) = runtime.after_map.as_mut() {
                snapshots.clear();
            }
            if let Some(values) = runtime.after_map_values.as_mut() {
                values.clear();
            }
            runtime.consumed = Default::default();
            runtime.rng_writers.clear();
            runtime.particle_draws.0.clear();
            runtime.state.effects.direct_draws.clear();
            if let Some((music, unlocked)) = runtime.state.pending_music.take() {
                runtime.state.selected_music = Some(if runtime.state.clock.sudden_death {
                    music.select_sudden_death()
                } else {
                    music.select(unlocked, &mut runtime.state.rng)
                });
                let seed = runtime.state.rng.seed;
                runtime.rng_writers.push((None, seed));
            }
            // Rendering refreshes the joints' cached matrices (read when
            // dynamics reclaim a chain) and runs particleSort
            // (psdisp.c:0x8039FC70); both happen only when a display pass
            // separated this tick from the previous one.
            match runtime.display_pass {
                DisplayPass::Rendered if runtime.frame != 0 => {
                    for fighter in &mut runtime.state.fighters {
                        fighter.0.prepare_dynamic_display_caches();
                    }
                    runtime.state.particles.sort_for_display(7);
                    render_cameras(&mut runtime.state);
                    display_posed_articles(&mut runtime.state, runtime.frame - 1);
                    display_held_items(&mut runtime.state);
                }
                DisplayPass::RollbackLoop if runtime.frame != 0 => {
                    rollback_loop_cameras(&mut runtime.state);
                }
                _ => {}
            }
        }
        let runtime = self.runtime.as_mut();
        // gm_16AE.c:546-590, every scene frame in a team battle: a player
        // out of stocks whose fighter sleeps takes a teammate's stock on a
        // Start press (fn_8016B918, ifStock_802F7EFC, Player_LoseStock,
        // gm_8016C658, fn_8016719C).
        const START: u32 = 0x1000;
        for fighter in runtime.state.fighters.iter().filter(|f| !f.0.player.secondary) {
            let player = &fighter.0.player;
            let port = usize::from(player.id);
            let start = runtime.pads[port].buttons.0 & START != 0;
            if player.team.is_some()
                && player.stocks == 0
                && fighter.0.status.disabled
                && start
                && !runtime.start_held[port]
            {
                unimplemented!("gm_16AE.c:576-586: a team battle's stock steal on a Start press");
            }
        }
        runtime.start_held = runtime.pads.map(|pad| pad.buttons.0 & START != 0);
        // gm_GetMatchOutcome returns OUTCOME_TIMEOUT before checking teams
        // (gm_16AE.c:886-891). Team result ranking then uses fn_80165E7C's
        // team scores and fn_801661E0's subscore tie-break (gm_1601.c:2853-2878,
        // 2926-2936), not individual stocks. Keep countdown replays runnable
        // until this unwitnessed result branch is actually reached.
        if runtime.state.clock.timed_out()
            && runtime.state.fighters.iter().any(|f| f.0.player.team.is_some())
        {
            unimplemented!("fn_80165E7C/fn_801661E0: timed team result ranking needs a retail witness");
        }
        // The pause takes effect next tick.
        let finished = runtime.state.clock.timed_out()
            || stock_outcome_decided(
                runtime
                    .state
                    .fighters
                    .iter()
                    .filter(|fighter| !fighter.0.player.secondary)
                    .map(|fighter| (fighter.0.player.team, fighter.0.player.stocks)),
            );
        if finished && !runtime.match_finished {
            // The GAME! (or TIME!) banner's GObj (classifier 0xE, p_link
            // 14), created by the scene's frame callback before this
            // tick's procs: it takes the address freed last. A match with
            // a time limit first destroys a HUD GObj of the timer's, whose
            // address the banner then takes.
            let gobjs = &mut runtime.state.effects.gobjs;
            if runtime.state.clock.timer.is_some() {
                let timer = gobjs.outside();
                gobjs.destroy(hsd_gobj::GObjClass::Hud, timer);
            }
            gobjs.create(hsd_gobj::GObjClass::Hud);
        }
        runtime.match_finished |= finished;
        // fn_8016CFE0 -> fn_8016758C, every scene frame. Only the count of
        // ticks between two revivals matters, not their place in the frame.
        runtime.state.revival_offsets.tick();
        // gm_Scene_Vs_OnFrame -> fn_8016CD98, only while no outcome is decided.
        if !runtime.match_finished {
            runtime.state.clock.advance();
        }
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
            } else if index >= crate::scene_stage::last::TAG_BASE {
                let tag = index - crate::scene_stage::last::TAG_BASE;
                let map = (tag / 3) as u8;
                let phase = tag % 3;
                Registration {
                    s_link: if phase == 0 { 1 } else { 4 },
                    p_link: 5,
                    priority: 0,
                    object: map,
                    callback: Callback::Stage {
                        map: Some(map),
                        address: match phase {
                            0 => 0x801C1CD0,
                            1 => 0x801C1D38,
                            _ => match runtime.state.stage {
                                SceneStage::Battlefield(_) => melee_gr::battle::map_callback(map),
                                SceneStage::Stadium(_) => {
                                    melee_gr::stadium::procs::map_callback(map)
                                }
                                SceneStage::Kongo(_) => melee_gr::kongo::procs::map_callback(map)
                                    .expect("an event map's gobj proc")
                                    .1,
                                _ => melee_gr::last::procs::map_callback(map),
                            },
                        },
                    },
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
                    Some(format!("frame {} proc {:?}: {error:#}", runtime.frame, row).into());
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
    pub fn enable_spawn_recording(&mut self) {
        self.runtime.state.effects.events.enable();
    }
    pub fn finish_spawn_recording(&mut self) -> BTreeMap<u64, Vec<serde_json::Value>> {
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
    /// Replace the RNG seed between ticks, as a recorder's write to the
    /// retail `seed` word (0x804D5F90) does at a boundary.
    pub fn set_rng_seed(&mut self, seed: u32) {
        self.runtime.state.rng.seed = seed;
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

/// grStory_801E3334: Yoshi's Story's map 3 proc, the Shy Guy spawner.
const STORY_SHY_GUY_PROC: u32 = 0x801E_3334;

/// The other fighters holding ledges, offered to `player`'s proc for
/// ft_80082E3C (retail walks the live fighter list).
fn offer_ledge_holders(state: &mut InitialState, player: usize) {
    use crate::scene_fighter::with_fighter;
    use melee_ft::fighter::ledge::LedgeHolder;
    let mut holders = [None; 6];
    for (slot, other) in holders.iter_mut().zip(&state.fighters) {
        *slot = with_fighter!(other, |f| LedgeHolder::of(&f.core));
    }
    let holders = holders
        .into_iter()
        .enumerate()
        .filter(|&(other, _)| other != player)
        .filter_map(|(_, holder)| holder);
    with_fighter!(&mut state.fighters[player], |f| f
        .core
        .ledge_holders
        .offer(holders));
}

/// ftCo_800A4A40 for a fighter whose tether article is out, before its
/// accessory proc (the grapple beam's button code reads it there).
fn offer_nearest_fighter(state: &mut InitialState, player: usize) {
    use crate::scene_fighter::with_fighter;
    if !with_fighter!(&state.fighters[player], |f| f.core.tether_article) {
        return;
    }
    let mut views = [None; 6];
    for (slot, other) in views.iter_mut().zip(&state.fighters) {
        *slot = Some(with_fighter!(other, |f| melee_cpu::FighterView::of(f)));
    }
    let (position, half_size) = with_fighter!(&state.fighters[player], |f| (
        f.core.physics.position,
        f.core.cpu.half_size
    ));
    let others = views
        .into_iter()
        .enumerate()
        .filter(|&(other, _)| other != player)
        .filter_map(|(_, view)| view);
    let nearest =
        melee_cpu::nearest_fighter(position, half_size, &state.assets.arena, others);
    with_fighter!(&mut state.fighters[player], |f| f.core.nearest_fighter =
        nearest);
}

/// An article's request of its owner from inside its physics proc
/// (retail calls the fighter there): the owner reacts, then for a catch the
/// article hangs from the owner's part again (Item_8026AB54: attach, then
/// the kind's pickup callback).
fn deliver_owner_request(
    state: &mut InitialState,
    id: u32,
    kind: melee_types::ItemKind,
    slot: u8,
    request: melee_it::OwnerRequest,
) {
    use crate::scene_items::SceneItems;
    use melee_it::ItemDispatch;
    let Some(index) = state
        .fighters
        .iter()
        .position(|fighter| crate::scene_fighter::with_fighter!(fighter, |f| f.player.id == slot))
    else {
        return;
    };
    let part = crate::scene_fighter::with_fighter!(&mut state.fighters[index], |f| f
        .article_request(&state.assets.fighters[index], kind, request));
    match (request, part) {
        (melee_it::OwnerRequest::Catch, None)
        | (melee_it::OwnerRequest::Released | melee_it::OwnerRequest::ArticleStep(_), _) => {}
        (melee_it::OwnerRequest::Catch, Some(part)) => {
            let lifetime = state.items.common().lifetime;
            let half_life_scale = state.items.common().half_life_scale;
            let assets = state.assets.items.get(kind);
            let item = state.items.get_mut(id).expect("caught article");
            item.attach_to_holder(slot, part, assets, lifetime, half_life_scale);
            (SceneItems::logic(kind).picked_up)(
                item,
                &mut melee_it::ItemAnimationContext {
                    owner: None,
                    holder: None,
                    map: &mut state.map,
                    assets,
                    partner: None,
                    rng: None,
                },
            );
        }
    }
}

/// Whether a stock match is decided, from each player's (team, stocks).
/// gm_GetFFAOutcome (8016BF74): every player but at most one is out of
/// stocks. gm_GetTeamBattleOutcome (gm_16AE.c:830-858): every team but at
/// most one has no stocks left between its players.
pub(crate) fn stock_outcome_decided(players: impl Iterator<Item = (Option<u8>, u8)>) -> bool {
    // Each side's stocks: a team's summed, a free-for-all player's own.
    let mut sides = [(None::<u8>, 0u32); 6];
    let mut count = 0;
    for (team, stocks) in players {
        let side = match team {
            Some(_) => sides[..count].iter().position(|side| side.0 == team),
            None => None,
        };
        let side = side.unwrap_or_else(|| {
            sides[count] = (team, 0);
            count += 1;
            count - 1
        });
        sides[side].1 += u32::from(stocks);
    }
    let out = sides[..count].iter().filter(|side| side.1 == 0).count();
    out + 1 >= count
}

/// The team a fighter's hitboxes strike items for (itcoll.c:427-431): a
/// thrown fighter's thrower's (ftLib_80086EB4 of x1064's owner), else its own.
fn thrown_hitbox_team(
    fighters: &[crate::scene_fighter::SceneFighter],
    fighter: &crate::scene_fighter::SceneFighter,
) -> Option<u8> {
    match fighter.commands.thrown_by {
        Some(spawn) => fighters
            .iter()
            .find(|f| f.spawn_number == spawn)
            .and_then(|thrower| thrower.player.team),
        None => fighter.player.team,
    }
}

/// A posed article's bone matrix, its attach bone constrained to its
/// holder's part as the holder stands now.
/// Fighter GObjs a Versus match can hold: four players and their partners.
pub(crate) const MAX_FIGHTERS: usize = 8;
/// Camera subjects one tick can hold: retail links them in a list with no
/// bound (cm_804D6468); this is the port's fixed pool, room for eight
/// fighters, a stage subject and the framed items.
const MAX_CAMERA_SUBJECTS: usize = 64;
/// Run `f` over the camera subjects as one slice of filled slots, without
/// allocating.
pub(crate) fn with_subjects<'a, R>(
    mut subjects: impl Iterator<Item = &'a mut melee_cm::Subject>,
    f: impl FnOnce(&mut [Option<&'a mut melee_cm::Subject>]) -> R,
) -> R {
    let mut slots: [Option<&'a mut melee_cm::Subject>; MAX_CAMERA_SUBJECTS] =
        std::array::from_fn(|_| subjects.next());
    assert!(
        subjects.next().is_none(),
        "more than {MAX_CAMERA_SUBJECTS} camera subjects"
    );
    let count = slots.iter().take_while(|slot| slot.is_some()).count();
    f(&mut slots[..count])
}
fn article_bone_matrix(
    fighters: &mut [crate::scene_fighter::SceneFighter],
    poses: &mut crate::article_pose::ArticlePoses,
    templates: &[crate::article_pose::ArticleSkeleton],
    resources: &crate::scene_items::Resources,
    item: &melee_it::ItemCore,
    bone: usize,
    tick: u64,
) -> hsd_types::Mtx {
    if !crate::article_pose::ArticlePoses::poses_kind(templates, item.kind) {
        return crate::scene_items::free_item_bone_matrix(resources, item, bone);
    }
    let holder = crate::scene_items::owner_index(fighters, item.owner, item.owner_secondary)
        .expect("a posed article's holder fighter");
    let fighter = &mut fighters[holder];
    let part = usize::from(item.holder_part);
    let hand = fighter.bone_matrix(Some(part));
    let orientation = fighter.orientation_target(part);
    poses.bone_matrix(item, templates, bone, tick, &hand, orientation)
}

/// A display pass draws each held item at its holder part's matrix (the
/// hand constraint sets that part up), and the root's cached matrix keeps
/// that translation until an HSD_JObjSetTranslate dirties it.
fn display_held_items(state: &mut InitialState) {
    for item in state.items.iter_mut() {
        item.held_display_root = None;
        if !item.held {
            continue;
        }
        let Some(index) =
            crate::scene_items::owner_index(&state.fighters, item.owner, item.owner_secondary)
        else {
            continue;
        };
        let matrix = state.fighters[index]
            .0
            .core
            .item_holder(item.holder_part, &state.assets.fighters[index])
            .part_matrix();
        item.held_display_root = Some(hsd_types::Vec3::new(
            matrix.0[0][3],
            matrix.0[1][3],
            matrix.0[2][3],
        ));
    }
}

/// A display pass draws each held posed article (its render callback's
/// HSD_JObjDispAll), setting up its joints with the holder as it stands.
fn display_posed_articles(state: &mut InitialState, after_tick: u64) {
    let templates = &state.assets.items.article_skeletons;
    for item in state.items.iter() {
        if item.owner.is_none()
            || !crate::article_pose::ArticlePoses::poses_kind(templates, item.kind)
        {
            continue;
        }
        let holder =
            crate::scene_items::owner_index(&state.fighters, item.owner, item.owner_secondary)
                .expect("a posed article's holder fighter");
        let fighter = &mut state.fighters[holder];
        let part = usize::from(item.holder_part);
        let hand = fighter.bone_matrix(Some(part));
        let orientation = fighter.orientation_target(part);
        state
            .article_poses
            .display(item, templates, after_tick, &hand, orientation);
    }
}

/// A homing article's candidates: every fighter in the fighter list with its
/// out-of-play flag and ftLib_800866DC's camera bone position, and the items
/// it_8026C258 accepts (hold kinds 4..=7, not held by an owner) at their
/// ECB centre (it_8026BB88: fadds of the halved top and bottom).
fn lock_on_targets(state: &mut InitialState, targets: &mut melee_it::LockOnTargets) {
    for fighter in state.fighters.iter_mut() {
        crate::scene_fighter::with_fighter!(fighter, |f| {
            let f: &mut melee_ft::fighter::Fighter = f;
            targets.fighters.push(melee_it::LockOnFighter {
                player: f.core.player.id,
                disabled: f.core.status.disabled,
                position: f.core.camera_bone_position(),
                team: f.core.player.team,
            });
        });
    }
    for item in state.items.iter() {
        if item.destroyed || !(4..=7).contains(&item.hold_kind) {
            continue;
        }
        if item.held && item.owner.is_some() {
            continue;
        }
        let ecb = item.collision.as_ref().expect("item map collision").ecb;
        let offset = 0.5 * (ecb.top.y + ecb.bottom.y);
        targets.items.push(melee_it::LockOnItem {
            position: hsd_types::Vec3::new(
                item.position.x + 0.0,
                item.position.y + offset,
                item.position.z + 0.0,
            ),
        });
    }
}

/// The stage limits item callbacks read: the blast zones (Stage_GetBlastZone*
/// Offset) and the camera centre (Stage_UnkSetVec3TCam_Offset).
fn item_bounds(assets: &crate::assets::Assets) -> melee_it::ItemBounds {
    melee_it::ItemBounds {
        left: assets.arena.left,
        right: assets.arena.right,
        bottom: assets.arena.bottom,
        top: assets.arena.top,
        camera_offset: hsd_types::Vec2::new(
            assets.stage_camera.offset_x,
            assets.stage_camera.offset_y,
        ),
    }
}

/// ftColl_8007BE3C: an expired phantom credits its source fighter's current
/// move (plStale_UpdateStaleMovesFromFighter, ftColl_80076444) while the
/// victim's ProcessHit runs; the victim only records whom to credit.
fn credit_phantom_source(state: &mut crate::initial_state::InitialState, player: usize) {
    use crate::scene_fighter::with_fighter;
    use melee_coll::damage_log::HitSource;
    let Some(source) = with_fighter!(&mut state.fighters[player], |f| f
        .combat
        .pending_credit
        .take())
    else {
        return;
    };
    let victim = with_fighter!(&state.fighters[player], |f| f.spawn_number);
    match source {
        HitSource::Fighter(source) => {
            let credited = state
                .fighters
                .iter()
                .position(|f| f.spawn_number == source);
            if let Some(slot) = credited {
                partner_fighters::take_partner_stale_table(
                    &mut state.fighters,
                    &state.assets.fighters,
                    slot,
                );
                let assets = &state.assets.fighters[slot];
                with_fighter!(&mut state.fighters[slot], |f| f.core.credit_hit(victim, assets));
            }
        }
        // ftColl_8007BE3C tests the classifier of the GObj at dmg.x1894 when
        // the lockout ends (retail 0x8007BF4C: lwz 0x1894; 0x8007BF50: lhz
        // 0x0; only 4, a fighter, and 6, an item, credit anyone). An item
        // that has gone by then left a freed GObj, whose first word is the
        // allocator's free-list link (HSD_ObjFree, 0x8037AD20): nobody is
        // credited, and the move does not stale. An item created at that
        // address since is read in its place: its owner (ip->owner) and
        // attack (xD88 / xD8C), never the victim itself.
        HitSource::Item(source) => {
            let Some((item_owner, secondary, attack)) = state
                .items
                .iter()
                .find(|item| !item.destroyed && item.gobj.index() == source.gobj)
                .map(|item| (item.owner, item.owner_secondary, item.stale_source))
            else {
                return;
            };
            let owner = crate::scene_items::owner_index(&state.fighters, item_owner, secondary)
                .filter(|&owner| owner != player);
            if let Some(owner) = owner {
                if let Some(attack) = attack {
                    partner_fighters::take_partner_stale_table(
                        &mut state.fighters,
                        &state.assets.fighters,
                        owner,
                    );
                    let assets = &state.assets.fighters[owner];
                    state.fighters[owner].core.credit_item_hit(attack, assets);
                }
                let combo = &state.assets.fighters[owner].combo;
                state.fighters[owner].combat.combo.record(
                    victim,
                    attack.map(|a| a.move_id),
                    combo,
                );
            }
        }
    }
}

/// The display pass's camera renders that feed gameplay, in HSD_GObj_80390FC0
/// order (render priority): the magnifier camera (0) before the main camera
/// (2), so the magnifier sees the previous pass's off-screen flags.
fn render_cameras(state: &mut InitialState) {
    // ifMagnify_802FBBDC: the HUD is visible in a running Versus match.
    for fighter in &mut state.fighters {
        let f = &mut fighter.0;
        // ftLib_80086ED0: invisible, x221E_b2 (the dead, star, screen and
        // revival states) and x2220_b7 hide the fighter.
        let drawn = !f.effect_state.invisible && !f.hidden_by_life_state();
        f.offscreen.magnified = f.offscreen.outside_camera && drawn;
    }
    // fn_800301D0 -> Camera_8002A4AC, then each fighter's render callback
    // (ftDrawCommon_80080E18 -> ftLib_80086A8C -> Camera_80030CD8).
    let camera = state.camera.render_camera(&state.assets.stage_camera);
    let screen = state.camera.screen;
    state.rendered_camera = camera;
    // Camera_800310B8: cm_804D6464's viewing matrix, inverted for the screen KO.
    let copy_view = state
        .camera
        .render_copy_camera(&state.assets.stage_camera)
        .view_matrix();
    let mut inverse_copy_view = copy_view;
    hsd_anim::mtx::mtx_inverse(&copy_view, &mut inverse_copy_view);
    for fighter in &mut state.fighters {
        let f = &mut fighter.0;
        // ftDrawCommon_80080E18 tests x221F_b3 first (retail 0x80080E34): a
        // sleeping fighter (a transforming character's other form) is not
        // drawn, so its off-screen bit keeps what its last pass awake left.
        // The form that wakes off screen reads as on screen to the next
        // pass's magnifier (ifMagnify_802FBBDC asks Player_GetEntity's
        // fighter, 0x802FBD1C), and the count it took over (ftCommon_8007EFC8
        // copies x1910, ftcommon.c:1309) restarts from zero.
        if f.status.disabled {
            continue;
        }
        if matches!(
            f.state_data,
            melee_ft::fighter::MotionData::Life(melee_ft::fighter::life::LifeState::ScreenKo(_))
        ) {
            // x2220_b7 places the fighter from camera space and
            // ftLib_80086A8C then reports it on screen.
            f.place_screen_ko(&inverse_copy_view);
            f.offscreen.outside_camera = false;
            continue;
        }
        let on_screen = melee_cm::to_screen(&camera, f.camera.bone_position, screen)
            .is_some_and(|point| point.on_screen);
        f.offscreen.outside_camera = !on_screen;
    }
}

/// What Slippi Online's rollback loop runs between two ticks in place of
/// the display pass (Online/Core/LoopEngineForRollback.asm, 0x801A5014,
/// slippi-ssbm-asm 0e90140): Camera_8002A4AC on the main camera, then
/// ftLib_80086A8C (0x80086A8C) for both fighters of every player slot. The
/// call is direct, not through the render callback, so the draw's own
/// conditions do not apply: a sleeping fighter (x221F_b3) is tested too,
/// and a fighter in the screen KO (x2220_b7) is reported on screen without
/// being placed (the placement is ftDrawCommon_80080E18's, 0x80080E90). The
/// magnifier (ifMagnify_802FBBDC) does not run; the netplay build does not
/// read it (`SlippiCodes::offscreen_damage_by_camera_limits`).
fn rollback_loop_cameras(state: &mut InitialState) {
    let camera = state.camera.render_camera(&state.assets.stage_camera);
    let screen = state.camera.screen;
    state.rendered_camera = camera;
    for fighter in &mut state.fighters {
        let f = &mut fighter.0;
        let in_screen_ko = matches!(
            f.state_data,
            melee_ft::fighter::MotionData::Life(melee_ft::fighter::life::LifeState::ScreenKo(_))
        );
        f.offscreen.outside_camera = !in_screen_ko
            && !melee_cm::to_screen(&camera, f.camera.bone_position, screen)
                .is_some_and(|point| point.on_screen);
    }
}

/// `UCF DB.asm`'s Ice Climbers branch (Gecko hook 0x800C9A44), after
/// Popo's input proc smash turned: Player_GetEntityAtIndex(slot, 1), and when
/// Nana exists, her newest follow sample (cpu.x444, fp+0x1ECC) takes Popo's
/// facing (+0x18) and a full stick toward it (+0x6).
fn partner_turn(state: &mut InitialState, player: usize) {
    let Some(turn) = state.fighters[player].0.input.hardware.partner_turn.take() else {
        return;
    };
    let slot = state.fighters[player].player.id;
    let Some(partner) = crate::scene_items::owner_index(&state.fighters, Some(slot), true) else {
        return;
    };
    let follow = &mut state.fighters[partner].0.cpu.follow;
    let sample = &mut follow.entries[follow.write];
    sample.facing = turn.facing;
    sample.stick[0] = turn.stick_x;
}

/// The sound draws a fighter's scripts issued with no graphics command ahead
/// of them. A motion change flushes the fighter's pending efAsync requests
/// (efAsync_QueueFlush at retail 0x800694A0, fighter.c:951) before the new
/// script's frame-0 commands run (ftAction_8007349C at 0x8006A0A4), so below
/// link 9 what that change sealed dispatches between the old script's sound
/// draws and the new script's.
fn resolve_issued_sounds(
    f: &mut melee_ft::fighter::Fighter,
    player: usize,
    scene_assets: &crate::assets::Assets,
    effects: &mut melee_ef::Effects,
    particles: &mut hsd_particle::system::ParticleSystem,
    rng: &mut gekko_math::HsdRng,
    below_link_9: bool,
) -> Result<()> {
    let assets = &scene_assets.fighters[player];
    let sound_waits = f.resolve_sound_draws_before_seal(assets, rng);
    // The old script's graphics commands queued before that change: its
    // flush created their effects too (efAsync_QueueFlush, newest first).
    let sealed_graphics = below_link_9 && f.resolve_graphics_commands_before_seal(assets, rng);
    if (sound_waits || sealed_graphics) && below_link_9 {
        // A teardown queued ahead of those graphics ran before the change
        // (Luigi's Green Missile landing: efLib_DestroyAll, then the motion
        // change whose flush creates the landing dust).
        let timing = if sealed_graphics {
            melee_ef::EffectTiming::SealedAfterTeardown
        } else {
            melee_ef::EffectTiming::Sealed
        };
        effects.flush::<melee_ft::fighter::RetailTrig>(
            timing,
            player,
            &mut f.core,
            &scene_assets.common_particle_bank,
            &scene_assets.particle_bank,
            particles,
            rng,
        )?;
    }
    f.resolve_sound_draws(assets, rng);
    Ok(())
}

/// The graphics commands a fighter's script issued below link 9: the efAsync
/// requests its motion change sealed dispatch first (Fighter_ChangeMotionState,
/// fighter.c:951), then each command draws its random offsets
/// (ftCo_8009F834).
fn resolve_issued_graphics(
    f: &mut melee_ft::fighter::Fighter,
    player: usize,
    scene_assets: &crate::assets::Assets,
    effects: &mut melee_ef::Effects,
    particles: &mut hsd_particle::system::ParticleSystem,
    rng: &mut gekko_math::HsdRng,
) -> Result<()> {
    if !f.commands.graphics.is_empty() || !f.commands.landing_effects.is_empty() {
        effects.flush::<melee_ft::fighter::RetailTrig>(
            melee_ef::EffectTiming::Sealed,
            player,
            &mut f.core,
            &scene_assets.common_particle_bank,
            &scene_assets.particle_bank,
            particles,
            rng,
        )?;
    }
    f.resolve_graphics_commands(&scene_assets.fighters[player], rng);
    Ok(())
}

#[allow(clippy::too_many_arguments)] // Borrow each subsystem independently while dispatching a concrete fighter.
fn dispatch_fighter(
    f: &mut melee_ft::fighter::Fighter,
    proc: FighterProc,
    player: usize,
    _frame: u64,
    state_pads: &[PadSample; 4],
    scene_assets: &crate::assets::Assets,
    map: &mut melee_mp::CollMap,
    effects: &mut melee_ef::Effects,
    particles: &mut hsd_particle::system::ParticleSystem,
    rng: &mut gekko_math::HsdRng,
    radial_forces: &mut melee_lb::radial_force::RadialForces,
    wind: melee_gr::wind::Wind,
) -> Result<()> {
    let assets = &scene_assets.fighters[player];
    // efAsync_Spawn (800679B0): from link 9 on, effects are created when
    // their command runs, reading the joint then.
    f.effect_state.dispatch_at_once = proc.s_link() >= 9;
    let was_in_hitlag = f.in_hitlag();
    let had_effect_callbacks = f.effect_state.hitlag_callbacks;
    let had_article_hitlag = f.effect_state.article_hitlag;
    match proc {
        FighterProc::Status => {
            // Fighter_8006A360: the parasol timer (fighter.c:1577) precedes
            // the hitlag countdown.
            f.tick_parasol_timer(assets)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            f.proc_status()
        }
        FighterProc::Animation => {
            f.proc_anim(assets, map, rng)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
        }
        FighterProc::CpuGate => f.proc_cpu_gate(),
        FighterProc::Input => {
            // HSD_PadGameStatus[fp->x618_player_id]: in a Vs match
            // the human slot's port is its player index.
            let pad: PadSample = state_pads[usize::from(f.player.id)];
            f.proc_input(assets, &pad);
            f.finish_input(assets, rng)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
        }
        FighterProc::Update => {
            f.proc_update(assets, map, wind);
            if let Some(splash) = f.core.physics.water_splash.take() {
                crate::scene_stage::garden::splash(scene_assets, particles, splash, rng)?;
            }
            f.check_blast_zone(assets, &scene_assets.arena, rng)
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        }
        FighterProc::Map => {
            f.proc_map_with_assets(assets, map)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
        }
        FighterProc::Pose => f.proc_pose(assets, map),
        FighterProc::Accessories => {
            f.proc_accessories();
            // accessory2 (accessory3 in hitlag) before the scene's accessory1.
            f.tether_accessory(assets, map)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
        }
        FighterProc::HitboxPositions => {
            effects.flush::<melee_ft::fighter::RetailTrig>(
                melee_ef::EffectTiming::Deferred,
                player,
                &mut f.core,
                &scene_assets.common_particle_bank,
                &scene_assets.particle_bank,
                particles,
                rng,
            )?;
            // Fighter_8006C80C: Fighter_UnkApplyTransformation_8006C0F0
            // after efAsync_QueueFlush, then accessory4.
            if !f.status.disabled {
                f.core.update_flat_matrix();
            }
            if !f.status.disabled
                && !f.in_hitlag()
                && !f.screen_ko_accessory(scene_assets.stage_camera.bottom())
                && !f.item_throw_accessory(assets)
                && !f.core.egg_accessory()
            {
                f.character_accessory(assets, rng);
                f.character_map_accessory(assets, map);
            }
            f.proc_hitbox_positions();
        }
        FighterProc::Grab => f.proc_grab(),
        FighterProc::HitDetection => f.proc_hit_detection(),
        FighterProc::ProcessHit => f.proc_process_hit_on_map(assets, rng, map),
        FighterProc::Dynamics => f.proc_dynamics_with_forces(assets, map, radial_forces.fields()),
        FighterProc::Camera => f.proc_camera_with_map(assets, &scene_assets.stage_camera, map),
        FighterProc::PlayerMirror => f.proc_player_mirror(),
    }
    // Fighter_8006D044/8006D10C invoke the installed callbacks on transitions,
    // not every frozen frame. New models created later are not retroactively paused.
    if proc == FighterProc::Status && was_in_hitlag && !f.in_hitlag() && had_effect_callbacks {
        effects.set_owner_hitlag(player, false);
    }
    if proc == FighterProc::ProcessHit
        && !was_in_hitlag
        && f.in_hitlag()
        && f.effect_state.hitlag_callbacks
    {
        effects.set_owner_hitlag(player, true);
    }
    // The same transitions for an article's pre/post-hitlag pair: the
    // scene applies the freeze with this proc's item requests.
    let article_hitlag = if proc == FighterProc::Status && was_in_hitlag && !f.in_hitlag() {
        had_article_hitlag.map(|kind| (kind, false))
    } else if proc == FighterProc::ProcessHit && !was_in_hitlag && f.in_hitlag() {
        f.effect_state.article_hitlag.map(|kind| (kind, true))
    } else {
        None
    };
    if let Some((kind, frozen)) = article_hitlag {
        let owner = f.player.id;
        f.core.item_requests.push(melee_it::ItemRequest::Control {
            owner,
            kind,
            control: melee_it::ItemControl::OwnerHitlag(frozen),
        });
        if frozen {
            f.article_hitlag_begin();
        } else {
            f.article_hitlag_end();
        }
    }
    // lb_800119DC requests in issue order: the pool prepends, and its list
    // order is the order lb_800101C8 sums the fields in. Queue lifetime is
    // owned by the scene's ground controller.
    for gust in f.commands.gusts.take_all() {
        let impulse = match gust {
            melee_ft::fighter::commands::GustRequest::Impulse(impulse) => impulse,
            // ftAction_80073118 / ftCo_8009E714: literal bone, rounded
            // fixed-point operands.
            melee_ft::fighter::commands::GustRequest::Wind(wind) => {
                let c = &mut f.core;
                let center = melee_ft::fighter::caches::bone_position(
                    &mut c.skeleton,
                    c.animation.root,
                    usize::from(wind.bone),
                    Vec3::new(
                        0.003906 * f32::from(wind.x),
                        0.003906 * f32::from(wind.y),
                        0.0,
                    ),
                );
                melee_lb::radial_force::RadialImpulse {
                    center,
                    frames: if wind.timer < 0 {
                        120
                    } else {
                        i32::from(wind.timer)
                    },
                    strength: 0.003906 * f32::from(wind.magnitude),
                    decay: 0.003906 * f32::from(wind.decay),
                    phase_step: 0.003906 * f32::from(wind.angle),
                }
            }
        };
        radial_forces.insert(impulse);
    }
    // Smash voices (opcode 18) and random sounds (opcode 38) issued before
    // any graphics command draw before the following effect boundary; those
    // behind a graphics command draw after its offsets, in script order
    // (resolve_graphics_commands).
    resolve_issued_sounds(
        f,
        player,
        scene_assets,
        effects,
        particles,
        rng,
        proc.s_link() < 9,
    )?;
    // Opcode 54's footstep pitch draws (ftAction_80072CD8 -> ft_PlaySFX).
    f.resolve_terrain_footsteps(rng);
    if proc.s_link() >= 9 && !f.commands.graphics.is_empty() {
        // efAsync_Spawn (800679B0): link >=9 dispatches each command now,
        // before the next graphics command draws its three random offsets.
        effects.flush::<melee_ft::fighter::RetailTrig>(
            melee_ef::EffectTiming::BeforeGraphics,
            player,
            &mut f.core,
            &scene_assets.common_particle_bank,
            &scene_assets.particle_bank,
            particles,
            rng,
        )?;
        let pending = std::mem::take(&mut f.effects);
        for command in f.commands.graphics.take_all() {
            f.commands.graphics.push(command);
            f.resolve_graphics_commands(assets, rng);
            // A command whose joint was read when it ran (it carries the
            // matrix) is dispatched as created; the rest read it now.
            effects.flush::<melee_ft::fighter::RetailTrig>(
                melee_ef::EffectTiming::Immediate,
                player,
                &mut f.core,
                &scene_assets.common_particle_bank,
                &scene_assets.particle_bank,
                particles,
                rng,
            )?;
            effects.flush::<melee_ft::fighter::RetailTrig>(
                melee_ef::EffectTiming::Deferred,
                player,
                &mut f.core,
                &scene_assets.common_particle_bank,
                &scene_assets.particle_bank,
                particles,
                rng,
            )?;
        }
        f.effects = pending;
    } else {
        // Fighter_ChangeMotionState flushed efAsync when this proc changed
        // motion, before the new script's graphics or landing dust drew
        // their random offsets: dispatch what that entry sealed.
        resolve_issued_graphics(f, player, scene_assets, effects, particles, rng)?;
    }
    effects.flush::<melee_ft::fighter::RetailTrig>(
        melee_ef::EffectTiming::Immediate,
        player,
        &mut f.core,
        &scene_assets.common_particle_bank,
        &scene_assets.particle_bank,
        particles,
        rng,
    )?;
    f.effect_state.dispatch_at_once = false;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use hsd_gobj::World;
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
        assert!(validate_saved_resume(&resume, &stage(), 2, 2).is_ok());
        resume.current = Some((ProcKey { object: 9, ..key }, Continuation::Invoke));
        assert!(validate_saved_resume(&resume, &stage(), 2, 2).is_err());
    }
    #[test]
    fn m3_proc_order() {
        let rows = registrations(&stage(), 2, 2);
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
            // p_link 18 follows the fighters: the quake models at s_link 1
            // (grLib_801C9CEC) and the camera at s_link 18 (Camera_Create).
            if phase == 1 {
                expected.push((1, 18, 1, Callback::Quakes));
            }
            if phase == 18 {
                expected.push((18, 18, 0, Callback::Camera));
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
    use std::{fs::File, io::BufRead, path::Path};
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
        let expected =
            melee_diff::read_trace(melee_test_support::trace::open(&path).unwrap()).unwrap();
        let metadata: Vec<serde_json::Value> =
            melee_test_support::trace::open(&scenario.trace_path("particles.jsonl.meta.jsonl"))
                .unwrap()
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
mod recovery;
#[cfg(test)]
mod reflection;

#[cfg(test)]
mod item_continuation;

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
mod partner_fighters;
mod transformation;
#[cfg(test)]
mod yoshi_state;

pub mod rendered_pose;

#[cfg(test)]
mod fd_background;

#[cfg(test)]
struct TestSimulation {
    engine: Simulation,
    pads: melee_sim::inputs::PadScript,
}
#[cfg(test)]
impl TestSimulation {
    fn with_inputs(state: InitialState, pads: melee_sim::inputs::PadScript) -> Self {
        Self {
            engine: Simulation::new(state),
            pads,
        }
    }
    fn tick_without_snapshot(&mut self) -> Result<()> {
        self.engine
            .set_inputs(self.pads.samples(self.engine.frame()));
        self.engine.tick_without_snapshot()
    }
    fn tick(&mut self) -> Result<Record> {
        self.tick_without_snapshot()?;
        Ok(crate::diagnostics::snapshot(
            self.engine.state(),
            self.engine.frame() - 1,
        ))
    }
}
#[cfg(test)]
impl std::ops::Deref for TestSimulation {
    type Target = Simulation;
    fn deref(&self) -> &Self::Target {
        &self.engine
    }
}
#[cfg(test)]
impl std::ops::DerefMut for TestSimulation {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.engine
    }
}

#[cfg(test)]
mod api_fault_tests {
    use crate::*;
    #[test]
    fn simulation_errors_and_panics_fault_without_counting_failed_ticks() {
        let files =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files");
        if !melee_test_support::require_files([files.join("PlCo.dat")]) {
            return;
        }
        let config = MatchConfig::versus(
            Stage::FinalDestination,
            [
                PlayerConfig::new(Port::P1, Character::Fox),
                PlayerConfig::new(Port::P2, Character::Marth),
            ],
        )
        .with_seed(Seed(42));
        let assets = GameAssets::load(files, &config).unwrap();
        let mut healthy = Match::new(&assets, config).unwrap();
        for _ in 0..100 {
            healthy.step(&Inputs::default()).unwrap();
        }
        for panic in [false, true] {
            let mut game = healthy.clone();
            game.engine.runtime.state.fighters[0].0.motion_row.anim = if panic {
                melee_ft::fighter::state::unimplemented_anim
            } else {
                |_, _| Err("injected animation failure".into())
            };
            let tick = game.tick();
            assert!(matches!(
                game.step(&Inputs::default()),
                Err(StepError::Simulation(_))
            ));
            assert_eq!(game.tick(), tick);
            assert_eq!(game.status(), MatchStatus::Faulted);
            assert!(matches!(game.observe(), Err(StateError::Faulted)));
            assert_eq!(game.step(&Inputs::default()), Err(StepError::Faulted));
            game.clone_from(&healthy);
            game.step(&Inputs::default()).unwrap();
            assert_eq!(game.tick(), Tick(tick.0 + 1));
        }
    }
}

/// it_8026FE68 (8026FE68): two items' clanking hitboxes met. Each side whose
/// damage, less it_804D6D28 +B4, is below the other's stops: its hit group
/// remembers the other item (it_8026FAC4, which also closes that group to
/// further meetings this pass), xC48 keeps the largest clank damage and the
/// clank spark (0x41C) plays at the midpoint. The victim side goes first.
/// (xCF4, xC38, xCD4 and the knockback direction xCB8 have no reader among
/// the supported items.)
fn item_clank(
    items: &mut melee_it::ItemPool,
    other_index: usize,
    other_hit: usize,
    victim_id: u32,
    victim_hit: usize,
    (victim_point, other_point): (Vec3, Vec3),
    meets: &mut [bool; 4],
) {
    // retail 8026FE9C..: fadds, then fmuls by 0.5.
    let midpoint = Vec3::new(
        0.5 * (other_point.x + victim_point.x),
        0.5 * (other_point.y + victim_point.y),
        0.5 * (other_point.z + victim_point.z),
    );
    let gap = items.common().clank_priority_gap;
    let other_id =items.iter().nth(other_index).expect("listed item").id;
    let (other_damage, victim_damage, other_tag, victim_tag) = {
        let other = items.iter().nth(other_index).expect("listed item");
        let victim = items.iter().find(|i| i.id == victim_id).expect("struck item");
        (
            other.hitboxes[other_hit].as_ref().expect("clanking hit").descriptor.damage,
            victim.hitboxes[victim_hit].as_ref().expect("clanking hit").descriptor.damage,
            other.hitbox_victim(),
            victim.hitbox_victim(),
        )
    };
    let stop = |items: &mut melee_it::ItemPool,
                id: u32,
                hit: usize,
                opponent: u32,
                damage: f32,
                meets: Option<&mut [bool; 4]>| {
        let item = items.get_mut(id).expect("clanking item");
        let mark = melee_it::ItemPool::group_victim_mark(item);
        let group = item.hitboxes[hit].as_ref().expect("clanking hit").descriptor.group;
        // it_8026FA2C: lbColl_80008688 records the opponent on every live
        // hitbox of the group; with chk, a newly recorded one leaves
        // it_804D6D1C.
        if let Some(meets) = meets {
            for (slot, own) in item.hitboxes.iter().enumerate() {
                if own.as_ref().is_some_and(|own| {
                    own.descriptor.group == group && !own.victims.contains(&opponent)
                }) {
                    meets[slot] = false;
                }
            }
        }
        // retail 0x8026FEFC / 0x8027006C: mode 4 (the capsule's rehit
        // timer, after which it forgets the item) when the stopped hitbox's
        // x41_b5 is set, else the permanent mode 3.
        item.record_clank_victim(hit, group, opponent);
        let clank_damage = gekko_math::msl::fctiwz(damage);
        if clank_damage > item.pending_clank_damage {
            item.pending_clank_damage = clank_damage;
        }
        item.events.push(melee_it::ItemEvent::Effect {
            id: CLANK_SPARK,
            position: midpoint,
        });
        if let Some(mark) = &mark {
            items.share_group_victims(id, mark);
        }
    };
    if gekko_math::msl::fctiwz(victim_damage) - gap < gekko_math::msl::fctiwz(other_damage) {
        stop(items, victim_id, victim_hit, other_tag, victim_damage, Some(meets));
    }
    if gekko_math::msl::fctiwz(other_damage) - gap < gekko_math::msl::fctiwz(victim_damage) {
        stop(items, other_id, other_hit, victim_tag, other_damage, None);
    }
}

/// efSync_Spawn 0x41C: the spark of two clanking hitboxes.
const CLANK_SPARK: u16 = 0x41C;

/// it_802706D0 (802706D0): other items' hitboxes against this item, in item
/// list order. An item later in the list first meets this item's own live
/// hitboxes (it_804D6D1C): two clanking hitboxes stop each other by damage
/// (it_8026FE68, [`item_clank`]). Otherwise a contact with the hurt capsules
/// (it_802706D0_sub3) records the victim in the hitter's group, sets the
/// hitter's dealt damage and logs a kind 2 hit for it_80270E30. An inert
/// touch fails closed; the stats and hit sound are not modelled.
/// Each port's team in a team battle without friendly fire, for the
/// items' x20_team_id (their owner's team); empty otherwise.
struct ItemTeams([Option<u8>; 4]);
impl ItemTeams {
    fn of(fighters: &[crate::scene_fighter::SceneFighter]) -> Self {
        let mut teams = [None; 4];
        for f in fighters {
            if !f.player.friendly_fire {
                teams[usize::from(f.player.id)] = f.player.team;
            }
        }
        Self(teams)
    }
    /// itcoll.c:561-563: both items' owners fight on one team.
    fn allied(&self, a: Option<u8>, b: Option<u8>) -> bool {
        let team = |owner: Option<u8>| owner.and_then(|port| self.0[usize::from(port)]);
        team(a).is_some() && team(a) == team(b)
    }
}

fn item_hits_by_items(
    items: &mut melee_it::ItemPool,
    victim_id: u32,
    capsules: &melee_it::hurt::HurtCapsules,
    teams: &ItemTeams,
    log: &mut melee_it::hurt::ItemHitLog,
) {
    use melee_coll::geometry::Capsule;
    use melee_types::{GroundOrAir, HitElement};
    struct Landing {
        hitter: u32,
        hit: usize,
        contact: Vec3,
    }
    let reaches = |hit: &melee_coll::hitbox::HitCapsule, ground: GroundOrAir| {
        let grounded = ground == GroundOrAir::Ground;
        (hit.descriptor.hit_air && !grounded) || (hit.descriptor.hit_ground && grounded)
    };
    let mut landings = melee_types::fixed::FixedVec::<Landing, 16>::default();
    let mut after_victim = false;
    for index in 0..items.len() {
        let (other_id, other_destroyed) = {
            let other = items.iter().nth(index).expect("listed item");
            (other.id, other.destroyed)
        };
        if other_id == victim_id {
            after_victim = true;
            continue;
        }
        if other_destroyed {
            continue;
        }
        let (victim, other) = {
            let victim = items.iter().find(|i| i.id == victim_id).expect("struck item");
            let other = items.iter().nth(index).expect("listed item");
            (victim, other)
        };
        // Items sharing an owner (or both unowned) pass each other unless
        // the hitter reaches kindred items (xDCD b7) or the victim was
        // dropped or thrown (xDCE b2). Two owners are one
        // when their player is (ftLib_80086FD4, itcoll.c:555-560): Popo's
        // and Nana's ice blocks pass each other.
        let kindred = victim.owner == other.owner;
        if kindred && !other.strikes_kindred_items && !victim.hurt_by_owner {
            continue;
        }
        // itcoll.c:561-563: a team battle without friendly fire spares the
        // hitter's team unless the hitter ignores teams (xDCD b6, set with
        // b5) or the victim was let go (xDCE b1, set with b0).
        if teams.allied(victim.owner, other.owner) && !other.hits_owner && !victim.hurt_by_owner {
            continue;
        }
        // it_804D6D1C: this item's hitboxes that may meet the later item's
        // (xDD0 b1, an item grab, is never set by a supported kind).
        let mut meets = [false; 4];
        if after_victim {
            let other_tag = other.hitbox_victim();
            for (slot, own) in victim.hitboxes.iter().enumerate() {
                meets[slot] = own.as_ref().is_some_and(|own| {
                    own.descriptor.element != HitElement::Catch
                        && reaches(own, other.ground_or_air)
                        && !own.victims.contains(&other_tag)
                });
            }
        }
        let tag = victim.hitbox_victim();
        for id in 0..other.hitboxes.len() {
            let (victim, other) = {
                let victim = items.iter().find(|i| i.id == victim_id).expect("struck item");
                let other = items.iter().nth(index).expect("listed item");
                (victim, other)
            };
            let Some(hit) = &other.hitboxes[id] else {
                continue;
            };
            let desc = &hit.descriptor;
            if !other.hit_flags[id].hits_items
                || !reaches(hit, victim.ground_or_air)
                || hit.victims.contains(&tag)
            {
                continue;
            }
            if meets.contains(&true) {
                let mut met = None;
                for (slot, own) in victim.hitboxes.iter().enumerate() {
                    let Some(own) = own.as_ref().filter(|_| meets[slot]) else {
                        continue;
                    };
                    let inert = desc.element == HitElement::Inert
                        || own.descriptor.element == HitElement::Inert;
                    let clank = !inert && desc.clank && own.descriptor.clank;
                    if !(clank || (inert && desc.element != own.descriptor.element)) {
                        continue;
                    }
                    // lbColl_80007AFC(hit, own, item->scl, arg_item->scl)
                    // passes its second capsule first (see clank.rs).
                    let Some(points) = melee_coll::geometry::hitbox_pair_contact(
                        Capsule {
                            start: own.previous_position,
                            end: own.position,
                            radius: own.descriptor.radius * victim.scale,
                        },
                        Capsule {
                            start: hit.previous_position,
                            end: hit.position,
                            radius: desc.radius * other.scale,
                        },
                    ) else {
                        continue;
                    };
                    if inert {
                        unimplemented!("it_802706D0: an inert item hitbox touching another item's");
                    }
                    met = Some((slot, points));
                    break;
                }
                if let Some((slot, points)) = met {
                    item_clank(items, index, id, victim_id, slot, points, &mut meets);
                    continue;
                }
            }
            if capsules.is_empty() {
                continue;
            }
            let (victim, other) = {
                let victim = items.iter().find(|i| i.id == victim_id).expect("struck item");
                let other = items.iter().nth(index).expect("listed item");
                (victim, other)
            };
            let Some(hit) = &other.hitboxes[id] else {
                continue;
            };
            let desc = &hit.descriptor;
            // arg_item->xD0C == 2: the hurt capsules take no hits.
            if victim.hurt_intangible {
                continue;
            }
            assert_ne!(
                desc.element,
                HitElement::Inert,
                "it_802706D0: an inert item hitbox"
            );
            let contact = capsules.iter().find_map(|capsule| {
                melee_coll::geometry::capsule_contact(
                    Capsule {
                        start: hit.previous_position,
                        end: hit.position,
                        radius: desc.radius * other.scale,
                    },
                    Capsule {
                        start: capsule.start,
                        end: capsule.end,
                        radius: capsule.radius,
                    },
                    &capsule.matrix,
                    3.0 * victim.scale,
                )
            });
            // it_802706D0_sub3 -> it_8026FAC4 records the victim in every
            // hitbox of the landing one's group at once, so a later hitbox
            // of that group no longer reaches it this frame.
            let group = desc.group;
            let landed = landings.iter().any(|l| {
                l.hitter == other.id
                    && other.hitboxes[l.hit]
                        .as_ref()
                        .is_some_and(|h| h.descriptor.group == group)
            });
            if let Some(contact) = contact.filter(|_| !landed) {
                landings.push(Landing {
                    hitter: other.id,
                    hit: id,
                    contact: contact.position,
                });
            }
        }
    }
    let tag = items
        .get_mut(victim_id)
        .expect("struck item")
        .hitbox_victim();
    for landing in landings.iter() {
        let hitter = items.get_mut(landing.hitter).expect("hitting item");
        let mark = melee_it::ItemPool::group_victim_mark(hitter);
        let hit = hitter.hitboxes[landing.hit].clone().expect("landing hit");
        let desc = &hit.descriptor;
        // it_8026FAC4 -> it_8026FA2C: the hit's group remembers the victim.
        melee_coll::detection::record_victim(&mut hitter.hitboxes, desc.group, tag);
        if let Some(mark) = &mark {
            items.share_group_victims(landing.hitter, mark);
        }
        let hitter = items.get_mut(landing.hitter).expect("hitting item");
        let damage = gekko_math::msl::fctiwz(desc.damage);
        hitter.pending_damage_dealt = damage;
        let source = melee_it::hurt::ItemHitSource::Item {
            owner: hitter.owner,
            owner_secondary: hitter.owner_secondary,
            position: hitter.position,
            velocity_x: hitter.velocity.x,
        };
        let victim = items.get_mut(victim_id).expect("struck item");
        victim.pending_damage_taken += damage;
        if damage > victim.largest_damage_taken {
            victim.largest_damage_taken = damage;
        }
        log.push(melee_it::hurt::ItemHit {
            source,
            damage: desc.damage,
            angle: desc.angle,
            growth: desc.growth,
            weight_knockback: desc.weight_knockback,
            base_knockback: desc.base_knockback,
            element: desc.element,
            contact: landing.contact,
        });
    }
}

#[cfg(test)]
#[path = "frame/barrel_owner_tests.rs"]
mod barrel_owner_tests;

#[cfg(test)]
mod kongo_barrel_tests;

#[cfg(test)]
mod kongo_log_tests;

#[cfg(test)]
mod revival_floor_tests;

#[cfg(test)]
mod script_color_tests;

#[cfg(test)]
mod item_script_tests;
