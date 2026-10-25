//! Whole terminal crate births. The Goomba's later lifecycle stays guarded; the
//! ReDead's runs on (w25_redead_fall_700_v01 gates it).
use super::*;
use crate::scenario::Scenario;
use melee_it::ItemDispatch;
use std::{io::BufRead, path::PathBuf};

const CASES: [(&str, usize, u32); 4] = [
    ("corpus_v3_fd_items_sword_e266e1150_p1", 1874, 43),
    ("corpus_v3_fd_items_sword_e4abe72ec_p2", 297, 44),
    ("corpus_v3_fd_items_sword_e5ca12c3b_p0", 595, 43),
    ("corpus_v3_fd_items_taru_eae02b66b_p2", 285, 43),
];
fn root() -> PathBuf {
    std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    )
}
fn expect_guard(f: impl FnOnce(), message: &str) {
    let error = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).unwrap_err();
    let text = error
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| error.downcast_ref::<&str>().copied())
        .unwrap();
    assert!(text.contains(message), "{text}");
}

#[test]
fn complete_monster_births_match_raw_and_run_real_birth_procs() {
    // Independently read DOL @210 (804DC808), used by 8027C668 fmul/66C frsp.
    assert_eq!(
        core::f64::consts::FRAC_PI_2.to_bits(),
        0x3FF9_21FB_5444_2D18
    );

    for (name, ticks, kind) in CASES {
        let scenario =
            Scenario::load(&root().join(format!("harness/scenarios/{name}.toml"))).unwrap();
        assert_eq!(scenario.frames as usize, ticks);
        let raw = scenario.trace_path("tick.raw.jsonl");
        let pads = crate::trace::pad_script(&scenario)
            .unwrap()
            .with_display_from(&raw)
            .unwrap();
        let mut simulation = TestSimulation::with_inputs(
            InitialState::from_savestate_traces(&scenario).unwrap(),
            pads,
        );
        let mut births = 0;
        let mut rows = 0;
        for (frame, line) in melee_test_support::trace::open(&raw)
            .unwrap()
            .lines()
            .enumerate()
        {
            let row: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
            assert_eq!(row["frame"], frame);
            let display = simulation.pads.display_pass(frame as u64);
            simulation.engine.set_display_pass(display);
            simulation.tick_without_snapshot().unwrap();
            for port in 0..2 {
                let bytes = row["fighters"][port]["bytes"].as_str().unwrap();
                let word = |o: usize| u32::from_str_radix(&bytes[o * 2..o * 2 + 8], 16).unwrap();
                let fighter = &simulation.state().fighters[port].0;
                assert_eq!(u32::from(fighter.motion_state.action.0), word(0x10));
                for (actual, offset) in [
                    (fighter.physics.position.x, 0xB0),
                    (fighter.physics.position.y, 0xB4),
                    (fighter.physics.position.z, 0xB8),
                ] {
                    assert_eq!(actual.to_bits(), word(offset), "{name} tick{frame} p{port}");
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
            for (item, raw_item) in live.into_iter().zip(recorded) {
                let bytes = raw_item["bytes"].as_str().unwrap();
                let word = |o: usize| u32::from_str_radix(&bytes[o * 2..o * 2 + 8], 16).unwrap();
                assert_eq!(item.kind as u32, word(0x10));
                assert_eq!(u32::from(item.motion), word(0x24));
                if item.hold_kind == 6 {
                    assert_eq!(
                        crate::scene_items::SceneItems::logic(item.kind).states
                            [usize::from(item.motion)]
                        .animation_id as u32,
                        word(0x28)
                    );
                }
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
                        word(offset),
                        "{name} tick{frame} item {:?} +{offset:X}",
                        item.kind
                    );
                }
                if item.hold_kind != 6 {
                    continue;
                }
                births += 1;
                assert_eq!((frame, item.kind as u32), (ticks - 1, kind));
                let flags = word(0xDC8);
                for (actual, mask) in [
                    (item.pushed_by_fighters, 0x8),
                    (item.pushed_by_items, 0x4),
                    (item.pushed_by_open_palm, 0x2),
                    (item.grabbable, 0x400),
                    (item.spin_ignores_facing, 0x40),
                ] {
                    assert_eq!(actual, flags & mask != 0, "{name} constructor flag{mask:X}");
                }
                let collision_flags =
                    u8::from_str_radix(&bytes[0xDCD * 2..0xDCD * 2 + 2], 16).unwrap();
                assert_eq!(item.hits_owner, collision_flags & 0x4 != 0);
                assert_eq!(item.strikes_kindred_items, collision_flags & 0x1 != 0);
                assert_eq!(word(0x518), 0);
                assert!(item.owner.is_none() && item.stale_source.is_none());
                assert_eq!(item.animation_frame.to_bits(), word(0x5CC));
                assert_eq!(item.animation_rate.to_bits(), word(0x5D0));
                assert_eq!(item.life_timer.to_bits(), word(0xD44));
                assert_eq!(item.spawn_intangible_frames.unwrap().to_bits(), word(0xD40));
                assert_eq!(
                    u8::from_str_radix(&bytes[0xB68 * 2..0xB68 * 2 + 2], 16).unwrap(),
                    0
                );
                assert_eq!(word(0x374), 0);
                let melee_it::ItemScratch::Monster(base) = &item.scratch else {
                    panic!("typed monster scratch")
                };
                assert!(base.joint.is_none());
                assert_eq!(word(0xDD4), 0);
                assert_eq!(base.joint_index as u32, word(0xDD8));
                for (actual, offset) in [
                    (base.floor_normal.x, 0xDDC),
                    (base.floor_normal.y, 0xDE0),
                    (base.floor_normal.z, 0xDE4),
                ] {
                    assert_eq!(actual.to_bits(), word(offset));
                }
                for (actual, offset) in [
                    (base.turn_frames, 0xDE8),
                    (base.bounces, 0xDEC),
                    (base.last_voice, 0xDF0),
                ] {
                    assert_eq!(actual as u32, word(offset));
                }
                match &base.behavior {
                    melee_it::MonsterBehavior::Klaptrap { .. } => {
                        panic!("unexpected Klaptrap in crate birth witness")
                    }
                    melee_it::MonsterBehavior::Kuriboh {
                        walking_speed,
                        spawn_timer,
                    } => {
                        assert_eq!(walking_speed.to_bits(), word(0xDF4));
                        assert_eq!(*spawn_timer as u32, word(0xDF8));
                        assert_eq!(item.pose_steps, 0);
                        assert!(item.jumped_on.is_some());
                        assert_eq!(word(0xD30), 0x802D8098);
                        assert_ne!(
                            u8::from_str_radix(&bytes[0xDD0 * 2..0xDD0 * 2 + 2], 16).unwrap()
                                & 0x80,
                            0
                        );
                    }
                    melee_it::MonsterBehavior::ReDead(redead) => {
                        assert!(redead.captive.is_none());
                        assert_eq!(word(0xE0C), 0);
                        assert_eq!(redead.timer as u32, word(0xE1C));
                        assert_eq!(redead.cooldown as u32, word(0xE20));
                        assert_eq!(redead.staggers as u32, word(0xE24));
                        for (actual, offset) in [
                            (redead.hurt_offsets[0].x, 0xE34),
                            (redead.hurt_offsets[0].y, 0xE38),
                            (redead.hurt_offsets[0].z, 0xE3C),
                            (redead.hurt_offsets[1].x, 0xE40),
                            (redead.hurt_offsets[1].y, 0xE44),
                            (redead.hurt_offsets[1].z, 0xE48),
                            (redead.hurt_radius, 0xE4C),
                            (redead.collision_box.top, 0xE50),
                            (redead.collision_box.bottom, 0xE54),
                            (redead.collision_box.right, 0xE58),
                            (redead.collision_box.left, 0xE5C),
                        ] {
                            assert_eq!(
                                actual.to_bits(),
                                word(offset),
                                "ReDead owned snapshot +{offset:X}"
                            );
                        }
                        assert_eq!((item.article_state, item.pose_steps), (1, 1));
                        assert!(item.jumped_on.is_none());
                        assert_eq!(word(0xD30), 0);
                    }
                }
                let object = simulation
                    .runtime
                    .item_objects
                    .iter()
                    .find(|(id, _)| *id == item.id)
                    .unwrap()
                    .1;
                let procs: Vec<_> = simulation
                    .world
                    .iter_gobj_procs(object)
                    .map(|p| simulation.world.proc(p))
                    .collect();
                let mut links: Vec<_> = procs.iter().map(|p| p.priority()).collect();
                links.sort_unstable();
                assert_eq!(links, melee_it::ITEM_PROCESS_LINKS);
                for phase in [14, 16] {
                    let proc = procs.iter().find(|p| p.priority() == phase).unwrap();
                    assert_ne!(
                        proc.frame_tag(),
                        3,
                        "actual born-this-frame phase{phase} executed"
                    );
                }
                for phase in [0, 1, 4, 5, 9, 11, 12, 13] {
                    assert_eq!(
                        procs
                            .iter()
                            .find(|p| p.priority() == phase)
                            .unwrap()
                            .frame_tag(),
                        3,
                        "earlier phases await next ordinary tick"
                    );
                }
            }
            rows += 1;
        }
        assert_eq!((rows, births), (ticks, 1));
        let state = simulation.state();
        let monster = state.items.iter().find(|i| i.hold_kind == 6).unwrap();
        if monster.kind == melee_types::ItemKind::Leadead {
            // The ReDead's lifecycle runs on (w25_redead_fall_700_v01 gates
            // it); only a right-facing birth stays refused.
            let mut positive = monster.clone();
            positive.facing = 1.0;
            let logic = crate::scene_items::SceneItems::logic(positive.kind);
            let assets = state.assets.items.get(positive.kind);
            expect_guard(
                || (logic.facing_constructor.unwrap().after_query)(&mut positive, assets),
                "right-facing ReDead",
            );
            continue;
        }
        let assets = state.assets.items.get(monster.kind);
        let mut item = monster.clone();
        let logic = crate::scene_items::SceneItems::logic(item.kind);
        let bounds = melee_it::ItemBounds {
            left: f32::NEG_INFINITY,
            right: f32::INFINITY,
            bottom: f32::NEG_INFINITY,
            top: f32::INFINITY,
            camera_offset: Default::default(),
        };
        let rng = std::cell::Cell::new(state.rng);
        expect_guard(
            || {
                (logic.states[usize::from(item.motion)].physics)(
                    &mut item,
                    &melee_it::ItemPhysicsContext {
                        owner: None,
                        targets: &Default::default(),
                        assets,
                        bounds: &bounds,
                        rng: &rng,
                    },
                )
            },
            "monster Physics",
        );
        let mut map = state.map.clone();
        expect_guard(
            || {
                (logic.states[usize::from(item.motion)].collision)(
                    &mut item,
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
            "monster Collision",
        );
        for callback in [logic.picked_up, logic.dropped, logic.thrown] {
            expect_guard(
                || {
                    callback(
                        &mut item,
                        &mut melee_it::ItemAnimationContext {
                            owner: None,
                            holder: None,
                            map: &mut map,
                            assets,
                            partner: None,
                            rng: Some(&rng),
                        },
                    )
                },
                "not implemented",
            );
        }
        let context = melee_it::ItemEventContext::new(assets, state.items.common());
        expect_guard(
            || {
                (logic.damage_received)(&mut item, &context);
            },
            "monster damage lifecycle",
        );
        expect_guard(
            || (logic.destroyed)(&mut item),
            "destroyed monster lifecycle",
        );
        if let Some(jumped_on) = item.jumped_on {
            expect_guard(
                || {
                    jumped_on(&mut item);
                },
                "it_802D8098",
            );
        }
        expect_guard(
            || {
                simulation.tick_without_snapshot().unwrap();
            },
            "monster Animation",
        );
    }
}
