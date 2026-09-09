# Dynamic bones: implementation and verification

## Current acceptance: tick SRT and post-render matrices

The reviewer-approved oracle split is now green, with no ignored bone tests:

| Oracle | P0 | P1 | Compared fields |
| --- | ---: | ---: | --- |
| Start scheduler boundary | 130/130 ticks | 130/130 ticks | rotate/scale/translate, all 73 bones |
| Idle scheduler boundary | 8/8 ticks | 8/8 ticks | rotate/scale/translate, all 73 bones |
| Start post-render VI | 130/130 aligned and matching | 130/130 aligned and matching | non-dirty bone mtx[0..11] |

The sole SRT exclusion is Euler `rotate[3]` when `JOBJ_USE_QUATERNION`
is clear; the C/asm justification remains below. Quaternion W is compared.
There are no SRT tolerances or dirty-bone exclusions. Dynamics bones 17–20,
fingers/part-animation bones, and all other bones now match. No discrepancy
remains in either oracle.

### Why matrices use the VI oracle

`HSD_JObjGetMtxPtr` in `third_party/melee-decomp/src/sysdolphin/baselib/jobj.h:697–701`
calls `HSD_JObjSetupMatrix` before returning the cache. Display reaches those
lazy demands through `HSD_JObjDispAll` (jobj.c:567–595), `HSD_JObjDisp`
(displayfunc.c:479), and envelope matrix setup (pobj.c:1151/1168).
A scheduler-boundary dump therefore contains a mixture of matrices rebuilt
by simulation queries and caches left from the previous render. Those caches
are not an oracle for that tick's newly evaluated pose. Per reviewer decision,
tick tests now compare SRT only; **matrices are tested separately**, not with
a one-tick shift or a weakened numerical comparison.

`start_fox_matrices_vi_130` groups the singleton M2 records in
`start_fd_fox.bones_vi_p0.jsonl` and `start_fd_fox.bones_vi_p1.jsonl` by frame.
Both files use `p0` keys; the filename selects the replay fighter. The test
matches metadata `(cur_anim_frame, cur_pos.x, cur_pos.y, cur_pos.z)` by exact
float bits against the same full scheduler replay used for the SRT gate.
It chooses the earliest matching tick at or after the preceding match,
allowing repeated VI samples of a stationary tick. Chronology matters:
Landing and Wait can have identical frame/position keys. Neither bone values
nor an assumed VI/tick index offset are used for alignment. Records without
a chronological matching tick are skipped and printed; none are skipped in
these captures. Tick zero is the imported savestate boundary, as in the SRT
test. No later oracle row changes the replay state.

For each tick, a cloned skeleton services `setup_matrix` demands for the
fighter's joints. This evaluates the current pose without modifying simulation
caches. Only the bones explicitly listed in that VI record's `dirty_bones`
are excluded, because retail did not rebuild them. Each player's first three
VI frames list all 73 bones as dirty: these align but compare zero words.
Thus each fighter has **127 frames with nonempty matrix coverage**. P0 checks
102,900 matrix words and P1 checks 99,120, all bit-exact (202,020 total).

### Motion-entry ordering correction

Two steps were missing or misplaced in `Fighter_ChangeMotionState`
(0x800693AC):

1. At fighter.c:996–997, retail calls `ftAnim_80070F28` (0x80070F28)
   and then `ftAnim_80070E74` (0x80070E74), **before** the new motion attachment
   and descriptor-pose reset. The first clears temporary part ownership
   (`flags_b5`, current selector x11); the second reinstalls persistent
   selections (previous selector x10), if any. The port now does the same.
   Retaining temporary part flags had suppressed the finger rest-pose reset
   at Wait entry, leaving bone 45 rotate[0] as BE32B000 instead of BE32B8C7
   beginning at tick 105.
2. The new main motion is evaluated at fighter.c:1298 and commands run at
   1342–1347. That path does **not** call `ftAnim_800707B0` (0x800707B0).
   Ordinary `ftAnim_8006EBA4` (0x8006EBA4), ftanim.c:380–385, calls main
   playback, commands, then part blends. Motion entry now advances main
   playback and commands only. This removes the extra part blend on the
   Landing transition (the former tick-75 finger discrepancy).

`cd harness && UV_CACHE_DIR=/tmp/melee-uv-cache uv run python asm.py
ftAnim_80070F28 --fused` and the same command for `ftAnim_80070E74` report
no fused sites. These additions only change flags, selectors and call order;
no new arithmetic or solver fusion sites were introduced. The previously
audited `ftAnim_ApplyPartAnim` arithmetic is reused for persistent selections.
No hsd-anim helpers or solver arithmetic changes were needed.

