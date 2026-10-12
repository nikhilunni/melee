//! Cold replay comparison. Expected state never initializes or repairs a tick.
use crate::replay_screen_codes::{resolve_screen, ScreenChoice, ScreenReason};
use crate::replay_stage_codes::{resolve_stage_codes, StageCodeChoice, StageCodeReason};
use crate::replay_cost::{self as cost, Part};
use crate::replay_game::{Game, StartError, Stopped};
use crate::scenario::Scenario;
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
    /// The screen code the console ran, instead of what the recording's
    /// frames show (`replay_screen_codes`).
    pub screen: Option<melee_lib::slippi::Screen>,
    /// The tick (as the report numbers frames) the console did not render
    /// before: a lag frame. A replay does not record display passes and the
    /// runner renders every tick, so this is a diagnostic for a stop that
    /// one skipped pass explains (the magnifier, a screen KO, the particle
    /// sort); the report says it was used.
    pub skipped_display_pass: Option<u64>,
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
    /// The replay's Slippi version: no build that wrote it carried the
    /// version its date names, or the date precedes the build (an unset
    /// console clock).
    Versioned,
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
            FixReason::Versioned => write!(f, " (by Slippi version, not the date)"),
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
    /// The port panicked on this tick (a fault in the port, not a recorded
    /// difference); the frames before it matched.
    Panicked {
        tick: u64,
        message: String,
    },
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
    /// Whether the Widescreen code ran in this comparison.
    pub screen: ScreenChoice,
    /// Set when the console created Fountain of Dreams' side platforms
    /// level (`SlippiCodes::fountain_level`): what the first frame showed.
    pub fountain: Option<&'static str>,
    /// Each leader's recorded character and action at the stop.
    pub context: String,
    /// Every field that differs on the diverging frame, in key order.
    pub differing: Vec<String>,
    /// `Setup::skipped_display_pass`.
    pub skipped_display_pass: Option<u64>,
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
        writeln!(f, "screen {}", self.screen)?;
        if let Some(reason) = self.fountain {
            writeln!(f, "fountain fod-level ({reason})")?;
        }
        if let Some(tick) = self.skipped_display_pass {
            writeln!(f, "no display pass before tick {tick} (--skip-display-pass)")?;
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
            Stop::Panicked { tick, message } => write!(
                f,
                "simulation panic at tick {tick} (Slippi {}): {message}",
                *tick as i64 + i64::from(slp::SLIPPI_FIRST_FRAME)
            ),
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
                Some(_) => {}
                None => reasons.push(format!(
                    "port {} controller fix {:?}/{:?} (neither UCF nor Dween, or UCF without a \
                     start date)",
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

/// The replay version of the first build with UCF 0.8: bb86519 (2021-03-31)
/// put it in every output, console and Dolphin, with `CURRENT_VERSION`
/// 3.9.0 (Recording/Recording.s). Every `g_ucf.bin` beside an older
/// `g_core.bin` is the 0.73 beta or 0.74.
const UCF_080_VERSION: slp::Version = slp::Version::new(3, 9, 0);
/// The replay version when 0.84 was first built (422bb78, 2023-05-07, a
/// separate console file `g_ucf_084.bin`; Dolphin's list took it at
/// 6d3e140, 2023-12-18, version 3.16.0).
const UCF_084_VERSION: slp::Version = slp::Version::new(3, 14, 0);
/// The replay version of the first console core built beside 0.74
/// (0ec965e, 2019-10-24; `g_ucf.bin` is 0.74 from 823067b, 2019-10-09).
const UCF_074_VERSION: slp::Version = slp::Version::new(3, 0, 0);

/// The day slippi-ssbm-asm's `Output/Console/g_core.bin` first wrote each
/// replay version below 3.9.0 (the commit that built it), ascending. A
/// recording dated earlier than its own version was made with the console's
/// clock unset, and its date says nothing.
const VERSION_FIRST_BUILT: [(slp::Version, &str); 15] = [
    (slp::Version::new(1, 7, 1), "2019-02-25"), // e10f583
    (slp::Version::new(2, 0, 0), "2019-03-19"), // 5e7a923
    (slp::Version::new(2, 0, 1), "2019-03-29"), // 7062149
    (slp::Version::new(2, 1, 0), "2019-05-03"), // 009b155
    (slp::Version::new(2, 2, 0), "2019-07-17"), // 26ab8a3
    (slp::Version::new(3, 0, 0), "2019-10-24"), // 0ec965e
    (slp::Version::new(3, 1, 0), "2020-01-18"), // 28df917
    (slp::Version::new(3, 2, 0), "2020-01-31"), // c8403dc
    (slp::Version::new(3, 3, 0), "2020-02-11"), // bc46aa7
    (slp::Version::new(3, 4, 0), "2020-06-06"), // 9398d52
    (slp::Version::new(3, 5, 0), "2020-06-13"), // 4e3b185
    (slp::Version::new(3, 6, 0), "2020-06-20"), // 729c1dc
    (slp::Version::new(3, 7, 0), "2020-07-12"), // b230594
    (slp::Version::new(3, 8, 0), "2020-12-06"), // 4b0002b
    (slp::Version::new(3, 9, 0), "2021-02-17"), // f99bdbe
];

/// The UCF versions a recording can have run, the likelier first, and
/// whether its date or its Slippi version decided. The version is the
/// stronger rule: it is written by the build, the date by a console clock
/// that is often unset (corpus recordings say 1949, 2006, 2012, 2016) or
/// wrong. A build older than 3.9.0 ran the 0.73 beta or 0.74, whatever its
/// date; its date chooses between them only when it is one the build can
/// have (not before the version existed, not after 0.8 replaced them).
fn ucf_candidates(
    version: slp::Version,
    day: Option<&str>,
) -> Option<(
    melee_lib::ControllerFix,
    Option<melee_lib::ControllerFix>,
    FixReason,
)> {
    use melee_lib::ControllerFix::{Ucf073, Ucf074, Ucf080, Ucf084};
    if version < UCF_080_VERSION {
        let first_built = VERSION_FIRST_BUILT
            .iter()
            .rev()
            .find(|(built, _)| *built <= version)
            .map_or("", |(_, day)| day);
        if let Some(day) = day.filter(|&day| first_built <= day && day < UCF_080) {
            let (dated, other) = ucf_by_date(day);
            return Some((dated, other, FixReason::Dated));
        }
        // The version built beside this core, and the other one a setup
        // may have paired with it; the frames choose.
        let (built, other) = if version < UCF_074_VERSION {
            (Ucf073, Ucf074)
        } else {
            (Ucf074, Ucf073)
        };
        return Some((built, Some(other), FixReason::Versioned));
    }
    let (dated, other) = ucf_by_date(day?);
    if dated == Ucf084 && version < UCF_084_VERSION {
        return Some((Ucf080, None, FixReason::Versioned));
    }
    Some((dated, other, FixReason::Dated))
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

/// The UCF version a UCF port ran. Slippi records only "UCF", so bound it
/// by the replay's Slippi version and date the recording by its start time
/// against Slippi's code history ([`ucf_candidates`]);
/// [`resolve_controller_fix`] settles two candidates. `Setup::controller_fix`
/// names the version instead, for the UCF ports only: a port recorded with
/// Dween's fix (the toggle set's value 2) runs no fix, whatever the others
/// run, because its recorded sticks are the fix's output.
fn ucf_version(
    replay: &Replay,
    p: &slp::PlayerStart,
    setup: Setup,
) -> Option<melee_lib::ControllerFix> {
    use melee_lib::ControllerFix;
    if !has_controller_fix(p) {
        return Some(ControllerFix::Off);
    }
    if is_dween(p) {
        // Dween's code only rewrites the fighter's sampled stick (fp+0x620,
        // at 0x8006B028), and Pre Frame records that stick afterwards (hook
        // 0x8006B0E0): the recorded inputs already carry the fix, so the
        // port runs none. Feeding them through `ControllerFix::Dween` would
        // apply it twice.
        return Some(ControllerFix::Off);
    }
    if !is_ucf(p) {
        return None;
    }
    if setup.controller_fix.is_some() {
        return setup.controller_fix;
    }
    Some(ucf_candidates(replay.version(), start_day(replay))?.0)
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
    match ucf_candidates(replay.version(), start_day(replay)) {
        Some((dated, _, _)) => Setup {
            controller_fix: Some(dated),
            ..setup
        },
        None => setup,
    }
}

/// Game Start's Dween: Slippi's per-port toggle byte (`ControllerFixOptions`,
/// written to both fields by Recording/SendGameInfo.asm until bb86519) is 2,
/// the value the "Arduino" code at 0x8006B028 tests for.
fn is_dween(p: &slp::PlayerStart) -> bool {
    p.dashback_fix == Some(2) && p.shield_drop_fix == Some(2)
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
pub(crate) fn resolve_controller_fix(game: &mut Game, setup: Setup) -> (Setup, Option<FixChoice>) {
    let replay = game.replay;
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
    let Some((dated, other, reason)) = ucf_candidates(replay.version(), start_day(replay)) else {
        return (setup, None);
    };
    let named = |fix| Setup {
        controller_fix: Some(fix),
        ..setup
    };
    let mut choice = FixChoice { fix: dated, reason };
    if let Some(other) = other.filter(|_| unsupported_setup(replay, named(dated)).is_empty()) {
        // An unported boundary or setup error before any disagreement
        // leaves the dated version; the comparison run reports it.
        let _t = cost::time(Part::FixProbe);
        let probe = catch_unwind(AssertUnwindSafe(|| {
            first_facing_disagreement(game, named(dated), named(other))
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

/// A dry pass found no answer because a run could not start or stopped
/// (an unported boundary, a simulation error); the comparison reports it.
pub(crate) struct NoAnswer;

/// Run the replay's inputs under two setups in step, up to the first tick
/// any compared field differs between them. `None` unless a UCF leader's
/// `facing_dir` is among what differs there (a dashback's first effect).
fn first_facing_disagreement(
    game: &mut Game,
    first: Setup,
    second: Setup,
) -> Result<Option<FacingDisagreement>, NoAnswer> {
    let first = game.run(first).map_err(|_| NoAnswer)?;
    let second = game.run(second).map_err(|_| NoAnswer)?;
    let replay = game.replay;
    let last_tick = game.unavailable().map(|(tick, _)| *tick);
    for index in 0..game.ticks() {
        let tick = game.tick_number(index);
        if last_tick == Some(tick) {
            break;
        }
        if !(game.observe(first, index) && game.observe(second, index)) {
            return Err(NoAnswer);
        }
        if game.same_recorded(first, second, index) {
            continue;
        }
        let leaders = game.leaders();
        return Ok(leaders.iter().enumerate().find_map(|(leader, &port)| {
            let (a, b) = (
                game.facing(first, index, leader),
                game.facing(second, index, leader),
            );
            (is_ucf(&replay.start.players[port]) && a != b).then(|| FacingDisagreement {
                tick,
                recorded_is_second: game.recorded_facing(index, leader) == b.as_ref(),
            })
        }));
    }
    Ok(None)
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
        NeutralTable::LevelFountainEntryByOrder,
    ]
    .into_iter()
    .find(|&table| fits(table))
    .map_or(SpawnRule::Retail, SpawnRule::NeutralTable)
}

/// Whether the console created Fountain of Dreams' side platforms level
/// (`SlippiCodes::fountain_level`), and why. A replay does not name the
/// code; the consoles that ran it spawned both leaders at the level
/// platforms' rows, which is how the first frame chose the spawn rule.
fn fountain_level(replay: &Replay) -> Option<&'static str> {
    use melee_lib::slippi::{NeutralTable, SpawnRule};
    let level = SpawnRule::NeutralTable(NeutralTable::LevelFountainEntryByOrder);
    (spawn_rule(replay) == level).then_some(
        "the first frame has both leaders on rows at y = 24, the first already in \
         EntryStart, which no other spawn rule gives",
    )
}

/// A cold scenario for the replay, from Game Start's rules and the recorded
/// inputs. Never initialized or repaired from recorded state.
pub fn cold_scenario(replay: &Replay, root: &Path, setup: Setup) -> Result<Scenario> {
    ensure!(
        unsupported_setup(replay, setup).is_empty(),
        "unsupported replay setup"
    );
    let scenario = cold_scenario_base(replay, root, setup)?;
    scenario.validate()?;
    Ok(scenario)
}

/// [`cold_scenario`] for a supported setup. Only [`set_candidate`]'s fields
/// depend on which codes the setup names, so one scenario serves every
/// candidate setup of a replay.
pub(crate) fn cold_scenario_base(replay: &Replay, root: &Path, setup: Setup) -> Result<Scenario> {
    // The boundary seed is found from a provisional scenario with Game
    // Start's seed.
    let cold = ColdScenario::from_replay(replay, "slippi_replay", replay.start.random_seed, false)?;
    let mut scenario = scenario_of(cold)?;
    scenario.root = root.into();
    if replay.start.timer_type() == 2 && replay.start.game_timer != 0 {
        scenario.time_limit = Some(replay.start.game_timer);
    }
    scenario.spawn = spawn_rule(replay);
    scenario.stadium_preload = replay.version() >= STADIUM_PRELOAD;
    scenario.stadium_frozen = replay.start.frozen_ps == Some(true);
    scenario.fountain_level = fountain_level(replay).is_some();
    set_candidate(replay, &mut scenario, setup);
    scenario.validate()?;
    let (seed, unlocked) =
        cold_boundary(replay, &scenario, setup).map_err(|e| anyhow::anyhow!(e))?;
    scenario.seed = Some(seed);
    scenario.all_characters_unlocked = Some(unlocked);
    Ok(scenario)
}

/// The scenario a cold replay's TOML (`ColdScenario::to_toml`) loads as.
///
/// Printing and parsing a long game's input list costs more than starting
/// its match, so the fields are moved across instead. Every value survives
/// TOML unchanged except a NaN, whose bits the text does not carry: an
/// input list holding one goes through the text, as it always did.
fn scenario_of(cold: ColdScenario) -> Result<Scenario> {
    let floats = |input: &slp::cold::ControllerFrame| {
        [input.stick, input.cstick, input.triggers]
            .into_iter()
            .flatten()
            .chain([input.trigger])
    };
    if cold.replay_rules.damage_ratio.is_nan()
        || cold.replay_inputs.iter().flat_map(floats).any(f32::is_nan)
    {
        return Ok(toml::from_str(&cold.to_toml()?)?);
    }
    Ok(Scenario {
        name: cold.name,
        savestate: None,
        expected: None,
        all_characters_unlocked: Some(cold.all_characters_unlocked),
        frames: cold.frames,
        seed: Some(cold.seed),
        gate: None,
        boundary_seed: None,
        game_start_seed: None,
        after_map: false,
        render_every_tick: false,
        no_display_pass_before: Vec::new(),
        sudden_death: false,
        time_limit: None,
        spawn: Default::default(),
        stadium_preload: false,
        stadium_frozen: false,
        frozen_stages: false,
        widescreen: false,
        widescreen_92_63: false,
        fountain_level: false,
        stage: cold.stage,
        fighters: cold
            .fighters
            .into_iter()
            .map(|fighter| crate::scenario::FighterScenario {
                slot: fighter.slot,
                kind: fighter.kind,
                controller: fighter.controller.into(),
                costume: fighter.costume,
                spawn_point: fighter.spawn_point,
                stocks: fighter.stocks,
                controller_fix: None,
            })
            .collect(),
        inputs: Vec::new(),
        input_clock: None,
        replay_inputs: cold.replay_inputs,
        replay_rules: Some(cold.replay_rules),
        gecko: Vec::new(),
        root: Default::default(),
    })
}

/// Write the codes a setup names into the replay's cold scenario: the stage
/// code, the screen and each port's controller fix.
pub(crate) fn set_candidate(replay: &Replay, scenario: &mut Scenario, setup: Setup) {
    scenario.frozen_stages = setup.frozen_stages.unwrap_or(false);
    scenario.set_screen(setup.screen.unwrap_or_default());
    for (fighter, port) in scenario.fighters.iter_mut().zip(replay.leader_ports()) {
        fighter.controller_fix = (!setup.ignore_controller_fixes).then(|| {
            ucf_version(replay, &replay.start.players[port], setup)
                .expect("checked by unsupported_setup")
                .name()
                .to_string()
        });
    }
}

/// What a replay does not record, as chosen for one comparison.
#[derive(Debug, Clone, Copy)]
pub struct Resolved {
    pub setup: Setup,
    pub controller_fix: Option<FixChoice>,
    pub stage_code: Option<StageCodeChoice>,
    pub screen: ScreenChoice,
}

/// What a replay does not record, read from its frames: the stage code
/// (`replay_stage_codes`), the screen code (`replay_screen_codes`) and the
/// UCF version ([`resolve_controller_fix`]). Each is decided by its own dry
/// pass before the comparison, which then runs under the one chosen setup
/// from the first frame. The codes are read first, under the dated UCF
/// version, so the dashback probe runs on the console's random stream; if
/// that probe then names the other UCF version, the codes are read again
/// under it.
pub fn resolve_setup(replay: &Replay, root: &Path, setup: Setup) -> Resolved {
    resolve_setup_of(&mut Game::new(replay, root), setup)
}

/// [`resolve_setup`] over the replay's shared runs, which the comparison
/// then reads too.
fn resolve_setup_of(game: &mut Game, setup: Setup) -> Resolved {
    let (coded, stage_code, screen) = resolve_codes(game, setup);
    let (fixed, controller_fix) = resolve_controller_fix(game, coded);
    let redated = matches!(
        controller_fix,
        Some(FixChoice {
            reason: FixReason::Dashback { .. },
            ..
        })
    );
    if !redated {
        return Resolved {
            setup: fixed,
            controller_fix,
            stage_code,
            screen,
        };
    }
    let uncoded = Setup {
        frozen_stages: setup.frozen_stages,
        screen: setup.screen,
        ..fixed
    };
    let (setup, stage_code, screen) = resolve_codes(game, uncoded);
    Resolved {
        setup,
        controller_fix,
        stage_code,
        screen,
    }
}

/// The stage code, then the screen under it. A stage code the standard
/// screen could not show (the wide screen's first difference came before
/// the stage's) is read again on the wide screen.
fn resolve_codes(game: &mut Game, setup: Setup) -> (Setup, Option<StageCodeChoice>, ScreenChoice) {
    let (staged, stage_code) = resolve_stage_codes(game, setup);
    let (screened, screen) = resolve_screen(game, staged);
    let stage_unread = matches!(
        stage_code,
        Some(StageCodeChoice {
            reason: StageCodeReason::Undecided,
            ..
        })
    );
    let wide_recorded = screen.screen != melee_lib::slippi::Screen::Standard
        && matches!(screen.reason, ScreenReason::Recorded { .. });
    if !(stage_unread && wide_recorded) {
        return (screened, stage_code, screen);
    }
    let unstaged = Setup {
        frozen_stages: None,
        ..screened
    };
    let (setup, stage_code) = resolve_stage_codes(game, unstaged);
    (setup, stage_code, screen)
}

pub fn run_file(path: &Path, root: &Path, setup: Setup) -> Result<Report> {
    let replay = {
        let _t = cost::time(Part::Parse);
        Replay::parse(&std::fs::read(path)?)?
    };
    let report = run(&replay, root, setup);
    cost::report(&path.display().to_string(), replay.frames.len());
    report
}

pub fn run(replay: &Replay, root: &Path, setup: Setup) -> Result<Report> {
    ensure!(
        !replay.incomplete && !replay.frames.is_empty(),
        "incomplete or empty replay"
    );
    let mut game = Game::new(replay, root);
    let Resolved {
        setup,
        controller_fix,
        stage_code,
        screen,
    } = resolve_setup_of(&mut game, setup);
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
        screen,
        fountain: fountain_level(replay),
        context: String::new(),
        differing: Vec::new(),
        skipped_display_pass: setup.skipped_display_pass,
        stop: Stop::Complete,
    };
    let reasons = unsupported_setup(replay, setup);
    if !reasons.is_empty() {
        report.stop = Stop::Unsupported(reasons);
        return Ok(report);
    }
    let _t = cost::time(Part::Compare);
    let run = match game.comparison_run(setup) {
        Ok(run) => run,
        Err(StartError::Scenario(error)) => {
            report.stop = Stop::NeedsSetup(format!("{error:#}"));
            return Ok(report);
        }
        Err(StartError::Start(error)) => return Err(error),
    };
    // The frame of the last tick the comparison reached: its leaders'
    // recorded actions are the report's context.
    let mut reached = None;
    let stop = compare(&mut game, run, &mut report.matched, &mut reached);
    if let Some(frame) = reached {
        report.context = leader_actions(replay, frame);
    }
    match stop? {
        Compared::Complete => {}
        Compared::Stopped(stop) => report.stop = stop,
        Compared::Diverged { index } => {
            let (expected, actual) = game.records(run, index);
            let diff = compare_frame(&expected, &actual)
                .expect("a tick that does not match has a first divergence");
            report.differing = expected
                .state
                .iter()
                .filter(|(key, value)| actual.state.get(*key) != Some(value))
                .map(|(key, _)| key.clone())
                .collect();
            report.stop = Stop::Diverged(diff);
        }
    }
    Ok(report)
}

/// How the comparison of a run with the recording ended.
enum Compared {
    Complete,
    /// Tick `index` does not match the recording.
    Diverged { index: usize },
    Stopped(Stop),
}

/// Compare the run with every recorded frame in turn, counting the frames
/// that match up to the first that does not.
fn compare<'a>(
    game: &mut Game<'a>,
    run: crate::replay_game::RunId,
    matched: &mut usize,
    reached: &mut Option<&'a slp::Frame>,
) -> Result<Compared> {
    for index in 0..game.ticks() {
        let tick = game.tick_number(index);
        if let Some((unavailable, reason)) = game.unavailable() {
            if *unavailable == tick {
                return Ok(Compared::Stopped(Stop::UnavailableInput {
                    tick,
                    reason: reason.clone(),
                }));
            }
        }
        let frame = game.frame(index);
        *reached = Some(frame);
        if !game.observe(run, index) {
            let message = match game.stop(run, index).expect("a run that stopped") {
                Stopped::Error(error) => {
                    return Err(error.take().unwrap_or_else(|| {
                        anyhow::anyhow!("simulation is poisoned by a prior tick error")
                    }))
                }
                Stopped::Panic(message) => message.clone(),
            };
            if !message.starts_with("not implemented:") {
                return Ok(Compared::Stopped(Stop::Panicked { tick, message }));
            }
            return Ok(Compared::Stopped(Stop::Unported {
                tick,
                action: frame
                    .ports
                    .iter()
                    .filter_map(|p| p.leader.post.as_ref())
                    .map(|p| action_name(p.action_state))
                    .collect::<Vec<_>>()
                    .join(" / "),
                reason: message,
            }));
        }
        if !game.matches(run, index) {
            return Ok(Compared::Diverged { index });
        }
        *matched += 1;
    }
    Ok(Compared::Complete)
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
    let setup = resolve_setup(replay, root, setup).setup;
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
    // Level Fountain of Dreams platforms (`--gecko fod-level`).
    if scenario.stage == "FountainOfDreams" {
        header["fountain_level"] = scenario.fountain_level.into();
    }
    // The Widescreen code (`--gecko widescreen`, or `widescreen-92-63` for
    // its earlier aspect).
    header["widescreen"] = scenario.widescreen.into();
    header["widescreen_92_63"] = scenario.widescreen_92_63.into();
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

    /// A cold scenario whose inputs hold these floats, among them values a
    /// careless text round trip would change.
    fn cold_with(floats: [f32; 4]) -> ColdScenario {
        let input = |frame: u64, port: u8, raw: Option<[i8; 2]>| slp::cold::ControllerFrame {
            frame,
            port,
            buttons_physical: 0x1160,
            buttons_processed: 0x8000_0100,
            stick: [floats[0], floats[1]],
            cstick: [floats[2], -0.0],
            trigger: floats[3],
            triggers: [0.3, 1.0],
            raw_stick: raw,
            raw_cstick: None,
            raw_stick_x: raw.map(|[x, _]| x),
        };
        ColdScenario {
            name: "slippi_replay".into(),
            frames: 2,
            seed: 0xFEDC_BA98,
            stage: "Battlefield".into(),
            all_characters_unlocked: false,
            fighters: [(0, "Fox", "scripted"), (3, "Marth", "cpu")]
                .into_iter()
                .map(|(slot, kind, controller)| slp::cold::ColdPlayer {
                    slot,
                    kind: kind.into(),
                    costume: 2,
                    spawn_point: -1,
                    stocks: 4,
                    controller,
                })
                .collect(),
            replay_rules: slp::cold::ReplayRules {
                game_start_seed: 0x8000_0001,
                time_seconds: 480,
                timer_type: 2,
                game_mode: 1,
                teams: false,
                item_spawn_behavior: -1,
                item_spawn_bitfields: [0xFF, 0, 1, 2, 0x80],
                damage_ratio: 1.0,
            },
            replay_inputs: vec![
                input(0, 0, Some([-128, 127])),
                input(0, 3, None),
                input(1, 0, None),
                input(1, 3, Some([5, -5])),
            ],
        }
    }

    #[test]
    fn a_cold_scenario_is_its_toml_without_the_text() {
        for floats in [
            [0.0, -0.0, 1.0, -1.0],
            [0.2875, -0.7125, 0.95, 1.0 / 140.0],
            [f32::MIN_POSITIVE, f32::from_bits(1), f32::MAX, f32::EPSILON],
            [f32::INFINITY, f32::NEG_INFINITY, 0.1, 0.300_000_04],
        ] {
            let through_text: Scenario =
                toml::from_str(&cold_with(floats).to_toml().unwrap()).unwrap();
            let moved = scenario_of(cold_with(floats)).unwrap();
            assert_eq!(format!("{moved:?}"), format!("{through_text:?}"), "{floats:?}");
            let bits = |scenario: &Scenario| -> Vec<u32> {
                scenario
                    .replay_inputs
                    .iter()
                    .flat_map(|input| {
                        [input.stick, input.cstick, input.triggers]
                            .into_iter()
                            .flatten()
                            .chain([input.trigger])
                    })
                    .map(f32::to_bits)
                    .collect()
            };
            assert_eq!(bits(&moved), bits(&through_text), "{floats:?}");
        }
        // A NaN's bits are not in the text: such a list goes through it.
        let nan = [f32::NAN, 0.0, 0.0, 0.0];
        let through_text: Scenario = toml::from_str(&cold_with(nan).to_toml().unwrap()).unwrap();
        let moved = scenario_of(cold_with(nan)).unwrap();
        assert_eq!(
            moved.replay_inputs[0].stick[0].to_bits(),
            through_text.replay_inputs[0].stick[0].to_bits()
        );
    }

    #[test]
    fn slippi_version_bounds_the_ucf_version_before_the_date() {
        use FixReason::{Dated, Versioned};
        let v = slp::Version::new;
        // (replay version, start day, candidates, what decided)
        for (version, day, fixes, reason) in [
            // A date the build can have: the date chooses.
            (v(2, 0, 1), Some("2019-06-24"), (Ucf073, None), Dated),
            (v(2, 0, 1), Some("2020-03-04"), (Ucf074, Some(Ucf073)), Dated),
            (v(3, 9, 0), Some("2021-07-01"), (Ucf080, None), Dated),
            // No build older than 3.9.0 carried 0.8, whatever the clock says.
            (v(2, 0, 1), Some("2021-06-17"), (Ucf073, Some(Ucf074)), Versioned),
            (v(2, 0, 1), Some("2023-05-05"), (Ucf073, Some(Ucf074)), Versioned),
            (v(3, 7, 0), Some("2024-03-01"), (Ucf074, Some(Ucf073)), Versioned),
            // A date before the version existed is an unset clock.
            (v(2, 0, 1), Some("2016-11-20"), (Ucf073, Some(Ucf074)), Versioned),
            (v(3, 0, 0), Some("1949-01-01"), (Ucf074, Some(Ucf073)), Versioned),
            (v(3, 3, 0), Some("2019-06-01"), (Ucf074, Some(Ucf073)), Versioned),
            (v(2, 0, 1), None, (Ucf073, Some(Ucf074)), Versioned),
            // 0.84 was first built with 3.14.0.
            (v(3, 12, 0), Some("2024-03-01"), (Ucf080, None), Versioned),
            (v(3, 16, 0), Some("2024-03-01"), (Ucf084, None), Dated),
        ] {
            assert_eq!(
                ucf_candidates(version, day),
                Some((fixes.0, fixes.1, reason)),
                "{version:?} {day:?}"
            );
        }
        assert_eq!(ucf_candidates(v(3, 9, 0), None), None);
    }
}
