//! Cold replay comparison. Expected state never initializes or repairs a tick.
use crate::{frame::Simulation, initial_state::InitialState, scenario::Scenario};
use anyhow::{ensure, Context, Result};
use melee_diff::{first_divergence, Divergence, Record};
use slp::{cold::ColdScenario, Replay};
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    path::Path,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct Setup {
    /// Slippi does not record the save's character unlock mask.
    pub all_characters_unlocked: Option<bool>,
    /// Optional audited pre-music seed, needed by pre-Frame-Start recordings.
    pub boundary_seed: Option<u32>,
}

#[derive(Debug)]
pub enum Stop {
    Complete,
    Unsupported(Vec<String>),
    NeedsSetup(String),
    UnavailableInput {
        tick: u64,
        reason: String,
    },
    Unported {
        tick: u64,
        action: String,
        reason: String,
    },
    Diverged(Divergence),
}

#[derive(Debug)]
pub struct Report {
    pub stage: String,
    pub players: Vec<String>,
    pub frames: usize,
    pub matched: usize,
    pub stop: Stop,
}

impl std::fmt::Display for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}; {}", self.stage, self.players.join(", "))?;
        writeln!(
            f,
            "{} / {} replay frames matched",
            self.matched, self.frames
        )?;
        match &self.stop {
            Stop::Complete => write!(f, "all recorded frames matched"),
            Stop::Unsupported(reasons) => write!(f, "unsupported setup: {}", reasons.join("; ")),
            Stop::NeedsSetup(reason) => write!(f, "setup not established: {reason}"),
            Stop::UnavailableInput { tick, reason } => {
                write!(f, "input unavailable at tick {tick}: {reason}")
            }
            Stop::Unported {
                tick,
                action,
                reason,
            } => write!(
                f,
                "unported boundary at tick {tick} (Slippi {}), {action}: {reason}",
                *tick as i64 + i64::from(slp::SLIPPI_FIRST_FRAME)
            ),
            Stop::Diverged(diff) => write!(f, "{diff}"),
        }
    }
}

/// Report every structural limitation, without loading DATs for an unsupported
/// matchup. A supported character does not imply its whole moveset is ported.
pub fn unsupported_setup(replay: &Replay) -> Vec<String> {
    let mut reasons = Vec::new();
    let start = &replay.start;
    if !matches!(start.stage, 8 | 31 | 32) {
        reasons.push(format!(
            "{} cold stage",
            slp::ids::stage_name(start.stage).unwrap_or("unknown")
        ));
    }
    if replay.leader_ports().count() != 2 {
        reasons.push("requires two leaders".into());
    }
    for port in replay.leader_ports() {
        let p = &start.players[port];
        let name = slp::ids::external_character_name(p.character).unwrap_or("unknown");
        if !crate::scene_fighter::SceneFighter::NAMES.contains(&name) {
            reasons.push(format!("port {} character {name}", port + 1));
        }
        if p.player_type != slp::PlayerType::Human {
            reasons.push(format!("port {} {:?} control", port + 1, p.player_type));
        }
        if p.damage_start != 0
            || p.damage_spawn != 0
            || p.handicap != 9
            || [p.offense_ratio, p.defense_ratio, p.model_scale]
                .iter()
                .any(|v| v.to_bits() != 1.0_f32.to_bits())
        {
            reasons.push(format!("port {} nonstandard fighter rules", port + 1));
        }
    }
    if start.pal == Some(true) {
        reasons.push("PAL mechanics".into());
    }
    if start.language == Some(0) {
        reasons.push("Japanese countdown assets (English setup only)".into());
    }
    if start.game_speed.to_bits() != 1.0_f32.to_bits() {
        reasons.push("nonstandard game speed".into());
    }
    if start.is_online() {
        reasons.push("Slippi Online initialization/seed resets".into());
    }
    if start.is_teams
        || start.game_mode() != 1
        || !matches!(start.timer_type(), 0 | 2)
        || start.item_spawn_behavior != -1
        || start.damage_ratio.to_bits() != 1.0_f32.to_bits()
    {
        reasons.push(
            "requires singles stock mode, items off, normal damage and countdown/no timer".into(),
        );
    }
    reasons
}

