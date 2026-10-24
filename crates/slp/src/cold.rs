//! Data-only bridge to a cold simulator scenario. No simulator dependency.
//! Missing setup facts are arguments, never inferred from post-frame state.
use crate::{ids, PlayerType, PreFrame, Replay, SLIPPI_FIRST_FRAME};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};

/// Slippi input fields retain their recording semantics. In particular these
/// floats are Fighter.input, not an invertible copy of HSD_PadGameStatus.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControllerFrame {
    pub frame: u64,
    pub port: u8,
    pub buttons_physical: u16,
    pub buttons_processed: u32,
    pub stick: [f32; 2],
    pub cstick: [f32; 2],
    pub trigger: f32,
    pub triggers: [f32; 2],
    pub raw_stick: Option<[i8; 2]>,
    pub raw_cstick: Option<[i8; 2]>,
    /// Raw main-stick X alone (Slippi 1.2+; Y only from 3.15): the HSD queue
    /// entry this frame consumed, which UCF's dashback and tumble read.
    #[serde(default)]
    pub raw_stick_x: Option<i8>,
}

impl ControllerFrame {
    /// A neutral pad for `port` on `frame`: what a player out of the match
    /// (Slippi writes no frames for an eliminated player) is taken to hold.
    /// `like` is the player's last recorded frame, for which raw fields the
    /// recording carries.
    pub fn neutral(frame: u64, like: &ControllerFrame) -> Self {
        Self {
            frame,
            port: like.port,
            buttons_physical: 0,
            buttons_processed: 0,
            stick: [0.0; 2],
            cstick: [0.0; 2],
            trigger: 0.0,
            triggers: [0.0; 2],
            raw_stick: like.raw_stick.map(|_| [0; 2]),
            raw_cstick: like.raw_cstick.map(|_| [0; 2]),
            raw_stick_x: like.raw_stick_x.map(|_| 0),
        }
    }

    pub fn from_pre(pre: &PreFrame) -> Result<Self> {
        Ok(Self {
            frame: crate::harness_frame(pre.frame).context("frame before match start")?,
            port: pre.player_index,
            buttons_physical: pre.buttons_physical,
            buttons_processed: pre.buttons_processed,
            stick: [pre.joystick_x, pre.joystick_y],
            cstick: [pre.cstick_x, pre.cstick_y],
            trigger: pre.trigger,
            triggers: [pre.physical_l_trigger, pre.physical_r_trigger],
            raw_stick: pre
                .raw_joystick_x
                .zip(pre.raw_joystick_y)
                .map(|(x, y)| [x, y]),
            raw_cstick: pre.raw_cstick_x.zip(pre.raw_cstick_y).map(|(x, y)| [x, y]),
            raw_stick_x: pre.raw_joystick_x,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayRules {
    pub game_start_seed: u32,
    pub time_seconds: u32,
    pub timer_type: u8,
    pub game_mode: u8,
    pub teams: bool,
    pub item_spawn_behavior: i8,
    pub item_spawn_bitfields: [u8; 5],
    pub damage_ratio: f32,
}

#[derive(Debug, Serialize)]
pub struct ColdPlayer {
    pub slot: u8,
    pub kind: String,
    pub costume: u8,
    pub spawn_point: i8,
    pub stocks: u8,
    pub controller: &'static str,
    /// The player's team in a team battle (Player Info team_id).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team: Option<u8>,
}

/// Compatible with melee-sim::Scenario. The seed is explicitly supplied at
/// B3's pre-music boundary; Game Start's distinct seed stays in replay_rules.
#[derive(Debug, Serialize)]
pub struct ColdScenario {
    pub name: String,
    pub frames: u64,
    pub seed: u32,
    pub stage: String,
    pub all_characters_unlocked: bool,
    pub fighters: Vec<ColdPlayer>,
    /// A team battle's friendly fire (StartMeleeRules +1 bit 0, the second
    /// game bitfield's low bit).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub friendly_fire: bool,
    pub replay_rules: ReplayRules,
    pub replay_inputs: Vec<ControllerFrame>,
}

/// Whether `port`'s last recorded Post Frame before `frame` left it without
/// stocks.
fn eliminated(replay: &Replay, frame: i32, port: usize) -> bool {
    replay
        .frames
        .range(..frame)
        .rev()
        .find_map(|(_, f)| f.ports[port].leader.post.as_ref())
        .is_some_and(|post| post.stocks == 0)
}

impl ColdScenario {
    pub fn from_replay(
        replay: &Replay,
        name: &str,
        boundary_seed: u32,
        all_characters_unlocked: bool,
    ) -> Result<Self> {
        ensure!(
            replay.first_frame() == Some(SLIPPI_FIRST_FRAME),
            "missing match start"
        );
        let mut replay_inputs = Vec::new();
        for (index, frame) in replay.frames.values().enumerate() {
            ensure!(
                crate::harness_frame(frame.number) == Some(index as u64),
                "frame gap at {}",
                frame.number
            );
            for port in replay.leader_ports() {
                match frame.ports[port].leader.pre.as_ref() {
                    Some(pre) => replay_inputs.push(ControllerFrame::from_pre(pre)?),
                    None => {
                        // Slippi writes no frames for a player out of
                        // stocks (a three- or four-player match goes on).
                        let leaders = replay.leader_ports().count();
                        let last = replay_inputs
                            .len()
                            .checked_sub(leaders)
                            .map(|at| replay_inputs[at].clone())
                            .filter(|_| eliminated(replay, frame.number, port))
                            .with_context(|| {
                                format!("frame {} port {port}: missing Pre Frame", frame.number)
                            })?;
                        replay_inputs.push(ControllerFrame::neutral(index as u64, &last));
                    }
                }
            }
        }
        let start = &replay.start;
        let fighters = replay
            .leader_ports()
            .map(|port| {
                let p = &start.players[port];
                Ok(ColdPlayer {
                    slot: p.port,
                    kind: ids::external_character_name(p.character)
                        .context("unknown character")?
                        .into(),
                    costume: p.costume,
                    spawn_point: p.spawn_point,
                    stocks: p.stock_start_count,
                    controller: match p.player_type {
                        PlayerType::Human => "scripted",
                        PlayerType::Cpu => "cpu",
                        PlayerType::Demo => "demo",
                        PlayerType::Empty => unreachable!(),
                    },
                    team: start.is_teams.then_some(p.team_id),
                })
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            name: name.into(),
            frames: replay.frames.len() as u64,
            seed: boundary_seed,
            stage: ids::stage_name(start.stage)
                .context("unknown stage")?
                .into(),
            all_characters_unlocked,
            fighters,
            friendly_fire: start.is_teams && start.game_bitfields[1] & 1 != 0,
            replay_inputs,
            replay_rules: ReplayRules {
                game_start_seed: start.random_seed,
                time_seconds: start.game_timer,
                timer_type: start.timer_type(),
                game_mode: start.game_mode(),
                teams: start.is_teams,
                item_spawn_behavior: start.item_spawn_behavior,
                item_spawn_bitfields: start.item_spawn_bitfields,
                damage_ratio: start.damage_ratio,
            },
        })
    }

    pub fn to_toml(&self) -> Result<String> {
        Ok(toml::to_string(self)?)
    }
}
