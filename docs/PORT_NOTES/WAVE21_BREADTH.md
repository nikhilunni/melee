# Wave 21 integrated checkpoint

Production source `6d0c850e8d495a1d50ce0a334b480bd18ee806ec`, tree
`fbe1bdf5dca0eedb9d7b18da01559cf0aaccb76f`. Fresh coordinator proof is
`target/wave21/main/`; its normal CLI was built after tests/clippy, frozen
with the original 3,490-scenario manifest, and used for the full sweep and
both replay corpora. Canonical fresh baseline:
`target/sweeps/6d0c850e8d495a1d50ce0a334b480bd18ee806ec.*`.

## Accepted changes and limits

**Held Sword common back-air entry** (`b029dc45`, worker `1d85d71e`): seven
original complete captures (7,893 rows per stream) now match through actual
common action 67 entry and remaining same-tick phases. Every capture ends
on that entry; later actual Animation/IASA and landing remain guarded.
Immutable `held_on_entry` ownership distinguishes a new held entry from
catching a Sword during an existing aerial. The latter already has a whole
1,344-tick witness and stays exact. Reentering with the held Sword installs
fresh ownership even when the old catch lock persists. Private character
rows do not inherit the common capability. No first-tick scheduler exception,
new hand/cache math or generalized aerial/throw admission is used.

Raw checks preserve actual commands, identity, owner, attachment part and
flags within their consuming semantic domains. Inactive stale Fall interrupt
storage and Fox Blaster attachment metadata are explicit controls, not Sword
assertions. Full evidence: `HELD_SWORD_BACK_AIR_ENTRY.md` and
`target/wave21/held-bair-entry/`.

**Crate-monster births** (`6d0c850e`, worker `a4c36511`): four original
complete captures (3,051 rows per stream) prove three Kuriboh births and one
left-facing ReDead birth from opened crates. Constructor order preserves
both distinct live facing queries, source RNG, initial collision and spin.
Typed ReDead attributes read its release timer as s16 120, not float bits;
shared monster setup clears the actual fighter/open-palm push flags.
Normal scheduler registrations and remaining same-tick item callbacks run;
next actual lifecycle callbacks and unsupported states remain explicit
refusals. No synthetic newborn flag or phase skip is used.

Both models are preloaded through the existing generic item pose route.
Actual API capture proves a new visible populated mesh group at birth and
identical matrices in independent live clones; step/observe/capture windows
allocate nothing. Clone construction is outside those windows. This is not
a claim of retail bone or pixel equality. Full evidence and addresses:
`WAVE21_CRATE_MONSTERS.md`; final amended-source worker artifacts are only
`target/wave21/crate-monsters/visual/`, not its earlier headless prototype.

## Combined validation

- Release workspace: **1,992 passed, zero failed, one existing ignore**,
  across 358 reports; m4 345, m5 310. All-target workspace clippy clean.
- All nine worktree-only missing-data failures pass in MAIN without optout;
  `worktree-data-limit-resolution.json` enumerates them.
- Harness **407 passed**; schema **zero layout violations**. WASM pure suites,
  four expected spline traps and **6,003 identical native/WASM hash lines**
  pass; release web check passes. No debug run or missing-data optout.
- Fresh full sweep: **3,490 scenarios; 3,332 particle-exact, 115 not
  comparable, 42 existing local failures and one existing particle fault**.
  **Zero regressions and eleven fixes** versus `f3c7f2f5`; zero regressions
  and **265 fixes** versus startup `0a99db31`. Every nonfix TSV row remains
  identical to the preceding checkpoint. Incomplete recordings and existing
  failures are not accepted divergences.
- Replay A: **108/108 complete, 964,782 matched frames**. Replay B:
  **486/502 complete, 5,102,652 matched frames**. All 610 tuples of path,
  category, matched frames and total frames are identical to the preceding
  checkpoint, checked mechanically after fresh coordinator executions.
- Focused checks cover raw constructor/entry state, real phase ordering,
  negative continuation callbacks, public API allocation windows and clones,
  old Sword catches, random items, HeavyGet, item contacts and three human
  matches. Genuine raw LCG audits, particle PC/LR order and port ledger-cadence
  seeds remain separate proofs; no global port caller tracing is claimed.

## Inventory

`target/parity/wave21-integrated/` counts 19,827 retail functions:
**4,206 ported, 218 partial, 1,374 body, 1,380 row, 232 referenced,
12,417 not started**. By retail bytes: **29.3% ported/partial; 38.1% with
body/row**. Historical in-game subtotal: **45.0% / 57.1%**. Ported plus
partial grows by eleven functions. Thirteen additional explicit guards
produce **286 fail-closed sites** (267 unimplemented, 19 bail), exposing
unsupported contexts. This inventory counts citations and code bytes, not
witnessed branches or a percentage of complete gameplay.

## Next bounded work

Common Sword F66/Hi68 entries have thirteen existing complete captures;
Down Air stays out because of character-specific callback replacement.
Klaptrap needs the real coupled birth-tick Ground path and item callbacks,
including stage-bank DPtcl emission, not constructor-only activation.
Read-only source/assets/stream plans and independent review are under
`target/wave21/{f-hi-entry-search,klaptrap-search}/` and
`target/wave22/{review,klaptrap-prep}/`. No candidate is accepted by those
plans. Later callbacks remain guarded. No new recordings or protected-data
changes are authorized; Ness yo-yo overlap still lacks complete particles.

## Quiet performance

At the integrated source: **7,674,384 stripped bytes; 6,897,664 text bytes;
56.502 ms load; 18.139 ms per 600 ticks**. Timing and shared duplication pass
(20 melee-ft duplicate labels, 94 across crates). Stripped size grows by
34,096 bytes and text by 16,384 bytes versus wave 20. The unchanged
stripped/text/C15 ceilings and Kirby duplicate-label baseline zero still
fail; no budget was raised. Full generated report is appended to
`docs/PERF.md`; evidence `target/perf/20261025T104941Z-51007`.
