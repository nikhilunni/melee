# Match-start FD replay

`cargo test -p hsd-particle --test live_fd_start -- --nocapture` matches
600/600 completed ticks of `start_fd_fox`: 1,051,650 canonical field comparisons, 23,092
particle-system RNG draws in order, and every final seed. The other 18 draws
are consumed in their ledger order before the particle phase. Nothing resets
the shared RNG after restoration. Final seed: `0x625BB67B`. First divergence: none.

This is a particle-system boundary test. The effect layer is deliberately an
input fixture, including its animated joint matrices. It does **not** prove
that Rust evaluates the warp effect, its spline, or fighter effect requests.

## Inputs and creation provenance

The initial snapshot is empty, with family counter 256 and seed 3427901605.
Its metadata says scheduler counter 1593; completed frame 0 says counter 0.
The scheduler resets at the match boundary. Frame 0 draws nothing; the first
FD background spawn occurs at frame 1. Frame 0 and frame 1 share VI 2; each
subsequent completed tick has a new VI.

All nine external requests use link 0, default descriptor velocity, no
AppSRT, and `hsd_8039EFAC` attachment (`type |= 0x700`). `hsd_8039F05C`
initializes position to zero; `hsd_8039D214` takes position from the attached
matrix before emission. The table gives that matrix's translation, as f32
hex words. Full row-major matrices and subsequent changes are in
`tests/support/start_fd_joints.json`; joint numbers are fixture identities,
not retail allocator addresses.

| Tick | Bank/kind | Joint | Matrix translation x/y/z | Caller provenance |
|---|---|---|---|---|
| 1 | 30/30000 | 0 | 00000000/00000000/00000000 | FD map4 animation0 DPtcl -> grLib_801C99C0 |
| 6 | 0/445 | 1 | C25492A7/4126CAF8/C01F62AC | First warp, effect 0x24, frame-0 DPtcl |
| 6 | 0/449 | 2 | C2700000/4139BCF0/00000000 | First warp, second frame-0 DPtcl |
| 11 | 0/445 | 3 | 4285B6AC/4126CAF8/C01F62AC | Second warp, frame-0 DPtcl |
| 11 | 0/449 | 4 | 42700000/4139BCF0/00000000 | Second warp, second frame-0 DPtcl |
| 15 | 0/448 | 2 | C2700000/41568AFA/00000000 | First warp, frame-9 DPtcl |
| 20 | 0/448 | 4 | 42700000/41568AFA/00000000 | Second warp, frame-9 DPtcl |
| 75 | 0/10 | 5 | C2700000/38D20000/00000000 | Landing effect 0x404 -> effect 0x18, frame-0 DPtcl |
| 80 | 0/10 | 6 | 42700000/38D20000/00000000 | Second landing, same route |

C paths are relative to `third_party/melee-decomp/src`:

- `melee/ft/ft_0C31.c:50-57,75-134`: `ftCo_Entry_Anim` calls
  `ftCo_800C6408` when the entry timer expires. It queues
  `efAsync_Spawn(..., 3, 0x43E, fighter_root, &entry.x8)`. The captured
  effect roots are at (-60,10,0)/(60,10,0) with uniform scale 0.96.
- `melee/ef/efasync.c:750-756`: dispatch 0x43E creates attached effect
  0x24 and applies that scale. `eflib.c:433-536,538-555` loads its model
  and animation, queues the first animation evaluation, and attaches its
  position to the fighter. `efasync.c:1122-1126` drains that initial
  animation queue before returning. Effect 0x24's DAT animation has two
  frame-0 DPtcl keys (445,449), then a frame-9 key (448); the independent
  `effect_animation_keys_identify_external_generator_requests` test reads
  those actual FObj streams and asserts the schedule.
