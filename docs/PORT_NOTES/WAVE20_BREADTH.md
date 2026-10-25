# Wave 20 integrated checkpoint

Production source `f3c7f2f5542d9cefbaf44fc4762970c2685f66ce`, tree
`2717fdd9c8885d662f5193de360a6f1614004b84`; fresh coordinator proof is
`target/wave20/main/`. The final normal CLI was built after tests/clippy and
copied before the full sweep and both replay corpora. The 3,490-scenario
manifest and binary hash are in `oracle-provenance.json` and
`scenario-manifest.json`. Canonical baseline:
`target/sweeps/f3c7f2f5542d9cefbaf44fc4762970c2685f66ce.*`.

## Accepted changes and limits

**HeavyGet terminal entry** (`3985d5e2`, worker `1433c7a5`): five original
complete captures, 1,292 rows each expected/raw/particles/meta/ledger, now
match through entry and the remaining same-tick shared phases. Every capture
ends on its first HeavyGet row. Both hands remain NULL; the container remains
unowned at rest. This proves an attempt/entry, not possession or attachment.
The next actual Animation and IASA callbacks refuse explicitly. No synthetic
first-tick flag, callback skip or scenario-name admission is used.

Retail entry clears throw flags, steps the entry animation directly, then
installs heavy scratch and the common damage callback. The callback lifetime
is represented alongside actual motion callback resets. An existing Marth
ThrowF scratch prerequisite is published at the original transition before
its animation step, preserving its untouched tail; no blanket HeavyGet
scratch reset is introduced. Unit model scale is guarded at import, including
the actual typed Game & Watch model depth. Later heavy carry, throws, damage
ownership and nonunit scaling remain unavailable. Full source evidence and
retail addresses: `WAVE20_HEAVY_ENTRY.md`.

**Mixed item-contact refusal** (`0b380fb0`, worker `ddf9c637`): normal retail
item damage clears the pending phantom log, but invincibility and intact
shell returns can bypass the existing clear. With a pending log those two
unwitnessed branches now refuse before ordinary response mutation. No new
item behavior is admitted, and no literal clear was added on insufficient
evidence. The intact-shell predicate retains retail zero/NaN comparison
semantics. Four complete ordinary item controls (12,523 raw rows) and the
accepted Stone controls remain exact. A bounded 45-scene/88,313-row search
found no qualifying whole mixed-contact witness; that is not exhaustive
negative proof. Details: `target/wave20/item-phantom-guard/validation.md` and
its independent review.

**Description corrections** (`f3c7f2f5`): seven scenario leading comments
were corrected to actual recorded terminal entries/contact timing. Full
parsed TOML before/after was mechanically equal; no inputs, frames, seeds,
setup or retail expectations changed. One allocation test identifier was
clarified to exclude clone creation; assertions are unchanged. The fresh
root checkpoint uses the corrected source and scenario bytes directly.

## Combined validation

- Release workspace: **1,983 passed, zero failed, one existing ignore**,
  across 356 test reports; m4 345, m5 307. All-target workspace clippy clean.
- Harness **407 passed**; schema **zero layout violations**. WASM pure suites,
  four expected spline traps and **6,003 identical native/WASM hash lines**
  pass; release web check passes. No debug run or missing-data optout.
- Fresh full sweep: **3,490 scenarios; 3,321 particle-exact, 115 not
  comparable, 53 existing local failures and one existing particle fault**.
  **Zero regressions and five fixes** versus `72aa5d9a`; zero regressions
  and **254 fixes** versus startup `0a99db31`. The five fixes are the
  HeavyGet captures, not a broader heavy lifecycle. Existing incomplete
  recordings and failures remain outside accepted exact behavior.
- Replay A: **108/108 complete, 964,782 matched frames**. Replay B:
  **486/502 complete, 5,102,652 matched frames**. Every path preserves its
  category and matched count versus the preceding checkpoint.
- Focused checks revalidate raw HeavyGet scratch/flags/ownership and scale
  guards, actual API step/observe/presentation capture without allocation,
  independent before-entry clones, negative continuation callbacks, mixed
  item refusal controls, strict whole HeavyGet/item/Stone captures and three
  human matches. Clone construction is outside measured windows. Genuine
  incoming raw LCG audits and port ledger-cadence end seeds are distinct
  from particle PC/LR instrumentation; no global port caller trace is claimed.

## Inventory and performance

`target/parity/wave20-integrated/` counts 19,827 retail functions:
**4,206 ported, 207 partial, 1,361 body, 1,381 row, 228 referenced,
12,444 not started**. By retail bytes: **29.3% ported/partial; 38.0% with
body/row**. Historical in-game subtotal: **45.0% / 57.0%**. Ported plus
partial grows by three functions. Five additional explicit guards produce
**273 fail-closed sites** (254 unimplemented, 19 bail), making unsupported
contexts visible. This inventory is citation classification, not witnessed
branch coverage or a percentage of complete gameplay.

Quiet unchanged-budget performance at this source: **7,640,288 stripped
bytes; 6,881,280 text bytes; 54.736 ms load; 19.147 ms per 600 ticks**.
Timing and shared duplication pass (20 melee-ft labels, 94 across crates).
Existing stripped/text/C15 size ceilings and Kirby duplicate-label baseline
zero still fail. No budget was raised. Full generated report is appended to
`docs/PERF.md`, evidence `target/perf/20261025T092717Z-27421`.

## Next bounded work

Crate-monster constructors (three Kuriboh births, one left-facing ReDead)
and held Sword common back-air entry have complete existing recordings and
real constructor/entry callback boundaries. Isolated work is underway; no
candidate is accepted yet. Later lifecycle callbacks stay guarded. No new
recordings or protected-data changes are authorized. Ness yo-yo overlap
requires a new complete particle recording and remains blocked on that
separate authorization.
