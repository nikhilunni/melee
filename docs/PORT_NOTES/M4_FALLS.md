# M4-T9: Fox and Marth falls and Marth shield entry on FD

Implemented and verified; left uncommitted. All sixteen Marth movement scenes
and Fox's backward aerial jump are registered in `m4_gate`, with matching
ordered particle RNG ledgers. The nine requested CLI gates match every tick
and all 49 keys. Existing scene, bone and particle gates remain unchanged.

## Port

- `FallState` retains an enum animation family, selected secondary pose and
  smoothed blend. `ftCo_Fall_Anim_Inner` normalizes horizontal velocity by the
  character's drift maximum, clamps it, selects direction relative to facing,
  and smooths the blend using PlCo +444/+448. The primary animation, action
  state and command stream continue while the secondary pose changes.
- Secondary attachment resets the interpolation skeleton and installs the
  archive's FigaTree at the current main frame, with its loop flag and rate.
  A changed selection advances and blends immediately; `ftCo_800CC988` then
  advances and blends again on that same tick. Existing audited quaternion/SRT
  interpolation supplies both partial blending and full-copy behavior.
- JumpAerial completion enters FallAerial (32), clears fast fall and initializes
  the aerial blend. Ordinary Fall preserves fast fall and clamps drift on
  entry. Both families reuse the ordinary aerial physics and landing/ledge
  collision callbacks; FallAerial's retail collision wrapper is in `ftswing.c`.
- Air dodge completion enters FallSpecial (35), preserves fast fall, uses all
  jumps, and retains mobility, gravity mode, forced landing lag and interrupt
  policy in `SpecialFallState`. This caller selects ordinary gravity/drift:
  retail's `xC != 0` branch **does not apply the stored mobility cap**. Landing
  enters LandingFallSpecial (43), with the caller's ten-frame lag and disabled
  interruptions. All values come from the character/common archives.
- `CharacterCallbacks::guard_variant` receives command outputs and runs once
  after GuardOn/GuardReflect setup. Marth's override selects model group 1,
  variant 1 and queues sound 190115 (volume 127, pan 64). Shared shielding
  continues to use PlMs.dat's shield joint, neutral pose and attributes.
  Yoshi's egg branch remains explicitly unimplemented in the default hook.
- FallSpecial's script requires opcode 46: decode the eight-bit color-animation
  ID and 18-bit duration, retaining a typed renderer request. Seeking skips the
  transient request as retail does. Marth's ledge escape also requires GFX
  `0x400`, which uses the existing `0x5A` brake-dust generator with negated
  facing. No new particle opcode or RNG site was needed.

Workspace layering is unchanged. No shared state branches on character kind;
Marth behavior stays in `ft-mars`, animation in `melee-ft`, scene effects in
`melee-sim`, and particle simulation in `hsd-particle`. Runtime consumes no
ledger fields, captured output poses or tick-specific behavior.

## Sixteen Marth movement scenes

Each 300-tick scene compares 49 keys; the three ledge scenes use 420 ticks.
The eight previously passing scenes remain covered alongside the new ports.

| Marth scene | Ticks | What it needed |
|---|---:|---|
| squat | 300 | Existing shared implementation |
| turn | 300 | Existing shared implementation |
| walk | 300 | Existing shared implementation |
| dash | 300 | Existing shared implementation |
| jump | 300 | Existing shared implementation |
| wavedash | 300 | Existing shared implementation |
| turnrun | 300 | Existing shared implementation |
| walkfast | 300 | Existing shared implementation |
| shield | 300 | Marth shield entry hook |
| spotdodge | 300 | Marth shield entry hook, including return from dodge |
| roll | 300 | Marth shield entry hook, including return from roll |
| airdodge | 300 | FallSpecial, script color request, LandingFallSpecial |
| ledge | 420 | Shared Fall blend and forward secondary pose |
| ledgeclimb | 420 | Shared Fall blend and forward secondary pose |
| ledgeescape | 420 | Shared Fall blend, forward secondary pose, reversed brake dust 0x400 |
| airjumpb | 300 | FallAerial and forward secondary pose |

Fox `airjumpb` additionally exercises the backward FallAerial secondary pose.
All seventeen new scene tests have independent ordered particle-draw checks.

## Corrections to the task notes

1. **Directional fall is an animation blend, not an action-state transition
   on stick input.** `ftCo_Fall_Anim_Inner` uses velocity. It attaches secondary
   submotions 21/22, 24/25 or 27/28 and retains state 29, 32 or 35. Retail state
   table entries 30/31 have `SM_None`; no supplied scenario enters them.
   The residual velocity at Marth's tick 90 remains below the blend deadzone;
   the first forward secondary selection occurs at tick 93.
2. **The Marth branch at ftCo_Guard.c:342-346 selects a model and plays a sound.**
   There is no character-specific offset calculation there or in its reflect
   counterpart at 924-928. The existing shared descriptor path reads the
   character's shield joint and pose. The original panic was in Marth's
   override, not the trait default. No invented shield offset was added.
3. **The airjumpb scripts are different.** Marth reverses drift at input frame
   62 and releases at 100; Fox releases at 80. The checked-in scenario comments
   and tracker explain that Fox timing would carry Marth offstage. Scenario
   files and recorded pads were used as supplied.
4. Marth ledge escape reaches the previously unsupported GFX `0x400`; it uses
   only already-ported RNG sites, consistent with the ledger note. FallSpecial
   adds a color command, with no RNG. State timing agrees with the supplied
   FallSpecial/FallAerial transition ticks.

