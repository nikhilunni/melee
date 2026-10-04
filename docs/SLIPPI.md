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
  No retail witness: recording one needs that build's code assembled from
  the GPL source; the whole 9638-frame replay matching is the check.
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
`melee-lib`): `Off`, `Ucf073`, `Ucf074`, `Ucf080`, `Ucf084`, and `Dween`,
which match setup refuses with the reason. Slippi does not record the UCF
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

`melee-sim replay` and `replay-batch` date the UCF version from the recording
(`ucf_by_date`) and, where the date allows two, read it from the frames
(below); `--controller-fix <off|ucf-0.73|ucf-0.74|ucf-0.8|ucf-0.84>` names
it instead for every port recorded with UCF. The report's `controller fix`
line says which version ran and why.

Not ported: Dween, and the 0.74 shield drop without the truncation that
Slippi's Dolphin lists and `g_toggles.bin` carried for one week (acc6f71,
2019-09-24, to 4062e22, 2019-10-01).

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
version does not help either (every game in this corpus says 2.0.1). So
`resolve_controller_fix` reads it from the frames, between those two known
versions only: it runs the port under both in step from the recorded inputs
(no recorded state enters either run); the two compute the same match until
a dashback they disagree on, and on that tick the recorded facing is one
version's. That version then runs the whole comparison from the first frame.
A game with no such dashback keeps the dated version (0.74 from 2019-10-09,
the console set's date), and nothing distinguishes the two for it.

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
  which its gate checks against the port's setup draws. On the other stages
  any boundary of the setup has served (the witnesses below follow their
  replays from one); `slippi_to_scenario.py` insists on the replay's own
  boundary only for these two, unless `--foreign-creation` is given.
- *Pads.* Pre Frame's joystick is the fighter's dead-zoned stick, with the
  raw X beside it (1.2+). `--retail-inputs` finds the raw Y whose clamped,
  dead-zoned pair is the recorded one, with the recorded raw X (what UCF
  reads); the tick input clock injects it.

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

Two Yoshi bridges are local probes, not witnesses: each shows a fault that is
still open.

- `slp_bf_yoshi_samus_t1000` (the same game to tick 1000) gates exact but
  differs in particle draw order at tick 870: Yoshi's Egg Throw (frame 20)
  and Samus's Run dust create a generator each on that tick (effect banks 9
  and 0), and the port creates them in the other order. The draw count is
  the same.
- `slp_ps_fox_yoshi_t1300` (`YOSHI/Fox vs Yoshi [PS] Game_20200222T144019.slp`,
  ports 1 and 4, boundary `start_ps_slippi74pre_ns_p14_fox2_yoshi4_4`) stops
  at tick 1091. Yoshi's plain GuardOn (ftYs_Init_8012BECC) does not clear the
  reflect and powershield bits (x221C_b1/b2) as ftCo_800924C0 does, nor write
  their countdowns (guard x14/x18, fp+0x2354/0x2358): both keep whatever the
  last state left in the motion scratch. Retail's memory in that trace: a
  delayed powershield (345) at 809 sets the bits with x14 = 1, x18 = 3; a hit
  at 810 leaves them set; DamageFly writes x14 = 1 (846); the next plain
  shield's stun (877) counts 1/2 down to 0/1; a later Damage writes x18 = 0.5
  and x14 = 23 (1049, 1060); so at 1090 the bits are still set and Fox's hit
  on the held egg takes the powershield branch of ftColl_80076CBC (effect 27,
  not 1052). The port starts a plain GuardOn out of any other state with both
  countdowns at zero, so the first stun clears the bits. Following retail
  needs the scratch words at +0x14 and +0x18 of every state Yoshi passes
  through (Damage, Walk and Turn write them in this trace).

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

## Consoles without Shy Guys or FD transitions (2026-10-03, not ported)

Three corpus games ran a stage that never drew what retail draws. The seed
check shows it on the tick after the first draw the console skipped; nothing
in a 2.0.1 replay names the code, and Game Start is byte-identical to the
games that do draw.

| Replay | Stops | The console never ran | Measured |
|---|---|---|---|
| `FOX/20200219 - HNC 9 - PM 0655 - Jigglypuff (Default) vs ZEN Fox (Red) - Yoshi_s Story.slp` | 121 | grStory_801E3418, the Shy Guy spawner (first group at tick 120: six draws) | with the call skipped the port matches 7314 / 7314 frames, every seed included; delayed by one, two or three ticks it stops at 121, 122, 123 |
| `FOX/01_51_21.616Z [314] Fox + [TITP] Captain Falcon (FD).slp` (UCF 0.74 forced) | 1886 | grLast_8021B2E8, the background's phase timer (first transition when it passes 1800) | with the transition never enabled: 9630 / 9630 |
| `MARTH/02_56_19 Captain Falcon + [PPAP] Marth (FD).slp` (UCF 0.74 forced) | 1887 | the same | 6854 / 6854 |

Those runs were temporary experiments, not committed. Retail does draw: the
first game's pads on retail (`slp_ys_jigglypuff_fox_t300`) spawn the group
at tick 120 and the port equals that trace; the corpus's other Yoshi's Story
games match with the Shy Guys, and its other FD games pass tick 1886 with the
transition (two tried without it, `MARTH/12_07_47` and `FOX/12_09_22`, stop
at 1886). So the seed check separates the two kinds of console on one tick.

The sites are single calls: `bl grStory_801E3418` at 0x801E3348 in
grStory_801E3334, and `bl grLast_8021B2E8` at 0x8021AAE4 in
grLast_8021AAB0. Tournament code sets of the time carried stage codes that
replace such a call with a `nop` ("disable Shy Guys", "disable Final
Destination background transitions"); the exact code text those consoles
ran has not been checked against a source. Porting them needs the code text
in `MELEE_GECKO_DIR`, a retail witness recorded with each, a flag beside
`stadium_frozen` in `SlippiCodes`, and a rule for choosing it per replay (a
runner option like `--controller-fix`, or the first seed the code changes).
