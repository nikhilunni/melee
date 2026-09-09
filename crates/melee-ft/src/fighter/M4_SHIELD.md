# M4-T4: Fox shield, spot dodge and roll on Final Destination

Implemented for the supplied no-hit scenarios; left uncommitted as requested.
Each scene matches **300 ticks, 49 keys, 0 divergences**. The callback replays
also compare raw shield scratch/flags and dodge hurt status; all three ordered
particle RNG ledgers match. No particle dump exists for these scenes, so this
is not a claim of independently verified particle-field or shielding-bone parity.

## Port

- GuardOn/Guard/GuardOff/GuardSetOff/GuardReflect callback tables, typed
  `MotionData::Guard`, startup windows, minimum hold, release latch, lightshield,
  shield health drain/regeneration, smoothed shield tilt and shield size.
- Owned shield and reflect collision volumes, enable/cache flags and enum hit
  callbacks. Shield pose comes from the neutral descriptor and tilt animation;
  its joint scale is simulation/model state. SkipAnim clears AObjs and command
  playback while retaining the Guard scratch required by later transitions.
- EscapeF/EscapeB/EscapeN tables, typed escape timer and entry facing,
  subaction-controlled normal/invincible/intangible hurt status, reversal flag,
  interrupt flag, stage-edge collision and the fighter-nudge suppression flag.
  Roll acceleration comes from animation TransN displacement and retained entry
  facing, through `ft_80085004` / `ft_80085030`.
- Immediate shield-effect requests dispatch at their fighter callback, while
  deferred graphics retain the existing link-9 queue. Shield effects follow
  the live shield joint and inherit world Y scale on effect update. Descriptor
  AppSRT allocation, attached transform refresh and shared particle ownership
  remain in `hsd-particle`; effect dispatch stays in `melee-sim`.
- Callback replays, three full-scene gates and ordered particle RNG tests;
  synthetic AppSRT lifecycle coverage checks link masking, shared transform
  updates and removal with the owning joint.

`CharacterCallbacks::guard_variant` owns the Yoshi egg-shield and Marth model
branches (`ftCo_Guard.c:335-350,917-934`). `escape_variant(rolling)` owns Samus
roll and Yoshi roll/spot-dodge branches (`ftCo_Escape.c:78-94,228-241`). Fox
uses the default ordinary behavior; the named non-Fox bodies fail explicitly
inside those trait defaults. There is no character-kind match in the shared
Guard/Escape implementation.

## Corrections and effect routing

| Scene | Observed motion sequence (tick: retail motion) |
|---|---|
| shield | 31:182 GuardReflect, 39:179 Guard, 91:180 GuardOff, 106:14 Wait |
| spotdodge | 31:182 GuardReflect, 35:235 EscapeN, 57:178 GuardOn, 65:179 Guard, 81:180 GuardOff, 96:14 Wait |
| roll | 31:182 GuardReflect, 35:233 EscapeF, 66:178 GuardOn, 74:179 Guard, 81:180 GuardOff, 96:14 Wait |

The task's labels for **178 and 182 were reversed in meaning**: 178 is GuardOn,
181 is GuardSetOff (shield stun), and 182 is GuardReflect. Thus the recordings
require digital startup reflect-window bookkeeping but do not exercise shield
stun or a successful powershield hit. The roll ends at approximately x=-26.4.
There is no independent roll velocity table in this path: the animation's
TransN track supplies the displacement. Also, the decomp's `bool` escape entry
argument carries the five-frame PlCo.x324 item-throw timer in retail; Rust
stores and decrements an integer instead of converting it to a boolean.

| Request | Dispatch |
|---|---|
| shield startup 0x417 | synchronous effect-table 0xB, including particle 45 (0x2D) with attached AppSRT |
| shield hold 0x418 | synchronous effect-table 0xC |
| spot-dodge dust 0x407 | existing randomized fighter-offset path, particle 60 (0x3C) |
| roll dust 0x3F3 | existing randomized fighter-offset path, particle 11 (0x0B) |

Each dust request consumes the existing three `ftCo_8009F834` offset draws.
The emitter/interpreter sites were already implemented for dash; shield adds
AppSRT creation and mutable joint attachment, not a guessed emitter shape.
The synthetic scale test initially assumed ideal (2,3,4) recovery. With the
user's explicit approval, it now asserts the retail-rounded bits
`3FFFFFFF/403FFFFF/407FFFFF` without tolerances: HSD vector-length recovery
rounds each value one ULP below the corresponding input scale. The existing
unsupported-interaction test now exercises a real shield entry and its proc
instead of demanding a panic for this newly supported feature; all remaining
unsupported interactions and the invalid installed callback stay checked.

## Validation

All commands ran from the repository root and exited zero, with local assets present.
Logs: `/tmp/m4-shield-final-gate.log` and `/tmp/m4-shield-final-00.log` through
`/tmp/m4-shield-final-13.log`.

