//! A simulated tick as a replay's recorded fields see it.
//!
//! A Slippi frame holds a few fields of each leader (and Nana): Post
//! Frame's kind, action, position and so on, the seed the next frame starts
//! from, and Pre Frame's seed. The recorded frames are prepared once
//! ([`Expected`]) as (where to look, value) pairs, and each simulated tick
//! is kept as an [`Observation`]: the end seed, the recorded fighters'
//! snapshot values and the Pre Frame seed check. Comparing the two reads
//! values by index; no record, map or key string is built per tick.
use crate::frame::Simulation;
use crate::replay_cost::{self as cost, Part};
use melee_diff::{Record, Value};
use melee_lib::diagnostics::FighterValues;
use melee_types::snapshot::SnapValue;
use slp::Replay;

/// The most draws one tick is searched for (a tick with more is not checked).
const MAX_TICK_DRAWS: usize = 1 << 16;

/// A recorded fighter: a leader or its follower (Nana). Member
/// `2 * leader index` is the leader, the next one its follower.
pub(crate) const MEMBERS: usize = 8;
/// The fighters a member is looked for among (`p0` to `p7`).
const FIGHTERS: usize = 8;

/// What one tick of a run showed, as the recording's fields see it.
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct Observation {
    /// The simulation's seed at the end of the tick.
    pub(crate) end_seed: u32,
    /// Each member's fighter in play (found by player slot and internal
    /// kind), as an index into the run's values; `None` when the recording
    /// has no Post Frame for the member or no such fighter is in play.
    pub(crate) members: [Option<u32>; MEMBERS],
    /// The Pre Frame seed check: `Some(seen)` for each member the recording
    /// has a Pre Frame for, when the tick's draws were searched.
    pub(crate) input_seeds: [Option<u32>; MEMBERS],
}

/// Where a recorded value is found in an [`Observation`].
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Target {
    /// `rng.seed`.
    Seed,
    /// A member's snapshot value.
    Field { member: u8, index: u8 },
    /// A key no simulated record has.
    Absent,
}

/// The recorded frames as values to compare.
pub(crate) struct Expected {
    /// The path of each fighter value (`FighterValues::get`).
    paths: Vec<String>,
    kind: usize,
    player_id: usize,
    facing_dir: Option<u8>,
    entries: Vec<(Target, SnapValue)>,
    /// Each tick's range of `entries`.
    ticks: Vec<std::ops::Range<u32>>,
}

impl Expected {
    /// Every recorded frame's values, keyed by where a run's observation
    /// holds them. `paths` are a fighter's snapshot paths. `seeds_follow`
    /// is whether the next frame's Frame Start seed is this tick's last
    /// seed: not in a Slippi Online game, whose netplay code writes a new
    /// seed before every tick, so that no end-of-tick seed is recorded.
    pub(crate) fn compile(
        replay: &Replay,
        frames: &[&slp::Frame],
        paths: Vec<String>,
        seeds_follow: bool,
    ) -> Self {
        let _t = cost::time(Part::Expected);
        let index_of = |name: &str| paths.iter().position(|path| path == name);
        let mut expected = Self {
            kind: index_of("kind").expect("a fighter snapshot has its kind"),
            player_id: index_of("player_id").expect("a fighter snapshot has its player"),
            facing_dir: index_of("facing_dir").map(|index| index as u8),
            entries: Vec::new(),
            ticks: Vec::with_capacity(frames.len()),
            paths,
        };
        for (index, frame) in frames.iter().enumerate() {
            let start = expected.entries.len() as u32;
            let mut entries = std::mem::take(&mut expected.entries);
            slp::frame_values(
                replay,
                frame,
                frames.get(index + 1).copied().filter(|_| seeds_follow),
                |key, value| {
                    entries.push((expected.target(key), snap_value(value)));
                },
            )
            .expect("a frame of the match");
            expected.entries = entries;
            expected.ticks.push(start..expected.entries.len() as u32);
        }
        expected
    }

    fn target(&self, key: slp::FrameKey) -> Target {
        match key {
            slp::FrameKey::Seed => Target::Seed,
            slp::FrameKey::Field {
                leader,
                follower,
                path,
            } => self
                .paths
                .iter()
                .position(|known| known == path)
                .filter(|_| leader < MEMBERS / 2)
                .map_or(Target::Absent, |index| Target::Field {
                    member: (2 * leader + usize::from(follower)) as u8,
                    index: index as u8,
                }),
        }
    }

    /// A tick's recorded values, each with where an observation holds it.
    pub(crate) fn entries(&self, index: usize) -> &[(Target, SnapValue)] {
        let range = &self.ticks[index];
        &self.entries[range.start as usize..range.end as usize]
    }

