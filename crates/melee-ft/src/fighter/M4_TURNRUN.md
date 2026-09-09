# M4-T7: Fox running reversal, fast walk and ledge options on FD

Implemented and verified for the four supplied scenes; no commit. TurnRun and
WalkFast each match **300 ticks, 49 keys, zero divergences**. CliffClimbQuick
and CliffEscapeQuick each match **420 ticks, 49 keys, zero divergences**.
Raw fighter replays and ordered particle RNG comparisons also pass.

## Port

- `turn_run.rs` owns reversed-stick entry, typed entry-facing/pause scratch,
  animation freezing until the old-direction momentum is exhausted, facing
  reversal, acceleration/friction, edge stopping and the exit to Run or Wait.
  Run supplies frame zero; RunBrake retains its animation phase. TurnRun entry
  retains momentum (`Ft_MF_SkipAnimVel`). The subsequent Run interrupt delay
  comes from PlCo +430, with no scenario-derived constants.
- WalkFast already matched its first retail scenario. Its existing shared
  tier selection, animation phase/rate conversion and physics needed no fix.
- Climb and escape share enum callbacks and the retained `MotionData::Cliff`.
  Entry advances the animation, sets grab exclusions and nudge suppression;
  the first physics callback snaps the stepped TransN before ground detection.
  Airborne physics follows the map endpoint until TransN Y and Z are both
  nonnegative. Grounded physics then follows TransN displacement and uses
  edge-stopping collision. Animation completion returns through Wait/Fall;
  ordinary Wait IASA can enter WalkSlow on the same tick.
- Common ground conversion now extracts root-motion X velocity. Retail clamps
  the *old* ground velocity and then overwrites it from self velocity; it does
  not clamp the incoming self velocity. The previous helper incorrectly clamped
  self velocity. Vertical self velocity remains until the next ground physics
  callback, as before.
- Subaction `HurtStatus` and the independent timed ledge intangibility already
  provide typed vulnerability state. New `GrabExclusions` preserves the full
  grab-category mask: 0x1FF during catch/wait, 0x20 during ledge options, and
  reset on motion entry. The raw replay covers both vulnerability sources,
  grab masks, nudge suppression, TransN position/displacement, floor index,
  ledge scratch, TurnRun facing/pause scratch and existing movement fields.
- Asset loading, motion tables and scheduler dispatch remain in their existing
  layers. Effect dispatch stays in melee-sim and particle arithmetic stays in
  hsd-particle. Neither needed a behavior change for these scenes.

These retail state bodies contain no character-kind branches. Existing
`CharacterCallbacks` defaults remain the character boundary; no character
crate import, character-kind match, new dependency or runtime trace access was
introduced. Raw replay inputs remain the initial boundary, recorded pads and
external pre-draw seeds; the complete scene independently produces RNG.
No expected values, comparison tolerances or field exclusions changed.

## Observed transitions and corrections

| Scene | Retail transitions (tick: motion) |
|---|---|
| turnrun | 31:20 Dash, 42:21 Run, 61:19 TurnRun, 91:21 Run, 101:23 RunBrake, 119:14 Wait |
| walkfast | 31:15 WalkSlow, 41:16 WalkMiddle, 54:17 WalkFast, 76:14 Wait |
| ledgeclimb | 70:252 CliffCatch, 77:253 CliffWait, 232:255 CliffClimbQuick, 266:15 WalkSlow, 272:14 Wait |
| ledgeescape | 70:252 CliffCatch, 77:253 CliffWait, 232:259 CliffEscapeQuick, 281:14 Wait |

TurnRun starts at x=5.6400118. It pauses at animation frame 9 on tick 70,
resumes and flips facing on tick 80, then enters Run on tick 91. Climb becomes
grounded on tick 254; escape on tick 252. Their subaction intangibility ends
on ticks 262 and 266, respectively. Escape finishes at x=-50.719238.

The supplied ticks, motion IDs and dust counts agree with the recordings.
**Quick versus Slow is selected by damage percentage**, compared with PlCo
+488 (`ftCo_CliffClimb.c:76-78`, `ftCo_CliffEscape.c:17-19`), not by hang
duration. Hang duration controls the separate CliffWait timeout. Quick uses
submotions **220/224**; 219/223 are Slow. Selecting the Slow descriptors was
the first implementation mismatch, corrected from the retail enum table.

## Effects and RNG

| Scene | Total ledger draws | Randomized dust ticks (three offset draws each) |
|---|---:|---|
| turnrun | 9,482 | 34, 49, 58, 61, 70, 101 |
| walkfast | 9,215 | none |
| ledgeclimb | 13,031 | 258 |
| ledgeescape | 13,169 | 261 |

All particle sites match in order on every recorded tick, and each complete
scene matches the final seed on every tick. Existing dust dispatch and audited
particle sites cover all requests. No new particle opcode, generator shape,
RNG site, captured transform or runtime schedule was added.

