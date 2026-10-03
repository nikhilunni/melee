# Slippi replay tests (lane B4)

2026-09-09. `melee-sim replay FILE.slp` parses the complete replay, reports
its stage/characters/ports, cold-constructs supported setups from DAT assets,
replays controller inputs, and compares each frame using `melee-diff`'s
bit-pattern equality. It never initializes a fighter from post-frame state,
repairs the RNG between frames, or searches seeds against an expected trace.

**No bundled fixture qualifies for an actual cold replay comparison.** Seven
have an unsupported cold stage or character; after B5, `v3.16.slp` has a
supported stage/matchup but requires online RNG reconstruction. Their measured matched
counts are zero, with explicit setup reasons, not eight successful simulation
runs. Separate adapter tests project existing independent Dolphin cold
oracles into Slippi fields: **599 full frames each on FD and Battlefield**
match, including RNG. These projections are not new `.slp` fixtures.

## Commands and setup contract

```sh
cargo run -q -p melee-sim -- replay crates/slp/tests/data/v3.13.slp
# A qualifying offline English/NTSC replay, normal entry/music, controller fixes off:
cargo run -q -p melee-sim -- replay match.slp --all-characters-unlocked true
# For an old replay without Frame Start, additionally supply an independently
# measured B3 pre-music seed (decimal), never the Game Start seed by default:
cargo run -q -p melee-sim -- replay match.slp \
  --all-characters-unlocked false --boundary-seed 3427901605
```

Slippi does not record the save's unlock mask. The CLI therefore requires
`--all-characters-unlocked true/false` for an otherwise supported setup.
An unsupported fixture is reported without reading DATs; a missing setup fact
is reported separately. A ported-state mismatch prints the tick, field,
expected/actual values with hexadecimal float bits, and matched count, and
returns a nonzero exit status. Unsupported action tables and explicit
`unimplemented!` hooks stop the prefix with an explanation; arbitrary panics
and simulation errors remain failures. A corrupt late controller field stops
at that tick, retaining the verified prefix rather than guessing an input.

The current runnable setup is two human leaders in ascending distinct ports,
FD, Battlefield or Yoshi's Story, registered characters, valid costumes,
normal speed/damage/scale/handicap, singles stock mode, items off, normal
countdown and music. Stock count is preserved (1..99); time/countdown rules
are preserved and time-up stops at an explicit unported boundary. CPU/demo
control, Slippi Online setup/seed resets, PAL, and Japanese countdown assets
are reported unsupported. UCF/Dween stops at the first nonzero main-stick
sample; its fixes are not implemented. Arbitrary Gecko modifications, forced
music, alternative entry modes and pause/resume are not verified contracts.

`slp::cold::ColdScenario::from_replay` accepts the replay plus the explicit
pre-music seed and unlock flag. `to_toml()` produces the cold-scenario shape:
`name`, `frames`, `seed`, `stage`, `all_characters_unlocked`, `[[fighters]]`,
`[replay_rules]`, and `[[replay_inputs]]`, with no savestate. Fighters retain
slot, character, costume, stock count, controller type and spawn marker.
`replay_rules` retains stock/timer mode, time limit, teams, item settings,
damage ratio and the separately named `game_start_seed`. `Scenario` ingests
this shape; `run` replays its pads without opening an expected trace. The
older `slp-dump --scenario` metadata/VI-style export remains available; it is
not the cold bridge. The replay runner performs the supported-setup checks
before constructing a `Scenario`.

Workspace dependency direction stays `melee-sim -> slp -> melee-diff`.
Slippi conversion owns no fighter/stage logic. The small DynamicModelDesc
reader lives in `hsd-archive`; countdown composition stays in `melee-sim`.

## Recording source and field mapping

