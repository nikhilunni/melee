# C4: standalone effect engine and fixed pools

**Incomplete: final workspace acceptance is blocked by a replaced shared M2 capture.**
Validation stopped under AGENTS.md; no expected values were edited.

2026-09-09, combat lane. No commits or git write commands. The harness assets,
scenarios, traces and decomp were read only; the pre-existing decomp symlink
change was preserved. No expected oracle values or comparisons were changed.

## Ownership and dependency direction

`melee-ft -> melee-ef`; `melee-ef` never depends on `melee-ft` or `melee-sim`.
The runtime dependencies are `hsd-particle`, `hsd-anim`, `hsd-archive`, `hsd-types`,
`melee-types`, `gekko-math`, `anyhow` and `serde_json`. Archive decoding remains
at the archive boundary. JSON construction remains behind the optional fixture
recorder. `melee-lb` is a test-only dependency for the retail trig implementation.

`EffectOwner` exposes the queue, a bone/root matrix and facing. `FighterCore`
implements it, without acquiring a scene, particle system, archive or character
callback dependency. `Effects::flush<T>` receives the concrete core, so the body
is instantiated once for the shared core and retail trig type. The scheduler
supplies a statically dispatched bone-matrix closure to `Effects::tick<T>`.
This keeps scene membership and character-enum dispatch in melee-sim.

The fighter retains graphics-command decoding, common-bone selection, randomized
local offsets and the outgoing-pose sampling boundary. Those are fighter concerns.
Its graphics command buffer, effect requests and flush scratch now use inline,
fixed-capacity storage. Consumers import request types directly from `melee-ef`;
there is no cross-layer re-export facade.

| New crate path | Responsibility |
|---|---|
| `src/lib.rs` | efAsync/efSync dispatch, death/model effects, archive construction, animation/particle routing, efLib_Update at s_link 15 |
| `src/request.rs` | EffectRequest, EffectOwner, flat request queue and outgoing-pose resolution |
| `src/fixed.rs` | Safe inline fixed-capacity collection; exhaustion is explicit |
| `src/pool.rs` | Prepared model slots, reset/reuse, async eviction, sync admission, camera request consumption |
| `src/tables.rs` | Static supported request-to-model/particle rows and DPtcl allowlist |
| `src/dust.rs`, `src/egg_shell.rs` | Positional/directional generators and shell burst ordering |
| `src/spline.rs` | Unchanged effect spline evaluation, with Clone for initialization only |
| `src/fixture_spawns.rs` | Moved optional concrete fixture recorder; disabled path unchanged |
| `src/tests.rs` | Allocation, queue ordering, complete model reset and eviction regression tests |

`melee-sim` constructs `Effects` from its assets, calls flush at the existing
fighter proc boundaries and calls update in the existing scheduler slot. Stage,
HUD and particle scheduler wiring still record external fixture inputs through
the same hook; they do not move into the effect engine.

## Capacities and retail evidence

Retail does **not** impose one numeric limit on every kind of effect storage.
In particular, `HSD_ObjAllocInit(..., 4)` takes alignment, not a four-slot limit.
The port-specific limits below are identified separately from retail limits.

| Storage | Capacity | Source / interpretation |
|---|---:|---|
| Active async models | 64 | `efLib_Create` 0x8005BE88, `eflib.c:442-447`: evict when `efLib_EffectCount >= 64` |
| Active sync models | 64 | Explicit port bound. Retail excludes sync models from EffectCount and allocates through HSD_ObjAlloc (`eflib.c:442-464`). Shields and entry select sync loading (`efasync.c:407,429,453,751`); egg shell does too (`efsync.c:84,228-292`). Exhaustion fails explicitly. |
| Instance table | 128 | Sum of the separately enforced async and sync bounds |
| Prepared model slots | 64 per descriptor, 16 descriptors | Each descriptor can fill its class's entire pool. Static model IDs are the existing supported ef dispatch subset; this is initialization storage, not 1,024 simultaneously active effects. |
| Fighter request queue | 64 per fighter | Explicit port bound. `efAsync_QueueInit` 0x80067980 / `efasync.c:1466-1470` uses HSD_ObjAlloc with no numeric queue cap. |
| Graphics command queue | 64 per fighter | Same port bound at the command-to-request boundary; no first-use allocation |
| Partition, resolution and flush scratch | 64 entries each | Same request bound; flat entries carry optional resolved matrices instead of recursive Vec batches |
| Shell animation scratch | 32 | `efLib_AnimQueue`, `eflib.c:524-528`; the supported shell burst uses 12 (`efsync.c:228-292`) |
| Live fighter attachments | 512 booleans | Existing two-player scene and 256-bone identity stride; a port identity bound, not an ef allocator size |
| Camera request buffer | 64 per frame | Explicit port bound; request dispatch is `EF_SPAWN_CAMERA_SHAKE`, `efasync.c:1365-1368`. Consumers can drain in order; headless link 15 discards unconsumed requests, which have no simulation/RNG output. |
| Effect draw observations | 4,096 per frame | Explicit port diagnostic bound; no retail equivalent. Allocated at initialization, guarded before each generator creation, drained by the scheduler. `Generator::with_application_transform` logs at most one initial-emission draw per creation. |
| Model animation events | Asset-derived per model | Twice the maximum per-joint sum of (track stream bytes + 2 endpoint callbacks). KEY packs can emit multiple events per step; AObj rewind can stop and interpret. Capacity is allocated before ticking and retained on reuse. |

