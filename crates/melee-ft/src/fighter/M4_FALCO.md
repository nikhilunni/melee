# Lane C: Falco idle, match start and movement

Implemented and verified, left uncommitted. All eighteen supplied Falco scenes
match every tick and all 49 keys, including the produced RNG seed. All eighteen
ordered particle RNG ledgers also match. Data and capture corrections are in
[FALCO_DATA.md](../../../../docs/FALCO_DATA.md).

## Changes and character boundaries

`ft-falco` owns `ftDataFalco` loading, the four costume descriptors, animation
and part counts, OnLoad/OnDeath state and capabilities. Its complete special
attributes use the existing typed Fox-layout reader, moved into `melee-ft`.
Both character crates select their own archive root; neither depends on the
other. Existing Fox reader tests retain every expected value and comparison.

Falco uses the existing Basic double jump and ordinary guard/escape defaults.
No new `CharacterCallbacks` method or Falco kind check was needed. Walljump
capability and all four special capabilities come from OnLoad and ftdata.c;
actual unsupported walljump, special-move, combat and item transitions remain
explicit boundaries. Falco's dynamic set is empty, loaded directly from data.

Three assumptions in the shared code were exposed:

| Location | Correction |
| --- | --- |
| `melee-sim/src/frame.rs` | A second explicit Fox/Marth match remained despite the single-list contract. It now uses the existing `with_fighter!` macro. Falco registration itself is one line in `scene_fighter.rs` plus its crate dependency. |
| `melee-ft/fighter/effects.rs`, `melee-sim/effects.rs` | Falco's jump animation requests `0x3F7`. Retail routes this through async kind 6 to common model effect `0x12`, with facing and floor rotation. Its animation emits existing particle 9. The existing descriptor evaluator and particle engine handle it unchanged. |
| `melee-ft/fighter/spawn.rs`, both effect modules | Motion changes must flush the queued effects using the outgoing pose. Previously every deferred request waited until scheduler link 9, which reversed requests across a transition. |

## Wavedash root cause and exactness

Before the queue fix, `wavedash_fd_falco` matched 126 ticks, then tick 126
first differed at `rng.seed`: expected `810E6E59`, actual `586388CF`.
The ordered ledger independently showed the jump-flash and special-landing
particle generators traversed in the wrong order. This is the second jump
attempt: Falco goes from KneeBend through jump/air dodge into
LandingFallSpecial within the same tick.

`Fighter_ChangeMotionState` (`fighter.c:950-951`, retail **800694A0**)
sets the root translation and calls `efAsync_QueueFlush` before installing
the next pose. The shared fighter now seals each pending queue in retail
reverse insertion order and captures its outgoing joint transforms. The scene
consumes that batch at the owning proc boundary, retaining its order relative
to synchronous destruction and later deferred requests. It never samples the
replacement pose for a flushed positional effect. No tick, character kind,
ledger seed or expected result selects this behavior.

The unchanged audited offset path consumes three draws even for zero ranges.
The `0x3F7` path uses `ftCo_8009F834` block_70: **8009FCF8, 8009FD1C,
8009FD44 fmadds**, with separately rounded range doubling and random centering.
No new floating-point arithmetic or particle interpreter branch was introduced.
The existing model rotation and matrix helpers retain their retail semantics.

Read-only assembly commands used:

```sh
python3 harness/asm.py ftCo_8009F834 --fused
python3 harness/asm.py efAsync_Dispatch --fused
python3 harness/asm.py Fighter_ChangeMotionState
python3 harness/asm.py hsd_8039D9C8
```

The focused regression is the full 300-tick wavedash gate plus its independent
ordered ledger; the other seventeen Falco gates and every Fox/Marth gate
exercise the shared path. All pre-existing particle-field oracle tests pass.

## Capture corrections and boundaries

- There is no `ftFalco/types.h` in the pinned decomp. Falco includes
  `ftFox/types.h` and calls `ftFx_Init_OnLoadForFalco` to copy that layout.
- Falco has 67 joints and zero dynamic chains/colliders; no dynamics override.
- Falco's start sequence matches 0:322, 6:323, 35:324, 65:29, 77:42, 107:14.
  P2 Fox in this capture lands at tick 80, not 75, after staggered entry.
