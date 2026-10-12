//! Slippi Online (netplay) games: one more setup of a replay.
//!
//! What the netplay build changes, from slippi-ssbm-asm (logic described,
//! not copied; GPL-3; paths at 0e90140, the build of 2020-07-21, unless a
//! later commit is named):
//!
//! - **The seed is rewritten every frame.** `Online/Core/InitOnlinePlay.asm`
//!   (0x8016E748, the instruction before Game Start's seed is recorded at
//!   0x8016E74C) stores the negotiated RNG offset as the seed, so the
//!   Ground and Players are created from it as in any game, and adds a GObj
//!   (class 4, p_link 7) whose proc `FN_SyncRNG` runs at s_link 0, before
//!   any fighter proc, on every frame: seed = scene frame counter
//!   (0x80479D60) turned left sixteen bits, plus the offset. Frame Start
//!   records the seed after that write. Nothing a tick draws reaches the
//!   next tick's stream.
//! - **A rollback runs ticks again without rendering.**
//!   `Online/Core/StartEngineLoop.asm` (0x801A4DE4) loads the state saved at
//!   the first predicted frame; `LoopEngineForRollback.asm` (0x801A5014)
//!   then loops gm_801A4D34's tick body up to the frame the game had
//!   reached plus the new one, calling only Camera_8002A4AC and
//!   ftLib_80086A8C between ticks where the display pass would have run.
//!   The recorder sends every tick it runs, so a replay holds a frame once
//!   per simulation of it; the last one is the game's.
//! - **Codes.** The replay's Gecko list names them ([`Listed`]).
//!
//! The runner therefore gives an online game three things no other setup
//! has, none of them state of the simulated match: each tick's first seed,
//! taken from Frame Start as the netplay code wrote it; for a tick that a
//! rollback ran again, the rollback loop's camera work in place of the
//! display pass; and the listed codes instead of codes read from frames.
use melee_lib::slippi::Screen;
use slp::Replay;

/// The recorded fields whose zeros an online game's comparison takes as
/// equal whatever their sign: the knockback velocity (Post Frame's attack
/// speed, fp+0x8C / +0x90).
///
/// An online game is not computed by a console. Slippi's Dolphin runs
/// netplay in deterministic mode, where its JIT computes `fnmsub(s)` as
/// `b - a * c` (project-slippi/Ishiiruka, branch `slippi`,
/// `Source/Core/Core/PowerPC/Jit64/Jit_FloatingPoint.cpp`, `fmaddXX`: "We
/// implement nmsub a little differently ((b - a*c) instead of
/// -(a*c - b))"). The Gekko negates the difference, so an exact
/// cancellation is -0 on a console and +0 there. Fighter_procUpdate decays
/// airborne knockback with two such instructions (0x8006B9C8, 0x8006B9E4):
/// a fighter knocked along the floor that leaves it keeps `kb_vel.y` at
/// -0 on a console (as the port computes) and +0 in an online game.
/// Recordings show it: airborne with a nonzero x, consoles record -0 on
/// 2675 frames and +0 on 468 (26 games, Slippi 3.5 on), the 15 online
/// games -0 on 9 and +0 on 1463. Nothing follows from the sign there: the
/// angle is atan2f(+-0, x > 0), whose sine is +0 either way.
///
/// The port keeps the console's arithmetic. The comparison counts each
/// zero it took as equal (`OnlineReport::zeros_of_the_other_sign`) and
/// `--strict-zeros` compares them by sign.
pub const ZERO_SIGN_FREE: [&str; 2] = ["kb_vel.x", "kb_vel.y"];

/// Whether two f32 bit patterns are zeros of opposite sign.
pub(crate) fn zeros_of_opposite_sign(recorded: u32, seen: u32) -> bool {
    (recorded | seen) << 1 == 0 && recorded != seen
}

pub use melee_lib::slippi::online_sync_seed as sync_seed;