Current changes are confined to `melee-ft`: `src/anim/playback.rs` exposes
main-only stepping within the crate; `src/fighter/{commands,spawn}.rs` restores
retail motion-entry order; `tests/fighter_support/{mod,replay,rendered_pose}.rs`
and `tests/start_fox_bones_130.rs` implement the oracle split and shared replay
observer; this README documents the results. No commits were made.

Final validation for this revision:

- `cargo test -p melee-ft --test start_fox_bones_130 -- --include-ignored --nocapture`:
  all four tests passed before removing the two remaining ignores; output
  reports the VI counts above.
- `cargo gate` after removing the ignores: **540 passed, 0 failed, 1 ignored**
  (the remaining ignore is the pre-existing melee-sim schema doctest).
  Includes unignored `start_fox_600`, both SRT oracles, the VI matrix oracle,
  and the native-C arithmetic oracles.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `git diff --check`: clean. Cargo commands were run sequentially.

## Historical boundary-import investigation

The following report and complete per-tick mismatch inventory describe the
**previous** cache/SRT comparison contract, before the reviewer-approved oracle
split and the motion-entry fixes above. Its failure counts and ignored-test
status are historical, retained to show the effect of the boundary imports.


The full `start_fox_600` replay passes both fighters, 600 records each,
all 24 fields and all 16 fighter RNG draws. **Bone-oracle acceptance remains
incomplete after the reviewer-requested boundary imports.** The sole comparison
exclusion is now unused Euler `rotate[3]`, as approved below. Both oracle tests
remain ignored in the normal gate because their explicit runs still fail.
No matrix or other field exclusions, tolerances or expected values changed.

## Follow-up results after boundary imports

Before changing import or runtime behavior, both oracle tests were run with
only the approved Euler W exclusion. The complete 138-row inventory below
lists every differing bone/field/component for both players through seven
lossless patterns. Before imports, start has 127,716 differing words and idle
has 7,308. These sets do **not** converge: start is clean at ticks 0–6, has
540 differences at 7–11 and 1,080 at 12–74, then mostly 1,032–1,038;
idle grows from 84 at tick 0 to 1,032 on every tick 1–7. P0/P1 start
onsets are five ticks apart. The larger counts are entirely cached matrices;
finger X-rotation differences begin at ticks 75/80, recur at 105/110, and
persist on some later ticks. The final row still has 1,034 differing words.

Boundary work, confined to test fixture import:

- Verified the existing raw-Fighter +0x2F0 sentinel and +0x2F4 descriptor
  import (stride 0x18), including node count and multipliers. The raw Fighter
  contains a pointer to linked DynamicsData, not the pointed-to spring bytes;
  those positions, lengths, velocity axes and angular velocities are already
  restored from the owned savestate heap. No pose-derived reset replaces that
  history and no later raw row initializes runtime state.
- Added raw frame-zero import of all five +0x8B0 part-animation slots:
  state, duration, progress, rate, previous/current selectors, and canonical
  ftData bone-list bindings. Both players' slots have previous/current = -1
  at both boundaries. Their unused scalar words are retained verbatim, but
  `ftAnim_800707B0` does not execute an inactive slot. There is no missing
  active part-animation playback state to reconstruct at these boundaries.
- Imported main-tree rotations, scale, translation and cached matrices from
  the **tick-zero bone oracle only**. This includes every joint rather than
  a hand-maintained subset: the initial inventory also found stale palm caches
  on bones 26 and 56, outside the dynamics and part-animation lists. Main-tree
  flags and accumulated scale, blend-tree SRT/caches, and linked spring state
  remain supplied by the saved boundary. Later oracle rows are comparison
  targets only. Both tests are now exact on every included tick-zero word.

After imports the start inventory is unchanged; idle tick 0 changes from
patterns P1/P2 to no differences, and ticks 1–7 remain exactly P3/P3.
Thus the complete post-import inventory is the table below with only that
single idle row changed to `0 | 0 | 0 | — | —`. Post-import totals are
127,716 start and 7,224 idle differing words. No additional fields are ignored.

| First remaining discrepancy | Expected bits | Actual bits | Classification |
| --- | --- | --- | --- |
| Start t7, P0, bone 8, mtx[0] | 3F703E4D | 3F800000 | other; port MTX_DIRTY |
| Idle t1, P0, bone 10, mtx[0] | 3F6B0076 | 3F6B5B0F | other; port MTX_DIRTY |
| Start t75, P0, bone 43, rotate[0] | 00000000 | 3ECD8400 | part-animation |
| Start t105–107, P0, bone 45, rotate[0] | BE32B8C7 | BE32B000 | persistent part-animation |

