# Wave 22 integrated checkpoint

Production/checkpoint source `f1288878be775815aa6b31c39d1b2c8586a703b6`, tree
`d41b8465a00a85eaeb51b06ed27e5ddc70db48f8`. Fresh coordinator proof is
`target/wave22/main/`. The normal CLI was built after all release tests,
clippy and the test-only followup commit, then frozen with the original
3,490-scenario manifest before the full sweep and both replay corpora.
Canonical fresh baseline:
`target/sweeps/f1288878be775815aa6b31c39d1b2c8586a703b6.*`.

## Accepted changes and limits

**Held Sword common F66/Hi68 entry** (`46b28c65`, worker `fc3f1785`): thirteen
original complete captures, 12,633 rows per stream and 516,729 genuine raw
LCG edges. Runtime changes only widen the existing actual common-row audit
and immutable held-entry callback refusal; entry ownership, direct script
step, scheduler and arithmetic stay unchanged. Existing B67 and the whole
1,344-tick midair catch remain exact. Reentry with an old catch lock installs
fresh ownership. Next actual Animation/IASA, private rows, Down Air, landing
and shooting remain guarded. No new hand/cache/effect math. Details:
`HELD_SWORD_FORWARD_UP_ENTRY.md`.

**Giant Punch paired-hitbox invariant** (`9a1af8e2`, worker `36e01f38`): only
the impossible-state panic classification changes. Original Donkey Kong and
Kirby scripts create/clear hitboxes 0/1 together, including finite loops and
all 56 conservative yield/end boundaries. Actual command ownership is
PlDk.dat rows 322/323/327/328 and PlKb.dat rows 393/394/398/399, not the copy model archive.
The source proof includes immediate batches, ordinary entry clears, retained
SkipHit/UpdateCmd transitions and the typed Ness-only revival exception.
Malformed/custom states still panic; scaling remains refused. No gameplay
admission or arithmetic change. All 3,490 scenario rows are byte-identical to
its preceding baseline. Details: `GIANT_PUNCH_PAIRED_HITBOX_INVARIANT.md` and
`docs/COVERAGE_AUDIT.md`; related stale address comments were corrected from
actual assembly.

**Klaptrap coupled path/item birth** (`1e3062e1`, worker `210e5708`): three
original complete 1,666-tick captures, 4,998 rows per stream. This admits the
actual birth tick, not just a constructor: preallocated dynamic identity
Ground root/wrapper, live facing query, B-spline path/tangent/Euler math,
barrel radius rejection, item constructor/pose/script/hitbox and all real
remaining same-tick callbacks. The article emits its actual bank 30/program
30005 particle attached to DFS1 without AppSRT. Matrix proof is the recorded
birth-frame DFS1 local SRT and all 12 world matrix words for each capture;
it is not whole-bone, cache-lifecycle or pixel equality.

Next actual Ground Animation 1 and item Animation remain guarded. Other path
slot, vertical/singular/PI Euler cases, audio envelope, ended animation,
detach/catch/destruction/damage and later motion states remain refused.
Negative controls exercise their real source conditions. Raw tests use typed
live Ground-parent identity rather than equating local handles with RAM
pointers; existing FirstPosition encoding is preserved. No newborn/timer/
phase scheduling exception or hardcoded rest geometry is used. The existing
matrix helpers borrow unchanged sampled locals when no overrides are
requested, avoiding an incidental 16-joint copy limit; actual overrides keep
their limit. Actual step/observe/capture allocate nothing and before-birth
clones continue independently. Details: `WAVE22_KLAPTRAP_BIRTH.md`.

**WASM guard-test completion** (`f1288878`): the new tangent endpoint test
uses native should-panic coverage and a WASI ignored-test subprocess with
its exact expected diagnostic, matching the existing four guard tests.
Zero-test selection and unrelated traps are rejected. There is no production
arithmetic or admission change. Root's combined proof includes all five.

## Combined validation

- Release workspace: **2,006 passed, zero failed, one existing ignore**,
  across 360 reports; m4 **345**, m5 **312**. All-target workspace clippy clean.
- All nine worktree-only missing-data failures pass in MAIN without optout;
  `worktree-data-limit-resolution.json` enumerates them.
- Harness **407 passed**; schema **zero layout violations**. WASM pure suites,
  **five expected spline traps**, **6,003 identical native/WASM hash lines**
  and the release web build pass. Release only; no debug run.
- Fresh full sweep: **3,490 scenarios; 3,348 particle-exact, 115 not comparable,
  26 existing local failures and one existing particle fault**. **Zero
  regressions and 16 fixes** versus `6d0c850e`; zero regressions and **281
  fixes** versus startup `0a99db31`. Every nonfix full TSV row is identical.
  Existing incomplete recordings/failures are not accepted divergences.
- Fresh replay A: **108/108 complete, 964,782 matched frames**. Fresh replay B:
  **486/502 complete, 5,102,652 frames**. All 610 tuples of path/category/
  matched/total frames equal the preceding checkpoint. These final root
  executions are fresh, unlike the explicitly labelled exact-tree worker
  replay aliases in intermediate `46b`/`9a` baselines.
- Focused checks cover raw constructor state/flags/matrix/order, strict
  whole particles and ordered PC/LR sites, genuine incoming raw LCG chains
  plus port ledger-cadence seeds, actual API allocation windows and clones,
  old Sword/monster/heavy/item controls and three human matches. Clone
  construction is outside measured allocation windows; local callback
  refusals do not promise whole-tick rollback or global port caller tracing.

## Inventory

`target/parity/wave22-integrated/` counts 19,827 retail functions:
**4,211 ported, 225 partial, 1,372 body, 1,381 row, 231 referenced, 12,407 not
started**. By retail bytes: **29.4% ported/partial; 38.1% with body/row**.
Historical in-game subtotal: **45.2% / 57.2%**. Ported plus partial grows by
12 functions. Net 13 additional explicit guards produce **299 fail-closed
sites** (280 unimplemented, 19 bail), exposing unsupported contexts. These
are citation/code-byte classifications, not witnessed branch coverage or a
percentage of complete gameplay; every reporting group remains in scope.

## Next bounded work

Five original complete held Sword Down Air entry captures are available.
The proposed static common-Down-animation capability must keep both normal
Link/YoungLink entry and their direct bounce restart closed; existing
midair catches and actual-row/private checks must remain intact. Read-only
plans and independent closure review are under `target/wave23/down-air-prep/`
and `target/wave22/{down-air-search,review}/`. No candidate is accepted by
those plans. No new recording or protected-data change is authorized. Ness
yo-yo overlap still lacks whole-capture particles; second-live-log data ends
at creation without coexistence/lifecycle proof. Broader stages/items/CPU/
modes/menus remain in scope; replay depth follows breadth.

## Quiet performance

At the integrated source: **7,675,632 stripped bytes; 6,897,664 text bytes;
55.485 ms load; 18.417 ms per 600 ticks**. Timing and shared duplication pass
(20 melee-ft duplicate labels, 94 across crates). Stripped size grows by
1,248 bytes versus wave 21; text size is unchanged. The unchanged stripped/
text/C15 ceilings and Kirby duplicate-label baseline zero still fail. No
budget was raised. Full generated report is appended to `docs/PERF.md`;
evidence `target/perf/20261025T120423Z-76805`.