    /// Whether fighter value `index` is one whose zero sign an online
    /// game's comparison leaves free (`replay_online::ZERO_SIGN_FREE`).
    pub(crate) fn zero_sign_free(&self, index: u8) -> bool {
        crate::replay_online::ZERO_SIGN_FREE.contains(&self.paths[usize::from(index)].as_str())
    }

    /// The record key of a member's value (`pN.<path>`).
    pub(crate) fn key(&self, target: Target) -> Option<String> {
        match target {
            Target::Field { member, index } => Some(format!(
                "{}{}",
                member_prefix(usize::from(member)),
                self.paths[usize::from(index)]
            )),
            Target::Seed | Target::Absent => None,
        }
    }

    /// Where a leader's `facing_dir` is.
    pub(crate) fn facing(&self, leader: usize) -> Option<Target> {
        Some(Target::Field {
            member: (2 * leader) as u8,
            index: self.facing_dir?,
        })
    }

    /// What the simulation's last tick showed; the recorded fighters'
    /// values are appended to `values`. Slippi before 3.4.0 sampled each
    /// fighter at the end of its map proc, so that sample stands for the
    /// fighter when the simulation captures it.
    pub(crate) fn observe(
        &self,
        simulation: &Simulation,
        previous_seed: u32,
        leaders: &[usize],
        frame: &slp::Frame,
        values: &mut Vec<FighterValues>,
    ) -> Observation {
        let count = simulation.fighter_count().min(FIGHTERS);
        let mut fighters: [Option<FighterValues>; FIGHTERS] = [None; FIGHTERS];
        for (index, fighter) in fighters.iter_mut().enumerate().take(count) {
            *fighter = Some(simulation.fighter_values(index));
        }
        for &(index, sample) in simulation.after_map_values().unwrap_or_default() {
            if index < count {
                fighters[index] = Some(sample);
            }
        }
        let end_seed = simulation.seed();
        let mut observation = Observation {
            end_seed,
            members: [None; MEMBERS],
            input_seeds: [None; MEMBERS],
        };
        let mut recorded_seeds = [None; MEMBERS];
        for (member, player) in member_frames(leaders, frame) {
            recorded_seeds[member] = player.pre.as_ref().map(|pre| pre.random_seed);
            let Some(post) = player.post.as_ref() else {
                continue;
            };
            let slot = SnapValue::U64(leaders[member / 2] as u64);
            let kind = SnapValue::I64(i64::from(post.internal_character));
            let found = fighters.iter().flatten().find(|fighter| {
                fighter.get(self.player_id).as_ref() == Some(&slot)
                    && fighter.get(self.kind).as_ref() == Some(&kind)
            });
            if let Some(fighter) = found {
                observation.members[member] = Some(values.len() as u32);
                values.push(*fighter);
            }
        }
        observation.input_seeds = input_seeds(previous_seed, end_seed, recorded_seeds);
        observation
    }

    /// The recorded and the simulated record of an observed tick, as a
    /// report prints them: `pN` is the Nth leader's fighter in play (a
    /// player's other fighters, such as Nana or Zelda's sleeping form,
    /// shift fighter-list indices), Nana's frame is keyed `pN.follower`,
    /// and both carry the Pre Frame seed check as `pN.input_seed`.
    /// `member` gives a member's observed values.
    pub(crate) fn records<'v>(
        &self,
        replay: &Replay,
        leaders: &[usize],
        frame: &slp::Frame,
        next: Option<&slp::Frame>,
        observation: &Observation,
        member: impl Fn(usize) -> Option<&'v FighterValues>,
    ) -> (Record, Record) {
        let mut recorded = slp::frame_record(replay, frame, next).expect("a frame of the match");
        let mut state = std::collections::BTreeMap::new();
        state.insert(
            "rng.seed".to_string(),
            Value::UInt(u64::from(observation.end_seed)),
        );
        for (at, player) in member_frames(leaders, frame) {
            let prefix = member_prefix(at);
            if let Some(values) = member(at) {
                for (index, path) in self.paths.iter().enumerate() {
                    let value = values.get(index).expect("every fighter emits every path");
                    state.insert(format!("{prefix}{path}"), Value::from(value));
                }
            }
            if let (Some(seen), Some(pre)) = (observation.input_seeds[at], player.pre.as_ref()) {
                let key = format!("{prefix}input_seed");
                recorded
                    .state
                    .insert(key.clone(), Value::UInt(u64::from(pre.random_seed)));
                state.insert(key, Value::UInt(u64::from(seen)));
            }
        }
        let simulated = Record {
            frame: recorded.frame,
            phase: recorded.phase.clone(),
            state,
        };
        (recorded, simulated)
    }
}

/// Each member of the leaders' ports with its recorded frame.
pub(crate) fn member_frames<'f>(
    leaders: &'f [usize],
    frame: &'f slp::Frame,
) -> impl Iterator<Item = (usize, &'f slp::PlayerFrame)> {
    leaders.iter().enumerate().flat_map(move |(index, &port)| {
        let ports = &frame.ports[port];
        [(2 * index, &ports.leader), (2 * index + 1, &ports.follower)]
    })
}