No particle field dumps were supplied for these four scenes. The evidence is
full scene traces, raw fighter fields and ordered RNG, plus passing existing
particle-field regressions. **Would you like field-level particle verification?
If so, please provide dumps for `turnrun_fd_fox`, `ledgeclimb_fd_fox` and
`ledgeescape_fd_fox`.** WalkFast adds no particle effect. No Dolphin was run.

## Retail audit

Commands ran read-only in `harness` with `UV_CACHE_DIR=/tmp/melee-uv-cache`.
Full audit output: `/tmp/melee-t7-fused.log`; selected complete disassembly:
`/tmp/melee-t7-asm.log`. Addresses were resolved with `symbols.py`.

```sh
uv run python asm.py ftCo_TurnRun_Enter ftCo_TurnRun_Anim ftCo_TurnRun_Phys ftCo_TurnRun_Coll ftCo_8009AB9C ftCo_CliffClimb_Phys ftCo_CliffClimb_Coll ftCo_8009B040 ftCommon_8007D7FC ftCommon_8007D92C ft_80084FA8 --fused
uv run python asm.py ft_80085030 ftCo_8009AAFC fn_800CA644 --fused
uv run python asm.py ftCo_TurnRun_Enter ftCo_TurnRun_Anim ftCo_TurnRun_Phys ftCo_CliffClimb_Phys ftCommon_8007D6A4
```

| Arithmetic | Retail instruction |
|---|---|
| TurnRun initial stick acceleration | 800C9F28 fmuls, then 800C9F44 fadds; separate rounding |
| TurnRun positive-acceleration correction | **800C9FA4 fnmsubs**, acceleration minus friction times multiplier |
| TurnRun negative-acceleration correction | **800C9FD8 fmadds** |
| Ledge X snap | **8009AD1C fmadds**, identical operation to existing 800815A8 |
| Ledge Y snap | 8009AD2C fadds |
| Grounded TransN acceleration | **8008505C fmsubs**, facing times displacement minus ground velocity |
| Ground conversion | 8007D6CC fmuls; ground velocity overwritten at 8007D708 |

Entry, animation, input predicates and collision add no fused arithmetic.
Previously audited walk, integration and particle kernels are reused.

## Validation

All commands used the local assets. Baseline workspace gate: **588 passed,
zero failed, one pre-existing ignored doctest**. Final results are recorded
below; logs use `/tmp/melee-t7-final-<name>.log`.

| Command | Result |
|---|---|
| `cargo run -q -p melee-sim -- gate harness/scenarios/turnrun_fd_fox.toml` | 300 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/walkfast_fd_fox.toml` | 300 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/ledgeclimb_fd_fox.toml` | 420 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/ledgeescape_fd_fox.toml` | 420 ticks, 49 keys, 0 divergences |
| `cargo test -p melee-sim --test m4_gate` | 27 passed |
| `cargo test -p melee-ft --test movement_fox_states` | 17 passed |
| `cargo run -q -p melee-sim -- gate harness/scenarios/idle_fd_fox.toml` | 600 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/start_fd_fox.toml` | 600 ticks, 49 keys, 0 divergences |
| `cargo test -p melee-ft --test start_fox_bones_130` | 4 passed |
| `cargo test -p hsd-particle` | 58 passed, including all live_fd replays |
| `cargo test -p melee-sim --test m2_gate` | 1 passed |
| `cargo gate` | 600 passed, 0 failed, 1 pre-existing ignored doctest |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean (`/tmp/melee-t7-clippy.log`) |
| `cargo fmt --all` | success |
| `git -c core.fsmonitor=false diff --check` | success |

## Limits

Slow climb/escape, C-stick ledge options, CliffAttack, the long-hang timeout,
running jump cancels, occupied-ledge arbitration, ceilings, items and status
effects remain explicit unsupported paths. The two same-body Slow descriptors
are deliberately not loaded. No particle-field parity is claimed for the new
scenes without their dumps. Rendering, audio and controller playback remain
outside this slice. No requested check was skipped; harness tests are not
needed because no harness code changed.

No protected traces, scenarios, ROMs or decomp files were modified. No game
data was added and nothing was committed.

## Files changed

- `TRACKER.md`
- `crates/melee-ft/src/fighter/{assets,dash,landing,ledge,mod,procs,run,spawn,state,turn,turn_run}.rs`
- `crates/melee-ft/src/fighter/{README.md,M4_TURNRUN.md}`
- `crates/melee-ft/tests/fighter_support/{mod,replay}.rs`
- `crates/melee-ft/tests/movement_fox_states.rs`
- `crates/melee-sim/src/initial_state/fighter.rs`
- `crates/melee-sim/tests/m4_gate.rs`
