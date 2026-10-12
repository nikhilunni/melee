//! A console's lag frames, read from a replay's frames.
//!
//! A frame that overruns its retrace leaves two pad polls queued, and the
//! next pass of gm_801A4D34 runs two ticks before one display pass
//! (lb_80019894 reads the queue count once per loop pass, 0x800198B0). The
//! display pass writes game state (`docs/SLIPPI.md`, "Screen KO positions",
//! "The magnifier's damage tick", "A lag frame on Fountain of Dreams"), so
//! the second tick differs from the same tick after a pass. A replay does
//! not record display passes and the runner renders before every tick; a
//! console's lag frame then shows as a stop with nothing wrong in the port.
//!
//! The comparison may explain such a stop, under one rule. It takes the
//! stopping tick again (or, where the pass's effect shows later, the
//! earlier tick the stop names: [`Plan`]) with no display pass before it,
//! from a copy of the match kept a few ticks back, and accepts the lag
//! frame only if
//!
//! 1. every frame from that tick through the stop then matches the
//!    recording completely, Pre Frame seeds included, and
//! 2. so does every frame of a margin after the stop that depends on what
//!    the pass wrote ([`LagKind`]).
//!
//! A skipped pass that only moves the stop later is not a lag frame: the
//! run goes back to what it was and the stop is reported as it would have
//! been. Each accepted lag frame is listed in the report with its tick and
//! this evidence. The simulation is not told anything: a lag frame is one
//! `Simulation::skip_display_pass`, the same call `--skip-display-pass`
//! makes.
use crate::replay::Setup;
use crate::replay_game::{Game, RunId, REWIND_REACH};
use crate::replay_observed::member_frames;
use melee_types::CommonMotionState;

/// Whether a comparison explains stops by lag frames.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum LagFrames {
    /// Infer them under the module's rule and list each in the report.
    #[default]
    Infer,
    /// A display pass before every tick, whatever the stop
    /// (`--no-lag-frames`).
    Off,
}

/// What a skipped display pass left in the recording.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LagKind {
    /// A fighter in the screen KO is placed from its camera-space position
    /// only by the display pass (ftDrawCommon_80080E18: PSMTXMultVec at
    /// 0x80080E90), so the tick after a skipped pass keeps the last
    /// placement, moved by the tick's own fall.
    ScreenKo,
    /// The off-screen bit is set by the display pass (ftLib_80086A8C from
    /// 0x80080E44) and read by the next one (ifMagnify_802FBBDC,
    /// 0x802FBD3C); Fighter_8006A360 counts ticks of it (0x8006A89C) to
    /// each point of damage. A pass skipped at the crossing delays the
    /// first point by one tick.
    Magnifier,
    /// The display pass sorts the particle lists, so a tick without one
    /// before it draws its particles' random values in another order and
    /// number.
    Particles,
}

impl LagKind {
    /// The name the JSONL carries.
    pub fn name(self) -> &'static str {
        match self {
            Self::ScreenKo => "screen-ko",
            Self::Magnifier => "magnifier",
            Self::Particles => "particles",
        }
    }
}

/// A display pass the console is inferred to have skipped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LagFrame {
    /// The tick (as the report numbers frames) with no display pass before
    /// it.
    pub tick: u64,
    pub kind: LagKind,
    /// The tick the comparison stopped at with a pass before every tick.
    pub stop: u64,
    /// The first recorded field that differed there.
    pub field: String,
    /// How many recorded frames after the stop were required to match, and
    /// did.
    pub confirmed: usize,
    /// The margin ended with the recording, not at its full length.
    pub to_end: bool,
}

impl std::fmt::Display for LagFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            tick,
            stop,
            field,
            confirmed,
            ..
        } = self;
        write!(f, "lag frame: no display pass before tick {tick} (")?;
        match self.kind {
            LagKind::ScreenKo => write!(
                f,
                "screen KO: {field} at tick {stop} is the recording's only without it"
            )?,
            LagKind::Magnifier => write!(
                f,
                "magnifier: {field} at tick {stop} is the recording's only with the \
                 off-screen count a tick late, as this pass or the next one skipped makes it"
            )?,
            LagKind::Particles => write!(
                f,
                "particle order: {field} at tick {stop} is the recording's only without it"
            )?,
        }
        let margin = match (self.kind, self.to_end) {
            (_, true) => "to the end of the recording",
            (LagKind::ScreenKo, false) => "to the end of the KO",
            (_, false) => "after it",
        };
        write!(f, "; the {confirmed} frames {margin} then match)")
    }
}

