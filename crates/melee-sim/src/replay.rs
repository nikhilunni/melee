//! Cold replay comparison. Expected state never initializes or repairs a tick.
use crate::{frame::Simulation, initial_state::InitialState, scenario::Scenario};
use anyhow::{ensure, Result};
use melee_diff::{first_divergence, Divergence, Record};
use slp::{cold::ColdScenario, Replay};
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    path::Path,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct Setup {
    /// Slippi does not record the save's character unlock mask. A replay
    /// with Frame Start events shows it (the music draw); older ones need it.
    pub all_characters_unlocked: Option<bool>,
    /// An audited pre-music seed, overriding the one derived from Game Start.
    pub boundary_seed: Option<u32>,
    /// Replay a recording made with controller fixes (UCF/Dween) without
    /// them. A divergence may then be the fix's, so reports say so.
    pub ignore_controller_fixes: bool,
    /// The fix every port recorded with UCF runs, instead of the version
    /// dated from the recording (Slippi records "UCF", not which one).
    pub controller_fix: Option<melee_lib::ControllerFix>,
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
    /// Controller fixes the recording had and the run ignored.
    pub ignored_fixes: bool,
    /// Each leader's recorded character and action at the stop.
    pub context: String,
    /// Every field that differs on the diverging frame, in key order.
    pub differing: Vec<String>,
    pub stop: Stop,
}

