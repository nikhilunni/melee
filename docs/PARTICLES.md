# HSD particle simulation

`crates/hsd-particle` now runs the complete idle Final Destination particle
script and its sphere emitter, with owned state and caller-supplied
`&mut HsdRng`. It also supports disc, line and cone emitters and the common
instructions listed below. This is a subset of `generator.c` / `particle.c`,
not a complete effects system. It contains no `psdisp.c` renderer. Live `idle_fd_fox` replay now matches
all 600 RNG ticks and all eight captured state frames; see the live replay
result below.

The ledger's **6k+1 means six draws per newly emitted particle**, not per
currently live particle. Particle creation immediately interprets its script;
subsequent updates of FD's four-tick particles do not draw. Several cohorts
coexist, so live population and emission count differ substantially.

## FD data and animation provenance

All offsets here are relative to the GrNLa.dat archive data section. The tests
read the owned disc from `harness/roms/files/GrNLa.dat`; they skip only when
that file is absent. No disc data files are included in this change.

`map_head` is at `0x358`. Its models link at `+8` selects model set 4 with
`Ground_ModelDesc` stride `0x34` (entry `0x17C`). Its joint-animation list at
`+4` has eleven animations; only animation 0 contains the spawn track.
That AnimJoint root is `0x49270`. AnimJoint `0x493B0` on joint index 16
(JObjDesc `0x48FC8`, local identity SRT) references AObj `0x49260`:
flags `0x20000000`, end frame 4000. FObj `0x4924C` has track type `0x28`,
start frame 0, value/slope formats 0/0, and a six-byte KEY stream.
The value's **bits** are `0x001D4C1E`, decoded as bank `bits & 63 = 30`,
kind `(bits >> 6) & 0xFFFFFF = 30000`.

`JObjUpdateFunc` (`jobj.c`, `0x8036FDC0`) forwards this integer payload.
`efLib_Cb_DPtcl` routes stage banks through `grLib_801C99C0`, which calls
`hsd_8039EFAC` with link 0. Existing `hsd-anim` already produces
`JObjEvent::DPtcl`; **no hsd-anim change was necessary**. The real test loads
and evaluates the actual JObj/AnimJoint hierarchy, handles this event, and
supplies the evaluated joint matrix each tick. The separate bank test finds
one spawn at local animation tick 0 over 4000 FObj evaluations.

`map_ptcl` is a version `0x42` bank with first descriptor ID 30000 and five
descriptors. `map_texg` has three texture groups. Kind 30000 has:

| Field | Value |
|---|---|
| Generator shape / attached runtime type | sphere 8 / `0x708` |
| Kind after bank Locate | `0x08400000` |
| Generator / particle lifetime | 3999 / 4 |
| Emission rate / initial count | 10 / one `HSD_Randf()` draw |
| Radius / angle | -4000 / about 0.08726646 |
| Velocity | `(0, 0, -0.01)` |
| Size | 30 |
| Parameter 1 / 2 / 3 | bits `0x3F29C91F` / 0 / 0 |
| Script | texture 0; set RGBA; BA random RGBA; set five-tick fade; wait 4; end |

Bank parsing preserves the internal bank-relative pointers, independently of
DAT relocations. Version 0 and versions 0x40–0x43, sparse descriptor slots,
nonzero descriptor ID bases, texture presence and palette metadata are
supported. The next public bounds each public bank; a descriptor's program
ends at the next descriptor or bank boundary and may include alignment
padding. Texture pixels and form/geometry banks are not loaded.

## Frame order, lists, and lifetime

`ParticleSystem::proc_main` corresponds to `efLib_particles_proc_main`,
s_link 15 / p_link 11; `proc_aux` to the following p_link 12 proc. Each calls
`hsd_8039CEAC` (particles) **before** `hsd_8039EE24` (generators). Main skips
links 1 and 2; aux skips link 0. These are literal masks: links 3 and above
can update in both procs, rather than forming two disjoint partitions.

