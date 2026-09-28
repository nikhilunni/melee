# Fox–Marth on Final Destination: completeness

Complete (2026-09-28). The recorded human matches established exactness for
particular games; this milestone established coverage of the matchup's legal
inputs. The scope, exit criteria and evidence below are the template for the
next matchup or stage: copy this file, change the scope, and fill the table.

## Scope

Two human-controlled players, Fox and Marth, Final Destination, four stocks,
eight minutes, ordinary versus rules, random item spawning off. Character
articles (lasers and blasters) and Sudden Death Bob-ombs remain in scope. Include both port assignments,
both facings, full analog controller inputs, and varied seeds. The current
keyboard adapter is only a subset of this input space. Inventory timeout and
tie resolution explicitly; do not silently redefine a complete match as only
one ending through stock exhaustion. CPU, other stages/characters, random
items, and exact rendered pixels are separate milestones.

`MatchRules::time_limit_seconds` sets the timer (480 for eight minutes). A
timed-out stock tie reports `MatchOutcome::SuddenDeath`, and
`Match::sudden_death` continues into the retail Sudden Death match.

## Exit criteria (the template for the next matchup or stage)

- Every audited in-scope gap is implemented and tested; unknown reachability
  is resolved rather than labeled out of scope without evidence.
- A reviewed interaction matrix links reachable transitions/branches to
  scenarios, with missing coverage visible. Existing motion-row coverage is
  not sufficient.
- Focused Dolphin traces compare exact state, RNG and relevant item/particle
  fields. Extend observation where existing gated fields cannot distinguish
  the new behavior; do not weaken expected values.
- Debug/release workspace gates, workspace all-target clippy, zero-allocation
  and capture-nonmutation checks pass. Performance regressions are measured
  using the existing serial perf gate.
- A fixed, versioned generated corpus and additional human full matches pass,
  including swapped ports. Record corpus seeds, tick budget and results so
  subsequent changes rerun the same acceptance workload.

Finite testing cannot prove every possible input sequence. The defensible
claim is audited in-scope implementation coverage plus a reproducible exact
and robustness corpus, with any residual limitations stated explicitly.

After this milestone, expand one axis at a time: the same matchup on another
stage, then an additional character on FD. Reuse the coverage matrix and
reproducer pipeline to expose each expansion's new shared-engine requirements.

## Result (2026-09-28): every exit criterion met

| Exit criterion | Evidence | Open |
| --- | --- | --- |
| Every audited in-scope gap implemented and tested; reachability resolved | `COVERAGE_AUDIT.md` and `INTERACTION_MATRIX.md`: every boundary is ported and witnessed, fails closed as out of scope (Start pause, other stages and characters), or is shown not reachable with decomp, geometry or `melee-sim search` evidence (Fox's other shield-break orientation, a shield break off the stage, laser plus melee on one shield, ground Dancing Blade leaving FD, repeated wall jumps, the DownDamage wall bounce, Fox holding a Bob-omb on the ledge, Fire Fox AirHi -> Hi, the scratch-word guards). Ported from the shared shield-impact code but unwitnessed for Fox's shield: two impacts in one frame (Marth's shield takes a laser plus a blast in `sudden_death_shieldlaserbomb_fd_marth`) | none |
| A reviewed interaction matrix | `INTERACTION_MATRIX.md` (regenerate with `harness/interaction_matrix.py`): no MISSING or unverified cells remain (52 -> 0 on 2026-09-27/28) | none |
| Focused Dolphin traces compare state, RNG and item/particle fields | `m5_gate`: 258 tests, including `MATRIX_WITNESSES` (61 scenarios) and `CORPUS_V3_MATCHES` (48); bones and particle gates; every witness recorded this milestone is exact | none |
| Debug/release gates, clippy, zero allocation, capture non-mutation | Full debug and release gates 1526/0 and 1527/0 (2026-09-27, final); clippy clean; the alloc and capture gates are part of `cargo gate` | none: the perf gate passes (2026-09-28) with the size baseline raised by the user (`docs/PERF.md`) |
| Fixed, versioned corpus and full matches, both port layouts | Below | none |

**Corpus workload.** The explorer (`cargo run -p melee-replay --release --example explore -- harness/roms/files <out> <count> <skip> [sudden-death]`, corpus version 3) is deterministic per seed. Each normal case is a full match from a retail match-start boundary, both port layouts, three input profiles.

| Batch | Cases | Result |
| --- | --- | --- |
| Normal, skip 2000, 300 seeds | 1,800 | FlyReflectCeil (fixed, `corpus_v3_s0_ef4efb740_p1`) |
| Normal, skip 3000, 300 seeds | 1,800 | Reflector turnFrames through a smash (fixed, `corpus_v3_s0_efaccaf3d_p0`) |
| Normal, skip 4000, 300 seeds | 1,800 | grounded special fall (fixed, `corpus_v3_s1_e7d968d2d_p1`) |
| Normal, skip 5000 and 6000, 300 seeds each | 3,600 | clean |
| Sudden Death, skips 0..5000 | 7,300 | faults fixed and gated as `corpus_sd_*` |
| Sudden Death, skip 7000 and 8000, 1,000 seeds each | 6,000 | FallAerial and Ottotto while holding (fixed) |
| Normal, skip 7000, 300 seeds | 1,800 | a catch cut on its CatchWait entry tick left CaptureFlash sealed (fixed, `corpus_v3_s1_e19a3b12e_p2`) |
| Sudden Death, skip 9000, 1,000 seeds | 3,000 | jump-squat up smash with a Bob-omb; then a rain bomb's creation intangibility (both fixed, `corpus_sd_s1_e00088bc5_p0`) |
| Normal, skip 8000, 300 seeds; Sudden Death, skip 10000, 1,000 seeds | 4,800 | clean; bridged samples `corpus_v3_s0_e01a74e09_p0` (6,001 ticks) and `corpus_sd_s1_e2726590c_p0` (1,683) exact |

Exactness sample: 34 clean explorer cases (20 Sudden Death, 14 full matches) bridged to Dolphin were all exact; the full matches `corpus_v3_s0_e035918d1_p1` and `corpus_v3_s1_e42b75250_p1` (one per port layout) are gated with the fault witnesses. Human full matches: `match_fd_foxmarth` (6,083 ticks) and `match2_fd_foxmarth` (10,059 ticks).
