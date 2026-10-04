//! Cold replay comparison. Expected state never initializes or repairs a tick.
use crate::replay_stage_codes::{resolve_stage_codes, StageCodeChoice};
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
    /// Whether the console ran the frozen-stage code, instead of what the
    /// recording's seeds show (`replay_stage_codes`).
    pub frozen_stages: Option<bool>,
}

/// The UCF version a recording's ports run and what decided it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixChoice {
    pub fix: melee_lib::ControllerFix,
    pub reason: FixReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixReason {
    /// `Setup::controller_fix`.
    Named,
    /// The recording's start date against Slippi's code history.
    Dated,
    /// The date allows two versions; on this tick their dashbacks first
    /// disagree and the recorded facing is this version's, not `over`'s.
    Dashback {
        tick: u64,
        over: melee_lib::ControllerFix,
    },
}

impl std::fmt::Display for FixChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.fix.name())?;
        match self.reason {
            FixReason::Named => write!(f, " (named)"),
            FixReason::Dated => write!(f, " (by date)"),
            FixReason::Dashback { tick, over } => write!(
                f,
                " (not {}: the recorded facing at tick {tick}, the first dashback they disagree on)",
                over.name()
            ),
        }
    }
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
    /// The UCF version the recording's UCF ports ran in this comparison.
    pub controller_fix: Option<FixChoice>,
    /// Whether the frozen-stage code ran in this comparison; `None` on a
    /// stage it does not change.
    pub stage_code: Option<StageCodeChoice>,
    /// Each leader's recorded character and action at the stop.
    pub context: String,
    /// Every field that differs on the diverging frame, in key order.
    pub differing: Vec<String>,
    pub stop: Stop,
}

impl std::fmt::Display for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}; {}", self.stage, self.players.join(", "))?;
        if let Some(choice) = &self.controller_fix {
            writeln!(f, "controller fix {choice}")?;
        }
        if let Some(choice) = &self.stage_code {
            writeln!(f, "stage code {choice}")?;
        }
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
                    "port {} controller fix {:?}/{:?} (Dween, or UCF without a start date)",
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

/// The first Slippi version recorded with the Stadium transformation
/// preload code. Replays before the Gecko code list (3.3) do not name it;
/// Slippi's own playback enables it by this version (Ishiiruka
/// EXI_DeviceSlippi.cpp, "Write PS pre-load byte": major > 1, or 1 with
/// minor > 2).
const STADIUM_PRELOAD: slp::Version = slp::Version {
    major: 1,
    minor: 3,
    build: 0,
};

/// The first day a Slippi output carried UCF 0.74: the Dolphin code lists
/// and the console toggle set `g_toggles.bin` (slippi-ssbm-asm acc6f71).
const UCF_074_FIRST_BUILT: &str = "2019-09-24";
/// The day 0.74 replaced `Binary/UCF/Ucf0.73Beta.bin` in the console's
/// `g_ucf.bin` (823067b).
const UCF_074_CONSOLE: &str = "2019-10-09";
/// The day 0.8 replaced 0.74 (bb86519).
const UCF_080: &str = "2021-03-31";
/// 0.84 (422bb78 and the console set that followed).
const UCF_084: &str = "2024-02-01";

/// The UCF versions a recording started on `day` can have run: the one
/// Slippi's build of that day installed and, where setups are known to have
/// run another, that one too. Consoles kept old builds: one recorded with
/// the 0.73 beta on 2020-02-08 (`11_12_26 Marth + Peach (BF)`, the dashback
/// at tick 7087), as others kept 2019's spawn code into March 2020.
fn ucf_by_date(day: &str) -> (melee_lib::ControllerFix, Option<melee_lib::ControllerFix>) {
    use melee_lib::ControllerFix::{Ucf073, Ucf074, Ucf080, Ucf084};
    match day {
        d if d < UCF_074_FIRST_BUILT => (Ucf073, None),
        d if d < UCF_074_CONSOLE => (Ucf073, Some(Ucf074)),
        d if d < UCF_080 => (Ucf074, Some(Ucf073)),
        d if d < UCF_084 => (Ucf080, None),
        _ => (Ucf084, None),
    }
}

