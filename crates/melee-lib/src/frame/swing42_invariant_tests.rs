//! Actual followup plus isolated malformed-state, entry-reset and A-release controls.
use super::*;
use crate::scenario::Scenario;
use melee_ft::fighter::{
    item_swing::SwingInput,
    state::{InputPhase, COMMON},
    MotionData,
};
use melee_ft::input::{Buttons, WaitPredicate};
use melee_types::{CommonMotionState as S, ItemKind};
use std::path::PathBuf;

#[test]
fn original_swing_followup_and_real_iasa_invariant_controls() {
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    );
    for (name, actor, frames, captain) in [
        ("sword_swing42_fd_peach_captainfalcon", 1, 520, true),
        ("peach4_pull_sword_swings", 0, 1400, false),
    ] {
        let scenario =
            Scenario::load(&root.join(format!("harness/scenarios/{name}.toml"))).unwrap();
        assert_eq!(scenario.frames, frames);
        assert!(melee_test_support::require_files(scenario.required_files()));
        let pads = crate::trace::pad_script(&scenario)
            .unwrap()
            .with_display_from(&scenario.trace_path("tick.raw.jsonl"))
            .unwrap();
        let mut sim = TestSimulation::with_inputs(
            InitialState::from_savestate_traces(&scenario).unwrap(),
            pads,
        );
        let (mut private_rows, mut transitions, mut swing_rows, mut controls) = (0, 0, 0, 0);
        let mut previous = 0;
        for frame in 0..frames {
            sim.engine.set_display_pass(sim.pads.display_pass(frame));
            sim.tick_without_snapshot().unwrap();
            let state = sim.state();
            let fighter = &state.fighters[actor].0;
            assert_eq!(
                fighter.character.table().swing42_first_row.map(|r| r.0),
                captain.then_some(341)
            );
            let action = fighter.motion_state.action.0;
            if captain && action == 341 {
                private_rows += 1;
                transitions += usize::from(previous == S::SwordSwing4 as u16);
                assert_eq!(fighter.motion_row.id, S::SwordSwing4);
                assert_eq!(fighter.motion_row.animation, 295);
                assert!(
                    matches!(fighter.state_data, MotionData::Swing(s) if s.input == SwingInput::SmashFollowUp)
                );
            }
            previous = action;
            if !fighter.held_item.is_some_and(|h| h.kind == ItemKind::Sword)
                || !matches!(fighter.state_data, MotionData::Swing(_))
            {
                continue;
            }
            swing_rows += 1;
            if !captain {
                assert!(
                    matches!(fighter.state_data, MotionData::Swing(s) if s.input != SwingInput::SmashFollowUp)
                );
            }
            if controls != 0 || action == 341 {
                continue;
            }
            controls += 1;
            let assets = &state.assets.fighters[actor];
            let callback = COMMON[S::SwordSwing4 as usize].iasa;

            // The real IASA consumes b3 even after A was released. Pressing A
            // again cannot restore the entry-owned continuous-hold scratch.
            let mut released = fighter.clone();
            if let MotionData::Swing(s) = &mut released.state_data {
                s.a_held = true;
            }
            released.input.current.held.0 &= !Buttons::A.0;
            released.commands.grab_release = true;
            released.commands.rapid_jab_loop_end = true;
            callback(&mut released, InputPhase { assets });
            assert!(matches!(released.state_data, MotionData::Swing(s) if !s.a_held));
            assert!(!released.commands.grab_release && !released.commands.rapid_jab_loop_end);
            released.input.current.held |= Buttons::A;
            released.commands.grab_release = true;
            callback(&mut released, InputPhase { assets });
            assert_eq!(released.motion_state.action, fighter.motion_state.action);

            let mut requested = fighter.clone();
            requested.input.current.held |= Buttons::A;
            if let MotionData::Swing(s) = &mut requested.state_data {
                s.a_held = true;
            }
            requested.commands.grab_release = true;
            if captain {
                callback(&mut requested, InputPhase { assets });
                assert_eq!(requested.motion_state.action.0, 341);
                assert!(
                    matches!(requested.state_data, MotionData::Swing(s) if s.input == SwingInput::SmashFollowUp)
                );
            } else {
                // Deliberately corrupt a genuine default-None actor's input;
                // this is a local invariant control, not a retail Peach path.
                let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    callback(&mut requested, InputPhase { assets })
                }))
                .unwrap_err();
                let message = panic
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .or_else(|| panic.downcast_ref::<&str>().copied())
                    .unwrap();
                assert!(
                    message.contains("ftswing.c:97: retail Swing42 requires a static motion"),
                    "{message}"
                );
            }

            // A real common ground-input entry replaces a deliberately stale
            // borrowed script/control stack and clears the entire throw word
            // BEFORE stepping the newly selected ordinary smash script.
            let mut reset = fighter.clone();
            reset.commands.borrowed_script =
                Some(std::sync::Arc::from([melee_cmd::Command::GrabRelease]));
            assert!(reset.commands.borrowed_script.is_some());
            reset.commands.script.instruction = Some(usize::MAX);
            reset.commands.script.return_stack.push(usize::MAX);
            reset.commands.script.loops.push(melee_cmd::CommandLoop {
                start: usize::MAX,
                remaining: 2,
            });
            reset.commands.throw_accessory = true;
            reset.commands.move_cue = true;
            reset.commands.throw_reverse = true;
            reset.commands.grab_release = true;
            reset.commands.rapid_jab_loop_end = true;
            reset.input.current.held = Buttons::A;
            reset.input.pressed = Buttons::A;
            reset.input.current.stick.x = 1.0;
            reset.input.current.stick.y = 0.0;
            reset.input.current.cstick = Default::default();
            reset.input.horizontal.tilt = 0;
            reset
                .interrupt_ground(assets, &[WaitPredicate::SmashSide])
                .unwrap();
            assert_eq!(reset.motion_state.action, S::SwordSwing4.into());
            assert!(
                matches!(reset.state_data, MotionData::Swing(s) if s.input == SwingInput::Smash && s.a_held)
            );
            assert!(reset.commands.borrowed_script.is_none());
            assert_ne!(reset.commands.script.instruction, Some(usize::MAX));
            assert!(
                reset.commands.script.return_stack.is_empty()
                    && reset.commands.script.loops.is_empty()
            );
            assert!(
                !reset.commands.throw_accessory
                    && !reset.commands.move_cue
                    && !reset.commands.throw_reverse
                    && !reset.commands.grab_release
                    && !reset.commands.rapid_jab_loop_end
            );
        }
        assert!(
            swing_rows > 0 && controls == 1,
            "{name}: swings={swing_rows} controls={controls}"
        );
        if captain {
            assert!(private_rows > 0 && transitions > 0);
        } else {
            assert!(swing_rows > 0);
        }
        eprintln!("{name}: {frames} complete ticks, swings={swing_rows}, private={private_rows}, transitions={transitions}, controls={controls}");
    }
}
