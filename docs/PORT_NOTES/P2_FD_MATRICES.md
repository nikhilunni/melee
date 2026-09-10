# P2: FD background matrices on demand

Status: implemented and uncommitted. All correctness/static gates pass; performance acceptance remains unmet on the loaded host.
No commits or Git writes. No game data, scenarios, expected values, tolerances,
performance budgets or decomp files changed.

## Consumers and matrix timing

All six FD models (maps 3..8) still execute `BackgroundAnimation::tick` at every
registered `Ground_801C1CD0` callback. Animation interpretation, joint clocks,
SRT updates, events and stage transition order are unchanged. Dirty matrices
remain dirty until a reader asks for them; `JObjTree::get_mtx` constructs the
necessary ancestor chain with the existing audited arithmetic.

| Consumer | Matrix timing |
|---|---|
| Live particle generators, including attached effect generators | Both Ground animation publication and map 7's post-rotation publication query `ParticleSystem::has_joint_attachment` before computing a joint matrix. The query scans the existing generator list, including inherited child attachments and zero-rate generators retained by live particles. No attachment cache is introduced. |
| New animation particle requests | `BackgroundAnimation::evaluate` already computes every DPtcl request's joint matrix before constructing the `SpawnRequest`. Spawn receives it immediately, and publication sees the new attachment in the same callback. |
| FD tilt generator | Creation and transform updates explicitly read map 7 joint 5 with `joint_matrix(5)`. Its AppSRT attachment is independent of ordinary `attachment_id`, so these reads remain explicit. Tilt completion still reads joint 2's animation clock. |
| Collision bindings | `update_collision` explicitly computes each bound joint. FD's map 3 root binding and Yoshi's Story's animated bindings remain unchanged. |
| Yoshi's Story detached puff | The request reads descendant 1 directly with `joint_matrix(1)`, replacing an allocating all-joint matrix list. |
| Dream Land replacement animation frame zero | This path intentionally reads the outgoing pose cache. `select_animation` materializes joints carrying DPtcl tracks before evaluating the replacement SRTs. Initial model creation still retains its original unevaluated matrix-cache semantics. |
| Fixture event recorder | `EventSink::finish` selects joints using all spawns in the recording and retains updates before their first attachment. An enabled recorder therefore requests full joint history, preserving the existing fixture output exactly. The normal disabled recorder adds no allocation or serialization. |
| Particle/effect matrix oracles | The start oracle reads live stage generators' `joint_matrix`; effect-model matrices come from the separate, unchanged effect trees. FD transition checks read attached generator matrices and shared AppSRTs. They do not require unattached stage matrices. |

The shared helper uses concrete types, scans existing storage, preserves model
joint traversal order and allocates nothing. Model joints are allocated in
preorder; the optional map-scale wrapper is added afterward and is outside the
archive joint namespace. Existing joint IDs therefore remain unchanged.

The new regression test compares all six models' animation clocks and spawn
matrices against eager evaluation for 600 ticks. It reads every previously
unobserved matrix after 240 ticks, then again after another 360 ticks, comparing
all floating-point words by `to_bits`. Existing scene and captured-particle
oracles cover the live transition and attachment lifecycle.

## Interleaved A/B

Reference: current main `322b1ce9b1a70cf3b66fc9147ad827fa14264485`.
Main sources were exported with read-only `git archive` to
`/private/tmp/p2-main-source`; the harness is a read-only-use symlink to the
same local inputs. Both release Criterion executables were built before timing
and copied to `/private/tmp/p2-{main,candidate}-ticks`. The candidate's changed
release crates were cleaned and rebuilt to prevent Cargo from reusing the
exported tree's artifacts when sharing the target directory.

The unchanged `cargo bench -p melee-sim --bench ticks` target is invoked directly
with `--bench --noplot`. Four rounds alternate AB, BA, AB, BA, with independent
Criterion output directories. Each run measures both load and `ticks_600`.
Raw logs, mean estimates and host load/CPU-idle samples:
`/private/tmp/p2-ab/`. Reported times are Criterion mean estimates.

| Round / order | Main load (ms) | P2 load (ms) | Main ticks (ms, 95% CI) | P2 ticks (ms, 95% CI) | Tick change |
|---|---:|---:|---:|---:|---:|
| 1 / main, P2 | 183.958 | 183.708 | 27.183 (26.598..27.948) | 25.556 (25.323..25.770) | -5.99% |
| 2 / P2, main | 177.348 | 176.617 | 26.062 (25.922..26.228) | 25.278 (25.063..25.547) | -3.01% |
| 3 / main, P2 | 177.411 | 183.114 | 26.223 (25.987..26.501) | 25.703 (25.289..26.143) | -1.98% |
| 4 / P2, main | 193.343 | 179.176 | 28.046 (27.279..28.808) | 25.398 (25.206..25.626) | -9.44% |

The host was **loaded**, despite no concurrent builds or tests from this session.
Pre-run aggregate CPU idle ranged from 56.5% to 66.1% on 12 logical CPUs (Mach `HOST_CPU_LOAD_INFO`, sampled for three seconds before each run).
The 1-minute load average ranged from 7.80 to 11.76.
Every paired round improved, but the spread and load-time variation show host
interference. These results **do not demonstrate** the requested idle-host
23.6 ms +/-1% band (23.364..23.836 ms). Main itself measured 26.062..28.046 ms,
above the supplied idle-host S11-a range of 24.78..24.99 ms. A quiet-host rerun
is needed to establish the absolute target; no timing budget was changed.

## Final validation

Strict workspace totals (summed from the test-result lines):

