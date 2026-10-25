//! Complete common Down Air entry captures; subsequent callbacks are refused.
use super::*;
use crate::scenario::Scenario;
use melee_ft::fighter::state::{AnimationPhase, InputPhase};
use melee_types::ItemKind;
use std::{io::BufRead, path::PathBuf};

fn root() -> PathBuf {
    std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    )
}
fn expect_guard(f: impl FnOnce(), target: &str) {
    let error = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).unwrap_err();
    let text = error
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| error.downcast_ref::<&str>().copied())
        .unwrap();
    assert!(text.contains(target), "{text}");
    assert!(
        text.contains("held Sword aerial entry continuation"),
        "{text}"
    );
}

#[test]
fn complete_down_entries_preserve_raw_owned_state_and_refuse_callbacks() {
    for (name, frames, actor, action) in [
        ("corpus_v3_fd_items_sword_e45a17231_p2", 1634, 1, 69),
        ("corpus_v3_fd_items_sword_e9fa97e1e_p0", 442, 1, 69),
        ("corpus_v3_fd_items_sword_nocont_e0ac13e8e_p1", 1344, 1, 69),
        ("corpus_v3_fd_items_sword_nocont_ecdf8887e_p1", 966, 0, 69),
        ("corpus_v3_fd_items_taru_e5935012b_p2", 1383, 1, 69),
    ] {
        let scenario =
            Scenario::load(&root().join(format!("harness/scenarios/{name}.toml"))).unwrap();
        assert_eq!(scenario.frames, frames);
        let raw = scenario.trace_path("tick.raw.jsonl");
        assert!(melee_test_support::require_files(
            scenario.required_files().into_iter().chain([raw.clone()])
        ));
        let pads = crate::trace::pad_script(&scenario)
            .unwrap()
            .with_display_from(&raw)
            .unwrap();
        let mut simulation = TestSimulation::with_inputs(
            InitialState::from_savestate_traces(&scenario).unwrap(),
            pads,
        );
        let mut previous_identity: Option<(u32, u32, hsd_gobj::GObjAddress)> = None;
        let mut rows = 0;
        let mut entries = 0;
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
            let state = simulation.state();
            let fighter = &state.fighters[actor].0;
            let bytes = row["fighters"][actor]["bytes"].as_str().unwrap();
            let word = |o: usize| u32::from_str_radix(&bytes[o * 2..o * 2 + 8], 16).unwrap();
            assert_eq!(
                u32::from(fighter.motion_state.action.0),
                word(0x10),
                "{name} tick{frame} action"
            );
            assert_eq!(
                fighter.animation.frame.to_bits(),
                word(0x894),
                "{name} tick{frame} frame"
            );
            assert_eq!(
                i32::from(fighter.physics.ground_or_air) as u32,
                word(0xe0),
                "{name} tick{frame} air"
            );
            for (v, o) in [
                (fighter.physics.position.x, 0xb0),
                (fighter.physics.position.y, 0xb4),
                (fighter.physics.position.z, 0xb8),
                (fighter.physics.self_velocity.x, 0x80),
                (fighter.physics.self_velocity.y, 0x84),
                (fighter.physics.self_velocity.z, 0x88),
                (fighter.physics.knockback_velocity.x, 0x8c),
                (fighter.physics.knockback_velocity.y, 0x90),
                (fighter.physics.knockback_velocity.z, 0x94),
                (fighter.physics.facing, 0x2c),
                (fighter.combat.hitlag_remaining, 0x195c),
            ] {
                assert_eq!(v.to_bits(), word(o), "{name} tick{frame} +{o:x}");
            }
            if (65..=69).contains(&fighter.motion_state.action.0) {
                assert_eq!(
                    fighter.commands.allow_interrupt,
                    word(0x2218) & 0x8000_0000 != 0,
                    "{name} tick{frame} active interrupt"
                );
                assert_eq!(
                    fighter.commands.variables[0] as u32,
                    word(0x2200),
                    "{name} tick{frame} active command0"
                );
            }
            assert_eq!(
                fighter.held_item.is_some(),
                word(0x1974) != 0,
                "{name} tick{frame} held handle presence"
            );
            let live = state
                .items
                .iter()
                .filter(|i| !i.destroyed)
                .collect::<Vec<_>>();
            let raw_items = row["items"].as_array().unwrap();
            assert_eq!(live.len(), raw_items.len());
            let mut identity = None;
            for (item, raw_item) in live.iter().zip(raw_items) {
                let hex = raw_item["bytes"].as_str().unwrap();
                let iw = |o: usize| u32::from_str_radix(&hex[o * 2..o * 2 + 8], 16).unwrap();
                assert_eq!(item.kind as u32, iw(0x10));
                assert_eq!(u32::from(item.motion), iw(0x24));
                assert_eq!(item.owner.map(u64::from), raw_item["owner"].as_u64());
                assert_eq!(item.held, iw(0xdc8) & (1 << 12) != 0);
                assert_eq!(item.life_timer.to_bits(), iw(0xd44));
                for (v, o) in [
                    (item.position.x, 0x4c),
                    (item.position.y, 0x50),
                    (item.position.z, 0x54),
                ] {
                    assert_eq!(v.to_bits(), iw(o), "{name} tick{frame} item+{o:x}");
                }
                if item.held && item.kind == ItemKind::Sword {
                    let owner = usize::from(item.owner.unwrap());
                    let owner_bytes = row["fighters"][owner]["bytes"].as_str().unwrap();
                    let ow =
                        |o: usize| u32::from_str_radix(&owner_bytes[o * 2..o * 2 + 8], 16).unwrap();
                    let raw_gobj = u32::from_str_radix(
                        raw_item["gobj"].as_str().unwrap().trim_start_matches("0x"),
                        16,
                    )
                    .unwrap();
                    assert_eq!(iw(0x518), ow(0));
                    assert_eq!(ow(0x1974), raw_gobj);
                    assert_eq!(state.fighters[owner].0.held_item.unwrap().item, item.id);
                    assert_eq!(u32::from(item.holder_part), iw(0xdc4));
                    assert_eq!(
                        item.holder_part,
                        state.assets.fighters[owner]
                            .bones
                            .model
                            .animation_translation
                    );
                    let visible = u8::from_str_radix(&owner_bytes[0x221e * 2..0x221e * 2 + 2], 16)
                        .unwrap()
                        & 0x10
                        != 0;
                    assert_eq!(!state.fighters[owner].0.commands.held_item_hidden, visible);
                    if owner == actor {
                        identity = Some((raw_gobj, item.id, item.gobj));
                    }
                }
            }
            if let (Some(old), Some(current)) = (previous_identity, identity) {
                if old.0 == current.0 {
                    assert_eq!(old, current, "{name} tick{frame} same held identity");
                }
            }
            if frame + 1 == frames as usize {
                entries += 1;
                assert_eq!(fighter.motion_state.action.0, action);
                assert_eq!(fighter.animation.frame, 1.0);
                assert_eq!(word(0x2210), 0);
                assert!(!fighter.commands.allow_interrupt);
                assert!(
                    !fighter.commands.grab_release
                        && !fighter.commands.throw_reverse
                        && !fighter.commands.rapid_jab_loop_end
                );
                assert_eq!(fighter.held_item.unwrap().kind, ItemKind::Sword);
                assert!(matches!(
                    fighter.state_data,
                    melee_ft::fighter::MotionData::Aerial {
                        held_on_entry: true,
                        ..
                    }
                ));
                assert!(identity.is_some());
                assert_eq!(
                    identity, previous_identity,
                    "entry preserves the held identity"
                );
            }
            previous_identity = identity;
            rows += 1;
        }
        assert_eq!((rows, entries), (frames as usize, 1));
        let state = simulation.state();
        for unlocked in [false, true] {
            for animation in [true, false] {
                let mut fighter = state.fighters[actor].0.clone();
                fighter.commands.allow_interrupt = unlocked;
                let mut map = state.map.clone();
                let mut rng = state.rng;
                let saved = (
                    fighter.animation.frame.to_bits(),
                    fighter.animation.remainder.to_bits(),
                    fighter.physics.facing.to_bits(),
                    fighter.combat.hitlag_remaining.to_bits(),
                    format!("{:?}", fighter.commands),
                    fighter.effects.clone(),
                );
                let row = melee_ft::fighter::state::COMMON[action as usize];
                expect_guard(
                    || {
                        if animation {
                            (row.anim)(
                                &mut fighter,
                                AnimationPhase {
                                    assets: &state.assets.fighters[actor],
                                    map: &mut map,
                                    rng: &mut rng,
                                },
                            )
                            .unwrap();
                        } else {
                            (row.iasa)(
                                &mut fighter,
                                InputPhase {
                                    assets: &state.assets.fighters[actor],
                                },
                            );
                        }
                    },
                    if animation {
                        "ftCo_AttackAir_Anim"
                    } else {
                        "ftCo_AttackAir_IASA"
                    },
                );
                assert_eq!(
                    (
                        fighter.animation.frame.to_bits(),
                        fighter.animation.remainder.to_bits(),
                        fighter.physics.facing.to_bits(),
                        fighter.combat.hitlag_remaining.to_bits(),
                        format!("{:?}", fighter.commands),
                        fighter.effects.clone()
                    ),
                    saved
                );
            }
        }
        expect_guard(
            || {
                simulation.tick_without_snapshot().unwrap();
            },
            "ftCo_AttackAir_Anim",
        );
    }
}