/// Frame Start is after pending music and before the first complete scheduler
/// pass. Undo only the audited music draw; never use Game Start as that seed.
pub fn cold_scenario(replay: &Replay, root: &Path, setup: Setup) -> Result<Scenario> {
    ensure!(
        unsupported_setup(replay).is_empty(),
        "unsupported replay setup"
    );
    let unlocked = setup.all_characters_unlocked.context(
        "supply --all-characters-unlocked true/false from the recording setup; Slippi lacks this flag")?;
    let seed = match setup.boundary_seed {
        Some(seed) => seed,
        None => {
            let seed = replay
                .frames
                .values()
                .next()
                .and_then(slp::Frame::scheduler_start_seed)
                .context("no Frame Start seed; supply an independently recorded --boundary-seed")?;
            if unlocked && matches!(replay.start.stage, 31 | 32) {
                seed.wrapping_sub(gekko_math::HsdRng::INCREMENT)
                    .wrapping_mul(0xB9B3_3155)
            } else {
                seed
            }
        }
    };
    let cold = ColdScenario::from_replay(replay, "slippi_replay", seed, unlocked)?;
    let mut scenario: Scenario = toml::from_str(&cold.to_toml()?)?;
    scenario.root = root.into();
    scenario.validate()?;
    Ok(scenario)
}

pub fn run_file(path: &Path, root: &Path, setup: Setup) -> Result<Report> {
    let replay = Replay::parse(&std::fs::read(path)?)?;
    run(&replay, root, setup)
}

pub fn run(replay: &Replay, root: &Path, setup: Setup) -> Result<Report> {
    ensure!(
        !replay.incomplete && !replay.frames.is_empty(),
        "incomplete or empty replay"
    );
    let mut report = Report {
        stage: slp::ids::stage_name(replay.start.stage)
            .unwrap_or("unknown")
            .into(),
        players: replay
            .leader_ports()
            .enumerate()
            .map(|(index, port)| {
                let p = &replay.start.players[port];
                format!(
                    "port {} -> p{index} {} costume {} {:?} stocks {}",
                    port + 1,
                    slp::ids::external_character_name(p.character).unwrap_or("unknown"),
                    p.costume,
                    p.player_type,
                    p.stock_start_count
                )
            })
            .collect(),
        frames: replay.frames.len(),
        matched: 0,
        stop: Stop::Complete,
    };
    let reasons = unsupported_setup(replay);
    if !reasons.is_empty() {
        report.stop = Stop::Unsupported(reasons);
        return Ok(report);
    }
    if setup.all_characters_unlocked.is_none()
        || (setup.boundary_seed.is_none()
            && replay
                .frames
                .values()
                .next()
                .and_then(slp::Frame::scheduler_start_seed)
                .is_none())
    {
        report.stop = Stop::NeedsSetup("recording unlock flag required; old versions also need an audited pre-music boundary seed (see docs/SLIPPI.md)".into());
        return Ok(report);
    }
    let scenario = cold_scenario(replay, root, setup)?;
    // A bad late controller field must not discard an already comparable
    // prefix. Do not install guessed pads for the unavailable tick.
    let unavailable = scenario.replay_inputs.iter().find_map(|input| {
        crate::inputs::replay_pad(input)
            .err()
            .map(|error| (input.frame, format!("port {}: {error}", input.port + 1)))
    });
    let pad_frames = unavailable
        .as_ref()
        .map_or(scenario.frames, |(frame, _)| *frame);
    let prefix: Vec<_> = scenario
        .replay_inputs
        .iter()
        .filter(|input| input.frame < pad_frames)
        .cloned()
        .collect();
    let pads = crate::inputs::PadScript::from_replay_inputs(
        &prefix,
        pad_frames as usize,
        &scenario.fighters.iter().map(|f| f.slot).collect::<Vec<_>>(),
    )?;
    let mut simulation = Simulation::with_inputs(InitialState::from_parameters(&scenario)?, pads);
    // Observation zero is a reset store, not Slippi frame -123.
    let initial = simulation.tick()?;
    if let Some(seed) = replay
        .frames
        .values()
        .next()
        .and_then(slp::Frame::scheduler_start_seed)
    {
        ensure!(
            initial.state["rng.seed"] == melee_diff::Value::UInt(u64::from(seed)),
            "boundary seed does not reach first Frame Start"
        );
    }
    for expected in slp::to_trace(replay) {
        if let Some((tick, reason)) = &unavailable {
            if *tick == expected.frame {
                report.stop = Stop::UnavailableInput {
                    tick: *tick,
                    reason: reason.clone(),
                };
                return Ok(report);
            }
        }
        if replay.start.timer_type() == 2
            && replay.start.game_timer != 0
            && expected.frame >= 123 + u64::from(replay.start.game_timer) * 60
        {
            report.stop = Stop::Unported {
                tick: expected.frame,
                action: "match timer".into(),
                reason: "time-up/result processing is not ported".into(),
            };
            return Ok(report);
        }
        let frame = &replay.frames[&(expected.frame as i32 + slp::SLIPPI_FIRST_FRAME)];
        for port in replay.leader_ports() {
            let post = frame.ports[port]
                .leader
                .post
                .as_ref()
                .context("missing Post Frame")?;
            let pre = frame.ports[port]
                .leader
                .pre
                .as_ref()
                .context("missing Pre Frame")?;
            let player = &replay.start.players[port];
            if (player.dashback_fix.is_some_and(|fix| fix != 0)
                || player.shield_drop_fix.is_some_and(|fix| fix != 0))
                && (pre.joystick_x != 0.0 || pre.joystick_y != 0.0)
            {
                report.stop = Stop::Unported {
                    tick: expected.frame,
                    action: action_name(post.action_state),
                    reason: format!(
                        "port {} controller fixes (UCF/Dween) not implemented",
                        port + 1
                    ),
                };
                return Ok(report);
            }
            if !ported_state(post.action_state) {
                report.stop = Stop::Unported {
                    tick: expected.frame,
                    action: format!("port {} {}", port + 1, action_name(post.action_state)),
                    reason: "no implemented action callback table; frame not counted as matched"
                        .into(),
                };
                return Ok(report);
            }
        }
        let result = catch_unwind(AssertUnwindSafe(|| simulation.tick()));
        let mut actual = match result {
            Ok(record) => record?,
            Err(payload) => {
                let message = payload
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .or_else(|| payload.downcast_ref::<&str>().copied())
                    .unwrap_or("unknown panic");
                if !message.starts_with("not implemented:") {
                    anyhow::bail!("simulation panic at tick {}: {message}", expected.frame);
                }
                report.stop = Stop::Unported {
                    tick: expected.frame,
                    action: frame
                        .ports
                        .iter()
                        .filter_map(|p| p.leader.post.as_ref())
                        .map(|p| action_name(p.action_state))
                        .collect::<Vec<_>>()
                        .join(" / "),
                    reason: message.into(),
                };
                return Ok(report);
            }
        };
        actual.frame = expected.frame;
        if let Some(diff) = compare_frame(&expected, &actual) {
            report.stop = Stop::Diverged(diff);
            return Ok(report);
        }
        report.matched += 1;
    }
    Ok(report)
}