/// The recording's start day, `YYYY-MM-DD`.
fn start_day(replay: &Replay) -> Option<&str> {
    replay
        .metadata
        .as_ref()
        .and_then(|m| m.get("startAt"))
        .and_then(|s| s.as_str())?
        .get(..10)
}

/// The UCF version a UCF port ran. Slippi records only "UCF", so date the
/// recording by its start time against Slippi's code history
/// ([`ucf_by_date`]); [`resolve_controller_fix`] settles a date with two
/// candidates. Dween is not ported. `Setup::controller_fix` names the
/// version instead.
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
    if !is_ucf(p) {
        return None;
    }
    Some(ucf_by_date(start_day(replay)?).0)
}

/// Game Start's UCF: both the dashback and shield drop fields are 1.
fn is_ucf(p: &slp::PlayerStart) -> bool {
    p.dashback_fix == Some(1) && p.shield_drop_fix == Some(1)
}

/// The setup with the recording's dated UCF version named, when its ports
/// ran UCF and no version is named yet; otherwise the setup as given.
pub(crate) fn dated_controller_fix(replay: &Replay, setup: Setup) -> Setup {
    let any_ucf = replay
        .leader_ports()
        .any(|port| is_ucf(&replay.start.players[port]));
    if setup.ignore_controller_fixes || !any_ucf || setup.controller_fix.is_some() {
        return setup;
    }
    match start_day(replay).map(ucf_by_date) {
        Some((dated, _)) => Setup {
            controller_fix: Some(dated),
            ..setup
        },
        None => setup,
    }
}

/// Name the UCF version for a recording whose date allows two.
///
/// The two dashbacks differ in one case (0.74 smash turns on a full stick
/// either way, 0.73 only toward the turn), and until a game reaches it the
/// versions compute the same match. So run the port under both in step, from
/// the recorded inputs alone; on the first tick a UCF port's facing differs
/// between them, the recorded facing is one version's. That version then
/// runs the comparison (every frame, from the start). A game the versions
/// never disagree on keeps the dated one.
pub fn resolve_controller_fix(
    replay: &Replay,
    root: &Path,
    setup: Setup,
) -> (Setup, Option<FixChoice>) {
    let ucf_ports = || {
        replay
            .leader_ports()
            .map(|port| &replay.start.players[port])
    };
    if setup.ignore_controller_fixes || !ucf_ports().any(is_ucf) {
        return (setup, None);
    }
    if let Some(fix) = setup.controller_fix {
        let reason = FixReason::Named;
        return (setup, Some(FixChoice { fix, reason }));
    }
    let Some((dated, other)) = start_day(replay).map(ucf_by_date) else {
        return (setup, None);
    };
    let named = |fix| Setup {
        controller_fix: Some(fix),
        ..setup
    };
    let mut choice = FixChoice {
        fix: dated,
        reason: FixReason::Dated,
    };
    if let Some(other) = other.filter(|_| unsupported_setup(replay, named(dated)).is_empty()) {
        // An unported boundary or setup error before any disagreement
        // leaves the dated version; the comparison run reports it.
        let probe = catch_unwind(AssertUnwindSafe(|| {
            first_facing_disagreement(replay, root, named(dated), named(other))
        }));
        if let Ok(Ok(Some(disagreement))) = probe {
            let (fix, over) = if disagreement.recorded_is_second {
                (other, dated)
            } else {
                (dated, other)
            };
            let tick = disagreement.tick;
            choice = FixChoice {
                fix,
                reason: FixReason::Dashback { tick, over },
            };
        }
    }
    (named(choice.fix), Some(choice))
}

/// The first tick two setups' runs differ in a UCF leader's facing.
struct FacingDisagreement {
    tick: u64,
    /// The recorded facing is the second run's, not the first's.
    recorded_is_second: bool,
}