Particles occupy sixteen global head/next lists in ascending link order,
with optional references back to generators. Newly emitted particles are
prepended, interpreted immediately, and only then is the generator's count
decreased by one. Generator allocation has an unusual insertion rule:
after the current generator (the traversal cursor's successor), or after
the head if the cursor is null or at the tail. It is not simple append.
Supported instructions cannot create nested children; those opcodes error
before any child state is invented.

For a generator not masked or paused (`kind & 0x800`), joint attachment is
updated, emission count accumulated, particles emitted while count >= 1,
and then nonzero generator life decremented. Zero generator life means
indefinite duration. Expiration with surviving children changes rate to 0
and life to 1, retaining the generator. Thus **0 * Randf still draws**,
including the tick the final child dies before generator removal. Type bit
`0x80` kills children on generator expiration. An optional particle capacity
models allocation failure: geometry still draws, but no particle script runs.

Particle allocation sets life to descriptor life + 1 (u16 wrapping), wait to
1 for nonempty bytecode, and white primary / transparent black environment
color. Updates interpolate timers, decrement nonzero wait and interpret when
it reaches zero, decrement life, delete on zero, then apply gravity,
friction and velocity in that order. Pause freezes everything. A zero
particle life wraps to 65535 on update; it is not the generator's indefinite
life convention. Unknown instructions report `UnsupportedOpcode { opcode,
pc }`; malformed reads and nonyielding scripts also return explicit errors.
After an error, discard or restore the partial tick before continuing.

## RNG sites and floating-point audit

Addresses are the retail **branch instruction**, not the return address.
`DrawLog` records these in exact execution order without replacing the shared
RNG. There are seven distinct FD sites: one per-generator site and six
per-emission sites.

| Site | Symbol offset | Exact condition |
|---|---|---|
| `0x8039EF00` | `hsd_8039EE24+0xDC` | Generator eligible under link mask, not paused, emission rate >= 0; includes zero rate and no emissions |
| `0x8039EB74` | `hsd_8039DAD4+0x10A0` | Each shape-8 emission; latitude range nonzero and distance from double pi >= float 0.001 |
| `0x8039EBCC` | `hsd_8039DAD4+0x10F8` | Each shape-8 emission, after latitude selection, unconditionally |
| `0x8039B088` | `hsd_8039930C+0x1D7C` | Interpreter reaches BA; primary red random delta, even if delta is zero |
| `0x8039B0F4` | `hsd_8039930C+0x1DE8` | Same BA execution, green after red |
| `0x8039B160` | `hsd_8039930C+0x1E54` | Same BA execution, blue after green |
| `0x8039B1CC` | `hsd_8039930C+0x1EC0` | Same BA execution, alpha after blue |

For FD, the radius is negative, suppressing shape-8's radius draw at
`0x8039EBF0`; angle is nonnegative, suppressing the pre-loop angular draw.
Its latitude range selects `EB74`; the alternative branch draws at `EB04`
and `EB5C`. Generator creation additionally draws at `0x8039F250` when kind
bit `0x100` is clear and rate is nonnegative. Other supported shapes/scripts
have additional logged sites; the 6k+1 assertion applies only to this FD script.

The audit used the retail DOL, with these reproducible commands:

```sh
cd harness
uv run python asm.py hsd_8039DAD4 --fused
uv run python asm.py hsd_8039930C --fused
uv run python asm.py hsd_8039EE24 --fused
uv run python asm.py hsd_8039F05C --fused
```

| Operation | Retail instructions reproduced |
|---|---|
| Emission accumulation | `EF0C` fmadds |
| Velocity projection | `DEA0` rounded multiply then `DEA4` fmadds |
| Disc/cone angle interpolation | `E0C8`, `E114`, `E314`, `E3A8`, `E3E8` fmadds |
| Sphere position | `ED18`, `ED28`, `ED38` fmadds |
| Scalar magnitudes and radii | Separate sums/products, then three double fnmsub Newton steps; same kernel as `gekko_math::msl::sqrtf` |
| Sphere azimuth | Double pi multiplication, double multiplication by 2, then one f32 rounding |
| BA/BB random color | Separate fmuls/fadds, signed byte delta, clamp, fctiwz and byte store |
| A8 random position | `A4A4`, `A4EC`, `A528` fmsubs; position addition separate |
| AC random size / BC random pose | `A818` / `B514` fmadds |
| ED random rotation | `C8D8` fmadds for continuous random; discrete branch remains multiply/divide/add |
| Particle physics / scalar interpolation | Separate operations, no fused multiply-add |

Matrix operations reuse `hsd-anim`'s already audited paired-single kernels.
Inverse trig is supplied via its `InverseTrig` interface; real tests use
`melee-lb`'s retail implementation. No host libm is used in simulation.
The existing `hsd-anim` normalization kernel documents its unresolved
`FMULS FRC TRUNCATION PENDING` limitation; this port inherits it. The live FD replay below now checks particle position/velocity bits for
eight captured frames.

## Opcode coverage

Every byte in the supported ranges is exercised by behavior tests, including
all vector/color masks, both wait encodings, and each texture-flip mode.
Unsupported entries fail explicitly; they are not skipped as in the C
switch's default case.

| Bytes | Supported behavior |
|---|---|
| `00–3F` | Wait, including extended count and zero-time nop |
| `40–7F` | Texture pose plus wait |
| `80–87`, `88–8F` | Selective position set/add |
| `90–97`, `98–9F` | Selective velocity set/add |
| `A0`, `AC` | Size interpolation, deterministic/random |
| `A1–A3` | Disable texture; gravity; friction |
| `A6`, `A7`, `A8` | Random life; probabilistic deletion; random position offsets |
| `A9` | Fixed-aperture velocity randomization; retail inverse trig and fused rotation (Dream Land) |
| `AB`, `BE` | Scalar/component velocity multiplication |
| `AD–B1` | Primary/environment and mirror flags |
| `BA`, `BB` | Four signed random color deltas |
| `BC` | Random texture pose |
| `C0–CF`, `D0–DF` | Selective primary/environment target and duration |
| `E3–E8` | Palette; S/T flip modes; direction flags; trail |
| `ED` | Random rotation, continuous/discrete |
| `FA`, `FB`, `FC`, `FD` | Counted loop mark/back; unconditional mark/jump |
| `FE`, `FF` | End/delete |
| All other bytes | `UnsupportedOpcode` |

Not yet ported: child particle/generator creation/remaps (`A4/A5/AA/B9/EF–F2`),
force/JObj operations, AppSRT transforms, material/ambient/alpha compare
tracks, additional color/velocity operations, callbacks and user data.
Generator shapes 2 (tornado), 5 (rectangle), and custom shapes error.
Shapes 0/1/3/4/6/7/8 are supported; camera-facing generators error at emission
and AppSRT generators error at creation. Deferred detach requests and
arbitrary callback-created list mutations remain unimplemented. Other FD
animations/effect kinds and the full stage's transitions are not validated.

## Measured cold-start result and verification

Run the real-archive test:

```sh
cargo test -p hsd-particle --test real_fd_particles -- --nocapture
```

It starts FD model 4 animation 0 at local tick 0 with seed 1, processes the
one creation draw separately, and runs main then aux for 120 ticks. This is
an isolated stage-animation cold start, **not** a complete match boot or a
restored `idle_fd_fox.sav`. It verifies exact call-site order, 1+6 times the
newly emitted cohort, every surviving age cohort, generator life and child
count, and identical output on repetition.

Per-tick draw counts:

```text
43,19,1,1,61,55,31,37,43,25,25,31,19,55,43,55,7,49,37,1,
19,49,55,55,61,55,37,25,31,61,19,31,13,49,43,43,37,19,7,43,
7,7,7,43,19,13,19,1,43,7,7,61,19,25,43,49,25,49,43,25,
7,13,49,25,13,19,55,13,25,37,19,13,37,25,31,37,25,43,1,43,
19,25,37,25,19,1,43,55,25,25,19,25,19,13,19,1,7,7,25,37,
19,1,55,55,37,37,55,55,43,31,31,25,55,31,55,43,13,19,25,31
```

Final seed: `0xE7BBE242`. First twelve live populations:
`7,10,10,10,13,19,24,30,27,22,21,20`.
The recorded savestate ledger instead begins `61,13,43`; reproducing those
counts requires its initial count/seed and whole-tick integration, not a
hardcoded count sequence. The dump tools now expose that state.

Capture three completed ticks from the existing savestate (repository root):

```sh
MELEE_PARTICLES_SAVESTATE="$PWD/harness/roms/idle_fd_fox.sav" \
MELEE_PARTICLES_OUT="$PWD/harness/traces/idle_fd_fox.particles.jsonl" \
MELEE_PARTICLES_TICKS=3 \
"$HOME/Projects/dolphin-scripting/build/Binaries/Dolphin.app/Contents/MacOS/Dolphin" \
  -e "$PWD/harness/roms/GALE01.iso" \
  --script "$PWD/harness/dolphin_particle_snippet.py" \
  -v OGL -C Dolphin.Core.EmulationSpeed=0 \
  -C Dolphin.Core.SIDevice0=6 -C Dolphin.Core.SIDevice1=6 \
  -C Dolphin.Core.SIDevice2=0 -C Dolphin.Core.SIDevice3=0
```

The `.done` sidecar signals completion; `.err` records capture failures.
The snippet removes its listeners when complete; close Dolphin afterwards.
Runtime struct fields central to replay are:

| Struct | Layout (hex offsets) |
|---|---|
| Generator (`94` bytes) | next `00`, kind `04`, rate/count `08/0C`, JObj `10`, life/type `14/16`, program `20`, position/velocity `24/30`, child count `50`, aux `60..90` |
| Particle (`98` bytes) | next `00`, kind `04`, wait `1A`, loop count `1C`, program `20`, PC/mark/loop PC `24/26/28`, life `2A`, velocity/position `2C/40`, generator `88`, AppSRT `8C` |
| AppSRT (`A4` bytes) | generator `04`, translation/rotation/scale `08/14/24`, use count `32`, matrix `34`, ID `A0` |

[PARTICLES_DUMP.md](PARTICLES_DUMP.md) contains capture details,
the complete `0x94` generator / `0x98` particle / `0xA4` AppSRT layouts,
canonical record fields and fake-memory tests. It captures the savestate's
initial state separately from N completed scheduler ticks. No live capture
was performed for this change. Restoring those records into Rust and
comparing live numeric state remains follow-up work.

Validation commands:

```sh
cargo gate
cargo clippy --workspace --all-targets -- -D warnings
cd harness && uv run python -m pytest -q
```

The sandboxed run uses `UV_CACHE_DIR=/tmp/melee-uv-cache` to keep uv's cache
inside a writable directory. No test expectations were loosened. Source
changes are confined to hsd-particle, the two harness scripts and their
tests, and these two documentation files. Cargo also updates its lockfile
for hsd-particle's hsd-anim dependency and melee-lb test dependency. Root
Cargo.toml, hsd-anim, TRACKER.md, CLAUDE.md, third_party and harness/dolphin
are unchanged; nothing was committed.

Final verification: `cargo gate` passed (including all 35 hsd-particle tests with the disc present); workspace clippy passed with warnings denied; harness pytest passed, 181 tests.

## Live savestate replay result (2026-09-08)

The `idle_fd_fox` captures now pass both live tests:

```sh
cargo test -p hsd-particle --test live_fd -- --nocapture
```

- **600/600 ticks:** particle draw counts, ordered branch sites and RNG
  variants match the ledger; all **600 final seeds** match the decoded tick
  trace. There are 18,306 particle draws: 600 emission-count draws and 2,951
  emissions with six draws each. Every particle draw is `HSD_Randf`
  (`pc=0x8038054C`). First divergence: **none**.
- **8/8 state snapshots:** all 10,120 canonical field comparisons pass,
  including f32 bits, list order, color bytes, inactive counters, program
  offsets, generator associations, and family allocator state. The initial
  snapshot also round-trips without dropping or adding canonical fields.
- First eight per-tick particle draw counts: **61, 13, 43, 1, 37, 49, 7, 13**.
  The test prints the full 600-count vector. Its histogram is:

  | Draws per tick | Ticks |
  |---|---|
  | 1 | 30 |
  | 7 | 64 |
  | 13 | 67 |
  | 19 | 67 |
  | 25 | 50 |
  | 31 | 55 |
  | 37 | 58 |
  | 43 | 56 |
  | 49 | 67 |
  | 55 | 59 |
  | 61 | 27 |

The restored boundary has one generator (family 257, remaining life 3890,
count bits `0x3EB28A00`), three particles, and family allocator 425.
The initial seed is **1286746018**, verified against the initial sidecar
and every ledger post-draw seed by replaying the LCG. Frame 0's tick-trace
seed is the result of the first 61 draws, not the starting seed. Final seed
after tick 599 is **`0x72AA79F8`**. The existing emission arithmetic, immediate
interpretation, main/aux masks, list order, and lifetime sequencing already
match these captures; no changes to them were needed.

The actual ledger has **12**, not 11, external-draw ticks:

| Caller branch | Ticks | Variant |
|---|---|---|
| `ftCo_8008A7A8+0x114` (`0x8008A8BC`) | 115, 120, 235, 240, 355, 360, 475, 480, 595 | `HSD_Randi(100)`, PC `0x8038059C` |
| `grLast_8021ADD0+0x270` (`0x8021B040`) | 109 | `HSD_Randf` |
| `grLast_8021ADD0+0x13C` (`0x8021AF0C`) | 196 | `HSD_Randf` |
| `grLast_8021ADD0+0x22C` (`0x8021AFFC`) | 509 | `HSD_Randf` |

All twelve precede the particle procs. The test advances the shared RNG
for those external calls in ledger order, then runs `proc_main` followed
by `proc_aux`. It rejects unknown callers or external calls interleaved
with the particle block. Expected particle counts, sites, and seeds never
control particle updates, and the RNG is never reset between ticks.
The Wait result is unused; this models its RNG consumption, not fighter
animation selection. The external calls are the only ledger-driven input
after restoring the initial snapshot.

### Restoration and additional state

`tests/support/restore.rs` uses `melee_diff::read_trace`; both melee-diff and
serde_json are dev-dependencies only. Its field adapter restores every
canonical field present in these FD records and emits actual updated state
for comparison. Program identity/resolution is derived by matching owned
bank bytecode. Unknown fields, unresolved programs, unsupported non-sphere
auxiliary shapes, non-null AppSRTs, and pending detach requests fail
explicitly. This is an FD loader, not a claim of general AppSRT support.
Missing local captures or GrNLa.dat skip cleanly; no dump regeneration is
needed, and no expected fields or comparison tolerances changed.

New owned state fields:

| Rust state | C member and offset |
|---|---|
| `Particle.alpha_compare` | `aCmpCount` 54, `aCmpRemain` 78, parameters 57/58, targets 7A/7B |
| `Particle.alpha_compare_mode` | `aCmpMode` 56 |
| `Particle.point_joint_offset` | `pJObjOfs` 59 |
| `Particle.material` | `matColCount` 5A, `matColRemain` 74, current 7C/7D, targets 80/81 |
| `Particle.ambient` | `ambColCount` 5C, `ambColRemain` 76, current 7E/7F, targets 82/83 |
| `Particle.appsrt_id` / `Generator.appsrt_id` | `appsrt` 8C / 54, normalized optional reference |
| `ParticleSystem.pending_generators` | `hsd_804D78F4`, SList data at 04, owned references |

The existing `family_counter` is now exposed for restoration;
`from_live_lists` preserves captured list order, family IDs and associations
without allocation or RNG consumption. New byte-pair tracks retain the
retail defaults (`particle.c:461–483`, asm `80398E18..80398E7C`) and the
countdown/copy behavior (`particle.c:732–766`, asm
`80399474..80399518`: integer `lhz/subi/sth`, then target-byte copies).
A lifecycle regression checks pause, intermediate bytes and completion.

Rust fields not supplied by canonical records:

- `program` bytes and `texture_images`: loaded from the owned GrNLa.dat bank
  using the captured descriptor ID and texture group. Descriptor creation
  parameters and its redundant initial type/life/rate fields also come from
  that bank; runtime type/life/rate and the active sphere state are restored
  from the capture. The loader's constructor uses a private scratch RNG;
  its initial count is overwritten and it cannot consume the replay RNG.
- `joint_matrix`: resolved from the initial metadata's generator JObj pointer
  and cached matrix. This joint's animation-0 track only emits the generator;
  its identity transform stays fixed for this replay. No future snapshot is
  used as input. General animated attachment restoration remains outside
  this FD test.
- Internal generator `id` and `next_id`: assigned from initial list order
  and its successor. `generator_cursor` starts empty; C resets it at every
  generator pass. The family allocator is restored independently, not
  inferred from the largest surviving family ID.
- `particle_capacity`: unlimited, matching the observed successful
  allocations; the canonical dump carries no pool capacity. Opaque callback,
  user-data and allocator pointers are metadata, not canonical state, and
  are not executed by this adapter. There are no callbacks/AppSRTs/pending
  detaches in this capture.

The eight state frames establish live bit-exact particle position/velocity
parity for this scene, superseding the earlier cold-start-only limitation.
State beyond frame 7 is not captured; the 600-tick claim covers RNG behavior.

Final validation for this live-replay change: `cargo gate` passed with the
local traces present (both live tests and 38 hsd-particle tests total),
`cargo clippy --workspace --all-targets -- -D warnings` passed, and
`cd harness && UV_CACHE_DIR=/tmp/melee-uv-cache uv run python -m pytest -q`
passed (181 tests). No commits, trace edits, or dump-format changes.

## M4-T2 running dust (2026-09-09)

The complete scene's `dash_fd_fox` gate matches 300 ticks x 49 keys, and its
new `dash_particle_rng_sites_match_the_retail_ledger_in_order` test compares
**every particle RNG branch site in order on all 300 ticks**, using the ledger
only as expected output. All dust descriptors and programs come from the owned
`EfCoData.dat`. No per-tick particle schedule or captured matrix drives runtime.

| Tick / animation GFX | Actual dispatch | Particle descriptor chain (bank 0) |
|---|---|---|
| 34 / Dash 0x3FF | async kind 6; effect-table entry 5, world position, facing and floor rotation | frame-0 DPtcl 9; A5 children 7 and 8 |
| 49 / Run 0x3FE | async kind 5; efLib_CreateGenerator_Translate_FacingDir | 263; EF children 264 and 265, blend mode 7 |
| 56 / RunBrake 0x401 | async kind 5; efLib_CreateGenerator_Translate_FacingDir | 90 |

All seven descriptors use shape 0. The three parent programs have zero-radius
emission; children 7/8/265 use the existing random disc radius and azimuth;
264 uses the existing negative-angle pre-loop selection. The task's suggested
kind-2 route is implemented for direct particle IDs, but **none of these three
recorded GFX commands takes kind 2**. The fighter draw sites also belong to the
later switch, not the early direct-ID branch.

The previously audited interpreter/emitter arithmetic already covers every new
site in this recording. These paths are now named in `rng_sites.rs` and proven
by the full-scene ledger test (counts below cover 300 ticks):

| Site / symbol offset | Count | Gate |
|---|---:|---|
| 8039E1E4 / DAD4+710 | 11 | disc emission, radius >= 0 |
| 8039E3D4 / DAD4+900 | 11 | nonnegative angle, disc modes other than 6/7 |
| 8039E088 / DAD4+5B4 | 1 | negative-angle disc pre-loop with count >= 1 |
| 8039B5E0 / 930C+22D4 | 8 | BD random target speed, even with zero velocity |
| 8039A810 / 930C+1504 | 2 | AC random size, even with zero range |
| 8039C4F8 / 930C+31EC | 4 | E4 random S flip, mode low bits == 3 |
| 8039C58C / 930C+3280 | 4 | E5 random T flip, mode low bits == 3 |
| 8039C870 / 930C+3564 | 2 | ED discrete rotation, divisions != 0 |
| 8039F250 / F05C+1F4 | 1 | initial emission count: kind 0x100 clear, rate >= 0 |

Run/brake require a **static shared AppSRT**, created by eflib.c:730-758.
Generator position and particle simulation remain local; the transform stores
world translation, facing rotation and unit scale. Negative facing sets kind
bit 0x40000. A5/EF children without their own AppSRT share the parent's owned
transform and retain local position, matching particle.c:1098-1124/1162-1190.
The lifetime test verifies the reference survives parent deletion and is
released after the last child. Descriptor-created or mutable attached AppSRTs,
AppSRT-transforming bytecode and rendering remain explicit unsupported paths.
Effect destruction now routes owned joints through hsd_8039D688's generator
expiration rules. No unrelated particle program or emitter shape was added.

The retail fusion audit was rerun for hsd_8039DAD4, hsd_8039930C and
hsd_8039F05C. Existing radius/azimuth, BD/AC and ED arithmetic was retained;
AppSRT inheritance introduces no float arithmetic. **No dash particle dump was
supplied or recorded.** The 300-tick result proves fighters, shared seed and
ordered particle draws, not independent bitwise parity of every dust particle
position/velocity. Existing idle and match-start particle-field oracles still
pass. `crates/hsd-particle/docs/PARTICLES.md` does not exist; this is the
repository's canonical particle coverage document.


### Dash particle capture follow-up (incomplete)

The subsequently supplied 300-tick dash dump now has a strict replay in
`tests/live_fd_dash.rs`. Of 442,389 comparisons, 425 still fail: all are AppSRT
display caches starting at tick 50. Generator/particle simulation and AppSRT
SRT/ownership fields match. The new `appsrt` module owns the cache fields and
ports their display update with explicit camera view / psFrameNum inputs
(`psDispSubAppSRT`, 803A1F90..2184). The dump does not contain those external
inputs, so the test remains red rather than importing expected cache outputs
or excluding fields. Full audit and results: `docs/PORT_NOTES/M4_DASH.md`.


## M4-T3 jump, aerial-jump and landing dust (2026-09-09)

The full-scene jump gate matches **300 ticks, 49 keys, zero divergences**.
`hsd-particle/tests/live_fd_jump.rs` matches **489,588 canonical fields**,
**9,544 ordered particle RNG draws** and every final seed over all 300 ticks
(final seed `0x059317FC`). This includes generator/particle numeric state and
list order. The original dash display-cache helper now lives in
`tests/support/dust_replay.rs` and is reused unchanged; jump has **zero** such
fields to exclude. Dash still matches 441,857 fields with its 532 documented
render-cache exclusions, superseding the earlier incomplete follow-up above.

| Tick | Animation request / async dispatch | Particle descriptor, bank 0 |
|---|---|---|
| 36, 106 | GFX 0x402, async kind 0, live fighter root joint | 0x59 (89) |
| 113 | GFX 0x403, async kind 0, live fighter root joint | 0x5E (94) |
| 55, 152 | Landing request 0x404, async kind 6, effect-table 0x18 | DPtcl 10 |

All three use the existing disc emitter (shape 0); no new particle opcode,
emitter math or AppSRT path was necessary. The effect layer now refreshes
attached fighter-joint matrices before particle emission. Its five external
requests and subsequent joint matrices were captured from the production
jump gate into `tests/support/jump_fd_spawns.{rs,json}`; the fixture has no
particle outputs or captured retail matrices. Particle descriptors/bytecode
remain loaded from the owned archives. Runtime has no fixture dependency.

The task's dust-spawner count is correct, but both randomized calls are
**landing** commands at 55/152. Jump and aerial-jump GFX commands take
`ftCo_09F7.c:115-133`'s kind-0 branch and consume no offset RNG. The previously
ported block-70 sites FCDC/FD00/FD24 therefore execute twice each. Particle
sites below are existing, audited paths newly verified by the jump field oracle:

| Retail site / symbol offset | Count | Behavior |
|---|---:|---|
| 8039B5E0 / 930C+22D4 | 20 | BD random target speed |
| 8039A810 / 930C+1504 | 8 | AC random size |
| 8039E1E4 / DAD4+710 | 9 | disc radius |
| 8039E3D4 / DAD4+900 | 9 | disc azimuth |
| 8039E088 / DAD4+5B4 | 4 | negative-angle disc pre-loop |

The complete ledger has 9,554 draws: 9,544 particle, six landing offset,
two Wait-choice and two stage draws. `melee-sim/tests/m4_gate.rs` independently
checks the full-scene ordered particle sites with produced RNG. See
[the fighter report](PORT_NOTES/M4_JUMP.md) for all commands.