- `melee/ft/kinds/ftCommon/ftCo_09F7.c:197-207,250-264`:
  `ftCo_8009F834` draws three random position offsets and forwards 0x404
  with the floor angle via queue kind 5. `efasync.c:288-293` creates
  positional effect 0x18 and sets its Z rotation. The same FObj test
  verifies effect 0x18's frame-0 key is bank 0, generator 10. The RNG ledger
  alone identifies the three caller sites, not their arguments; the effect
  bank, dump program identities and C dispatch establish this route.
- `melee/ef/eflib.c:857-1013`: these IDs take the ordinary DPtcl route
  through `efLib_SpawnParticleEffect` to `hsd_8039EFAC(0,bank,kind,jobj)`.
  `melee/gr/grlib.c`'s `grLib_801C99C0` supplies the equivalent FD route.
- `sysdolphin/baselib/generator.c:1026-1244`: attachment wrapper and
  descriptor initialization. Velocity/rate/lifetimes are loaded from the
  owned bank, never copied from completed particle/generator state.

The fixture contains nine external allocations. Kind 445's program executes
`EF 01 BE 07; EF 01 BF 07; FF`: it creates 446 and 447 synchronously and
immediately deletes its own particle. Two 40-tick generators therefore make
160 internal allocations. Those children consume 160 creation draws, inherit
the parent's family and joint, and insert at the live generator cursor.
Their allocations still advance the family counter. The other three creation
draws are stage 30000 and the two 448s. The other six external requests have
negative emission rates and consume no creation draw. Final allocator: 425.

### Joint fixture boundary

`start_fd_joints.json` contains 237 changed matrices, extracted **only** from
`particles.joints[].fields.matrix` in the machine-local metadata sidecar.
Rows are `[completed_tick, normalized_joint, [12 u32 float words]]`. The
extraction is reproducible with `tests/support/extract_start_fd_joints.py`.
It reads the capture and prints JSON; it never overwrites the oracle.
No completed particle or generator numeric state, counters, child requests,
program counters, draw counts, or seeds are copied into this input fixture.
Bank bytecode and texture availability still come from owned DAT archives.

These are the effect-owned matrices used by the particle phase. They are
fixture inputs because effect animation and its spline are outside this
port. They are not independent validation of the corresponding joint
transforms. Replacing this fixture with the actual effect evaluator is a
whole-game integration task. The existing idle replay continues to restore
its single fixed joint from the initial metadata.

## Phase ordering

`fighter.c:897-911` registers fighter procs at s_links 0,1,2,3,4,6,7,8,9,
12,13,14,16,18,22. Entry animation runs in the early fighter update. Calls
made before s_link 9 are queued (`efasync.c:1458-1463`), then flushed by
`Fighter_8006C80C` at s_link 9 (`fighter.c:2552-2557`). Initial warp/landing
keys consequently run before either particle proc.

Effect animation updates also run at s_link 15, p_link 11, **priority 0**
(`eflib.c:474-481,534`). Particle main is s_link 15, p_link 11,
**priority 1** (`eflib.c:170-172`); aux is the following p_link 12,
priority 1. `gobjplink.c:35-44` orders GObjs by priority, and
`gobjproc.c:12-108` carries that order into the s_link proc list. Thus the
frame-9 warp keys at ticks 15/20 also precede main. All nine requests in this
scenario land before main; none belongs between main's particle and
generator walks or between main and aux. Internal EF child requests happen
inside immediate particle interpretation during the generator walk.

Main runs particle interpretation before generator emission, skipping links
1/2; aux skips link 0. Later fighter procs do not draw in this corpus.
After the tick snapshot, a display pass calls `particleSort` (psdisp.c,
0x8039FC70), which **mutates the simulation lists**: stable buckets by
`((kind >> 25) & 7) + (TexEdge ? 0 : 8)`. The replay performs that sort
between displayed ticks. This is required for subsequent RNG-site order;
no GX rendering is ported. The initial tick-7 mismatch before this fix was
list[0].particle[10].position[0]: expected 0xC3612A00, actual 0xC38E7888.
It was a list-order mismatch, not an arithmetic error.

