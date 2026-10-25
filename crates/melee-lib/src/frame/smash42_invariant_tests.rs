//! Original common-S4 command ownership; injected controls are not new retail paths.
use super::*;
use crate::scenario::Scenario;
use melee_ft::fighter::{
    state::{InputPhase, COMMON},
    Fighter, MotionData,
};
use melee_ft::input::{Buttons, WaitPredicate};
use melee_types::CommonMotionState as S;
use std::{io::BufRead, path::PathBuf};

/// Named schema owns layout; hsd-archive owns byte-order decoding.
struct FighterFields {
    motion: u32,
    animation_frame: u32,
    variables: u32,
}
impl FighterFields {
    fn from_schema() -> Self {
        let schema: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../harness/schema/fighter.generated.json"
        )))
        .unwrap();
        let field = |name: &str, representation: &str| {
            let entry = schema["fields"]
                .as_array()
                .unwrap()
                .iter()
                .find(|f| f["name"] == name)
                .unwrap();
            let actual = entry["ctype"]
                .as_str()
                .unwrap_or_else(|| entry["type"].as_str().unwrap());
            assert_eq!(actual, representation, "raw schema {name}");
            u32::try_from(entry["offset"].as_u64().unwrap()).unwrap()
        };
        Self {
            motion: field("motion_id", "s32"),
            animation_frame: field("cur_anim_frame", "f32"),
            variables: field("cmd_vars", "u32[4]"),
        }
    }
}
fn raw_bytes(hex: &str) -> Vec<u8> {
    assert_eq!(hex.len() % 2, 0);
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

/// Same named external-input contract as melee-sim::inputs::recorded_events.
/// The unit-test library and its dev-cycle's normal library have distinct
/// ExternalEvents types; decode only this input, not any expected game state.
fn recorded_events(expected: &serde_json::Value) -> crate::ExternalEvents {
    let stage_read = match expected.get("events") {
        None => crate::StageRead::Unrecorded,
        Some(events) => {
            if events["stage_read_completed"].as_bool().unwrap() {
                crate::StageRead::Completed
            } else {
                crate::StageRead::InFlight
            }
        }
    };
    crate::ExternalEvents { stage_read }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Owner {
    Link,
    YoungLink,
    Fox,
    Ness,
}
fn invoke(fighter: &mut Fighter, assets: &melee_ft::fighter::assets::FighterAssets) {
    let input = fighter.motion_row.iasa;
    input(fighter, InputPhase { assets });
}
fn fresh_a(fighter: &mut Fighter) {
    fighter.input.current.held = Buttons::A;
    fighter.input.pressed = Buttons::A;
    fighter.input.released = Buttons::default();
}
fn locked(fighter: &Fighter) -> Fighter {
    let mut local = fighter.clone();
    // Isolate the combo predicate from the unchanged unlocked-prefix ordering.
    local.commands.allow_interrupt = false;
    local
}

#[test]
fn original_smash42_owners_preserve_private_ness_and_real_input_controls() {
    let fields = FighterFields::from_schema();
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    );
    for (name, actor, frames, owner) in [
        ("corpus_v3_fd_link_fox4_e9943b4ab_p2", 0, 110, Owner::Link),
        (
            "catchwait_windmill_wall_ps_mewtwo_younglink4",
            1,
            5330,
            Owner::YoungLink,
        ),
        ("fsmashcharge_fd_fox", 0, 300, Owner::Fox),
        ("ness_bat_fd_fox4", 0, 620, Owner::Ness),
    ] {
        let scenario =
            Scenario::load(&root.join(format!("harness/scenarios/{name}.toml"))).unwrap();
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
        let (
            mut rows,
            mut common_rows,
            mut private_rows,
            mut combo_entries,
            mut controls,
            mut private_controls,
        ) = (0, 0, 0, 0, 0, 0);
        let mut previous = None;
        let mut expected_rows = melee_test_support::trace::open(&scenario.expected_path())
            .unwrap()
            .lines();
        let mut completed_reads = 0;
        for (frame, line) in melee_test_support::trace::open(&raw)
            .unwrap()
            .lines()
            .enumerate()
        {
            let row: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
            assert_eq!(row["frame"], frame);
            // Match the normal scripted adapter's complete recorded inputs.
            // In particular Stadium's asynchronous form reads are inputs too.
            let expected: serde_json::Value =
                serde_json::from_str(&expected_rows.next().unwrap().unwrap()).unwrap();
            assert_eq!(expected["frame"], frame);
            let events = recorded_events(&expected);
            if events.stage_read == crate::StageRead::Completed {
                completed_reads += 1;
            }
            simulation.engine.set_external_events(events);
            simulation
                .engine
                .set_raw_sticks(simulation.pads.raw_sticks(frame as u64));
            simulation
                .engine
                .set_recorded_pad_queue_x(simulation.pads.recorded_pad_queue_x(frame as u64));
            simulation
                .engine
                .set_display_pass(simulation.pads.display_pass(frame as u64));
            simulation.tick_without_snapshot().unwrap();
            rows += 1;
            let state = simulation.state();
            let fighter = &state.fighters[actor].0;
            let assets = &state.assets.fighters[actor];
            let bytes = raw_bytes(row["fighters"][actor]["bytes"].as_str().unwrap());
            let raw = hsd_archive::Reader::new(&bytes);
            let action = fighter.motion_state.action.0;
            assert_eq!(
                i32::from(action),
                raw.s32(fields.motion).unwrap(),
                "{name} tick{frame} action"
            );
            let common = (58..=62).contains(&action);
            let private = action == 341 && owner != Owner::Fox;
            let capability = matches!(owner, Owner::Link | Owner::YoungLink);
            assert_eq!(
                fighter.character.table().forward_smash_combo.map(|a| a.0),
                capability.then_some(341)
            );
            if common || private {
                assert!(matches!(fighter.state_data, MotionData::Smash { .. }));
                assert_eq!(
                    fighter.animation.frame.to_bits(),
                    raw.f32(fields.animation_frame).unwrap().to_bits(),
                    "{name} tick{frame} frame"
                );
                // Owned S4/bat command state, not unrelated retained union words.
                assert_eq!(
                    fighter.commands.variables[0],
                    raw.u32(fields.variables).unwrap(),
                    "{name} tick{frame} var0"
                );
            }
            if private {
                private_rows += 1;
                assert_eq!(fighter.motion_row.id, S::None);
                assert_eq!(
                    fighter.motion_row.animation,
                    if owner == Owner::Ness { 62 } else { 295 }
                );
                assert!(!std::ptr::fn_addr_eq(
                    fighter.motion_row.iasa,
                    COMMON[S::AttackS4S as usize].iasa
                ));
                if capability && previous == Some(S::AttackS4S as u16) {
                    combo_entries += 1;
                }
                if owner == Owner::Ness
                    && fighter.commands.variables[0] != 0
                    && private_controls == 0
                {
                    private_controls += 1;
                    // Real Ness private callback: its positive bat variable is
                    // not eligibility for common S4. Only the input is injected.
                    let mut local = locked(fighter);
                    fresh_a(&mut local);
                    let before = (local.motion_state.action, local.animation.frame.to_bits());
                    invoke(&mut local, assets);
                    assert_eq!(
                        (local.motion_state.action, local.animation.frame.to_bits()),
                        before
                    );
                    assert!(local.character.get::<ft_ness::init::Ness>().bat);
                }
            }
            if owner == Owner::YoungLink && frame == 1675 {
                assert_eq!(action, S::Fall as u16);
            }
            previous = Some(action);
            if !common {
                continue;
            }
            common_rows += 1;
            assert!(std::ptr::fn_addr_eq(
                fighter.motion_row.iasa,
                COMMON[S::AttackS4S as usize].iasa
            ));
            if controls != 0 || (capability && fighter.commands.variables[0] == 0) {
                continue;
            }
            controls += 1;
            assert!(fighter.held_item.is_none());
            let mut requested = locked(fighter);
            // The real default-None actor has no authored positive; this one
            // variable injection is explicitly a malformed/custom-state control.
            if !capability {
                requested.commands.variables[0] = 1;
            }
            fresh_a(&mut requested);
            if capability {
                invoke(&mut requested, assets);
                assert_eq!(requested.motion_state.action.0, 341);
                assert_eq!(requested.motion_row.animation, 295);
                assert!(matches!(requested.state_data, MotionData::Smash { .. }));
            } else {
                let error = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    invoke(&mut requested, assets)
                }))
                .unwrap_err();
                let message = error
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .or_else(|| error.downcast_ref::<&str>().copied())
                    .unwrap();
                assert!(
                    message.contains("ftCo_800CED30: retail S4 followup requires a static motion"),
                    "{message}"
                );
            }
            if capability {
                let mut held_only = locked(fighter);
                held_only.input.current.held = Buttons::A;
                held_only.input.pressed = Buttons::default();
                let before = (
                    held_only.motion_state.action,
                    held_only.animation.frame.to_bits(),
                );
                invoke(&mut held_only, assets);
                assert_eq!(
                    (
                        held_only.motion_state.action,
                        held_only.animation.frame.to_bits()
                    ),
                    before
                );

                let mut released = locked(fighter);
                released.input.current.held = Buttons::default();
                released.input.pressed = Buttons::default();
                released.input.released = Buttons::A;
                invoke(&mut released, assets);
                assert_eq!(released.motion_state.action, fighter.motion_state.action);
                // Unlike Swing42 continuous hold, release does not permanently
                // disarm the still-open S4 window: a later edge can select341.
                fresh_a(&mut released);
                invoke(&mut released, assets);
                assert_eq!(released.motion_state.action.0, 341);

                let mut no_window = locked(fighter);
                no_window.commands.variables[0] = 0;
                fresh_a(&mut no_window);
                invoke(&mut no_window, assets);
                assert_eq!(no_window.motion_state.action, fighter.motion_state.action);
            }
            // Real public ground input enters a new ordinary, unheld smash.
            // It must replace a stale borrowed owner/control stack and var0.
            let mut reset = fighter.clone();
            reset.commands.variables[0] = 1;
            reset.commands.borrowed_script =
                Some(std::sync::Arc::from([melee_cmd::Command::SetVariable {
                    index: 0,
                    value: 1,
                }]));
            reset.commands.script.instruction = Some(usize::MAX);
            reset.commands.script.return_stack.push(usize::MAX);
            reset.commands.script.loops.push(melee_cmd::CommandLoop {
                start: usize::MAX,
                remaining: 2,
            });
            assert!(reset.commands.borrowed_script.is_some());
            fresh_a(&mut reset);
            reset.input.current.stick.x = 1.0;
            reset.input.current.stick.y = 0.0;
            reset.input.current.cstick = Default::default();
            reset.input.horizontal.tilt = 0;
            reset
                .interrupt_ground(assets, &[WaitPredicate::SmashSide])
                .unwrap();
            assert_eq!(reset.motion_state.action, S::AttackS4S.into());
            assert!(reset.commands.borrowed_script.is_none());
            assert_ne!(reset.commands.script.instruction, Some(usize::MAX));
            assert!(
                reset.commands.script.return_stack.is_empty()
                    && reset.commands.script.loops.is_empty()
            );
            assert_eq!(reset.commands.variables[0], 0);
        }
        assert_eq!(rows, frames as usize);
        assert!(expected_rows.next().is_none());
        if owner == Owner::YoungLink {
            assert_eq!(
                completed_reads, 1,
                "actual recorded Stadium completion input"
            );
        }
        match owner {
            Owner::Link => {
                assert_eq!(private_rows, 1, "entry-only last row");
                assert_eq!(combo_entries, 1);
                assert_eq!(controls, 1);
            }
            Owner::YoungLink => {
                assert_eq!(private_rows, 39);
                assert_eq!(combo_entries, 1);
                assert_eq!(controls, 1);
            }
            Owner::Fox => {
                assert!(common_rows > 0);
                assert_eq!(controls, 1);
            }
            Owner::Ness => {
                assert!(private_rows > 0);
                assert_eq!(common_rows, 0);
                assert_eq!(controls, 0);
                assert_eq!(private_controls, 1);
            }
        }
        eprintln!("{name}: {rows}complete rows,common={common_rows},private={private_rows},combo_entries={combo_entries},common_controls={controls},private_controls={private_controls}");
    }
}