### Why further boundary import cannot remove these differences

A temporary diagnostic rebuilt selected matrices on a **cloned tree**, never
on the replay state and never as a replacement comparison. The oracle's
non-demanded matrix caches track the preceding tick's rebuilt matrix:

| Oracle word | Oracle | Rebuilt same tick | Rebuilt prior tick | Prior-tick matrix words matching |
| --- | --- | --- | --- | ---: |
| idle t1 P0 bone 10 mtx[0] | 3F6B0076 | 3F6ADB9C | 3F6B0076 | 12/12 |
| idle t2 P0 bone 10 mtx[0] | 3F6ADB9C | 3F6AC2EF | 3F6ADB9C | 12/12 |
| start t7 P0 bone 19 mtx[0] | 3DE6F0BA | BE3BDC7E | 3DE6F0BA | 12/12 |
| start t8 P0 bone 19 mtx[0] | BE3BDC7E | 3E85C4DC | BE3BDC7E | 12/12 |

This points to missing between-tick display-side matrix demands, not erroneous
spring arithmetic. The relevant retail paths are `HSD_JObjDispAll`
(jobj.c:567–595), `HSD_JObjDisp` (displayfunc.c:479), and envelope setup's
`HSD_JObjSetupMatrix(envelope->jobj)` (pobj.c:1151/1168). Simulation procs
only rebuild joints queried by collision/camera; the scene does not execute
those display demands between scheduler boundaries. Rebuilding everything
before comparison would produce the **current** pose and violate the cache
contract. No such workaround or dirty-bone exclusion was applied.

The first non-cache discrepancy is the t75 finger rotation above. Suspected
C-order site: `Fighter_ChangeMotionState` evaluates main animation through
`ftAnim_8006E9B4` (fighter.c:1274/1298), then executes the action script
(fighter.c:1342–1347); ordinary `ftAnim_8006EBA4` separately calls
`ftAnim_800707B0` after main playback and commands (ftanim.c:380–385).
The port's `change_motion_state` calls `step_animation`, which also advances
part blends immediately. That is a candidate extra part evaluation on the
Landing/Wait transition tick; the persistent bone-45/49 values also warrant
checking pose reset versus command-installed part poses. This is a suspected
runtime evaluator/order discrepancy, not a proven solver error. In accordance
with the follow-up's stop condition, no evaluator or display-path changes
were made, and neither ignored oracle test was enabled.

Full component diagnostics can be reproduced without editing tests:

```sh
MELEE_BONE_MISMATCH_DETAILS=1 cargo test -p melee-ft --test start_fox_bones_130 -- --include-ignored --nocapture --test-threads=1
```

`BONE_DIFF` JSON lines enumerate scene, tick, player, bone, field, component,
expected/actual bits and category, including empty ticks. The before-import
inventory was captured before the three fixture additions above; its table
is retained so the effect of those imports is reviewable.

## Follow-up validation and files

- Baseline `cargo gate`: 536 passed, 0 failed, 3 ignored.
- Explicit `start_fox_bones_130 --include-ignored` run: exclusion regression
  test passes; both oracle tests run all their records and fail on the exact
  post-import differences above. They remain ignored, with updated reasons.
- Final `cargo gate`: **537 passed, 0 failed, 3 ignored** (two bone oracles
  and the pre-existing doctest). The un-ignored 600-tick match-start replay,
  existing idle replay and 50,000-step native-C oracle all pass.
- `cargo clippy --workspace --all-targets -- -D warnings`: **clean**.
- `git diff --check`: clean. One cargo command at a time; no commits.

This follow-up changes only five files under melee-ft:

- `src/dynamics/README.md`: exclusion evidence, complete before-import table,
  post-import inventory, cache/evaluator diagnostics and validation.
- `tests/fighter_support/mod.rs`: raw boundary part-animation state/bindings.
- `tests/fighter_support/saved_pose.rs`: tick-zero bone-oracle SRT/cache import.
- `tests/fighter_support/replay.rs`: invoke boundary import; mode-aware W
  exclusion; complete optional component diagnostics.
- `tests/start_fox_bones_130.rs`: precise remaining ignore reasons and a
  regression test retaining quaternion W and every other component.

No solver, evaluator, renderer, hsd-anim, harness, trace or protected-file
changes were made in this follow-up. The following sections retain the
original port's implementation and audit context.

## Structure and retail sources

