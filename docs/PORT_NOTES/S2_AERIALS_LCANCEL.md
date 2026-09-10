# S2: aerials, landing lag, L-cancel and autocancel

2026-09-10, `lane/chars`. Uncommitted; no Git write commands or protected-path
writes. The follow-up extends ownership to ground-pose IK and the perf census.
All seventeen scenes now pass in both profiles, and merge-check passes.
Duplication and size budgets pass. The remaining acceptance failure is the
isolated perf tick-time mean: 30.140 ms exceeds the fixed 25.947 ms cap.
Unmodified C15 also exceeded that cap in a control run (39.559 ms). No timing
threshold or expected trace word was changed.

## Scene evidence

The local recordings use tick ordinals different from the approximate prompt:
Fox nair lands at tick 99, returning to Wait at 114 (full lag) or 106 (L-cancel).
The current CLI always calls `gate_items` and reports 62 keys for recordings with
an `items` member, even when their item arrays are empty. Its exact successful
line is **`300 ticks, 62 keys, 0 divergences`**; the 49 fighter/RNG fields remain
fully checked. `m5_gate` retains the requested 49-field gate plus ordered particle
draw checks. No comparator or reporting behavior was changed to print 49.

| Scene | Attacker state@tick, from attack entry | Target state@tick, after initial Wait | First-divergence history | Final release CLI |
|---|---|---|---|---|
| `nair_fd_fox` | 65@81 → 70@99 → 14@114 | 76@89 → 14@117 | submotion row correction: 83 p0.cur_anim_frame | 300 / 62 / 0 |
| `fair_fd_fox` | 66@81 → 71@99 → 14@121 | 76@87 → 42@96 → 14@126 | submotion row correction: 87 p1.cur_anim_frame | 300 / 62 / 0 |
| `bair_fd_fox` | 67@81 → 72@95 → 14@115 | Wait 14 throughout | submotion row correction: 88 p1.cur_anim_frame | 300 / 62 / 0 |
| `uair_fd_fox` | 68@81 → 73@101 → 14@119 | 83@94 → 42@122 → 14@152 | submotion row correction: 93 p1.cur_anim_frame | 300 / 62 / 0 |
| `dair_fd_fox` | 69@81 → 74@96 → 14@114 | 82@91 → 14@116 | submotion row correction: 91 p1.cur_anim_frame | 300 / 62 / 0 |
| `nairlc_fd_fox` | 65@81 → 70@99 → 14@106 | 76@89 → 14@117 | submotion row correction: 83 p0.cur_anim_frame | 300 / 62 / 0 |
| `fairlc_fd_fox` | 66@81 → 71@99 → 14@110 | 76@87 → 42@96 → 14@126 | submotion row correction: 87 p1.cur_anim_frame | 300 / 62 / 0 |
| `bairlc_fd_fox` | 67@81 → 72@95 → 14@105 | Wait 14 throughout | submotion row correction: 88 p1.cur_anim_frame | 300 / 62 / 0 |
| `uairlc_fd_fox` | 68@81 → 73@101 → 14@110 | 83@94 → 42@122 → 14@152 | submotion row correction: 93 p1.cur_anim_frame | 300 / 62 / 0 |
| `dairlc_fd_fox` | 69@81 → 74@96 → 14@105 | 82@91 → 14@116 | submotion row correction: 91 p1.cur_anim_frame | 300 / 62 / 0 |
| `nair_fd_marth` | 65@121 → 42@156 → 14@186 | Wait 14 throughout | submotion row correction: 123 rng.seed | 300 / 62 / 0 |
| `fair_fd_marth` | 66@95 → 42@132 → 14@162 | 76@100 → 42@114 → 14@144 | submotion row correction: 100 p1.cur_anim_frame; pose stop 154; endpoint/IK port resolves it | 300 / 62 / 0 |
| `bair_fd_marth` | 67@95 → 42@127 → 14@157 | Wait 14 throughout | submotion row correction: 100 p1.cur_anim_frame | 300 / 62 / 0 |
| `uair_fd_marth` | 68@121 → 42@157 → 14@187 | Wait 14 throughout | submotion row correction: 152 p0.facing_dir | 300 / 62 / 0 |
| `dair_fd_marth` | 69@95 → 74@134 → 14@166 | 80@100 → 14@135 | submotion row correction: 100 p1.cur_anim_frame; pose stop 166; endpoint/IK port resolves it | 300 / 62 / 0 |
| `dairlc_fd_marth` | 69@95 → 74@134 → 14@150 | 80@100 → 14@135 | submotion row correction: 100 p1.cur_anim_frame; pose stop 150; endpoint/IK port resolves it | 300 / 62 / 0 |
| `fairlc_fd_marth` | 66@113 → 71@127 → 14@142 | Wait 14 throughout | submotion row correction: 128 p0.cur_pos.y; pose stop 142; endpoint/IK port resolves it | 300 / 62 / 0 |