| Command / suite | Debug | Release |
|---|---|---|
| `cargo gate` / `cargo gate --release` | 1,127 passed, 0 failed, 3 existing ignores | 1,127 passed, 0 failed, 3 existing ignores |
| M4 | 261 passed, 0 failed, 0 ignored | 261 passed, 0 failed, 0 ignored |
| M5 | 89 passed, 0 failed, 0 ignored | 89 passed, 0 failed, 0 ignored |
| Allocation suite | 33 passed, 0 failed, 0 ignored | 33 passed, 0 failed, 0 ignored |

This includes every gated start scene, the shared Battlefield/Yoshi's Story/
Dream Land callbacks, the particle/effect matrix and full-field oracles, both
ordered full-match particle streams, and the new delayed-read regression.
`tools/check-release-math.sh` passed both profiles and fused math at opt levels
0, 1, 2, 3, s, z. The three existing ignores are unchanged.

Raw validation logs: `/private/tmp/p2-validation/`.
| Additional command | Result |
|---|---|
| `tools/check-release-math.sh` | PASS, including all six optimization levels |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo fmt --all -- --check` | PASS (exit 0, no output) |
| `tools/merge-check.sh lane/combat` | PASS, every step |
| `git diff --check` | PASS (exit 0, no output) |

The explicit scene commands were `cargo run -q -p melee-sim -- gate
harness/scenarios/<scene>.toml`, also repeated with `--release` before `-p`.
Their complete final lines, identical in both profiles, are:

```text
match_fd_foxmarth:
6083 ticks, 62 keys, 0 divergences
match2_fd_foxmarth:
10059 ticks, 62 keys, 0 divergences
match_fd_marth_scripted:
1600 ticks, 62 keys, 0 divergences
```

Both workspace runs contain:

```text
test frame::fd_background::fd_unobserved_animation_clocks_and_late_matrix_reads_match_eager ... ok
test frame::fd_background::human_match_fd_transition_attachments ... ok
test frame::start_tests::start_effect_matrices_and_particle_state ... ok
test match_fd_foxmarth_6083_ticks_and_ordered_particle_draws ... ok
test match2_fd_foxmarth_10059_ticks_and_ordered_particle_draws ... ok
```

Selected exact suite final lines (debug, then release):

```text
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 31.45s
test result: ok. 261 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 105.79s
test result: ok. 89 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 68.59s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.13s
test result: ok. 261 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 11.26s
test result: ok. 89 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.24s
```

The merge-check's final eight lines are:

```text
[PASS] rebase: main is an ancestor of lane/combat; checking the lane tree
[PASS] build
[PASS] gate
[PASS] m4_gate
[PASS] m5_gate
[PASS] hsd-particle
[PASS] clippy
[PASS] fmt
```


First perf-gate evidence: `target/perf/20260910T183045Z-7925`:

```text
[REGRESSION] perf-gate: 3672720 stripped bytes, 3358720 text bytes; load 182.273 ms; ticks_600 26.388 ms
```

Only tick timing failed: 26.388 ms exceeded the previous-PASS +10% limit
(26.269 ms) and the fixed P1 ceiling (25.947 ms). Size and duplication passed.
The census is **19 melee-ft / 7 melee-sim / 100 across crates**, matching S11's
recorded census. Character duplicate counts remain 1 each, except ft-mario and
ft-fox-family at 0; all 311 audited common labels have one definition.
The complete failed measurements remain in `docs/PERF.md`.

After all validation finished and a 45-second cooldown, the unchanged source
was rerun through `tools/perf-gate.sh`. Evidence: `/Users/nikhilunni/Projects/melee-lanes/combat/target/perf/20260910T185851Z-22978`:

```text
[REGRESSION] perf-gate: 3672720 stripped bytes, 3358720 text bytes; load 180.072 ms; ticks_600 27.409 ms
```

This rerun also failed only tick timing; size, load time and duplication passed.
Its tick mean's 95% CI was 26.095..29.047 ms. The host was still loaded:
CPU idle was 54.8% before the gate and 63.1% afterward;
load averages were 6.82/11.62/13.87
before and 7.53/11.22/13.64 afterward.
Host samples: `/private/tmp/p2-perf-retry.json`; command output:
`/private/tmp/p2-perf-retry.log`.

**Unmet acceptance:** an idle-host A/B demonstrating the absolute 23.6 ms
within-1% target, and a passing `tools/perf-gate.sh`. The paired loaded runs
consistently favor P2, but cannot certify that absolute target or establish the
entire residual cost's cause. Every model's retail-required animation work is
still present. No perf limit was raised, no failed measurement was removed,
and no unrelated host process was stopped. A quiet-host rerun is required.


## Changed files

- `crates/hsd-particle/src/system.rs`: query existing joint attachments.
- `crates/melee-ef/src/fixture_spawns.rs`: expose the recorder's history requirement.
- `crates/melee-gr/src/last/animation.rs`: preserve replacement-frame spawn caches.
- `crates/melee-sim/src/scene_stage.rs`: shared consumer-driven publication helper.
- `crates/melee-sim/src/scene_stage/last.rs`: filter map 7's post-rotation publication.
- `crates/melee-sim/src/frame.rs`: filter Ground publication; direct puff-joint read.
- `crates/melee-sim/src/frame/fd_background.rs`: eager-versus-lazy clock/matrix regression.
- `docs/PERF.md`: generated performance evidence, including the loaded failure.
- `docs/PORT_NOTES/P2_FD_MATRICES.md`: consumer audit, A/B table and validation report.
- `TRACKER.md`: P2 task and session status.