- `melee-lb/src/dynamics.rs`: owned `DynamicBoneSet`, `BoneSpring` and
  `SpringParameters`; allocation/setup from `lb_8000FD48` (8000FD48), parameter
  application from `lb_80011710` (80011710), and `lb_8001044C` (8001044C,
  size 11A8). The solver builds natural/current matrices, applies stiffness,
  gravity, external force, inertia and angular constraints, checks colliders
  and floors, updates angular velocity, converts the correction through
  quaternions, damps the dominant Euler axis, and advances the parent matrix.
- Small helpers retain the C order. `floor_plane` is `lb_800103D8` (800103D8),
  `force_at` is `lb_800101C8` (800101C8, forwarded by 800103B8), and the private
  segment/sphere predicate is `lbColl_80005C44` (80005C44).
  Force fields are supplied in scene order; this does not implement the
  scene's eight-slot force allocation/expiry registry (`lb_800100B0` etc.).
- `melee-lb/src/dynamics/arithmetic.rs`: `lbVector_Normalize` (8000D2EC),
  `lbVector_Angle` (8000D620), `lbVector_RotateAboutUnitAxis` (8000D8F4),
  `lbVector_CreateEulerMatrix` (8000E530). These use lbvector's polynomial
  sine/cosine and MSL's estimate/Newton sqrt; quaternion operations reuse
  the existing audited hsd-anim helpers and SDK paired-single kernels.
- `melee-ft/src/dynamics/mod.rs`: reads the complete ftData +2C spring
  descriptors and implements `ftCo_8009CB40` selection/animation locking.
  `Fighter::prepare` constructs the chains. Motion changes enable or disable
  them, including the 0x100 sentinel. `proc_dynamics_with_map` supplies
  `mpCheckFloor`; the original no-map method remains for paths without active
  stage-floor queries. Sets execute in descriptor order after collider updates,
  following `Fighter_8006D9AC` / `ftCo_8009DD94` (8006D9AC / 8009DD94).
- The existing savestate reader restores the initial linked DynamicsData
  state, saved SRT, flags and matrix caches for main/blend skeletons. Only
  tick zero is imported. Later bone records are assertions, never inputs.
  The replay checks tick and RNG alignment against the separate bone capture's
  raw trace when it is present. No hsd-anim additions were needed.

Integration is for the existing Fox fixture. Other characters' special
costume/hat rules, Mewtwo/Peach force start indices, Flatzone scaling and the
scene force registry are not integrated or certified. The generic solver
accepts colliders, forces, force start index, pause and floor providers.

## Fox's on-disc set

PlFx.dat ftData +2C points to archive data offset 297C. There is one set,
root 17, count 4: joints **17, 18, 19, 20**. Joints 17–19 are springs; 20 is
the terminal position node. All four parameter entries are identical:

| Parameter | Value |
| --- | --- |
| DynamicsDesc multipliers | (1, 1, 0.04363323003053665) |
| stiffness / convergence | 1 / 0 |
| natural Euler rotation / stored W | (0, 0, 0) / 0 |
| maximum deviation | 0.7853981852531433 |
| stored rotation max/min | (+pi, +pi, +pi) / (-pi, -pi, -pi), f32 |
| angular damping | 0.008726646192371845 |
| maximum step | 0.05235987901687622 |

Gravity is the descriptor Z multiplier divided by each link's saved length.
Fox has one dynamics collider, attached to bone 41; the existing reader
already retained its offset/radius. P0's set is active through tick 104 and
becomes disabled at Wait on tick 105; P1 follows five ticks later.

The 600-tick RNG ledger has no draw whose call site is inside the solver.
The replay asserts this and still matches the ten Wait draws and six landing
requests. No RNG object enters the generic solver.

## Fusion audit

Executed `cd harness && UV_CACHE_DIR=/tmp/melee-uv-cache uv run python asm.py
<symbols> --fused`; the cache override avoids a sandbox write outside the
workspace. The solver has **18** fused instructions:

| Purpose | Retail instructions |
| --- | --- |
| Collider endpoint | 80010AE4, 80010AF8, 80010B0C `fmadds` |
| Distance sqrt refinements | 80010B78, 80010B88, 80010B98 `fnmsub` (double) |
| Tangent side squared | 80010C04 `fmsubs` |
| Tangent sqrt refinements | 80010C1C, 80010C2C, 80010C3C `fnmsub` (double) |
| Ground endpoints | 80010CF8, 80010CFC, 80010EC4, 80010EC8 `fmadds` |
| Ground horizontal squared | 80010F3C `fnmsubs` |
| Ground sqrt refinements | 80010F54, 80010F64, 80010F74 `fnmsub` (double) |

