# Dynamic bones: implementation and verification limits

The full `start_fox_600` replay now runs un-ignored and passes both fighters,
600 records each, all 24 fields and all 16 fighter RNG draws. **The requested
full bone-matrix acceptance is not complete.** Both new strict bone tests
remain failing; no expected bits, dirty-bone exclusions or tolerances changed.

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

## Oracle results and first mismatches

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

## Final checks

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