All scenes share the running jump-cancel prerequisite: the initial back-air gate
stopped at `dash.rs`'s `reject_running_jump` before KneeBend (tick 71 in Fox).
The first aerial implementation then hit the Attack status/scratch invariant;
the new typed Aerial scratch was added to that invariant. The first complete
sweep exposed my animation-index error: retail submotions are 68..72 and 73..77,
distinct from action states 65..69 and 70..74. Correcting those static rows
produced thirteen complete matches. No expected word changed.

Target IDs from `melee-types/src/motion_state.rs`: 76 = `DamageHi2`,
80 = `DamageN3`, 82 = `DamageLw2`, 83 = `DamageLw3`; 42 = `Landing`,
14 = `Wait`. Local fair Marth also transitions its target through Landing 42;
local fair L-cancel Marth misses the target.

## Retail behavior and static dispatch

- `ftCo_AttackAir.c:49-73,76-125`: A or a fresh C-stick threshold crossing
  enters the character table's `ENTER_AERIAL` function pointer. Its default is
  the one concrete common entry; Link/Young Link and Game & Watch must bind
  their own entry when ported. No character-kind branch or generic fighter was added.
  `ft_0DF1.c:72-81` supplies the strict previous/current C-stick edge tests;
  PlCo DC/E0 and angle 20 are loaded data.
- `ftCo_AttackAir.c:129-150`: clear interrupt, allow-landing variable and throw
  flags; preserve fast fall; install motion then advance the entry animation.
  Throw flag b3 reverses facing once. Animation completion enters Fall.
- All five IASA callbacks share retail `DO_IASA` (`AttackAir.c:154-194`):
  after script unlock, the supported item-free path checks the tether hook,
  another aerial and aerial jump. It does not add Fall's special/air-dodge/
  float predicates. Held-item throw input remains explicitly unported through
  the existing `Interaction::HeldItem` status stop (`AttackAir.c:52-61`).
- `ftCo_LandingAir.c:14-63`: `commands.variables[0]` selects directional lag
  versus autocancel. Direction chooses the existing `attributes.landing` field;
  scripts supply the windows and hitbox data, including Marth's tipper.
- The L-cancel age is `input.buttons.shield`, Fighter x67F, which the existing
  input sampler resets on synthesized `HSD_PAD_LR`. It is distinct from x680's
  raw digital timer. PlCo E4 is read as an integer window and E8 as a divisor.
  Retail 8008D6A4 divides in single precision; 8008D6A8 truncates with `fctiwz`;
  a zero result becomes one frame. The comparison is strictly age < window.
- `ftCo_LandingAir.c:66-75`: land and install at speed 1, then set speed to
  `(animation_length + 0.1f) / lag`. Assembly 8008D764/8008D768 has separate
  `fadds`/`fdivs`. Ordinary Landing retains its character reset hook; LandingAir
  does not call that hook. LandingAir has no input interruption callback.
- Aerial collision uses `ft_80082C74` / `ft_80081D0C`, the existing ordinary air
  collision without ledge or soft-landing selection. Existing grounded Landing
  callbacks provide physics, collision and animation completion.
- `fn_800CAF78`, `ftCo_Jump.c:62-88`, supplies the running KneeBend entry
  needed by these recordings. Dash/Run/RunBrake invoke it at their existing
  priority sites. The stick path uses the relaxed threshold; XY uses its edge.
- Aerial move identities join the fixed stale-history table so successive
  subaction hitbox replacements retain the current attack instance.

Read-only assembly evidence: `/tmp/s2-asm.txt`. Audited entry/selector/landing
symbols contain no fused multiply-add sites; the facing product is 8008CF38.

## Initial ground-pose boundaries (resolved by follow-up)