/// Run the replay's inputs under two setups in step, up to the first tick
/// any compared field differs between them. `None` unless a UCF leader's
/// `facing_dir` is among what differs there (a dashback's first effect).
fn first_facing_disagreement(
    replay: &Replay,
    root: &Path,
    first: Setup,
    second: Setup,
) -> Result<Option<FacingDisagreement>> {
    let mut first = ColdRun::start(replay, &cold_scenario(replay, root, first)?)?;
    let mut second = ColdRun::start(replay, &cold_scenario(replay, root, second)?)?;
    let leaders: Vec<usize> = replay.leader_ports().collect();
    let last_tick = first.unavailable.as_ref().map(|(tick, _)| *tick);
    for expected in slp::to_trace(replay) {
        if last_tick == Some(expected.frame) {
            break;
        }
        let frame = &replay.frames[&(expected.frame as i32 + slp::SLIPPI_FIRST_FRAME)];
        let tick = |run: &mut ColdRun| -> Result<Record> {
            let record = run.simulation.tick()?;
            let record = run.simulation.after_map_record(&record).unwrap_or(record);
            Ok(by_leader(&record, &leaders, frame, expected.frame))
        };
        let (a, b) = (tick(&mut first)?, tick(&mut second)?);
        if expected
            .state
            .keys()
            .all(|key| a.state.get(key) == b.state.get(key))
        {
            continue;
        }
        return Ok(leaders.iter().enumerate().find_map(|(index, &port)| {
            let key = format!("p{index}.facing_dir");
            let (a, b) = (a.state.get(&key), b.state.get(&key));
            (is_ucf(&replay.start.players[port]) && a != b).then(|| FacingDisagreement {
                tick: expected.frame,
                recorded_is_second: expected.state.get(&key) == b,
            })
        }));
    }
    Ok(None)
}

/// A replay's cold simulation, before its first compared tick.
pub(crate) struct ColdRun {
    pub(crate) simulation: Simulation,
    /// Observation zero: a reset store, not Slippi frame -123.
    pub(crate) initial: Record,
    /// The first tick whose recorded controller cannot be installed. A bad
    /// late controller field must not discard an already comparable prefix,
    /// and no guessed pads stand in for it.
    pub(crate) unavailable: Option<(u64, String)>,
}