Additional audited sites are cited next to their expressions:
initial link lengths 8000FFA8/FFAC; floor intersection 80010428;
force accumulation 800102B0/C0/D0 and 80010340/354/368;
angle dot 8000D748/750; polynomial/axis rotations 8000D9D8/D9DC,
8000DA3C/DA40 and 8000DA6C..DADC; Euler polynomial/entries
8000E5A8..E808; segment/sphere 80005DDC..80005E9C.
`lb_800100B0`, `lb_800103B8`, `lb_80011710`, `ftCo_8009DD94` and
`Fighter_8006D9AC` have no fused instructions. Audited HSD callees include
8037ECE0, 8037EC4C, 8037EB28, EulerToQuat, PSMTXQuat/Concat/MultVec and
PSVECCrossProduct; those implementations were reused without edits.

## Original port results before reviewer decisions

The match-start initial pose matches **73 bones × 22 words × 2 fighters =
3,212 words**. Both fighters run through all 130 records. Across those records,
289,211 / 417,560 words match. All **3,120 XYZ rotation words** on bones
17–20 match (130 × 2 × 4 × 3). This partial diagnostic does not waive the
remaining fields: the test still fails on its first mismatch.

**First match-start mismatch:** tick 1, P0, bone 17, `rotate[3]`, in the
dynamics set: expected **38100000**, actual **00000000**. At ticks 1–6 the
only differences are this unused Euler W word on joints 17–19 of both players.
Retail `HSD_QuatLib_8037EB28` writes a Vec3 at solver stack +6C. Instructions
80011250/80011254 load +74 and **+78**, and 80011258/8001125C copy both words
into JObj rotate Z/W. The solver never writes stack +78. The port preserves
its prior W; it does not invent a constant or feed later oracle words into
runtime state. Modeling that stack history is outside the owned solver.

There are also substantive remaining pose/cache differences:

- At start tick 7, P0 bone 8 `mtx[0]` (outside dynamics): expected 3F703E4D,
  actual 3F800000. The port leaves this cache dirty (`flags=00000048`).
  Tail joints 19/20 also acquire cached-matrix differences. No render-side
  matrix-demand traversal was added, and a blanket matrix rebuild would not
  preserve the retail stale-cache contract.
- At start tick 75 the first non-W rotation differences are P0 finger joints
  43/44/45/47/48/49, X rotation, outside dynamics. This remains unresolved.
- **First idle mismatch:** tick 0, P0 bone 17 `mtx[0]`, in dynamics: expected
  **BEA99E6C**, actual **BE7B72FD**. There are 84 differing initial cached
  matrix words; the savestate import's joint flags are 00000049. The two
  captures have matching tick/seed boundaries, but the tick bone dump does
  not serialize JObj flags. No game-dirty exception can be proven from it,
  so all 8 idle records are compared and the test fails.

`dynamics_one_bone_50000` compiles independent decomp excerpts plus the
retail-fused lbvector/MSL helpers with contraction disabled. All 50,000
random directional steps / 150,000 output components match bit-for-bit.
It covers stiffness, gravity, angular inertia and angular constraints. It
**does not** certify collision branches, external force fields, quaternion
matrix conversion, render caches or the unused stack word. A separate test
checks that the stored verbatim C excerpts still match the pinned decomp.

## Original port checks before reviewer decisions

- Baseline `cargo gate`: 533 passed, 0 failed, 2 ignored.
- `cargo test -p melee-ft --test start_fox_600`: passes, un-ignored.
- `cargo test -p melee-lb --test dynamics_ref_oracle`: 2 passed.
- `cargo gate --no-fail-fast`: **536 passed, 2 failed, 1 ignored**. Only
  `start_fox_bones_130` and `idle_fox_bones_8` fail. Existing idle replay passes.
- `cargo clippy --workspace --all-targets -- -D warnings`: **clean**.

No commits, game-data writes, harness edits, other-crate changes or changes
to TRACKER.md, CLAUDE.md, root Cargo.toml or third_party.

Changed files, relative to crates/:

- melee-lb/src/lib.rs, dynamics.rs, dynamics/arithmetic.rs
- melee-lb/tests/dynamics_ref_oracle.rs
- melee-lb/tests/ref/dynamics/{driver.c,lbvector.c.inc,retail_vector.c.inc,
  spring_step.c.inc,NOTICE}
- melee-ft/src/lib.rs, dynamics/{mod.rs,README.md}
- melee-ft/src/fighter/{assets.rs,mod.rs,procs.rs,spawn.rs,README.md,START_FOX.md}
- melee-ft/tests/{start_fox_600.rs,start_fox_bones_130.rs}
- melee-ft/tests/fighter_support/{mod.rs,replay.rs,saved_pose.rs}