| Scene | First unexecutable tick | Exact checked prefix |
|---|---:|---:|
| `fair_fd_marth` | 154 | 0..153 (154 ticks) |
| `dair_fd_marth` | 166 | 0..165 (166 ticks) |
| `dairlc_fd_marth` | 150 | 0..149 (150 ticks) |
| `fairlc_fd_marth` | 142 | 0..141 (142 ticks) |

Every preceding 49-key snapshot matched in the diagnostic replay. The stop is
`ft_0899.c:109-232: LegCorrection`, emitted by `FighterCore::proc_pose`.
Before the follow-up, `collision/pose.rs` returned unsupported when `floor_probe` misses
a foot beyond the floor endpoint. Retail `fn_8008998C`, `ft_0899.c:44-54`,
instead gets the left/right floor endpoint and computes its vertical delta;
nonzero correction calls `lbBgFlash_80021410`, the two-joint IK solver.

Concrete fairlc evidence: target foot `(86.53035, 1.3385773, 0.75631595)`,
fighter root `(83.14299, 0.0001, 0)`, ground-pose flags 3. The foot is beyond
FD's floor. This occurs at Wait entry after the shortened LandingAir has
finished. Dair likewise completes full/shortened lag before its pose stop.
Temporary pose diagnostics were removed. The user subsequently extended S2
ownership to this path; its exact port is described below. S3/S5 files and
special states were not edited. No contradiction with an existing oracle was found.

## RNG and particle evidence

No new effect or sound RNG behavior was needed on the passing scenes. Across
all seventeen recorded ledgers, external sites are: normal spark 80063990 (8),
extra spark 800785CC (8), slash rotation 80063B70 (3), Wait choice 8008A8BC (42),
graphics offsets 8009FCDC/8009FD00/8009FD24 (123 each), FD setup/resume
8021AEC8/8021AFFC (17 each), and HUD shake 802F4D44/802F4D54 (396 each).
There are no additional sound RNG sites in these ledgers. Full-seed comparison
and ordered particle-site comparisons remain active. Particle fixtures for
connecting Fox nair and uair are exported by `fixture-spawns`, which validates
the complete fighter/item scenario before writing external spawn inputs.

## Initial validation (before the authorized follow-up)

Initial strict baseline: **1,017 passed, 0 failed, 3 pre-existing ignored**.
Final workspace commands used `--no-fail-fast` so the known S2 failures did not
prevent validation of the remaining suites. No missing-data opt-out was set.

Focused `aerial_rules`: 3 passed (direction/facing, C-stick edge priority and
non-retrigger, strict L-cancel window/truncation/minimum with varied common data).

Release M5: **42 passed, 4 failed**. The four failures are the named full Marth
scenes above; all 28 pre-existing gates and 13 complete S2 scenes pass, as does
the additional four-scene prefix test.

Exact particle replay lines (release):

```text
nair_fd_fox matched 300/300 ticks: 498786 fields, 9657 ordered particle draws, all final seeds; final seed 0x6a710b7e
uair_fd_fox matched 300/300 ticks: 491109 fields, 9295 ordered particle draws, all final seeds; final seed 0x141d7c01
```

Each reports `mismatches: 0`. The unchanged shared replay policy excludes
1,848 renderer-only AppSRT display-cache fields per scene.

Performance measurements did **not** pass the ratcheted copy budget. The first
run measured melee-ft 854 versus 845; all other compiling-crate counts stayed
at C15. Reusing the existing empty callback and inlining three small input/
forwarding helpers lowered that to 850. No tolerance, baseline, compiler profile,
character count, census selector or expected value was changed. These are new
concrete definitions, not a reintroduced per-character common graph. The
remaining five definitions are still a real acceptance failure.

Both full workspace profiles, run with `--no-fail-fast`, report **1,037 passed,
5 failed, 3 pre-existing ignored**. All five failures are the known pose stop:
four full M5 scenes and `dair_fd_marth_allocation_budget`. All existing suites
remain green. M4 is 261/261 in both profiles (debug 181.28 s, release 8.55 s).

`nairlc_fd_fox` measures zero simulate-only allocations over all 299 measured
ticks. Its new ceiling is zero. The new Marth dair zero ceiling remains enabled,
but its complete measurement cannot finish past tick 166; no complete zero-allocation
claim is made for that scene. All nine old allocation tests pass with unchanged
ceilings and measured totals. The overall allocation suite is 10 passed / 1 failed.