impl std::fmt::Display for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}; {}", self.stage, self.players.join(", "))?;
        writeln!(
            f,
            "{} / {} replay frames matched{}",
            self.matched,
            self.frames,
            if self.ignored_fixes {
                " (controller fixes ignored)"
            } else {
                ""
            }
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

/// Slippi stage ids with a cold setup.
const COLD_STAGES: [u16; 6] = [2, 3, 8, 28, 31, 32];

/// Report every structural limitation, without loading DATs for an unsupported
/// matchup. A supported character does not imply its whole moveset is ported.
pub fn unsupported_setup(replay: &Replay, setup: Setup) -> Vec<String> {
    let mut reasons = Vec::new();
    let start = &replay.start;
    if !COLD_STAGES.contains(&start.stage) {
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
        if !melee_lib::diagnostics::CHARACTERS.contains(&name) {
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
        if !setup.ignore_controller_fixes && has_controller_fix(p) {
            match ucf_version(replay, p, setup) {
                Some(melee_lib::ControllerFix::Dween) => {
                    reasons.push(format!("port {} controller fix Dween", port + 1));
                }
                Some(_) => {}
                None => reasons.push(format!(
                    "port {} controller fix {:?}/{:?} (Dween or pre-0.74 UCF)",
                    port + 1,
                    p.dashback_fix,
                    p.shield_drop_fix
                )),
            }
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

/// The first Slippi version whose Post Frame is sampled at the end of the
/// fighter procs rather than after the map proc.
const POST_FRAME_AT_CAMERA: slp::Version = slp::Version {
    major: 3,
    minor: 4,
    build: 0,
};

/// The UCF version a UCF port ran. Slippi records only "UCF", so date the
/// recording by its start time against Slippi's code history: 0.74 entered
/// g_ucf.bin on 2019-10-09, 0.8 on 2021-03-31, 0.84 in 2024-02. Earlier
/// recordings ran the 0.73 beta (not ported); Dween is not ported.
/// `Setup::controller_fix` names the version instead.
fn ucf_version(
    replay: &Replay,
    p: &slp::PlayerStart,
    setup: Setup,
) -> Option<melee_lib::ControllerFix> {
    use melee_lib::ControllerFix;
    if !has_controller_fix(p) {
        return Some(ControllerFix::Off);
    }
    if setup.controller_fix.is_some() {
        return setup.controller_fix;
    }
    if p.dashback_fix != Some(1) || p.shield_drop_fix != Some(1) {
        return None;
    }
    let date = replay
        .metadata
        .as_ref()
        .and_then(|m| m.get("startAt"))
        .and_then(|s| s.as_str())?;
    let day = date.get(..10)?;
    match day {
        d if d < "2019-10-09" => None,
        d if d < "2021-03-31" => Some(ControllerFix::Ucf074),
        d if d < "2024-02-01" => Some(ControllerFix::Ucf080),
        _ => Some(ControllerFix::Ucf084),
    }
}

fn has_controller_fix(p: &slp::PlayerStart) -> bool {
    p.dashback_fix.is_some_and(|fix| fix != 0) || p.shield_drop_fix.is_some_and(|fix| fix != 0)
}

/// The cold boundary: the pre-music seed and the unlock flag.
///
/// Game Start's seed is taken at 0x8016E74C, before fn_8016E730 creates the
/// Ground and Players, so the boundary is that seed advanced by the setup's
/// fixed draws. The first Frame Start (2.2+) follows the music selection,
/// which draws once only for an all-unlocked save on a rule-6 stage: equal
/// seeds mean no draw, one step means the save was all-unlocked.
fn cold_boundary(
    replay: &Replay,
    scenario: &Scenario,
    setup: Setup,
) -> Result<(u32, bool), String> {
    let first_frame_start = replay
        .frames
        .values()
        .next()
        .and_then(slp::Frame::scheduler_start_seed);
    if let Some(seed) = setup.boundary_seed {
        let unlocked = setup
            .all_characters_unlocked
            .ok_or("an explicit boundary seed needs --all-characters-unlocked")?;
        return Ok((seed, unlocked));
    }
    let lib_setup =
        melee_lib::diagnostics::ScenarioSource::setup(scenario).map_err(|e| format!("{e:#}"))?;
    let boundary =
        melee_lib::diagnostics::boundary_seed_from_creation(&lib_setup, replay.start.random_seed)
            .map_err(|e| format!("{e:#}"))?;
    let Some(first) = first_frame_start else {
        return setup
            .all_characters_unlocked
            .map(|unlocked| (boundary, unlocked))
            .ok_or_else(|| {
                "no Frame Start: the recording's unlock flag is required \
                 (--all-characters-unlocked)"
                    .into()
            });
    };
    let mut after_music = gekko_math::HsdRng::new(boundary);
    after_music.rand();
    if first == boundary {
        Ok((boundary, setup.all_characters_unlocked.unwrap_or(false)))
    } else if first == after_music.seed {
        Ok((boundary, true))
    } else {
        Err(format!(
            "Game Start seed {:08X} does not reach the first Frame Start {first:08X} \
             through the setup draws (boundary {boundary:08X})",
            replay.start.random_seed
        ))
    }
}

/// The recording's spawn rule. Replays before the Gecko code list (3.3) do
/// not name their codes, so choose among the known discrete rules by the
/// first frame: a NeutralSpawn table when every leader stands exactly at
/// its row, otherwise retail. Frame zero's comparison checks the choice;
/// nothing else of the recorded state is used.
fn spawn_rule(replay: &Replay) -> melee_lib::slippi::SpawnRule {
    use melee_lib::slippi::{NeutralTable, SpawnRule};
    let Some(first) = replay.frames.values().next() else {
        return SpawnRule::Retail;
    };
    let fits = |table| {
        replay.leader_ports().enumerate().all(|(order, port)| {
            let (Some(post), Some((spawn, facing))) = (
                first.ports[port].leader.post.as_ref(),
                melee_lib::slippi::neutral_spawn(table, i32::from(replay.start.stage), order),
            ) else {
                return false;
            };
            post.position_x.to_bits() == spawn.x.to_bits()
                && post.position_y.to_bits() == spawn.y.to_bits()
                && post.facing_direction.to_bits() == facing.to_bits()
        })
    };
    [NeutralTable::V2020, NeutralTable::V2019]
        .into_iter()
        .find(|&table| fits(table))
        .map_or(SpawnRule::Retail, SpawnRule::NeutralTable)
}

/// A cold scenario for the replay, from Game Start's rules and the recorded
/// inputs. Never initialized or repaired from recorded state.
pub fn cold_scenario(replay: &Replay, root: &Path, setup: Setup) -> Result<Scenario> {
    ensure!(
        unsupported_setup(replay, setup).is_empty(),
        "unsupported replay setup"
    );
    let build = |seed: u32, unlocked: bool| -> Result<Scenario> {
        let cold = ColdScenario::from_replay(replay, "slippi_replay", seed, unlocked)?;
        let mut scenario: Scenario = toml::from_str(&cold.to_toml()?)?;
        scenario.root = root.into();
        if replay.start.timer_type() == 2 && replay.start.game_timer != 0 {
            scenario.time_limit = Some(replay.start.game_timer);
        }
        scenario.spawn = spawn_rule(replay);
        if !setup.ignore_controller_fixes {
            for (fighter, port) in scenario.fighters.iter_mut().zip(replay.leader_ports()) {
                let fix = ucf_version(replay, &replay.start.players[port], setup)
                    .expect("checked by unsupported_setup");
                fighter.controller_fix = Some(fix.name().to_string());
            }
        }
        scenario.validate()?;
        Ok(scenario)
    };
    let provisional = build(replay.start.random_seed, false)?;
    let (seed, unlocked) =
        cold_boundary(replay, &provisional, setup).map_err(|e| anyhow::anyhow!(e))?;
    build(seed, unlocked)
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
        ignored_fixes: setup.ignore_controller_fixes
            && replay
                .leader_ports()
                .any(|port| has_controller_fix(&replay.start.players[port])),
        context: String::new(),
        differing: Vec::new(),
        stop: Stop::Complete,
    };
    let reasons = unsupported_setup(replay, setup);
    if !reasons.is_empty() {
        report.stop = Stop::Unsupported(reasons);
        return Ok(report);
    }
    let scenario = match cold_scenario(replay, root, setup) {
        Ok(scenario) => scenario,
        Err(error) => {
            report.stop = Stop::NeedsSetup(format!("{error:#}"));
            return Ok(report);
        }
    };
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
    let leaders: Vec<usize> = replay.leader_ports().collect();
    // Slippi before 3.4.0 sampled Post Frame at 0x8006C5D8, the end of each
    // fighter's map proc (Fighter_8006C27C, s_link 6), before hit detection
    // and damage; 3.4.0 moved it to 0x8006DA34 in the camera proc (s_link
    // 18), after the fighter procs the trace reads (slippi-ssbm-asm 9398d52).
    simulation.capture_after_map(replay.version() < POST_FRAME_AT_CAMERA);
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
        let frame = &replay.frames[&(expected.frame as i32 + slp::SLIPPI_FIRST_FRAME)];
        report.context = leader_actions(replay, frame);
        let result = catch_unwind(AssertUnwindSafe(|| simulation.tick()));
        let actual = match result {
            Ok(record) => {
                let record = record?;
                simulation.after_map_record(&record).unwrap_or(record)
            }
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
        let actual = by_leader(&actual, &leaders, frame, expected.frame);
        if let Some(diff) = compare_frame(&expected, &actual) {
            report.differing = expected
                .state
                .iter()
                .filter(|(key, value)| actual.state.get(*key) != Some(value))
                .map(|(key, _)| key.clone())
                .collect();
            report.stop = Stop::Diverged(diff);
            return Ok(report);
        }
        report.matched += 1;
    }
    Ok(report)
}

/// The simulated record keyed like the replay's: `pN` is the Nth leader's
/// fighter in play, found by player slot and internal kind (a player's
/// other fighters, such as Nana or Zelda's sleeping form, shift fighter-list
/// indices). Nana's recorded follower frame is keyed `pN.follower`.
fn by_leader(actual: &Record, leaders: &[usize], frame: &slp::Frame, tick: u64) -> Record {
    let fighter_of = |slot: usize, kind: i64| {
        (0..8).find(|k| {
            actual.state.get(&format!("p{k}.player_id"))
                == Some(&melee_diff::Value::UInt(slot as u64))
                && actual.state.get(&format!("p{k}.kind")) == Some(&melee_diff::Value::Int(kind))
        })
    };
    let mut state = std::collections::BTreeMap::new();
    if let Some(seed) = actual.state.get("rng.seed") {
        state.insert("rng.seed".to_string(), seed.clone());
    }
    for (index, &port) in leaders.iter().enumerate() {
        let ports = &frame.ports[port];
        let members = [
            (ports.leader.post.as_ref(), format!("p{index}.")),
            (ports.follower.post.as_ref(), format!("p{index}.follower.")),
        ];
        for (post, prefix) in members {
            let Some(post) = post else { continue };
            let Some(k) = fighter_of(port, i64::from(post.internal_character)) else {
                continue;
            };
            let from = format!("p{k}.");
            for (key, value) in actual.state.range(from.clone()..) {
                let Some(field) = key.strip_prefix(&from) else {
                    break;
                };
                state.insert(format!("{prefix}{field}"), value.clone());
            }
        }
    }
    Record {
        frame: tick,
        phase: actual.phase.clone(),
        state,
    }
}

fn leader_actions(replay: &Replay, frame: &slp::Frame) -> String {
    replay
        .leader_ports()
        .enumerate()
        .filter_map(|(index, port)| {
            let post = frame.ports[port].leader.post.as_ref()?;
            Some(format!(
                "p{index} {} {}",
                slp::ids::internal_character_name(post.internal_character).unwrap_or("?"),
                action_name(post.action_state)
            ))
        })
        .collect::<Vec<_>>()
        .join(" / ")
}

pub fn compare_frame(expected: &Record, actual: &Record) -> Option<Divergence> {
    first_divergence([expected], [actual])
}

fn action_name(id: u16) -> String {
    melee_types::CommonMotionState::try_from(i32::from(id))
        .map(|state| format!("{state:?} ({id})"))
        .unwrap_or_else(|_| format!("character action {id}"))
}
