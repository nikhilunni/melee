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
control, PAL, and Japanese countdown assets are reported unsupported (Slippi
Online games were too until 2026-10-12: "Slippi Online" below). UCF/Dween stops at the first nonzero main-stick
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

**How a replay is simulated (2026-10-11).** The dry passes below (stage
code, screen, UCF version) and the comparison all read from one set of
runs per replay (`replay_game`): each candidate setup is simulated once,
only as far as a reader asks, and every later reader starts from its first
tick without simulating it again. About 3.1 ticks are simulated per
recorded frame; `docs/PERF.md`, "Replay runner", has the measurements.
- `MELEE_REPLAY_COST=1` prints each replay's time and simulated ticks per
  part of the runner to stderr.
- A new code a replay does not record (a stage code, say) is a field in
  `replay_game::Codes` and its own dry pass.

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
  (0x8016E510, in Slippi's asm from 2019-10-02, commit b5f06ae) places the
  Nth present player at its table row, facing by sign of x. Late-2019 console builds used the
  same table except Dream Land ((-46.6, 37.0), (47.389, 37.0)); April 2019
  builds used retail markers. `melee_lib::slippi::SpawnRule` models these;
  the runner picks the one the first frame shows exactly. An Ice Climbers
  pair stands on either side of its row (Popo five units ahead of the
  Player position, Nana five behind: fighter.c:241, 0x80067CE8), so the
  runner asks only that the row lie between them.
- *April 2019 consoles* ran Achilles' 20XX Neutral Spawns instead
  (`Binary/FasterMeleeSettings/20xxNeutralSpawns.bin`, in
  `console_tournament.json` until b5f06ae; first injection 0x80263058): the
  two games below show the Nth player on a neutral stage *marker* with Game
  Start's `spawn_point` holding that order (read from their first frames;
  the binary's stage table is not decoded). It is not ported as a rule. The runner still
  finds the placement from frame zero: Battlefield's markers equal the later
  table's coordinates bit for bit (`FALCO/00_46_28`, ports 1 and 3: the
  table), and Stadium's neutral markers are markers 0 and 1, so retail
  markers with the recorded `spawn_point` fit (`FALCO/02_33_48`:
  (-39.999996, 31.999992), which the table's (-40, 32) does not). A stage
  whose markers match neither would stop at frame 0 and need the code's
  marker table decoded from the binary.
- *Entry delay by spawn order.* `External/NeutralSpawn.asm` from 2019-10-21
  ([1a01aec](https://github.com/project-slippi/slippi-ssbm-asm/commit/1a01aec47d))
  until 2020-01-14
  ([4dc7447](https://github.com/project-slippi/slippi-ssbm-asm/commit/4dc7447fdc),
  "remove feature causing neutral spawn desync") also ran
  `SetSpawn_AdjustEntryFrames`: Player_SetUnk4C (0x80035FDC) with 5 x spawn
  order, so the first player leaves Entry on the first tick and the second
  five ticks later. Consoles still ran that build in March 2020 (HNC 11,
  2020-03-04). `NeutralTable::V2019EntryByOrder` (`spawn =
  "neutral-2019-entry"`); the runner chooses it when the first leader is
  already in EntryStart on the first frame, which no other rule produces.
  Retail runs the restored entry code in `slp_fod_level_fox_jigglypuff_r1_t13500`
  ("Consoles with level Fountain of Dreams platforms" below): the six words
  put back into the 2020 block by `harness/gecko_fod_level.py`.
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
preload and Frozen Stadium codes are (below), and so are Frozen Stages and
the Widescreen code, which consoles ran beside Slippi's own sets.

## Controller fixes (UCF), 2026-09-28

Tournament replays ran the Universal Controller Fix Gecko codes (Game Start
`dashback_fix`/`shield_drop_fix` = 1). The port carries them as a per-port
`ControllerFix` (`melee_ft::input::controller_fix`, re-exported by
`melee-lib`): `Off`, `Ucf073`, `Ucf074`, `Ucf080`, `Ucf084`, and `Dween`
(below). Slippi does not record the UCF
version; the caller chooses it (the 0.73 beta until October 2019, 0.74 for
2019-2020 console builds, 0.8 from 2021, 0.84 from 2024).

| Version | Slippi source (logic ported, not copied; GPL-3) | Hook |
|---|---|---|
| 0.73 beta | `Binary/UCF/Ucf0.73Beta.bin` (machine code; removed from the tree in 87ae36e) | 0x800C9A44: as 0.74, but the stick must point toward the turn; 0x800998A4: as 0.74 |
| 0.74, 0.8 | `External/UCF 0.74/UCF DB.asm`, `External/UCF 0.8/Logic/UCF DB.asm` | 0x800C9A44, ftCo_Turn_IASA's first flip: slow turn to smash turn; Popo also rewrites Nana's newest follow sample |
| 0.74, 0.8 | `.../UCF SD.asm` | 0x800998A4, ftCo_80099894: refuse a rim-angled spot dodge so the shield drops |
| 0.8 | `External/UCF 0.8/Logic/UCF Tumble.asm` | 0x800908F4, ftCo_DamageFall_IASA's wiggle age |
| 0.84 | `External/UCF 0.84/UCF/UCF Pad Buffer + 1.0 Cardinals.asm` | 0x8006B460, the human input proc: a per-port four-sample raw ring, cardinal sticks snapped to exactly 1.0, a lower-rim tick count |
| 0.84 | `UCF Dashback.asm`, `UCF Shield Drop.asm`, `UCF Tumble.asm` | as above, reading the ring (dashback: stick toward the new facing, any kind's second fighter; shield drop: platforms only, roll window) |
| 0.84 | `UCF Shield Drop Extended.asm` | 0x8009A0B8, ftCo_8009A080: a rim count above one passes the drop's stick test |
| 0.84 | `UCF SDI.asm`, `UCF Shield SDI.asm` | 0x8008E54C / 0x80093294: a raw jump over 62 from inside the SDI radius SDIs |
| 0.84 | `UCF DBOOC SquatRv Fix.asm` | 0x800D65EC: a fresh x tap on the rim lowers the squat release line to 0.59 |

The 0.74 and 0.8 dashback and shield-drop programs are identical (checked
on the machine code, below); 0.73 and 0.84 ship as machine code only and are
ported from their disassembly. The 0.73/0.74/0.8 codes
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

`melee-sim replay` and `replay-batch` bound the UCF version by the replay's
Slippi version, date it from the recording inside that bound
(`ucf_candidates`) and, where two remain, read it from the frames (below); `--controller-fix <off|ucf-0.73|ucf-0.74|ucf-0.8|ucf-0.84>` names
it instead for every port recorded with UCF. The report's `controller fix`
line says which version ran and why.

Not ported: the 0.74 shield drop without the truncation that Slippi's
Dolphin lists and `g_toggles.bin` carried for one week (acc6f71,
2019-09-24, to 4062e22, 2019-10-01).

### Dween's fix and the per-port toggle (2026-10-03)

Console builds from 2019-02 to bb86519 (2021-03-31) could run the toggle
set `g_toggles.bin` ("UCF + Arduino Toggle UI") instead of `g_ucf.bin`:
each port chooses off, UCF or "Arduino" on the character select screen, a
byte per port at `ControllerFixOptions` (rtoc - 0xDD8, 0x804DEC08). Game
Start's `dashback_fix` and `shield_drop_fix` are that one byte written
twice (Recording/SendGameInfo.asm), so they are always equal: 0 off, 1 UCF,
2 Dween. The set's UCF hooks (0x800C9A44, 0x800998A4, "Check for Toggle")
run for a byte of 1 and Dween's code for 2.

Dween's code (`External/UCF + Arduino Toggle UI/Arduino/Arduino - Check for
Toggle.asm`, the same 448 bytes in every build of the set) is one injection
at 0x8006B028, the store of the pad's buttons in the human arm of the input
proc. It rewrites the fighter's dead-zoned main stick (fp+0x620/+0x624)
before anything reads it, and keeps the pad's normalized stick x per port
for the next tick. `melee_ft::input::controller_fix::dween` ports it
(`ControllerFix::Dween`, scenario name `dween`; logic ported, not copied;
GPL-3):

| Branch | Condition | Effect |
|---|---|---|
| Dashback | no shield (analog trigger zero, no Z/R/L), no B/X/Y, A not pressed this tick; stick y zero, x non-zero and under PlCo+0x3C (0.8); last tick's pad x inside the dead zone (PlCo+0x0) and at least PlCo+0x8 (0.25) away | x reads zero this tick: a tap that passes through the tilt range is still a fresh full tap on the next |
| Shield drop | shield held on both ticks (analog product positive, or Z/R/L with last tick's held word); stick on the same side as last tick with x670 at least 3; y at or below PlCo+0x314 (-0.7) and above -0.8; (abs x + 0.0125)^2 + (0.0125 - y)^2 over 1 | the stick becomes (+/-0.725, -0.6875), short of the spot dodge line, so the platform drop runs |

Witnesses (`UCF_WITNESSES`), recorded with the Gecko code `dween` (the
0x8006B028 block of `Output/Console/g_toggles.bin` at 009b155, then
`004DEC08 00030002` to set the four toggle bytes to 2), exact with 0
differing particle-site ticks:

| Scenario | Retail |
|---|---|
| `dween_dashback_fd_fox` | stick 0, -40, -80: Wait at tick 120, Turn at 121, Dash at 122 (without the code: slow Turn 120, Dash 125) |
| `dween_dashback_slow_fd_fox` | -20, -36, -80: 0.2 from the last sample, left alone (Turn 120, Dash 125) |
| `dween_dashback_a_fd_fox` | A pressed with the first tilt: forward tilt at 120 (a zeroed stick would jab) |
| `dween_shielddrop_bf_fox` | shielding, tilt right, then (56, -56): Pass at 136 (without the code: spot dodge) |
| `dween_shielddrop_fresh_bf_fox` | shielding, neutral straight to (56, -56): left alone, spot dodge at 136 |

**A replay's Dween port runs no fix.** Pre Frame is sampled at 0x8006B0E0,
after the code, and its joystick is fp+0x620: the recorded stick is already
the code's output (in `FALCO/Game_20200505T030157.slp` at tick 1850 the raw
X is 34 and the recorded joystick 0.0). The code changes nothing else, so
the runner gives such a port `ControllerFix::Off`; running `Dween` on those
inputs would apply it twice (the zeroed sample becomes the next tick's
"neutral"; four of the seven games then match fewer frames). The runner used
to hand a Dween port the UCF version named for the game's other port, which
ran UCF's dashback where the console ran none:
`PEACH/Game_20161204T213742.slp` stopped at tick 5376 and
`FALCO/Game_20190912T035958.slp` at 5426 on that port's Turn.

### The UCF 0.73 beta and which version a recording ran (2026-10-03)

Slippi's console set installed `Ucf0.73Beta.bin` as `g_ucf.bin` until
823067b (2019-10-09); the Dolphin lists and the console toggle set carried
the same program as source (`External/UCF + Arduino Toggle UI/UCF/UCF 0.73
{Dashback,Shield Drop} - Check for Toggle.asm`) until acc6f71 (2019-09-24).
`melee_ft::input::controller_fix::ucf073` ports the binary:

| | 0.73 beta | 0.74 / 0.8 |
|---|---|---|
| Turn's second frame | high half of the script frame, `lhz 0x3E8(fp)` = 0x4000 (CommandInfo frame_count; equal to the animation frame in Turn, ftAction_80073240 having just sampled it) | `cur_anim_frame` (+0x894) = 2.0 |
| Stick | `stick.x * mv.co.turn.facing_after` (+0x2344) >= PlCo+0x3C: **toward the turn** | `fabs(stick.x)` >= PlCo+0x3C: either way |
| Pad queue column | fp+0xC (slot) | fp+0x618 (port); the same for a human |
| Popo's partner | `gobj->next_gx` (+0x10): Nana, created right after Popo on the fighters' GX link (Player_80031AD0), never relinked | Player_GetEntityAtIndex(slot, 1) |
| Shield drop's lower limit | `-PlCo+0x2C` (the fast walk threshold, 0.8) | the literal -0.8 |

So the versions differ in one case: a slow turn whose stick, on the second
frame, is a full fresh tap back toward the old facing. 0.74 makes it a smash
turn (the facing flips at once, no dash, since the stick is not toward the
new facing); 0.73 leaves the slow turn. The shield drops compute the same
result.

The claim that 0.74 and 0.8 are one program was checked on the machine code:
the 0x800C9A44 and 0x800998A4 blocks of `g_ucf.bin` are byte-identical at
823067b (0.74, 2019-10-09), bb86519 (0.8, 2021-03-31) and 14b0f39; b89e160 /
d43a2a6 (2019-11-03) add only the per-port toggle test in front, and the
2022 text output differs only in the `backup` macro's stack frame.

**Version by recording.** `ucf_by_date`: before 2019-09-24 only 0.73 existed
in any Slippi output; from 2021-03-31 0.8; from 2024-02 0.84. In between a
setup ran 0.73 or 0.74, and the date does not settle it: consoles kept old
builds (`MARTH/11_12_26 Marth + Peach (BF).slp`, 2020-02-08, ran 0.73, as
other consoles kept 2019's spawn code into March 2020). The replay's Slippi
version does not separate the two either (every game in this corpus says
2.0.1; it does bound the choice, below). So
`resolve_controller_fix` reads it from the frames, between those two known
versions only: it runs the port under both in step from the recorded inputs
(no recorded state enters either run); the two compute the same match until
a dashback they disagree on, and on that tick the recorded facing is one
version's. That version then runs the whole comparison from the first frame.
A game with no such dashback keeps the dated version (0.74 from 2019-10-09,
the console set's date), and nothing distinguishes the two for it.

**The Slippi version bounds it (2026-10-03).** The date comes from the
console's clock, which is often unset or wrong: the second corpus has
recordings dated 1949, 2006, 2012, 2016 and 2018 (before Slippi 2.0.1
existed) and 2.0.1 recordings dated 2021 and 2023. The replay version is
written by the build, so `ucf_candidates` applies it first, from the history
of `Output/Console/g_core.bin` (the version it writes) beside `g_ucf.bin`:

| Replay version | UCF built beside it | Source |
|---|---|---|
| below 3.0.0 | the 0.73 beta (`g_ucf.bin` until 823067b, 2019-10-09, when the core wrote 2.2.0) | 2.0.1: 7062149 (2019-03-29) to 009b155 |
| 3.0.0 to 3.8.0 | 0.74 | 0ec965e (2019-10-24) |
| 3.9.0 on | 0.74 until bb86519 (2021-03-31), then 0.8, in the console and Dolphin outputs alike | f99bdbe (2021-02-17) wrote 3.9.0 first |
| 3.14.0 on | 0.84 exists: a separate console file `g_ucf_084.bin` (422bb78, 2023-05-07); Dolphin's list takes it at 6d3e140 (2023-12-18, 3.16.0). `g_ucf.bin` stays 0.8 | |

A recording below 3.9.0 therefore ran 0.73 or 0.74 whatever its date. Its
date still chooses between them when it is a date the build can have: not
before the version's first build (`VERSION_FIRST_BUILT`; earlier means an
unset clock) and not after 0.8 replaced them. Otherwise the version built
beside the core is the first candidate and the other the second, and the
frames choose as above; the report says `(by Slippi version, not the date)`.
A recording below 3.14.0 dated into 0.84's time runs 0.8. A recording below
3.9.0 with no date at all is no longer refused.

Second corpus: three 2.0.1 recordings dated 2021-06-17 and 2023-05-05 were
given 0.8 and stopped on its tumble wiggle (`cur_anim_frame` in DamageFall /
DamageFly at ticks 2256, 7339, 7529); all three now match to the last frame.
Every game of that corpus whose result depends on the version agrees with
the table (forced runs, `replay-batch --controller-fix`): six below 3.0.0
match further under 0.73 than 0.74, and three 3.9.0 games further under 0.8
than either.

**Witnesses** (`UCF_WITNESSES`), recorded with the Gecko codes `ucf-0.73`
(`git show 823067b^:Binary/UCF/Ucf0.73Beta.bin`) and `ucf-0.74`
(`git show b89e160^:Output/Console/g_ucf.bin`) in `MELEE_GECKO_DIR`, all
exact with 0 differing particle-site ticks:

| Scenario | Retail |
|---|---|
| `ucf_dashback_fd_fox_ucf073` | stick toward the turn: Dash at tick 121, as 0.74 |
| `ucf_dashback_away_fd_fox_ucf073` | stick -40 then +80: Fox still faces right at tick 121; the slow turn flips him at 125 |
| `ucf_dashback_away_fd_fox_ucf074`, `..._ucf08` | the same pads: Fox faces left from tick 121 |
| `ucf_dashback_fd_iceclimbers_ucf073` | Popo's smash turn rewrites Nana's newest follow sample through the GX link; she dashes at 126 |
| `ucf_shielddrop_bf_fox_ucf073` | the rim notch drops through the platform at tick 136, as 0.74 |

**Corpus** (108 games; `replay-batch`, no flag): 85 match to the last
frame (41 before, when the 57 games older than 2019-10-09 were refused; 81
with every game forced to 0.74), and no game matches fewer frames, with or
without `--controller-fix ucf-0.74`. Forced to 0.73 the result is the same
85. Four games reach a dashback the versions disagree on, and all four show
0.73: three from 2019 (`FOX/11_41_27` tick 1637, `FOX/12_39_37` tick 2522,
`FALCO/12_47_36` tick 3630, each a facing the port's 0.74 had flipped) and
`MARTH/11_12_26` (2020-02-08, tick 7087). Of the 50 games from 2019-10-09 on,
that one is the only one with such a dashback, so no game in this corpus
shows 0.74 itself: the other 49 match under either version as far as they
match at all.

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

## The Pre Frame seed check (2026-10-03)

Pre Frame carries the RNG seed at the fighter's input proc (hook 0x8006B0E0),
in every Slippi version. The port keeps retail's draw sequence but not its
timing inside a tick (effect requests flush at their own points), so the
runner checks membership, not position: the recorded seed must be one the
port's stream passes through between the previous tick's end seed and this
tick's (`compare_input_seeds`). When the port's stream has left retail's, the
first quiet tick fails it and the report names `pN.input_seed`: the drift
happened on the tick before, hundreds of ticks ahead of the fighter field
that would otherwise show it. The reported actual value is the tick's first
seed. A game can match every fighter field to the end and still fail this
check: the port drew a different number of values (an effect or sound).

## A replay played back on retail (2026-10-03)

A replay holds post-frame fields only, so a stop cannot be triaged against
it: no RNG seed (before 3.x), no items, no particles. Most stops in this
corpus are random draws gone out of step long before they show (a
DamageFlyRoll draw, a turnip's face). The bridge feeds the replay's pads to
retail in Dolphin from a boundary with the replay's setup; the ordinary gate
and `triage` then compare the port with a full retail trace of the same game.

```sh
melee-sim replay <game.slp> --retail-inputs inputs.jsonl   # setup, seeds, raw pads per tick
cd harness && uv run python make_boundary.py --stage Battlefield --players Marth Peach \
    --costumes 1 1 --ports 2 4 --time-limit 8 \
    --gecko ucf-0.8 neutral-spawn --spawn neutral-2020 --no-register
    # Final Destination, Fountain of Dreams: a boundary for this replay alone
    #   ... --game-start-seed <header game_start_seed> --name start_<...>_r<replay>
    # Pokemon Stadium: the console's codes, --gecko ... ps-preload [ps-frozen]
cd harness && uv run python slippi_to_scenario.py inputs.jsonl --name slp_<...> \
    --boundary start_<...> --ticks 6500
cd harness && uv run python record_many.py scenarios/slp_<...>.toml
melee-sim gate harness/scenarios/slp_<...>.toml             # follows `gate` to slp_<...>_cold
cargo run -q --release -p slp --example frames -- <game.slp> > frames.jsonl
cd harness && uv run python replay_vs_trace.py frames.jsonl slp_<...>   # is it the console's game?
```

What makes retail follow the replay:

- *Ports.* `make_boundary.py --ports 2 4` seats the players on the replay's
  ports (controllers in those ports only; `MELEE_SI_PORTS` for every
  recorder pass). Entry delay is per slot and fighters move before GO, so a
  match on other ports leaves the replay within 130 ticks.
- *Codes.* `--gecko` installs the build's codes from boot: NeutralSpawn
  (`neutral-spawn`: `External/NeutralSpawn/NeutralSpawn.asm`'s C2 block from
  `Output/Console/g_mods_tournament.txt`, in `MELEE_GECKO_DIR`) and UCF. The
  savestate keeps the installed code list in RAM, so a scenario recorded from
  the boundary lists exactly the boundary's codes. `--spawn` names the port's
  spawn rule for the cold scenario; `--time-limit 8` is the tournament timer.
  The Stadium codes (`ps-preload`, `ps-frozen`) set `stadium_preload` and
  `stadium_frozen` in the boundary's scenarios and in every scenario bridged
  from it; the header names the console's (replay version and `frozen_ps`)
  and `slippi_to_scenario.py` refuses a boundary whose codes differ.
- *Seed.* The scenario's `boundary_seed` is the replay's pre-music seed
  (`boundary_seed_from_creation`); every tracer writes it over the saved
  seed before the first tick. Retail then draws what the game drew: star or
  screen KO, the roll, the faces.
- *Creation.* What a stage draws while it is created precedes the boundary,
  so `boundary_seed` cannot change it. Final Destination keeps its
  background's two accelerations (grLast_8021AC30, four draws) and Fountain
  of Dreams its platforms' first waits (grIzumi_801CC358): from a boundary
  of another seed retail leaves the console's stream at the stage's next
  draw (FD: tick 209; FoD: the seed at 723, the heights at 854). There the
  boundary is made for the replay: `make_boundary.py --game-start-seed N`
  (the header's `game_start_seed`) writes Game Start's seed where Slippi
  read it (0x8016E74C in fn_8016E730, before the Ground and Players exist).
  The scripting API has memory breakpoints only, so `boundary_script.py`
  takes the first store after that address (gm_801A4B08 at 0x8016E75C
  storing gm_AnyControllerPressedStart at gm_80479D58 +0x14); nothing draws
  between. The savestate's seed is then the header's `boundary_seed`, which
  `slippi_to_scenario.py` checks, and the cold twin names `game_start_seed`,
  which its gate checks against the port's setup draws. Dream Land keeps one
  creation draw too, the first fly-by's delay (grOldPupupu_80210C7C, 3000 to
  4000 ticks): when it runs out the fly-by draws four values
  (grOldPupupu_80210D10) on another tick than the console's, so a bridge
  past tick 3000 needs the replay's own boundary as well ("Five stops of
  the second corpus" below). Battlefield keeps its first background delay
  the same way (grBattle_BG_Callback0, 0x8021A344, 2400 to 3600 ticks: "The
  bridge follows the console" below). Yoshi's Story takes any boundary (its
  creation draw is overwritten, 0x801E3304). `slippi_to_scenario.py` insists
  on the replay's own boundary for these two, for Dream Land past tick 3000
  and for Battlefield past tick 2400, unless `--foreign-creation` is given.
  The Battlefield witnesses below longer than that were made before the
  rule and follow their replays only up to the first background change.
- *Display passes.* gm_801A4D34 runs one tick per queued pad poll and then
  renders once. The tick input clock's injected entry and the retrace's own
  poll made two, so a recording ran two ticks per render, where a console
  renders after every tick. What the display pass feeds back into the game
  then differs: a tether's rope reads rendered joint matrices (so the wall
  catch and everything after it), the magnifier's damage, a screen KO's
  position. `slippi_to_scenario.py` therefore writes
  `render_every_tick = true` (`--dolphin-cadence` leaves it out): the
  recorder shortens the pad queue to one entry that drops a poll while full
  (HSD_PadRenewRawStatus's qtype 2; `one_entry_queue` in
  `harness/dolphin/tick_trace.py`), so psFrameNum moves on every tick.
  Bridges recorded before 2026-10-11 lack it; one that touches a tether or
  an off-screen fighter may be another game than the console's from there.
- *Pads.* Pre Frame's joystick is the fighter's dead-zoned stick, with the
  raw X beside it (1.2+). `--retail-inputs` finds the raw bytes that read as
  every recorded field (sticks, HSD's direction bits, triggers, buttons)
  with the recorded raw X (what UCF reads); the tick input clock injects
  them, `record.py` checks each tick consumed them and `replay_vs_trace.py`
  compares the recording's pads with Pre Frame ("The bridge follows the
  console" below).

The port gates the cold twin `<name>_cold.toml`: the boundary built from
parameters with its own `seed`, then `boundary_seed`, driven by the pads the
recording consumed (a `scripted` cold scenario with `expected`). The retail
scenario names it in `gate`, which `Scenario::load` follows, because the
savestate importer covers neither slots other than 0..n nor a replaced seed.
What the savestate fixed before the seed changed (each fighter's CPU timer)
stays the boundary's; nothing a human match reads.

Witnesses (`SLIPPI_REPLAY_WITNESSES` in `m5_gate.rs`), all exact with items
and particle draw order; retail follows each replay tick for tick over the
recorded range:

| Scenario | Replay | Finds |
|---|---|---|
| `slp_bf_captainfalcon_pikachu_t2900` | HNC 3, Captain Falcon vs Pikachu (BF, ports 1, 2) | Pikachu's second Quick Attack zip changes motion twice (frames 12 and 13, 0x80126FE4 / 0x8012700C); the second flushes the cheek spark the first queued, in the animation proc, before the other fighter's link-9 effects |
| `slp_bf_marth_peach_t6500` | Marth + Peach (BF, ports 2, 4) | a turnip clanking with a fighter's hitbox records him with its rehit timer (mode 4 when x41_b5, 0x80077B8C) and clanks with him again 33 ticks later |
| `slp_ps_fox_falco_t4000` | Fox vs Falco (PS, ports 1, 4) | nothing: the port equals retail to tick 4000 |
| `slp_ys_fox_falco_t1900` | Fox vs Falco (YS, ports 1, 2) | a throw runs its script on entry (ftAnim_8006EBA4, 0x800DD3FC), so its voice draw (0x80088A18) is made in the captor's input proc, before the stage's procs: at tick 1817 it precedes the puff timer's draw (lr 0x801E36B4), which the port had drawn first, taking the other value (a puff 16 ticks early) |
| `slp_ys_jigglypuff_fox_t300` | HNC 9, Jigglypuff vs Fox (YS, ports 2, 3) | Fox leaves Entry on the tick Randall's puff timer expires (15): ftCo_800C6408 queues the entry warp (0x800C66C8) after its motion change (0x800C644C), so it waits for the link-9 flush and its generators are created after the puff's |
| `slp_dl_peach_fox_t1300` | Peach vs Fox (DL, ports 1, 4) | Peach, sparking from a shine, starts her down smash in the input proc (1048): Fighter_ChangeMotionState flushes the queued spark (0x800694A0) before the new script's frame-0 commands (0x8006A0A4), so the spark generator's initial count (0x8039F250) is drawn ahead of the smash voice and it emits on its first update |
| `slp_bf_yoshi_samus_t860` | Yoshi vs Samus (BF, ports 1, 3) | the Yoshi Bomb's collision callback catches a ledge twice (828): ftCliffCommon_80081298 enters CliffCatch (0x8012E9D8) and the callback calls ftCliffCommon_80081370 again (0x8012E9E8), so there are two ledge flashes and the second generator draws on every tick it lives |

One Yoshi bridge is a local probe, not a witness: it shows a fault that is
still open.

- `slp_bf_yoshi_samus_t1000` (the same game to tick 1000) gates exact but
  differs in particle draw order at tick 870: Yoshi's Egg Throw (frame 20)
  and Samus's Run dust create a generator each on that tick (effect banks 9
  and 0), and the port creates them in the other order. The draw count is
  the same. `slp_fod_yoshi_yoshi_r1_t10100` shows it four more times ("The
  Yoshi stops" below).

`slp_ps_fox_yoshi_t1300` (`YOSHI/Fox vs Yoshi [PS] Game_20200222T144019.slp`,
ports 1 and 4, boundary `start_ps_slippi74pre_ns_p14_fox2_yoshi4_4`), which
stopped at tick 1091 on the stale powershield scratch of Yoshi's plain
GuardOn, is exact since 2026-10-12 and a witness ("The Yoshi stops" below).

`slp_dl_peach_fox_t1300` plays the replay's pads but is not its game after
tick 78: that console spawned Dream Land from the 2019 NeutralSpawn row (no
code text for it in `MELEE_GECKO_DIR`), the boundary from the 2020 one, and
Peach lands a tick later. It is retail all the same, and reaches the same
draw order the replay stopped on at tick 4403; the replay now matches to its
last frame.

`slp_ps_fox_falco_t4000` was recorded without Slippi's Stadium preload code,
which that console ran (above): retail draws DamageFlyHi at tick 3820 where
the replay shows DamageFlyRoll, the preload's earlier form draw having moved
every later draw by one. With `stadium_preload` the replay itself matches to
its last frame.

The Marth and Peach replay then stopped at tick 7087 on Peach's facing in
Turn: retail with UCF 0.8 has flipped it by then, as the port had
(`slp_bf_marth_peach_t7300`, a tick-trace probe in the local data, exact to
7300); the console had not, because it ran the UCF 0.73 beta ("The UCF 0.73
beta and which version a recording ran" above). With 0.73 the replay matches
to its last frame.

A bridge on Final Destination from a boundary of another seed does not keep
the replay's random stream. The background's two accelerations
(grLast_8021AC30, four draws) are drawn at stage creation, before the
boundary, so they stay the boundary's: retail's background meets its limits
(one draw each, grLast_8021ADD0) on other ticks than the console's did.
`MARTH/02_56_19 Captain Falcon + [PPAP] Marth (FD).slp` from
`start_fd_slippi8_p12_captainfalcon2_marth0_4` (local data,
`slp_fd_captainfalcon_marth_t1950`, exact to 1950) leaves the console's
stream at tick 209 and its fighters near tick 1630, so it says nothing about
that replay's stop at 1887. Fountain of Dreams does the same with its
platforms' first waits.

With the boundary created from the replay's Game Start seed (*Creation*
above) retail follows such a replay. Two complete games, each from its own
boundary, 3000 ticks, exact with items and particle draw order:

| Scenario | Replay, boundary | Creation | Stage draws in the range |
|---|---|---|---|
| `slp_fd_marth_marth_t3000` | `MARTH/12_07_47 Marth + Marth (FD).slp` (ports 1, 2; UCF 0.73); `start_fd_slippi73_p12_marth4_marth1_4_r120747` | Game Start seed 2534789673 written over the menus' 3684702607; the savestate holds 2462485393, the seed the port's eight setup draws reach | grLast_8021ADD0 at 206, 309, 606, 909, 1006, 1406, 1509, 1806, 2109, 2206, 2606, 2709 |
| `slp_fod_fox_falco_t3000` | `FALCO/11_43_09 Fox + Falco (FoD).slp` (ports 2, 4; UCF 0.74); `start_fod_slippi74_p24_fox2_falco0_4_r114309` | 1225871461 over 3417641259; the savestate holds 2669272315, the port's six draws | grIzumi_801CC358 at 858, 913, 931, 1925, 1958, 1964, 2010, 2689, 2847 |

That retail follows the replay rests on three checks: the savestate's seed
equals the header's `boundary_seed` (retail's creation drew what the port's
model of it draws); the cold twin gates exact, `rng.seed` included, on every
tick; and the port's own run of the replay matches every recorded frame of
both games (14414 and 5763) with the Pre Frame seed check, so over these
ticks retail's random stream is the console's.

## Display passes are not in a replay (2026-10-03)

The magnifier's off-screen flag is set by the display pass (ftLib_80086A8C
from each fighter's render callback; ifMagnify reads it on the next pass),
and Fighter_8006A360 counts PlCo +7AC ticks of it before each point of
damage. Retail does not render after every tick: the tick trace records
psFrameNum and the port follows it, but a replay has no such field and the
runner renders every tick. A game can therefore stop on `pN.percent`, one
point apart, with nothing wrong in the port.

`FALCO/02_33_48 Falco + Marth (PS).slp` does (frame 3042: expected 83.3,
actual 84.3). Marth's forward smash launches Falco off the right side; with
the camera shaking, Falco's projected x passes the scissor's right edge
between ticks 2982 (637.4) and 2983 (658.2), eighteen pixels past it, so
this is no rounding question. The console's first point of damage comes one
tick after the port's. Three clocks, three answers:

| Run | First magnifier point (tick) |
|---|---|
| Port, a display pass after every tick | 3042 |
| The console's replay | 3043 |
| Retail in Dolphin, same inputs (`slp_ps_falco_marth_ns_t3200`; a pass every other tick there) | 3045, and the port gates it exact |

Dropping the one display pass before tick 2983 (or 2984) in the port makes
the replay match all 11421 frames, seeds included: the console skipped one
render there. Nothing in the recording says so, and the runner does not
guess; the stop stays, as an external event like disc latency.

**The other consoles' `percent` stops** (second corpus, after the
Widescreen code took the HNC ones). Four are the same thing: the port one
point above the console, the damaged fighter far off the stage, and the
screen not the cause (two of them match the standard screen and not the
wide one at an earlier magnifier tick; the other two never separate them).

| Replay | Stop | Off-screen fighter | Screen read from the frames |
|---|---|---|---|
| `PEACH/Game_20191013T105824.slp` | 6022 | Link at (137.6, 46.1); the console's point comes at 6023 | standard, at tick 2509 |
| `ZELDA_SHEIK/Marth vs Sheik [FD] Game_20181024T232813.slp` | 6345 | Zelda at (-183.1, 3.2) | standard, at tick 2454 |
| `ZELDA_SHEIK/Game_20120109T050633.slp` | 2013 | Zelda at (-174.9, -20.8) | undecided |
| `ZELDA_SHEIK/19_56_47 [TREW] Sheik + Captain Falcon (DL).slp` | 3073 | Captain Falcon at (155.4, 58.4) | undecided |

The other two are not the magnifier: `FOX/Game_20190824T124750.slp` (1482:
28.58 recorded, 28.61 in the port) and `ZELDA_SHEIK/21_27_18 Zelda + Falco
(DL).slp` (4666: 85.37 and 85.17) differ by a hit's damage, not a point.
The second was a stale-move entry the port did not make ("Zelda and Sheik
stops" below) and now matches to its last frame.

Nothing in these recordings bounds when a display pass happened. They are
Slippi 2.0.1: Pre Frame and Post Frame only, one pair per fighter and tick
whatever the console rendered. The frame bookend (3.0.0, `da05c16`) carries
the frame number and, later, the latest finalized frame; neither says
whether the frame was drawn. So the runner still renders every tick and
these four stops stay.

**Nana takes no magnifier damage.** The four `follower.percent` stops of
that corpus (two HNC, two not) were a port fault, not a display pass:
Fighter_8006A360 tests x221F_b4, the player's second fighter, at 0x8006A830
(`lbz 0x221F; extrwi. r0, r0, 1, 28; bne`) before the camera's zoom and
skips the whole block, where the port counted her magnified ticks like
Popo's and gave her the point. With the test all four pass their stop
(1178 to 3921, 1931 to 2765, 2098 to 3799, 2903 to 6004; each then reaches
an unported Ice Climbers boundary) and no Ice Climbers game of either
corpus matches fewer frames. There is no directed retail witness: the
bridge of one of them (`slp_bf_luigi_iceclimbers_t2300`, from
`start_bf_slippi74_ns_p14_luigi2_iceclimbers0_4`) leaves the console's game
at tick 540 on Luigi's height (retail in Dolphin 1.946, the console's
replay 1.146), long before Nana is off screen. The cause is stale heap, not
the bridge: Fighter_Create leaves Luigi's +222C (`x222C_cycloneCharge`)
unwritten, the booted boundary's block holds 0x0932038D there, and his first
aerial Cyclone takes the charged branch, where the console's block held
zero. The cold twin now carries the boundary's word and gates all 2300
ticks (`SLIPPI_REPLAY_WITNESSES`); a replay still runs with zero
(`docs/PORT_NOTES/STALE_HEAP.md`).

## Frozen Stages: the stage code a replay does not record (2026-10-03)

Some consoles ran a stage that never drew what retail draws: no Shy Guys,
no Final Destination background phases, no Stadium transformation, no wind
on Dream Land. The seed check shows it on the tick after the first draw the
console skipped; nothing in the replay names the code, and Game Start is
byte-identical to the games that do draw (`frozen_ps` is 0: the recorder
only tests the instruction at 0x801D45FC).

**The code.** `melee_lib::slippi::SlippiCodes::frozen_stages` is "Frozen
Stages" [UnclePunch, Fizzi]: slippi-ssbm-asm `External/Frozen All/Core/1-4.asm`,
built into `Output/Console/g_stages_all` (logic ported, not copied; GPL-3).
It entered the Slippi tree on 2021-06-02 (3438f54, `Binary/FreezeAllStages.bin`,
32 bytes); nothing in slippi-ssbm-asm or Slippi Nintendont before that
carries the Shy Guy, Stadium or Dream Land write (every blob of both
histories was searched), so the 2020 consoles loaded it themselves (Slippi
Nintendont copies a user `.gct` into RAM when Cheats is on, `kernel/Patch.c`).
Four writes, one instruction or one table entry each:

| Write | Site | Effect | Port |
|---|---|---|---|
| `041E3348 60000000` | `bl grStory_801E3418` in grStory_801E3334 | the Shy Guy spawner never runs (retail's first group: tick 120, six draws) | `melee_gr::story::Story::shy_guys_disabled` |
| `0421AAE4 60000000` | `bl grLast_8021B2E8` in grLast_8021AAB0 | the phase update never runs: the hold timer stays at zero, so the first transition (its draws at tick 1885) and every later phase never come | `melee_gr::ground::Ground::phase_update_disabled` |
| `041D1548 60000000` | `bl grStadium_801D4548` in grStadium_801D1520 | the transformation controller never runs: no timer, no form, and not the preload code's draw on the first waiting tick (85), whose hook is inside it | `melee_gr::stadium::transform::Transformation::disabled` |
| `043E67E0 00000000` | grOp_803E67D8[2], Whispy's cycle table `{ 0, 1, 2 }` | the cycle's third step is the wait instead of the wind: Whispy turns to the fighters, then draws a new wait and blink timer (two draws retail does not make) | `melee_gr::pupupu::Pupupu::wind_disabled` |

The three nops are older standalone codes: "Disable Yoshi's Story Shyguys
[Zauron]" and "Disable Pokemon Stadium Transformations [Zauron]" in
Dolphin's `Data/Sys/GameSettings/GALE01r2.ini`, and "Disable FD Background
Transitions [Achilles, Dan Salvato]", which Slippi's Dolphin builds have
always installed (`Binary/FasterMeleeSettings/DisableFdTransitions.bin` in
`netplay.json` from 3e47f17, 2019-01-07). 20XX TE's Frozen Mode
(dansalvato/20XXTE `source/loader/code/frozen_mode.mgc`) has the same three
but stops Whispy at 0x802115B4 (the waiting state's `ble`) and adds
0x8020F4C0 (Kongo Jungle 64); that Dream Land write gives other draws than
the consoles show and is not ported.

**Witnesses** (`frozen_stages_code_matches_retail`), recorded with the Gecko
code `frozen-stages` (the four Core lines of `g_stages_all.txt`, in
`MELEE_GECKO_DIR`; a scenario says `frozen_stages = true` beside it), all
exact with 0 differing particle-site ticks; with the port's code off each
leaves retail on the tick named:

| Scenario | Retail with the code | Port without it stops at |
|---|---|---|
| `frozen_stages_ys_fox_marth4` (2400 ticks) | no Shy Guy is ever created | 121 |
| `frozen_stages_fd_marth_marth4` (13600) | the first background phase to the end, past the star field's ticks | 1886 |
| `frozen_stages_ps_fox_marth4` (7500, with `ps-preload`) | no form draw at tick 85, no transformation; the jumbotron and audience go on drawing | 85 |
| `frozen_stages_dl_fox_marth4` (5000) | Whispy turns and waits again, nobody is pushed | 1176 |

**Which replays ran it.** `replay_stage_codes::resolve_stage_codes` reads it
from the frames, on the four stages the code changes, before the comparison
and before the UCF probe (`resolve_setup`): the port runs the recorded
inputs without and with the code in step; nothing is asked of the recording
until the two runs differ (a compared field or the tick's end seed); from
then on each is compared with it as the comparison would be, Pre Frame
seeds included, and the first tick only one of them matches names that
setup. It then runs the whole comparison from the first frame; no recorded
state enters any run. When the runs never differ over the recording, or
both stop matching on the same tick (another fault came first), the stage is
retail's. The report's `stage code` line says which and why;
`--stage-codes <none|frozen-stages>` names it instead.
The bridge carries the choice: `--retail-inputs` writes `frozen_stages` in
its header on these stages, and a boundary made with `--gecko frozen-stages`
sets the flag in its scenarios (`gecko.SCENARIO_FLAGS`).

| Corpus | Complete before | after | Chosen `frozen-stages` |
|---|---|---|---|
| `public-v3.7` (108) | 103 | 106 | 3: `HNC 9 Jigglypuff vs Fox (YS)` 121 to 7314 / 7314; `Fox + Captain Falcon (FD)` 1886 to 9630 / 9630; `Captain Falcon + Marth (FD)` 1887 to 6854 / 6854 |
| `public-v3.7-b` (502) | 247 | 284 | 73 (frames matched 2,878,309 to 3,342,557) |

No replay matches fewer frames in either corpus. One reports fewer:
`HNC 10 Luigi vs Zelda (PS)` passed tick 85 and reached Zelda's
`walk/squat transition: InvalidTracks` panic at tick 555, which the runner
reports as an error without a frame count (five other Zelda games stop on
that panic with or without this change). Every game on these four stages
that reaches its deciding tick is decided; the undecided ones stop earlier
on something else. In `public-v3.7-b`:

| Stage | HNC consoles (Jan-Mar 2020) | Other games |
|---|---|---|
| Yoshi's Story | 21 frozen (12 complete) | 49 not frozen (40 complete) |
| Final Destination | 10 frozen (6 complete) | 10 frozen (all complete), 43 not (35 complete) |
| Dream Land | 16 frozen (9 complete) | 47 not frozen (36 complete) |
| Pokémon Stadium | 16 frozen (none complete) | 56 not frozen (37 complete) |

The ten other frozen Final Destination games are two 2.0.1 consoles, one
2.0.1 Dolphin recording and the seven Slippi 3.0.0 / 3.3.0 games that
stopped on `rng.seed` at 1885: those stops are the code (each game matches
to its last frame with it), not the Frame Start seed check being stricter.

**What was still open on the HNC consoles** (Stadium games stopping on
`input_seed` 700 to 4700 ticks in, 24 games stopping on `percent`) was the
Widescreen code: the next section.

## Widescreen: the screen code a replay does not record (2026-10-03)

Forty of the HNC games ran on a 16:9 picture. Nothing in the replay says
so. Two things show it: the magnifier's damage falls a tick (or many) later
than on a 4:3 screen, and Pokémon Stadium's jumbotron keeps a close-up the
4:3 screen would have dropped, so the stage draws its next mode later.

**The code.** `melee_lib::slippi::SlippiCodes::widescreen` is "Widescreen
16:9" [Dan Salvato, mirrorbender, Achilles1515, UnclePunch]: slippi-ssbm-asm
`External/Widescreen`, with the off-screen bounds of e9457ee (2019-11-03,
"use 4:3 offscreen damage behavior for widescreen"); logic ported, not
copied; GPL-3. Slippi's Dolphin lists carried it from then; the console
output `g_screen_wide` dates from 4548f3f (2021-05-31), so the 2020 consoles
loaded it themselves, as they did Frozen Stages. Its writes:

| Write | Site | Effect | Port |
|---|---|---|---|
| `C236A4A8` | `lfs f1, 0x34(r31)` in CObjLoad (0x8036A2EC): the description's perspective aspect | every perspective CObj's aspect is the description's `* 320 / 219` (`fmuls`, `fdivs`), so a projected x lies nearer the centre by 219/320. The camera's framing does not change: it reads the description's aspect (cm_803BCB64), not the CObj's | `melee_cm::Screen::aspect`, taken by the rendered CObj (`GameCamera::render_camera`) |
| `04030C7C 38000064`, `04030C88 3800021C` | Camera_80030BBC's `lhz` of the scissor's left and right | the on-screen test compares the window x with 100 and 540. That is about where the 4:3 edges fall in the wider picture (320 -/+ 219 = 101, 539), a pixel outside them: a fighter leaves the screen where the 4:3 screen's x would be -1.46 or 641.46, so up to one crossing tick later, and the magnifier's sixty-tick count (Fighter_8006A360) starts later or not at all | `melee_cm::to_screen` with `Screen::Widescreen` |
| `04086B24 60000000` | ftLib_80086A8C's test of Camera_80030CFC | the model is drawn whether or not it is near the screen; display only | none |
| `043BB05C`, `044DDB28..58`, `C22FCFC4`, `044DDB84` | screen flash, bubble zoom and placement, nametag scale | display only | none |

Pokémon Stadium's close-up test (grStadium_801D32D0) projects the player's
camera bone through the main CObj and asks that a 124-pixel box around it
fit the viewport: with the wide aspect a fighter out to the 4:3 picture's
edge and beyond still fits, the close-up lasts, and the mode draw that would
have ended it is not made. Dream Land's fly-bys read the CObj's aspect too
(Camera_800307D0), for where a background model is placed: nothing drawn
from the random stream depends on it.

Nintendont's own "Force Widescreen" (`kernel/PatchWidescreen.c`: C_MTXPerspective's
aspect times 4/3, the scissor untouched) is not this. Measured on one game
with a temporary experiment (`YLINK/20200205 - HNC 13 - ... Peach vs Young
Link - Battlefield`, 1565 / 4511 on the standard screen, the port a point
above): that patch and the Slippi code without its bounds (the tree before
e9457ee) each stop at 1566 with the port a point below, and the aspect with
the bounds of 100 and 540 matches all 4511 frames. Only the last was run
over the corpus.

**Witnesses**, recorded from boundaries booted with the Gecko code
`widescreen` (the code's text at 4d15973, `Output/Netplay/GALE01r2.ini`
without the deleted Offscreen Bubble Fix line, in `MELEE_GECKO_DIR`; a
scenario says `widescreen = true` beside it). The aspect hook runs when a
CObj is loaded, so the code must be installed before the match is created:
on another boundary's savestate only the bounds would apply. All exact with
0 differing particle-site ticks:

| Scenario | Retail |
|---|---|
| `widescreen_hover_slow_ps_marth_jigglypuff4` (`widescreen_code_matches_retail`; boundary `start_ps_wide_marth_jigglypuff4`) | Jigglypuff drifts off Stadium's right side and hovers at x = 170.2: the magnifier's first point of damage comes at tick 352. The port without the code leaves the recording at tick 340 (it has the point at 341) |
| `standard_hover_slow_ps_marth_jigglypuff4` (boundary `start_ps_marth_jigglypuff4`) | the same pads without the code: the point comes at tick 341 |
| `slp_ps_fox_marth_wide_t2000` (`SLIPPI_REPLAY_WITNESSES`; boundary `start_ps_slippi74_ns_frozen_wide_p12_fox0_marth0_4`: UCF 0.74, NeutralSpawn, preload, Frozen Stages, Widescreen) | `FOX/20200122 - HNC 21 - PM 0952 - Fox (Default) vs Marth (Default) - Pokemon Stadium.slp` played back for 2000 ticks: the jumbotron's close-up outlasts tick 945. The port without the code leaves the recording there on `rng.seed` (the stage proc's draw) |
| `slp_ps_fox_marth_standard_t2000` (boundary `start_ps_slippi74_ns_frozen_p12_fox0_marth0_4`) | the same pads without the code (the inputs header's `widescreen` set to false): retail's seed leaves the widescreen recording's at tick 945, with the value the port computes without the code |

A second pair with a faster drift (`widescreen_hover_fast_...`,
`standard_hover_fast_...`, local data) gates exact too but does not separate
the screens: its first point comes at 326 with the code and 325 without,
and the port without the code passes the widescreen recording.

**Which replays ran it.** `replay_screen_codes::resolve_screen` reads it from
the frames, as the stage code is read: the port runs the recorded inputs on
the standard screen and the wide one in step (`recorded_between`, shared
with the stage code); nothing is asked of the recording until the two runs
differ; the first tick only one of them matches names the screen, which
then runs the whole comparison from the first frame. When they never differ,
or both stop matching on the same tick, the screen is retail's. It is read
after the stage code and under it; when the standard screen left the stage
code undecided and the frames then show the wide screen, the stage code is
read again on the wide screen (`resolve_codes`). The report's `screen` line
says which and why; `--screen <standard|widescreen>` names it instead. The
bridge carries the choice: `--retail-inputs` writes `widescreen` in its
header, a boundary made with `--gecko widescreen` sets the flag in its
scenarios, and `slippi_to_scenario.py` refuses a boundary whose flag differs.

| Corpus | Complete before | after | Read as widescreen | Read as standard | Never separated |
|---|---|---|---|---|---|
| `public-v3.7` (108) | 106 | 106 | 0 | 35 | 73 |
| `public-v3.7-b` (502) | 325 | 356 | 40, all HNC (frames matched 3,780,417 to 3,995,461; 4,003,840 with Nana's fix, above) | 148 (7 HNC) | 313 |

No replay matches fewer frames in either corpus, and every recorded
scenario gates as before (1762 pass, 66 fail, the same ones). The HNC
series in the second corpus (120 games, 114 runnable): 55 complete before,
86 after; `percent` stops 24 to 3, `input_seed` stops 17 to 4.

| Stage (HNC) | Games | Widescreen | Complete |
|---|---|---|---|
| Battlefield | 23 | 4 | 19 |
| Dream Land | 17 | 4 | 13 |
| Final Destination | 16 | 5 | 10 |
| Fountain of Dreams | 21 | 2 | 12 |
| Pokémon Stadium | 20 | 17 | 10 |
| Yoshi's Story | 23 | 8 | 22 |

Not every station ran it: seven HNC games match the standard screen and
not the wide one (forced to the wide screen each matches fewer frames), and
73 never put a fighter in the band that separates the two. On Stadium the
jumbotron separates them within the first minutes of almost every game,
which is why 17 of 20 are read there. One game outside the series was read
as widescreen before Nana's magnifier damage was fixed (above): the wide
screen had only delayed a point she never takes.

**Still open on the widescreen consoles.** Two HNC games now stop on
`percent` with the port one point *below* the console (`HNC 1 Fox vs
Dr. Mario (PS)` at 7008, `HNC 21 Marth vs Jigglypuff (FoD)` at 5267), and
three Stadium games on `input_seed` further in than before (2470, 3709,
10437). Both tests read the CObj of the last display pass, which a replay
does not record (above); nothing here separates a skipped pass from a
difference in the code those consoles ran.

## Consoles with level Fountain of Dreams platforms (2026-10-11)

Six second-corpus games, all Fountain of Dreams at one tournament day (HNC,
2020-03-04, stations 4, 12, 13, 18, 21 and 22), stopped at tick 0 on
`p0.cur_anim_frame` (EntryStart frame 0 recorded, Entry -1 in the port).
That field was only the first one compared: no spawn rule fitted their first
frame, so the runner fell back to retail's.

**What the frames require.** Every game of that day, on every stage (25 in
the corpus), has the first player in EntryStart on the first frame: entry by
spawn order (above). Its Fountain of Dreams games differ from all 35 others
in both corpora in two more things, and in nothing else:

| | Ordinary game | 2020-03-04 |
|---|---|---|
| Leaders on the first frame | (-41.25, 21.0), (41.25, 27.0): NeutralSpawn's row | (-41.25, 24.0), (41.25, 24.0) |
| Where each lands on its side platform | y = 16.125 and 22.125 | y = 19.125 both |

A landing height is 0.75 x the platform's height + 1.125 (the map scale and
the collision joint's parent), so the platforms were created at 24.0 and
24.0 where retail passes 20.0 and 28.0 (`yakumono_param` +0x0 and +0x8, the
`lfs f1` at 0x801CC02C and 0x801CC084 before grIzumi_801CBE64's two calls of
grIzumi_801CCBDC, which stores the argument as height and target, gp+D0 and
gp+D4). Each spawn row is 4.875 above its platform in both columns. The
platforms then move as retail's do: five whole games match with nothing
else of the stage changed.

**The code is not public.** No text was found for it (2026-10-03 and again
2026-10-11): every `Output/Console/*.bin` of slippi-ssbm-asm at 4dc7447^
(NeutralSpawn's row is (21, 27) at every commit of the file and in
UnclePunch's Training-Mode copy; the External directory today names no
Fountain code but "Disable FoD During Doubles"); Slippi
Nintendont's history for a Gecko line on 0x801CBB00..0x801CD400; 20XX TE's
source; Altimor's 1.03 (`qol/spawns.cpp` has (21, 27), `stages/fod.cpp`
only the reflection); Slippi's "Lagless FoD" (0x801CBB90, 0x801CC8AC,
0x801CBE9C, 0x801CBEF0, 0x801CBF54, 0x80390838, 0x801CD250, 0x801CCDCC:
reflection, particles and the pillar's x scale, neither height); and a
GitHub code search for a C2 or 04 line at 0x801CC02C, 0x801CC030,
0x801CC084, 0x801CC088 or 0x801CCBDC. Those consoles loaded their own
`.gct`, as they did Frozen Stages and Widescreen; this one changed between
2020-02-19 and 2020-03-04. Whether it was a Gecko code or a changed
`GrIz.dat` cannot be told from a replay.

**The port.** `SlippiCodes::fountain_level` makes both creation heights 24.0
(`fountain_creation_heights`, at the two argument loads), and
`NeutralTable::LevelFountainEntryByOrder` (`spawn =
"neutral-2020-entry-fod-level"`) is the 2020 table with entry by spawn order
and Fountain of Dreams' first two singles rows at y = 24. Rows 2 and 3 and
the teams rows were never shown and are not ported (`neutral_spawn` returns
`None`: the setup fails instead of guessing).

**Which replays ran it.** The first frame says so, as it says the spawn
rule: `spawn_rule` tries the level table last, and it fits only when both
leaders stand on the y = 24 rows of Fountain of Dreams with the first
already in EntryStart. No game that another rule fits can change. The report
prints a `fountain fod-level (...)` line with the reason; `--retail-inputs`
writes `fountain_level` in its header on this stage, and
`slippi_to_scenario.py` refuses a boundary whose flag differs.

**Retail witness, with constructed codes.** `harness/gecko_fod_level.py`
writes two codes into `MELEE_GECKO_DIR` that do what the frames require and
nothing else (a scenario lists them in `gecko` and says `fountain_level =
true`):

| Code | Text | Why this form |
|---|---|---|
| `fod-level` | `C21CC02C` and `C21CC084`, three lines each: `bl .+8`, `.float 24.0`, `mflr r6`, `lfs f1, 0(r6)` | the instruction replaced is the load of the creation height itself (`lfs f1, 0x0(r6)` / `lfs f1, 0x8(r6)`); r6 is dead after it and the caller's own `bl` follows, with its lr saved in its frame |
| `neutral-spawn-entry-fod-level` | `neutral-spawn` (the 2020 C2 block at 0x8016E510) with `SetSpawn_AdjustEntryFrames` back after the facing store (`mr r3, r31`; `mulli r4, r30, 5`; `branchl r12, 0x80035FDC`, the lines the source comments out since 4dc7447; ten relative branches across them moved 24 bytes) and the two singles rows at y = 24 | the consoles' entry timing and rows |

| Scenario | Retail |
|---|---|
| `start_fod_slippi74_level_p12_fox3_jigglypuff4_4_r0728` (boundary; created from the replay's Game Start seed 1209004498, which reaches the header's boundary seed 1675093088) | Fox leaves Entry on the first tick and Jigglypuff five ticks later; both stand at y = 24 and land on platforms at 19.125 (tick 68). The cold twin gates exact over 600 ticks; without `fountain_level` it leaves retail at tick 68 (Fox falls through where the platform is), and with any other spawn rule at tick 0 |
| `slp_fod_level_fox_jigglypuff_r1_t13500` (`SLIPPI_REPLAY_WITNESSES`) | `JIGGLYPUFF/20200304 - HNC 12 - PM 0728 - Fox (Green) vs Jigglypuff (Yellow) - Fountain of Dreams.slp` played back for the whole game (13500 ticks, a display pass after every tick): the port equals retail on every tick, seed included, and the port's own run of the replay matches all 13511 recorded frames, so retail under these codes plays the console's game, seven KOs long |

That is evidence for the behaviour, not for the console's code text: a code
that differed in something no fighter field or seed shows (the pillar's
model, say) would pass the same checks.

`slp_fod_level_fox_jigglypuff_t13500` (local data) is the same pads without
`render_every_tick`, from the bridge before it wrote that flag: retail there
leaves the console's game near tick 3500 and ends at 6914, as other
two-ticks-per-render recordings do. It is not a witness.

| Replay | Before | After |
|---|---|---|
| `JIGGLYPUFF/20200304 - HNC 12 - PM 0728 - Fox (Green) vs Jigglypuff (Yellow)` | 0 / 13511 | 13511 / 13511 |
| `GANONDORF/20200304 - HNC 4 - PM 0900 - Ganondorf (Default) vs Marth (Default)` | 0 / 15081 | 15081 / 15081 |
| `FALCO/20200304 - HNC 13 - PM 0659 - Falco (Red) vs Sheik (Red)` | 0 / 7607 | 7607 / 7607 (widescreen, read at 1356) |
| `PIKACHU/20200304 - HNC 18 - PM 0832 - Falco (Green) vs Pikachu (Blue)` | 0 / 8616 | 8616 / 8616 (widescreen, 1331) |
| `YLINK/20200304 - HNC 21 - PM 0854 - Fox (Default) vs Young Link (Black)` | 0 / 10312 | 10312 / 10312 (widescreen, 4142) |
| `ZELDA_SHEIK/20200304 - HNC 22 - PM 0855 - Falco (Default) vs Zelda (White)` | 0 / 11472 | 8969 / 11472 (widescreen, 2258), then `p1.cur_pos.x`: the second player's fighter, in special state 344, is 0.025 further left on the console one tick before the port moves it, as Falco (DamageHi2) leaves hitlag. Another fault, open |

Both corpora, `replay-batch` before and after: `public-v3.7-b` 434 to 439
complete of 502 (frames matched 4,676,200 to 4,740,296), these six the only
replays that change; `public-v3.7` unchanged (107 of 108). No replay matches
fewer frames and no replay's controller fix, stage code or screen choice
changes.

## The pad queue is not in a replay (2026-10-03)

UCF 0.73/0.74/0.8's dashback compares the hardware pad queue's newest
sample with the one two polls earlier (`qread - 1` and `qread - 3`,
"Controller fixes" above). The runner builds that column from Pre Frame's
raw X at this tick and two ticks ago: one poll per tick. A console that
polled twice within one tick (a slow frame) has last tick's sample there
instead, and nothing in the recording says so.

`PICHU/Game_20161120T232910.slp` (2158 / 16108, Turn frame 2 recorded where
the port restarts the turn as a smash turn) is that case, not a different
fix. Over the whole game, the second tick of a slow Turn with a full stick:

| Raw X change over two ticks / one tick against 75 | Console left the slow turn | Console kept it |
|---|---|---|
| over / under | 93 (92 Dash, one KneeBend) | 1 (tick 2158) |
| over / over | 3 | 0 |
| under / under | 43 (already past frame 2's test: other causes) | 16 |

Tick 2158's raw X runs -2, -53, -100; tick 9848's runs -3, -53, -100 on the
same port and dashes. Against the sample one tick back (-53) the change is
47, under the bound, which is what a queue holding one extra poll reads.
The other leader had respawned 20 ticks earlier. Forced to 0.74 the stop is
the same; with the fix off the game stops at tick 172, so the console did
run UCF. The stop stays, as an external event like the display pass.

## Samus's wall tether and the display cadence (2026-10-11: port faults)

Three Samus games stopped on `input_seed` with the port exactly three draws
ahead while she is on the aerial grapple's wall tether (AirCatchHit, 358),
and an earlier reading here called them console events: the bridge of
`SAMUS/Samus vs Puff [YS] Game_20180824T234533` (`slp_ys_samus_jigglypuff_t2900`)
equals the port for 2900 ticks. That recording ran two ticks per render
("Display passes" above). The rope reads rendered joints, so from the wall
catch on it is not the console's game, and the port (which follows a
recording's display clock) followed it there.

Recorded with `render_every_tick` (`slp_ys_samus_jigglypuff_r1_t2900`, the
same boundary and pads), retail in Dolphin gives the console's seed on every
tick, and the port left it at 2807 with the replay's own values (retail
0x32DC8D48, port 0x263BD4BD). The faults, all the port's:

| Fault | Retail | Replays (matched before to after) | Witness |
|---|---|---|---|
| Graphics commands issued from the rope's proc were created in script order | The rope runs in accessory2 (it_802BAC80 from Fighter_CallAcessoryCallbacks_8006C624, s_link 8). A catch, hop or let-go there changes motion and steps the new script (ftCo_8009B390: 0x8009B3E8, 0x8009B3F0), whose frame-0 graphics reach efAsync_Spawn below link 9 and queue (0x80067948); the link 9 flush creates them newest first (0x3FF, 0x6C, 0x6B). The scene called accessory2 after it had resolved the proc's graphics, so these waited for the link 9 proc and were dispatched there at once, oldest first | `SAMUS/Samus vs Puff [YS]` 2807 to 15686 / 15686; `SAMUS/17_27_54 Samus + Falco (YS)` 6767 to 13142 / 13142; `SAMUS/Game_20161113T214526` 7250 to 10866 / 10866; `SAMUS/18_51_20 Samus + Samus (FD)` 13358 to 20934 / 20934; `CPTFALCON/Game_20190629T183728` 10309 to 13908 / 13908 | `slp_ys_samus_jigglypuff_r1_t2900` (2799, 2807) |
| The Charge Shot's muzzle flash was created before the hand glow went | ftSs_SpecialN_801293BC: it_802B56E4 (0x8012946C) runs the shot's own efLib_DestroyAll (it_802B5CBC, 0x802B5CEC) before ftSs_SpecialN_801291F0 (0x80129504) and efSync_Spawn (0x80129524), so the flash's three generators go in behind the head the glow left. The port flushed the flash ahead of the launch request; a dust generator alive at the time then drew other values and emitted (three draws) on its last tick | `PIKACHU/13_03_44 Samus + Pikachu (FD)` 4155 to 7413 / 7413 | `slp_fd_samus_pikachu_r1_t4250` (4146..4151) |
| A second motion change in one proc left the first's graphics behind | Fighter_ChangeMotionState's efAsync_QueueFlush (fighter.c:951) takes every request queued so far, the frame-0 graphics of the state being left included. Samus's Screw Attack catches a ledge twice (ftCliffCommon_80081298, then ftCliffCommon_80081370 again): retail creates flash, 0x6C, 0x6B, then the same three again; the port created both flashes first. The draw count is the same; the generators draw each other's values | none by count | `slp_fd_samus_pikachu_r1_t4250` (2503: 0 differing particle ticks) |
| used_tether was cleared by a death | Only creation (fighter.c:777) and a grounded motion change (fighter.c:1138) clear +0x2228 bit 6; Fighter_UnkInitReset_80067C98 writes bit 1 of that byte (0x80068318). Samus, dead after an aerial grapple without landing, has no grapple off the revival platform: Z is an aerial | `SAMUS/20200205 - HNC 12 ... Pokemon Stadium` 2748 to 12611 / 12611 | `slp_ps_samus_captainfalcon_wide_r1_t2850` (2749; boundary `start_ps_slippi74_ns_frozen_wide_p23_samus3_captainfalcon0_4`) |
| A tether's captor walked toward a victim behind the hold joint | fn_800DA190 (the pull's accessory1, installed at 0x800D9E44) skips fn_800DA054 for Link, Samus and Young Link (0x800DA1A4..C0) while the article reels the catch in. With the button code's beam Samus can catch a fighter behind her | `SAMUS/20200305 - HNC 15 ... Pokemon Stadium` 2596 to 8260 | `samus_grab_code_behind_fd_fox4` (289), directed |
| No revival invincibility after a grapple or an item catch off the platform | ftCo_RebirthWait_IASA: ftCo_800D7100 (0x800D57F4) and ftCo_800C3B10 (0x800D5804) are priority inputs like the others, so ftColl_8007B7A4 (0x800D58C4) follows them. The port returned before it | the same game, 8260 to 9976 / 9976 | `samus_rebirth_tether_laser_fd_fox4` (411, 478), directed |

The HNC 15 game cannot be bridged (its console spawned by the 2019 entry
rule, for which there is no Gecko text; `slp_ps_samus_falco_wide_r1_t2700`
is another game from tick 1 and is not registered), hence the two directed
witnesses.

**A lag frame on Fountain of Dreams.** `SAMUS/Game_20190629T124717` stops at
7813 on `input_seed`. Played back with a render after every tick
(`slp_fod_samus_marth_r1_t7900`, local data), retail in Dolphin follows the
replay's fighters and seeds to tick 7809 and equals the port for all 7900
ticks; on tick 7810 the console drew 220 values where retail and the port
draw 189, all in the particle interpreter. `melee-sim replay <game>
--skip-display-pass 7809` (the tick runs with no display pass before it, so
no particleSort) matches all 9587 frames: the console dropped one render
there, as that stage does on a console. Since proven on retail with a
lag-frame recording, and the runner now infers it ("Lag frames read from a
replay" below).

## State-timing and position stops in the second corpus (2026-10-03)

The `cur_anim_frame`, `cur_pos` and `motion_id` stops of `public-v3.7-b`
(Ice Climbers and the level Fountain of Dreams consoles aside). Six port
faults, each a retail callback wired to the wrong helper or a branch not
ported; five are pinned by the replay's own window played back on retail
(`SLIPPI_REPLAY_WITNESSES`), exact with items and particle draw order.

| Fault | Retail | Replays (matched before to after) | Witness |
|---|---|---|---|
| Rebound stopped at a floor's edge | ftCo_Rebound_Coll (0x80099EFC) calls ft_80083F88 (0x80099F08: ft_80082708, Fall off the floor), not the edge-stopping ft_80084104 the port had borrowed from the attacks: the clank's recoil (ftCommon_800804A0) carries a fighter standing at an edge off it | `YOSHI/Yoshi vs Falcon [DL] Game_20200222T180833` 269 to 4951 / 4951; `MARIO/21_26_40 Luigi + [C2] Mario (FoD)` 911 to 9345 / 9345; `MARIO/YLink vs Mario [DL] Game_20200226T224213` 1034 to 9835 / 9835; `PICHU/20200108 - HNC 10 ... Final Destination` 6305 to 14674 / 14674 | `slp_fod_luigi_mario_t1100` (retail falls at 912; boundary `start_fod_slippi73_p23_luigi1_mario4_4_r212640`) |
| A delayed powershield's reflect bubble stayed where it was created | ftCo_GuardReflect_Phys calls ftColl_8007AEF8 (0x80094060) on every tick out of hitlag. ftCo_8009388C (GuardOn, digital press inside the window) leaves the shield bubble off (x221B_b0 clear) and the port cleared the reflect bubble's cached position only for an active shield, so a sliding fighter's bubble stayed behind and the projectile hit the body or the later shield | `FALCO/20200212 - HNC 5 ... Dream Land` 579 to 8278 / 8278; `SAMUS/18_51_20 Samus + Samus (FD)` 10477 to 13358 / 20934 (then `input_seed`, Samus) | `slp_dl_falco_falco_fz_t700` (the laser is reflected at 579; `start_dl_slippi74_fz_p12_falco3_falco2_4`, with `frozen-stages`) |
| A multi-jump fighter could not cancel a meteor by holding up | doIasa's jump test is ftCo_800CB8E0, which for can_multijump runs ftCo_800D730C(gobj, true): past the first aerial jump it takes a held stick or X/Y, with no tap and no lockout (0x800D73F4..) | `GANONDORF/Ganon vs Puff [BF] Game_CC9E0098B74E_20200311T211125` 8796 to 12864 / 12864 | `slp_bf_ganondorf_jigglypuff_t8900` (8797; `start_bf_slippi74_p14_ganondorf1_jigglypuff4_4`) |
| DownReflect (335) not ported | ftCo_DownBound_Coll (0x80097E40) tests ftCo_800C7CA0 after its floor pass: ground knockback over PlCo +1B0 into a hugged wall enters DownReflect through fn_800C7DC4 (0x800C7DC4): off the floor, the ground knockback speed (xF0) along the wall's normal times PlCo +1BC, flags 0x18040, then x placed from TransN only for Collide_RightWallHug (0x800C7F38; the other wall places y), ft_80081DD4, intangibility. Its landing (ftCo_DownReflect_Coll, 0x800C8028) is ftCo_80097D88, which keeps mv+4's top byte (the last wall), where the tumble's ftCo_80097D40 clears it | `YLINK/Falco vs YLink [PS] Game_20210125T004228` 4567 to 16314 / 16314; `YOSHI/Game_20190511T011950` 12537 to 13403 / 13403; `MARIO/Game_20191207T130511` 5428 to 6489 / 7723 (then `input_seed`) | none recorded: see below |
| The slow ledge options landed into Wait | ftCo_CliffClimb_Coll (0x8009ADA4) serves CliffClimb, CliffAttack and CliffEscape, quick and slow: an airborne landing only lands (ftCo_8009AE14). The port had that for the quick states and sent the slow ones (100% and over) through CliffCatch's landing | `JIGGLYPUFF/19_17_52 Jigglypuff + Falco (YS)` 4301 to 8629 / 8629 | `slp_ys_jigglypuff_falco_t4400` (4302; `start_ys_slippi8_p34_jigglypuff4_falco0_4`) |
| A captor threw while frozen with its victim | The throw test (ftCo_800DD1E4) is CatchWait's IASA, which Fighter_Spaghetti_8006AD10 does not run in hitlag. A light hit on the captured fighter (Young Link's returning boomerang, 0.93%) freezes both for three ticks; the port tested the stick for a throw outside the IASA and threw on the flick | `YLINK/20_16_16 [PWN] Falco + Young Link (FD)` 961 to 6080 / 6080 | `slp_fd_falco_younglink_t1100` (962; `start_fd_slippi8_p34_falco1_younglink0_4_r201616`) |

DownReflect has no retail recording yet. The three replays that reach it
are on Pokemon Stadium (a transformation's wall), from consoles that spawned
from the stage's markers 0 and 1 on ports 2/4 and 3/4 (x = -39.999996, the
marker's joint, not NeutralSpawn's -40.0): Game Start names the spawn point
per player, which `--retail-inputs` does not carry, so a boundary made from
the menus on those ports starts retail at markers 2 and 3
(`slp_ps_falco_younglink_t4700`: Falco at x = 70, dead by tick 365, and the
match over at 3345; local data, not a witness). From a NeutralSpawn
boundary retail starts at -40.0 exactly and has left the replay before the
bounce (`slp_ps_mario_yoshi_t5500`, exact for 5500 ticks with Mario
shielding at 5428; local data). What supports the port is
the retail asm and the three recordings, each entering the state once
(4567, 12537, 5428) and landing out of it on the next tick, with every
later frame of two of the games matching.

Stops that are not port faults, or are left open:

- *Screen KO (5 games): a console's lag frame.* Proven on retail with a
  display pass after every tick; see "Screen KO positions: a console's lag
  frames". (An earlier reading here rested on `--skip-display-pass` alone.)
- *`PICHU/Game_20161120T232910` (2158)*: the pad queue, above.
- *DamageFlyRoll drawn where the console drew none (2 games).*
  `PEACH/Peach vs Sheik [FoD] Game_20190820T104324` (9065, Sheik in her
  aerial needle charge, hit by Peach's back air) and
  `YOSHI/Game_20190309T173757` (9961, Yoshi in the aerial Egg Roll, hit by a
  neutral air): the port picks DamageFlyLw / DamageFlyN as recorded, then its
  ftCo_8008DCE0 roll (PlCo +23C, 30%) succeeds. In both the port's roll is
  the third value after the previous Pre Frame seed (0.123 and 0.007); the
  victim's hit-detection proc draws two values first. The recorded outcome
  needs the roll at the second value (0.413, 0.491) or from the fourth on:
  the port draws one value too many, or too few, before the roll when the
  victim is in these looping specials. Needs the windows on retail.
  Both were take-damage hooks whose draws retail makes before the roll:
  Sheik's dropped needle (533db83e) and, on 2026-10-12, the Egg Roll's shell
  burst ("The Yoshi stops" below); both games now match to their last frame.
- *`DOC/Doc vs Fox [BF] Game_20180701T000930` (8598)*: Fox's FlyReflectCeil
  under Battlefield starts at y = -25.461 recorded, -22.261 in the port (x
  and the state match). Fixed 2026-10-11 (ab0fa45b, "Link and Young Link
  stops" below).
- *`DOC/16_29_39 Dr. Mario + Jigglypuff (FD)` (5638)*: Jigglypuff, hit one
  tick into JumpAerialF1 (341) out of DamageFlyTop, leaves hitlag with
  (-2.438, +7.257) recorded and (-2.315, +7.036) in the port. Fixed
  2026-10-11 ("The Dr. Mario stops" below): the port scaled the launch by
  PlCo +190 for a row that only borrows JumpAerialF.
- *Aerial hookshot (2 games).* `YLINK/Game_20190727T200303` (2632, Young
  Link's AirCatch (361) beside Dream Land's wall: x = -98.765 recorded,
  -99.184 in the port, the tick the chain pulls him off the wall) and
  `LINK/Game_20190911T210634` (9904, Link's AirCatch, y one ulp apart).
- *`MARIO/Game_20190309T094546` (6305)*: Pikachu's grounded Thunder loop
  (360) ends (362) on frame 8 in the recording; the port's lead bolt is
  still there (LEAD_GONE not set), on a Stadium transformation.

Corpora before and after these six changes (`replay-batch`, no replay
matches fewer frames):

| Corpus | Complete | Frames matched |
|---|---|---|
| `public-v3.7` (108) | 106 to 106 | 946,234 (unchanged) |
| `public-v3.7-b` (502) | 325 to 337 | 3,780,417 to 3,864,066 |

Two games outside the triaged rows complete as well: `DK vs Falco [FD]
Game_A45C27B4D631_20200306T221823` (122 to 9855 / 9855, the delayed
powershield) and `Falco vs Ganon [PS] Game_20200405T150206` (1071 to 6932 /
6932).

## The second corpus's other RNG drifts (2026-10-03)

Twenty `public-v3.7-b` games outside the HNC consoles and without Ice
Climbers stopped on `input_seed` or `seed`. For each, the port's distance
from the recorded Pre Frame seed was measured on the drift tick and the
window played back on retail where a boundary can be made. Five causes are
fixed, each with the replay's own window as its witness
(`SLIPPI_REPLAY_WITNESSES`); the first was fixed on main in the same hours
(8778d6a2), and this wave adds its Donkey Kong witness:

| Fault | Retail | Witness | Replays |
|---|---|---|---|
| A delayed powershield (ftCo_8009388C) has no shield bubble, only the reflect volume; the port cleared that volume's cached position only while the bubble was installed, so it stayed where its first test put it | ftCo_GuardReflect_Phys (0x8009403C) calls ftColl_8007AEF8 every tick it runs | `slp_fd_donkeykong_falco_t300`: Donkey Kong, sliding out of a run, powershields at 121 and reflects Falco's laser at 122 | `DK vs Falco [FD] Game_A45C27B4D631` 122 to 9855 / 9855; `Falco vs Ganon [PS] Game_20200405T150206` 1071 to 6932 / 6932 |
| Pokémon Stadium's screen tested the slot's first fighter, which for a player on Sheik is the sleeping Zelda (suppressed: the close-up was lost at once and a new mode drawn) | grStadium_801D2344 (0x801D2344) asks Player_GetEntity | `slp_ps_zelda_sheik_t900` (the port left retail at 727) | `Zelda vs Sheik [PS] Game_20180218T194152` 1769 to 7859 / 7859; `Peach vs Zelda [PS] Game_20200212T214444` 3220 to 14406 / 14406 |
| Sheik's take-damage hook drops her charged needle, whose launch draws four values; the port created it after the proc, behind the reaction's draw | ftCommon_8007DB58 precedes ftCo_8008DCE0's draw (0x8008E124); it_802B00F4 draws at 0x802B0180..0x802B01D8 | `slp_bf_sheik_marth_needles_t1700`: the needle falls at 0.12 a tick (the port's at 0.24) | `ZELDA_SHEIK/Game_20191013T141233` 1620 to 10088 / 10088 |
| A landing's bounce spark was queued for the scene, so the thrown item's break roll took its first value | it_8026E15C: it_80278800's draws (0x80278A30..78), then it_8026DDFC's (0x8026DE58) | `slp_bf_peach_falco_t3100`: Peach's thrown Mr. Saturn bounces on at 3025 (the port's broke) | `Peach vs Falco [BF] Game_20180225T201631` 3026 to 13556 / 13556 |
| An item that bounced off a moving line kept the line's speed on every later tick | Item_802697D4 clears x58 and x64 once added (0x802698F0, 0x80269954) | `slp_ys_samus_falco_t6900`: a Bomb on Randall's cloud (`docs/PORT_NOTES/SAMUS.md`) | `Mario vs Falcon [PS] Game_20181117T184322` 5510 to 12366 / 12366 (with the fix taken out it stops at 5510 again; which item is not in the replay) |

The seed check is membership within a tick, so a stop names the tick after
the drift but not its size. Walking the stream both ways from the port's
end-of-tick seed gives it: the recorded seed N draws behind means the port
drew N more by then. A draw made on the right tick with another value (the
needle, Mr. Saturn) shows only when its consequence does.

**Open, with a cause.** `ZELDA_SHEIK/Game_20161023T221503` (3039, the port
five draws ahead; `slp_bf_falco_sheik_t3100`, local data, follows the replay
to that tick). Falco holds his Reflector's hitbox out while Sheik throws
needles three ticks apart. The hitbox strikes the first needle at 3033 and
the second at 3036; the third, created at 3036, is not struck at 3039.
Retail's hit log holds GObj pointers (lbColl_80008688): the third needle's
GObj is the first one's address (0x80D95940 in the tick trace, freed at 3033
and handed out again), so the hitbox takes it for a victim it already has.
The port names an item victim by its unique id, and the hitbox strikes
again (spark, glance roll, bounce spark). Following retail needs the GObj
allocator's free list, which every GObj kind shares.
Fixed 2026-10-11 ("Zelda and Sheik stops" below): the game matches 10883 /
10883.

**Fixed 2026-10-10.** `FALCO/Game_20190420T213132` (2336, now complete;
`slp_fod_fox_falco_t2400`). Fox techs Fountain of Dreams' wall while smoking
from Falco's Fire Bird. ftCo_800C1E64 calls efAsync_Spawn (0x800C1F88, the
flash 0x41D) after Fighter_ChangeMotionState (0x800C1F1C), whose color step
(ftCo_800C0408) has already queued the smoke (0x413); the queue is newest
first, so retail creates the flash's generator before the smoke's. The port
pushed the flash ahead of the smoke. The ceiling tech (ftCo_800C23FC) has
the same shape and the same fix, without a retail witness.

`LINK/16_45_19 [YETI] Fox + Link (FD)` (7127, now complete;
`slp_fd_fox_link_t7200`). The fault was Link's, not the Reflector's: Link
is hit out of his aerial hookshot, and it_802A2B10 removes the article with
Item_8026A8EC (0x802A2B88), whose efLib_DestroyAll (0x8026A988) leaves the
generator insertion cursor at the list's tail. The hit's electric sparks
then go in behind the head and do not emit on that tick; the port had them
emit (child generator 332, eight extra draws). Retail draws the percent
shake on 7126 as the port does.

**The Samus stops off the tether** were port faults or a lag frame, not
console events: see "Samus's wall tether and the display cadence".
`slp_fd_samus_pikachu_t4250` (two ticks per render) gated exact because
its counts agreed while its particle call sites did not (ticks 3933, 3934,
4147, 4148, 2503); `slp_fd_captainfalcon_samus_t10400` and
`slp_fod_samus_marth_t7900` are incomplete recordings of games that had
already left their replays at a tether.

**Dream Land with the 2019 spawn row** (`LINK/11_58_33 Marth + Link`,
`DK/21_45_04 Donkey Kong + Ganondorf`, `YLINK/12_28_49 Young Link +
Ganondorf`): no code text for that NeutralSpawn row is in `MELEE_GECKO_DIR`,
so retail cannot be made to follow these games and they were measured only.

## Ice Climbers: Nana after a KO (2026-10-10)

Three port faults around the climbers' deaths and revival, each the port
keeping a state's scratch where retail reads the motion id or runs a step
once (`docs/PORT_NOTES/ICECLIMBERS.md`, "Deaths"):

| Fault | Retail | Replays of `public-v3.7-b` (matched before to after) | Witness |
|---|---|---|---|
| Nana stayed on the revival platform when Popo left it with a special | ftCo_RebirthWait_IASA asks ftLib_800873CC (0x800873CC: motion 12 or 13) whether her leader still revives; the port asked the revival scratch, which a special entered from the platform keeps | `ICE_CLIMBERS/20200129 - HNC 7 ... Final Destination` 2765 to 2773 / 8067 (then `input_seed`: Popo's aerial Ice Shot) | `corpus_v3_ys_iceclimbers_fox4_e929b49af_p0` and two more explorer cases |
| Nana let go of the Belay landed inside the floor | Fighter_procMap (0x8006C2B8) counts the ECB lock on every tick, with or without a collision callback; Nana's hang (361) has none and the port counted the lock inside the callbacks | none | `corpus_v3_fd_iceclimbers_fox4_e5935012b_p1` |
| A sleeping Nana took a revival slot on every tick | ftCo_800BFD9C ends a death once: gm_80167320(slot, 1) takes one shared slot (fn_80167638, held 0x90 frames). The port repeated it while she slept, so Popo's next revival on Yoshi's Story, Dream Land or Fountain of Dreams stood at the wrong slot (x -5 for 11) | `ICE_CLIMBERS/Falco vs ICs [YS] Game_001AE99D2A67_20180307T175129` 7179 to 9325 / 9325; `ICE_CLIMBERS/20200108 - HNC 15 ... Fountain of Dreams` 4487 to 6911 / 7695 (then Nana's slide from a laser on the moving platform, `follower.cur_pos.x` 58.439 for 58.422) | none on a shared-marker stage (three dry-run sweeps on Yoshi's Story never had Nana die alone); `iceclimbers_ko_nana_alone_then_popo_soon_fd_fox4` shows the same order on Final Destination, where each player has its own marker |

The two `follower.cur_pos.x` stops one ulp apart (`ICE_CLIMBERS/Game_20191209T171524.slp`
at 200, `Game_20190903T151039.slp` at 423) are not shown to be port faults.
The first, played back on retail (`slp_ys_jigglypuff_iceclimbers_t400`,
local data, from the unregistered boundary `start_ys_jigglypuff_iceclimbers4`
made with `--gecko ucf-0.74 neutral-spawn --ports 1 2`), gates exact to
tick 400: retail in Dolphin has Nana at 0x41EC558E on that tick, as the
port does, where the console's recording has 0x41EC558D. Her CPU stick
(-76, -75) and her recorded joystick agree on all three. Not diagnosed: a
velocity difference below a position ulp can hide for many ticks, and a
replay records no velocity.

## Link and Young Link stops (2026-10-11)

Seven `public-v3.7-b` games stopped with Link or Young Link in play. Three
port faults, each fixed; two display passes already described above; two
left open.

| Fault | Retail | Replays (matched before to after) | Witness |
|---|---|---|---|
| The hookshot's chain paid a link out at the hand on every tick of hitlag | it_802A6A78 (accessory 3, hitlag) walks the chain as it_802A4BFC does, but a link the hand is within a link's length of stays in the hand (0x802A6D20: `ble` to `return 0`); it_802A4BFC and it_802A5320 bring it out at the hand. The port shared one walk, so a claw that struck a fighter left the chain with one link too many per hitlag tick: all out a tick early on the wall (state 8 for 6), and a hang of another length | `LINK/Game_20190911T210634` 9904 to 13970 / 13970; `YLINK/Game_20190727T200303` 2632 to 8673 / 8673 | `slp_fd_link_marth_t10000` (boundary `start_fd_slippi73ns_p14_link1_marth2_4_r210634`): the claw strikes Marth at 9891 and Final Destination's underside at 9897; retail keeps 18 links out through the hitlag, the port had 20 |
| A boomerang destroyed in the thrower's hand left the generator insertion cursor where it was | The take-damage hook ftLk_SpecialS_RemoveBoomerang1 calls it_802A07B4 (0x800EC094), which ends in Item_8026A8EC (0x802A07F8): efLib_DestroyAll (0x8026A988) walks the boomerang's joints before the Damage script creates the electric sparks, as for the hookshot (above) | `YLINK/12_28_49 Young Link + Ganondorf (DL)` 4880 to 6789 (then the ceiling bounce below) | `links_boomerang_hand_hit_fd_fox4`, `links_yl_boomerang_hand_hit_fd_fox4`: Fox's Reflector strikes Link on frame 12 of SpecialS1 (tick 212); the port's random stream left retail's on that tick |
| A fighter bouncing off a ceiling in a pose taller than its tumble stayed where the bounce placed it | ftCo_800C18A8 moves cur_pos after Fighter_ChangeMotionState (0x800C1A48) and calls ft_80082084 (0x800C1A4C), which never moves the root joint: mpColl_LoadECB_JObj measures the new pose's joints, still standing at the contact position, from the placed cur_pos. The ECB is offset by the placement, and a pose whose top is above the tumble's reaches the ceiling again and is pushed down out of it. The port moved the root first. The wall arm (ft_80081F2C, 0x800C1A28) has the same shape and the same fix | `YLINK/12_28_49 Young Link + Ganondorf (DL)` 6789 to 6907 / 6907 (y -55.253 recorded, -50.863 in the port: the difference is the two ECB tops', 12.905 - 8.515); `DOC/Doc vs Fox [BF] Game_20180701T000930` 8598 to 8627 / 8627 (12.680 - 9.479) | none on retail: neither game can be played back (Dream Land's 2019 spawn row; no raw stick in the other recording), and 75 stick variations of `corpus_v3_s0_ef4efb740_p1` all bounce in Marth's stretched pose (top 13.335 against 12.443), where nothing is pushed. That witness and its ceiling tech stay exact |

Not port faults, as described above: `PEACH/15_52_33 Peach + Young Link
(DL)` (8501, a star KO's display pass) and `PEACH/Game_20191013T105824`
(6022, the magnifier's damage tick).

Open, measured only:

- `LINK/Game_20190406T175018` (10399, Pokemon Stadium). The port follows
  the console's Pre Frame seeds for 10398 ticks; on tick 10398 the console
  draws 67 values and the port 3 (the stage's flash roll and two idle
  generators). 64 is eight particles of the eight-draw kind the audience
  fireworks emit (0x8039B088..0x8039B448), and the burst lasts about thirty
  ticks. Link is in frame 34 of an air dodge holding a bomb, Luigi in jump
  squat, the stage 98 ticks into the Water form's announcement. The window
  cannot be played back: retail in Dolphin
  (`slp_ps_link_luigi_t10500`, local data, boundary
  `start_ps_slippi73pre_p13_link4_luigi2_4`) leaves the console near tick
  3137, where Luigi's launch takes DamageFlyRoll for the console's
  DamageFlyLw, and its match ends at 7902. The port's cold twin follows
  that retail run exactly for all 7902 ticks, particle call sites included,
  so both streams are the port's: the Stadium's screen draws depend on
  display passes (above).
- `LINK/11_58_33 Marth + Link (DL)` (3267, the 2019 spawn row). A bomb
  that glanced Marth (1%, tick 3184) lands and goes off at 3258; the port's
  draws equal the console's for six ticks of the blast and differ from tick
  3264 (160 for 172, then 138 for 132, 179 for 171). The same blast with
  nobody struck is exact on retail (`links_bomb_up_lands_dl_marth4`), as is
  a bomb burning out in the hand (`links_bomb_held_fuse_fd_fox4`).

## The Dr. Mario stops (2026-10-11)

The five `public-v3.7-b` games that stopped with Dr. Mario in play:

| Replay (`DOC/`) | Stop | Cause | Now |
|---|---|---|---|
| `20200101 - HNC 7 - PM 0608 - Dr. Mario vs Peach - Pokemon Stadium` | 5162, unported: a special while holding an item | Dr. Mario throws Megavitamins with Peach's turnip in hand. No row of ftDr_Init_MotionStateTable reads fp->item_gobj; the item stays in hand | 9027 / 18884, then `input_seed` (below) |
| `16_29_39 Dr. Mario + Jigglypuff (FD)` | 5638, `cur_pos` | ftCo_Damage_CheckAirMotion (0x8008E498) reads fp->motion_id (0x19..0x26, 0xEC); the port matched the row's common state, so Jigglypuff's JumpAerialF1 (341) took the PlCo +190 launch scale (0.95) when the shield button had just been pressed | 16995 / 16995 |
| `Game_20191106T211939` | 4813, `cur_pos` | not a port fault: the star KO's display pass ("State-timing and position stops"); checked again, dropping the pass before tick 4814 matches 7932 / 7932 | unchanged |
| `20200219 - HNC 1 - PM 0655 - Fox vs Dr. Mario - Pokemon Stadium` | 7008, `percent` | open: the widescreen magnifier, one tick early on the console (below) | unchanged |
| `Doc vs Fox [BF] Game_20180701T000930` | 8598, `cur_pos` | FlyReflectCeil with a bounce pose taller than the tumble: fixed on main by ab0fa45b (below) | 8627 / 8627 |

**Witnesses.** `slp_ps_drmario_peach_fzw_t5600` is the HNC 7 game played
back on retail (boundary
`start_ps_slippi74_ns_frozen_wide_p12_drmario2_peach1_4`): rows 344 and 343
with the turnip held, from tick 5162. Seven directed branches of it replace
Dr. Mario's pad from tick 5207, where he stands holding the turnip
(`drmario_held_turnip_{s,airs,hi,airhi,lw,airlw,appeal}_ps_peach`): the
sheet (345, 346), the punch (347, 348), the tornado (349, 350) and the side
taunt with its own pill (341). Row 342 is never entered: ftCo_800DEAE8
picks 341 facing either way. All exact with items and particle call sites.
Mario shares the rows and has no witness; his audit stays closed.
`slp_fd_drmario_jigglypuff_t5800` (boundary
`start_fd_slippi74ns_p12_drmario2_jigglypuff3_4_r162939`) is the Jigglypuff
game: retail launches her at (-2.893, 4.132) on tick 5633, the port had
(-2.749, 3.926).

**The bridge follows fighters longer than seeds on a widescreen Stadium.**
Retail in Dolphin played the HNC 7 game's pads with the fighters where the
console had them until tick 8700, but its seed left the console's at tick
1004: the jumbotron's close-up test reads the CObj of the last display
pass, and Dolphin's passes are not the console's. The port follows both
(the gate with the trace's display clock, the replay with a pass after
every tick). So the game's next stop (`input_seed` at 9027: the port draws
76 values on tick 9026 where the console drew 64) could not be played back:
`slp_ps_drmario_peach_fzw_t9200` (local data) is exact for 9200 ticks and
is not the console's game after 8700. No single dropped display pass in
9019..9028 moves the stop.

**HNC 1, Fox vs Dr. Mario (7008).** Dr. Mario, launched off the left side,
crosses the widescreen bound slowly: his window x in the port is 101.32,
100.05, 98.45 on the passes after ticks 6947, 6948, 6949, against the
code's left bound of 100 (`x >= 100` on the truncated value, 0x80030C80).
The port has him off screen from the third and gives the magnifier's point
at 7009; the console gave it at 7008, as if the second were already
outside. Measured, none of which explains it: a left bound of 101 passes
this stop, but bounds of 101 and 539 over the 45 games read as widescreen
make 35 match fewer frames and only this one more, so the fleet ran 100 and
540; no single dropped pass in 6884..6952 moves the stop (a skipped pass
can only delay the flag). The console's projection differs from the port's
by at least 0.05 of a pixel here, from something a replay does not record.

**Doc vs Fox on Battlefield (8598).** Fixed on main the same day by the
bounce's ECB pass ("Link and Young Link stops" above, ab0fa45b): Fox's
bounce pose is taller than his tumble (ECB tops 12.680 and 9.479), so
ft_80082084 pushes him down by the difference; the game now matches 8627 /
8627. What this wave adds: a second retail recording of the arm where
nothing is pushed, `bf_marth_fox4_flyreflectceil_eb7292cdf_p2` (Marth off
the same Battlefield line at tick 3453, found by the explorer, 1 case in
480), exact before and after that change; and the reason the pushed case
has no recording. The game's bridge (`slp_bf_drmario_fox_t8628`, local
data, boundary `start_bf_slippi73ns_p13_drmario4_fox2_4`) is exact for the
8121 ticks retail played, but retail's seed leaves the console's at tick
3633 and the fighters at 4935, and Fox is dead by 8120.

That recording's Pre Frame raw X is zero for both ports on tick 2 (75 and
71 beside it), which `--retail-inputs` refuses; the bridge was made with a
scratch fallback to the nearest raw pair for that one tick (in Entry). The
tool itself is unchanged.

Corpora before and after (`replay-batch`; no replay matches fewer frames,
and every scenario recorded before gates as it did: 2335 pass, 25 fail):

| Corpus | Complete | Frames matched |
|---|---|---|
| `public-v3.7` (108) | 107 to 107 | 956,403 (unchanged) |
| `public-v3.7-b` (502) | 409 to 410 | 4,506,818 to 4,522,040 |

(Measured on f577dcce, before the Link wave's changes were merged in.)

## Ice Climbers: the thirteen stops of the fifth wave (2026-10-11)

Thirteen `public-v3.7-b` games with the Ice Climbers stopped after the wave
above. Seven port faults account for nine of them; each was found by playing
the window back on retail, except the stale table (below).

| Fault | Retail | Replays (matched before to after) | Witness |
|---|---|---|---|
| An ice block's slide effect waited for the block's link 9 flush | Item_80268E5C flushes the item's efAsync queue at every motion change (0x80268F44): a block that lands (itClimbersice_UnkMotion3_Coll) makes generator 14006 in its collision proc, ahead of the fighters' dust | `ICs vs Marth [YS] Game_20200625T131113` 131 to 8177 / 8177; also the first stop of `19_56_13 ... Falco (FD)` (1421), `Game_20190504T214222` (684) and `ICs vs Fox [PS] Game_20200222T132857` (155) | `slp_ys_iceclimbers_marth_t200` |
| A Yoshi Bomb star's first hitbox sat two units low | it_802B2FC8 calls it_802B322C after Item_80268B18 (0x802B3064), once the creation's map pass has lifted the star; the port's state change ran before it, so link 11 placed the hitbox at the unresolved position and a clank came a tick late | `20200212 - HNC 19 ... Yoshi ... Dream Land` 923 to 13516 / 13516 | `slp_dl_iceclimbers_yoshi_t1000` (retail with the replay's pads; the clank at 900) |
| A captor's knockback-free hit gave the victim hitlag and x221A_b3 | Fighter_ProcessHit's reaction branch is dmg.kb_applied alone (fighter.c:2852): Falcon Dive's catch hit adds percent (fighter.c:2958) and nothing else; the captor's hitlag holds the victim through x1A5C | `20_25_09 [BAAA] Ice Climbers + Captain Falcon (YS)` 2385 to 9551 / 9551 | `slp_ys_iceclimbers_captainfalcon_r202509_t2450` |
| A thrower stayed ungrabbable to the end of its throw | ftCo_800DD724 clears x1A6A at the release (0x800DD7B8) | `19_56_13 [BEN2] Ice Climbers + Falco (FD)` 5810 to 11012 / 11012 | `slp_fd_iceclimbers_falco_r195613_t5900` |
| The climbers' friction followed the floor's material | ft_GetGroundFrictionMultiplier returns 1.0 for Popo and Nana (0x80084A50..5C) | `Game_20190504T214222` 6058 to 13072 / 13072; `20200108 - HNC 15 ... Fountain of Dreams` 6911 to 7695 / 7695; `ICs vs Fox [PS] Game_20200222T132857` 4475 to 6719 | `slp_fod_iceclimbers_marth_r214222_t6150` |
| A revival emptied the player's stale table again | plStale_ResetStaleMoveTableForPlayer is called from the stock loss (0x800D3528) and the player's creation (0x80038940) only | `20200129 - HNC 7 ... Jigglypuff ... Final Destination` 4979 to 8067 / 8067 | none (below) |
| Nana's hitlag held a victim whose link Popo's Squall Hammer had cleared | ftNn_Init_80123954 writes nana_fp->x1A5C = NULL when she does not join (0x801239A8, 0x80123AE8); her throw goes on (victim_gobj), but her hitlag no longer reaches the victim, which the forward throw's knockback-free hit does not freeze either | `Falcon vs ICs [FD] Game_20181118T001727` 17175 to 19192 / 19192; `ICs vs Fox [PS] Game_20200222T132857` 6719 to 9326 / 9326 | `iceclimbers_nana_throw_popo_squall_fd_falco`, `..._late_fd_falco` |

The unported boundary `ftCo_800AC5A0: steering with no knockback` was
reached only through the third fault. Retail does not enter behaviour 18 on
that hit (Nana's +0x221A is 0x00 and CpuFighter x18 stays 1 through the four
frozen ticks), so the branch stays `unimplemented!`. What it would read: r30
is the caller's fighter pointer (ftCo_800B2790 keeps fp there, 0x800B27A4),
so the stick's y is the low byte of the Fighter's address; r5 is whatever
ftCo_800B2AFC's last call left.

**A bridge with the Ice Climbers needs the replay's own boundary on every
stage.** Nana's CPU state is made at creation, before the boundary seed:
from another boundary retail's Nana leaves the replay at her first hit
(`slp_ys_iceclimbers_captainfalcon_t2450`, local data: at 348 the console's
Nana steers out of a hit, stick (-89, 89), and SDIs; retail's stick is
neutral on that tick). Made with
`--game-start-seed`, the boundary follows (`..._r202509_t2450`).
`slp_dl_iceclimbers_yoshi_t1000` is from a boundary of another seed: retail,
but not that replay's game.

**Not every replay can be played back.** `slp_fd_captainfalcon_iceclimbers_r001727_t17300`
(local data) is exact against retail for the 7510 ticks retail ran, but
retail's match ended there: the port driven by the bridge's pads leaves the
port driven by the replay near tick 350 (Falcon jumps in the replay and is
grabbed in the bridge), so the pads rebuilt from Pre Frame do not act as
the console's did there. Not diagnosed. The seventh fault was
therefore witnessed with a directed scene instead. `slp_fd_iceclimbers_jigglypuff_r1030_t5050`
(local data, HNC 7) is exact too and also leaves its replay before the
stale hit at 4979, so the stale table has no retail witness: the evidence
is the two call sites in the retail asm and the recording (Nana's jab at
3734, after Popo's death at 3715 and before the revival at 3775, is the
fifth entry when her next jab does 2 x 0.95).

Left as they were:

- `19_16_18 Ice Climbers + Samus (PS)` (6698, `input_seed`): the port is
  five draws behind after tick 6697, both climbers in the grounded Ice Shot
  (341, frames 12 and 36). Not played back: on Stadium the players stand
  at (-39.999996, 31.999992) and (40.0, 32.0) on ports 1 and 4, which a
  boundary made from the menus on those ports does not give ("State-timing
  and position stops", the Stadium spawn points).
- `20_31_01 Captain Falcon + Ice Climbers (FoD)` (7022): the pad queue
  ("The pad queue is not in a replay"). Falcon's raw X runs 1, 45, 100 over
  ticks 7020..7022: 99 over two ticks, 55 over one, and the console kept the
  slow turn. The same with UCF 0.74 and 0.8 named.
- The two one-ulp `follower.cur_pos.x` stops. `Game_20191209T171524` was
  played back again from its own Game Start seed
  (`slp_ys_jigglypuff_iceclimbers_r171524_t400`, local data): retail in
  Dolphin still has 0x41EC558E where the console recorded 0x41EC558D, and
  the port gates exact. `Game_20190903T151039` (Stadium) was not played
  back.

## Zelda and Sheik stops; GObj addresses (2026-10-11)

Six games of `public-v3.7-b` stopped with Zelda or Sheik in play. Two were
port faults; four stop on a display pass (below).

**An article left by another article keeps that article's attack.**
`ZELDA_SHEIK/21_27_18 Zelda + Falco (DL)` (4666: 85.37 recorded, 85.17 in
the port). it_8027B0C4 (0x8027B0C4) gives a new item the attack id and
instance of SpawnItem.x4_parent_gobj2: the owner's current ones when that
is the fighter, the spawning article's own when it is an article
(0x8027B158). Din's Fire's explosion names the fire (it_802C4580,
0x802C4608). The port took Zelda's current attack; hit out of the end
state (whose motion change has removed the take-damage callback, so the
fire is still hers), she had none, and the explosion's hit never entered
her stale-move table. Witness `sheikzelda_zd_din_end_hurt_fd_zelda`
(`MATRIX_WITNESSES`): the next Din's Fire deals 6.37, where the port dealt
7.0. Ness's PK Fire pillar, PK Flash burst and PK Thunder segments name
their spawner the same way (from the asm; no recording separates them).
The game matches 11122 / 11122. It could not be played back on retail: it
ran the 2019 Dream Land spawn row.

**A hitbox's victim list keeps GObj addresses.**
`ZELDA_SHEIK/Game_20161023T221503` (3039). Every GObj comes from one
allocator whose free list is last in, first out (HSD_ObjAlloc 0x8037ABC8,
HSD_ObjFree 0x8037AD20), and a victim list is never cleared of a destroyed
victim, so an item created where a struck one was is not struck.
`hsd_gobj::GObjAddresses` keeps the identity those addresses give; every
GObj the match creates or destroys passes through it in retail's order:
items and their ItemLink GObjs, effect models, screen-shake models, the
stage's start GObjs and the GO banner. `ItemCore::hitbox_victim` is the
item's GObj address. The game matches 10883 / 10883;
`slp_bf_falco_sheik_t3100_cold` is in `SLIPPI_REPLAY_WITNESSES`.

Two oracles:

```sh
cd harness && uv run python gobj_ledger_record.py scenarios/<name>.toml --out <file.jsonl>
melee-sim gobj-diff harness/scenarios/<name>.toml --ledger <file.jsonl>   # every event, in order
melee-sim gobj-diff harness/scenarios/<name>.toml                         # item identities, any tick trace
```

`dolphin/gobj_ledger.py` watches `gobj_alloc_data.freehead` and records
each store with the GObj whose proc is running. A ledger kept as
`harness/traces/<name>.gobj.raw.jsonl.zst` is gated by
`gobj_addresses_match_retail_ledgers` (`GOBJ_LEDGER_WITNESSES`, each
scenario event for event; `gobj-diff --ledger <file> --show-from A
--show-to B` lists both sides with the proc each retail event ran in).
What the first fifteen ledgers corrected:

| Retail | Port before |
|---|---|
| A KO's quake, then its blast, after the article ftCo_800D331C destroyed (ft_0D31.c:220) | blast, quake, article |
| A struck fighter's article goes (take_dmg_cb) before the hit's quake | quake first |
| A wall bounce's quake precedes its queued flash (ftCo_800C7CA0) | flash first |
| A throw's release hit lands in the thrower's animation proc: the quake's model is stepped by that tick's quake procs (s_link 1, p_link 18) and lasts 30 ticks, not 31 | requested at the victim's next proc, a tick late |
| Common models whose efAsync row does not set EF_STATE_ASYNC wait out their owner's hitlag (efasync.c:105-527) | only some did |
| Shield Breaker installs the effect hitlag callbacks (ftMs_SpecialN_801365A8) | not installed |
| Spent models are freed p_link 11 first, then the KO models' p_link 12 | creation order |
| A chain's links are GObjs: created after the article, freed newest first before it | not modelled |
| Peach's pulled item, then the pull's model (spawnVeg 0x8011D018) | model first |

A HUD model rebuilt in place (a player's icons at a KO or return: four
consecutive events on one address) leaves the list as it was and is not
modelled; `gobj-diff` skips it.

**The whole recorded set (2026-10-12).** Item identities differed from the
tick trace somewhere in 100 of the 2713 scenario files with a raw trace (the 55
of the day before, the scenarios recorded since, and each bridge's `_cold`
twin). A ledger of every one (`gobj_ledger_record.py`; a bridge needs
`MELEE_GECKO_DIR`) put them in the groups below, each one order:

| Retail | Port before | Scenarios | Ledger (tick) |
|---|---|---|---|
| A death frees the articles and the held item (the callbacks and Item_8026A8EC of ftCo_800D331C, ft_0D31.c:99-125), then the fighter's own models (ftCommon_8007DB24, :132), then the quake and the blast | the fighter's models first | Fox with the Blaster out | `corpus_v2_s0_e49_p0` (2544) |
| A hit's take-damage callback (ftCommon_8007DB58) frees the article before the damage entry's floor-bounce model (efAsync 0x406), Fighter_ChangeMotionState's efLib_DestroyAll and the quake | the article last | Link, Young Link, Luigi, Marth vs Peach, Mewtwo | `corpus_v3_fd_luigi_fox4_ef9e858d4_p0` (1338), `corpus_v3_fd_mewtwo_fox4_ece1e43b7_p0` (1599) |
| Items go in the order Item_8026A8EC is called: the arrow, then the bow (ftLk_800EAF58), each one's models just before it | pool (creation) order, every model before every item | Link | `corpus_v3_fd_link_fox4_ef17f3433_p1` (355) |
| DownBound's bounce model (0x406, ftCo_800978D4) is queued behind the new script's frame-0 commands; the flush takes the newest first, so the model precedes the quake of Roy's script | the quake first | Roy (9) | `corpus_v3_fd_roy_fox4_e4f8edfa8_p2` (1777) |
| A bursting egg (it_802B2C38) in a hit's callback: the script's quake at once, then twelve shell models whose parent is the egg (efSync 0x4CF, efLib_Create_Attach_Pos), freed with it | the quake a tick late; the fragments lived their own fifty ticks | Yoshi | `corpus_v3_bf_yoshi_pikachu4_ecad2716f_p1` (352, 380) |
| The hit's quake (ftCo_8008DCE0), then Bury's mound | the mound first | Donkey Kong's Headbutt | `corpus_v3_fd_donkeykong_fox4_ee98155f3_p0` (1583) |
| The ice block (efAsync 0x415) is created in the hit's proc, after its quake | at the next tick's link 9 flush | Ice Climbers | `iceclimbers_nana_throw_popo_squall_fd_falco` (1957, 1960) |
| Battlefield's incoming background is a new GObj (grBattle_80219D84); the outgoing one is freed when its fade ends | not modelled | every Battlefield game past the first swap | `slp_bf_marth_peach_t6500` (3268, 3468) |
| Each map of a Stadium form is a GObj (grStadium_801D10F8, Ground_801C4A08) | not modelled | every transforming Stadium game | `corpus_v3_ps_fox_marth4_e75fb4a9a_p1` (4287, 4408) |
| Dream Land's flyby is one or three GObjs (grOldPupupu_80210D10), each freed when its 3999-frame animation ends (grOldPupupu_80210C34) | only its random draws | every Dream Land game past the first flyby | `slp_dl_peach_younglink_r1_t8700` (3196, 7196) |
| The end banner is a HUD GObj created before the next tick's procs; in a timed match a HUD GObj of the timer's is freed first and the banner takes its address | not modelled | a match played to its end | `corpus_v2_s0_e49_p0` (3677), `slp_bf_drmario_fox_t8628` (8007) |

**Where an article is freed.** The port flushes a fighter's effect requests
when its proc returns and frees the articles the proc removed after that;
retail frees them inside the proc. Moving the frees would move every
particle teardown with them, so only the addresses move:
`EffectRequest::ArticlesFreed` marks the point of the fighter's queue where
retail had called Item_8026A8EC, the flush marks the allocator there
(`GObjAddresses::mark`), and after the proc the scene rewinds to the mark,
frees the articles and replays what the flush did since (`rewind`,
`replay`); an object replayed at another address is renamed in its holder
(`Effects::rename_gobjs`, `ItemPool::rename_gobjs`, `Quakes::rename_gobjs`).
A death and every take-damage callback set the mark; a callback that tears
the fighter's own effects down after its article (Mewtwo's) sets it itself.

After these changes 2 of the 2790 files now recorded differ: `slp_ps_fox_drmario_wide9263_r1_t7200`
and its twin, a recording that stops matching at tick 1671 on any build (it
does not gate; the port keeps an item retail's trace no longer has).
Thirteen of the ledgers are in `GOBJ_LEDGER_WITNESSES` (28 now). Not found
in any ledger, so still unverified: the states that call
Fighter_SetEffectHitlagCallbacks and the port does not (about fifty), Whispy's
wind (the loop quake already matched), and the link GObjs of Sheik's
chain, the Grapple Beam and the Belay rope.

**Article removers and the generator cursor (audit, from the C; not
verified on retail).** Item_8026A8EC inside a fighter's proc also leaves the
generator insertion cursor on the list's tail (`EffectRequest::
ArticleDestroyed`). The port queues that for Mr. Game & Watch's
ftGw_Init_OnDamage, his aerials' landings and ftGw_Init_8014A538, Link's
boomerang and hookshot and Samus's Grapple Beam. It does not on these
paths, where the fighter's code goes on in the same proc:
- Mr. Game & Watch: ftGw_SpecialAirHi_Coll (0x8014E3D4) removes the
  trampoline (it_802C8158) before ftCo_LandingFallSpecial_Enter; his only
  uncovered path.
- take-damage callbacks, followed by the damage entry (the hit's overlay,
  the Damage script's commands): Fox's and Falco's Blaster (it_802AEAB4),
  Peach's parasol, Toad and turnip, Sheik's chain (it_802BB20C), Ness's
  yo-yo and bat, Link's arrow and bow when nothing else is out, the cape,
  Yoshi's egg (it_802B2890), the Ice Climbers' ice block and rope, the
  Megavitamin, the milk, Mewtwo's Disable and a full Shadow Ball;
- the Ice Climbers' ftPp_SpecialAirN_Coll (the ice block, then
  ftCo_LandingFallSpecial_Enter), Ness's ftNs_AttackS4_IASA (the bat, then
  ftCo_Wait_IASA), Mewtwo's four Loop IASA callbacks, Peach's ft_8008A348.
A death is covered: its teardown walks the fighter's joints after the
articles. No recording shows any of these as a difference in particle
order (it shows only when the cursor is not already the tail), so none was
changed.

Checked on main at 6064692d, before and after: of 2790 scenario files 2689
gate exact on both builds and no result changes; `gobj-diff` differs on 103
before and 2 after, none newly; `replay-batch` matches the same frames in
every replay (`public-v3.7` 108 complete, 964,782 frames; `public-v3.7-b`
475 complete, 5,005,881 frames).

### The throw-release quake and three consoles

The throw-release timing is its own commit. It moves the camera's shake a
tick earlier after a throw, which three recordings see:

| Replay | Without it | With it |
|---|---|---|
| `PEACH/Game_20191013T105824` (FoD, standard screen) | 6022, `percent`, the port a point above (listed above as a skipped display pass) | 7298 / 7298 |
| `DOC/20200101 - HNC 7 - PM 0608 - Dr. Mario vs Peach - Pokemon Stadium` (widescreen) | 9027, `input_seed` | 16853, `input_seed` |
| `PEACH/20200219 - HNC 6 - PM 1113 - Marth vs WOO Peach - Pokemon Stadium` (widescreen) | 5251, `input_seed` | 4456, `percent`, the port a point below |

The third matches fewer frames. Its window was played back on retail with
a render after every tick (`slp_ps_marth_peach_wide_r1_t4600`, local data;
boundary `start_ps_slippi74_ns_frozen_wide_p12_marth0_peach0_4`: UCF 0.74,
NeutralSpawn, preload, Frozen Stages, Widescreen), with a GObj ledger:

- Peach's throw shakes at 4372 (its script's quake, freed at 4403, 31
  ticks) and releases at 4375 (the release quake, freed at 4405, 30
  ticks). The port with the timing equals all 716 ledger events; without
  it the release quake is freed at 4406.
- Marth leaves the screen and the magnifier's point comes at trace tick
  4458 on retail. The port with the timing has it at 4458 and equals
  retail on every fighter key of all 4600 ticks; main and the port without
  the timing have it at 4457, their one differing fighter key.
- The console's recording has the point at trace tick 4457 (replay frame
  4456): a tick before retail in Dolphin with the same codes. No single
  skipped display pass from 4370 to 4457 moves the port's point there
  (`--skip-display-pass`), and a lag frame cannot bring a point forward.
  One pixel of the widescreen right bound moves it two ticks (539: 4455),
  so the shake's phase decides it.

So main agreed with that console by being a tick late in the shake; what
the console ran differently is not in the replay. The recording did not
gate then: Mr. Saturn's position left retail at 3710 (its release box, see
the HNC 6 row under "Pokemon Stadium bridges"; fixed since), with the seed 17
ticks apart and equal again from 3747.

### Display-pass stops under the every-tick rule

None of these has an every-tick retail recording, so none is confirmed as
"not a port fault"; each is consistent with one lag frame
(`--skip-display-pass`, on this branch):

| Replay | Stop | One skipped pass |
|---|---|---|
| `PIKACHU/Game_20190629T164554` | 4576, star KO position | at 4576: 7668 / 7668 |
| `ZELDA_SHEIK/19_56_47 Sheik + Captain Falcon (DL)` | 3073, magnifier | at 3013 or 3014: 15970 / 15970 |
| `ZELDA_SHEIK/Marth vs Sheik [FD] Game_20181024T232813` | 6345, magnifier | at 6288: 13618 / 13618 |
| `ZELDA_SHEIK/Game_20120109T050633` | 2013, magnifier | at 1996: 10793 / 10941, then `percent` again |

Corpora before and after, on main at e251d486 (`replay-batch`); every
scenario recorded before gates as it did (2433 pass before, 2436 after, of
2467):

| Corpus | Complete | Frames matched |
|---|---|---|
| `public-v3.7` (108) | 107 to 107 | 956,403 (unchanged) |
| `public-v3.7-b` (502), without the throw-release commit | 431 to 433 | 4,653,593 to 4,667,893; no replay fewer |
| `public-v3.7-b` (502), with it | 431 to 434 | 4,653,593 to 4,676,200; one replay fewer (above) |

## Screen KO positions: a console's lag frames (2026-10-11, proven on retail)

Five games of `public-v3.7-b` stop on the `cur_pos` of a fighter in the
screen KO (DeadUpFall, 6, or DeadUpFallHitCamera, 7), and a sixth on
`motion_id` there. The five are console lag frames, shown below on retail
with a display pass after every tick; the sixth was a port fault. No stop is
in the star KO (DeadUpStar), which reads nothing a display pass writes: its
flight aims at Stage_GetCamBoundsTopOffset, read in the animation proc when
the hold ends (ftCo_DeadUpStar_Anim, 0x800D42E4).

**What the screen KO reads, and when.** ftCo_800D4580 sets x2220_b7 and
keeps the fighter's camera-space position at fp+0x2350; the physics proc
moves that vector (ftCo_DeadUpFall_Phys: lbVector_Lerp at 0x800D4D60 while
it approaches the screen, lbVector_Add at 0x800D4DA8 while it slides down).
`cur_pos` is written from it only in the display pass:

- the camera GObj's render callback fn_800301D0 calls Camera_8002A4AC
  (0x800301FC), which sets cm_804D6464's eye and interest from
  `transform_copy` (Camera_8002AF68);
- the fighter's render callback ftDrawCommon_80080E18, in the pass that has
  game_camera +0x398's low bits at 1 (Camera_80031060; Camera_800311EC's
  first draw), tests x2220_b7 (0x80080E6C) and calls Camera_800310B8
  (0x80080E80: HSD_CObjSetupViewingMtx on cm_804D6464),
  HSD_CObjGetInvViewingMtxPtr (0x80080E84) and
  `PSMTXMultVec(inverse view, fp+0x2350, &fp->cur_pos)` (0x80080E90), then
  copies `cur_pos` into the root joint's translation (0x80080ECC).

So the position a tick ends with is the previous display pass's placement,
moved by Fighter_procUpdate's ordinary integration of `self_vel`: zero
through the hold, the approach and the impact, the fall speed
(PlCo +0x550, gravity +0x554 to +0x558) while it slides. A tick that no
display pass precedes keeps that integrated value: x holds bit for bit, y
holds or drops by the fall speed, and the next pass places the fighter two
steps on. That is retail's code, not an emulation artifact, and the port
does the same (`render_cameras`, `Fighter::place_screen_ko`).

**The recording pairs.** gm_801A4D34 runs one tick per queued pad poll
(lb_80019894 reads qcount once per pass of its loop, 0x800198B0) and then
renders once: a frame that overruns its retrace leaves two polls queued, and
the next pass runs two ticks before one render. The recorder plays that
with `no_display_pass_before = [N]` beside `render_every_tick`
(`slippi_to_scenario.py --no-display-pass-before N`; `lag_frame` in
`harness/dolphin/tick_trace.py`): records N-1 and N run in one frame and
psFrameNum does not move at N. Each bridgeable game was recorded twice from
its own boundary, with the replay's pads and seed:

| Game (first stop, runner frame) | Every tick (`_r1_`) | One lag frame (`_lagN_`) |
|---|---|---|
| `PIKACHU/Game_20190629T164554` (4576, Sheik in DeadUpFall's hold); `start_fod_slippi73ns_p24_sheik3_pikachu2_4_r164554` | `slp_fod_sheik_pikachu_r1_t4700`: retail places Sheik at record 4577 (18.393, 128.844) where the console recorded the held (18.888, 130.005); 122 fighter records differ from there to 4699 | `slp_fod_sheik_pikachu_lag4577_t4700`: both fighters' recorded positions and actions equal the console's on every record from 4527 to 4699 |
| `FOX/Fox vs Marth [YS] Game_20200115T192755` (5873, Marth in DeadUpFallHitCamera); `start_ys_slippi74ns_p14_fox0_marth1_4` | `slp_ys_fox_marth_r1_t6000`: (11.269, 44.157) against the console's held (10.180, 48.796) at 5874; 22 records differ to 5999 | `slp_ys_fox_marth_lag5874_t6000`: 0 differ, 5800 to 5999 |
| `YOSHI/17_50_44 Falco + Yoshi (FD)` (9868, Yoshi sliding down); `start_fd_slippi73ns_p34_falco1_yoshi1_4_r175044` | `slp_fd_falco_yoshi_r1_t10000`: (-9.612, 11.072) against the console's (-10.915, 11.433), x held and y down by the fall speed, at 9869; 36 records differ to 9999 | `slp_fd_falco_yoshi_lag9869_t10000`: 0 differ, 9800 to 9999 |

A retail record R is runner frame R - 1 (`--skip-display-pass 4576` is
`no_display_pass_before = [4577]`). The port gates all six recordings exact
over their full length, with 0 differing particle ticks, so it equals retail
both with a pass after every tick and across the lag frame; the positions
after a lag frame stay off the every-tick game's by a few hundredths to the
end of the KO (which is why the console's later frames need it too), in
retail and in the port alike. The six `_r1_` and `_lagN_` scenarios are in
`SLIPPI_REPLAY_WITNESSES`. `slp_fod_sheik_pikachu_lag4576_t4700`
(the lag frame one record early, which the port also gates exact) holds the
wrong record and matches the console no better than the every-tick game.

Two games cannot be bridged (their consoles spawned by the 2019 rules, for
which there is no Gecko text): `DOC/Game_20191106T211939` (4813) and
`PEACH/15_52_33 Peach + Young Link (DL)` (8501). Their recorded frames carry
the same signature (x held bit for bit in DeadUpFallHitCamera, y down 0.2
and 1.7) and `replay --skip-display-pass` at the stop carries the first to
its last frame (7932 / 7932) and the second to 11591 (an `input_seed` stop
with Peach hit out of her float, not of this class). By the rule that only
an every-tick recording settles it, these two rest on the three above, not
on a recording of their own.

**What the runner could do (proposed here; implemented since, "Lag frames
read from a replay" below).** A lag frame is not
in a replay, but in a screen KO it leaves an exact trace: the recorded
`cur_pos` equals the previous frame's x bit for bit, with y equal or lower
by the tick's integrated fall, while the port's display pass has moved it.
The runner could keep a clone of the match while a recorded leader is in
actions 6 to 10, and on a `cur_pos` stop of that fighter rerun the tick with
no display pass before it; if the whole frame then matches (every field and
the seed check), it goes on from there and the report lists the frame as an
inferred lag frame, as it lists the UCF version it read from the frames. It
should not skip a pass on any other evidence: outside a screen KO (and the
magnifier and a tether) a lag frame leaves nothing in the recorded fields
to confirm it. With `--skip-display-pass` named by hand, four of the five
games already complete.

**Mr. Game & Watch hit the screen in the wrong state.** ftCo_DeadUpFall_Anim
compares x34_scale.z with 1.0 (0x800D4BC8) and enters DeadUpFallHitCameraFlat
(8) for a flat fighter (0x800D4BD8); ftCo_800D4580 and ftCo_800D481C face the
root to the screen only when the scale is 1.0 (0x800D4650, 0x800D489C). The
port entered motion 7 and turned the root for everyone.
`GAMEANDWATCH/Game_20190629T112449`: 6971 to 8209 / 8209. There is no retail
recording of a flat fighter's screen KO: none of the 96 recorded Mr. Game &
Watch scenarios holds one, and this game's bridge
(`slp_ps_gameandwatch_falco_r1_t7100`, from
`start_ps_slippi73pre_p12_gameandwatch3_falco2_4`; the port gates it exact
for 7100 ticks) is no longer the console's game by the KO: on Pokemon
Stadium's first transformation its positions leave the replay's (record
4139, a floor four ticks out of step) and its random stream follows by 4339.
Why a Stadium bridge leaves its replay there is not diagnosed.

Corpora before and after, on main at c82bba7f (`replay-batch`; no replay
matches fewer frames, and only the Mr. Game & Watch game matches more: the
five lag-frame stops stay, since the runner renders every tick):

| Corpus | Complete | Frames matched |
|---|---|---|
| `public-v3.7` (108) | 107 to 107 | 956,403 (unchanged) |
| `public-v3.7-b` (502) | 434 to 435 | 4,676,200 to 4,677,438 |

Of the 2529 scenario files, 2474 gate exact with the change; the other 55
(missing or shorter traces, other work in flight) give the same result on
the build before it.

## The magnifier's damage tick (2026-10-11)

Every replay stop where a fighter's `percent` is one point from the
console's at an off-screen damage tick, taken again under the every-tick
rule: each window was played back on retail in Dolphin with the game's own
codes and a display pass after every tick (`render_every_tick`). This
replaces what "Display passes are not in a replay", "Still open on the
widescreen consoles", the HNC 1 paragraph of "The Dr. Mario stops" and
"Display-pass stops under the every-tick rule" say about these games.

**Order of operations**, from the asm and the raw tick traces (each record
carries the fighter struct: the off-screen bit at +0x221F bit 7, the count
at +0x1910, the window position of the last pass at +0x2188):

1. *The tick.* The quake's proc (s_link 1) writes this tick's shake offset;
   Fighter_8006A360 (the fighter's first proc, fighter.c:898) runs the
   count at 0x8006A830..0x8006A8F8: not for a player's second fighter
   (x221F_b4, 0x8006A830), only with the camera unzoomed (Camera_80031144
   == 1.0, 0x8006A83C) and below PlCo +0x7B0 percent (0x8006A874); when
   ifMagnify_802FC998(player) (0x8006A880) and Player_GetMoreFlagsBit3
   (0x8006A890) hold, x1910 is incremented (0x8006A89C), otherwise zeroed
   (0x8006A8AC); at PlCo +0x7AC it deals PlCo +0x7B4 and is zeroed. The
   camera's own proc then moves the camera. Nothing here reads a display
   pass's result except through ifMagnify's flag.
2. *The display pass* (HSD_GObj_80390FC0, by GX link priority). The
   magnifier's camera is first (priority 0, ifmagnify.c:528):
   ifMagnify_802FBBDC clears every player's `is_offscreen` and sets it for
   a player whose current fighter (Player_GetEntity, 0x802FBD1C) has the
   off-screen bit (ftLib_80086B64, 0x802FBD3C) and is visible
   (ftLib_80086ED0, 0x802FBD4C). The main camera is next (priority 2,
   camera.c:4110): fn_800301D0 loads the CObj from this tick's camera and
   shake (Camera_8002A4AC, 0x800301FC) and draws the fighters, whose
   callback ftDrawCommon_80080E18 skips a sleeping fighter (x221F_b3,
   0x80080E34) and otherwise calls ftLib_80086A8C (0x80080E44): it projects
   the camera bone with a fresh look-at of that CObj
   (lbVector_WorldToScreen), truncates (`fctiwz`, 0x80030C48) and compares
   with the scissor (0x80030C7C..0x80030CA8), setting x221F_b0.
3. So the bit a pass sets is seen by the magnifier on the next pass and
   counted on the tick after that. With a pass after every tick: bit set
   in the record of tick T, count 1 at T + 1, the point of damage at
   T + 60. A pass that is skipped delays the bit; nothing can bring a point
   forward.
4. The Widescreen code changes two inputs of the test and nothing of its
   place in the frame: the CObj's aspect (0x8036A4A8) and the bounds
   (0x80030C7C, 0x80030C88).

**The games.** Trace ticks; a replay frame N is trace tick N + 1.

| Replay | Stop before | Retail, every tick | Cause | Now |
|---|---|---|---|---|
| `ZELDA_SHEIK/Game_20120109T050633` (BF) | 2013, the port a point above | `slp_bf_sheik_peach_r1_t2200`: no point at 2014; the port left retail there | port fault: the sleeping form's off-screen bit (below) | 10941 / 10941 |
| `ZELDA_SHEIK/Marth vs Sheik [FD] Game_20181024T232813` | 6345, a point above | `slp_fd_marth_sheik_r1_t6500`: the point at 6349, not 6346 | the same port fault | 13618 / 13618 |
| `PEACH/20200219 - HNC 6 ... Marth vs WOO Peach - Pokemon Stadium` | 4456, a point below | `slp_ps_marth_peach_wide9263_r1_t4600` (local data): the point at 4457, the console's tick; 4458 with the 320:219 code | the console's Widescreen code had the 92:63 aspect (below) | 5251, `input_seed` |
| `DOC/20200219 - HNC 1 ... Fox vs ME Dr. Mario - Pokemon Stadium` | 7008, a point below | `slp_ps_fox_drmario_wide9263_nopause_r1_t7200`: the point at 7009, the console's tick | 92:63 | 9684, `input_seed` |
| `DK/20200115 - HNC 19 ... Donkey Kong vs Donkey Kong - Pokemon Stadium` | 7930, `percent` | not recorded | 92:63 (the 320:219 run leaves the recording at 4262, the standard one at 7930) | 12548 / 12548 |
| `MARTH/20200115 - HNC 21 ... Marth vs Jigglypuff - Fountain of Dreams` | 5267, a point below | not recorded | 92:63 | 5394 / 5394 |
| `FALCO/02_33_48 Falco + Marth (PS)` (first corpus) | 3042, a point above | `slp_ps_falco_marth_frz_r1_t3200`: bit at 2983, point at 3043, as the port | not a port fault: the console's point is a tick later than retail's | unchanged |
| `ZELDA_SHEIK/19_56_47 [TREW] Sheik + Captain Falcon (DL)` | 3073, a point above | `slp_dl_sheik_captainfalcon_r1_t3300`: bit at 3014, point at 3074, as the port | not a port fault, as above | unchanged |

`FOX/Game_20190824T124750` (1482) is a `percent` stop of another kind (a
hit's damage, 28.58 against 28.61) and is not in this list.

**A sleeping fighter keeps its off-screen bit.** The port projected every
fighter on every pass. Retail does not draw a transforming character's
sleeping form, so its x221F_b0 is whatever its last pass awake left (clear,
if it has never been off screen). A transformation off screen copies the
count to the waking form (ftCommon_8007EFC8, ftcommon.c:1309), the next
pass's magnifier asks the waking form and finds the bit clear, and the next
tick zeroes the count; the pass after sets the bit again. In
`slp_bf_sheik_peach_r1_t2200` Sheik's count is 42 at tick 1996 when she
transforms: Zelda's reads 42, 0, 1, 2 over ticks 1996..1999 where the port
went on to sixty at 2014. The form that goes to sleep keeps its bit set
(its struct shows `off` and the old window position to the end of the
trace), so a later transformation back on screen counts one tick before
the next pass clears it. Both recordings were classed as a console's lag
frame by `--skip-display-pass` alone; both consoles were right.

**The Widescreen code's aspect was 92:63 on the 2020 consoles.**
`External/Widescreen/Overwrite CObj Values.asm` multiplied the
description's aspect by 92 / 63 from its first commit (703f953, 2019-01-17)
until ef8e734 (2019-10-29, "update fov to 320:219"); the bounds of 100 and
540 came five days later (e9457ee). The HNC consoles, which loaded their
own code list, ran the older float with the bounds: no tree of
slippi-ssbm-asm holds that pair (every commit from 2019 to mid-2021 was
searched as text for `42B80000 427C0000` beside `04030C7C 38000064`), so the text
is the consoles' own; `widescreen-92-63` in `MELEE_GECKO_DIR` is
`widescreen.txt` with those two floats (`harness/gecko.py`). The aspect is
0x3FE38B73 against 0x3FE3AE24: the picture is wider by 0.06%, 0.13 of a
pixel at the bounds.

What shows it:

- HNC 6. In the port at 320:219 Marth's window x on the passes after ticks
  4396..4398 is 539.24, 539.93, 541.48 against the right bound of 540; the
  console's point needs the second to be outside. Retail in Dolphin with
  the 92:63 text (boundary
  `start_ps_slippi74_ns_frozen_wide9263_p12_marth0_peach0_4`, camera dump
  recorded): the main CObj's aspect reads 0x3FE38B73, Marth's
  window x reads 539, 540, 541 with the off-screen bit at 4397, and the
  point comes at 4457. The port equals that recording on every fighter key
  of its 4600 ticks (the gate itself stopped at 3710 on Mr. Saturn's release
  box, since fixed: it now passes).
- HNC 1. Dr. Mario's x at 320:219 is 101.32, 100.05, 98.45; with 92:63
  retail reads 101, 99 with the bit at tick 6949 and the point at 7009
  (`slp_ps_fox_drmario_wide9263_nopause_r1_t7200`, exact for 7200 ticks
  with 0 differing particle ticks; `SLIPPI_REPLAY_WITNESSES`).
- No widescreen game of either corpus matches fewer frames at 92:63 than at
  320:219 (below), and six match more: the four above and two Stadium games that stopped on `input_seed`, where the jumbotron's close-up test (grStadium_801D32D0) reads the same aspect (`DOC/20200101 - HNC 7` 16853 to 18884 / 18884, `PIKACHU/20200219 - HNC 11` 10437 to 13554 / 13554).

So the one-tick question of HNC 6 has a code for an answer, not a console:
a lag frame was ruled out because it can only delay a point, a different
bound because 539 moves the point two ticks, and neither the preload nor
the frozen-stage code touches the camera. The earlier measurement that
"bounds of 101 and 539 make 35 games match fewer frames" stands: the
consoles' bounds were 100 and 540 and their picture a fraction wider.

`replay_screen_codes::resolve_screen` now reads the screen among three
candidates run in step (standard, 320:219, 92:63;
`replay_stage_codes::recorded_among`): a run that stops matching the
recording on a tick another still matches is left behind, and runs never
told apart are taken for the first in that order. The report names each
rejected screen and its tick; `--screen widescreen-92-63` names the code
instead. `--retail-inputs` writes `widescreen_92_63` in its header and a
boundary made with `--gecko widescreen-92-63` sets the flag.

**The two stops that stay.** `FALCO/02_33_48` and `ZELDA_SHEIK/19_56_47`
have a fighter launched off the right side on the standard screen (window
x 637 then 658; 635 then 643). Retail with a pass after every tick gives
the point where the port does, and both scenarios gate exact with 0
differing particle ticks (`SLIPPI_REPLAY_WITNESSES`); each console's point
is one tick later. One display pass skipped before the crossing
(`--skip-display-pass 2982`, `3013`) makes each whole replay match (11421 /
11421 and 15970 / 15970 on this branch). That is what is established:
retail equals the port, and one lag frame is sufficient. That those
consoles dropped a frame there is not shown by anything recorded. (Since
recorded on retail with that lag frame, which gives the console's game,
and inferred by the runner: "Lag frames read from a replay" below. Captain
Falcon's point is not a tick later on the console: it never comes.)

**Tooling found on the way.**

- `slippi_to_scenario.py` no longer feeds a recorded Start press to retail.
  HNC 1's Dr. Mario presses Start at tick 1671; the console had Pause off
  and retail in Dolphin paused for the rest of the recording
  (`slp_ps_fox_drmario_wide9263_r1_t7200`, local data, is that paused run).
- `boundary_script.py` saved a Sheik start on Final Destination before the
  sleeping Zelda existed: the Game Start seed plus eight draws, where
  creation makes ten. It now waits for one sleeping fighter per Zelda or
  Sheik player (`start_fd_slippi73_ns_p24_marth0_sheik1_4_r232813b`; the
  boundary without the `b` is the early save).
- `neutral-spawn-2019` (`harness/gecko.py`): the 2020 NeutralSpawn text with
  the Dream Land row of late 2019. Retail places the fighters where
  `NeutralTable::V2019` does (`start_dl_slippi73_ns2019_p14_sheik1_captainfalcon4_4`
  gates exact), so the Dream Land games of that row can be bridged.
- The earlier Stadium bridge of `FALCO/02_33_48` (`slp_ps_falco_marth_ns_t3200`)
  ran without the console's Frozen Stadium code; the new boundary
  `start_ps_slippi73prefrz_ns_p14_falco3_marth4_4` carries it.

Corpora before and after (`replay-batch`, main at c82bba7f; no replay
matches fewer frames; every recorded scenario gates as before, 2249 pass of 2281 before and 2254 after, the five more being scenarios recorded here):

| Corpus | Complete | Frames matched |
|---|---|---|
| `public-v3.7` (108) | 107 to 107 | 956,403 (unchanged) |
| `public-v3.7-b` (502) | 434 to 440 | 4,676,200 to 4,705,765 |

## Lag frames read from a replay (2026-10-11)

A console's lag frame runs two ticks before one display pass ("Screen KO
positions" above), and the display pass writes game state. A replay does not
record display passes. The runner now infers a lag frame from the frames,
under the rule the user approved, and only under it
(`crates/melee-sim/src/replay_lag_frames.rs`):

1. The comparison runs with a display pass before every tick, as before.
2. On a stop of one of the three kinds below, the runner takes the named
   tick again with no display pass before it (the same
   `Simulation::skip_display_pass` that `--skip-display-pass` calls; nothing
   is added to the simulation) from a copy of the match kept a few ticks
   back.
3. It accepts the lag frame only if every frame from that tick through the
   stop then matches the recording completely, Pre Frame seeds included,
   **and** every frame of the kind's margin after the stop does too. A
   candidate that merely moves the stop later is rejected: the run goes back
   to what it was and the stop is reported exactly as without inference.
4. Each accepted lag frame is a line of the report (tick, kind, the field
   and tick of the stop it explains, how many frames confirmed it) and an
   entry of the JSONL's `lag_frames` list. `--no-lag-frames` (`replay` and
   `replay-batch`) turns inference off; `lag_frames` is then `null`, and the
   rest of the JSONL is byte for byte what it was before this change.

| Kind | Stop it explains | Ticks tried | Margin that must match |
|---|---|---|---|
| `screen-ko` | every differing field is the `cur_pos` of a fighter recorded in actions 6 to 10 (ftDrawCommon_80080E18 places it, 0x80080E90) | the stopping tick | the rest of that KO and the frame the fighter leaves it on; a further stop inside it must be a lag frame of the same KO by the same rule (at most four) or the first is rejected |
| `magnifier` | every differing field is a `percent` | stop - 60 and stop - 59: the pass that sets the off-screen bit (ftLib_80086A8C from 0x80080E44) and the next one, which reads it (ifMagnify_802FBBDC, 0x802FBD3C); either one skipped starts the count (Fighter_8006A360, 0x8006A89C; PlCo +0x7AC = 60) a tick late | 120 frames: the delayed point's tick and, while the fighter stays out, the next one |
| `particles` | every differing field is a Pre Frame `input_seed` | the stop and the eight ticks before it, nearest first (the seed check is membership in a tick's draws, so a stream off by a few draws passes busy ticks before a quiet one stops it: four ticks in the Samus game) | 600 frames, each with its seed check |

Any other stop (a motion, a position outside a screen KO, facing, an
unported boundary) is never retried. The rewindable run is a second
simulation of the comparison's setup, started only when a comparison stops
on one of these kinds; it copies the simulation every 16 ticks and keeps 80
ticks of copies. Games that complete without a stop cost nothing more.

**Why the margins.** A screen KO's positions stay a few hundredths off the
every-tick game's to the end of the KO, so the whole KO is the evidence. A
magnifier lag frame's only trace is the count's phase, which the next point
shows again. A seed stop is the weakest signal (membership, not equality),
so its margin is the longest: in `SAMUS/Game_20190629T124717`, skipping the
pass before 7811 instead of 7809 passes the stop and matches eight more
ticks; only 7809 matches on (to the end of the game). Without the margin
rule that candidate would have been accepted.

**What this cannot tell apart.** A port fault that has the same effect as
one skipped pass (the off-screen bit a tick early; a particle order the
display pass should not have changed) passes the rule as a lag frame would.
That has happened: two Zelda games matched completely with one hand-named
skipped pass and were port faults ("A sleeping fighter keeps its off-screen
bit" above). So an inferred lag frame of a mechanism is trusted because
retail in Dolphin, recorded with a pass after every tick, equals the port at
that stop (the port is right without the lag frame) and, recorded with the
lag frame, equals the console. The table says which games have that.

**Results** (`replay-batch`, main at d1f39d6f; before / with `--no-lag-frames`
/ with inference):

| Corpus | Complete | Frames matched |
|---|---|---|
| `public-v3.7` (108) | 107 / 107 / 108 | 956,403 / 956,403 / 964,782 (all) |
| `public-v3.7-b` (502) | 446 / 446 / 453 | 4,771,099 / 4,771,099 / 4,805,913 |

With `--no-lag-frames` both JSONL files equal the ones from before the
change in every field but the new one. With inference no replay matches
fewer frames, and nine replays hold one inferred lag frame each:

| Replay | Stop before | Lag frame (runner tick) | Now | Retail recordings |
|---|---|---|---|---|
| `FOX/Fox vs Marth [YS] Game_20200115T192755` | 5873 `cur_pos` | 5873, screen KO, 22 frames | 7816 / 7816 | proven: `slp_ys_fox_marth_r1_t6000`, `_lag5874_` |
| `PIKACHU/Game_20190629T164554` | 4576 `cur_pos` | 4576, screen KO, 121 | 7668 / 7668 | proven: `slp_fod_sheik_pikachu_r1_t4700`, `_lag4577_` |
| `YOSHI/17_50_44 Falco + Yoshi (FD)` | 9868 `cur_pos` | 9868, screen KO, 36 | 11052 / 11052 | proven: `slp_fd_falco_yoshi_r1_t10000`, `_lag9869_` |
| `DOC/Game_20191106T211939` | 4813 `cur_pos` | 4813, screen KO, 70 | 7932 / 7932 | proven since: `slp_fod_fox_drmario_r211939_r1_t5000`, `_lag4814_` ("The bridge follows the console" below) |
| `PEACH/15_52_33 Peach + Young Link (DL)` | 8501 `cur_pos` | 8501, screen KO, 62 | 11591 / 14695, `input_seed` | proven since, from the replay's own boundary: `slp_dl_peach_younglink_r155233_r1_t8700`, `_lag8502_` ("The bridge follows the console" below) |
| `FALCO/02_33_48 Falco + Marth (PS)` (first corpus) | 3042 `percent` | 2982, magnifier, 120 | 11421 / 11421 | proven: `slp_ps_falco_marth_frz_r1_t3200` (bit at record 2983, point at 3043), `_lag2983_` (bit 2984, point 3044, the console's tick) |
| `ZELDA_SHEIK/19_56_47 Sheik + Captain Falcon (DL)` | 3073 `percent` | 3013, magnifier, 120 | 15970 / 15970 | proven: `slp_dl_sheik_captainfalcon_r1_t3300` (point at 3074), `_lag3014_` (no point: Captain Falcon is back on screen with the count at 59; the console's percent stays 108.78) |
| `SAMUS/Game_20190629T124717` (FoD) | 7813 `input_seed` | 7809, particles, 600 | 9587 / 9587 | proven: `slp_fod_samus_marth_r1_t7900` (the console's Pre Frame seeds leave retail's draws at record 7814), `_lag7810_` (all 7899 on them) |
| `LINK/11_58_33 Marth + Link (DL)` | 3267 `input_seed` | 3264, particles, 600 | 10982 / 10982 | proven, by a pair another agent recorded the same night (local data in the main checkout, not registered here): `slp_dl_marth_link_r115833_t3400` (the console's seeds leave retail's draws at record 3268), `_r115833_lag3265_t3400` (all 3399 on them); both gate exact |

A retail record R is runner tick R - 1. Every recording above gates exact
over its full length with 0 differing particle ticks (the port equals retail
with and without the lag frame) and is in `SLIPPI_REPLAY_WITNESSES`.

**The Peach bridge does not reach its KO.** `slp_dl_peach_younglink_r1_t8700`
and `_lag8502_` (local data, boundary
`start_dl_slippi73_ns2019_p24_peach0_younglink4_4`, not registered): retail's
random stream leaves the console's at record 3197 (the console two draws
ahead, Young Link in action 357), a DamageFlyRoll draw differs at 3512 and
Young Link loses his last stock at 6484. The port's cold twin equals that
retail trace for all 6599 ticks, while the port run from the replay's own
inputs follows the console to 8501: the bridge's pads and the replay's are
not the same game from 3196. Not diagnosed (the bridge's raw pads are
reconstructed from the recorded sticks; "The pad queue is not in a replay").
The lag frame at 8501 therefore rests on the three proven screen KOs.
(Diagnosed since: Dream Land keeps its first fly-by delay from creation, and
that boundary was another seed's. "The bridge follows the console" below.)

**The one new stop.** `PEACH/15_52_33`, 11591, `p0.input_seed` with Peach in
DamageAir3 and Young Link in JumpF: random-stream drift on the tick before
(Peach hit out of her float); no skipped pass within eight ticks explains
it. Not a display-pass stop.
(A port fault, shown on retail: "The stop at 11591 is a port fault" below.)

**What else a display pass writes.** The completeness argument for "these
three kinds are what a lag frame changes": every write made in the display
pass (HSD_GObj_80390FC0's walk of the GX links) that outlives the pass and
that a tick reads, from the decomp's render callbacks (fighter, item,
stage, camera, magnifier, HUD, effects and particles). The port's display
pass (`melee-lib/src/frame.rs`, the `display_pass` branch) does four
things: the dynamics joints' caches, the particle sort, `render_cameras`
and held articles' poses.

| Display-pass write | Read by a tick | Port | A replay can show it |
|---|---|---|---|
| `fp->x221F_b0` and the window position +0x2188: ftLib_80086A8C (0x80086A8C) from ftDrawCommon_80080E18; not for a sleeping fighter | only through the next row | `render_cameras` | `percent`, sixty ticks on |
| the magnifier's `is_offscreen`: ifMagnify_802FBBDC (GX priority 0, so it reads the previous pass's bit) | ifMagnify_802FC998 in Fighter_8006A360 (0x8006A880) | `render_cameras` | `percent` |
| a screen-KO'd fighter's `cur_pos` and root translation (0x80080E90, 0x80080ECC) | everything that reads `cur_pos` | `Fighter::place_screen_ko` | `cur_pos` |
| the main CObj's eye and interest and `cm_804D6464`: Camera_8002A4AC from fn_800301D0 (0x800301FC) | Stadium's jumbotron close-up test (grStadium_801D32D0); Camera_800307D0 for Dream Land's background model and the audio pan | the close-up from `state.rendered_camera`; the other two are not game state | seeds on Stadium |
| the particle lists' order: particleSort (0x8039FC70) from psDispParticles (0x803A0088) | the particle interpreter's order, so the draws | `sort_for_display`, in the display pass | seeds |
| every drawn joint's matrix and dirty flag (HSD_JObjDispAll) | readers of a raw matrix or of the flag: the dynamics chains (ftCo_8009CB40), Thrown's hip test (ftCommon_8007E3EC), the tether and chain links | the dynamics joints and held articles only; the rest of the skeleton is cleaned by tick-time setups. Not shown to diverge, and not traced reader by reader | position in a throw or on a tether, in principle; none seen |
| the magnifier's edge state: ifMagnify_802FB73C (0x802FB73C) | bonus and statistics records only (fighter.c:2577, pltrick.c:180, plbonuslib.c:812) | not modelled | no |
| Stadium's jumbotron capture flags (grpstadium.c:1261..1301) | hidden flags and the image pointer only | not modelled | no |

Static particle joints, which HSD_JObjDisp would spawn at draw (0x80374A4C,
efLib_Cb_SPtcl), do not occur: no stage, fighter, effect, item or HUD file
on the disc has a joint with JOBJ_PTCL (a structural scan of 861 files; the
typed walk of the six stages and every `Pl*.dat` covers 11,180 joints). No
display code calls HSD_Rand. Not read: the per-kind item render tables, the
bodies of lbShadow and lbRefract, the HUD callbacks (stock, timer, name
tag), sysdolphin's draw internals, and stages outside the six.
## Pokemon Stadium bridges; the Link, Ice Climbers and HNC Stadium stops (2026-10-12)

### A transforming Stadium needs the replay's own boundary

Three Stadium bridges had left their consoles (the Mr. Game & Watch game
at its first transformation, a Link game near tick 3137, an HNC game's seed
at 1004). The cause of the first two is the boundary; the third is two
things already described (the 92:63 aspect, and a recording made before
`render_every_tick`).

**Stadium draws at creation.** grStadium_801D13E0 draws the base form's
first duration (one draw, before the Players exist: `Transformation::new`,
the one stage draw of `setup_draws` for `GrKind::PStadium`). That is ahead
of the boundary seed, so `boundary_seed` cannot change it: from a boundary
of another seed retail begins its first transformation on another tick than
the console. The port's cold twin inverts the boundary's own seed and
follows retail there, which is why such a bridge gates exact and still is
not the replay's game.

`GAMEANDWATCH/Game_20190629T112449` is the measurement. Its Game Start seed
is 2746006713; the boundary `start_ps_slippi73pre_p12_gameandwatch3_falco2_4`
was created from 2493903823 (its saved 1272613626 less the five setup
draws), and the two first draws are 11628 and 12900 of 65536.

| Recording | Boundary | Console's Pre Frame seed kept |
|---|---|---|
| `slp_ps_gameandwatch_falco_r1_t7100` | another seed's | to frame 4025 of 7099 |
| `slp_ps_gameandwatch_falco_r112449_t8200` | `..._r112449`, made with `--game-start-seed 2746006713` | all 8199 frames (the whole game) |
| `slp_ps_link_luigi_t10500` (before `render_every_tick`) | another seed's | to frame 1169; its match ended at 7902 |
| `slp_ps_link_luigi_r175018_t10500` | `start_ps_slippi73pre_p13_link4_luigi2_4_r175018` | all 10499 frames |
| `slp_ps_iceclimbers_samus_r191618_t6900` | `start_ps_slippi73pre_p14_iceclimbers1_samus4_4_r191618` (Game Start seed and spawn points, below) | all 6899 frames |

`slippi_to_scenario.py` now asks for the replay's own boundary on Stadium as
on Final Destination and Fountain of Dreams, unless the console's codes stop
the transformation (`stadium_frozen`: the wait never ends, 0x801D45FC;
`frozen_stages`: the controller never runs, 0x801D1548). The two HNC bridges
below are from such boundaries and keep their consoles' seeds. Dream Land,
Battlefield and Yoshi's Story draw at creation too (two, one and one draws);
whether those outlive the boundary was not measured here, and the Dream Land
bridge below was made from its replay's seed.

**Spawn points.** Some consoles started their players at spawn points that
Game Start names per player (PlayerInitData +5, copied by fn_8016D8AC at
0x8016D9E4): Ice Climbers and Samus on ports 1 and 4 stand at Stadium's
markers 0 and 1. `--retail-inputs` now writes `spawn_point` per player,
`make_boundary.py --spawn-points 0 1` writes them into gmVsMelee_StartData
at the creation hook the Game Start seed uses, and `slippi_to_scenario.py`
refuses a boundary whose points differ. This is what "Stadium consoles that
set per-player spawn points cannot be bridged" waited for.

**Is a bridge the console's game?** `harness/replay_vs_trace.py` (with the
slp example `frames`) answers it: each Pre Frame seed must lie on retail's
random stream inside its tick, and Post Frame's fields must equal the record
(for Slippi before 3.4.0 only against an `after_map` sample; otherwise the
seed decides). It is the port's own membership test, so a seed that falls
inside a busy tick's draws passes even when the counts already differ: the
Dream Land game below reads as leaving at frame 3267 where the first
different count is at 3264.

### The particle, bone and camera passes ran another display cadence

Found while reading the Ice Climbers recording. `record.py` runs each pass
in its own Dolphin. The particle, bone and camera snippets rebuild a
scenario from the TOML and carried `inputs`, `fighters`, `input_clock` and
`boundary_seed` only: not `render_every_tick`, nor `no_display_pass_before`.
Those passes of a `render_every_tick` recording therefore ran two
ticks per render, and from the first tick a display pass feeds back into
they are another game than the tick trace. Measured by each pass's
end-of-tick seed against the tick pass's:

| Recording | RNG ledger pass | Particle dump pass |
|---|---|---|
| `slp_ps_iceclimbers_samus_r191618_t6900` | equal, 6900 records | leaves at record 239; its match ended at 4820 |
| `slp_ps_link_luigi_r175018_t10500` | equal | leaves at 250; ended at 6699 |
| `slp_ps_gameandwatch_falco_r112449_t8200` | equal | leaves at 2047; ended at 7841 |
| `slp_ps_gameandwatch_falco_r1_t7100` | equal | leaves at 2047 |
| `slp_ps_fox_drmario_wide9263_r1_t9800` | equal | leaves at 2078 |
| `slp_ps_marth_peach_wide9263_r1_t5400` | equal | leaves at 4457 |
| `slp_dl_marth_link_r115833_t3400`, `..._lag3265_t3400` | equal | leave at 357 |
| `slp_fd_falco_yoshi_lag9869_t10000` | equal | leaves at 6038; wrote 9396 of 10000 records |

The gate, `triage`'s first divergence and `particle-sites` read the tick
trace, the ledger and the particle dump's initial state, so every result
above and before stands. What does not: `particles-diff`, triage's
"Particle generators" section and any bone or camera dump of a
`render_every_tick` scenario, past the record its pass left the game. A
dump that ended early reads as "0 differing ticks" beyond its last record.
The existing dumps of such scenarios were left in place; re-record a
scenario's passes before reading its particles.

The fix is one list, `tick_trace.PLAYBACK_KEYS`, that all three snippets
copy, and a check in `record.py`: every pass's seeds must equal the tick
pass's on every record, and the pass must be whole, or the recording fails.
Recordings without `render_every_tick` pass the check as they are (nine
checked, tick clock and VI clock, start boundaries included).

### The five stops

Each window was played back on retail with a display pass after every tick
and compared with its replay (`replay_vs_trace.py`) and with the port.

| Replay | Stop before | Retail, every tick | Cause | Now |
|---|---|---|---|---|
| `LINK/Game_20190406T175018` (PS) | 10399, `input_seed` | `slp_ps_link_luigi_r175018_t10500`: keeps the console's seed on all 10499 frames; the port left retail at 10367 | port fault: an air dodge's item window (below) | 15345 / 15345 |
| `ICE_CLIMBERS/19_16_18 Ice Climbers + Samus (PS)` | 6698, `follower.input_seed` | `slp_ps_iceclimbers_samus_r191618_t6750` (and `_t6900`): keeps the console's seed throughout; the port left retail at 6680 | port fault: an ice block's efLib_DestroyAll was queued (below) | 12040 / 15027, `follower.input_seed` again, not looked at |
| `LINK/11_58_33 Marth + Link (DL)` | 3267, `input_seed` | `slp_dl_marth_link_r115833_t3400`: equals the port for 3400 ticks; its particle interpreter draws 160, 138, 179 on records 3265..3267, the console 172, 132, 171 | a console's lag frame, shown on retail: `slp_dl_marth_link_r115833_lag3265_t3400` (no display pass before record 3265) draws the console's counts and keeps its seed on all 3399 frames; the port equals it too | unchanged; `--skip-display-pass 3264` matches 10982 / 10982 |
| `DOC/20200219 - HNC 1 ... Fox vs ME Dr. Mario - Pokemon Stadium` | 9684, `input_seed` | `slp_ps_fox_drmario_wide9263_r1_t9800`: equals the port for 9800 ticks and leaves the console on the same frame, 9684 | not a port fault found: retail in Dolphin under the console's known codes does what the port does. On record 9684 retail's jumbotron changes mode (grStadium_801D2A60, five draws, and grStadium_801D2528's duration; its last change was at 9006) and the tick draws 21 values, the console 12; on the next 9 against 11 | unchanged; no single `--skip-display-pass` in 9676..9685 moves it |
| `PEACH/20200219 - HNC 6 ... Marth vs WOO Peach - Pokemon Stadium` | 5251, `input_seed` | `slp_ps_marth_peach_wide9263_r1_t5400` (local data): keeps the console's seed on all 5399 frames; the port left retail at 3710, and now leaves it at 5250 | two faults. The first, a port fault, fixed: a Mr. Saturn thrown again before it had landed (Marth caught Peach's in the air) keeps its release box on the floor, because it_80275BC8 saves the box in use (xC0C = xC1C, 0x80275C8C) and the landing restores that (it_80275DFC); retail's xC0C and xC1C both read (1.75, 2.1, 2.1, 1.925) against the kind's (2.5, 3, 3, 2.75), and the port restored the kind's. The second is open and is the replay's stop: on record 5250 retail creates five effect families (its counter goes 3536 to 3541) and the port three; among retail's is a generator of program 64 at (-55.1, 27.35), by the left platform's edge, that the port does not make, with five more draws (0x8039E1E4, 0x8039E3D4 twice each, then 0x8039C870). Marth has just started Run and Peach a back air high above; no item is alive | 5251, `input_seed`, the second fault |

**An air dodge's first three frames.** ftCo_EscapeAir_IASA (0x80099C24)
counts mv.co.escapeair.timer down from PlCo +334 (3). While it runs each
tick calls ftCo_80095328 (0x80099C6C), the air throw test the jumps and
falls use, and when nothing is thrown ftCo_800D705C (0x80099CA4), which
opens the aerial catch window, as the dodge's entry does (0x80099AC4). A
throw state takes back the momentum the dodge replaced (the saved self_vel,
0x80099C84..0x80099C98); a drop in place (ftCo_80095744) leaves the dodge
running. The port's air dodge counted the timer and did neither. Link holds
a bomb, air dodges at 10365 and presses A with R held on the dodge's second
frame: the bomb drops (the port, keeping it, lacked 64 draws at 10398). Directed witnesses `links_airdodge_bomb_{throw,drop,cstick,third_frame,fourth_frame}_fd_fox4`
(the window's last frame throws, the next does nothing, on retail as in the
port before and after) and `peach_airdodge_turnip_catch_fd_fox4` (A in the
dodge opens the window; the falling turnip is caught a tick later).

**An ice block's generators go when it falls.** fn_802C1D44 (a sliding
block leaves its floor) calls efLib_DestroyAll itself (0x802C1D68), as
it_802C1854 does when a slope stops the block (0x802C1934). The port queued
both as efAsync requests. At 6680 a block slides off Stadium's left edge:
retail's generator list after the tick reads 14005, 8, 7, 107, 108, 14004
(the tick's four new generators linked behind the head the destruction
left); the port had 8, 7, 107, 108, 14005, 14004, with the block's
generator still at the head when they were created, and 26 particle draws
for retail's 32.

Scenarios in `SLIPPI_REPLAY_WITNESSES`: the Link, Mr. Game & Watch, Ice
Climbers, HNC 1 and both Dream Land bridges; the six directed ones are in
`MATRIX_WITNESSES`.


## The Yoshi stops (2026-10-12)

Three games of `public-v3.7-b` stopped on Yoshi; two port faults, both
played back on retail.

| Fault | Retail | Replays (matched before to after) | Witness |
|---|---|---|---|
| Yoshi's plain GuardOn started both powershield countdowns at zero | ftYs_Init_8012BECC (0x8012BECC) neither clears the reflect and powershield bits (x221C_b1/b2) nor writes their countdowns (guard x14, x18: fp+0x2354, fp+0x2358), where ftCo_800924C0 clears all four (0x80092504..10). The motion scratch is one union a motion change does not clear, so ftCo_80093BC0 counts down, as floats, what the last state to write those words left, and a bit a powershield set survives until a stun takes its word below zero; meanwhile a hit on the egg takes the powershield branch of ftColl_80076CBC (effect 27, not 1052) | `YOSHI/Fox vs Yoshi [PS] Game_20200222T144019` 1093 to 11164 / 11164; `MARIO/Game_20191207T130511` 6489 to 7723 / 7723 | `slp_ps_fox_yoshi_t1300` (809, 1049, 1060, 1091) |
| The Egg Roll's shell burst drew after the damage reaction's roll | The take-damage hook fn_8012EDE8 (from ftCommon_8007DB58) spawns the burst at once (efSync_Spawn 0x4CF at 0x8012EE50): twelve fragments, three draws each (0x80060848, 0x800608D4, 0x800608E0), before ftCo_8008DCE0's DamageFlyRoll roll (0x8008E124) | `YOSHI/Game_20190309T173757` 9961 to 12893 / 12893 | `slp_fod_yoshi_yoshi_r1_t10100` (9961; boundary `start_fod_slippi73ns_p14_yoshi2_yoshi3_4_r173757`, every tick) |

**The stale words.** The port keeps an image of the two scratch words
(`melee_ft::fighter::scratch_tail`): each common row whose scratch reaches
them reports what it wrote, a character's rows through
`CharacterCallbacks::SCRATCH_TAIL`, and a motion change folds that in. A tick
trace holds every fighter's memory, so `melee-sim scratch-diff <scenario>`
checks the image on every tick of any recording; it found the writers (the
damage rows' float and byte stores, FallSpecial, Walk, Turn, TurnRun, Entry,
the screen KO, DamageIce, Bury, YoshiEgg, the catch's pointer, the Egg Roll)
and every recorded Yoshi scenario now agrees on every countdown
(`docs/PORT_NOTES/M4_YOSHI.md`). The Mario game was not played back: Game
Start names its spawn points per player, which a boundary cannot set
("State-timing and position stops" above); it stopped with Yoshi in his
shield hold (342) under Mario's up aerial and completes with the first fix
alone.

**The burst.** On tick 9961 retail draws the hit spark (0x800785D0, the
generator's count at 0x8039F250), the thirty-six fragment values, then the
roll, and Yoshi stays in DamageFlyN; the port's roll took the first of the
fragments' values and entered DamageFlyRoll, with the same number of draws,
so the seed still agreed at the end of the tick. An `EffectRequest::EggShell`
now says whether retail spawned it inside a callback, and the damage entry
reserves such a burst's draws where retail made them
(`EffectQueue::reserve_creation_draws`, as for Sheik's dropped needle).

`slp_fod_yoshi_yoshi_r1_t10100` gates all 10100 ticks on every key and item
but differs in particle call-site order on four ticks (4954, 5555, 7417,
8799): each is 22 to 25 ticks into an aerial Egg Throw (365), the fault of
`slp_bf_yoshi_samus_t1000` above (two generators created on one tick in
the other order; the same draws). Fixed since (below): the named `m5_gate`
test `egg_roll_shell_burst_draws_before_the_reaction_roll` now asserts the
call sites too.

**Mr. Game & Watch's screen KO** now has two retail recordings, both with a
display pass after every tick and both exact in the port
(`docs/PORT_NOTES/GAMEWATCH.md`): the directed
`gamewatch_screenko_flat_fd_fox4` (DeadUpFall at 2983,
DeadUpFallHitCameraFlat at 3034) and the replay's own
`slp_ps_gameandwatch_falco_r112449_t8200` (6921, 6972).

**The generator cursor.** His explorer match
`corpus_v3_fd_gameandwatch_fox4_e17844051_p0` (5464, particle draws only) is
exact too: ftGw_Init_OnDamage destroys the article he has out inside his
proc, and Item_8026A8EC's efLib_DestroyAll walk (hsd_8039D688) leaves the
generator insertion cursor on the list's tail before the hit's burn overlay
is created (hsd_8039D9C8 links behind the head when the cursor is null or
the tail, behind the cursor's successor otherwise). It shows only when the
cursor is not the tail, here a list of one generator that then got
successors.

**The aerial Egg Throw's ticks** were another queue, the egg's own. Thrown
under the stage, the egg hits it in its collision proc
(itYoshieggthrow_UnkMotion1_Coll, 0x802B2BD8): the pass queues the bounce
spark (it_80277C40 -> it_80278800, efAsync on the item), then it_802B2C38
changes the motion, and Item_80268E5C's efAsync_QueueFlush (0x80268F44)
creates the spark's generator there; the burst's sparkle (0x4CE) is queued
after and waits for link 9. The port resolved the spark's request after the
proc and queued it behind the sparkle. `slp_bf_yoshi_samus_t1000` and
`slp_fod_yoshi_yoshi_r1_t10100` have no differing call site now. (The
launch, it_802B28C8, was not the cause.)

Corpora before and after these changes, on main at d1f39d6f
(`replay-batch`; no replay matches fewer frames, and only the three Yoshi
games match more). Of the 2529 scenario files of that day, 2488 gated exact
before and 2491 after (`slp_ps_fox_yoshi_t1300` and its twin, and the Mr.
Game & Watch match); the other 38 give the same result on both builds.

| Corpus | Complete | Frames matched |
|---|---|---|
| `public-v3.7` (108) | 107 to 107 | 956,403 (unchanged) |
| `public-v3.7-b` (502) | 446 to 449 | 4,771,099 to 4,785,336 |

## Five stops of the second corpus (2026-10-11)

Five `public-v3.7-b` games listed as open and undiagnosed, each played back
on retail from its own setup with a display pass after every tick
(`render_every_tick`). Four were port faults, each one cause; the fifth is
retail and the port against the console. Trace ticks; a replay frame N is
trace tick N + 1.

| Replay | Stop before | Retail, every tick | Cause | Now |
|---|---|---|---|---|
| `FOX/Game_20190824T124750` (DL) | 1482, `percent`: 28.58 recorded, 28.61 in the port | `slp_dl_fox_falco_r1_t1600`: Falco's laser deals 2.88 | a phantom's credit reads a freed GObj (below) | 8316 / 8316 |
| `MARIO/Game_20190309T094546` (PS) | 6305, `motion_id`: Pikachu's Thunder loop ends in the recording | `slp_ps_mario_pikachu_r1_t6400`: the loop ends at 6298 where the port's went on | a Thunder bolt answers to its caller, not its owner (below) | 7472 / 7472 |
| `ZELDA_SHEIK/20200304 - HNC 22 - PM 0855 - Falco (Default) vs Zelda (White) - Fountain of Dreams` | 8969, `cur_pos`: Sheik 0.025 further left | `slp_fod_level_falco_zelda_r1_t9100`: Sheik moves from 8970, in the port from 8974 | the repeated-hit victim is forgotten by the hitstun bit (below) | 11472 / 11472 |
| `DK/21_45_04 [INFP] Donkey Kong + [C2] Ganondorf (DL)` | 4779, `input_seed` | `slp_dl_donkeykong_ganondorf_gs_r1_t4900`: 28 draws at 4770, 22 in the port | the damage flash needs damage logged this frame (below) | 10442 / 10442 |
| `PICHU/Game_20161120T232910` (DL) | 2158, `motion_id`: Pichu's Turn kept in the recording | `slp_dl_captainfalcon_pichu_r1_t2300`: retail dashes at 2158, as the port does | not a port fault: the console's pad queue (below) | unchanged |

All five scenarios gate exact over their full length with 0 differing
particle-site ticks and are in `SLIPPI_REPLAY_WITNESSES`.

**A phantom's credit reads the GObj that is there when its lockout ends.**
A phantom hit (half damage, no knockback) is applied by ftColl_8007BE3C when
the victim's lockout (dmg.x189C) runs out, and only then credits its source:
the function loads the GObj pointer kept at dmg.x1894 (retail 0x8007BF4C) and
switches on its classifier (0x8007BF50: `lhz 0x0`; 4, a fighter, calls
plStale_UpdateStaleMovesFromFighter and ftColl_80076444; 6, an item,
plStale_UpdateStaleMovesFromItem and ftColl_8007646C; anything else nothing).
Falco's Phantasm ghost grazes Fox at tick 1374 (lockout 4, damage 3.5) and
its lifetime ends at 1377; at 1378 x1894 still holds 0x80D527C0, now a freed
GObj whose first word is the allocator's free-list link (HSD_ObjFree,
0x8037AD20), so nobody is credited and the side special never enters Falco's
stale table. The port had captured the item's owner and attack at the hit and
credited them at the end: one more entry in the table, the laser a slot
older, 2.91 instead of 2.88. `melee_coll::damage_log::ItemSource` is now the
item's GObj address (`hsd_gobj::GObjAddresses`, "GObj addresses" above), and
the credit goes to whatever live item has that address when the lockout
ends: the same item's current owner and attack, another item created there
since, or nobody.

**A Thunder bolt answers to the fighter that called it.** The bolt keeps its
caller at xDD4 +0x38 (it_802B1DF8), and everything it does to Pikachu goes
through that pointer, not ip->owner: the lead's end
(it_2725_Logic39_Destroyed, 0x802B204C: `lwz 0xE0C`, then
ftPk_SpecialLw_SetState_Unk0) and the floor strike's quake
(itPikachuthunder_UnkMotion1_Coll, 0x802B2250); Pikachu in turn reaches the
lead by its own pointer (mv.pk.speciallw.x0: 0x80127680, 0x80127758,
0x80127770). The kind has no reflected callback, so a cape or a Reflector
changes only the owner. The port routed all three by owner: Mario's cape took
the lead bolt (tick 6284 of the recording), the bolt's end (6297) told Mario,
and Pikachu's loop waited for its script. `ItemLogic::tracker` names the
fighter an article answers to (its owner by default; the bolt's creator), for
the owner report, the destroyed notice, control requests and the callbacks'
owner context. The recording is retail but not the replay's game past tick
4233: its boundary was not created from the replay's Game Start seed, and
Stadium keeps its first base-form duration from creation ("Pokemon Stadium
bridges" above), so the first transformation comes on another tick. It
reaches the same reflected lead at 6284, and the replay itself now matches
to its last frame.

**The repeated-hit victim is forgotten by the hitstun bit.** ftColl_800764DC
(Fighter_procAnim out of hitlag, fighter.c:1686) clears the attacker's
victim pointer (x2094) when the victim's x221C_b6 is clear (0x80076504: `lbz
0x221C`; `extrwi. 1, 30`) and its grace frames (x2098) have run out. The port
asked whether the victim's damage state still had hitstun to count, and a
tech out of hitstun leaves that timer where it was: Falco, down-thrown and
teching in place, stayed Sheik's victim. Her first needle then found the
victim with another move and counted from zero where retail starts a new
victim at one (ftColl_800763C0), so the fifth needle (tick 8969) did not
reach PlCo +0x4C4 and the push (0.025 a tick for twenty ticks,
ftColl_80076528) began four ticks late, with the sixth. The test is now the
bit, and it runs only when the attacker's animation proc does (not asleep,
not in hitlag).

**The damage flash needs damage logged this frame.** ftCo_8008DCE0 installs
the hit's colour animation through ftCo_8008DA4C, which tests
dmg.x1838_percentTemp first (0x8008E19C: `lfs 0x1838`; `fcmpu`; `beq`). A
hit resolved by Fighter_ProcessHit has its damage there; the cargo carry's
breakout does not: ftCo_Shouldered_Anim and ftCo_8009C744 give each fighter
its damage with Fighter_UnkTakeDamage_8006CC30 and call ftCo_8008E908
directly (ftCo_Shouldered.c:73, 113), so neither flashes. The port tested the
hit's own damage and installed the flash. With a charged Giant Punch that
shows in the random stream: the charge's steam is the secondary colour
program, restarted by the motion change, and it spawns its first puff at
once unless the primary slot is busy (ft_800BFF70). Retail draws the puff's
three offsets (ftCo_8009F834) and its generator emits on the tick of the
breakout (4770); the port's flash held the primary slot and the puff was
skipped.

That game needed its own boundary. Dream Land draws the first fly-by's delay
when the stage is created (grOldPupupu_80210C7C, 3000 to 4000 ticks), before
the boundary seed is written, so from another boundary the fly-by draws
(grOldPupupu_80210D10, four values) on another tick: tick 3771 from
`start_dl_slippi73_ns2019_p23_donkeykong4_ganondorf4_4` against 3856 from
the replay's own `..._r214504` (created from its Game Start seed 3902852940).
The first bridge (`slp_dl_donkeykong_ganondorf_r1_t4900`, local data) gates
exact for 4900 ticks and says nothing about the stop: by tick 4615 Whispy
blows in the replay and not in that recording. `slippi_to_scenario.py` now
refuses a foreign boundary on Dream Land past tick 3000.

**Pichu's turn: the console's pad queue, again.** Retail in Dolphin under
UCF 0.73 with a pad queue of one poll per tick reads -100 against -2 at tick
2159 (`pad_queue_x` in the trace) and restarts the turn as a dash, as the
port does; the console kept the slow turn, which the code does only when the
older sample is the previous tick's -53 ("The pad queue is not in a replay"
above). Two probes with the recorder's lag frame at record 2158 or 2159
(`slp_dl_captainfalcon_pichu_lag2158_t2300`, `..._lag2159_t2300`, local
data; the port gates both exact) dash as well: a lag frame as the recorder
plays it runs two ticks in one frame but still gives the queue one sample
per tick, so it does not reproduce a queue holding an extra poll. Captain
Falcon stays in RebirthWait through the window in the replay, retail and the
port alike; nothing about the revival platform is involved.

Corpora before and after (`replay-batch`; no replay matches fewer frames):

| Corpus | Complete | Frames matched |
|---|---|---|
| `public-v3.7` (108) | 108 to 108 | 964,782 (unchanged) |
| `public-v3.7-b` (502) | 453 to 457 | 4,810,990 to 4,827,157 |

(On main at 3e20e9de, with the runner's lag-frame inference; before that
merge the same four games took the second corpus from 446 to 450.) The
four games above are the only replays that change. Every scenario
recorded before gates as it did (2529 files: 2488 pass, 41 do not, the same
ones), and of the 72 recorded during the work only the four bridges above
change, from their stop to exact.

Seen on the way: the particle dump of
`slp_dl_donkeykong_ganondorf_gs_r1_t4900` differed from the port at tick 4852
in one generator's x (-9.6 in retail, -23.6 in the port), with the same
draw sites and seeds. It is the burst of a lost stock icon (generator 0xF7,
ifStock_802F8298), Ganondorf's in port 3: the HUD keeps a place per port
(descendants 2..5 of ScInfDmg_scene_data) and the port indexed them by
player order. Fixed; the dump agrees on every tick.

## The bridge follows the console (2026-10-12)

Several bridges had not reproduced their consoles while the port, run from
the replay, did. Each case was taken again with a checker that compares a
recording with its replay input for input. None was a pad the bridge had
rebuilt wrongly in a way a fighter reads; four other things were wrong, and
with them fixed every case is settled on retail.

### The checks

```sh
cargo run -q --release -p slp --example frames -- <game.slp> > frames.jsonl
cd harness && uv run python replay_vs_trace.py frames.jsonl <scenario>   # recording against replay
cd harness && uv run python pad_schedule_check.py <scenario>             # recording against schedule
cargo run -q --release -p melee-sim --example retail_pads -- <dir>       # which replays can be fed
```

- `replay --retail-inputs` writes only raw pads that read as Pre Frame
  (`melee-sim/src/replay_retail_pads.rs`): each candidate goes through HSD's
  clamp (HSD_PadClampCheck3, 0x80376E90), its direction test
  (HSD_PadADConvertCheck1, 0x803771D4: length 30 and a sector) and the
  fighter's dead zone, and must give the recorded sticks, direction bits and
  triggers with the recorded raw X. A pad that cannot be fed ends the
  schedule there: the header's `ticks` is that tick and `pads_end` says why;
  `slippi_to_scenario.py` refuses a longer scenario.
- `record.py` fails a tick-clock recording whose records did not dequeue
  the scheduled pads (`pad_schedule_check.py`: the queue entry's stick bytes
  and the status's buttons and triggers).
- `replay_vs_trace.py` compares each record's HSD status and queue entry
  with Pre Frame and prints the first differing pad; from GO on (frame 84) a
  difference fails the bridge. With `slippi_to_scenario.py --after-map` the
  recording carries the map proc's sample and every Post Frame field is
  compared too.

### What was wrong

| Fault | Where it showed | Fix |
|---|---|---|
| A dead-zoned stick axis was fed as zero, so HSD's direction bits (the held word, fp+0x65C) were not the console's. The raw Y beside a recorded X, and both C-stick bytes, were derived from the fighter's values | `PEACH/15_52_33`: 25 of the first 6000 pads (the C-stick's up bit at tick 3011: raw (0, 27) is under the threshold of 30); one to four steps of the older witnesses. No fighter code reads those bits, so no game changed | the search above |
| A boundary of another seed on a stage that keeps a creation draw. Dream Land's first fly-by delay was found the same day ("Five stops of the second corpus" above); Battlefield keeps its first background delay the same way (grBattle_BG_Callback0, 0x8021A344, 2400 to 3600 ticks) | `PEACH/15_52_33`, the bridge "not diagnosed" above: the fly-by's four draws (0x80210D68..) come at record 3196 where the console drew them at 3197, with every pad and fighter field equal until 3207. `DOC/Doc vs Fox [BF]`: the background's two draws (lr 0x8021A508) at record 3633 | `slippi_to_scenario.py` asks for the replay's own boundary on Battlefield past tick 2400, as on Dream Land past 3000. Yoshi's Story takes any boundary: its creation draw is overwritten (0x801E3304) |
| The one-entry queue of `render_every_tick` left qwrite at the five-entry queue's value, so the first poll accepted while a tick ran overwrote the injected entry or its history | record 124 of the Dream Land bridges (raw X 82, Dolphin's poll of the tick before's pad, for the scheduled 106) and record 302 of `slp_ps_fox_drmario_wide9263_r1_t9800` (78 for 16). The fighter's status was right; what UCF read on that tick depended on when the poll came | qwrite is 0 from installation: such polls go to entry 0, which nothing reads |
| A recorded raw X of zero beside the tick before's stick, buttons and triggers was refused ("no raw stick Y gives ...") | tick 2 or 3 of 26 replays (`DOC/Doc vs Fox [BF]`), and 64 to 105 ticks after GO in four games | It is a console's failed poll: PAD_ERR_TRANSFER zeroes the SDK entry and HSD_PadRenewMasterStatus keeps the last status (controller.c:386, `cmpwi r0, -3` at 0x803778B0). The bridge feeds it as one (`raw = { err = -3 }`): retail keeps the pad and UCF reads the zero |

Two readings were not faults of the bridge at all:

- *`slp_fd_captainfalcon_iceclimbers_r001727_t17300` "near tick 350".* The
  replay shows Captain Falcon in JumpF on frame 346 and retail in
  CapturePulledLw; on the console he was grabbed on that tick too (ThrownLw
  from frame 353 in the replay itself). Slippi 2.0.1 reads Post Frame when
  the fighter's map proc returns, before the grab resolves. That recording
  really leaves its console at record 3722, because it predates
  `render_every_tick`: the magnifier's point comes two ticks late. Recorded
  again from a boundary with the replay's spawn points
  (`slp_fd_captainfalcon_iceclimbers_r001727b_r1_t3900`, local data) retail
  follows the replay for all 3899 frames and the port equals it.
- *The 84 position ticks of `slp_fod_level_fox_jigglypuff_r1_t13500`.* 83 of
  them are ten throws (ThrownF, ThrownHi, ThrownLw): the captor's accessory
  pins the victim after the victim's map proc, where Post Frame was read.
  `slp_fod_level_fox_jigglypuff_aftermap_t3100` (local data) records that
  sample over the first two throws: seeds, actions, positions and percents
  equal the replay on every one of 3099 frames.

Before GO, Pre Frame's stick need not be a pad's (-127/128 in `19_18_23
Luigi + Fox (DL)`, 1.0 without a direction bit in two Pichu games); the pad
then keeps what HSD's side records and the record says `frozen`. Those
three games hold such sticks after GO too and end their schedules there. A port that ran Dween's fix records
the code's output, not the pad: seven replays end their schedule at the
first rewritten stick. Of 610 replays 570 read on every tick, 30 hold failed
polls or frozen sticks, and 10 end early (`retail_pads`).

Re-bridged with the new writer, `slp_ys_iceclimbers_marth_t200` and
`slp_ys_jigglypuff_fox_t300` give the recorded schedule step for step;
`slp_bf_yoshi_samus_t860` differs in one step of 595 and
`slp_ys_fox_falco_t1900` in four of 2193, each a raw Y that restores a
direction bit.

### The cases, on retail

Every recording below has a display pass after every tick, was fed its
replay's pads on every frame (`replay_vs_trace.py`), consumed its schedule,
and gates exact in the port over its whole length.

| Replay | Stop | Retail | Verdict |
|---|---|---|---|
| `PEACH/15_52_33 Peach + Young Link (DL)` | 8501, screen KO position | `slp_dl_peach_younglink_r155233_r1_t8700` (boundary `start_dl_slippi73_ns2019_p24_peach0_younglink4_4_r155233`): keeps the console's seed on all 8699 frames and places Peach at frame 8501 at (-12.372, 155.081) where the console recorded the held (-11.008, 158.963); 61 frames differ to the end of the KO. `..._lag8502_t8700` (no display pass before record 8502): no frame differs | a console's lag frame, now proven; the port equals both |
| the same game | 11591, `input_seed` | `slp_dl_peach_younglink_r155233_lag8502_t11700` (local data) keeps the console's seed on all 11699 frames; the port leaves retail at record 11587 | **port fault**, open (below) |
| `DOC/Game_20191106T211939` (FoD) | 4813, screen KO position | `slp_fod_fox_drmario_r211939_r1_t5000` (boundary `start_fod_slippi74_ns2019entry_p14_fox2_drmario0_4_r211939`, with the constructed code `neutral-spawn-2019-entry`): Dr. Mario at (-23.595, 70.570) for the console's (-22.556, 74.932) at frame 4813, 70 frames differ; `..._lag4814_t5000`: no position differs in 4999 frames | a console's lag frame, now proven |
| `DOC/20200219 - HNC 1 ... Fox vs ME Dr. Mario - Pokemon Stadium` | 9684, `input_seed` | `slp_ps_fox_drmario_wide9263_r2_t9800`: leaves the console at frame 9684, as the port; `..._lag9382b_t9800` (no display pass before record 9382): keeps the console's seed on all 9799 frames | a console's lag frame 302 ticks before the stop (below); the port equals both |
| `ICE_CLIMBERS/20_31_01 Captain Falcon + Ice Climbers (FoD)` | 7022, the pad queue | `slp_fod_captainfalcon_iceclimbers_r203101_r1_t7100` (boundary `start_fod_slippi73_ns_p13_captainfalcon1_iceclimbers0_4_r203101`): Captain Falcon leaves his slow turn for a Dash at record 7023 (`pad_queue_x` [100, 1]) and the seed leaves the console's at frame 7026, as in the port. `..._poll7023_t7100` (local data; `unconsumed_poll_before = [7023]`): keeps the console's seed on all 7099 frames | not a port fault: one poll no tick consumed in the console's queue, shown sufficient on retail |
| `ICE_CLIMBERS/Game_20190903T151039` (PS) | 423, Nana's x one ulp | `slp_ps_iceclimbers_jigglypuff_r151039_r1_t500` (local data; `start_ps_slippi73pre_p12_iceclimbers0_jigglypuff4_4_r151039`): 0xC131386A at frame 423, the console 0xC1313869; seeds and pads equal throughout | retail in Dolphin equals the port, not the console |
| `ICE_CLIMBERS/Game_20191209T171524` (YS) | 200, Nana's x one ulp | `slp_ys_jigglypuff_iceclimbers_r171524b_r1_t400` (local data; `..._r171524b`, with the replay's spawn points): 0x41EC558E, the console 0x41EC558D | the same |

**The two Nana positions.** Both are the tick's whole difference: the frame
before is equal bit for bit, y is equal, the random stream and both pads are
the console's, and from there the x stays an ulp or so apart (40 and 71
frames in the windows recorded). So with a faithful bridge the answer is
neither a pad nor the port: retail in Dolphin computes the port's value and
the console recorded another. Nothing in a replay says whether that is
Dolphin's arithmetic or something the console ran. The 2019-12-09 game's
Ice Climbers port ran Dween's fix (its schedule ends at tick 786); the
window to 400 holds no rewritten stick.

**The pad queue.** UCF's dashback reads queue entries qread-1 and qread-3.
With one poll per tick Captain Falcon's raw X runs 1, 45, 100 and the
change over two ticks is 99: the turn is cut short, on retail as in the
port. With
a poll that no tick dequeued between records 7022 and 7023 the code reads
100 against 45 (`pad_queue_x` of record 7023 is [100, 45]), leaves the slow
turn, and retail then plays the console's game to the end of the
recording. What put the poll there is not in the replay.
`PICHU/Game_20161120T232910` (2158) is the same shape ("Five stops of the
second corpus" above, where the recorder's lag frame did not reproduce it);
an unconsumed poll was not recorded for it.

### A lag frame that shows 302 ticks later (HNC 1)

Stadium's jumbotron (grStadium_801D2344) leaves the player close-up (mode
8) when the close-up box no longer fits the viewport (grStadium_801D32D0,
which reads the CObj of the last display pass). The next mode is chosen by
grStadium_801D2A60: one or more draws, except when the counter xF2 has
reached `yakumono_param->x50`: then the standings (mode 14) come with no
draw, for a fixed 300 frames (grStadium_801D2528). That is the one mode
change that leaves nothing in the random stream.

In this game the close-up entered at record 9006 is lost at record 9382 (in
the port, with 803 frames of its timer left), into the standings; their
timer ends at record 9684 and the next mode draws (0x801D2ABC five times,
0x801D27C8 once): 21 values on that tick. The console drew 12 there and 11
on the next. With no display pass before record 9382 the test reads the
camera of a tick earlier, the close-up lasts one tick more, the standings
end at 9685, and retail draws 12 and then 11 (one mode draw and its
duration): the console's seeds on every frame. `--skip-display-pass 9381`
makes the port match all 13405 frames of the replay. Lag frames at records
9683, 9684 and 9685 (three recordings, local data) change nothing on
retail, which is why "no single skipped pass near the stop" had found
nothing.

**What rule would let the runner infer it.** Not a wider search: the
candidate is named by the port's own state. On an `input_seed` stop on
Pokemon Stadium whose tick (or the one before) holds the end of a
jumbotron mode that was entered without a draw (the standings, entered
from a lost close-up), try the display pass before the tick that close-up
was lost on, and the two beside it; accept under the `particles` margin
unchanged (every frame from the candidate through the stop and 600 more,
seeds included). Three candidates, each fixed by where the simulation
itself lost the close-up, and none when the silent mode was entered by a
timer. The runner is unchanged: its rule still tries only the stop and the
eight ticks before, so this game still stops at 9684 without
`--skip-display-pass 9381`.

### The stop at 11591 is a port fault

`slp_dl_peach_younglink_r155233_lag8502_t11700` (local data: it does not
gate) follows its console past the stop, so the stop is the port's. At
record 11587 Young Link's boomerang (item kind 59) strikes Peach under her
open parasol (state 367, frame 45; 144.49 to 146.49 percent). Retail's
generator list after the tick reads 10, 59, 58, 57, 24, 23, 21, 22: three
new generators (57, 58, 59) directly behind the head, then the hit's four.
The port has 10, 24, 23, 21, 22, 58: one of the three, at the tail, and 32
particle draws for retail's 39. The parasol is an article removed on
damage, as the boomerang in hand and Mr. Game & Watch's articles are
("The generator cursor" above); which call creates 57 and 59 was not
followed. Every fighter key still agrees on that tick.
## Slippi Online (2026-10-12)

A Slippi Online (netplay) game is not a console's game with other inputs:
the seed is rewritten every frame, a rollback runs ticks again without
rendering, the build carries its own codes, and the machine is an emulator.
`melee-sim replay` runs them as one more setup
(`crates/melee-sim/src/replay_online.rs`); the 14 games refused as "Slippi
Online initialization/seed resets" and the 3.6.0 game refused as "Game Start
seed ... does not reach the first Frame Start" (all `public-v3.7-b`, July and
August 2020) now run, 13 of the 15 to their last frame.

### What Online changes

Sources: project-slippi/slippi-ssbm-asm at 0e90140 (2020-07-21, the netplay
build of these games; `Output/Netplay/GALE01r2.ini` lists its codes) and
fcf47f1 (2026-05, for the later code set); project-slippi/slippi-wiki
`SPEC.md`; project-slippi/Ishiiruka, branch `slippi`. Logic described, not
copied (GPL-3).

| | A console or local game | An online game | Source |
|---|---|---|---|
| Seed at match creation | whatever the menus left | the negotiated RNG offset, stored at 0x8016E748, one instruction before Game Start reads the seed (0x8016E74C). The Ground and Players are created from it as in any game, so `boundary_seed_from_creation` holds | `Online/Core/InitOnlinePlay.asm` |
| Seed during the match | one stream from the boundary on | **written before every tick, not only at a rollback**: a GObj (class 4, p_link 7) whose proc `FN_SyncRNG` runs at s_link 0, before any fighter proc, stores `rotl(scene frame counter, 16) + offset` (the counter is gm_80479D58 +8, 0x80479D60; zero on the first tick). Frame Start records the seed after that write, so the first Frame Start equals Game Start's seed | the same file |
| Match setup | the menus' StartMeleeData | copied from Dolphin's match block: ports 1 and 2, spawn points 0 and 1 named per player (Game Start's `spawn_point`), four stocks, eight minutes, the stage; A inputs cleared on every port so no Zelda/Sheik swap | the same file |
| Pause | the engine stops; no frames are recorded | with Pause off in the rules (Game Bitfield 3, 0x08: 12 of the 15 games) a client-side pause that stops nothing; with it on, the retail pause. The scene frame counter runs on through a pause (SPEC, Frame Start), so the seeds then leave the tick count | `Online/Core/InitPause.asm` (0x8016E904) |
| Rollback | none | the state saved at the first predicted frame S is loaded and ticks S to F run in one pass of gm_801A4D34, F being the frame about to be simulated; between them the code calls Camera_8002A4AC and ftLib_80086A8C for every fighter and branches back to the tick body: **no display pass** | `StartEngineLoop.asm` (0x801A4DE4), `LoopEngineForRollback.asm` (0x801A5014), `TriggerSendInput.asm` (0x80376A28) |
| In the replay | each frame once | each frame once per simulation of it, in the order simulated. Nothing marks an event as rolled back; Frame Bookend carries `latest finalized frame` (3.7.0). `slp::Replay::frames` keeps each frame's last version (a repeated Frame Start now drops everything the earlier version recorded, a follower included) and `frame_sends` the order | SPEC; `crates/slp/src/lib.rs` |
| Codes | not recorded before 3.3.0; read from the frames | **named**: the Gecko List event (3.3.0+) is the code table the game ran. `slp::gecko` decodes it and `replay_online::Listed` reads from it what the port models | SPEC; `Recording/SendGameInfo.asm` |
| Arithmetic | the Gekko | Dolphin's JIT in deterministic mode ("The emulator's zeros" below) | Ishiiruka `Jit_FloatingPoint.cpp` |

**Codes of the 2020 netplay build that change a match** (0e90140; the same
list in every one of the 14 Slippi 3.7.0 games, the 3.6.0 game's without
Widescreen):

| Code | Site | Port |
|---|---|---|
| NeutralSpawn | 0x8016E510 | the 2020 table, chosen by the first frame as before |
| UCF 0.74 with the per-port toggle (and Dween) | 0x800C9A44, 0x800998A4, 0x8006B028 | by version and date as before; no dashback probe, a netplay build carries one version |
| Disable FD Background Transitions | `0421AAE4 60000000` | `frozen_stages` on Final Destination |
| FreezeStadium (`Online/Core/PreventFileAlarms`) | 0x801D4578, in grStadium_801D4548 after its first call: branches to the function's end (0x801D4FD8) | `frozen_stages` on Pokémon Stadium: the same as the nop of the call at 0x801D1548. Game Start's `frozen_ps` is 0 (the recorder tests 0x801D45FC only) |
| **BrawlOffscreenDamage** | 0x8006A880, the `bl ifMagnify_802FC998` of Fighter_8006A360 | `SlippiCodes::offscreen_damage_by_camera_limits`: off screen means outside the stage's camera limits (Stage_GetCamBounds*Offset, `fcmpo` left, right, top, bottom), unless vanished (x221F_b1) or in DeadUpStar or DeadUpFall. No camera, screen code or display pass enters the magnifier's damage |
| Stadium preload | as offline | never reached: the controller does not run |
| Widescreen (optional, the recording player's own) | 0x8036A4A8, 0x80030C7C, 0x80030C88 | `Screen::Widescreen`, read from the list (320 / 219 by the hook's floats) |
| Lagless FoD | 0x801CBB90 and seven writes in grIzumi | display only (the reflection, an animation, a matrix copy); the two Fountain games match with the stage as it is |
| Init Stage Data, Init Player Data | 0x801C154C, 0x80068EEC | zero the new GObj's data; the port's state is already defined |
| FreezeGlitchFix (from 2020-07-21) | `041239A8 60000000`, in ftNn_Init_80123954 | not ported: an online game with Ice Climbers and this code is refused |
| PreventCharacterCrowdChants, the sound hooks, mute music | 0x80321D70, 0x8038D224, 0x80088224, ... | no effect on a compared field or a draw in these games |

Later builds add codes that do change the match and are not ported
(fcf47f1's `affects-gameplay` list): FreezeDeadUpFallPhysics (0x800D4C1C,
0x800D4D68, 0x80080E80), DesyncProofBGTransformations (a C2 at 0x8021AAE4),
the Stadium file codes (0x801C65C8, 0x801D4760, 0x801D457C, 0x800165AC),
WhispyBlowDirFix (0x8008653C), NanaDeterminism (0x800AC5B8),
PSCameraIndependentMonitor (0x801D24FC), PreventWobbling (0x800DB880,
0x8008F090), and the optional Disable Screen Shake (0x80030E44). A game whose
list holds one is refused with each named ("netplay code ... at ... is not
ported"), so a newer corpus says what to port: `crates/slp/tests/data/v3.12.slp`
(2022) names five and `v3.16.slp` (2024) eight.

### What the runner does

A game is online by Game Start's major scene (8, Slippi 3.7.0 on) or, without
that field, when every Frame Start seed is the sync's for its frame (the
3.6.0 game). The report says which. Then, and only for such a game:

1. **Each tick starts from its Frame Start seed**, set on the simulation
   before the tick (`Simulation::set_rng_seed`, as a boundary seed is: the
   simulated match holds no new state). The first-frame check that the setup
   draws reach the first Frame Start does not apply (that seed is the
   sync's); the music draw falls before the first write.
2. **A tick a rollback ran** takes the rollback loop in place of the display
   pass (`DisplayPass::RollbackLoop`: the rendered camera and each fighter's
   off-screen bit, no particle sort, no screen KO placement, no joint
   caches, no magnifier). Which ticks: from `frame_sends`, every frame after
   the first of a pass that steps back, up to one past the newest frame sent
   before it; the first frame of the pass starts from the saved state and
   keeps what preceded it when it last ran (`replay_online::rollbacks`).
3. **The codes are the list's** (`Listed`): the stage code, the screen and
   the off-screen rule; the report's `stage code` and `screen` lines say
   "(the replay's Gecko list)". No dry pass runs. Lag frames are not
   inferred (`lag_frames` is null in the JSONL).
4. The report's `online (...)` line and the JSONL's `online` object carry
   the counts below. Every game that is not online has `"online": null` and
   is otherwise byte for byte what it was.

**What is checked less than in a console game, exactly:**

- *The end-of-tick seed.* The next Frame Start is the netplay code's write,
  not what this tick's draws left, so `rng.seed` is not compared; no
  recording of an online game holds it. What remains of the seed: each Pre
  Frame seed must lie on the port's stream between the tick's first seed
  and its last, as before. A tick's draws after the last fighter's input
  proc are seen only through what they do. The report counts the Pre Frame
  seeds that follow draws (`pre_frame_seeds_after_draws`: 764 of 27,220 in
  the witness game; the rest equal the Frame Start seed and say only that
  nothing drew first).
- *Frame Start itself* is supplied, not derived. The diagnostic: how many
  supplied seeds are not `rotl(tick count, 16) + offset`
  (`seeds_off_the_tick_count`). It is 0 for all 15 games and the three
  fixtures (170,810 frames), so on these the supplied seed is the one the
  port would compute from the tick count and Game Start alone. A nonzero
  count means a pause, and `--retail-inputs` refuses such a game (the
  recorder derives the seed from the counter).
- *The sign of a zero knockback velocity* (next section).

Every other recorded field is compared on every frame, bit for bit, as
before.

### The emulator's zeros

With the above, 13 games stopped 400 to 16,000 frames in on `kb_vel.y`:
+0.0 recorded, -0.0 in the port, a fighter knocked along the floor that has
just left it. The port is the console's value. Fighter_procUpdate decays
airborne knockback with `fnmsubs` (0x8006B9C8, 0x8006B9E4:
`kb -= decay * cos/sin(atan2f(kb.y, kb.x))`); with `kb.y` zero and `kb.x`
positive the sine is +0 and the Gekko's -(a*c - b) is -0. Slippi's Dolphin
computes it otherwise: in deterministic mode (netplay) its JIT does not fuse,
and "We implement nmsub a little differently ((b - a*c) instead of
-(a*c - b))" (Ishiiruka `Source/Core/Core/PowerPC/Jit64/Jit_FloatingPoint.cpp`,
`fmaddXX`), which is +0.

| Evidence | -0 | +0 |
|---|---|---|
| Console recordings (26 games, Slippi 3.5+), airborne, `kb_vel.x` nonzero | 2675 frames | 468 |
| The 15 online games, the same frames | 9 | 1463 |
| The witness game on retail in this project's Dolphin against its replay | 221 fighter-frames where the replay has +0 | (the replay's value) |

Console recordings hold +0 too, but not from this instruction. Split by
whether the air decay ran on the tick (the frame before airborne as well,
its zero `kb_vel.y` kept and its `kb_vel.x` changed; last versions only):
consoles have -0 on 2504 such frames and +0 on 2 (one each in two doubles
games), the online games +0 on 1054 and -0 on none. The consoles' other
466 +0 frames (and the online games' 97) are zeros no decay has touched:
the tick a fighter leaves the floor, hitlag, a fresh horizontal hit. The
port computes those +0 as well, and every console game is compared by
sign: the rule below applies only to a game the runner identified as
online (`Game::set_strict_zeros`; a test pins it).

The last row is exact: the retail recording of the game below differs from
the replay in the sign of `kb_vel.y` on 221 fighter-frames and in nothing
else, and 221 is the count the runner reports for that game.

The port keeps the console's arithmetic. For an online game the comparison
takes a recorded zero and the port's zero as equal whatever their sign in
`kb_vel.x` and `kb_vel.y` only (`replay_online::ZERO_SIGN_FREE`), counts them
(`zeros_of_the_other_sign`) and shows the recorded one in a divergence
report so the field named is a real difference. `--strict-zeros` compares
them by sign (the 13 games then stop where they did). Nothing follows from
the sign in this path: the angle is atan2f(+-0, x > 0) and its sine is +0
either way. Not modelled: the same JIT's unfused multiply-add (two double
operations where the Gekko fuses), which can differ in the last bit of a
result; no compared field of these games shows it. Emulating that machine
would mean a second arithmetic in `gekko-math`; it is not done.

### Retail evidence

A netplay session cannot be run in the recorder: InitOnlinePlay loads the
match from Slippi Dolphin's EXI device (`FN_LoadMatchState`), the rollback's
savestates are that device's too, and this project's Dolphin has neither. So
the game is played back locally with the transport's effects supplied:

- the one netplay code that changes these matches and stands alone, as
  Gecko text: `online-offscreen-damage` (the C206A880 block of
  `Output/Netplay/GALE01r2.ini` at 0e90140, in `MELEE_GECKO_DIR`; a scenario
  says `online_offscreen_damage = true` beside it), with `ucf-0.74`,
  `neutral-spawn` and `widescreen`;
- the seed: a scenario's `online_rng_offset = N` makes the tracer store
  `rotl(scene frame counter, 16) + N` before every tick
  (`harness/dolphin/tick_trace.py`, `sync_seed`: at the tick boundary,
  where nothing draws before the netplay proc would run; it checks that the
  tick then ran on the counter it seeded for), `validate_ticks.py` reaches
  each record's seed from that seed, and the port's tick driver makes the
  same write (`Simulation::sync_seed_each_tick`);
- ports 1 and 2 with spawn points 0 and 1 (`make_boundary.py --spawn-points
  0 1`), eight minutes.

Not reproduced: a rollback's loop (retail renders after every tick here; the
bridge prints how many of the scenario's ticks one ran).

| Scenario | Retail |
|---|---|
| `start_bf_online37_wide_p12_drmario4_marth2_4` (boundary) | the layout with the four codes: its cold twin gates exact over 600 ticks |
| `slp_online_bf_drmario_marth_r1_t13611` (`SLIPPI_REPLAY_WITNESSES`) | `DOC/Game_20200721T200318.slp` (Slippi 3.7.0, Dr. Mario vs Marth, Battlefield) played back whole, 13611 ticks, a display pass after every tick. The port equals retail on every tick, seed included (62 keys, 0 divergences). `replay_vs_trace.py --online-rng-offset 39851`: retail follows the replay on all 13610 frames (Pre Frame seeds, actions, positions, percents), so retail under these codes and seeds plays the online game. Without `online_offscreen_damage` the port leaves retail at record 1782 (`p1.percent`, a magnifier point the code does not give); without `online_rng_offset` at record 1 |

`start_bf_online37_p12_drmario4_marth2_4` (the same boundary without
Widescreen) is local data, exact too and unused: the replay's player ran
Widescreen and the bridge insists on the replay's codes.

### The 15 games

`replay-batch` at `--jobs 4`, before and after (the runner as committed):

| Game (`public-v3.7-b`) | Stage | Before | After |
|---|---|---|---|
| `DOC/Game_20200721T200318` | BF | refused | 13610 / 13610 |
| `FOX/Game_20200801T193753` | FD | refused | 2722 / 2722 |
| `GANONDORF/Game_20200808T002332` | FoD | refused | 10947 / 10947 |
| `JIGGLYPUFF/Game_20200719T184251` | FD | refused | 7955 / 7955 |
| `LINK/Game_20200725T232021` | YS | refused | 1070 / 13543, then `p1.cur_anim_frame` |
| `LUIGI/Game_20200727T190741` | PS | refused | 11225 / 11225 |
| `MARTH/Game_20200817T043324` | BF | refused | 14423 / 14423 |
| `PEACH/Game_20200808T225408` | FD | refused | 19879 / 19879 |
| `PICHU/Game_20200808T202714` | PS | refused | 16687 / 16687 |
| `PIKACHU/Game_20200729T213607` | FoD | refused | 2918 / 2918 |
| `ROY/Game_20200711T210029` (3.6.0) | FD | setup seed | 13325 / 13325 |
| `ROY/Game_20200822T235500` | PS | refused | 6706 / 6706 |
| `YLINK/Game_20200722T224303` | BF | refused | 1938 / 9969, then `p1.cur_anim_frame` |
| `YLINK/Game_20200801T003748` | BF | refused | 18382 / 18382 |
| `YOSHI/Game_20200801T185053` | YS | refused | 7959 / 7959 |

The two stops are one port fault and not Online's: Marth's Counter attack
(SpecialLwHit, action 370) against a Link or Young Link projectile. In
`YLINK/0722` the recorded slash stops on its frame 6 from tick 1937
(hitlag: it struck something, with Young Link forty units away) and the
port's runs on; in `LINK/0725` Marth, his counter finished, takes 3% at
tick 1070 with Link shielding sixty units away and the port has no hit.
Both stop on the same tick with the rollback ticks rendered, skipped or
looped and on either screen. Not diagnosed.

**What each piece is shown by** (the 15 games run again with one piece
changed, the knockback zeros left free):

| Piece | Changed to | Games that then stop |
|---|---|---|
| Off-screen rule | the magnifier | 10, on `percent` (1319 to 8642 frames in) |
| Stage code | none | the three Stadium games (4238 to 4378: the transformation) and one Final Destination game (`ROY/0711`, 4000, `input_seed`); the other three Final Destination games match either way |
| Screen | standard | none: nothing recorded separates the screens here |
| Rollback ticks | a display pass, or none at all | none: nothing recorded separates the three |

So the rollback loop's model is from the source alone: no game of this corpus
shows it, and retail cannot run it. A recorded field that depends on it (a
tether's rope, a screen KO's position, the particle sort) would.

Both corpora, `replay-batch`: `public-v3.7-b` 458 to 471 complete of 502
(frames matched 4,838,502 to 4,988,248); the other 487 games' JSONL lines
are identical apart from the new `online` field, null on each; no replay
matches fewer frames and none changes an inferred setup. `public-v3.7`: all
108 lines identical apart from that field.

`~/melee-data/replays` holds no replays outside the two sampled corpora, so
there is no further online game on this machine; the three online fixtures
in `crates/slp/tests/data` are the only others (`netplay.slp`, 2020, now
128 / 128; the two later ones refused for their codes).