impl ColdRun {
    pub(crate) fn start(replay: &Replay, scenario: &Scenario) -> Result<Self> {
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
        let mut simulation =
            Simulation::with_inputs(InitialState::from_parameters(scenario)?, pads);
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
        // Slippi before 3.4.0 sampled Post Frame at 0x8006C5D8, the end of
        // each fighter's map proc (Fighter_8006C27C, s_link 6), before hit
        // detection and damage; 3.4.0 moved it to 0x8006DA34 in the camera
        // proc (s_link 18), after the fighter procs the trace reads
        // (slippi-ssbm-asm 9398d52).
        simulation.capture_after_map(replay.version() < POST_FRAME_AT_CAMERA);
        Ok(Self {
            simulation,
            initial,
            unavailable,
        })
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
/// its row (an Ice Climbers pair: on either side of it), otherwise retail. The version that restaggers the entries shows
/// in the first leader, which has already left Entry on the first frame (no
/// other rule enters anyone before the sixth tick), so it has risen off its
/// row's height. Frame zero's comparison checks the choice; nothing else of
/// the recorded state is used.
fn spawn_rule(replay: &Replay) -> melee_lib::slippi::SpawnRule {
    use melee_lib::slippi::{NeutralTable, SpawnRule};
    let Some(first) = replay.frames.values().next() else {
        return SpawnRule::Retail;
    };
    let entry_start = melee_types::CommonMotionState::EntryStart as u16;
    let fits = |table: NeutralTable| {
        replay.leader_ports().enumerate().all(|(order, port)| {
            let (Some(post), Some((spawn, facing))) = (
                first.ports[port].leader.post.as_ref(),
                melee_lib::slippi::neutral_spawn(table, i32::from(replay.start.stage), order),
            ) else {
                return false;
            };
            // A zero entry delay starts EntryStart on the first tick.
            let entered = table.entry_delay(order) == Some(0);
            // A kind with a second fighter (Ice Climbers) stands each one
            // off the Player's position along its facing (fighter.c:241,
            // 0x80067CE8; Popo five units ahead, Nana five behind), so the
            // pair straddles the row instead of standing on it.
            let follower = first.ports[port].follower.post.as_ref();
            let at_row = match follower {
                None => post.position_x.to_bits() == spawn.x.to_bits(),
                Some(follower) => {
                    let (low, high) = if post.position_x < follower.position_x {
                        (post.position_x, follower.position_x)
                    } else {
                        (follower.position_x, post.position_x)
                    };
                    low < spawn.x && spawn.x < high
                }
            };
            at_row
                && post.facing_direction.to_bits() == facing.to_bits()
                && (post.action_state == entry_start) == entered
                && (entered || post.position_y.to_bits() == spawn.y.to_bits())
        })
    };
    [
        NeutralTable::V2020,
        NeutralTable::V2019,
        NeutralTable::V2019EntryByOrder,
    ]
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
        scenario.stadium_preload = replay.version() >= STADIUM_PRELOAD;
        scenario.stadium_frozen = replay.start.frozen_ps == Some(true);
        scenario.frozen_stages = setup.frozen_stages.unwrap_or(false);
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

/// What a replay does not record, read from its frames: the stage code
/// (`replay_stage_codes`) and the UCF version ([`resolve_controller_fix`]).
/// Each is decided by its own dry pass before the comparison, which then
/// runs under the one chosen setup from the first frame. The stage code is
/// read first, under the dated UCF version, so the dashback probe runs on
/// the console's random stream; if that probe then names the other UCF
/// version, the stage code is read again under it.
pub fn resolve_setup(
    replay: &Replay,
    root: &Path,
    setup: Setup,
) -> (Setup, Option<FixChoice>, Option<StageCodeChoice>) {
    let (staged, stage_code) = resolve_stage_codes(replay, root, setup);
    let (fixed, controller_fix) = resolve_controller_fix(replay, root, staged);
    let redated = matches!(
        controller_fix,
        Some(FixChoice {
            reason: FixReason::Dashback { .. },
            ..
        })
    );
    if !redated || setup.frozen_stages.is_some() {
        return (fixed, controller_fix, stage_code);
    }
    let unstaged = Setup {
        frozen_stages: None,
        ..fixed
    };
    let (setup, stage_code) = resolve_stage_codes(replay, root, unstaged);
    (setup, controller_fix, stage_code)
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
    let (setup, controller_fix, stage_code) = resolve_setup(replay, root, setup);
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
        controller_fix,
        stage_code,
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
    let ColdRun {
        mut simulation,
        initial,
        unavailable,
    } = ColdRun::start(replay, &scenario)?;
    let leaders: Vec<usize> = replay.leader_ports().collect();
    let mut previous_seed = seed_of(&initial);
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
        let ComparedTick {
            expected,
            actual,
            end_seed,
        } = compared_tick(&actual, &leaders, frame, expected, previous_seed);
        previous_seed = end_seed;
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

/// One simulated tick beside its recorded frame, both keyed by leader and
/// with the Pre Frame seed check added.
pub(crate) struct ComparedTick {
    pub(crate) expected: Record,
    pub(crate) actual: Record,
    /// The simulation's seed at the end of the tick.
    pub(crate) end_seed: Option<u32>,
}

impl ComparedTick {
    pub(crate) fn matches(&self) -> bool {
        compare_frame(&self.expected, &self.actual).is_none()
    }
}

/// Key the tick's record like the recording's and add the seed check from
/// the previous tick's end seed.
pub(crate) fn compared_tick(
    record: &Record,
    leaders: &[usize],
    frame: &slp::Frame,
    mut expected: Record,
    previous_seed: Option<u32>,
) -> ComparedTick {
    let end_seed = seed_of(record);
    let mut actual = by_leader(record, leaders, frame, expected.frame);
    if let (Some(from), Some(to)) = (previous_seed, end_seed) {
        compare_input_seeds(from, to, leaders, frame, &mut expected, &mut actual);
    }
    ComparedTick {
        expected,
        actual,
        end_seed,
    }
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

pub(crate) fn seed_of(record: &Record) -> Option<u32> {
    match record.state.get("rng.seed")? {
        melee_diff::Value::UInt(seed) => u32::try_from(*seed).ok(),
        _ => None,
    }
}

/// The most draws one tick is searched for (a tick with more is not checked).
const MAX_TICK_DRAWS: usize = 1 << 16;

/// Add `pN.input_seed` to both records: Pre Frame's seed (hook 0x8006B0E0,
/// in the fighter's input proc) must be one the port's stream passes through
/// during the same tick, from the previous tick's end seed (`from`) to this
/// one's (`to`). The port does not keep retail's draw timing inside a tick,
/// only the sequence, so the test is membership, not position. A stream that
/// has left retail's fails it on the first quiet tick, which names the drift
/// long before a fighter field shows it. The actual value reported is the
/// tick's first seed.
fn compare_input_seeds(
    from: u32,
    to: u32,
    leaders: &[usize],
    frame: &slp::Frame,
    expected: &mut Record,
    actual: &mut Record,
) {
    use melee_diff::Value;
    let mut rng = gekko_math::rng::HsdRng { seed: from };
    let mut path = vec![from];
    while rng.seed != to {
        if path.len() > MAX_TICK_DRAWS {
            return;
        }
        rng.rand();
        path.push(rng.seed);
    }
    for (index, &port) in leaders.iter().enumerate() {
        let ports = &frame.ports[port];
        let members = [
            (&ports.leader, format!("p{index}.input_seed")),
            (&ports.follower, format!("p{index}.follower.input_seed")),
        ];
        for (member, key) in members {
            let Some(pre) = member.pre.as_ref() else {
                continue;
            };
            let seen = if path.contains(&pre.random_seed) {
                pre.random_seed
            } else {
                from
            };
            expected
                .state
                .insert(key.clone(), Value::UInt(u64::from(pre.random_seed)));
            actual.state.insert(key, Value::UInt(u64::from(seen)));
        }
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

/// PlCo's stick dead zone (ftCommonData x0/x4): Pre Frame's joystick is
/// Fighter.input's, zero at or inside it.
const STICK_DEAD_ZONE: f32 = 0.275;
/// HSD's analog trigger scale (HSD_PadScale): raw 140 is 1.0.
const TRIGGER_MAX: f32 = 140.0;

/// The replay as a retail recording's input schedule: a header with the
/// match setup (the Game Start seed and the pre-music boundary seed it
/// reaches, spawn rule, timer, stage, each player's slot, kind and
/// costume), then one line per player and tick with the raw
/// `PADStatus` values the tick input clock injects
/// (`harness/slippi_to_scenario.py`). `tick` is the scheduler tick that
/// consumes the pad: Slippi frame + 124.
///
/// Slippi before 3.15 records the raw main-stick X and the fighter's
/// dead-zoned sticks. The raw Y is the one nearest the processed value
/// whose clamped, dead-zoned pair (HSD_PadClampCheck3, then the fighter's
/// dead zone) equals the recorded pair; every pair must have one.
pub fn write_retail_inputs(
    replay: &Replay,
    root: &Path,
    setup: Setup,
    mut out: impl std::io::Write,
) -> Result<()> {
    let (setup, _, _) = resolve_setup(replay, root, setup);
    let scenario = cold_scenario(replay, root, setup)?;
    let players: Vec<_> = scenario
        .fighters
        .iter()
        .map(|f| {
            serde_json::json!({
                "slot": f.slot,
                "kind": f.kind,
                "costume": f.costume,
                "stocks": f.stocks,
                "controller_fix": f.controller_fix,
            })
        })
        .collect();
    let mut header = serde_json::json!({
        // Game Start's seed, taken at 0x8016E74C before fn_8016E730 creates
        // the Ground and Players (`make_boundary.py --game-start-seed`), and
        // the same seed after the setup's draws.
        "game_start_seed": replay.start.random_seed,
        "boundary_seed": scenario.seed,
        "spawn": scenario.spawn_name(),
        "all_characters_unlocked": scenario.all_characters_unlocked,
        "time_limit": scenario.time_limit,
        "stage": scenario.stage,
        "ticks": scenario.frames + 1,
        "players": players,
    });
    // The Stadium codes the console ran: the boundary must run them too
    // (`make_boundary.py --gecko ps-preload ps-frozen`). Other stages never
    // reach the code, so their boundaries need not carry it.
    if scenario.stage == "PokemonStadium" {
        header["stadium_preload"] = scenario.stadium_preload.into();
        header["stadium_frozen"] = scenario.stadium_frozen.into();
    }
    // Frozen Stages, on the stages it changes (`--gecko frozen-stages`).
    if crate::replay_stage_codes::changes_stage(replay.start.stage) {
        header["frozen_stages"] = scenario.frozen_stages.into();
    }
    writeln!(out, "{header}")?;
    let dead = |v: f32| if v.abs() <= STICK_DEAD_ZONE { 0.0 } else { v };
    for input in &scenario.replay_inputs {
        let pad = crate::inputs::replay_pad(input)?;
        let derived = pad.raw_sticks();
        let mut stick = input.raw_stick.unwrap_or(derived.stick);
        if let (None, Some(x)) = (input.raw_stick, input.raw_stick_x) {
            let y = (-128i16..=127)
                .map(|y| y as i8)
                .filter(|&y| {
                    let clamped = melee_ft::input::pad::normalize_stick(x, y);
                    dead(clamped.x) == input.stick[0] && dead(clamped.y) == input.stick[1]
                })
                .min_by_key(|&y| (i16::from(y) - i16::from(derived.stick[1])).abs());
            let Some(y) = y else {
                anyhow::bail!(
                    "tick {} port {}: no raw stick Y gives {:?} with raw X {x}",
                    input.frame + 1,
                    input.port,
                    input.stick
                );
            };
            stick = [x, y];
        }
        let cstick = input.raw_cstick.unwrap_or(derived.cstick);
        let trigger = |value: f32| -> Result<u8> {
            let raw = (value * TRIGGER_MAX).round();
            ensure!(
                raw / TRIGGER_MAX == value,
                "trigger {value} is not a multiple of 1/140"
            );
            Ok(raw as u8)
        };
        writeln!(
            out,
            "{}",
            serde_json::json!({
                "tick": input.frame + 1,
                "slot": input.port,
                "button": input.buttons_physical,
                "stick": stick,
                "cstick": cstick,
                "triggers": [trigger(input.triggers[0])?, trigger(input.triggers[1])?],
            })
        )?;
    }
    Ok(())
}

fn action_name(id: u16) -> String {
    melee_types::CommonMotionState::try_from(i32::from(id))
        .map(|state| format!("{state:?} ({id})"))
        .unwrap_or_else(|_| format!("character action {id}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use melee_lib::ControllerFix::{Ucf073, Ucf074, Ucf080, Ucf084};

    #[test]
    fn ucf_version_follows_slippi_code_history() {
        // (start day, the build's version, the other a setup may have run)
        for (day, dated, other) in [
            ("2019-04-20", Ucf073, None),
            ("2019-09-23", Ucf073, None),
            // 0.74 reached Dolphin and the toggle set, then the console.
            ("2019-09-24", Ucf073, Some(Ucf074)),
            ("2019-10-08", Ucf073, Some(Ucf074)),
            ("2019-10-09", Ucf074, Some(Ucf073)),
            ("2020-02-08", Ucf074, Some(Ucf073)),
            ("2021-03-30", Ucf074, Some(Ucf073)),
            ("2021-03-31", Ucf080, None),
            ("2024-01-31", Ucf080, None),
            ("2024-02-01", Ucf084, None),
        ] {
            assert_eq!(ucf_by_date(day), (dated, other), "{day}");
        }
    }
}