Clippy initially reported `items_after_test_module` because aerial landing code
was appended below the existing test. The existing test module was moved to the
end without assertion changes; the rerun passed. `tools/check-release-math.sh`
and format checks pass. Standalone `cargo test -p melee-sim --test m4_gate` ends:

```text
test result: ok. 261 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 187.68s
```

`tools/merge-check.sh lane/chars` exits 1. Protected data, trace availability,
main ancestry and build checks pass. Its gate stage stops at the same Marth
dair allocation pose failure (10 passed, 1 failed). Later merge-check stages do
not run; the separate profile, M4, clippy, math and format commands above provide
their own results.

| Command | Result |
|---|---|
| Each of the 17 `cargo run -q --release -p melee-sim -- gate harness/scenarios/<scene>.toml` | 13 exact full matches; 4 pose stops |
| `cargo test -p melee-ft --test aerial_rules` | 3 passed |
| `cargo test --release -p melee-sim --test m5_gate -- --nocapture` | 42 passed, 4 failed |
| `cargo test --release -p hsd-particle --test live_nair_fd_fox --test live_uair_fd_fox -- --nocapture` | Both passed |
| `cargo gate --no-fail-fast` | 1,037 passed, 5 failed, 3 ignored |
| `cargo gate --release --no-fail-fast` | 1,037 passed, 5 failed, 3 ignored |
| `cargo test -p melee-sim --test alloc_gate -- --nocapture` | 10 passed, 1 failed |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS after test-module placement fix |
| `tools/check-release-math.sh` | PASS |
| `cargo fmt --all -- --check` | PASS |
| `tools/merge-check.sh lane/chars` | FAIL at known pose stop |

Final isolated `CARGO_HOME=/private/tmp/p1-cargo-home CARGO_NET_OFFLINE=true
tools/perf-gate.sh` exits 1. Size and timing ceilings pass; copy ceilings fail.
An earlier run overlapping workspace validation also failed timing; it remains
recorded in `docs/PERF.md`, but the isolated run is the final timing evidence:

```text
melee-ft melee-ft copies: 850 > 845 + 0
C15 melee-ft copies: 850 > 845
C15 total copies: 1445 > 1440
[REGRESSION] perf-gate: 3483248 stripped bytes, 3178496 text bytes; load 175.138 ms; ticks_600 24.349 ms
```

Final instantiation census (all compiling crates):

| Crate | melee-ft copies | C15 ceiling |
|---|---:|---:|
| melee-ft | 850 | 845 |
| melee-sim | 128 | 128 |
| ft-captain | 66 | 66 |
| ft-falco | 67 | 67 |
| ft-fox | 67 | 67 |
| ft-fox-family | 0 | 0 |
| ft-mario | 0 | 0 |
| ft-mars | 66 | 66 |
| ft-peach | 67 | 67 |
| ft-purin | 66 | 66 |
| ft-yoshi | 68 | 68 |
| Total | 1,445 | 1,440 |

## Follow-up: floor endpoints and two-joint IK

The actual reference file in this pinned decomp is `lb/lb_020A.c`, rather than
`lbbgflash.c`. The new `melee-lb/src/ik.rs` ports `lbBgFlash_80021410` and its
`fn_8002113C` world-axis rotation / `fn_80020AEC` transform helpers. Parent
matrices are explicitly demanded in retail order (80020B44/68), including
matrix-independent joints; the final profile/CLI gates were rerun after this
cache-order audit correction. Concrete
`TwoJointIk` names the hip, knee, foot, extended foot, adjusted target and lengths.
No heap work occurs in the solver or ground-pose path.

`collision/pose.rs` now follows `fn_8008998C` exactly: probe the connected floor;
if the foot is beyond its endpoint, use existing `CollMap::floor_get_left/right`
walkers and subtract the fighter root's Y from that endpoint's Y. `floor_probe`
needed no change. The endpoint fallback retains the initialized zero normal.
The contact epsilon is 0.0001; the target's height adjustment is capped by a
slope of +/-0.45, with the equal-X branch setting delta to zero. These are retail
constants, unrelated to the loaded-data L-cancel parameters.

