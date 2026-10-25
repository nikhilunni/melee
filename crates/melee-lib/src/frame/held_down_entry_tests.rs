//! Link and Young Link keep their own down aerial: a held battering item
//! refuses its entry and its bounce restart (the private callbacks are
//! unaudited with an item), while the common row admits one.
use super::*;
use crate::scenario::Scenario;
use melee_types::ItemKind;
use std::path::PathBuf;

fn root() -> PathBuf {
    std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    )
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
                let mut unheld = actual.clone();
                unheld.animation.frame = length + 1.0;
                hit(&mut unheld, &state.assets.fighters[0]);
                assert_eq!(unheld.animation.frame.to_bits(), length.to_bits());
                assert!(!unheld.character.get::<$kind>().specials.down_air.armed);
                assert!(std::ptr::fn_addr_eq(
                    unheld.motion_row.anim,
                    melee_ft::fighter::state::COMMON[S::AttackAirLw as usize].anim
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
    let mut non_sword = common.clone();
    non_sword.held_item.as_mut().unwrap().kind = ItemKind::Dosei;
    motion_refuses(|| {
        non_sword
            .change_motion_state(S::AttackAirLw.into(), &state.assets.fighters[0])
            .unwrap()
    });
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