/// What shows a replay to be an online game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnlineReason {
    /// Game Start's major scene is 8 (Slippi 3.7.0 on).
    Scene,
    /// Game Start has no scene field, and every Frame Start seed is the
    /// sync's for its frame: Game Start's seed plus the frame counter
    /// turned sixteen bits. No offline stream does that twice running.
    Seeds,
}

/// An online game's per-tick facts, in the order of its recorded frames.
#[derive(Debug, Clone)]
pub struct Online {
    pub reason: OnlineReason,
    /// The RNG offset: Game Start's seed.
    pub offset: u32,
    /// Each tick's Frame Start seed.
    seeds: Vec<u32>,
    /// Whether a rollback ran the tick's last version in its loop, with no
    /// display pass since the tick before.
    resimulated: Vec<bool>,
    /// How many frames the game sent more than once.
    pub frames_resent: usize,
    /// The most frames one rollback went back.
    pub longest_rollback: usize,
}

impl Online {
    /// The replay's online facts; `None` for an offline game or one without
    /// a Frame Start for every frame.
    pub fn of(replay: &Replay) -> Option<Self> {
        let offset = replay.start.random_seed;
        let frames: Vec<&slp::Frame> = replay
            .frames
            .values()
            .filter(|frame| slp::harness_frame(frame.number).is_some())
            .collect();
        let seeds: Vec<u32> = frames
            .iter()
            .map(|frame| frame.scheduler_start_seed())
            .collect::<Option<_>>()?;
        let synced = |(index, seed): (usize, &u32)| *seed == sync_seed(index as u32, offset);
        let reason = match replay.start.major_scene {
            Some(_) if replay.start.is_online() => OnlineReason::Scene,
            Some(_) => return None,
            None if seeds.len() > 1 && seeds.iter().enumerate().all(synced) => OnlineReason::Seeds,
            None => return None,
        };
        let first = frames.first()?.number;
        let (resimulated, frames_resent, longest_rollback) =
            rollbacks(&replay.frame_sends, first, frames.len());
        Some(Self {
            reason,
            offset,
            seeds,
            resimulated,
            frames_resent,
            longest_rollback,
        })
    }

    /// The seed tick `index` starts from.
    pub fn seed(&self, index: usize) -> u32 {
        self.seeds[index]
    }

    /// Whether tick `index` ran in a rollback's loop.
    pub fn resimulated(&self, index: usize) -> bool {
        self.resimulated[index]
    }

    pub fn ticks_resimulated(&self) -> usize {
        self.resimulated.iter().filter(|&&again| again).count()
    }

    /// How many supplied seeds are not the sync's for the tick's own count
    /// (`sync_seed(index, offset)`): the scene frame counter left the tick
    /// count (it runs on through a pause) or the seed is not the sync's.
    pub fn seeds_off_the_tick_count(&self) -> usize {
        let off = |(index, seed): (usize, &u32)| *seed != sync_seed(index as u32, self.offset);
        self.seeds
            .iter()
            .enumerate()
            .filter(|&pair| off(pair))
            .count()
    }
}

impl std::fmt::Display for Online {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let reason = match self.reason {
            OnlineReason::Scene => "Game Start's scene",
            OnlineReason::Seeds => "every Frame Start seed is the netplay sync's",
        };
        write!(
            f,
            "online ({reason}): each tick starts from its Frame Start seed, {} of {} the sync's \
             for the tick count (offset {:08X}); {} frames sent again, {} ticks run in a rollback's \
             loop (longest rollback {})",
            self.seeds.len() - self.seeds_off_the_tick_count(),
            self.seeds.len(),
            self.offset,
            self.frames_resent,
            self.ticks_resimulated(),
            self.longest_rollback,
        )
    }
}