| Exact command | Final nonblank line |
|---|---|
| `cargo run -q -p melee-sim -- gate harness/scenarios/shield_fd_fox.toml` | 300 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/spotdodge_fd_fox.toml` | 300 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/roll_fd_fox.toml` | 300 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/idle_fd_fox.toml` | 600 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/start_fd_fox.toml` | 600 ticks, 49 keys, 0 divergences |
| `cargo test -p melee-sim --test m4_gate` | test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.20s |
| `cargo test -p melee-ft --test movement_fox_states` | test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.61s |
| `cargo test -p melee-ft --test start_fox_bones_130` | test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.82s |
| `cargo test -p hsd-particle --test live_fd --test live_fd_dash` | test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.04s |
| `cargo test -p melee-sim --test m2_gate` | test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s |
| `cargo test -p hsd-particle --test start_paths` | test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s |
| `cargo gate` | test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s |
| `cargo clippy --workspace --all-targets -- -D warnings` | Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 10.57s |
| `cargo fmt --all` | success, no output |
| `git -c core.fsmonitor=false diff --check` | success, no output |

Final workspace total: **572 passed, zero failed, 1 pre-existing ignored doctest**,
across 102 suite result lines. The last `cargo gate` line is the final
crate's empty doctest target. The combined particle command passes both
`live_fd` (two tests) and `live_fd_dash` (one test); the table records the latter
target's final line. Existing dash display-cache exclusions remain unchanged,
as documented in `M4_JUMP.md`; no retail expected data or tolerance changed.


## Fusion audit

Read-only commands ran from `harness` with `UV_CACHE_DIR=/tmp/melee-uv-cache`.
Full instructions were also inspected for arithmetic order and callback setup.

```sh
uv run python asm.py ftCo_80091BC4 ftCo_80091D58 ftCo_80091E78 ftCo_800925A4 Fighter_ProcessHit_8006D1EC ft_80085030 lb_8000D008 lb_8000C868 --fused
uv run python asm.py ftCo_800921DC ftCo_80093A50 ftCo_80093BC0 ftCo_80099314 ftCo_800998EC ft_80085030 ftCo_80091E78 --fused
uv run python asm.py ftCo_8009917C ftCo_800992A8 ftCo_80099314 ftCo_8009980C ftCo_800998EC ftCo_Escape_Anim ftCo_EscapeN_Anim ftCo_80091A4C ftCo_800924C0 ftCo_GuardOn_Anim ftCo_Guard_Anim ftCo_GuardOff_Anim ftCo_GuardSetOff_Anim ftCo_GuardReflect_Anim ftCo_80092908 ftCo_8009370C ftCo_80093A50 ftCo_80093BC0 --fused
uv run python asm.py lb_8000D008 lb_8000C868 lb_8000B4FC ftAnim_80070010 ftAnim_80070108 HSD_MtxGetScale --fused
uv run python asm.py efLib_Update efLib_SpawnParticleEffect hsd_8039D214 hsd_8039DAD4 hsd_8039930C hsd_8039F05C ftCo_8009F834 --fused
uv run python asm.py ftCo_80091BC4 ftCo_80091D58 ftCo_800925A4
uv run python asm.py ftAction_80071A14 ftAction_800718A4
```

| Operation | Retail arithmetic |
|---|---|
| Shield tilt angle / magnitude smoothing | `80091C78` / `80091D3C` fmadds; squared magnitude uses separate products/add at `80091CB4/CBC/CC0`; sqrt refinement fnmsub at `80091CE0/CF0/D00` |
| Shield size | fmadds at `80091DAC/DB4`; inlined pose copies at `80092014/1C` and GuardSetOff at `80093550/558` |
| Shield drain | `80092624` fmadds, then separately rounded product and subtraction |
| Guard pose interpolation | existing Euler blend order, fmadds at `8000C8A8/C8BC/C8D0/C8E4/C8F8/C90C` |
| Roll root-motion acceleration | `8008505C` fmsubs with retained entry facing |
| Dust randomized offset | existing block-70 fmadds at `8009FCF8/FD1C/FD44`; doubling and random-minus-half round separately |
| Stick angle, lightshield division, alpha, entry/window callbacks, effect update/attachment | no new fused sites; stick-angle negative-X quadrant retains double pi |

The full per-frame hit proc has fused hit-damage sites at `8006D2AC/2CC`;
those nonzero-damage branches remain explicitly M5. Particle fusion remains
in the existing audited implementation described by `docs/PARTICLES.md`.
The main math-audit final line is `8000C90C EC02007A fmadds f0, f2, f1, f0`;
the effect-audit final instruction is `8009FD44 EC02007A fmadds f0, f2, f1, f0`.

## Limits

Shield hit dispatch/damage/stun entry (`ftCo_Guard.c:630-650,661-713`), successful
powershield response (`875-882`), delayed powershield activation (`885-915`),
shield break (`428-436`, `fighter.c:2837-2843`) and grab out of shield
(`ftCo_Catch.c:30-34,87-90`) remain explicit `unimplemented!` boundaries.
GuardSetOff callback bodies and typed collision state exist, but these no-hit
captures never enter that state. Non-Fox trait branches, held items, C-stick
shield jump, shield platform drop and older combat/ledge boundaries remain
explicitly unsupported. EscapeB is wired but has no supplied retail replay.

