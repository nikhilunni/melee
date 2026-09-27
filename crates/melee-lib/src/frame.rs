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
    stage_objects: [Option<hsd_gobj::GObjId>; 10],
    state: InitialState,
    /// The pad each port consumed per tick: scenario input, never state.
    pads: [PadSample; 4],
    frame: u64,
    error: Option<std::sync::Arc<str>>,
    /// Diagnostic only: values observed around procs, never gameplay inputs.
    rng_writers: Vec<(Option<Callback>, u32)>,
    particle_draws: DrawLog,
    interface: [melee_if::PercentDisplay; 2],
    match_finished: bool,
    /// Scenario input: retail rendered (and so ran particleSort) between the
    /// previous tick and this one. Live play renders every tick; a recording
    /// that ran behind executes several ticks per VI frame and renders once.
    display_pass: bool,
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
            frame: self.frame,
            error: self.error.clone(),
            rng_writers: hsd_types::storage::clone_vec(&self.rng_writers),
            particle_draws: self.particle_draws.clone(),
            interface: self.interface.clone(),
            match_finished: self.match_finished,
            display_pass: self.display_pass,
        }
    }
}

impl Runtime {
    /// Item requests made during a proc, in request order: effects spawn and
    /// draw now; efAsync requests below s_link 9 wait for the item's link 9
    /// flush (Item_80269A9C -> efAsync_QueueFlush), which `flush` performs.
    fn drain_item_events(&mut self, flush: bool) -> Result<()> {
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
                    melee_it::ItemEvent::HitSpark { position, damage } => {
                        state.effects.spawn_item_hit_spark::<RetailTrig>(
                            position,
                            damage,
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
                    melee_it::ItemEvent::ScriptEffect(graphics) => {
                        // it_80278800: the offset's random spread, x then y then z.
                        let mut offset = graphics.offset;
                        let range = graphics.range;
                        offset.x += 2.0 * range.x * (state.rng.randf() - 0.5);
                        offset.y += 2.0 * range.y * (state.rng.randf() - 0.5);
                        offset.z += 2.0 * range.z * (state.rng.randf() - 0.5);
                        let kind = match graphics.id {
                            // block_680 / 6B4 / 6E8: efAsync EF_SPAWN_CAMERA_SHAKE.
                            0x513 => 2,
                            0x514 => 3,
                            0x515 => 4,
                            id => anyhow::bail!("it_80278800: item effect {id:#x}"),
                        };
                        // efAsync_Spawn below s_link 9 queues on the item.
                        item.queued_events.push(melee_it::ItemEvent::Quake {
                            kind,
                            joint: graphics.bone,
                            offset,
                        });
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
                        state.quakes.request(&mut state.camera, kind);
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
                }
            }
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
        let attack = item.stale_source;
        let reflected_owner = item.pending_reflection.and_then(|p| {
            if p.preserve_owner {
                item.owner
            } else {
                Some(p.owner)
            }
        });
        let stale_for = |slot| {
            state
                .fighters
                .iter()
                .position(|f| Some(f.player.id) == slot)
                .map_or(1.0, |i| {
                    state.fighters[i].combat.stale.multiplier_for(
                        attack.map(|a| a.move_id),
                        &state.assets.fighters[i].stale_weights,
                    )
                })
        };
        let current_stale = stale_for(owner_slot);
        let reflected_stale = stale_for(reflected_owner);
        item.stale_multiplier = current_stale;
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
                let holder = match (holder_index, held_part) {
                    (Some(index), Some(part)) => {
                        Some(state.fighters[index].0.core.item_holder(part))
                    }
                    _ => None,
                };
                state.items.animate::<SceneItems>(
                    id,
                    state.assets.items.get(kind),
                    owner.as_ref(),
                    holder,
                    &mut state.map,
                );
                // Item_8026A848 -> ftCommon_8007E6DC: the hand lets go.
                let released = state.items.get_mut(id).is_none_or(|item| !item.held);
                if let (Some(index), true) = (holder_index, released) {
                    state.fighters[index]
                        .0
                        .core
                        .release_held_item(id, &state.assets.fighters[index]);
                }
            }
            4 => state.items.physics::<SceneItems>(
                id,
                owner.as_ref(),
                &melee_it::ItemBounds {
                    left: state.assets.arena.left,
                    right: state.assets.arena.right,
                    bottom: state.assets.arena.bottom,
                },
                state.assets.items.get(kind),
            ),
            5 => {
                let contact = state.items.stage_contact(id, &mut state.map);
                state.items.collide::<SceneItems>(
                    id,
                    contact,
                    &mut state.map,
                    state.assets.items.get(kind),
                );
            }
            9 => {
                let item = state.items.get_mut(id).unwrap();
                // Item_80269A9C: hitlag skips the accessory callback.
                let in_hitlag = item.in_hitlag;
                if let melee_it::ItemScratch::Held(held) = &mut item.scratch {
                    if held.shot_pending && !in_hitlag {
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
            11 => {
                let item = state.items.get_mut(id).unwrap();
                item.update_hitboxes();
                item.decay_reflection_history();
            }
            14 => state.items.process_events_with_stale::<SceneItems>(
                id,
                reflected_stale,
                state.assets.items.get(kind),
            ),
            13 => self.detect_item_hurts(id),
            12 | 16 => {}
            _ => unreachable!(),
        }
        Ok(())
    }
    /// Item_80269C5C (item link 13): it_802703E8 lands fighter hitboxes on
    /// the item's hurt capsules, fighters in list order; it_802706D0 (other
    /// items' hitboxes) is unported and fails closed; it_80270E30 resolves.
    fn detect_item_hurts(&mut self, id: u32) {
        let state = &mut self.state;
        let Some(item) = state.items.get_mut(id) else {
            return;
        };
        let assets = state.assets.items.get(item.kind);
        let capsules = item.hurt_capsules(assets);
        if capsules.is_empty() {
            return;
        }
        let mut log = melee_it::hurt::ItemHitLog::default();
        for fighter in state.fighters.iter_mut() {
            let hits =
                crate::scene_fighter::with_fighter!(fighter, |f| f.strike_item(item, &capsules));
            for hit in hits {
                log.push(hit);
            }
        }
        let victim = state
            .items
            .iter()
            .find(|item| item.id == id)
            .expect("struck item");
        item_hits_by_items(state.items.iter(), victim, &capsules);
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
        let state = &mut self.state;
        let state_pads = &self.pads;
        match row.callback {
            Callback::Item { id, phase } => {
                self.dispatch_item(id, phase)?;
                self.drain_item_events(phase == 9)?;
            }
            Callback::Banner => {
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
                            // ifStatus_802F6EA4(4, ..., fn_8016B784): GO.
                            // Its GObj joins this s_link 0 pass after the
                            // countdown's, so it takes its first step now.
                            let mut go = state.go_banner.take().expect("preloaded GO banner");
                            go.start();
                            assert!(!go.tick(), "GO banner ended on its first step");
                            state.banner = Some(go);
                        }
                        BannerKind::Go => {
                            // fn_8016B784: the HUD, and with it the match clock.
                            state.clock.hud_enabled = true;
                            state.banner = None;
                        }
                    }
                }
            }
            Callback::Camera => {
                let [a, b] = &mut state.fighters;
                // cm_804D6468 runs newest first: player 2's subject, then player 1's.
                let mut subjects = [&mut b.0.camera, &mut a.0.camera];
                state
                    .camera
                    .update_standard(&mut subjects, &state.assets.stage_camera);
                let unzoomed = state.camera.zoom() == 1.0;
                for fighter in &mut state.fighters {
                    fighter.0.offscreen.camera_unzoomed = unzoomed;
                }
            }
            Callback::Quakes => state.quakes.animate(&mut state.camera),
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
                    grab_pairs::align(state, player)?;
                }
                if proc == FighterProc::Map {
                    grab_pairs::map_capture(state, player)?;
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
                            )
                        }));
                    }
                    for item in state.items.iter_mut() {
                        let owner = state
                            .fighters
                            .iter()
                            .position(|f| Some(f.player.id) == item.owner);
                        let hit = with_fighter!(&mut state.fighters[player], |f| {
                            f.detect_item_hit(item, &assets.fighters[player])
                        });
                        if let Some(contact) = hit {
                            if contact.logged_damage {
                                if let (Some(owner), Some(attack)) = (owner, item.stale_source) {
                                    state.fighters[owner].combat.stale.record_attack(attack);
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
                        let wind = match &state.stage {
                            SceneStage::Pupupu(stage) => stage.wind_at(f.physics.position),
                            _ => Vec3::ZERO,
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
                if offers_items {
                    crate::scene_fighter::with_fighter!(&mut state.fighters[player], |f| f
                        .core
                        .pickup_candidates
                        .withdraw());
                }
                if proc == FighterProc::Map {
                    crate::scene_fighter::with_fighter!(&mut state.fighters[player], |f| f
                        .core
                        .ledge_holders
                        .withdraw());
                }
                if let Some(kind) = state.fighters[player].0.quake_request.take() {
                    state.quakes.request(&mut state.camera, kind);
                }
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
                    crate::scene_fighter::with_fighter!(&mut state.fighters[index], |f| f
                        .release_from_dead_partner(&state.assets.fighters[index]))
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?;
                }
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
                                    stage_camera: &state.assets.stage_camera,
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
                if proc == FighterProc::ProcessHit {
                    credit_phantom_source(state, player);
                }
                if proc == FighterProc::Accessories {
                    grab_pairs::accessory(state, player)?;
                    crate::scene_fighter::with_fighter!(&mut state.fighters[player], |f| {
                        f.update_revival_platform();
                        // Fighter_CallAcessoryCallbacks_8006C624: hitlag
                        // (x2219_b5) runs accessory3 instead of accessory1.
                        if f.combat.thrown_pose.is_some() && f.combat.hitlag_remaining == 0.0 {
                            f.thrown_accessory(&state.assets.fighters[player]);
                        }
                    });
                }
                if proc == FighterProc::Animation {
                    grab_pairs::escape(state, player)?;
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
                            &state.assets.items,
                            &mut state.map,
                            world,
                            &mut self.item_objects,
                            position,
                            facing,
                        );
                        self.drain_item_events(false)?;
                    }
                }
                0x801C461C | 0x801CADBC | 0x801C1D38 => {}
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
                    if address == 0x8021_AAB0 {
                        self.radial_forces.tick();
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
                    } else if matches!(state.stage, SceneStage::Pupupu(_)) {
                        crate::scene_stage::pupupu::run_proc(
                            state,
                            map_id,
                            &mut self.particle_draws,
                        )?;
                    } else if state.stage.run_stage_proc(map_id, &mut state.rng)? {
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
                let percent = with_fighter!(&state.fighters[player], |f| f.physics.percent);
                let dead =
                    crate::scene_fighter::with_fighter!(&state.fighters[player], |f| matches!(
                        &f.state_data,
                        melee_ft::fighter::MotionData::Life(life) if life.stock_lost()
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
        for (slot, fighter) in state.fighters.iter_mut().enumerate() {
            crate::scene_fighter::with_fighter!(fighter, |f| {
                while !f.item_requests.is_empty() {
                    let mut request = f.item_requests.remove(0);
                    match &mut request {
                        melee_it::ItemRequest::Spawn(spawn)
                        | melee_it::ItemRequest::SpawnHeld(spawn)
                        | melee_it::ItemRequest::SpawnLaser { spawn, .. } => {
                            spawn.stale_source = f.combat.stale.attack()
                        }
                        melee_it::ItemRequest::Control { .. }
                        | melee_it::ItemRequest::PickUp { .. }
                        | melee_it::ItemRequest::Throw { .. }
                        | melee_it::ItemRequest::Drop { .. }
                        | melee_it::ItemRequest::Destroy { .. } => {}
                    }
                    let owner = matches!(request, melee_it::ItemRequest::SpawnHeld(_))
                        .then(|| f.item_owner(&state.assets.fighters[slot]));
                    crate::scene_items::request(
                        &mut state.items,
                        &state.assets.items,
                        &mut state.map,
                        world,
                        &mut self.item_objects,
                        request,
                        crate::scene_items::RequestOwner {
                            slot: Some(f.player.id),
                            held_item: owner.as_ref(),
                            stale_multiplier: f
                                .combat
                                .stale
                                .multiplier(&state.assets.fighters[slot].stale_weights),
                        },
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
/// One joint's local transform, as the retail bone dump records it.
#[derive(Clone, Copy, Debug)]
pub struct LocalSrt {
    pub rotate: [f32; 4],
    pub scale: [f32; 3],
    pub translate: [f32; 3],
    /// JOBJ_USE_QUATERNION; otherwise `rotate[3]` is unused stack data.
    pub quaternion: bool,
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
    pub(crate) fn is_faulted(&self) -> bool {
        self.runtime.error.is_some()
    }
    pub(crate) fn poison(&mut self, error: &str) {
        self.runtime.error = Some(error.into());
    }
    pub fn set_inputs(&mut self, pads: [PadSample; 4]) {
        self.runtime.pads = pads;
    }
    /// Whether a display pass (particleSort) precedes the next tick.
    pub fn set_display_pass(&mut self, rendered: bool) {
        self.runtime.display_pass = rendered;
    }
    pub fn frame(&self) -> u64 {
        self.runtime.frame
    }
    pub fn item_snapshot(&self, frame: u64) -> Record {
        crate::diagnostics::item_snapshot(&self.runtime.state.items, frame)
    }
    /// The stage's camera description (stage_info.cam_info).
    pub fn stage_camera(&self) -> melee_cm::StageCamera {
        self.runtime.state.assets.stage_camera
    }
    /// The particle system in the retail particle-dump format, for
    /// `melee-sim particles-diff`.
    pub fn particle_snapshot(&self, frame: u64) -> Record {
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
        crate::initial_state::particles::snapshot(
            &state.particles,
            state.rng.seed,
            frame,
            &LiveBanks(state),
        )
    }
    /// Every fighter's local joint SRT in part order, for bone-dump oracles.
    pub fn local_poses(&self) -> Vec<Vec<LocalSrt>> {
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
                        LocalSrt {
                            rotate: [
                                joint.rotate.x,
                                joint.rotate.y,
                                joint.rotate.z,
                                joint.rotate.w,
                            ],
                            scale: [joint.scale.x, joint.scale.y, joint.scale.z],
                            translate: [joint.translate.x, joint.translate.y, joint.translate.z],
                            quaternion: joint.flags & hsd_anim::jobj::JOBJ_USE_QUATERNION != 0,
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
        if state.banner.is_some() {
            rows.push(Registration {
                s_link: 0,
                p_link: 14,
                priority: 0,
                object: 0,
                callback: Callback::Banner,
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
        let interface = std::array::from_fn(|player| {
            melee_if::PercentDisplay::new(with_fighter!(&state.fighters[player], |f| f
                .physics
                .percent))
        });
        let mut runtime = Box::new(Runtime {
            radial_forces: Default::default(),
            item_objects: Default::default(),
            stage_objects: Default::default(),
            stage_start_rows: start_rows.clone(),
            state,
            interface,
            match_finished: false,
            display_pass: true,
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
            if runtime.frame != 0 && runtime.display_pass {
                for fighter in &mut runtime.state.fighters {
                    fighter.0.prepare_dynamic_display_caches();
                }
                runtime.state.particles.sort_for_display(7);
                render_cameras(&mut runtime.state);
            }
        }
        let runtime = self.runtime.as_mut();
        // gm_GetFFAOutcome (8016BF74): the supported two-player stock match ends
        // when only one player retains stocks. The pause takes effect next tick.
        runtime.match_finished |= runtime.state.clock.timed_out()
            || runtime.state.fighters.iter().any(|fighter| {
                crate::scene_fighter::with_fighter!(fighter, |f| f.player.stocks == 0)
            });
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
                            _ => melee_gr::last::procs::map_callback(map),
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

/// ftColl_8007BE3C: an expired phantom credits its source fighter's current
/// move (plStale_UpdateStaleMovesFromFighter, ftColl_80076444) while the
/// victim's ProcessHit runs; the victim only records whom to credit.
fn credit_phantom_source(state: &mut crate::initial_state::InitialState, player: usize) {
    use crate::scene_fighter::with_fighter;
    let Some(source) = with_fighter!(&mut state.fighters[player], |f| f
        .combat
        .pending_credit
        .take())
    else {
        return;
    };
    let victim = with_fighter!(&state.fighters[player], |f| f.spawn_number);
    for (slot, fighter) in state.fighters.iter_mut().enumerate() {
        let assets = &state.assets.fighters[slot];
        with_fighter!(fighter, |f| if f.spawn_number == source {
            f.core.credit_hit(victim, assets);
        });
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
        // revival states, all MotionData::Life) and x2220_b7 hide the fighter.
        let drawn = !f.effect_state.invisible
            && !matches!(f.state_data, melee_ft::fighter::MotionData::Life(_));
        f.offscreen.magnified = f.offscreen.outside_camera && drawn;
    }
    // fn_800301D0 -> Camera_8002A4AC, then each fighter's render callback
    // (ftDrawCommon_80080E18 -> ftLib_80086A8C -> Camera_80030CD8).
    let camera = state.camera.render_camera(&state.assets.stage_camera);
    // Camera_800310B8: cm_804D6464's viewing matrix, inverted for the screen KO.
    let copy_view = state
        .camera
        .render_copy_camera(&state.assets.stage_camera)
        .view_matrix();
    let mut inverse_copy_view = copy_view;
    hsd_anim::mtx::mtx_inverse(&copy_view, &mut inverse_copy_view);
    for fighter in &mut state.fighters {
        let f = &mut fighter.0;
        if matches!(
            f.state_data,
            melee_ft::fighter::MotionData::Life(melee_ft::fighter::life::LifeState::ScreenKo(_))
        ) {
            // ftDrawCommon_80080E18: x2220_b7 places the fighter from camera
            // space and ftLib_80086A8C then reports it on screen. A sleeping
            // fighter (x221F_b3) is not drawn at all.
            if !f.status.disabled {
                f.place_screen_ko(&inverse_copy_view);
                f.offscreen.outside_camera = false;
            }
            continue;
        }
        let on_screen = melee_cm::to_screen(&camera, f.camera.bone_position)
            .is_some_and(|point| point.on_screen);
        f.offscreen.outside_camera = !on_screen;
    }
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
    wind: Vec3,
) -> Result<()> {
    let assets = &scene_assets.fighters[player];
    let was_in_hitlag = f.combat.hitlag_remaining > 0.0;
    let had_effect_callbacks = f.effect_state.hitlag_callbacks;
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
            let pad: PadSample = state_pads[usize::from(f.player.id)];
            f.proc_input(assets, &pad)
        }
        FighterProc::Update => {
            f.proc_update(assets, map, wind);
            f.check_blast_zone(assets, &scene_assets.arena, rng)
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        }
        FighterProc::Map => {
            f.proc_map_with_assets(assets, map)
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
            // Fighter_8006C80C: accessory4 runs after efAsync_QueueFlush.
            if !f.status.disabled
                && f.combat.hitlag_remaining == 0.0
                && !f.screen_ko_accessory(scene_assets.stage_camera.bottom())
                && !f.item_throw_accessory(assets)
            {
                f.character_accessory(assets);
            }
            f.proc_hitbox_positions();
        }
        FighterProc::Grab => f.proc_grab(),
        FighterProc::HitDetection => f.proc_hit_detection(),
        FighterProc::ProcessHit => f.proc_process_hit(assets, rng),
        FighterProc::Dynamics => f.proc_dynamics_with_forces(map, radial_forces.fields()),
        FighterProc::Camera => f.proc_camera_with_map(assets, &scene_assets.stage_camera, map),
        FighterProc::PlayerMirror => f.proc_player_mirror(),
    }
    // Fighter_8006D044/8006D10C invoke the installed callbacks on transitions,
    // not every frozen frame. New models created later are not retroactively paused.
    if proc == FighterProc::Status
        && was_in_hitlag
        && f.combat.hitlag_remaining == 0.0
        && had_effect_callbacks
    {
        effects.set_owner_hitlag(player, false);
    }
    if proc == FighterProc::ProcessHit
        && !was_in_hitlag
        && f.combat.hitlag_remaining > 0.0
        && f.effect_state.hitlag_callbacks
    {
        effects.set_owner_hitlag(player, true);
    }
    // ftAction_80073118 / ftCo_8009E714: literal bone, rounded fixed-point
    // operands; queue lifetime is owned by the scene's ground controller.
    for wind in std::mem::take(&mut f.commands.wind_effects) {
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
        radial_forces.insert(melee_lb::radial_force::RadialImpulse {
            center,
            frames: if wind.timer < 0 {
                120
            } else {
                i32::from(wind.timer)
            },
            strength: 0.003906 * f32::from(wind.magnitude),
            decay: 0.003906 * f32::from(wind.decay),
            phase_step: 0.003906 * f32::from(wind.angle),
        });
    }
    for impulse in std::mem::take(&mut f.commands.radial_impulses) {
        radial_forces.insert(impulse);
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
    // S3: opcode 38 owns this draw before the following effect boundary.
    f.resolve_random_sound_commands(rng);
    if proc.s_link() >= 9 && !f.commands.graphics.is_empty() {
        // efAsync_Spawn (800679B0): link >=9 dispatches each command now,
        // before the next graphics command draws its three random offsets.
        effects.flush::<melee_ft::fighter::RetailTrig>(
            melee_ef::EffectTiming::BeforeGraphics,
            player,
            &mut f.core,
            &scene_assets.common_particle_bank,
            particles,
            rng,
        )?;
        let pending = std::mem::take(&mut f.effects);
        for command in std::mem::take(&mut f.commands.graphics) {
            f.commands.graphics.push(command);
            f.resolve_graphics_commands(assets, rng);
            effects.flush::<melee_ft::fighter::RetailTrig>(
                melee_ef::EffectTiming::Deferred,
                player,
                &mut f.core,
                &scene_assets.common_particle_bank,
                particles,
                rng,
            )?;
        }
        f.effects = pending;
    } else {
        if !f.commands.graphics.is_empty() || !f.commands.landing_effects.is_empty() {
            // Fighter_ChangeMotionState flushed efAsync (fighter.c:951) when
            // this proc changed motion, before the new script's graphics or
            // landing dust drew their random offsets: dispatch what that
            // entry sealed.
            effects.flush::<melee_ft::fighter::RetailTrig>(
                melee_ef::EffectTiming::Sealed,
                player,
                &mut f.core,
                &scene_assets.common_particle_bank,
                particles,
                rng,
            )?;
        }
        f.resolve_graphics_commands(assets, rng);
    }
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
mod recovery;
#[cfg(test)]
mod reflection;

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
        self.engine.set_inputs(std::array::from_fn(|p| {
            self.pads.sample(self.engine.frame(), p)
        }));
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

/// it_802706D0 (802706D0): other items' hitboxes against this item's hurt
/// capsules. Unported beyond detection: a landing contact fails closed.
fn item_hits_by_items<'a>(
    items: impl Iterator<Item = &'a melee_it::ItemCore>,
    victim: &melee_it::ItemCore,
    capsules: &melee_it::hurt::HurtCapsules,
) {
    if victim.hurt_intangible {
        return;
    }
    let tag = victim.hitbox_victim();
    for other in items.filter(|other| other.id != victim.id && !other.destroyed) {
        // Items sharing an owner (or both unowned) pass each other unless
        // the hitter reaches kindred items (xDCD b7) or the victim was
        // dropped or thrown (xDCE b2). Teams are off.
        let kindred = victim.owner == other.owner;
        if kindred && !other.strikes_kindred_items && !victim.hurt_by_owner {
            continue;
        }
        for (id, hit) in other.hitboxes.iter().enumerate() {
            let Some(hit) = hit else {
                continue;
            };
            let desc = &hit.descriptor;
            let grounded = victim.ground_or_air == melee_types::GroundOrAir::Ground;
            if !other.hit_flags[id].hits_items
                || !((desc.hit_air && !grounded) || (desc.hit_ground && grounded))
                || hit.victims.contains(&tag)
            {
                continue;
            }
            let touches = capsules.iter().any(|capsule| {
                melee_coll::geometry::capsule_contact(
                    melee_coll::geometry::Capsule {
                        start: hit.previous_position,
                        end: hit.position,
                        radius: desc.radius * other.scale,
                    },
                    melee_coll::geometry::Capsule {
                        start: capsule.start,
                        end: capsule.end,
                        radius: capsule.radius,
                    },
                    &capsule.matrix,
                    3.0 * victim.scale,
                )
                .is_some()
            });
            assert!(
                !touches,
                "it_802706D0: an item's hitbox landing on another item"
            );
        }
    }
}