- The re-recorded `airjumpb_fd_falco` trace exists and passes.
- Idle adds no RNG sites beyond the idle set. Start still uses the existing
  unlock-dependent music selection; no new scene setup logic was needed.
- New Falco particle dumps were not separately replayed field-by-field.
  Their ordered particle RNG ledgers are gated, as are all existing dump tests.
- The bone tests compare tick-boundary local SRT, not render-time matrix caches.
  Euler W is excluded only when quaternion mode is disabled, matching Marth.

## Validation

All local assets were present; the oracle tests actually ran without skips.
The initial workspace run completed its executable tests, but its doctest
phase overlapped the reader move and failed on stale dependency artifacts.
It is not claimed as a clean baseline. The final complete `cargo gate` passed.

| Command | Result |
| --- | --- |
| `cargo gate` | 685 passed, 0 failed, 1 pre-existing ignored doctest |
| `cargo test -p melee-sim --test m4_gate` | 101 passed, 0 failed |
| `cargo test -p hsd-particle` | 58 passed, 0 failed |
| `cargo test -p melee-ft` | 89 passed, 0 failed |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean; finished dev profile, exit 0 |
| `cargo fmt --all` | success |

Bone oracle command:

```sh
cargo test -p melee-sim --lib falco_bones -- --nocapture
```

Final oracle lines:

```text
start_fd_falco: 67 Falco bones (0 dynamic), 73 Fox bones, 130 ticks, 164080 SRT words, 0 matrix words (no rendered capture)
idle_fd_falco: 67 Falco bones (0 dynamic), 73 Fox bones, 8 ticks, 10080 SRT words, 0 matrix words (no rendered capture)
```

The eighteen CLI gate commands and their final lines are listed below.

| Exact command | Final line |
| --- | --- |
| `cargo run -q -p melee-sim -- gate harness/scenarios/idle_fd_falco.toml` | 600 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/start_fd_falco.toml` | 600 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/squat_fd_falco.toml` | 300 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/turn_fd_falco.toml` | 300 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/walk_fd_falco.toml` | 300 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/walkfast_fd_falco.toml` | 300 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/dash_fd_falco.toml` | 300 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/turnrun_fd_falco.toml` | 300 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/jump_fd_falco.toml` | 300 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/airjumpb_fd_falco.toml` | 300 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/shield_fd_falco.toml` | 300 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/spotdodge_fd_falco.toml` | 300 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/roll_fd_falco.toml` | 300 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/airdodge_fd_falco.toml` | 300 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/wavedash_fd_falco.toml` | 300 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/ledge_fd_falco.toml` | 420 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/ledgeclimb_fd_falco.toml` | 420 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/ledgeescape_fd_falco.toml` | 420 ticks, 49 keys, 0 divergences |

Logs: `/tmp/falco-gate.log`, `/tmp/falco-acceptance-{0..3}.log`,
`/tmp/falco-cli.log`, `/tmp/falco-bones.log`.
No harness code changed, so harness Python tests were not run.
No expected bits, tests or comparisons were weakened. No changes to traces,
ROMs, scenarios or the decomp submodule; no Dolphin invocation or commits.

## Changed files

- `Cargo.toml`, `Cargo.lock`, `TRACKER.md`
- `crates/ft-falco/Cargo.toml`
- `crates/ft-falco/src/{lib,attributes,init}.rs`
- `crates/ft-falco/tests/attributes.rs`
- `crates/ft-fox/src/{attributes,init}.rs`
- `crates/ft-fox/tests/attributes.rs`
- `crates/melee-ft/src/desc/{mod,fox_attributes}.rs`
- `crates/melee-ft/src/fighter/{effects,spawn}.rs`
- `crates/melee-ft/src/fighter/{README,M4_FALCO}.md`
- `crates/melee-sim/Cargo.toml`
- `crates/melee-sim/src/{effects,frame,scene_fighter}.rs`
- `crates/melee-sim/src/frame/falco_bones.rs`
- `crates/melee-sim/tests/m4_gate.rs`
- `docs/FALCO_DATA.md`
