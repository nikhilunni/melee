//! Full original windows; the terminal birth is not a lifecycle witness.
use super::*;
use crate::scenario::Scenario;
use melee_it::ItemDispatch;
use std::{io::BufRead, path::PathBuf};

const CASES: [&str; 3] = [
    "corpus_v3_kj_fox_marth4_e0fcbde24_p1",
    "corpus_v3_kj_fox_marth4_e75fb4a9a_p0",
    "corpus_v3_kj_fox_marth4_e89a89d0e_p0",
];
fn root() -> PathBuf {
    std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    )
}
fn guard(f: impl FnOnce(), expected: &str) {
    let error = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).unwrap_err();
    let text = error
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| error.downcast_ref::<&str>().copied())
        .unwrap();
    assert!(text.contains(expected), "{text}");
}
fn word(bytes: &str, offset: usize) -> u32 {
    u32::from_str_radix(&bytes[offset * 2..offset * 2 + 8], 16).unwrap()
}

#[test]
fn complete_klaptrap_birth_raw_parent_pose_and_actual_scheduler_order() {
    for name in CASES {
        let scenario =
            Scenario::load(&root().join(format!("harness/scenarios/{name}.toml"))).unwrap();
        assert_eq!(scenario.frames, 1666);
        let raw = scenario.trace_path("tick.raw.jsonl");
        let pads = crate::trace::pad_script(&scenario)
            .unwrap()
            .with_display_from(&raw)
            .unwrap();
        let mut simulation = TestSimulation::with_inputs(
            InitialState::from_savestate_traces(&scenario).unwrap(),
            pads,
        );
        let mut count = 0;
        for (frame, line) in melee_test_support::trace::open(&raw)
            .unwrap()
            .lines()
            .enumerate()
        {
            let row: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
            assert_eq!(row["frame"], frame);
            if frame == 1665 {
                simulation.runtime.callback_trace = Some(Vec::with_capacity(256));
            }
            simulation
                .engine
                .set_display_pass(simulation.pads.display_pass(frame as u64));
            simulation.tick_without_snapshot().unwrap();
            for port in 0..2 {
                let bytes = row["fighters"][port]["bytes"].as_str().unwrap();
                let fighter = &simulation.state().fighters[port].0;
                assert_eq!(u32::from(fighter.motion_state.action.0), word(bytes, 0x10));
                for (actual, offset) in [
                    (fighter.physics.position.x, 0xB0),
                    (fighter.physics.position.y, 0xB4),
                    (fighter.physics.position.z, 0xB8),
                ] {
                    assert_eq!(
                        actual.to_bits(),
                        word(bytes, offset),
                        "{name} tick{frame} p{port}"
                    );
                }
            }
            let live: Vec<_> = simulation
                .state()
                .items
                .iter()
                .filter(|i| !i.destroyed)
                .collect();
            let recorded = row["items"].as_array().unwrap();
            assert_eq!(live.len(), recorded.len(), "{name} tick{frame}");
            for (item, recorded) in live.into_iter().zip(recorded) {
                let bytes = recorded["bytes"].as_str().unwrap();
                assert_eq!(item.kind as u32, word(bytes, 0x10));
                assert_eq!(u32::from(item.motion), word(bytes, 0x24));
                for (actual, offset) in [
                    (item.position.x, 0x4C),
                    (item.position.y, 0x50),
                    (item.position.z, 0x54),
                    (item.velocity.x, 0x40),
                    (item.velocity.y, 0x44),
                    (item.velocity.z, 0x48),
                    (item.facing, 0x2C),
                ] {
                    assert_eq!(
                        actual.to_bits(),
                        word(bytes, offset),
                        "{name} tick{frame} item {:?} +{offset:X}",
                        item.kind
                    );
                }
                if item.kind != melee_types::ItemKind::Klap {
                    continue;
                }
                count += 1;
                assert_eq!(frame, 1665);
                assert_eq!((item.article_state, item.pose_steps), (0, 1));
                for (actual, offset) in [
                    (item.animation_frame, 0x5CC),
                    (item.animation_rate, 0x5D0),
                    (item.script.frame, 0x528),
                    (item.script.timer, 0x524),
                ] {
                    assert_eq!(actual.to_bits(), word(bytes, offset));
                }
                assert!(item.script.instruction.is_none());
                assert_eq!(word(bytes, 0x52C), 0);
                assert!(item.owner.is_none());
                assert_eq!(word(bytes, 0x518), 0);
                assert_eq!(item.collision.as_ref().unwrap().x34_flags.b1234, 3);
                // CollData begins at +378; its x34.b1234 is the same typed
                // projection used by initial_state::collision::restore.
                assert_eq!((word(bytes, 0x3AC) >> 27) & 15, 3);
                assert_eq!(item.life_timer.to_bits(), word(bytes, 0xD44));
                match item.spawn_intangible_frames {
                    Some(timer) => assert_eq!(timer.to_bits(), word(bytes, 0xD40)),
                    None => assert_eq!(word(bytes, 0xD40), 0),
                }
                let flags = word(bytes, 0xDC8);
                for (actual, mask) in [
                    (item.pushed_by_fighters, 8),
                    (item.pushed_by_items, 4),
                    (item.pushed_by_open_palm, 2),
                    (item.pushable, 0x20),
                    (item.grabbable, 0x400),
                    (item.spin_ignores_facing, 0x40),
                ] {
                    assert_eq!(actual, flags & mask != 0, "constructor flag{mask:X}");
                }
                let collision_flags = (word(bytes, 0xDCC) >> 16) & 0xFF;
                assert_eq!(item.hits_owner, collision_flags & 4 != 0);
                assert_eq!(item.strikes_kindred_items, collision_flags & 1 != 0);
                let melee_it::ItemScratch::Monster(base) = &item.scratch else {
                    panic!("monster scratch")
                };
                assert!(base.joint.is_none());
                assert_eq!(word(bytes, 0xDD4), 0);
                assert_eq!(base.joint_index as u32, word(bytes, 0xDD8));
                for (actual, offset) in [
                    (base.floor_normal.x, 0xDDC),
                    (base.floor_normal.y, 0xDE0),
                    (base.floor_normal.z, 0xDE4),
                ] {
                    assert_eq!(actual.to_bits(), word(bytes, offset));
                }
                for (actual, offset) in [
                    (base.turn_frames, 0xDE8),
                    (base.bounces, 0xDEC),
                    (base.last_voice, 0xDF0),
                ] {
                    assert_eq!(actual as u32, word(bytes, offset));
                }
                let parent =
                    (crate::scene_items::SceneItems::logic(item.kind).stage_parent)(item).unwrap();
                assert_eq!(parent.map, 11);
                assert_eq!(simulation.runtime.stage_objects[11], Some(parent.object));
                // Retail +DF4 is the live Ground GObj pointer (it_802E18B4),
                // whereas local GObjAddress numbers model reuse identity, not RAM addresses.
                assert_ne!(word(bytes, 0xDF4), 0);
                assert_ne!(
                    word(bytes, 0xDF4),
                    u32::from_str_radix(
                        recorded["gobj"].as_str().unwrap().trim_start_matches("0x"),
                        16
                    )
                    .unwrap()
                );
                let hit = item.hitboxes[0].as_ref().unwrap();
                // HitCapsule_Unk2 (retail value2) is FirstPosition, as diagnostics/items.rs maps it.
                assert_eq!(hit.phase, melee_coll::hitbox::CapsulePhase::FirstPosition);
                assert_eq!(word(bytes, 0x5D4), 2);
                // it_802790C0 (itanimlist.c:136..148) packs command bytes
                // into HitCapsule x40..43; compare their semantic fields,
                // preserving the unrelated per-capture x43 runtime bits.
                let flags = word(bytes, 0x614);
                let semantic = item.hit_flags[0];
                assert_eq!(u32::from(semantic.rehit_rate), (flags >> 20) & 0xFF);
                for (actual, mask) in [
                    (semantic.reflectable, 0x10000),
                    (semantic.damage_without_hitlag, 0x40000),
                    (semantic.shield_rehit, 0x20000),
                    (semantic.absorbable, 0x8000),
                    (semantic.misses_same_facing, 0x2000),
                    (semantic.shieldable, 0x4000),
                    (semantic.shield_bounce, 0x1000),
                    (semantic.defense_interaction, 0x800),
                    (semantic.hits_hurtboxes, 0x400),
                    (semantic.grabbable_hurtboxes_only, 0x200),
                    (semantic.hits_items, 0x100),
                ] {
                    assert_eq!(actual, flags & mask != 0, "{name} owned hit flag{mask:X}");
                }

                for (actual, offset) in [
                    (hit.descriptor.damage, 0x5E0),
                    (hit.descriptor.offset.x, 0x5E4),
                    (hit.descriptor.offset.y, 0x5E8),
                    (hit.descriptor.offset.z, 0x5EC),
                    (hit.descriptor.radius, 0x5F0),
                ] {
                    assert_eq!(actual.to_bits(), word(bytes, offset));
                }
            }
        }
        assert_eq!(count, 1);
        let item = simulation
            .state()
            .items
            .iter()
            .find(|i| i.kind == melee_types::ItemKind::Klap)
            .unwrap();
        let id = item.id;
        let trace = simulation.runtime.callback_trace.as_ref().unwrap();
        let relevant: Vec<_> = trace
            .iter()
            .filter_map(|r| match r.callback {
                Callback::Stage {
                    map: Some(map),
                    address,
                } if r.s_link == 4 && [5, 10, 11].contains(&map) => Some((map, address)),
                _ => None,
            })
            .collect();
        assert_eq!(
            relevant,
            [
                (10, 0x801C1D38),
                (10, 0x801D557C),
                (5, 0x801C1D38),
                (5, 0x801D577C),
                (11, 0x801C1D38),
                (11, 0x801D6668)
            ]
        );
        let phases: Vec<_> = trace
            .iter()
            .filter_map(|r| match r.callback {
                Callback::Item { id: actual, phase } if actual == id => Some(phase),
                _ => None,
            })
            .collect();
        assert_eq!(phases, [4, 5, 9, 11, 12, 13, 14, 16]);
        let object = simulation
            .runtime
            .item_objects
            .iter()
            .find(|(actual, _)| *actual == id)
            .unwrap()
            .1;
        for proc in simulation
            .world
            .iter_gobj_procs(object)
            .map(|p| simulation.world.proc(p))
        {
            assert_eq!(proc.frame_tag() == 3, [0, 1].contains(&proc.priority()));
        }
        // Read the independently recorded particle-followed DFS1 JObj, not a port-generated expectation.
        let meta = scenario.trace_path("particles.jsonl.meta.jsonl");
        let last = melee_test_support::trace::open(&meta)
            .unwrap()
            .lines()
            .last()
            .unwrap()
            .unwrap();
        let meta: serde_json::Value = serde_json::from_str(&last).unwrap();
        assert_eq!(meta["frame"], 1665);
        let generators = meta["particles"]["generators"].as_array().unwrap();
        let generator = generators
            .iter()
            .find(|g| g["program_kind"] == 30005)
            .unwrap();
        assert_eq!(generator["fields"]["bank"], 30);
        assert_eq!(generator["fields"]["appsrt"], 0);
        let joint = meta["particles"]["joints"]
            .as_array()
            .unwrap()
            .iter()
            .find(|j| j["pointer"] == generator["fields"]["jobj"])
            .unwrap();
        let pose = simulation
            .state()
            .assets
            .items
            .get(item.kind)
            .pose
            .as_ref()
            .unwrap();
        let local = pose.states[0].as_ref().unwrap()[0][1];
        for (actual, key) in [(local.translate, "translation"), (local.scale, "scale")] {
            for (axis, value) in [actual.x, actual.y, actual.z].into_iter().enumerate() {
                assert_eq!(
                    u64::from(value.to_bits()),
                    joint["fields"][key][axis].as_u64().unwrap()
                );
            }
        }
        for (axis, value) in [
            local.rotate.x,
            local.rotate.y,
            local.rotate.z,
            local.rotate.w,
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(
                u64::from(value.to_bits()),
                joint["fields"]["rotation"][axis].as_u64().unwrap()
            );
        }
        let matrix = pose.bone_matrix(item.article_state, item.pose_steps, 1, item.root_srt());
        for (index, value) in matrix.0.into_iter().flatten().enumerate() {
            assert_eq!(
                u64::from(value.to_bits()),
                joint["fields"]["matrix"][index].as_u64().unwrap(),
                "{name} DFS1 matrix{index}"
            );
        }
        let state = simulation.state();
        let mut copy = item.clone();
        let mut map = state.map.clone();
        guard(
            || {
                (crate::scene_items::SceneItems::logic(copy.kind).states[0].animation)(
                    &mut copy,
                    &mut melee_it::ItemAnimationContext {
                        owner: None,
                        holder: None,
                        map: &mut map,
                        assets: state.assets.items.get(item.kind),
                        partner: None,
                        rng: None,
                    },
                );
            },
            "Klaptrap Animation",
        );
        // The ordinary constructor must refuse missing map context before
        // creating a falsely complete live object or allocating a GObj identity.
        let mut pool = state.items.clone();
        let mut gobjs = state.effects.gobjs.clone();
        let before = pool.len();
        let spawn = melee_it::SpawnItem {
            kind: item.kind,
            hold_kind: 4,
            owner: None,
            secondary_owner: None,
            stale_source: None,
            attack_from_spawner: false,
            link_gobjs: 0,
            spawn_variant: 0,
            position: item.position,
            previous_position: item.position,
            velocity: Vec3::ZERO,
            facing: 0.0,
            damage: 0,
            auxiliary_damage: 0,
            spawn_argument: 0,
            initial_collision: true,
            auxiliary_flags: [0; 3],
            ground_or_air: melee_types::GroundOrAir::Air,
            launch_draws: 0,
            launch_seed: None,
        };
        let _ = gobjs.take_events().count();
        guard(
            || {
                pool.spawn::<crate::scene_items::SceneItems>(
                    spawn,
                    state.assets.items.get(item.kind),
                    &mut gobjs,
                );
            },
            "requires live facing context",
        );
        assert_eq!(pool.len(), before);
        assert_eq!(gobjs.take_events().count(), 0);
        let mut slot_rng = gekko_math::HsdRng::new(10_000);
        let mut expected_rng = slot_rng;
        assert_eq!(expected_rng.randi(3), 1);
        guard(
            || {
                melee_gr::kongo::rider::Rider::new(&mut slot_rng);
            },
            "path choice1",
        );
        assert_eq!(
            slot_rng, expected_rng,
            "refusal follows the actual selection draw"
        );
        let SceneStage::Kongo(stage) = &state.stage else {
            unreachable!()
        };
        let mut audio_rider = stage.rider.as_ref().unwrap().clone();
        let spline = &state.assets.stage_desc.splines[audio_rider.spline];
        // Synthetic envelope values trigger the real source predicate on the
        // actual witnessed path, rather than admitting or simulating audio.
        audio_rider.max_y = 100_000.0;
        guard(
            || {
                audio_rider.advance(spline, 0.0, Vec3::new(0.0, 100_000.0, 0.0));
            },
            "Ground_801C5440 audio",
        );
        guard(
            || {
                hsd_anim::spline::bspline_tangent(spline, 1.0);
            },
            "unwitnessed tangent endpoint",
        );
        let mut catch_state = state.clone();
        let SceneStage::Kongo(catch_stage) = &mut catch_state.stage else {
            unreachable!()
        };
        catch_stage.barrel.hold = melee_gr::kongo::barrel::Hold::Empty;
        let catch_position = catch_stage.barrel_position;
        catch_state.items.get_mut(id).unwrap().position = catch_position;
        let mut catch_world = simulation.world.clone();
        let mut catch_objects = simulation.runtime.stage_objects;
        let mut catch_items = simulation.runtime.item_objects.clone();
        guard(
            || {
                crate::scene_stage::kongo::run_proc(
                    &mut catch_state,
                    5,
                    &mut DrawLog::default(),
                    &mut catch_world,
                    &mut catch_objects,
                    &mut catch_items,
                )
                .unwrap();
            },
            "the barrel catches a Klaptrap",
        );
        let logic = crate::scene_items::SceneItems::logic(item.kind);
        let assets = state.assets.items.get(item.kind);
        let bounds = melee_it::ItemBounds {
            left: f32::NEG_INFINITY,
            right: f32::INFINITY,
            bottom: f32::NEG_INFINITY,
            top: f32::INFINITY,
            camera_offset: Default::default(),
        };
        let targets = melee_it::LockOnTargets::default();
        let rng = std::cell::Cell::new(state.rng);
        let context = melee_it::ItemPhysicsContext {
            owner: None,
            targets: &targets,
            assets,
            bounds: &bounds,
            rng: &rng,
        };
        // The captured article loops and has no end. A synthetic finite
        // descriptor exercises the explicit unported transition, without
        // claiming that a recorded birth takes it.
        let mut ended_assets = assets.clone();
        ended_assets.animation_ends[0] = Some(0.0);
        let ended_context = melee_it::ItemPhysicsContext {
            assets: &ended_assets,
            ..context
        };
        copy.animation_frame = 0.0;
        guard(
            || {
                (logic.states[0].physics)(&mut copy, &ended_context);
            },
            "ended animation/Randi100",
        );
        for motion in 1..logic.states.len() {
            guard(
                || {
                    (logic.states[motion].physics)(&mut copy, &context);
                },
                "unwitnessed Klaptrap motion physics",
            );
            guard(
                || {
                    (logic.states[motion].collision)(
                        &mut copy,
                        &mut melee_it::ItemCollisionContext {
                            stage_parent: None,
                            owner: None,
                            stage_contact: false,
                            map: &mut map,
                            assets,
                            bounds: &bounds,
                            rng: Some(&rng),
                            half_life_scale: 0.5,
                            entered_air: None,
                        },
                    );
                },
                "unwitnessed Klaptrap motion collision",
            );
        }
        copy = item.clone();
        let parent = (logic.stage_parent)(item).unwrap();
        guard(
            || {
                (logic.states[0].collision)(
                    &mut copy,
                    &mut melee_it::ItemCollisionContext {
                        stage_parent: Some(melee_it::StageParentView {
                            parent,
                            translation: Vec3::new(0.0, 5.0, 100.0),
                            rotation: Vec3::ZERO,
                        }),
                        owner: None,
                        stage_contact: false,
                        map: &mut map,
                        assets,
                        bounds: &bounds,
                        rng: Some(&rng),
                        half_life_scale: 0.5,
                        entered_air: None,
                    },
                );
            },
            "child Z >= 0",
        );
        let melee_it::ItemScratch::Monster(base) = &mut copy.scratch else {
            unreachable!()
        };
        base.behavior = melee_it::MonsterBehavior::Klaptrap { parent: None };
        guard(
            || {
                (logic.states[0].collision)(
                    &mut copy,
                    &mut melee_it::ItemCollisionContext {
                        stage_parent: None,
                        owner: None,
                        stage_contact: false,
                        map: &mut map,
                        assets,
                        bounds: &bounds,
                        rng: Some(&rng),
                        half_life_scale: 0.5,
                        entered_air: None,
                    },
                );
            },
            "NULL Klaptrap Ground parent",
        );
        guard(
            || {
                (logic.destroyed)(&mut copy);
            },
            "itKlap_Logic10_Destroyed",
        );
        guard(
            || {
                (logic.damage_received)(
                    &mut copy,
                    &melee_it::ItemEventContext::new(assets, state.items.common()),
                );
            },
            "it_2725_Logic10_DmgReceived",
        );
        guard(
            || {
                simulation.tick_without_snapshot().unwrap();
            },
            "Ground_801C1CD0: Klaptrap rider",
        );
    }
}