/// From the order the frames were sent in: for each of `count` frames from
/// `first`, whether its last version ran in a rollback's loop; then how
/// many frames were sent more than once and the longest step back.
///
/// A rollback loads the state saved at frame S and runs S up to the newest
/// frame F in one pass of gm_801A4D34, F being one past the newest the game
/// had run (TriggerSendInput.asm stores the frame being polled as the
/// rollback's end). So the stream steps back to S and climbs to F. Ticks
/// S + 1 to F have no display pass before them. S itself starts from the
/// saved state, taken at the top of the loop the last time S ran: what
/// preceded S then precedes it now.
fn rollbacks(sends: &[i32], first: i32, count: usize) -> (Vec<bool>, usize, usize) {
    let mut resimulated = vec![false; count];
    let mut sent = vec![0usize; count];
    let mut newest = i32::MIN;
    let mut previous = None;
    // The newest frame of the rollback pass now running, if one is.
    let mut pass_end = None;
    let mut longest = 0;
    for &frame in sends {
        let stepped_back = previous.is_some_and(|previous| frame <= previous);
        if stepped_back {
            longest = longest.max((newest - frame + 1).max(0) as usize);
            pass_end = Some(newest.saturating_add(1));
        }
        let slot = usize::try_from(i64::from(frame) - i64::from(first))
            .ok()
            .filter(|&slot| slot < count);
        if let Some(slot) = slot {
            sent[slot] += 1;
            match pass_end {
                // The frame the rollback loaded keeps what it had.
                Some(_) if stepped_back => {}
                Some(end) if frame <= end => resimulated[slot] = true,
                _ => resimulated[slot] = false,
            }
        }
        if pass_end.is_some_and(|end| frame >= end) {
            pass_end = None;
        }
        newest = newest.max(frame);
        previous = Some(frame);
    }
    let resent = sent.iter().filter(|&&times| times > 1).count();
    (resimulated, resent, longest)
}

/// The codes an online game's Gecko list names, as the port models them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Listed {
    /// `melee_lib::slippi::SlippiCodes::frozen_stages`, for this game's
    /// stage: on Final Destination the write `0421AAE4 60000000` ("Disable
    /// FD Background Transitions", in the netplay list's required codes);
    /// on Pokémon Stadium the 2020 netplay hook at 0x801D4578
    /// (`Online/Core/PreventFileAlarms/FreezeStadium.asm`), which leaves
    /// grStadium_801D4548 after its first call, before anything of the
    /// transformation controller: the same as the code's nop of the call.
    pub frozen_stages: bool,
    /// The Widescreen code of the recording player's Dolphin.
    pub screen: Screen,
    /// `Online/Core/BrawlOffscreenDamage.asm` (0x8006A880).
    pub offscreen_damage_by_camera_limits: bool,
}

const NOP: u32 = 0x6000_0000;
/// `li r0, 100` and `li r0, 540`: the Widescreen code's on-screen bounds.
const WIDE_LEFT: u32 = 0x3800_0064;
const WIDE_RIGHT: u32 = 0x3800_021C;
/// The two floats of the Widescreen code's CObj hook (0x8036A4A8) that
/// make its aspect factor: 320 / 219, or 92 / 63 until ef8e734.
const WIDE_320_219: [u8; 8] = [0x43, 0xA0, 0x00, 0x00, 0x43, 0x5B, 0x00, 0x00];
const WIDE_92_63: [u8; 8] = [0x42, 0xB8, 0x00, 0x00, 0x42, 0x7C, 0x00, 0x00];