fn fixture(name: &str) -> TestSimulation {
    let scenario = Scenario::load(&root().join(format!("harness/scenarios/{name}.toml"))).unwrap();
    assert!(melee_test_support::require_files(scenario.required_files()));
    let pads = crate::trace::pad_script(&scenario)
        .unwrap()
        .with_display_from(&scenario.trace_path("tick.raw.jsonl"))
        .unwrap();
    TestSimulation::with_inputs(
        InitialState::from_savestate_traces(&scenario).unwrap(),
        pads,
    )
}
fn motion_refuses(f: impl FnOnce()) {
    let error = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).unwrap_err();
    let text = error
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| error.downcast_ref::<&str>().copied())
        .unwrap();
    assert!(text.contains("holding an item: motion state"), "{text}");
}
fn select_down(f: &mut melee_ft::fighter::Fighter) {
    f.input.current.cstick = Default::default();
    f.input.previous.cstick = Default::default();
    f.input.current.stick.x = 0.0;
    f.input.current.stick.y = -1.0;
}

#[test]
fn link_down_air_admission_preserves_the_real_refusal_boundary() {
    use melee_ft::fighter::MotionData;
    use melee_types::CommonMotionState as S;
    // Borrow a genuine Sword hand descriptor for isolated refusal controls.
    // This is not a retail mixed Link+Sword continuation witness.
    let mut hand = fixture("corpus_v3_fd_items_sword_e9fa97e1e_p0");
    for frame in 0..442 {
        hand.engine.set_display_pass(hand.pads.display_pass(frame));
        hand.tick_without_snapshot().unwrap();
    }
    let sword = hand.state().fighters[1].0.held_item.unwrap();
    assert_eq!(sword.kind, ItemKind::Sword);
    macro_rules! check_family {
        ($kind:ty, $scene:literal) => {{
            let mut sim = fixture($scene);
            let state = sim.state();
            let assets = &state.assets.fighters[0];
            let mut normal = state.fighters[0].0.clone();
            normal.leave_ground();
            select_down(&mut normal);
            assert!(!normal.character.table().uses_common_down_air_animation);
            assert!(!normal.character.get::<$kind>().specials.down_air.armed);
            let mut refused = normal.clone();
            refused.held_item = Some(sword);
            refused.commands.allow_interrupt = true;
            refused.commands.variables[0] = 99;
            refused.commands.grab_release = true;
            refused.commands.throw_reverse = true;
            refused.commands.rapid_jab_loop_end = true;
            refused.jab_countdown = 7.0;
            let before = (
                refused.motion_state.action,
                refused.animation.frame.to_bits(),
                format!("{:?}", refused.commands.script),
                format!("{:?}", refused.state_data),
            );
            let enter = refused.character.table().enter_aerial;
            motion_refuses(|| enter(&mut refused, assets).unwrap());
            // The existing central check is after common entry's clears and
            // generic jab bookkeeping, before new row/script installation.
            assert!(!refused.commands.allow_interrupt);
            assert_eq!(refused.commands.variables[0], 0);
            assert!(
                !refused.commands.grab_release
                    && !refused.commands.throw_reverse
                    && !refused.commands.rapid_jab_loop_end
            );
            assert_eq!(refused.jab_countdown, 0.0);
            assert_eq!(
                (
                    refused.motion_state.action,
                    refused.animation.frame.to_bits(),
                    format!("{:?}", refused.commands.script),
                    format!("{:?}", refused.state_data)
                ),
                before
            );
            assert!(!refused.character.get::<$kind>().specials.down_air.armed);
            enter(&mut normal, assets).unwrap();
            assert_eq!(normal.motion_state.action, S::AttackAirLw.into());
            assert!(normal.character.get::<$kind>().specials.down_air.armed);
            assert!(matches!(
                normal.state_data,
                MotionData::Aerial {
                    held_on_entry: false,
                    ..
                }
            ));
            let mut controls = 0;
            // The complete unheld recording supplies real active capsules,
            // private callback and false history. Only the isolated clones'
            // late frame below is synthetic: no retail late-bounce claim.
            for frame in 0..480 {
                sim.engine.set_display_pass(sim.pads.display_pass(frame));
                sim.tick_without_snapshot().unwrap();
                let state = sim.state();
                let actual = &state.fighters[0].0;
                let attr = &actual.character.get::<$kind>().attributes.down_air;
                let length = attr.hit_frame_end - attr.hit_frame_start;
                if controls != 0
                    || actual.motion_state.action != S::AttackAirLw.into()
                    || !actual.character.get::<$kind>().specials.down_air.armed
                    || actual.commands.hitboxes[..3].iter().any(Option::is_none)
                {
                    continue;
                }
                assert!(actual.held_item.is_none());
                assert!(matches!(
                    actual.state_data,
                    MotionData::Aerial {
                        held_on_entry: false,
                        ..
                    }
                ));
                let mut held = actual.clone();
                held.animation.frame = length + 1.0;
                held.held_item = Some(sword);
                held.physics.fast_fall = true;
                let before_hits = format!("{:?}", &held.commands.hitboxes[..3]);
                let before_frame = held.animation.frame.to_bits();
                let before_anim = held.motion_row.anim;
                let before_script = format!("{:?}", held.commands.script);
                let hit = held.character.table().deal_damage.unwrap();
                motion_refuses(|| hit(&mut held, &state.assets.fighters[0]));
                // lwOnHit legitimately responds BEFORE requesting the restart.
                assert!(held.commands.hitboxes.iter().all(Option::is_none));
                assert_eq!(
                    format!("{:?}", held.character.get::<$kind>().specials.down_air.hits),
                    before_hits
                );
                assert_eq!(
                    held.physics.self_velocity.y.to_bits(),
                    attr.hit_vertical_speed.to_bits()
                );
                assert!(!held.physics.fast_fall);
                assert_eq!(
                    held.character
                        .get::<$kind>()
                        .specials
                        .down_air
                        .frame_start
                        .to_bits(),
                    attr.hit_frame_start.to_bits()
                );
                assert!(held.character.get::<$kind>().specials.down_air.armed);
                assert_eq!(held.animation.frame.to_bits(), before_frame);
                assert_eq!(format!("{:?}", held.commands.script), before_script);
                assert!(std::ptr::fn_addr_eq(held.motion_row.anim, before_anim));
                assert!(matches!(
                    held.state_data,
                    MotionData::Aerial {
                        held_on_entry: false,
                        ..
                    }
                ));
                let mut unheld = actual.clone();
                unheld.animation.frame = length + 1.0;
                hit(&mut unheld, &state.assets.fighters[0]);
                assert_eq!(unheld.animation.frame.to_bits(), length.to_bits());
                assert!(!unheld.character.get::<$kind>().specials.down_air.armed);
                assert!(std::ptr::fn_addr_eq(
                    unheld.motion_row.anim,
                    melee_ft::fighter::state::COMMON[S::AttackAirLw as usize].anim
                ));
                assert!(matches!(
                    unheld.state_data,
                    MotionData::Aerial {
                        held_on_entry: false,
                        ..
                    }
                ));
                controls += 1;
            }
            assert_eq!(
                controls, 1,
                "{} must supply genuine armed active capsules",
                $scene
            );
        }};
    }
    check_family!(ft_link::init::Link, "links_dair_fd_fox4");
    check_family!(ft_younglink::init::YoungLink, "links_yl_dair_fd_fox4");

    let state = fixture("gamewatch_air_attacks_fd_fox4").state().clone();
    let mut common = state.fighters[0].0.clone();
    common.leave_ground();
    common.held_item = Some(sword);
    select_down(&mut common);
    assert!(common.character.table().uses_common_down_air_animation);
    let enter = common.character.table().enter_aerial;
    enter(&mut common, &state.assets.fighters[0]).unwrap();
    assert_eq!(common.motion_state.action, S::AttackAirLw.into());
    assert!(matches!(
        common.state_data,
        MotionData::Aerial {
            held_on_entry: true,
            ..
        }
    ));
    let mut non_sword = common.clone();
    non_sword.held_item.as_mut().unwrap().kind = ItemKind::Dosei;
    motion_refuses(|| {
        non_sword
            .change_motion_state(S::AttackAirLw.into(), &state.assets.fighters[0])
            .unwrap()
    });
    for state_id in [S::AttackAirN, S::LandingAirLw] {
        let mut private = common.clone();
        motion_refuses(|| {
            private
                .change_motion_state(state_id.into(), &state.assets.fighters[0])
                .unwrap()
        });
    }
    for (x, y) in [(0.0, 0.0), (-1.0, 0.0), (0.0, 1.0)] {
        let mut private = common.clone();
        private.input.current.stick.x = x * private.physics.facing;
        private.input.current.stick.y = y;
        motion_refuses(|| enter(&mut private, &state.assets.fighters[0]).unwrap());
    }
    let state = fixture("peach_float_dair_cstick").state().clone();
    let mut private = state.fighters[0].0.clone();
    private.leave_ground();
    private.held_item = Some(sword);
    select_down(&mut private);
    private.input.pressed = melee_ft::input::Buttons::A;
    private
        .character
        .get_mut::<ft_peach::init::Peach>()
        .float_remaining = 30.0;
    // This private path is BASELINE-existing: Peach's separate
    // SPECIALS_KEEP_HELD_ITEM audit is true. It is not a common69 admission
    // or a new retail held-Peach witness; preserve that separate capability.
    assert!(ft_peach::float_attack::try_enter(&mut private, &state.assets.fighters[0]).unwrap());
    assert_eq!(private.motion_state.action.0, 348);
    assert_eq!(private.motion_row.id, S::None);
    assert_eq!(private.held_item, Some(sword));
}