Async eviction follows `efLib_RemoveLast` 0x8005BBB4: p_link 11 before p_link 12,
oldest first within each; `efLib_Destroy` skips sync models. Among the supported
models death descriptor 0x19 uses link 12 (`eflib.c:475-478`). Sync models are
never sacrificed to async pressure. Shell batches check their sync capacity
before holding fragments in animation scratch.

Every descriptor's trees, tracks, spline paths and traversal IDs are loaded and
cloned during initialization. Reserving all 64 variants per descriptor spends
initialization memory to allow any supported descriptor mix without changing
HSD's owned animation representation or allocating on a later spawn. Retired
models restore scalar joint/AObj state and FObj playback cursors while retaining
their immutable byte streams. The small `FObj::restore_playback` helper in
hsd-anim is the only engine-support change. Complete tree equality after three
100-update/reuse cycles is checked for every supported descriptor.

Effect joint identities remain monotonic, independent of reusable storage slots,
so generators and fixtures cannot confuse a new effect with an expired owner.
Motion-change flushes preserve immediate requests and prior sealed batches in
order, append newly resolved pending requests newest first, and retain each
outgoing matrix. Current-pose matrix setup still runs at the same dispatch sites,
even when a sealed outgoing matrix supplies the effect transform.

## Allocation measurements and scope

The existing allocation gate uses start_fd_fox, one warm-up tick and 599 measured
ticks. Alloc, alloc_zeroed and realloc count; scene construction and fixture
serialization are outside measurement. The pre-change run recorded:

```text
599 measured ticks: simulate-only 22588 allocations (37.709516/tick); with snapshot 97464 allocations (162.711185/tick); snapshot overhead 74876 (125.001669/tick)
```

The latest measurement was **21,672 simulate-only / 96,547 with snapshots**:
916 fewer simulate-only allocations (4.06%) and 917 fewer with snapshots. Small
one-to-two-allocation run variation was already present in the baseline. This
measurement preceded the final sync/async capacity distinction; that distinction
passed the focused pool tests and M5, but the full final verification stopped at
M2 before another allocation measurement.
The gate ceilings were tightened from 27,301 / 102,175 to 21,700 / 96,600.

The effect-owned request/graphics buffers, model storage/recycling, flush scratch,
attachment registry and animation event buffers allocate nothing during normal
operation after initialization. Separate focused tests count exactly zero for
queue processing, complete model animation/reset and repeated pool exhaustion /
reuse. The existing absent-fixture-sink test now imports the moved recorder.

The complete scene still allocates: fighter animation replacement and ECB work,
and hsd-particle generator/particle, descriptor, texture/AppSRT and diagnostic
ownership remain C5 work. The effect engine still calls that particle subsystem;
this report does not claim zero allocations for those downstream calls or for
the full simulation. Enabled fixture recording intentionally allocates JSON.

## Baseline and final validation

The task's historical `jab_fd_fox` particle 0/6 failure was already fixed in this
checkout. Before extraction, M4 passed 261/261 and M5 passed 8/8. The baseline
workspace executable tests reported 951 passed, zero failed and two existing
ignored combat scratch tests. The baseline command then failed at a ft-captain
doctest because an overlapping subsequent build replaced its referenced artifacts
(E0463). This was a validation sequencing error, not a recorded product failure;
subsequent validation commands ran sequentially, and stopped on the workspace M2 failure described below.

Validation logs are in `/tmp/c4-before-*.log` and `/tmp/c4-final-*.log`.
No harness Python code or protected files changed, so no harness test run was needed.

## Changed files and line counts

Rust line counts (all source and test .rs files): melee-sim **11,705 -> 10,638**,
a reduction of **1,067**. The new effect crate has **1,804** lines, including
**201** new test lines (**1,603** production lines). It also absorbs fighter
request/flush definitions; fixed storage, model reset and capacity enforcement
account for the additional implementation. No sim facade was left behind.

- New `crates/melee-ef/Cargo.toml` and the ten modules listed above.
- Workspace `Cargo.toml`, `Cargo.lock`; dependency declarations in
  `crates/{melee-ft,melee-sim,ft-yoshi}/Cargo.toml`.
- `crates/hsd-anim/src/fobj.rs`: allocation-free playback restoration.
- `crates/melee-ft/src/fighter/{effects,commands,mod,spawn}.rs`: owner adapter,
  fixed request/graphics storage and outgoing-pose boundary.