/// PlCo +0x7AC: the ticks of the off-screen count per point of magnifier
/// damage (Fighter_8006A360, 0x8006A8B4).
const MAGNIFIER_PERIOD: usize = 60;
/// The frames after a magnifier stop that must match: two periods, so the
/// delayed point (the tick after the stop) and, while the fighter stays off
/// screen, the next one both land on the recording's ticks.
const MAGNIFIER_MARGIN: usize = 2 * MAGNIFIER_PERIOD;
/// How far before a Pre Frame seed stop the skipped pass is looked for.
/// The seed check is membership in the tick's draws
/// (`replay_observed::input_seeds`), so a stream that drew a different
/// number of values can pass it for a few busy ticks before a quieter one
/// stops it: four in the one game known
/// (`SAMUS/Game_20190629T124717`, drift at 7809, stop at 7813).
const PARTICLE_REACH: usize = 8;
/// The frames after a seed stop that must match. A wrong stream passes the
/// membership test only while every tick draws more values than the stream
/// is off by; ten seconds of play hold quiet ticks, each of which demands
/// the exact seed. One skipped pass in the game above moves its stop eight
/// ticks on and is not that console's lag frame; the right one matches to
/// the end.
const PARTICLE_MARGIN: usize = 600;
/// How many lag frames one screen KO may hold: each further stop inside a
/// KO's margin is explained by the same rule or the first is not accepted.
const MAX_LAG_FRAMES_PER_KO: usize = 4;

// The magnifier's candidate is the deepest rewind.
const _: () = assert!(MAGNIFIER_PERIOD <= REWIND_REACH);

/// The screen KO's motion states: the fighter's `cur_pos` is the display
/// pass's (ftCo_800D4580 sets x2220_b7 for all of them).
fn in_screen_ko(action: u16) -> bool {
    (CommonMotionState::DeadUpFall as u16..=CommonMotionState::DeadUpFallHitCameraIce as u16)
        .contains(&action)
}

/// How a stop could be a lag frame's: which ticks to take again without a
/// display pass, and how far the recording must then match.
struct Plan {
    kind: LagKind,
    /// The ticks to try, as indices, in order; the first accepted is taken.
    candidates: Vec<usize>,
    /// The last frame of the margin (the recording may end before it).
    until: usize,
    field: String,
}

impl Plan {
    /// The plan for the run's stop at `stop`, by the fields that differ
    /// there; `None` for a stop no display pass explains.
    fn of(game: &Game, run: RunId, stop: usize) -> Option<Self> {
        let (expected, actual) = game.records(run, stop);
        let differing: Vec<&str> = expected
            .state
            .iter()
            .filter(|(key, value)| actual.state.get(*key) != Some(value))
            .map(|(key, _)| key.as_str())
            .collect();
        let field = differing.first()?.to_string();
        let members: Option<Vec<(usize, &str)>> =
            differing.iter().map(|key| member_field(key)).collect();
        let fields = members?;
        let all = |test: fn(&str) -> bool| fields.iter().all(|(_, field)| test(field));
        if all(|field| field.starts_with("cur_pos.")) {
            // Every fighter that differs is in the screen KO; the margin is
            // the rest of each one's KO and the frame it leaves on.
            let mut until = stop;
            for &(member, _) in &fields {
                let action = |index: usize| recorded_action(game, index, member);
                if !action(stop).is_some_and(in_screen_ko) {
                    return None;
                }
                let left = (stop + 1..game.ticks())
                    .find(|&index| !action(index).is_some_and(in_screen_ko))
                    .unwrap_or(game.ticks() - 1);
                until = until.max(left);
            }
            return Some(Self {
                kind: LagKind::ScreenKo,
                candidates: vec![stop],
                until,
                field,
            });
        }
        if all(|field| field == "percent") {
            // The bit the pass before tick T sets is read by the pass
            // before T + 1 and counted from tick T + 1, so the point lands
            // on T + 60: the crossing pass is sixty ticks before the stop,
            // and the pass after it (which reads the bit) delays the count
            // as well.
            let crossing = stop.checked_sub(MAGNIFIER_PERIOD)?;
            return Some(Self {
                kind: LagKind::Magnifier,
                candidates: vec![crossing, crossing + 1],
                until: stop + MAGNIFIER_MARGIN,
                field,
            });
        }
        if all(|field| field == "input_seed") {
            return Some(Self {
                kind: LagKind::Particles,
                candidates: (stop.saturating_sub(PARTICLE_REACH)..=stop).rev().collect(),
                until: stop + PARTICLE_MARGIN,
                field,
            });
        }
        None
    }
}