## Reviewer-approved Euler W exclusion

The comparison now omits **only `rotate[3]` on joints whose current
`JOBJ_USE_QUATERNION` flag (1 << 17) is clear**. Quaternion-mode joints still
compare all four words. No matrices, XYZ rotation, scale or translation words
are excluded, including dirty matrix caches.

C evidence: `lb_8001044C`, lb_00F9.c, declares a Quaternion `rotation`, then
passes it as `(Vec3*) &rotation` to `HSD_QuatLib_8037EB28`. That helper writes
only XYZ; `HSD_JObjSetRotation` copies all four words. Retail 0x80011224 passes
stack +0x6C to the Vec3 writer; 0x80011254 loads the unwritten word at +0x78;
0x8001125C stores it to JObj +0x28 (rotate.w). There is no store to stack +0x78
in this solver. `HSD_JObjMakeMatrix` chooses the Euler matrix path when the
quaternion flag is clear, and that path uses rotate.x/y/z only. The exclusion
follows the reviewer's explicit decision and does not normalize expected data.

## Before boundary-import changes: complete mismatch inventory

Captured with only the reviewer-approved Euler W exclusion; no runtime or
boundary-import changes preceded this measurement. Every tick and both fighters
are listed below. Pattern IDs are lossless: their definitions enumerate every
differing bone, field and component index. A dash means zero differing words.
`dynamics` = bones 17–20; `part` = membership in ftData animation bone lists
(27–38, 42–49, 51, 57–69); `other` = all remaining bones. `mtx[0–11]` means
all twelve individual components, not a matrix-level mismatch count.