Right leg precedes left. The pose captures all three local rotations, optionally
solves the chain, aligns the foot, then restores the raw rotations without
marking matrices dirty. Foot alignment now handles a zero normal as a true
no-op and applies the retail twenty-degree clamp. The solver's matrix changes
and dirty-flag ordering survive exactly as the caller prescribes. Existing
non-flat/short-floor body-tilt stops remain outside this leg path.

Assembly audit:

- `fn_8008998C`, 80089A28/2C, 80089AAC..AE4: separate add/subtract/divide/multiply.
- Plane projection: 800214D8 `fmadds`, 800214EC `fnmadds`, 80021500..598 fused
  component updates. Perpendicular correction: 800215D0 `fnmsubs`.
- Vector lengths retain separate squared products and `z*z + (x*x + y*y)`.
  MSL square roots use the existing three double Newton steps with `fnmsub`.
- `lbVector_Angle`: 8000D748/750 fused dot; Melee's existing `acosf`.
- Reach softening and law-of-cosines: 80021844..21900 unfused products, sums and
  divisions. The eleventh/tenth powers are evaluated in their retail order.
- Knee bend softening: 80021968..994 retains double promotion and both `frsp`
  intermediates, before the final single angle deltas.
- New SDK `mtx_rot_axis_rad` (`PSMTXRotAxisRad`, 80342530) transcribes paired
  products/sums and 803425C0 `fnmsubs` / 803425C4 `fmadds`. It reuses audited
  SDK normalization and existing matrix/quaternion kernels.

Read-only disassembly logs: `/tmp/s2-endpoint-asm.txt`, `/tmp/s2-ik-asm.txt`,
`/tmp/s2-ik-angle-asm.txt`, `/tmp/s2-ik-transform-asm.txt`,
`/tmp/s2-ik-rotate-asm.txt`, `/tmp/s2-ik-foot-asm.txt`, `/tmp/s2-rotaxis-asm.txt`.

### Solver fixture and bone coverage

`melee-lb/tests/data/fairlc_marth_142_ik.json` records the first right-leg solver
input at tick 142, before projection: positions, loaded lengths and normalized
knee world axis as raw float words. The fixture names the scene, tick, fighter,
leg and capture boundary. `leg_ik.rs` reads it instead of embedding numerical
inputs in the test. It compiles `tests/ref/ik.c`, an independent C transcription
with explicit audited fused operations and the existing Gekko/MSL/retail-acos
shims, and compares both reference and Rust results bit-for-bit. Angle deltas:
**`0x3dd67460`, `0xbe445120`**. These expected words come from that C oracle,
not from a new Dolphin capture. Temporary capture instrumentation is removed.

Existing `melee-sim::frame::marth_bones::{start_marth_bones_130,
idle_marth_bones_8}` use the recorded Marth match-start/idle bone fixtures.
The current run compares 202,863 local SRT words across 90 Marth and 73 Fox bones, including
match-start landing. **They do not compare rendered matrices.** The available
recordings contain cached tick matrices, not an aligned rendered Marth matrix
oracle; no aerial-landing bone recording exists. Consequently the new endpoint
IK matrix changes have source/assembly and full-scene behavioral validation,
plus the numerical solver oracle, but no Dolphin rendered-matrix oracle. No
existing bone test or exclusion was changed.

## Follow-up: duplication metric

The user-authorized metric is now duplicate labels, not total emitted copies.
Within each compiling crate, count each distinct label with multiplicity >1
once. Across crates, count each label with more than one defining crate once.
Both growth tolerances are fixed at zero. Total labels and total emitted
definitions remain informational in stdout and every new `docs/PERF.md` block.
The existing `melee_ft::` label filter and strict common-body uniqueness/missing
helper audit remain unchanged; size and timing thresholds are unchanged.

The committed C15 implementation is `4bb9f07`. The older report's `82459f1`
was its pre-commit parent plus uncommitted changes, so the baseline recount uses
a read-only archive of `4bb9f07` and a clean temporary Cargo target. Baseline
`tools/data/c15-duplicate-baseline.json` records the revision, compiler, commands,
counts and SHA-256 of each raw log. C15 has 20 duplicate labels in melee-ft,
7 in melee-sim, 1 in each of seven compiling character crates, zero in ft-mario
and ft-fox-family, and 99 labels shared across crates. The old informational
emitted totals reproduce exactly: 845 / 128 / 66..68, aggregate 1,440.