/// A fighter key's member (`replay_observed::MEMBERS`) and field:
/// `p1.follower.cur_pos.x` is member 3's `cur_pos.x`. `None` for a key of
/// no fighter (`rng.seed`).
fn member_field(key: &str) -> Option<(usize, &str)> {
    let (player, rest) = key.strip_prefix('p')?.split_once('.')?;
    let leader: usize = player.parse().ok()?;
    Some(match rest.strip_prefix("follower.") {
        Some(field) => (2 * leader + 1, field),
        None => (2 * leader, rest),
    })
}

/// A member's recorded Post Frame action on a tick.
fn recorded_action(game: &Game, index: usize, member: usize) -> Option<u16> {
    member_frames(game.leaders(), game.frame(index))
        .find(|(at, _)| *at == member)
        .and_then(|(_, player)| player.post.as_ref())
        .map(|post| post.action_state)
}

/// The lag frames that explain a stop, in tick order of their stops, and
/// the last frame the run was then required to match.
type Explained = (Vec<LagFrame>, usize);

/// One comparison's lag-frame inference.
pub(crate) struct Inference {
    setup: Setup,
    /// The comparison's rewindable run, once a stop has asked for one.
    run: Option<RunId>,
}

impl Inference {
    pub(crate) fn new(setup: Setup) -> Self {
        Self { setup, run: None }
    }

    /// Explain the comparison's stop at `stop` by lag frames. On success
    /// the comparison goes on with the returned run, which has no display
    /// pass before the inferred ticks and matches the recording through
    /// the returned frame. Otherwise `current` is as it was.
    pub(crate) fn explain(
        &mut self,
        game: &mut Game,
        current: RunId,
        stop: usize,
    ) -> Option<(RunId, Vec<LagFrame>, usize)> {
        let plan = Plan::of(game, current, stop)?;
        let run = match self.run {
            Some(run) => run,
            None => {
                // The stop's deepest candidate is within the copies kept.
                let from = stop.saturating_sub(2 * REWIND_REACH);
                let run = game.rewindable(self.setup, from).ok()?;
                self.run = Some(run);
                // The same setup takes the same ticks: it stops here too.
                let same = (0..stop)
                    .all(|index| game.observe(run, index) && game.matches(run, index))
                    && game.observe(run, stop)
                    && !game.matches(run, stop);
                if !same {
                    return None;
                }
                run
            }
        };
        let (found, through) = self.take_again(game, run, stop, plan, 1)?;
        Some((run, found, through))
    }