| Scene | Tick | P0 words | P1 words | Total | P0 pattern | P1 pattern |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| idle | 0 | 36 | 48 | 84 | P1 | P2 |
| idle | 1 | 516 | 516 | 1032 | P3 | P3 |
| idle | 2 | 516 | 516 | 1032 | P3 | P3 |
| idle | 3 | 516 | 516 | 1032 | P3 | P3 |
| idle | 4 | 516 | 516 | 1032 | P3 | P3 |
| idle | 5 | 516 | 516 | 1032 | P3 | P3 |
| idle | 6 | 516 | 516 | 1032 | P3 | P3 |
| idle | 7 | 516 | 516 | 1032 | P3 | P3 |
| start | 0 | 0 | 0 | 0 | — | — |
| start | 1 | 0 | 0 | 0 | — | — |
| start | 2 | 0 | 0 | 0 | — | — |
| start | 3 | 0 | 0 | 0 | — | — |
| start | 4 | 0 | 0 | 0 | — | — |
| start | 5 | 0 | 0 | 0 | — | — |
| start | 6 | 0 | 0 | 0 | — | — |
| start | 7 | 540 | 0 | 540 | P4 | — |
| start | 8 | 540 | 0 | 540 | P4 | — |
| start | 9 | 540 | 0 | 540 | P4 | — |
| start | 10 | 540 | 0 | 540 | P4 | — |
| start | 11 | 540 | 0 | 540 | P4 | — |
| start | 12 | 540 | 540 | 1080 | P4 | P4 |
| start | 13 | 540 | 540 | 1080 | P4 | P4 |
| start | 14 | 540 | 540 | 1080 | P4 | P4 |
| start | 15 | 540 | 540 | 1080 | P4 | P4 |
| start | 16 | 540 | 540 | 1080 | P4 | P4 |
| start | 17 | 540 | 540 | 1080 | P4 | P4 |
| start | 18 | 540 | 540 | 1080 | P4 | P4 |
| start | 19 | 540 | 540 | 1080 | P4 | P4 |
| start | 20 | 540 | 540 | 1080 | P4 | P4 |
| start | 21 | 540 | 540 | 1080 | P4 | P4 |
| start | 22 | 540 | 540 | 1080 | P4 | P4 |
| start | 23 | 540 | 540 | 1080 | P4 | P4 |
| start | 24 | 540 | 540 | 1080 | P4 | P4 |
| start | 25 | 540 | 540 | 1080 | P4 | P4 |
| start | 26 | 540 | 540 | 1080 | P4 | P4 |
| start | 27 | 540 | 540 | 1080 | P4 | P4 |
| start | 28 | 540 | 540 | 1080 | P4 | P4 |
| start | 29 | 540 | 540 | 1080 | P4 | P4 |
| start | 30 | 540 | 540 | 1080 | P4 | P4 |
| start | 31 | 540 | 540 | 1080 | P4 | P4 |
| start | 32 | 540 | 540 | 1080 | P4 | P4 |
| start | 33 | 540 | 540 | 1080 | P4 | P4 |
| start | 34 | 540 | 540 | 1080 | P4 | P4 |
| start | 35 | 540 | 540 | 1080 | P4 | P4 |
| start | 36 | 540 | 540 | 1080 | P4 | P4 |
| start | 37 | 540 | 540 | 1080 | P4 | P4 |
| start | 38 | 540 | 540 | 1080 | P4 | P4 |
| start | 39 | 540 | 540 | 1080 | P4 | P4 |
| start | 40 | 540 | 540 | 1080 | P4 | P4 |
| start | 41 | 540 | 540 | 1080 | P4 | P4 |
| start | 42 | 540 | 540 | 1080 | P4 | P4 |
| start | 43 | 540 | 540 | 1080 | P4 | P4 |
| start | 44 | 540 | 540 | 1080 | P4 | P4 |
| start | 45 | 540 | 540 | 1080 | P4 | P4 |
| start | 46 | 540 | 540 | 1080 | P4 | P4 |
| start | 47 | 540 | 540 | 1080 | P4 | P4 |
| start | 48 | 540 | 540 | 1080 | P4 | P4 |
| start | 49 | 540 | 540 | 1080 | P4 | P4 |
| start | 50 | 540 | 540 | 1080 | P4 | P4 |
| start | 51 | 540 | 540 | 1080 | P4 | P4 |
| start | 52 | 540 | 540 | 1080 | P4 | P4 |
| start | 53 | 540 | 540 | 1080 | P4 | P4 |
| start | 54 | 540 | 540 | 1080 | P4 | P4 |
| start | 55 | 540 | 540 | 1080 | P4 | P4 |
| start | 56 | 540 | 540 | 1080 | P4 | P4 |
| start | 57 | 540 | 540 | 1080 | P4 | P4 |
| start | 58 | 540 | 540 | 1080 | P4 | P4 |
| start | 59 | 540 | 540 | 1080 | P4 | P4 |
| start | 60 | 540 | 540 | 1080 | P4 | P4 |
| start | 61 | 540 | 540 | 1080 | P4 | P4 |
| start | 62 | 540 | 540 | 1080 | P4 | P4 |
| start | 63 | 540 | 540 | 1080 | P4 | P4 |
| start | 64 | 540 | 540 | 1080 | P4 | P4 |
| start | 65 | 540 | 540 | 1080 | P4 | P4 |
| start | 66 | 540 | 540 | 1080 | P4 | P4 |
| start | 67 | 540 | 540 | 1080 | P4 | P4 |
| start | 68 | 540 | 540 | 1080 | P4 | P4 |
| start | 69 | 540 | 540 | 1080 | P4 | P4 |
| start | 70 | 540 | 540 | 1080 | P4 | P4 |
| start | 71 | 540 | 540 | 1080 | P4 | P4 |
| start | 72 | 540 | 540 | 1080 | P4 | P4 |
| start | 73 | 540 | 540 | 1080 | P4 | P4 |
| start | 74 | 540 | 540 | 1080 | P4 | P4 |
| start | 75 | 522 | 540 | 1062 | P5 | P4 |
| start | 76 | 516 | 540 | 1056 | P3 | P4 |
| start | 77 | 516 | 540 | 1056 | P3 | P4 |
| start | 78 | 516 | 540 | 1056 | P3 | P4 |
| start | 79 | 516 | 540 | 1056 | P3 | P4 |
| start | 80 | 516 | 522 | 1038 | P3 | P5 |
| start | 81 | 516 | 516 | 1032 | P3 | P3 |
| start | 82 | 516 | 516 | 1032 | P3 | P3 |
| start | 83 | 516 | 516 | 1032 | P3 | P3 |
| start | 84 | 516 | 516 | 1032 | P3 | P3 |
| start | 85 | 516 | 516 | 1032 | P3 | P3 |
| start | 86 | 516 | 516 | 1032 | P3 | P3 |
| start | 87 | 516 | 516 | 1032 | P3 | P3 |
| start | 88 | 516 | 516 | 1032 | P3 | P3 |
| start | 89 | 516 | 516 | 1032 | P3 | P3 |
| start | 90 | 516 | 516 | 1032 | P3 | P3 |
| start | 91 | 516 | 516 | 1032 | P3 | P3 |
| start | 92 | 516 | 516 | 1032 | P3 | P3 |
| start | 93 | 516 | 516 | 1032 | P3 | P3 |
| start | 94 | 516 | 516 | 1032 | P3 | P3 |
| start | 95 | 516 | 516 | 1032 | P3 | P3 |
| start | 96 | 516 | 516 | 1032 | P3 | P3 |
| start | 97 | 516 | 516 | 1032 | P3 | P3 |
| start | 98 | 516 | 516 | 1032 | P3 | P3 |
| start | 99 | 516 | 516 | 1032 | P3 | P3 |
| start | 100 | 516 | 516 | 1032 | P3 | P3 |
| start | 101 | 516 | 516 | 1032 | P3 | P3 |
| start | 102 | 516 | 516 | 1032 | P3 | P3 |
| start | 103 | 516 | 516 | 1032 | P3 | P3 |
| start | 104 | 516 | 516 | 1032 | P3 | P3 |
| start | 105 | 518 | 516 | 1034 | P6 | P3 |
| start | 106 | 518 | 516 | 1034 | P6 | P3 |
| start | 107 | 518 | 516 | 1034 | P6 | P3 |
| start | 108 | 520 | 516 | 1036 | P7 | P3 |
| start | 109 | 518 | 516 | 1034 | P6 | P3 |
| start | 110 | 518 | 518 | 1036 | P6 | P6 |
| start | 111 | 518 | 518 | 1036 | P6 | P6 |
| start | 112 | 518 | 518 | 1036 | P6 | P6 |
| start | 113 | 518 | 520 | 1038 | P6 | P7 |
| start | 114 | 518 | 518 | 1036 | P6 | P6 |
| start | 115 | 518 | 518 | 1036 | P6 | P6 |
| start | 116 | 518 | 518 | 1036 | P6 | P6 |
| start | 117 | 518 | 518 | 1036 | P6 | P6 |
| start | 118 | 518 | 518 | 1036 | P6 | P6 |
| start | 119 | 518 | 518 | 1036 | P6 | P6 |
| start | 120 | 518 | 518 | 1036 | P6 | P6 |
| start | 121 | 518 | 518 | 1036 | P6 | P6 |
| start | 122 | 518 | 518 | 1036 | P6 | P6 |
| start | 123 | 518 | 518 | 1036 | P6 | P6 |
| start | 124 | 518 | 518 | 1036 | P6 | P6 |
| start | 125 | 516 | 518 | 1034 | P3 | P6 |
| start | 126 | 516 | 518 | 1034 | P3 | P6 |
| start | 127 | 516 | 518 | 1034 | P3 | P6 |
| start | 128 | 516 | 518 | 1034 | P3 | P6 |
| start | 129 | 516 | 518 | 1034 | P3 | P6 |