External RNG callers, all before the particle phase:

| Branch | Ticks | Variant |
|---|---|---|
| 8009FCDC/8009FD00/8009FD24 | 75,80 | Randf, three position-offset draws |
| 8021AFFC | 206 | Randf, grLast_8021ADD0+0x22C |
| 8021AEC8 | 305 | Randf, grLast_8021ADD0+0xF8 |
| 8008A8BC | 225,230,345,350 twice,465 twice,470,585,590 | Randi(100), Wait selection |

## Implemented paths and audit

- `SpawnRequest`, bank registration, position/velocity overrides, and owned
  joint identity/matrix input to `ParticleSystem::spawn`.
- A5 and EF synchronous child-generator requests, inherited family/joint,
  EF blend override, live-cursor insertion, same-pass child emission,
  immediate parent-particle deletion and surviving-child lifetime retention.
- B6 relative rotation interpolation; BD randomized target speed;
  E0 shared primary/environment random channel deltas.
- `sort_for_display`: the renderer's stable mutation of particle lists.
- Restore/snapshot adapter supports multiple banks and disc auxiliary fields.

Existing audited sphere/disc/negative-angle emission and zero-rate retained
lifetime branches already implemented the newly reached sites EB04, EB5C,
E3D4, E1E4 and E088. They needed integration and regression coverage, not
replacement arithmetic. AC/BA/BB/BC/E4/E5 were likewise already implemented.
Other opcodes, AppSRT, child particles/remaps and unsupported shapes still
fail explicitly.

Audit commands (read-only; scratch outputs outside the repository):

```
cd harness
UV_CACHE_DIR=/tmp/melee-uv-cache uv run python asm.py hsd_8039F05C --fused
UV_CACHE_DIR=/tmp/melee-uv-cache uv run python asm.py hsd_8039DAD4 --fused
UV_CACHE_DIR=/tmp/melee-uv-cache uv run python asm.py hsd_8039930C --fused
```

BD uses `fmadds` at 8039B5E8; squared magnitude uses separate products and
adds at B5F0..B604, followed by the existing three-step sqrt kernel
(fnmsub B61C/B62C/B63C). B6's target addition is unfused at AADC. E0 uses
`fmuls` BB60 then separate target `fadds` BB6C/BBB0 (and corresponding
G/B/A sites). Constructor and child-inheritance changes add no float math.
The existing emission fusion citations remain in `generator.rs`.

## Validation and files

- Baseline `cargo gate`: passed before the concurrent fighter changes.
- Final `cargo test -p hsd-particle -- --nocapture`: 47 tests passed,
  including both idle tests (600 ticks each), the match-start replay and
  the effect-key provenance test.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- Final `cargo gate`: hsd-particle passed; workspace failed in the concurrently
  edited `melee-ft` test `start_fox_600`, at `fighter/procs.rs:444`,
  `not implemented: lb_00F9.c:472-935: active dynamic-bone solver`.
  Its final diagnostic identifies tick 1, p0, s_link 16, state Entry,
  with completed records [1,1]. That code was not changed by this task.
- Regenerated matrix fixture compared byte-for-byte equal with `cmp`.

Changed files (all under `crates/hsd-particle`):

- `src/system.rs`, `src/generator.rs`, `src/particle.rs`, `src/color.rs`.
- `tests/live_fd_start.rs`, `tests/start_paths.rs`, `tests/opcodes.rs`,
  `tests/real_fd_particles.rs`, `tests/support/restore.rs`.
- `tests/support/start_fd_spawns.rs`, `tests/support/start_fd_joints.json`,
  `tests/support/extract_start_fd_joints.py`.
- `START_FD.md`.

No commits; no edits to root configuration, harness/oracle data, third_party,
CLAUDE.md, TRACKER.md or other crates. The only updated existing test coverage
list adds the newly supported opcodes; no golden values or comparisons were
weakened.