## Retail audit

Read-only commands ran in `harness` with `UV_CACHE_DIR=/tmp/melee-uv-cache`.
Fused audit: `/tmp/m4t9-fused.log`; selected full disassembly:
`/tmp/m4t9-asm.log` and `/tmp/m4t9-entry-blend-asm.log`.

```sh
uv run python asm.py ftCo_Fall_Anim_Inner ftCo_800CC988 ftCo_80096900 ftCo_FallSpecial_Phys ftCo_800923B4 ftCommon_8007D140 --fused
uv run python asm.py ftCo_Fall_Anim_Inner ftCo_80096900 ftCo_800923B4 ftCo_800939B4 ftAnim_8006EDD0 ftAnim_8006FE9C ftAnim_8006FF74 ftCo_FallAerial_Enter ftCo_FallSpecial_Anim ftCo_FallSpecial_Phys ftCo_FallSpecial_Coll ftCo_80096D28 ftAction_80072A5C --fused
uv run python asm.py ftCo_FallSpecial_Phys ftCommon_8007D140 ftAnim_8006EDD0
uv run python asm.py ftCo_Fall_Anim_Inner ftCo_80096900
uv run python asm.py ftAction_80072A5C ftCo_800BFFD0 --fused
uv run python asm.py ftCo_8009F834 efLib_CreateGenerator_Translate_FacingDir --fused
```

| Operation | Retail arithmetic |
|---|---|
| Fall smoothing | 800CCCA0 rounded subtraction, **800CCCA4 fmadds** |
| Facing-relative direction | 800CCC64 fmuls followed by comparison |
| Special-fall mobility | 8009696C fmuls |
| Special-fall ordinary drift | 80096BAC fmuls, 80096BC8 fadds, 80096BD4 fmuls; no contraction |
| Secondary attachment/blend wrappers, entry and collision | No fused instructions; existing audited SRT/quaternion kernels reused |
| Color command and Marth model/sound | Integer decode/output only |
| Reversed brake dust | Facing negation; existing randomized offsets at 8009FCF8/FD1C/FD44 reused |

## Validation

Local retail assets were present. Logs are `/tmp/m4t9-validation-00.log`
through `19.log`, indexed by `/tmp/m4t9-validation-summary.log`.

| Command | Result |
|---|---|
| Nine requested `cargo run -q -p melee-sim -- gate ...` commands | Each: 300 or 420 ticks, 49 keys, 0 divergences |
| Four `idle/start_fd_fox/marth` CLI gates | Each: 600 ticks, 49 keys, 0 divergences |
| `cargo test -p melee-sim --test m4_gate` | 65 passed, zero failed |
| `cargo test -p melee-ft --test start_fox_bones_130` | 4 passed, zero failed |
| `cargo test -p melee-sim --lib marth_bones -- --nocapture` | 2 passed; 202,672 SRT words |
| `cargo test -p hsd-particle` | 58 passed, zero failed |
| `cargo test -p melee-sim --test m2_gate` | 1 passed |
| `cargo test -p melee-sim --lib fall_states -- --nocapture` | 1 passed; 67 fall ticks, exact scratch and command clocks |
| `cargo gate` | 645 passed, zero failed, one pre-existing ignored doctest |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed; no warnings |
| `cargo fmt --all` | Passed |

Baseline workspace gate: **609 passed, zero failed, one ignored**. Final: **645 passed, zero failed, one ignored**. All new tests ran with local data.

The added raw replay verifies secondary selection/weight, main animation ID,
rate, command frame, jumps used, and every named FallSpecial scratch word.
Its six scenes exercise 31/12/12/3/3/6 falling ticks respectively. A decoder
mapping mistake in the new replay initially swapped the +2354 landing-lag
and +2358 interrupt fields; the adapter now follows the struct and retail
stores at 8009697C/80096980. No captured value or comparison tolerance changed.
The character-hook test checks retained model output, unrelated model-group
preservation and the single sound request.

## Limits

Grounded special-fall entry, alternate special-move gravity/mobility mode and
special fall with remaining jumps retain C-located `unimplemented!` branches.
Direct entries into unused directional action-state IDs are unsupported;
all reachable directional secondary animations are loaded. Existing boundaries
for items, combat, wall/ceiling interactions and Yoshi egg shielding remain.
No independent movement bone dumps or Marth movement particle-field dumps
were supplied, so new evidence is scene fields, raw fall scratch and ordered
RNG, alongside the existing bone/particle-field regressions. Color, model and
sound outputs are retained; renderer/audio execution remains outside this port.
The crowd recovery sound helper is outside the headless audio slice.

No requested acceptance check was omitted. No Dolphin invocation, protected
harness/decomp changes, expected-value changes, game-data additions or commits.
Harness Python tests were not needed because no harness code changed.

## Changed files

- `TRACKER.md`
- `crates/ft-mars/src/init.rs`, `crates/ft-mars/tests/attributes.rs`
- `crates/melee-ft/src/anim/playback.rs`
- `crates/melee-ft/src/fighter/{air_dodge,assets,commands,effects,fall,jump,landing,mod,procs,shield,spawn,state}.rs`
- `crates/melee-ft/src/fighter/{README.md,M4_FALLS.md}`
- `crates/melee-sim/src/effects/dust.rs`
- `crates/melee-sim/src/frame.rs`, `crates/melee-sim/src/frame/fall_states.rs`
- `crates/melee-sim/tests/m4_gate.rs`