| Pattern | Complete per-bone, per-field component differences |
| --- | --- |
| P1 | **dynamics** bones 17–18: `mtx[0–11]`<br>**other** bones 26: `mtx[0–11]` |
| P2 | **dynamics** bones 17–18: `mtx[0–11]`<br>**other** bones 26,56: `mtx[0–11]` |
| P3 | **dynamics** bones 19–20: `mtx[0–11]`<br>**other** bones 10,14–16,39,50,52,70: `mtx[0–11]`<br>**part** bones 27–38,42–49,51,57–66,68–69: `mtx[0–11]` |
| P4 | **dynamics** bones 19–20: `mtx[0–11]`<br>**other** bones 8–10,14–16,39,50,52,70: `mtx[0–11]`<br>**part** bones 27–38,42–49,51,57–66,68–69: `mtx[0–11]` |
| P5 | **dynamics** bones 19–20: `mtx[0–11]`<br>**other** bones 10,14–16,39,50,52,70: `mtx[0–11]`<br>**part** bones 27–38,42,46,51,57–66,68–69: `mtx[0–11]`<br>**part** bones 43–45,47–49: `mtx[0–11]; rotate[0]` |
| P6 | **dynamics** bones 19–20: `mtx[0–11]`<br>**other** bones 10,14–16,39,50,52,70: `mtx[0–11]`<br>**part** bones 27–38,42–44,46–48,51,57–66,68–69: `mtx[0–11]`<br>**part** bones 45,49: `mtx[0–11]; rotate[0]` |
| P7 | **dynamics** bones 19–20: `mtx[0–11]`<br>**other** bones 10,14–16,39,50,52,70: `mtx[0–11]`<br>**part** bones 27–38,43–44,47–48,51,57–66,68–69: `mtx[0–11]`<br>**part** bones 45,49: `mtx[0–11]; rotate[0]`<br>**part** bones 42,46: `mtx[0–11]; translate[1]` |
