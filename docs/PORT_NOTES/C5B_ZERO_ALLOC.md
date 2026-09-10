# C5-b allocation work and first complete PERF baseline

2026-09-09, battlefield lane; updated after the user's C5-b merge ruling.
The in-scope allocation reductions are complete. Zero allocations remain a C4/C8
follow-up, including the particle/AppSRT/RNG storage dependencies listed below.
Each scene now has a strict measured simulate-only ceiling; snapshot totals are
reported without an assertion. M2's 378-word mismatch is a known other-lane issue:
the VI oracle preceded matrix setup (60/73 dirty bones), and its test is moving
to the aligned post-render model. No oracle or expected value was changed.

This continuation ran no Git commands and did not edit TRACKER.md or the
protected effect, command, hitbox, frame-loop, game-data or decomp paths.

## What changed

All allocation counts include alloc, alloc_zeroed and realloc, exclude scene destruction,
loading and printing (owned per-tick Records are dropped inside the interval), and use one warm-up tick. Each scene uses its actual recorded
length (600, 300 or 480), not an extension with invented neutral inputs. Tests also
print maximum allocations in one tick and the number of allocating ticks. Snapshot
and simulate-only measurements independently reload the same initial state.

| Site (current source) | Previous allocation | Storage bound and source | Change / remaining work |
|---|---|---|---|
| `melee-ft/src/collision/ecb.rs:57` | A seven-position Vec every sample | Seven: `lb/types.h:186-187`, center `x108_joint` plus `x10C_joint[6]` | Inline array plus used length; duplicate suppression and matrix evaluation order unchanged. |
| `melee-ft/src/anim/playback.rs:162`, `:225`, `:292`, `:628`; `anim/root_motion.rs` | Whole-tree clones, part selection clone, temporary AObj/root traversal Vecs | Character's loaded joint count, bounded by `MAX_FT_PARTS = 140`, `ftparts.h:50-51`; `FighterPartsTable.parts_num`, `ft/types.h:46-50` | Validate selection before taking and reusing the existing tree; array for subtree selection; borrow-free successor traversal. AObj configuration is one concrete helper, formerly six closure instantiations. |
| `melee-ft/src/anim/attach.rs:57`, `:155`; `hsd-archive/src/desc/figatree.rs` | Selection Vec and two per-node grouping Vecs | 140 part slots, same `MAX_FT_PARTS`; grouping borrows parsed node ranges | Fixed selection array; iterator over track slices; all invalid mapping and translation-prefix checks precede mutation. |
| `melee-lb/src/anim.rs`; `hsd-anim/src/fobj.rs:452`, `jobj.rs:1307`; `melee-ft/src/fighter/spawn.rs` | AObj track Vec and track-byte copies on FigaTree replacement | At most 127 tracks per joint: signed `s8` FigaTree nodes, `lbanim.h:18-28`; loaded fighter joint count | Reserve runtime FObj buffers at construction, retain them when removing joint AObjs, share immutable FigaTrack streams using Arc. Interpreter arithmetic is unchanged. Generic AnimJoint attachment used by C8 remains allocating. |
| `melee-ft/src/fighter/procs.rs:488`, `:509` | Collider input `collect()` every dynamics tick | Eleven 0x28-byte records in Fighter +0x1670..0x1828 (`ft/types.h:1324-1333`); existing asset loader already asserts <=11 | Inline array and used length; samples still precede all character dynamics hooks. |
| `hsd-anim/src/jobj.rs:439`, `:1307`, `:1406` | Event buffer growth | One callback per FObj interpretation (`fobj.c`, `HSD_FObjInterpretAnim`); fighter reserve = joints ×127, engine animation reserve = loaded track count | Reserve during skeleton/animation attachment; callers must retain and drain buffers. This does not make on-demand C4 model import allocation-free. |
| `melee-gr/src/last/animation.rs:174`, `:183` | Event buffer discarded by `mem::take`, then a fresh request Vec on every evaluation | Loaded animation track count; at most one DPtcl callback per track per evaluation | Retain event and request buffers; return borrowed requests. Existing frame-loop call sites compile unchanged. |
| `hsd-particle/src/bank.rs:33`, `:44`, `:53`; `particle.rs`, `generator.rs`, `system.rs:125` | Program, texture-image and entire-bank copies at spawn; fresh empty Arc headers | Immutable bytes/tables sized by the loaded bank, not a runtime population pool | Shared program/image/descriptor tables; one shared empty image owner initialized in ParticleSystem construction. |
| `hsd-particle/src/system.rs:57` | BTreeMap bank registration and Arc bank allocation at first spawn | 65 bank slots: `particle.c:329-330`, `psInitDataBank` bank <65 | Fixed `[Option<ParticleBank>; 65]`, sharing immutable bank tables. |
| `hsd-particle/src/system.rs:301` | Stable-sort scratch for mixed display buckets | Sixteen buckets: `psdisp.c`, `particleSort` at 0x8039FC70 | Save each particle's input ordinal; in-place sort by (bucket, ordinal), exactly preserving ties and link masks. Ordinal is host scratch, never used by simulation math or snapshots. |
| `hsd-particle/src/system.rs:407` | Pending-generator Vec discarded after draining | Pending SList has no proven retail count | Clear and reuse its storage, preserving pending order; fixed-capacity provisioning remains blocked. |
| `hsd-particle/src/system.rs:102`, `:125`, `:198`, `:459` | Generator/particle Vec growth, AppSRT allocation and copy-on-write | **No fixed retail count found**; allocator evidence below | Remains. No guessed population ceiling, changed allocation-failure behavior, or approximated math. |
| `hsd-particle/src/rng_sites.rs:46` | Ordered draw-log Vec growth | No fixed per-tick retail draw count; depends on generators, particles and bytecode | Remains. No log entries dropped or truncated to make the allocation gate pass. |
| `melee-ft/src/fighter/effects.rs:91`, `:100`, `:123`, `:182`; `melee-sim/src/effects.rs` and `effects/*` | Effect partitions/queues and on-demand models/animations | C4 owns the effect pools | **Untouched, explicitly protected.** Allocation stacks confirm these paths still run after warm-up. |
| `melee-ft/src/fighter/commands.rs:401`, `:472` | Command side queues and per-part joint collection; `apply_part` also calls allocating AnimJoint/FObj loaders | C8 owns the interpreter | **Untouched, explicitly protected.** Idle-Fox allocation stacks reach both sites. |
| `hsd-archive/src/desc.rs:268`, `:351`, `:364` | Cycle-detection HashSet growth varied with randomized hashing and removed-entry tombstones during effect descriptor import | Host descriptor-validation path, retaining the existing MAX_DEPTH / MAX_NODES guards; not a retail runtime pool | Reusable Vec membership preserves cycle rejection and visitation order, removes seed-dependent allocation variation. Membership checks are linear in the active path. Import itself still allocates under C4/C8. |
| `melee-sim/src/trace.rs:26`; `melee-diff/src/snapshot.rs:90`; `melee-types/src/snapshot.rs` PrefixSink | Owned phase string, path strings, BTreeMap nodes and prefix composition | Current canonical scene schema: **49 keys**, checked by `trace::check_schema`; about 125 allocations per record | Retained as an opt-in diagnostic path; excluded from simulate-only ticking. See snapshot rationale below. |