    /// Try each of the plan's ticks without its display pass; the first
    /// whose frames match through the stop and the margin is the lag
    /// frame. If none does, the run is rewound to what it was at the stop.
    fn take_again(
        &self,
        game: &mut Game,
        run: RunId,
        stop: usize,
        plan: Plan,
        in_this_ko: usize,
    ) -> Option<Explained> {
        let last = last_comparable(game);
        let until = plan.until.min(last);
        let skipped = game.lag_frames(run).to_vec();
        let restore = game.checkpoint(run, *plan.candidates.iter().min()?)?;
        // Every candidate starts from the run as it is now: a trial leaves
        // its own copies behind, with its pass skipped, so the copies are
        // taken before the first trial.
        let starts: Vec<_> = plan
            .candidates
            .iter()
            .map(|&candidate| (candidate, game.checkpoint(run, candidate)))
            .collect();
        for (candidate, checkpoint) in starts {
            let tick = game.tick_number(candidate);
            if skipped.contains(&tick) || self.setup.skipped_display_pass == Some(tick) {
                continue;
            }
            let Some(checkpoint) = checkpoint else {
                continue;
            };
            let mut lag_frames = skipped.clone();
            lag_frames.push(tick);
            game.rewind(run, &checkpoint, lag_frames);
            let matches_stop = (checkpoint.index..=stop)
                .all(|index| game.observe(run, index) && game.matches(run, index));
            if !matches_stop {
                continue;
            }
            let Some((more, through)) = self.margin(game, run, stop, until, plan.kind, in_this_ko)
            else {
                continue;
            };
            if through == stop {
                // Nothing after the stop to confirm it with.
                continue;
            }
            let mut found = vec![LagFrame {
                tick,
                kind: plan.kind,
                stop: game.tick_number(stop),
                field: plan.field.clone(),
                confirmed: through - stop,
                to_end: through == last && plan.until > last,
            }];
            found.extend(more);
            return Some((found, through));
        }
        game.rewind(run, &restore, skipped);
        assert!(
            game.observe(run, stop),
            "a rewound run takes its ticks again"
        );
        None
    }

    /// Whether the run matches every frame after `stop` through `until`.
    /// Inside a screen KO a further stop may be a further lag frame of the
    /// same KO, accepted by the same rule; anywhere else a stop inside the
    /// margin rejects the lag frame being confirmed.
    fn margin(
        &self,
        game: &mut Game,
        run: RunId,
        stop: usize,
        until: usize,
        kind: LagKind,
        in_this_ko: usize,
    ) -> Option<Explained> {
        let mut more = Vec::new();
        let mut through = stop;
        while through < until {
            let index = through + 1;
            if !game.observe(run, index) {
                return None;
            }
            if game.matches(run, index) {
                through = index;
                continue;
            }
            if kind != LagKind::ScreenKo || in_this_ko >= MAX_LAG_FRAMES_PER_KO {
                return None;
            }
            let plan = Plan::of(game, run, index).filter(|plan| plan.kind == LagKind::ScreenKo)?;
            let (found, reached) = self.take_again(game, run, index, plan, in_this_ko + 1)?;
            more.extend(found);
            through = reached;
        }
        Some((more, through))
    }
}

/// The last frame the comparison can reach: the recording's last, or the
/// one before the first tick whose recorded controller cannot be installed.
fn last_comparable(game: &Game) -> usize {
    let last = game.ticks() - 1;
    match game.unavailable() {
        Some(&(tick, _)) => (0..=last)
            .find(|&index| game.tick_number(index) == tick)
            .map_or(last, |index| index.saturating_sub(1)),
        None => last,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fighter_key_names_its_member_and_field() {
        for (key, expected) in [
            ("p0.cur_pos.x", Some((0, "cur_pos.x"))),
            ("p1.percent", Some((2, "percent"))),
            ("p1.follower.cur_pos.y", Some((3, "cur_pos.y"))),
            ("p3.input_seed", Some((6, "input_seed"))),
            ("rng.seed", None),
        ] {
            assert_eq!(member_field(key), expected, "{key}");
        }
    }

    #[test]
    fn the_screen_ko_is_the_five_states_placed_by_the_display_pass() {
        let placed: Vec<u16> = (0..20).filter(|&action| in_screen_ko(action)).collect();
        assert_eq!(placed, [6, 7, 8, 9, 10]);
    }

    #[test]
    fn a_report_line_names_the_tick_and_the_evidence() {
        let lag = |kind, to_end| LagFrame {
            tick: 5873,
            kind,
            stop: 5873,
            field: "p1.cur_pos.x".into(),
            confirmed: 126,
            to_end,
        };
        assert_eq!(
            lag(LagKind::ScreenKo, false).to_string(),
            "lag frame: no display pass before tick 5873 (screen KO: p1.cur_pos.x at tick 5873 \
             is the recording's only without it; the 126 frames to the end of the KO then match)"
        );
        assert!(lag(LagKind::Particles, true)
            .to_string()
            .ends_with("the 126 frames to the end of the recording then match)"));
        assert!(lag(LagKind::Magnifier, false)
            .to_string()
            .ends_with("the 126 frames after it then match)"));
    }
}