New `duplicate-labels-v1` blocks migrate only the census baseline; historical
PASS timing/size values continue to apply. C15 duplicate ceilings stay fixed,
and subsequent PASS blocks can ratchet them down. Incomplete/failed runs never
raise a baseline. Eleven Python guard tests pass, including new concrete labels,
within-crate repetition, cross-crate repetition, aggregated duplicate rows,
missing evidence and non-ratcheting failed runs.

## Follow-up final acceptance

- `cargo gate --no-fail-fast` and `cargo gate --release --no-fail-fast`:
  **1,043 passed, 0 failed, 3 existing ignored** in each profile. All pre-existing
  suites remain green. M4 is **261/261** and M5 **46/46** in both profiles.
- `cargo test --release -p melee-sim --test m5_gate -- --nocapture`:
  `test result: ok. 46 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.85s`.
  All seventeen S2 full gates compare 300 ticks, 49 fighter/RNG keys and ordered
  particle sites with zero divergence. The former-boundary prefix test remains
  additional coverage, not a replacement for any full gate.
- All seventeen individual original release CLI commands exit 0 and print
  **`300 ticks, 62 keys, 0 divergences`** (the existing item-key reporting described above).
- `cargo test -p melee-sim --test alloc_gate -- --nocapture`:
  `test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.96s`.
  Both `nairlc_fd_fox` and `dair_fd_marth`: **299 measured ticks; simulate-only 0
  (0.000000/tick), peak 0, allocating ticks 0**. Existing ceilings are unchanged.
- `cargo test -p melee-sim --lib marth_bones -- --nocapture`: 2 passed;
  start 190,936 SRT words, idle 11,927 SRT words, zero matrix words.
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS. The initial
  assign-operation lint was resolved with a narrow annotation preserving retail
  multiplication operand order; no code behavior or lint command changed.
- `tools/check-release-math.sh` and `cargo fmt --all -- --check`: PASS.
- `python3 -m unittest discover -s tools/tests`: **24 tests passed**, including
  all 11 performance guard tests. Merge-guard tests use synthetic temporary Git
  objects and execute no Git write commands.

`tools/merge-check.sh lane/chars` exits **0**. Exact step results:

```text
[PASS] data: no tracked game data or protected lane changes
[PASS] data: oracle traces present
[PASS] rebase: main is an ancestor of lane/chars; checking the lane tree
[PASS] build
[PASS] gate
[PASS] m4_gate
[PASS] m5_gate
[PASS] hsd-particle
[PASS] clippy
[PASS] fmt
```

The final code reruns again passed **1,043 / 0 / 3** in each profile,
followed by clean clippy and fmt. Exact profile logs:
`/tmp/s2-final-current-debug.log`, `/tmp/s2-final-current-release.log`;
individual final CLI results: `/tmp/s2-final-current-scenes.json`.
Final isolated command (no concurrent validation commands):

```sh
CARGO_HOME=/private/tmp/p1-cargo-home CARGO_NET_OFFLINE=true tools/perf-gate.sh
```

It exits **1**, with duplication, size and load passing but tick time failing.
Exact final block:

```text
Duplicate-definition census (tolerance +0; label/definition totals informational):
  ft-captain: 1 duplicate labels; 43 labels; 66 definitions
  ft-falco: 1 duplicate labels; 44 labels; 67 definitions
  ft-fox: 1 duplicate labels; 44 labels; 67 definitions
  ft-fox-family: 0 duplicate labels; 0 labels; 0 definitions
  ft-mario: 0 duplicate labels; 0 labels; 0 definitions
  ft-mars: 1 duplicate labels; 43 labels; 66 definitions
  ft-peach: 1 duplicate labels; 44 labels; 67 definitions
  ft-purin: 1 duplicate labels; 43 labels; 66 definitions
  ft-yoshi: 1 duplicate labels; 45 labels; 68 definitions
  melee-ft: 20 duplicate labels; 815 labels; 852 definitions
  melee-sim: 7 duplicate labels; 101 labels; 128 definitions
  across crates: 99 duplicate labels
[REGRESSION] perf-gate: 3483952 stripped bytes, 3178496 text bytes; load 168.326 ms; ticks_600 30.140 ms
```

Exact failure lines:

```text
ticks_600_ns: 30139720.900 > 26444488.950 (previous 24040444.500, +10%)
P1 time ceiling ticks_600_ns: 30139720.900 > 25947000
```