/// Netplay codes that change the simulated match and that the port does
/// not run: slippi-ssbm-asm `Output/InjectionLists/list_netplay.json`'s
/// `affects-gameplay` entries at fcf47f1 that have no port, and one
/// optional code. A game whose list holds one is refused by name.
const UNPORTED: [(u32, &str); 13] = [
    (0x800D_4C1C, "FreezeDeadUpFallPhysics (InitHitVelocity)"),
    (0x800D_4D68, "FreezeDeadUpFallPhysics (UpdateFallVelocity)"),
    (0x8008_0E80, "FreezeDeadUpFallPhysics (UpdateModelPos)"),
    (0x801C_65C8, "Stadium CustomZeroBuffer"),
    (0x801D_4760, "Stadium GrPsxIsValid"),
    (0x801D_457C, "Stadium IngameCheckIfFrozen"),
    (0x8001_65AC, "StadiumFileLoad"),
    (0x8008_653C, "WhispyBlowDirFix"),
    (0x800A_C5B8, "NanaDeterminism"),
    (0x801D_24FC, "PSCameraIndependentMonitor"),
    (0x800D_B880, "PreventWobbling"),
    (0x8008_F090, "PreventWobbling (Wobble Check)"),
    (0x8003_0E44, "Disable Screen Shake"),
];

/// "Disable FD Background Transitions": the phase update's call.
const FD_PHASE_UPDATE_CALL: u32 = 0x8021_AAE4;
/// FreezeStadium.asm's hook in grStadium_801D4548.
const STADIUM_CONTROLLER_HOOK: u32 = 0x801D_4578;
/// ftNn_Init_80123954 + 0x54: External/FreezeGlitchFix, an Ice Climbers
/// code the port does not run.
const FREEZE_GLITCH_FIX: u32 = 0x8012_39A8;
const OFFSCREEN_DAMAGE_HOOK: u32 = 0x8006_A880;
const WIDE_COBJ_HOOK: u32 = 0x8036_A4A8;
const WIDE_LEFT_BOUND: u32 = 0x8003_0C7C;
const WIDE_RIGHT_BOUND: u32 = 0x8003_0C88;
/// Slippi stage ids.
const POKEMON_STADIUM: u16 = 3;
const FINAL_DESTINATION: u16 = 32;
/// The external character id of the Ice Climbers.
const ICE_CLIMBERS: u8 = 14;