- `crates/melee-ft/src/fighter/{damage,down,entry,grab,ledge,life,procs,shield}.rs`,
  `crates/melee-ft/tests/fighter_graphics.rs`, `crates/ft-yoshi/src/shield.rs`:
  direct imports/type paths for the moved requests; oracle assertions unchanged.
- Removed `crates/melee-sim/src/effects.rs`, `effects/{dust,egg_shell,spline}.rs`
  and `fixture_spawns.rs`; their implementation is owned by the new crate.
- `crates/melee-sim/src/{frame,lib}.rs`, `initial_state/{mod,cold}.rs`:
  initialization and scheduler wiring.
- `crates/melee-sim/tests/{alloc_gate,fixture_spawns}.rs`: tighter allocation
  ceilings and direct use of the moved fixture recorder.
- `CLAUDE.md`, `TRACKER.md`, this report: crate map, task/session status and evidence.

## Stop condition: shared M2 recording changed during validation

`cargo gate` exits 101 at the unchanged test
`fox_wait1_bones_match_the_real_game_bit_for_bit` in
`crates/melee-sim/tests/m2_gate.rs:121`, with **378** mismatched bone words.
The baseline gate had passed that same test. Filesystem timestamps show:

```text
/tmp/c4-before-gate.log                                  2026-09-09 19:46:14
harness/traces/fox_ys.bones.expected.jsonl                 2026-09-09 19:54:36
harness/traces/fox_ys.bones.expected.jsonl.meta.jsonl      2026-09-09 19:54:36
```

The untouched pre-extraction executable reproduces the same failure against the
replacement capture:

```sh
target/debug/deps/m2_gate-1414ea65b8ea13b9 --nocapture
```

It exits 101 with `378 bone words differ from the real game`. Comparing the
mismatch entries confirms the same keys and expected/actual values (their printed
order differs because the test uses HashMap). The new metadata's root scale is
`0.9599999785423279` on all axes. For example, frame 0, bone 41, matrix element 11
is expected bits `1045308674` (0.2013130486011505), actual bits `1045871728`
(0.2097032070159912), consistent with the observed scale discrepancy. M2 runs the
standalone Wait1 bone writer and does not call Effects or FObj::restore_playback.
The unchanged test's pose setup appears incompatible with the replacement capture;
this is not evidence of a C4 effect regression.

AGENTS.md requires stopping when a bit-exact test appears wrong rather than editing
its expected values. No M2 test, capture, metadata, game asset or decomp file was
changed. Fixing/regenerating that test's inputs is outside this task's writable
scope. The final workspace gate, release M4/M5, and a post-refinement clippy /
allocation / fixture refresh remain unverified. C4 stays in progress in TRACKER.

### Commands and latest exact result lines

The particle command passed **75** tests across 30 target/doc summaries; melee-ft
passed **90** across 24 summaries. The workspace run passed all five new effect
tests, including sync protection and link-ordered eviction, before stopping at M2.
The two pre-existing ignored combat scratch tests remain ignored.

`cargo test -p melee-sim --test m4_gate`

```text
test result: ok. 261 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 134.02s
```

`cargo test -p melee-sim --test m5_gate`

```text
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 11.33s
```

`cargo test -p melee-sim --test alloc_gate -- --nocapture`

```text
599 measured ticks: simulate-only 21672 allocations (36.180301/tick); with snapshot 96547 allocations (161.180301/tick); snapshot overhead 74875 (125.000000/tick)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.26s
```

`cargo gate`

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
```

The standalone debug M4 run was before the final capacity refinement; M5 was
rebuilt after it. The final workspace run stopped before re-reaching M4/M5.

```sh
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/shield_fd_fox.toml --out /tmp/shield.json
cmp /tmp/shield.json crates/hsd-particle/tests/data/shield_fd_spawns.json
```

Both commands exited 0; the fixture is byte-identical. This check also preceded
the final capacity refinement.

`cargo clippy --workspace --all-targets -- -D warnings` exited 0 before the final
capacity refinement:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 19.23s
```

`cargo fmt --all` was applied after the final Rust edits. `git diff --check`
passed. No release result is claimed: the sequential validation chain stopped
at `cargo gate` before those commands could run.

`cargo test -p hsd-particle` — exact target result lines:

```text
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 12.11s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 13.36s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.84s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.13s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.79s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.10s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.24s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.81s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.78s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.90s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.58s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.15s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.04s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.69s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.96s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.95s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.99s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.20s
test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`cargo test -p melee-ft` — exact target result lines:

```text
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.43s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.62s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.46s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.46s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.35s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.80s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.50s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.54s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Replacement M2 input SHA-256 values (read only):

```text
7a10323f911aab834e355a6df911425f3c1949421ca47a05e7e92a85933b84c9  harness/traces/fox_ys.bones.expected.jsonl
c598d66a013c872e0a500f8c3aab9aff3abb31b2ba83ded5474699063d43aabb  harness/traces/fox_ys.bones.expected.jsonl.meta.jsonl
```