Timing investigation kept every result and every fixed cap:

| Run, in measurement order | Load mean (ms) | 600-tick mean (ms) |
|---|---:|---:|
| First isolated S2 follow-up | 496.965 | 105.112 |
| S2 after an idle interval | 441.739 | 38.298 |
| Unmodified C15 control, `4bb9f07` | 223.183 | 39.559 |
| Final isolated S2, immediately after control | 168.326 | 30.140 |

For the C15 control, only the temporary benchmark's scenario path was pointed
at the existing absolute `chars/harness/scenarios/start_fd_fox.toml` so it reads
the same local data. No gameplay code, benchmark timing region, sampling setting,
or source-worktree file changed. Command: `CARGO_HOME=/private/tmp/p1-cargo-home
CARGO_NET_OFFLINE=true CRITERION_HOME=/private/tmp/s2-c15-timing-evidence cargo
bench --manifest-path /private/tmp/s2-c15-duplicate-baseline/Cargo.toml -p
melee-sim --bench ticks -- --noplot`. The C15 control also violates the historical
load and tick caps. This demonstrates that the absolute failures are not solely
an S2 change; it does not establish a controlled causal performance comparison
on this varying host. No slow sample was discarded and no baseline was promoted.

The final tick interval is 24.898..37.811 ms, with one high severe outlier.
The gate correctly uses the complete 30.140 ms mean and remains **REGRESSION**.
A timing recheck under stable host conditions remains necessary before claiming
the original full perf acceptance. The new duplication metric itself passes:
20 / 7 / 1 per compiling character / 99 across crates, unchanged from C15.

Final timing log: `/tmp/s2-followup-perf-after-control.log`. Control log:
`/tmp/s2-c15-timing.log`; raw control Criterion estimates:
`/private/tmp/s2-c15-timing-evidence`. All three full perf runs are retained in
`docs/PERF.md`, marked REGRESSION; none raises the passing baseline.

## Changed files

- `CLAUDE.md`
- `Cargo.lock`
- `TRACKER.md`
- `crates/hsd-anim/src/mtx.rs`
- `crates/hsd-particle/tests/data/README.md`
- `crates/melee-ft/src/collision/pose.rs`
- `crates/melee-ft/src/fighter/assets.rs`
- `crates/melee-ft/src/fighter/attack.rs`
- `crates/melee-ft/src/fighter/attack/stale.rs`
- `crates/melee-ft/src/fighter/character.rs`
- `crates/melee-ft/src/fighter/dash.rs`
- `crates/melee-ft/src/fighter/fall.rs`
- `crates/melee-ft/src/fighter/jump.rs`
- `crates/melee-ft/src/fighter/landing.rs`
- `crates/melee-ft/src/fighter/mod.rs`
- `crates/melee-ft/src/fighter/procs.rs`
- `crates/melee-ft/src/fighter/run.rs`
- `crates/melee-ft/src/fighter/state/callbacks/input.rs`
- `crates/melee-ft/src/fighter/state/common_table.rs`
- `crates/melee-ft/src/input/common.rs`
- `crates/melee-ft/tests/input_support/mod.rs`
- `crates/melee-lb/Cargo.toml`
- `crates/melee-lb/src/lib.rs`
- `crates/melee-sim/tests/alloc_gate.rs`
- `crates/melee-sim/tests/m5_gate.rs`
- `docs/PERF.md`
- `tools/perf-gate.sh`
- `tools/perf_report.py`
- `tools/tests/test_perf_gate.py`
- `crates/hsd-particle/tests/data/nair_fd_fox_spawns.json`
- `crates/hsd-particle/tests/data/uair_fd_fox_spawns.json`
- `crates/hsd-particle/tests/live_nair_fd_fox.rs`
- `crates/hsd-particle/tests/live_uair_fd_fox.rs`
- `crates/melee-ft/src/fighter/attack/aerial.rs`
- `crates/melee-ft/tests/aerial_rules.rs`
- `crates/melee-lb/src/ik.rs`
- `crates/melee-lb/tests/data/fairlc_marth_142_ik.json`
- `crates/melee-lb/tests/leg_ik.rs`
- `crates/melee-lb/tests/ref/ik.c`
- `docs/PORT_NOTES/S2_AERIALS_LCANCEL.md`
- `tools/data/c15-duplicate-baseline.json`