/// A member's key prefix: `pN.` or `pN.follower.`.
fn member_prefix(member: usize) -> String {
    if member.is_multiple_of(2) {
        format!("p{}.", member / 2)
    } else {
        format!("p{}.follower.", member / 2)
    }
}

/// Pre Frame's seed (hook 0x8006B0E0, in the fighter's input proc) must be
/// one the port's stream passes through during the same tick, from the
/// previous tick's end seed (`from`) to this one's (`to`). The port does not
/// keep retail's draw timing inside a tick, only the sequence, so the test
/// is membership, not position. A stream that has left retail's fails it on
/// the first quiet tick, which names the drift long before a fighter field
/// shows it. `recorded` is each member's Pre Frame seed; the value reported
/// for a seed the tick did not pass through is the tick's first seed.
fn input_seeds(from: u32, to: u32, recorded: [Option<u32>; MEMBERS]) -> [Option<u32>; MEMBERS] {
    let mut passed = [false; MEMBERS];
    let mut mark = |seed: u32| {
        for (passed, recorded) in passed.iter_mut().zip(&recorded) {
            *passed |= *recorded == Some(seed);
        }
    };
    let mut rng = gekko_math::rng::HsdRng { seed: from };
    mark(from);
    let mut draws = 0;
    while rng.seed != to {
        if draws >= MAX_TICK_DRAWS {
            return [None; MEMBERS];
        }
        rng.rand();
        draws += 1;
        mark(rng.seed);
    }
    std::array::from_fn(|member| {
        recorded[member].map(|seed| if passed[member] { seed } else { from })
    })
}

/// A trace value as the snapshot value it compares equal to.
fn snap_value(value: Value) -> SnapValue {
    match value {
        Value::Int(v) => SnapValue::I64(v),
        Value::UInt(v) => SnapValue::U64(v),
        Value::F32 { bits, .. } => SnapValue::F32(bits),
        Value::F64 { bits, .. } => SnapValue::F64(bits),
        Value::Str(text) => SnapValue::Str(text),
        Value::Null => SnapValue::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn after(seed: u32, draws: usize) -> u32 {
        let mut rng = gekko_math::rng::HsdRng { seed };
        for _ in 0..draws {
            rng.rand();
        }
        rng.seed
    }

    /// Two members' recorded Pre Frame seeds.
    fn recorded(seeds: [Option<u32>; 2]) -> [Option<u32>; MEMBERS] {
        let mut recorded = [None; MEMBERS];
        (recorded[0], recorded[2]) = (seeds[0], seeds[1]);
        recorded
    }

    #[test]
    fn a_pre_frame_seed_is_one_the_tick_passed_through() {
        let from = 0x1234_5678;
        let to = after(from, 5);
        // (recorded seeds, what the check reports for each)
        for (seeds, seen) in [
            // The tick's first seed, one in its middle and its last.
            (
                [Some(from), Some(after(from, 3))],
                [Some(from), Some(after(from, 3))],
            ),
            ([Some(to), None], [Some(to), None]),
            // A seed past the tick's end, or not on the stream: the first.
            ([Some(after(from, 6)), Some(7)], [Some(from), Some(from)]),
        ] {
            let checked = input_seeds(from, to, recorded(seeds));
            assert_eq!([checked[0], checked[2]], seen, "{seeds:x?}");
            assert_eq!(checked[1], None);
        }
        // A quiet tick passes through its one seed.
        let checked = input_seeds(from, from, recorded([Some(from), Some(to)]));
        assert_eq!([checked[0], checked[2]], [Some(from), Some(from)]);
    }

    #[test]
    fn recorded_keys_name_a_member_and_a_snapshot_value() {
        let paths = ["kind", "player_id", "facing_dir", "cur_pos.x"];
        let expected = Expected {
            paths: paths.iter().map(|path| path.to_string()).collect(),
            kind: 0,
            player_id: 1,
            facing_dir: Some(2),
            entries: Vec::new(),
            ticks: Vec::new(),
        };
        let field = |member, index| Target::Field { member, index };
        let key = |leader, follower, path| slp::FrameKey::Field {
            leader,
            follower,
            path,
        };
        for (key, target) in [
            (slp::FrameKey::Seed, Target::Seed),
            (key(0, false, "kind"), field(0, 0)),
            (key(1, false, "cur_pos.x"), field(2, 3)),
            (key(1, true, "facing_dir"), field(3, 2)),
            // Recorded, but no snapshot value: never equal.
            (key(0, false, "shield"), Target::Absent),
            (key(0, true, "shield"), Target::Absent),
        ] {
            assert!(expected.target(key) == target, "{key}");
        }
    }
}