impl Listed {
    /// Read the game's codes from its Gecko list, or say why the port
    /// cannot run the game its list describes.
    pub fn of(replay: &Replay) -> Result<Self, Vec<String>> {
        let Some(table) = replay.gecko_list.as_deref() else {
            return Err(vec!["an online game without a Gecko list".into()]);
        };
        let mut reasons: Vec<String> = UNPORTED
            .iter()
            .filter(|(address, _)| slp::gecko::codes(table).any(|code| code.address() == *address))
            .map(|(address, name)| format!("netplay code {name} at {address:08X} is not ported"))
            .collect();
        let ice_climbers = replay
            .leader_ports()
            .any(|port| replay.start.players[port].character == ICE_CLIMBERS);
        if ice_climbers && slp::gecko::codes(table).any(|code| code.address() == FREEZE_GLITCH_FIX)
        {
            reasons.push(format!(
                "netplay code FreezeGlitchFix at {FREEZE_GLITCH_FIX:08X} is not ported (Ice Climbers)"
            ));
        }
        let stage = replay.start.stage;
        let frozen_stages = if stage == FINAL_DESTINATION {
            match slp::gecko::write_at(table, FD_PHASE_UPDATE_CALL) {
                Some(NOP) => true,
                None => {
                    slp::gecko::injection_at(table, FD_PHASE_UPDATE_CALL).is_some() && {
                        // DesyncProofBGTransformations.asm (2021 on) hooks the
                        // same call with other behaviour.
                        reasons.push(format!(
                        "netplay code DesyncProofBGTransformations at {FD_PHASE_UPDATE_CALL:08X} \
                         is not ported"
                    ));
                        false
                    }
                }
                Some(other) => {
                    reasons.push(format!("write {other:08X} at {FD_PHASE_UPDATE_CALL:08X}"));
                    false
                }
            }
        } else {
            stage == POKEMON_STADIUM
                && slp::gecko::injection_at(table, STADIUM_CONTROLLER_HOOK).is_some()
        };
        let bounds = (
            slp::gecko::write_at(table, WIDE_LEFT_BOUND),
            slp::gecko::write_at(table, WIDE_RIGHT_BOUND),
        );
        let aspect = slp::gecko::injection_at(table, WIDE_COBJ_HOOK);
        let has = |floats: &[u8; 8]| {
            aspect.is_some_and(|body| body.windows(8).any(|window| window == floats))
        };
        let screen = match (aspect, bounds) {
            (None, (None, None)) => Screen::Standard,
            (Some(_), (Some(WIDE_LEFT), Some(WIDE_RIGHT))) if has(&WIDE_320_219) => {
                Screen::Widescreen
            }
            (Some(_), (Some(WIDE_LEFT), Some(WIDE_RIGHT))) if has(&WIDE_92_63) => {
                Screen::Widescreen92By63
            }
            _ => {
                reasons.push("a Widescreen code of another form than the two ported".into());
                Screen::Standard
            }
        };
        if !reasons.is_empty() {
            return Err(reasons);
        }
        Ok(Self {
            frozen_stages,
            screen,
            offscreen_damage_by_camera_limits: slp::gecko::injection_at(
                table,
                OFFSCREEN_DAMAGE_HOOK,
            )
            .is_some(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_zeros_of_opposite_sign_are_taken_as_equal() {
        let (plus, minus) = (0.0f32.to_bits(), (-0.0f32).to_bits());
        // (recorded, the port's, taken as equal)
        for (recorded, seen, free) in [
            (plus, minus, true),
            (minus, plus, true),
            // Equal values need no rule.
            (plus, plus, false),
            (minus, minus, false),
            // The smallest magnitudes are not zeros.
            (plus, 1, false),
            (0x8000_0001, plus, false),
            (1.0f32.to_bits(), (-1.0f32).to_bits(), false),
        ] {
            assert_eq!(
                zeros_of_opposite_sign(recorded, seen),
                free,
                "{recorded:08X} {seen:08X}"
            );
        }
        assert_eq!(ZERO_SIGN_FREE, ["kb_vel.x", "kb_vel.y"]);
    }

    #[test]
    fn a_rollback_runs_every_tick_after_the_loaded_one_without_a_display_pass() {
        // (frames in the order sent, the ticks run in a rollback's loop)
        let cases: [(&[i32], &[usize]); 6] = [
            // No rollback.
            (&[0, 1, 2, 3], &[]),
            // Back to 1 with 2 the newest: 1 is loaded, 2 and the new frame
            // 3 follow in the loop; 4 is an ordinary frame again.
            (&[0, 1, 2, 1, 2, 3, 4], &[2, 3]),
            // The same frame alone: the loop runs it and the new one.
            (&[0, 1, 2, 2, 3, 4], &[3]),
            // Two rollbacks: the second loads frame 3, which the first ran
            // in its loop, so 3 still has no display pass before it.
            (&[0, 1, 2, 1, 2, 3, 3, 4, 5], &[2, 3, 4]),
            // The second loads frame 1, which an ordinary pass first ran.
            (&[0, 1, 2, 1, 2, 3, 1, 2, 3, 4, 5], &[2, 3, 4]),
            // Back to the first frame: 1 and the new frame 2 run in the loop.
            (&[0, 1, 0, 1, 2, 3], &[1, 2]),
        ];
        for (sends, expected) in cases {
            let count = sends.iter().max().map_or(0, |&last| last as usize + 1);
            let (resimulated, _, _) = rollbacks(sends, 0, count);
            let ticks: Vec<usize> = (0..count).filter(|&tick| resimulated[tick]).collect();
            assert_eq!(ticks, expected, "{sends:?}");
        }
        let (_, resent, longest) = rollbacks(&[0, 1, 2, 1, 2, 3, 3, 4, 5], 0, 6);
        assert_eq!((resent, longest), (3, 2));
    }
}