Audited the Slippi recording hooks at revision
`fcf47f10dc244152c2ebaa3a9dec142ea42243b7`:
[Pre Frame](https://github.com/project-slippi/slippi-ssbm-asm/blob/fcf47f10dc244152c2ebaa3a9dec142ea42243b7/Recording/SendGamePreFrame.asm),
[Post Frame](https://github.com/project-slippi/slippi-ssbm-asm/blob/fcf47f10dc244152c2ebaa3a9dec142ea42243b7/Recording/SendGamePostFrame.asm),
[Frame Start](https://github.com/project-slippi/slippi-ssbm-asm/blob/fcf47f10dc244152c2ebaa3a9dec142ea42243b7/Recording/SendFrameStart.s),
[Game Start](https://github.com/project-slippi/slippi-ssbm-asm/blob/fcf47f10dc244152c2ebaa3a9dec142ea42243b7/Recording/SendGameInfo.asm).
The parser uses declared payload lengths for optional fields, following the
[Slippi specification](https://github.com/project-slippi/slippi-wiki/blob/master/SPEC.md).
No recording assembly was copied into this repository.

| Slippi field | Retail source / simulator mapping | Availability and limitation |
|---|---|---|
| Pre joystick X/Y | Fighter +620/+624, processed main stick | Already fighter-deadzoned; not identical to HSD normalized input for small raw deflections |
| Pre C-stick X/Y | Fighter +638/+63C | Same loss of original HSD values; do not divide these floats by 80 again |
| Pre processed buttons | Fighter +65C held word | Includes HSD direction bits and fighter Z/shoulder macros; not physical button bits alone |
| Pre physical buttons | Low 16 bits of **HSD_PadMasterStatus** (+02) | MasterStatus can differ from GameStatus at the consumed tick |
| Pre processed trigger | Fighter +650 | Deadzone, max(L,R), digital shoulder and Z processing have lost the separate original L/R values |
| Pre physical L/R | MasterStatus normalized analog L/R (+30/+34) | Some bundled recordings contain unusable values such as `0x65000C80`; never clamp or replace that garbage silently |
| Pre raw main X / Y | SDK input ring bytes | X from 1.2, Y from 3.15; a complete pair can use the existing audited HSD normalization routine |
| Pre raw C-stick X/Y | SDK input ring bytes | From 3.17; not HSD's post-clamp signed bytes |
| HSD analogA/B and normalized analogA/B | Not recorded | Not currently fields of `PadSample`; not synthesized |
| HSD last/trigger/repeat/release words, repeat_count, cross_dir, err | Not recorded as HSD state | Fighter buffers are produced by the port; full HSD state cannot be reconstructed |
| Post internal character | `pN.kind`, signed integer | Direct field |
| Post player index | `pN.player_id`, unsigned integer | Original physical port, not N |
| Post action-state ID | `pN.motion_id`, signed integer | Not animation/submotion ID |
| Post X/Y, facing, percent | `pN.cur_pos.x/y`, `facing_dir`, `percent` | Copy f32 bits |
| Post animation frame | `pN.cur_anim_frame` | 0.2+ |
| Post airborne | `pN.ground_or_air` | 2.0+ |
| Post self air X/Y speed | Fighter +80/+84 -> `pN.self_vel.x/y` | 3.5+; **do not substitute ground X speed** when grounded |
| Post attack X/Y speed | Fighter +8C/+90 -> `pN.kb_vel.x/y` | 3.5+ |
| Post self ground X speed | Fighter +EC | Parsed, outside the existing 49-key snapshot |
| Post jumps remaining | No emitted `jumps_used` | Needs character max jumps; omitted |
| Post misc action variable/hitstun | No emitted value | Union field is not uniformly hitstun, and not an existing gate key |
| Missing Z coordinates/velocities and other snapshot fields | Omitted | Expected is a subset; no zeros fabricated |
| Adjacent next Frame Start seed | `rng.seed` | Relation below; omit final frame and missing/nonadjacent Frame Start |

`replay_pad` combines physical low button bits with processed HSD direction
bits, stripping the fighter-generated shield bit. The existing fighter input
proc then recomputes the Z and shoulder macros. Complete raw stick pairs use
`normalize_stick`; otherwise the already processed floats are used. This is
a reconstruction of the consumed subset under ordinary input processing,
not a claim to recover all HSD pad memory.

The independent pad test reads Fighter input words from the existing ledger
captures and compares the resulting seven `PadSample` words bit-for-bit with
`inputs.pN` in the corresponding expected traces. Walk, shield and jump each
give **300 ticks x 2 ports**, all **1,800 samples equal**. For example,
`walk_fd_fox` tick 31 contains HSD button `0x00080000` and normalized X
`0x3F0CCCCD`; physical button bits alone would wrongly produce button zero.

**Evidence limit:** these captures contain GameStatus, not MasterStatus.
The test therefore supplies physical buttons/L/R from recorded GameStatus.
It establishes the adapter's mapping on these scenes, not that Slippi's
MasterStatus always equals GameStatus. Full pad equality is impossible in
general: a deadzoned zero cannot distinguish raw X=0 from X=1 without the
raw bytes; individual triggers and analogA/B also cannot be recovered from
the processed trigger. Dedicated tests demonstrate that loss and reject
invalid physical triggers. A simultaneous Slippi/GameStatus recording is
still needed to verify the raw-source phase on a real replay.

## Seed and frame alignment

There are three distinct seed phases:

1. Game Start is injected at `0x8016E74C`, early in `fn_8016E730`, before
   Ground/fighter creation. B3 does not implement all initialization before
   Ground creation, so this seed is metadata, not the cold constructor seed.
2. Frame Start is a scheduler s_link-0 process on p_link 7, before fighter
   animation on p_link 8. This is the first full frame's initial RNG state.
3. Pre Frame is injected at `0x8006B0E0`, in the fighter input proc on
   s_link 3, **after animation may draw randomness**. It is not generally the
   scheduler-start seed. The task's statement that Pre Frame reads at frame
   start is therefore too strong; it cannot be used as an unconditional
   fallback for old replays' end-of-frame RNG comparison.

For the audited unpaused retail scenes, with no intervening RNG consumer:

```text
cold observation 0 = setup/tick-counter reset, after music (no scheduler pass)
Slippi -123        = cold observation 1
replay tick k      = Slippi k - 123 = cold observation k + 1
expected end seed(k) = FrameStart(k + 1)
```

`PadScript` prepends a neutral row for observation zero. The runner consumes
that setup observation once and renumbers each subsequent complete snapshot
to replay tick zero onward. `slp::to_trace` only uses a genuinely adjacent
Frame Start seed; the last frame, a gap, or a pre-2.2 replay omits `rng.seed`.
The legacy `Frame::start_seed` helper remains for metadata; comparison uses
the explicitly named `scheduler_start_seed` instead.

On `start_fd_fox`, ledger observation 0 has seed `CC51A0A5`; observation 1
has `C37A8245` after 32 draws. Both ledger observations have VI frame 2.
Every one of the 599 ledger intervals advances the previous observation's
seed to the next through exactly its recorded draws, and all 600 end seeds
equal the independently decoded tick trace. Caller attribution accounts for
the observed draws as scheduled fighter animation, stage and particle work;
no render/pre-scheduler caller occurs in this capture.

A concrete counterexample to treating Pre Frame as Frame Start is cold tick
225: its start seed is `7521A167`, Wait animation draws to `73D6168E`
(`ftCo_8008A7A8`, LR `8008A8C0`) before input, and its final seed is
`17C631A3` after 44 draws. Slippi's Frame Start would record the first seed,
Pre Frame the second, and the tick trace the third. The complete hook order
plus recorded ledger establishes this relation on this scene; it is not a
claim that arbitrary mods, rendering changes, online seed resets or pauses
preserve that relation.

For normal FD/Battlefield music, derive B3's pre-music seed from first Frame
Start by undoing zero draws (locked characters) or one draw (all unlocked):
`previous = (seed - 2531011) * 0xB9B33155` modulo 2^32. B3 then executes its
audited initialization interval forward. The runner verifies that observation
zero reaches first Frame Start. No later recorded seed enters simulation.
The BF projection test independently exercises the one-music-draw case.

## Cold input release and port order

B3 left `status.input_frozen` true forever. Neutral cold gates did not expose
this. Normal Versus creates countdown model 3 / animation 0 from
`IfAll.usd:ScInfCnt_scene_models`. `if_802F73C4` runs at s_link 0, p_link 14,
animates its joints and checks `lb_8000B09C`. Its completion callback
`fn_8016B7F8 -> ftLib_800868A4` releases fighter input before s_link 3.
The port now drives that existing HSD animation; it does not hard-code a
tick-85 release. The test compares all recorded freeze bits and confirms
release at cold tick 85 (replay tick 84), for input in that same pass.

Retail `fn_8016E2BC` creates players in ascending active slot order. For
leaders on Slippi ports 0 and 2, snapshot indices are p0 and p1, while
`player_id` remains 0 and 2. Inputs stay keyed by physical port, and fighter
dispatch reads the fighter's player ID, not its list index. Game Start's
spawn marker is preserved separately (`-1` means the stage marker for that
slot). The real `v3.13` fixture pins 0/2 -> p0/p1 mapping; a cold oracle
projection verifies 599 frames with a gap and verifies that an X press on
port 2 actually changes p1. Followers remain excluded from this leader-only
trace adapter; Ice Climbers cannot use this as a full fighter-list mapping.

## Fixture results

The files and their MIT attribution are unchanged; see
`crates/slp/tests/data/NOTICE` (peppi origin). Ports below are physical,
one-based ports. Each CLI invocation reports the same zero-frame floor
checked by `fixture_matched_frame_counts_never_decrease`.

| Fixture | Frames | Stage | Characters / ports | Support and first boundary | Matched |
|---|---:|---|---|---|---:|
| `ics.slp` | 344 | FD | P1 Ice Climbers, P2 Jigglypuff CPU | Stage supported; both characters, follower handling and CPU unsupported | 0 |
| `joystick_udlr.slp` | 342 | FD | P1 Marth, P2 Ganondorf CPU | Marth/stage supported; Ganondorf/CPU unsupported | 0 |
| `netplay.slp` | 128 | Fountain of Dreams | P1 Samus, P2 Marth | Stage/Samus/Online setup unsupported | 0 |
| `v0.1.slp` | 8,236 | Dream Land | P1 Fox, P2 Ganondorf | Stage/Ganondorf unsupported; no Frame Start seed | 0 |
| `v3.12.slp` | 124 | Pokemon Stadium | P1 Marth, P2 Marth | Characters supported; stage/Online setup unsupported | 0 |
| `v3.13.slp` | 148 | FD | P1 Fox, P3 Pichu | Stage/Fox supported; Pichu unsupported, items enabled (behavior 2) | 0 |
| `v3.16.slp` | 308 | Yoshi's Story | P1 Fox, P2 Falco | Stage and characters supported; Online per-frame RNG resets unsupported | 0 |
| `v3.18.slp` | 941 | Fountain of Dreams | P1 Marth, P2 Captain Falcon CPU 7 | Characters supported; stage/CPU unsupported | 0 |

No first unported **action state** has been reached in this real corpus;
the stop is before frame zero. The 150-frame `AttackAirN (65)` stop test is
explicitly a synthetic protocol probe, not a measured fixture result.

## Recording needed and remaining work

Record **offline NTSC 1.02 English, Final Destination, Fox vs Fox, human
ports 1/2, costumes 0/1, one stock, items NONE, normal speed/damage, controller
fixes disabled**, using Slippi with Frame Start events (2.2+). Record the
unlock setting and use normal music without holding L/R on entry. Keep both
pads neutral for the first **600 recorded frames (-123 through 476)**,
then have P1 jump and press A in the air: **AttackAirN (65)** is the desired
first unported action. This is a requested recording recipe, not a promise
that unseen Slippi modifications or pad phases will match the retail port.
For strongest evidence, capture GameStatus and an RNG ledger alongside that
replay, starting before `fn_8016E730`, as specified in `COLD_START.md`.
Do not use an online match for this first comparison.

Remaining: that actual supported `.slp` fixture and a positive corpus floor;
MasterStatus/GameStatus co-recording; earlier Game Start initialization;
forced/modified music and Gecko compatibility; online resets/rollback scene
setup; other cold stages/characters; CPU/followers; controller fixes; timers,
stock loss and match end; batch aggregation. Partial/finalized rollback-stream
reconstruction is not expanded by this lane.

## Verification

Focused tests cover real gap mapping, missing optional fields, seed adjacency,
signed-zero preservation, TOML rule/seed separation, the eight corpus floors,
two 599-frame independent cold projections, ported-state one-bit failure,
unported action naming, late bad input, gapped input routing, 1,800 pad samples,
599 ledger intervals, and countdown freeze timing. Local DAT/oracle tests
skip when their machine-local resources are absent.

Verified commands/results:

- `cargo gate`: **772 passed, 0 failed, 1 pre-existing ignored** (baseline: 757 passed).
- `cargo clippy --workspace --all-targets -- -D warnings`: passed after moving
  the countdown test module below its implementation; no lint suppression.
- `cargo fmt --all -- --check` and `git diff --check`: passed.
- `cargo run -q -p melee-sim -- replay crates/slp/tests/data/<fixture>`:
  all eight invocations completed with the setup reports above.
- Harness: **186 passed**, using the main checkout's existing environment:
  `/Users/nikhilunni/Projects/melee/harness/.venv/bin/python -m pytest -q`
  from this lane's `harness` directory.

No Dolphin run, commits, existing scenario edits, fixture edits, or
trace/ROM/decomp writes were performed. The pre-existing lane symlink/typechange
entries for traces/ROMs/decomp were preserved.

Changed files (new files included):

```text
Cargo.lock
TRACKER.md
docs/COLD_START.md
docs/SLIPPI.md
crates/hsd-archive/src/desc.rs
crates/hsd-archive/src/desc/model.rs
crates/slp/Cargo.toml
crates/slp/src/cold.rs
crates/slp/src/event.rs
crates/slp/src/lib.rs
crates/slp/src/trace.rs
crates/slp/tests/replay_mapping.rs
crates/melee-sim/Cargo.toml
crates/melee-sim/src/countdown.rs
crates/melee-sim/src/frame.rs
crates/melee-sim/src/initial_state/cold.rs
crates/melee-sim/src/initial_state/mod.rs
crates/melee-sim/src/inputs.rs
crates/melee-sim/src/lib.rs
crates/melee-sim/src/main.rs
crates/melee-sim/src/replay.rs
crates/melee-sim/src/scenario.rs
crates/melee-sim/src/trace.rs
crates/melee-sim/tests/slippi_oracle.rs
crates/melee-sim/tests/slippi_replay.rs
```

## Lane B5: first real Yoshi's Story attempt (2026-09-09)

Yoshi's Story cold setup is now supported, with the match-start and cold
600 x 49 gates described in YOSHIS_STORY.md. The replay stage whitelist
includes stage-select ID 8. Its normal music rule is 0, so deriving the cold
boundary from Frame Start does **not** undo a draw when characters are
unlocked. A separate Story oracle projection verifies 599 complete frames.

Actual invocation and complete output:

```text
$ cargo run -q -p melee-sim -- replay crates/slp/tests/data/v3.16.slp
YoshisStory; port 1 -> p0 Fox costume 1 Human stocks 4, port 2 -> p1 Falco costume 0 Human stocks 4
0 / 308 replay frames matched
unsupported setup: Slippi Online initialization/seed resets
```

**Matched count: 0. First divergence: none measured**, because the explicit
online setup check stops before simulation. This is not a green replay or a
claim of a first-frame mismatch. The fixture-count floor remains zero; its
pinned reason changes from unsupported cold stage to Online. No fixture
bytes or expected trace values changed.

The only structural setup rejection remaining for this fixture is Online.
It is NTSC, English, singles stock mode, four stocks, 480 seconds, items off,
normal speed/damage/handicap/scale, with supported Fox/Falco costumes and
ports. Those stock/time settings are already carried by the B4 bridge.
Changing those rules or the major-scene flag alone would be insufficient.

Audited Slippi source at the same pinned revision as B4:
[Online/Core/InitOnlinePlay.asm](https://github.com/project-slippi/slippi-ssbm-asm/blob/fcf47f10dc244152c2ebaa3a9dec142ea42243b7/Online/Core/InitOnlinePlay.asm).
The injection at **0x8016E748** installs the negotiated RNG offset before
ordinary match setup. It then creates `FN_SyncRNG`, a priority-zero GObj proc
that runs before player animations. On **every frame** this callback replaces
the shared seed with `scene_frame_counter.rotate_left(16) + negotiated_offset`
(modulo 2^32). This is a netplay determinism mechanism, not a stock/timer/music
rule. The recording hook then samples that stream at Frame Start.

A read-only diagnostic of the unmodified fixture established:

- Game Start seed and first Frame Start seed: **0x00003AAE**.
- First scene counters: 0,1,2,3,4; corresponding Frame Start seeds:
  **00003AAE, 00013AAE, 00023AAE, 00033AAE, 00043AAE**.
- **308 / 308** recorded Frame Start seeds satisfy the reset formula with
  offset 0x3AAE. A regression test pins this evidence and verifies that Online
  is the sole setup rejection.
- First Post Frames retain Entry 322 and animation frame -1, with P1 at
  (-42,26.6) and P2 at (42,28), consistent with the stage's entry markers.

Per the lane boundary, work stops at netplay reconstruction. The runner does
not clear the online flag, inject successive recorded seeds, or relabel a
retail/offline simulation as an online replay. Supporting this path requires
an explicit online initialization/scheduler contract, its seed reset callback,
and then verification of finalized rollback frames and the additional Gecko
behavior. No complete rollback history was reconstructed in this lane.

B5 final checks: `cargo gate` **821 passed, 0 failed, 1 pre-existing ignored**;
clippy `--workspace --all-targets -- -D warnings`, fmt check and diff check
passed; harness pytest **186 passed**. The Story oracle projection is included
in the full gate. The complete B5 file list is in YOSHIS_STORY.md. No commit.

## Corpus runner (2026-09-28)

```sh
melee-sim replay-batch ~/melee-data/replays/public-v3.7 --ignore-controller-fixes --jsonl out.jsonl
```

replays every `.slp` under the paths in parallel and prints the first stops
grouped by cause, largest group first (the motion change when one differs,
with both leaders' recorded actions). `--all-characters-unlocked` (default
true) covers replays without Frame Start; `--ignore-controller-fixes` runs
UCF recordings without the fix and marks each report.

**Local corpus.** `~/melee-data/replays/public-v3.7/<CHARACTER>/`: 108
tournament games sampled from the public Slippi dataset
(huggingface.co/datasets/erickfm/slippi-public-dataset-v3.7). All are Slippi
2.0.1 console (Nintendont) recordings from 2019-2020, singles, items off,
four stocks, eight minutes, **UCF on for both ports**, no Frame Start.

**Setup contract changes.**
- *Seed.* Game Start's seed (0x8016E74C) precedes fn_8016E730's Ground and
  Player creation: the cold boundary is that seed advanced by the setup
  draws (`boundary_seed_from_creation`). Verified on every fixture with
  Frame Start; the first Frame Start then equals the boundary (no music
  draw) or its successor (all-unlocked save on a rule-6 stage).
- *Entry delay* is 5 x (slot + 1): ports 2 and 4 enter at ticks 10 and 20.
- *Spawns.* Slippi builds replace retail's slot markers. NeutralSpawn.asm
  (0x8016E510, in Slippi's asm from 2020-01) places the Nth present player
  at its table row, facing by sign of x. Late-2019 console builds used the
  same table except Dream Land ((-46.6, 37.0), (47.389, 37.0)); April 2019
  builds used retail markers. `melee_lib::slippi::SpawnRule` models the
  three; the runner picks the one the first frame shows exactly.
- *Fighter mapping.* Each Slippi port maps to its fighter in play by slot
  and kind; Nana compares as `pN.follower`.

**First corpus result** (UCF ignored): every replay passes Entry and the
countdown except two (an Ice Climbers spawn, one odd entry timing), and
stops 90-300 ticks in, almost all at a Dash, Turn or Pass (platform drop)
motion change: UCF's dashback and shield-drop fixes. Four stop at
Jigglypuff's costume hats (unported). The corpus is blocked on UCF.

**Slippi code sets that affect gameplay** (slippi-ssbm-asm
`Output/InjectionLists`): console core (NanaDeterminism, Stadium
transformation preload, PSCameraIndependentMonitor from 2021-08), tournament
mods (NeutralSpawn, FreezeGlitchFix, Disable FoD During Doubles), Frozen
Stadium, UCF 0.74 / 0.8 / 0.84, and netplay (per-frame RNG sync,
FreezeDeadUpFallPhysics, PreventWobbling, FreezeFDSlippi, Frozen PS). Each
must be ported before a replay recorded with it can match; the Stadium
preload and Frozen Stadium codes are (below).

## Controller fixes (UCF), 2026-09-28

Tournament replays ran the Universal Controller Fix Gecko codes (Game Start
`dashback_fix`/`shield_drop_fix` = 1). The port carries them as a per-port
`ControllerFix` (`melee_ft::input::controller_fix`, re-exported by
`melee-lib`): `Off`, `Ucf074`, `Ucf080`, `Ucf084`, and `Dween`, which match
setup refuses with the reason. Slippi does not record the UCF version; the
caller chooses it (0.74 for 2019-2020 console builds, 0.8 from 2021, 0.84
from 2024).

| Version | Slippi source (logic ported, not copied; GPL-3) | Hook |
|---|---|---|
| 0.74, 0.8 | `External/UCF 0.74/UCF DB.asm`, `External/UCF 0.8/Logic/UCF DB.asm` | 0x800C9A44, ftCo_Turn_IASA's first flip: slow turn to smash turn; Popo also rewrites Nana's newest follow sample |
| 0.74, 0.8 | `.../UCF SD.asm` | 0x800998A4, ftCo_80099894: refuse a rim-angled spot dodge so the shield drops |
| 0.8 | `External/UCF 0.8/Logic/UCF Tumble.asm` | 0x800908F4, ftCo_DamageFall_IASA's wiggle age |
| 0.84 | `External/UCF 0.84/UCF/UCF Pad Buffer + 1.0 Cardinals.asm` | 0x8006B460, the human input proc: a per-port four-sample raw ring, cardinal sticks snapped to exactly 1.0, a lower-rim tick count |
| 0.84 | `UCF Dashback.asm`, `UCF Shield Drop.asm`, `UCF Tumble.asm` | as above, reading the ring (dashback: stick toward the new facing, any kind's second fighter; shield drop: platforms only, roll window) |
| 0.84 | `UCF Shield Drop Extended.asm` | 0x8009A0B8, ftCo_8009A080: a rim count above one passes the drop's stick test |
| 0.84 | `UCF SDI.asm`, `UCF Shield SDI.asm` | 0x8008E54C / 0x80093294: a raw jump over 62 from inside the SDI radius SDIs |
| 0.84 | `UCF DBOOC SquatRv Fix.asm` | 0x800D65EC: a fresh x tap on the rim lowers the squat release line to 0.59 |

The 0.74 and 0.8 dashback and shield-drop programs are identical; 0.84 ships
as machine code only and is ported from its disassembly. The 0.74/0.8 codes
read the hardware pad queue (raw `PADStatus.stickX` now and two ticks ago,
before HSD's clamp), which the engine models as one poll per tick or takes
verbatim from a recording (`Simulation::set_recorded_pad_queue_x`). 0.84's
ring is fed from the consumed entry's four stick bytes
(`Simulation::set_raw_sticks`; derived from the pads when absent) and lives in
the match state, zero at start as when the code is installed. A console's ring
carries the previous game's last samples into a new match; four input ticks
refresh it, long before the countdown releases input.

API for a replay runner:

- scenario TOML: `controller_fix = "ucf-0.8"` on each `[[fighters]]`
  (`FighterScenario::controller_fix`, names in `ControllerFix::ALL`);
  `PadScript::from_replay_inputs` feeds Pre Frame raw joystick and C-stick
  bytes (derived from the processed sticks when absent);
- `melee-lib`: `PlayerSetup::controller_fix` (diagnostics) or
  `PlayerConfig::with_controller_fix` (public `Match`);
- a borrowed boundary: `Simulation::set_controller_fixes([ControllerFix; 4])`.

Recording with the codes: a scenario's `gecko = ["ucf-0.8"]` makes
`record.py` write the codes into the run's private Dolphin user folder
(`harness/gecko.py`, text read from `MELEE_GECKO_DIR`, never committed) and
enable cheats; the tracer checks every C2 injection holds its branch, keeps the
pad queue at one poll per tick and records `pad_queue_x` and
`pad_queue_sticks` (what the codes read) beside each tick. Witnesses
(`UCF_WITNESSES` in `m5_gate.rs`) gate exact with and without the codes:
`ucf_dashback_fd_fox_*`, `ucf_dashback_fd_iceclimbers_*`,
`ucf_shielddrop_bf_fox_*`, `ucf_tumble_fd_fox_*` and the `ucf084_*` set
(cardinal run, dashback, shield drop, rim-count drop, tumble, SDI, shield SDI,
squat release).

`melee-sim replay` and `replay-batch` date the UCF version from the recording
(`ucf_version`); `--controller-fix <off|ucf-0.74|ucf-0.8|ucf-0.84>` names it
instead for every port recorded with UCF (also for recordings older than
0.74, which are otherwise refused).

Not ported: the UCF 0.73 beta (`Binary/UCF/Ucf0.73Beta.bin`, a different
dashback program) and Dween.

## Pokémon Stadium codes, 2026-10-03

Two Slippi codes change the transformation controller (grStadium_801D4548);
`melee_lib::slippi::SlippiCodes` carries both and
`melee_gr::stadium::transform` runs them (logic ported, not copied; GPL-3).

| Code | Slippi source | Hooks | Selected by |
|---|---|---|---|
| Preload (`stadium_preload`) | `Common/Preload Stadium Transformations` (console core and netplay) | 0x801D14C8 (init clears isLoaded, map 2 +0xF0), 0x801D45EC (a waiting tick with isLoaded clear draws the form into +0xEC and starts its read), 0x801D460C / 0x801D4610 (the wait's end takes that form instead of drawing), 0x801D4724 (skips the read and falls into phase 1's poll in the same tick), 0x801D4F14 (the settling base arena clears isLoaded) | replay version 1.3.0 or later, the rule Slippi's playback uses (Ishiiruka EXI_DeviceSlippi.cpp, "Write PS pre-load byte") |
| Frozen (`stadium_frozen`) | `External/Frozen PS/Core/FreezePokemon.asm` | 0x801D45FC: `bge` becomes `b`, so the wait never ends (the timer keeps counting down) | Game Start's `frozen_ps` |

With preload the form's `Randi(4)` moves from the end of the base duration
to the controller's first waiting tick (and the tick after each return to
the base), which shifts every later draw by one; the form is announced on
the tick the duration ends, with no disc latency, so no external read event
is consumed. The skipped retail code is what sets map 2's xC4_b1, so the
poll at 0x801D4760 succeeds whatever the disc did. A frozen match with
preload still makes the first draw. The code is unchanged in the Slippi
asm from 2019-02 (then applied by a static patcher under a toggle) to now.
PSCameraIndependentMonitor (0x801D24FC) dates from 2021-08 and is not in
these recordings.

Witnesses (`pokemon_stadium_slippi_codes_match_retail`), recorded with the
Gecko codes `ps-preload` (the six preload injections from
`Output/Console/g_core.bin`) and `ps-frozen` (`041D45FC 480009DC`) in
`MELEE_GECKO_DIR`: `slippi_ps_preload_fox_marth4` (7500 ticks: retail draws
at ticks 85 and 6564, a whole transformation cycle) and
`slippi_ps_frozen_fox_marth4` (4600 ticks, both codes: the draw at tick 85
and no transformation). A scenario names the codes with
`stadium_preload = true` / `stadium_frozen = true` beside its `gecko` list.

Corpus: the six Pokémon Stadium games stopped at ticks 3820-4304; five now
match to the last frame (three transforming, two frozen). `MARTH/13_06_06
Marth + Fox (PS).slp` (frozen) still stops at tick 4257: Marth's idle
re-roll at tick 4167 (ftCo_8008A7A8, one `Randi(100)`) picks the other Wait
animation, so the random stream differs by then from a draw the recording
does not show. Not diagnosed; the jumbotron's close-up test
(grStadium_801D32D0) reads the rendered camera and draws on a change, which
is why Slippi later added PSCameraIndependentMonitor.
