# C14: release exactness

2026-09-09, `chars` lane. No commits or Git write commands. No changes to
`harness/roms`, `harness/traces`, `harness/scenarios`, or the decomp. The existing
decomp symlink/type change predates this task. The lane's existing roms/traces
symlinks resolve to the restored local data in the main checkout.

## Result

The optimized `utilt_fd_marth` command now prints **300 ticks, 49 keys,
0 divergences**. The original release workspace had **920 passed, 27 failed,
3 ignored, 21 failing targets**. After the fix it has **926 passed, 23 failed,
3 ignored, 19 failing targets**, across 149 result blocks. Two tests were added;
four existing failing tests were repaired. Both complete workspace runs were
repeated after the final capsule-collision caller protections, with the same
counts and identical debug/release assertion details. No expectations, tolerances, filters,
or missing-data policies were changed.

Debug and focused validation results are recorded below.

## Instruction semantics and the fold

The decomp's `docs/getting_started.md:102` links to the
[PowerPC ISA Book I v2.01](https://math-atlas.sourceforge.net/devel/assembly/ppc_isa.pdf).
Pages 107–108 specify one rounding for `fmadd[s]`/`fmsub[s]`, followed by
negation for `fnmadd[s]`/`fnmsub[s]`. Section 4.3.3 (page 87) specifies the
zero sign: opposite-sign exact cancellation is +0 under round-to-nearest;
negation subsequently produces -0. Same-sign negative zero terms instead
produce -0 before negation and +0 afterward. These are different operations
from moving the minus inside the FMA. NaN payload propagation and FPSCR exception
state remain outside this change's existing host-FMA model; the ISA separately
specifies NaN exceptions to ordinary negation.

On this host, rustc 1.96.0 (`ac68faa20`, LLVM 22.1.2),
`aarch64-apple-darwin`, the original `-a.mul_add(c, -b)` loses that ordering
under optimization. `(0.051f32, 0.0, 0.0)` returns `00000000` instead of
`80000000`. This occurs without fast-math. In the simulator it first affected
`p1.kb_vel.y` at frame 147, through `fighter/damage.rs:390`.

The chosen operation is:

```rust
result.copysign(f32::from_bits(!result.to_bits()))
```

The f64 version uses the same construction. The magnitude comes from the
already rounded FMA; the complemented bit pattern supplies only its opposite
sign. Neither signed zero nor signed underflow is special-cased, and the FMA
still rounds only once. This is portable Rust with no production `black_box`,
volatile access, allocation, unsafe code, or out-of-line barrier. A comment at
`negate_rounded` explains why it must not be simplified back to unary minus.

Local optimized assembly retains one FMA followed by register-only sign
handling (`fmov`, `mvn`, `fmov`, a mask, `bif` on this AArch64 compiler).
Alternatives tested in scratch:

- A sign-bit XOR, or wrapping addition of the sign bit, becomes `fneg` again
  and reproduces the wrong zero.
- A sign-dependent `abs`/negation selection also reproduced the wrong zero.
- Rotating the bit pattern before toggling a bit works on this compiler but
  is less clear and still needs multiple integer/register operations.
- `black_box` works but emits stack store/load traffic. It was not selected.

This addresses the observed lowering defect and is checked at every supported
opt level. It is not a claim that an arbitrary future compiler cannot introduce
a different miscompile; the optimized gates enforce the contract.

## Audit and regression proof

`rg` found all direct Rust `mul_add` calls in `gekko-math/src/fma.rs`.
`fmadds`/`fmadd` already express the fused sum directly. `fmsubs`/`fmsub`
negate the addend before fusion, which is the subtraction's required zero-sign
rule; they do not negate the fused result. These remain direct fused operations.
The four negated forms now use the protected result negation, including the
previously absent f64 `fnmadd` helper.

The signed-zero tests cover all eight helpers: both multiplicand/product signs,
both addend zero signs, positive/negative nonzero cancellation, underflow to
both zeros, and epsilon-squared residuals proving fusion was retained. Only
inputs go through `black_box` in the tests. Both tests failed against the old
implementation in release and pass with the fix. A separate reproduction linked
against the actual `gekko_math` crate prints `80000000` at opt levels
`0`, `1`, `2`, `3`, `s`, and `z`.

The caller audit found two further direct result negations in
`hsd-anim/src/mtx.rs:191` (`PSVECCrossProduct`, retail `80342E58`). The retail
sequence is `ps_msub` followed by `ps_neg`. Both negated components now use
`fnmsubs`; this preserves the rounded intermediate and repairs the remaining
10 cross-product signed-zero mismatches in the native-C sweep. Other inverse
matrix/quaternion mismatches disappeared with the central helper fix.

The same vulnerable shape also appeared in `melee-lb`'s capsule projections,
segment/sphere dot product and rotation intermediates, and the conditional attacker shield
pushback in `melee-ft`. Those now use `negate_rounded` on the existing result.
They were protected by audit; they were not additional observed failing targets.
Other search hits were ordinary binary subtraction or negation after a separate
multiply/divide, rather than direct negation of an FMA result. A nested
`fmsubs(..., fmadds(...))` scratch probe also preserves the signed-zero addend.

There is no `crates/gekko-math/README*` on this branch. The module documentation,
unit tests, committed estimate fixtures, native MSL oracle, and shared C FMA
header supplied the local contracts.

## Native C oracle

`hsd-anim/tests/mtx_oracle.rs:100` already uses:

```text
-std=c99 -O0 -ffp-contract=off -fno-builtin -fno-strict-aliasing
-fwrapv -Wno-incompatible-library-redeclaration
```

These flags apply to both the MSL objects and the matrix/quaternion driver,
regardless of Cargo profile. There are no fast-math flags. The shared
`gekko-math/tests/ref/gekko_fma.h` already holds negated FMA intermediates in
volatile C variables, preventing Clang from making the same fold (which its
comment records even at `-O0`). Apple Clang is 21.0.0 (`clang-2100.1.1.101`).
No C flag or oracle expectation needed changing. The two matrix oracle tests
now pass, including 129,706 inputs per sweep. Their old failures were Rust
code-generation problems, not missing `-ffp-contract=off`.

## Benchmark

The requested command initially failed: this branch had **no `ticks` bench**,
no Criterion dependency, and no `Simulation::tick_without_snapshot` API.
The perf lane's Criterion bench cannot be used unchanged against this source.
A dependency-free `ticks` bench was added here using the existing `tick()` API:
600 `start_fd_fox` ticks including canonical snapshot creation, 5 warmups,
50 samples. Import and simulation destruction are outside the timed interval;
missing required data is a hard error. This measures this branch's release tick
path, not the perf lane's snapshot-free Criterion metric.

`cargo bench -p melee-sim --bench ticks` was run before and after; original and
final benchmark executables were preserved and run sequentially in A/B/B/A
order, with no assistant-started builds/tests concurrent with those repeats.
Times below are milliseconds per 600 ticks; intervals are approximate 95%
Student-t intervals for the sample mean, not a guarantee about system noise.

| Run | Before mean (95% interval) | After mean (95% interval) |
|---|---|---|
| Initial command / first corrected build | 43.910 [41.476, 46.345] | 23.394 [23.214, 23.575] |
| Consecutive A/B repeat | 25.600 [24.781, 26.419] | 25.989 [25.075, 26.903] |
| Consecutive B/A repeat (B first) | 41.403 [38.103, 44.703] | 24.573 [24.126, 25.020] |
| After the final caller audit | 33.494 [29.721, 37.267] | 24.896 [24.381, 25.410] |

The closest consecutive pair is +1.52% in the mean, with overlapping intervals
and medians of 24.928 ms before / 24.837 ms after. The final caller-audit
repeat has medians of 24.558 ms before / 24.511 ms after. There is **no measurable
regression in these runs**; strong host timing variation prevents a claim of
zero cost or a speedup. All samples and both executables remain in local scratch.
No `black_box` construction was used to trade performance for exactness.
When the perf lane merges, reconcile the benchmark target with its Criterion
version and repeat its snapshot-free metric; these are deliberately different
measurement scopes.

## Gates added

- `CLAUDE.md` lists `cargo gate --release` next to `cargo gate` and removes the
  blanket claim that optimized FMA negation is automatically safe.
- `tools/check-release-math.sh` runs `cargo test -p gekko-math` in debug and
  release, then compiles/runs the actual FMA module's tests at all six opt
  levels. It exits on the first error and cleans only its own scratch directory.
- `.cargo/config.toml` already defines `gate = "test --workspace"`; Cargo forwards
  `--release`, so no new alias was necessary.
- `tools/merge-check.sh` is **absent on this branch**. When the perf lane's script
  lands, add this line immediately after its debug gate (the wrapper was
  verified by reading the perf lane's script):
  `step gate_release 'error\[|could not compile|FAILED|panicked' cargo gate --release --locked`.
  Do not suppress either exit status.
- `[profile.release]` remains `debug = 1`; no profile flag can repair this
  expression. Rust has no stable fast-math Cargo profile switch, and none of
  the build flags were changed to relax floating-point semantics.

## Final validation

| Command / check | Result |
|---|---|
| Initial `cargo gate` | Failed at the pre-existing Battlefield idle particle coverage assertion. |
| Final `cargo gate` | Same first failing target/assertion; full gate is not green. |
| `cargo gate --no-fail-fast` | **926 passed, 23 failed, 3 ignored**, 149 result blocks, 19 failed targets. |
| `cargo test --workspace --release --no-fail-fast` | **926 passed, 23 failed, 3 ignored**, 149 result blocks, 19 failed targets. |
| `cargo run -q --release -p melee-sim -- gate harness/scenarios/utilt_fd_marth.toml` | **300 ticks, 49 keys, 0 divergences**. |
| M4 in the complete debug/release runs | **261 passed, 0 failed** in each profile. |
| M4 fixture preflight through `Scenario::required_files` | All **133 scenarios and 128 ledgers** present; the M4 passes do not depend on early missing-data returns. |
| Linked `gekko_math` reproduction at opt levels 0/1/2/3/s/z | **80000000 at all six levels**. |

- `cargo test -p melee-sim --test m4_gate`: **261 passed**.
- `cargo test -p melee-sim --test m4_gate --release`: **261 passed**.
- `tools/check-release-math.sh`: passes. **28 tests** in each Cargo profile;
  **3 FMA-module tests** at each of the six opt levels.
- `cargo clippy --workspace --all-targets -- -D warnings`: passes.
- `cargo fmt --all`: completed; `git diff --check` and shell syntax check pass.

The workspace still has the pre-existing missing-data early-return policy in
some unrelated tests (C11). These counts report what Cargo ran; they do not
claim that unavailable optional estimate captures were restored or recorded.

## Remaining failure classification

The final debug and release no-fail-fast runs have **exactly the same 23
failing test names, locations and assertion details**. There are **zero remaining
release-only failures** in the available workspace corpus. The 19 failed targets
comprise 11 hsd-particle integration targets, five melee-ft integration targets,
and melee-sim's library, M5 and Slippi-oracle targets. The table lists every
failing test; none was repaired by changing trace data or weakening tests.

| Test | Classification | Assertion location | Evidence |
|---|---|---|---|
| `battlefield_idle_600_particles_and_ordered_rng` | Particle replay | `crates/hsd-particle/tests/support/dust_replay.rs:255:9` | Tick 161 coverage: 5564 vs 5456. |
| `dream_land_idle_600_particles_and_ordered_rng` | Particle replay | `crates/hsd-particle/tests/support/dust_replay.rs:255:9` | Tick 533 coverage: 47 vs 20. |
| `live_fd_every_dumped_frame_matches_every_field` | C13 boundary constant | `crates/hsd-particle/tests/live_fd.rs:50:5` | Same saved-seed assertion as the preceding test. |
| `live_fd_600_ticks_match_rng_counts_sites_and_seeds` | C13 boundary constant | `crates/hsd-particle/tests/live_fd.rs:50:5` | Saved seed 3182633190 vs hardcoded 1286746018. |
| `live_fd_jab_marth_300_ticks_match_every_field_and_rng_draw` | Particle replay | `crates/hsd-particle/tests/support/dust_replay.rs:283:5` | 471 nonzero generator/particle position and velocity mismatches; 488714 fields compared. |
| `live_fd_ledge_420_ticks_match_every_field_and_rng_draw` | Particle replay | `crates/hsd-particle/tests/support/dust_replay.rs:228:9` | Tick 245 draw count: 43 vs 58. |
| `live_fd_start_600_ticks_match_every_field_and_rng_draw` | Particle fixture adapter | `crates/hsd-particle/tests/live_fd_start.rs:25:14` | Unclassified draw site 0x801c26ac. |
| `jab_fd_fox_particles_300_ticks` | Particle replay | `crates/hsd-particle/tests/support/dust_replay.rs:226:13` | Tick 108 draw 37: 2151270612 vs 2151270512; separate from the runtime unsupported-effect error. |
| `ko_fd_marth_particles_480_ticks` | Particle replay | `crates/hsd-particle/tests/support/dust_replay.rs:283:5` | 511 AppSRT/generator/particle mismatches; 850461 fields compared. |
| `utilt_fd_marth_particles_300_ticks` | Particle replay | `crates/hsd-particle/tests/support/dust_replay.rs:283:5` | 469 generator/particle position and velocity mismatches; 491333 fields compared. The simulator tick/ordered-RNG gate passes. |
| `story_idle_600_particles_and_ordered_rng` | Particle replay | `crates/hsd-particle/tests/support/dust_replay.rs:228:9` | Tick 11 draw count: 0 vs 3. |
| `story_start_600_particles_and_ordered_rng` | Particle replay | `crates/hsd-particle/tests/support/dust_replay.rs:226:13` | Tick 27 draw 40: 2151277028 vs 2151280384. |
| `idle_fox_600` | C13 boundary assumption | `crates/melee-ft/tests/fighter_support/saved_pose.rs:73:9` | Saved blend bytes begin 40 A0 (5.0); assertion expects 40 C0 (6.0). |
| `fox_wait_playback_600` | C13 boundary constant | `crates/melee-ft/tests/real_fox_wait_playback.rs:121:5` | Initial animation-frame bits [1086324736, 1065353216] vs [1084227584, 0] (6/1 vs 5/0). |
| `start_fox_600` | C13 count constant | `crates/melee-ft/tests/fighter_support/replay.rs:302:9` | Observed fighter RNG draw count 15 vs asserted 16. |
| `idle_fox_bones_8` | C13 boundary assumption | `crates/melee-ft/tests/fighter_support/saved_pose.rs:73:9` | Same saved-blend-byte assertion as idle_fox_600. |
| `start_fox_state_callbacks_600` | C13 count constant | `crates/melee-ft/tests/start_fox_states.rs:140:5` | Observed fighter RNG draw count 15 vs asserted 16. |
| `frame::combat::fox_jab_hitboxes_hitlag_and_hitstun_match_retail_scratch` | Known runtime gap | `crates/melee-sim/src/frame/combat.rs:160:27` | Frame 108, fighter 1 HitDetection: unsupported ef particle 0/6. |
| `frame::falcon_bones::idle_falcon_partial_emission_particles_600` | C13 boundary assumption | `crates/melee-sim/src/frame/falcon_bones.rs:138:5` | Recreated initial state has no pending_emission; test requires Some. |
| `frame::start_tests::start_effect_matrices_and_particle_state` | C13 boundary assumption | `crates/melee-sim/src/frame.rs:866:9` | Recreated first ledger has RNG draws; test requires an empty array. |
| `jab_fd_fox_300_ticks_and_ordered_particle_draws` | Known runtime gap | `crates/melee-sim/tests/m5_gate.rs:59:28` | Frame 108, fighter 1 HitDetection: unsupported ef particle 0/6. |
| `jab_fd_marth_300_ticks_and_ordered_particle_draws` | C13 aggregate constant | `crates/melee-sim/tests/m5_gate.rs:7:9` | 300 ticks and ordered RNG pass; new draw total 9783 vs old assertion 9373. |
| `ledger_links_tick_end_to_next_scheduler_start_even_with_repeated_vi_frames` | C13 seed constant | `crates/melee-sim/tests/slippi_oracle.rs:95:5` | First ledger seed 772060177 vs hardcoded 3427901605. |

The ten particle replay/adapter failures are left as unresolved fixture-driven
field/RNG/coverage divergences, not declared harmless or fixed. Matching the
same failure in debug establishes that they are not release-only; it does not
by itself establish whether an old spawn fixture, adapter, or simulation logic
is responsible. No expected values were edited. The C13 constant/boundary
assertions and unsupported 0/6 effect are outside this task's requested fixes.

## Files changed

- `crates/gekko-math/src/fma.rs`: protected f32/f64 negation, f64 `fnmadd`,
  signed-zero and fusion regression tables.
- `crates/hsd-anim/src/mtx.rs`: preserve post-FMA negation in cross products.
- `crates/melee-lb/src/collision.rs`, `src/dynamics.rs`, and
  `src/dynamics/arithmetic.rs`: protect directly negated fused dot/rotation
  intermediates, including two calls to the local fused `dot` helper.
- `crates/melee-ft/src/fighter/damage.rs`: protect negated fused shield pushback.
- `crates/melee-sim/Cargo.toml` and `benches/ticks.rs`: reproducible local release
  benchmark for this branch's existing tick API, without new dependencies.
- `tools/check-release-math.sh`: CI-sized debug/release/all-opt-level check.
- `CLAUDE.md`, `TRACKER.md`, and this report: gates, status, evidence and limits.

Local evidence: `/private/tmp/c14-release-before.log`,
`c14-release-final.log`, `c14-debug-gate.log`, `c14-debug-full.log`,
`c14-m4-debug.log`, `c14-m4-release.log`, `c14-m4-fixtures.log`,
`c14-utilt-release.log`, `c14-math-ci.log`, `c14-clippy.log`, `c14-fmt.log`,
`c14-zeros-before.log`, `c14-repro-opts.log`, `c14-bench-before.log`,
`c14-bench-final.log`, `c14-bench-post-audit-before.log`,
`c14-bench-post-audit-after.log`, and `c14-bench-paired-{0-before,1-after,2-after,3-before}.log`.
The `.log.json` files summarize exact counts and assertion messages.