Paths in the site table are under `crates/`; retail paths are relative to
`third_party/melee-decomp/src/`. Array and metadata changes preserve traversal,
float width, scalar evaluation and RNG order. Tests adapting Vec to Arc compare
the exact same expected byte/image arrays using `as_ref()`.

## Retail count audit and remaining work

The requested particle/generator constants are not present in this checkout.
`generator.c:75` and `particle.c:348` initialize HSD object allocators by object
size and alignment. `objalloc.c:140-156` clears the flags and sets `num_limit = -1`,
`heap_limit_size = 0`, and `heap_limit_num = -1`. `HSD_ObjAlloc`, lines 76ff,
checks a count only when the count-limit flag is enabled. `melee/db/dballoc.c:35-56`
contains the developer-only switch that sets limits to the *observed peak*, not
fixed constants. There is no `psgen.c` in the pinned source; generator logic is in
`generator.c`. `docs/PARTICLES.md:381-382` also states that the imported canonical
dump carries no pool capacity.

Similarly, `FtPartsDesc` (`ft/types.h:602-605`) is model/visibility metadata.
The actual bone count is `FighterPartsTable.parts_num`, already read and bounded
by the port. The implementation cites that source rather than attributing 140
to FtPartsDesc.

A question about allowing explicit simulator limits with fail-closed overflow was
sent during the audit. No such limits were implemented without an answer.
Even granting that policy would not remove the demonstrated C4/C8 allocations
under the current prohibited-file boundaries. Completing C5 requires those
lanes' queue/part-attachment changes as well as particle, generator, AppSRT and
RNG-log provisioning. No tick arithmetic was approximated to work around this.

## Snapshot rationale

