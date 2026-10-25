# Wave 16 integrated breadth

2026-10-24. Moving Kongo barrel clocks (`cefbc068`) and neutral Party Ball
FALL/REST (`93683489`) are integrated. Production/test source is
`936834898000ba6b164d18fe3d5e5a5d9ca737f6`; neither change writes game data.
Independent source and retail-reference reviews found no demonstrated defects.

## Accepted behavior and evidence limits

- **Kongo moving barrel:** warm import restores the live typed AObj/FObj
  state while retaining archive joints and path references. The initial
  animation rate 1.0 advances in link 1 before link 4 installs the controller
  rate 0.5. Cold initialization is unchanged. Three complete captures cover
  1,666/2,726/3,580 ticks of catches, held positions and launches. Each cold
  twin shares its warm recording through `expected`; these are three
  captures and six initialization scenarios. Actual hidden cooldown,
  position, action and damage comparisons cover all rows of the first
  capture; public step/capture checks observe 18 held ticks and three
  launches without allocations. Log-standing, revival-floor, second-live-log,
  Klaptrap and other unsupported branches retain their guards.
- **Party Ball:** one complete 600-tick capture proves neutral left-facing,
  unowned/unheld zero-velocity spawn, FALL, flat-floor bounce and REST through
  tick 599. Warm and cold scenarios share that recording. Scoped Resume
  preserves the bounce effect's pre-pitch transform before the REST pitch
  write; the actual tick-495 spark y bits are `0x3f2cd360`. Tests check
  effect/Resume order, early origin flags, exact spread prefix, continued
  lifetime and REST easing. Positive facing, other spawn contexts, opening,
  carry, damage, other contacts and unwitnessed states remain guarded.
  See `PARTY_BALL_FALL_REST.md`.

Strict gates compare full recorded fighter/item fields and particles, ordered
particle RNG sites and per-tick port seeds under each ledger's display clock.
Separate raw ledger audits check every recorded LCG transition. This is not
global instrumentation of every nonparticle simulator RNG caller.

## Combined checkpoint

Root evidence: `target/wave16/main/`; candidate evidence:
`target/wave16/kongo-barrel/` and `target/wave16/partyball/`.

- Combined full sweep: 3,488 scenarios, **3,307 particle-exact**, 115 not
  comparable, 65 unchanged local failures and one unchanged particle fault.
  **Zero regressions, 14 fixes** against `486aaba1`, and 242 fixes against
  startup `0a99db31`. Three new cold scenarios are included in this sweep.
- Both replay corpora preserve every path's category and matched count:
  **108/108** complete, 964,782 matched frames; **486/502** complete,
  5,102,652 matched frames.
- Full release workspace: **1,947 passed, zero failures, one existing
  ignore** across 356 binaries. Workspace all-target clippy is clean.
  Harness: 407 passed; schema: zero layout violations. Root focused gates
  include both new strict groups, raw regressions, three complete human
  matches, the 37 prior item cases and actual API allocation/capture.
- WASM pure suites and four expected spline traps pass; native/WASM match
  over 6,003 hash lines. Release web check passes.
- Fresh inventory: 4,208 ported, 201 partial, 1,362 body, 1,381 row,
  229 referenced, 12,446 not started. All-retail bytes: **29.3% / 38.0%**;
  historical in-game group: **44.9% / 57.0%**. There are **269 explicit
  fail-closed sites** (250 unimplemented, 19 bail). The 17-site net increase
  reflects explicit guards for Party Ball's unsupported branches while the
  admitted path becomes exact; this is not 17 newly introduced divergences.
  Inventory resolution is 211 message/C-line, 13 enclosing, 45 unresolved.
  Citation candidates: 24,116/24,970 resolved (96.6%). These are citation/byte
  proxies, not a percentage of directly witnessed gameplay.
- Quiet repository performance measurement at `93683489`: load 55.560 ms,
  600 ticks 18.775 ms; timing and shared duplicate limits pass (20 melee-ft labels,
  94 across crates). Stripped size 7,639,936/text size 6,881,280 bytes exceed unchanged
  size ceilings; Kirby's one duplicate still exceeds baseline zero. The gate exits 1
  for these existing budget categories. No threshold or baseline increased.
  `docs/PERF.md` retains the complete generated report.

## Next bounded work

The next plans use existing recordings only:
`target/wave16/kongo-log-search/plan.md` covers the missing `Log::stood_on`
wiring and a complete 2,829-tick recording with 32 observed grounded-contact
rows. `target/wave16/revival-floor-search/plan.md` covers the revival-floor
callback's actual final-tick Wait entry, resolved position, protection and
color; its 1,679-tick capture has no later post-contact ticks. Existing
independent timeout captures cover already-ported protection expiry.

`target/wave16/cpu-next-search/plan.md` audits 98 explicit CPU scenarios:
95 accepted witnesses, an intentionally rejected unfinished warm save and
two incomplete match recordings. All explicit CPU scenarios are FD or BF,
with no CPU+items recording. Human/Nana fields do not prove configured CPU
control. No additional CPU guard has an eligible complete witness in that
catalog. A concrete new-recording pack is prepared in scratch; permission
remains pending, and no recordings or budgets were changed.