pub fn compare_frame(expected: &Record, actual: &Record) -> Option<Divergence> {
    first_divergence([expected], [actual])
}

fn action_name(id: u16) -> String {
    melee_types::CommonMotionState::try_from(i32::from(id))
        .map(|state| format!("{state:?} ({id})"))
        .unwrap_or_else(|_| format!("character action {id}"))
}

/// Use the existing action identities, not a numeric range that accidentally
/// includes unported attacks. A table may still reach an explicit missing hook.
fn ported_state(id: u16) -> bool {
    use melee_types::CommonMotionState as M;
    [
        M::Wait,
        M::WalkSlow,
        M::WalkMiddle,
        M::WalkFast,
        M::Turn,
        M::TurnRun,
        M::Dash,
        M::Run,
        M::RunBrake,
        M::KneeBend,
        M::JumpF,
        M::JumpB,
        M::JumpAerialF,
        M::JumpAerialB,
        M::Fall,
        M::FallAerial,
        M::FallSpecial,
        M::Squat,
        M::SquatWait,
        M::SquatRv,
        M::Landing,
        M::LandingFallSpecial,
        M::Attack11,
        M::DamageN2,
        M::GuardOn,
        M::Guard,
        M::GuardOff,
        M::GuardSetOff,
        M::GuardReflect,
        M::EscapeF,
        M::EscapeB,
        M::EscapeN,
        M::EscapeAir,
        M::Pass,
        M::CliffCatch,
        M::CliffWait,
        M::CliffClimbQuick,
        M::CliffEscapeQuick,
        M::CliffJumpQuick1,
        M::CliffJumpQuick2,
        M::Entry,
        M::EntryStart,
        M::EntryEnd,
    ]
    .iter()
    .any(|state| *state as i32 == i32::from(id))
}