The live Simulation::tick path calls InitialState::snapshot and currently emits
49 numeric fields, not the expanded ~3,200-key particle/raw-state dump. A Record
owns its map and strings because callers retain records and serialize/compare
them independently. Keeping that ownership costs about 125 allocations per tick,
with fixed schema and insertion order. It is acceptable for the opt-in diagnostic
API because training/throughput callers use tick_without_snapshot, which does not
construct a Record. This is an explicit diagnostic exception, not a claim that
snapshot allocation vanished. Per the merge ruling, snapshot totals are reported
only; the strict ceilings apply exclusively to independently measured simulation.

## Measurements and validation

Final command outputs and five-scene numbers are below. The existing COMPLETE
PERF baseline predates the allocation edits; it is not a final performance comparison.
Earlier exploratory allocation logs
are `/tmp/c5b-alloc.log`; temporary backtrace probes were removed, with evidence
in `/tmp/c5b-probe.log` and `/tmp/c5b-probe-idle.log`. Counting was disabled during
backtrace capture/printing, so these probe totals are not the final acceptance
measurement.

The initial `cargo gate` passed: **952 passed, 0 failed, 3 ignored**, 151 result
blocks (`/tmp/c5b-initial-gate.log`). The first normal perf invocation exited 1
because cargo-bloat's Cargo metadata attempted to unpack cached `crunchy` into
the sandbox's read-only global registry source directory. Using the existing
writable `CARGO_HOME=/tmp/c13-cargo-home` fixed that environment issue without a
lockfile change or installation. The script adds the concrete melee-ft library
to LLVM census inputs, rejects an empty core census, and labels complete blocks
explicitly while retaining PASS/REGRESSION baseline semantics.

The first COMPLETE/PASS block is `2026-09-10T02:48:01+00:00` in `docs/PERF.md`:
3,796,320 stripped bytes, 3,375,104 text bytes, 615 melee-ft copies charged to the
core crate and 2,109 charged to melee-sim. Its timing (748.969 ms load,
90.577 ms/600 ticks) overlapped the initial workspace gate and is noisy.
The pre-change benchmark binary was preserved as `/tmp/c5b-before-ticks`, SHA256
`3ac6f50cefa5386bc6173276756e0f87aec6094750a7ee61123f40b815943db0`, for an isolated
repeat before final performance comparison. Full per-crate bloat and per-function
LLVM tables are in PERF; concrete ECB, damage, overlap and animation bodies have
one copy. Generic callback helpers can still have multiple instantiations.


## Final allocation census and exact validation

The first continuation run caught randomized descriptor-import growth: start FD
measured 2,623 against 2,622 and start BF 2,360 against 2,359. Replacing the
cycle-membership HashSet with a retained vector removed that variation without
changing descriptor traversal or cycle checks. Two subsequent independent runs
produced identical totals in every scene. The ceilings were tightened further
from the ruling's historical counts to these exact final-tree measurements;
there is no one-allocation slack, retry policy, or snapshot assertion.

| Scene | Measured ticks | Simulate allocations / ceiling | Per tick | Peak / tick | Allocating ticks | With snapshot | Snapshot overhead / tick |
|---|---:|---:|---:|---:|---:|---:|---:|
| start_fd_fox | 599 | 2,621 | 4.375626 | 264 | 77 | 77,496 | 125.000000 |
| idle_fd_fox | 599 | 2,116 | 3.532554 | 45 | 86 | 76,991 | 125.000000 |
| jab_fd_marth | 299 | 1,018 | 3.404682 | 206 | 42 | 38,393 | 125.000000 |
| ko_fd_marth | 479 | 2,386 | 4.981211 | 668 | 57 | 62,261 | 125.000000 |
| start_bf_fox | 599 | 2,358 | 3.936561 | 264 | 69 | 77,233 | 125.000000 |

Start FD is down from 22,589 to 2,621 simulate-only allocations (88.40%).
Remaining storage and protected C4/C8 sites are explicitly listed above.
The per-scene comments name their effect/command dependencies; those lanes must
bring the ceilings to zero. Counts include all allocations in the measured tick,
including on-demand imports; no site is excluded by the counting allocator.

All commands below exited 0. Multi-binary crate totals sum the exact result
lines, including doctests. Math includes debug/release oracles and all six
standalone optimization levels (0, 1, 2, 3, s, z).

| Command | Passed / failed / ignored |
|---|---|
| `cargo test -p melee-sim --test alloc_gate -- --nocapture` | 5 / 0 / 0 |
| `cargo test -p melee-sim --test m4_gate` | 261 / 0 / 0 |
| `cargo test -p melee-sim --test m5_gate` | 8 / 0 / 0 |
| `cargo test -p melee-sim --test m4_gate --release` | 261 / 0 / 0 |
| `cargo test -p melee-sim --test m5_gate --release` | 8 / 0 / 0 |
| `cargo test -p hsd-particle` | 75 / 0 / 0 |
| `cargo test -p melee-ft` | 90 / 0 / 0 |
| `cargo test -p hsd-archive` | 47 / 0 / 0 |
| `tools/check-release-math.sh` | 74 / 0 / 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | Exit 0 |
| `cargo fmt --all` | Exit 0 |