Please record particle dumps for `shield_fd_fox`, `spotdodge_fd_fox` and
`roll_fd_fox` to independently verify particle/AppSRT fields. A shielding bone
capture would also verify the shield pose beyond the raw scratch comparisons.
The current evidence is exact 49-key traces, raw callback state and ordered
RNG draws, plus the existing idle/start/dash particle-field regressions.
Rendering and audio playback are outside the headless simulation.

No Dolphin runs, commits or changes under `harness/traces`, `harness/roms`,
`harness/scenarios` or the decomp submodule. Harness tests were not run because
no harness code changed.

## Files touched

- `TRACKER.md`
- `crates/hsd-particle/src/generator.rs`
- `crates/hsd-particle/src/system.rs`
- `crates/hsd-particle/tests/start_paths.rs`
- `crates/melee-ft/src/anim/playback.rs`
- `crates/melee-ft/src/collision/ground.rs`
- `crates/melee-ft/src/fighter/M4_SHIELD.md`
- `crates/melee-ft/src/fighter/README.md`
- `crates/melee-ft/src/fighter/assets.rs`
- `crates/melee-ft/src/fighter/commands.rs`
- `crates/melee-ft/src/fighter/effects.rs`
- `crates/melee-ft/src/fighter/escape.rs`
- `crates/melee-ft/src/fighter/mod.rs`
- `crates/melee-ft/src/fighter/procs.rs`
- `crates/melee-ft/src/fighter/shield.rs`
- `crates/melee-ft/src/fighter/spawn.rs`
- `crates/melee-ft/src/fighter/state.rs`
- `crates/melee-ft/src/fighter/walk.rs`
- `crates/melee-ft/tests/fighter_support/replay.rs`
- `crates/melee-ft/tests/fox_spawn_native.rs`
- `crates/melee-ft/tests/movement_fox_states.rs`
- `crates/melee-lb/src/trigf.rs`
- `crates/melee-sim/src/effects.rs`
- `crates/melee-sim/src/effects/dust.rs`
- `crates/melee-sim/src/frame.rs`
- `crates/melee-sim/tests/m4_gate.rs`

## M4-T4 particle-dump follow-up (2026-09-09)

The newly supplied tick-boundary dumps now pass the shared full-field
`hsd-particle/tests/support/dust_replay.rs` runner. Each fixture was logged
from the production `melee-sim` scenario gate, including spawn requests,
attachment updates, destruction and the shield caller's generator flag
operation. See `hsd-particle/tests/data/README.md` for reproduction. No
simulation outputs are fixtures, and no runtime routing or shield attachment
math needed changing.

| Test | Ticks | Compared fields | Ordered particle draws | AppSRT display-cache fields excluded |
|---|---:|---:|---:|---:|
| `live_fd_shield` | 300 | 446,320 | 9,327 | 224 |
| `live_fd_spotdodge` | 300 | 447,906 | 9,059 | 336 |
| `live_fd_roll` | 300 | 468,444 | 9,659 | 336 |

The first mismatch in all three scenes was at **tick 31,
`particles.appsrt[0].generator_index`: expected `UInt(1)`, actual `Null`**.
It occurred 8 times for shield and 12 times each for spot dodge and roll.
Every other compared field matched on the first replay. The snapshot adapter
had unconditionally emitted null for this field, even though the port retains
the allocating generator's identity as the shared AppSRT ID.

The adapter now normalizes the live allocating generator for descriptor kind
0x20000. Retail `hsd_8039F05C` writes `AppSRT.gp` at **0x8039F698**;
`psRemoveGeneratorSRT` clears it at **0x803A4428**. Explicit effect-created
AppSRTs retain null, as before. Both assembly sites and their decomp callers
were read. This fixes actual-state serialization; no expected value or
comparison was changed. All final seeds and ordered draw sites match.

Only the existing `psDispSubAppSRT` display-cache exclusion applies, with the
counts above. The new tests print their counts under `--nocapture`. This
supersedes the earlier statement that these scenes have no particle dump.

Validation for this follow-up (local assets present, no skipped asset tests):

- Six new integration targets with `--nocapture`: all pass and print the
  counts above; `/tmp/melee-six-final.log`.
- `cargo test -p hsd-particle`: 58 passed, zero failed.
- `cargo test -p melee-sim --test m4_gate`: 19 passed, covering the 11 movement
  scenes plus ordered particle RNG checks. Idle and match-start also pass in
  `cargo gate`, completing all 13 scenes.
- `cargo gate`: 588 passed, zero failed, one pre-existing ignored doctest
  across 108 suite results; `/tmp/melee-final-gate.log`.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo fmt --all` and `git -c core.fsmonitor=false diff --check`: passed.

Temporary capture instrumentation was removed. No protected harness paths or
submodule files were changed, no Dolphin was run, and no commits were made.