The final debug M4 run was repeated after the descriptor membership change.
Its 261 tests pass on the same final production source as the release checks.

Exact result lines:

`cargo test -p melee-sim --test m4_gate` — `/tmp/c5b-ruling-m4-debug-final.log`

```text
test result: ok. 261 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 88.18s
```

`cargo test -p melee-sim --test alloc_gate -- --nocapture` — `/tmp/c5b-ruling-alloc-tight.log`

```text
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 20.72s
```

`cargo test -p melee-sim --test m5_gate` — `/tmp/c5b-ruling-m5-debug.log`

```text
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 17.57s
```

`cargo test -p melee-sim --test m4_gate --release` — `/tmp/c5b-ruling-m4-release.log`

```text
test result: ok. 261 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 14.98s
```

`cargo test -p melee-sim --test m5_gate --release` — `/tmp/c5b-ruling-m5-release.log`

```text
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.19s
```

`cargo test -p hsd-particle` — `/tmp/c5b-ruling-particle.log`

```text
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.84s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.33s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.50s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.28s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.17s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.16s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.58s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.66s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.11s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.07s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.14s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.66s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.19s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.30s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.48s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.71s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.06s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.61s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.05s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.66s
test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`cargo test -p melee-ft` — `/tmp/c5b-ruling-fighter.log`

```text
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.34s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.97s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.10s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.48s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.53s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.20s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.06s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.92s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.98s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`cargo test -p hsd-archive` — `/tmp/c5b-ruling-archive.log`

```text
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 24.63s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`tools/check-release-math.sh` — `/tmp/c5b-ruling-math.log`

```text
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.42s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.34s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
Fused math: opt-level=0
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
Fused math: opt-level=1
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
Fused math: opt-level=2
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
Fused math: opt-level=3
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
Fused math: opt-level=s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
Fused math: opt-level=z
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`cargo clippy --workspace --all-targets -- -D warnings` — `/tmp/c5b-ruling-clippy.log`

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 36.93s
```

`cargo fmt --all` — `/tmp/c5b-ruling-fmt.log`

```text
(no output; exit 0)
```


## M2 ruling and PERF limits

The earlier final debug workspace run had 950 passed, 6 failed, 3 ignored: five
then-zero allocation assertions and the known M2 mismatch. The preserved pre-C5
CLI reproduced all 378 actual mismatching words (`/tmp/c5b-before-m2.jsonl`).
The user subsequently identified the pre-matrix VI timing mismatch and instructed
this lane to ignore M2. Its alignment fix belongs to another lane.

The requested focused checks are recorded above. Full workspace gates were not
rerun in this continuation; no claim is made that the known M2 test passes.
The first COMPLETE PERF block remains the pre-edit baseline described above.
No final isolated timing comparison was run, and no performance tolerance was
raised. The five perf-script fixture tests passed in the earlier baseline work.

## File list

Production files under `crates/`:

- `hsd-anim/src/{fobj.rs,jobj.rs}`
- `hsd-archive/src/desc.rs`, `hsd-archive/src/desc/figatree.rs`
- `hsd-particle/src/{bank.rs,generator.rs,particle.rs,system.rs}`
- `melee-ft/src/anim/{attach.rs,playback.rs,root_motion.rs}`
- `melee-ft/src/collision/ecb.rs`, `melee-ft/src/desc/bones.rs`
- `melee-ft/src/fighter/{procs.rs,spawn.rs}` (dynamics and construction only)
- `melee-gr/src/last/animation.rs`, `melee-lb/src/anim.rs`
- `melee-sim/src/initial_state/particles.rs` (Arc compatibility only)

Tests: `crates/hsd-archive/tests/desc.rs`,
`crates/hsd-particle/tests/{common/mod.rs,start_paths.rs,support/restore.rs}`,
`crates/melee-sim/tests/alloc_gate.rs`, `tools/tests/test_perf_gate.py`.

Tooling/documentation: `tools/{perf-gate.sh,perf_report.py}`, `docs/PERF.md`,
`docs/PORT_NOTES/C5B_ZERO_ALLOC.md`, `TRACKER.md` (earlier work only; untouched
in this continuation).

The pre-existing decomp symlink/type-change status was preserved. Temporary
allocation-probe source was removed. No lockfile, protected path, or unrelated
user edit was changed.
