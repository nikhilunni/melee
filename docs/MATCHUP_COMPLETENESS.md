# Fox–Marth on Final Destination: interaction coverage

Approved next milestone, 2026-09-11. The recorded steel thread established
exactness for particular matches. It did not establish completeness for all
legal inputs in this matchup. Keep breadth paused while closing that gap.

## Status — 2026-09-28: exit criteria

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

## Status — 2026-09-26

Resumed. Corpus v2 starts from retail boundaries, and any case replays in
Dolphin exactly (`harness/replay_to_scenario.py`). DamageFlyRoll landed; phantom
hits and simultaneous hit logs are implemented; seven generated full matches
match retail start to GAME after twelve shared fixes
(`PORT_NOTES/CORPUS_BRIDGE_FIXES.md`). Open: captured damage outside
low-capture/throw states, Reflector ground-to-air, timer/Sudden Death
bridging the rest of the corpus. The screen KO, the timeout and Sudden
Death (with the live transition and its Bob-omb rain) landed 2026-09-26.

## Coverage audit — 2026-09-26

`COVERAGE_AUDIT.md` classifies every explicit boundary and the silent gaps.
The remaining reachable work is Sudden Death's item interactions (pickup,
hits on Bob-ombs, chained explosions, holding and throwing), grab-pair hits,
and a few contact combinations; everything else is out of scope or under
investigation there.

## Paused handoff — 2026-09-12

Implementation is paused after `415a230` at the user's request. Recent committed
packets are wall/ceiling recovery (`987fe7f`), CaptureJump/airborne release
(`fb53f39`) and revival-laser contact (`415a230`). Both full workspace profiles
pass 1,443 tests, with zero failures and one existing ignore; Clippy, formatting,
225 harness tests and the native build pass. Existing size/time performance
limits remain red; the latest packet does not increase binary size.

The fixed version 1 corpus has 48 cases: 20 match finishes, 17 full 6,000-tick
runs and 11 faults, compared with 46 faults initially. Remaining reproduced
fault families are DamageFlyRoll, captured-pull damage, fighter/item phantom
contacts, ScreenKO camera and Reflector ground departure. Passing generated
runs establish robustness, not retail exactness or complete interaction coverage.

DamageFlyRoll is the next prepared packet: three Fox retail recordings cover
flight, floor collision and hitstun expiry. Its source draft is unapplied and
its regression tests are not registered. Marth-specific roll coverage remains
open. Timer/Sudden Death fixtures and diagnostics are captured, but their
implementation is still pending. The table below retains other unresolved
branches beyond those currently reproduced by the corpus.

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

## Work order

1. **Make UI failures reproducible.** Record match configuration, seed, asset
   identity and the controller inputs actually consumed at each simulation
   tick, including the attempted failing tick. Preserve the panic diagnostic
   and tick at the application boundary, stop stepping the failed match, and
   export a replay that runs through `melee-lib` without rendering. Use bounded
   preallocated recording storage with an explicit capacity policy; do not
   add allocations to the simulation tick. A rolling tail alone is insufficient
   without a restorable checkpoint. Replaying in the port reproduces a fault;
   Dolphin recording from an equivalent boundary establishes correctness.
2. **Audit reachable behavior.** Start from installed common/special rows,
   input transitions, collision responses, item callbacks, script commands,
   match flow and presentation requests. Search explicit panics plus unsupported
   errors, placeholder no-ops and fallback behavior. Classify each gap as
   reachable, demonstrably out of scope, or needing investigation. A source
   search is a seed list, not a completeness proof. Track branch/transition
   coverage, not merely whether a motion row ran once.
3. **Close frequent failures in small oracle-backed packets.** Start with
   laser/shield response, then special-move transitions, combat contact edge
   cases, and match endings. Record focused retail scenarios before porting;
   audit assembly and preserve operation ordering. Give each packet named
   `cargo gate` acceptance tests and clippy before implementation begins.
4. **Exercise combinations.** Combine directed scripts with deterministic,
   state-aware input exploration: hold/release durations, analog thresholds,
   airborne/grounded changes, edge positions, hitlag/hitstun, shielding,
   grabs, percent ranges and simultaneous contacts. Retain seeds and reduce
   failures to short reproducers. Legal cold-start trajectories matter;
   fabricated invalid state combinations do not prove reachable bugs.
5. **Promote a regression corpus.** Every discovered failure becomes a
   focused regression and, where practical, a retail differential scenario.
   Keep long human matches for integration and soak coverage. Crash-free
   generated runs establish robustness only; they are not bit-exact oracles.

## Confirmed source gaps to triage first

This is a partial source audit, not an exhaustive reachability classification.

| Area | Evidence | Next packet |
| --- | --- | --- |
| Laser contacts | Ordinary shield, lightshield, airborne hit, grazing deflection, fresh/delayed/return/stale reflection and powershield are implemented; four reflection recordings and a 600-tick revival-invincibility contact pass both full profiles; [revival notes](PORT_NOTES/REVIVAL_LASER.md) | Item phantom contact and remaining shield depletion combinations; [reflection notes](PORT_NOTES/LASER_REFLECTION.md) |
| Marth Counter | Aerial entry, hit/miss, landing and offstage completion implemented and gated; counterpart transitions source-audited | Deliberate support loss during both phases; projectile contacts |
| Fox Reflector | Turn, button/tap aerial jump cancel and preserved turn landing implemented; seven directed gates pass | Remaining phase preservation and edge departures; `PORT_NOTES/REFLECTOR_INPUT.md` |
| Fox recovery | Five counterpart/rebound witnesses pass both full workspace gates. Two additional charge/travel contact recordings pass state, RNG, particle, bone, allocation and both full workspace gates ([contact notes](PORT_NOTES/FIRE_CONTACT.md)). See [recovery notes](PORT_NOTES/RECOVERY_COLLISIONS.md). | Remaining travel landing, charge/ending departure and wall/ledge combinations |
| Wall and ceiling recovery | Fox WallJump, both StopCeil trajectories and a Marth wall-contact control pass full raw/SRT/particle/allocation checks and both profiles | Repeated/mirrored wall jumps and ceiling-triggered ledge exits; [recovery notes](PORT_NOTES/WALL_CEILING.md) |
| Shared contacts | Mutual clank, both priority winners, no-rebound and airborne controls pass directed raw/SRT/particle/allocation checks and both full profiles | Phantom, invincible and simultaneous contact reachability; [contact notes](PORT_NOTES/CONTACT_CLOSURE.md) |
| Shield exits | Standing/dash/run grabs, C-stick jumps, delayed powershield and reflection implemented; reflector overflow now passes the full 600-tick fall/down/stand recovery trajectory with both full profiles passing | Remaining depletion combinations |
| Ledge variants | C-stick options and hang timeout pass ten directed fighter/RNG/particle, raw, SRT and allocation gates; slow rows are source-ported | Recorded slow-option witnesses, occupied ledges, further priorities and wall/ceiling interactions |
| Match endings | Revival lifecycle, the screen KO, the timeout (TIME!, standings, scene exit), the live transition into Sudden Death and its Bob-omb rain through the bomb KO are gated, cold and from savestates; [notes](PORT_NOTES/SUDDEN_DEATH.md) | Bob-omb interactions beyond the idle rain (pickup, throws, hits on bombs); simultaneous-KO draws |
| Diagonal smash | Authored fallback and stick priority implemented; four directed Fox/Marth gates pass | Full regression/commit status in TRACKER.md |
| DamageFlyRoll | Landed (`PORT_NOTES/DAMAGE_FLY_ROLL.md`): three Fox witnesses pass | Marth witness; DamageFly wall/ceiling tech chain |
| Hitstun exit | Attack entry, tumble/ordinary input priority and air-dodge knockback decay implemented; seven directed gates pass | Full regression/commit status in TRACKER.md; `PORT_NOTES/POST_HITSTUN_INPUT.md` |
| Airborne grab victim | Sustained high hold, air Cut, air Jump and both captor/input variants of CaptureJump pass six full recordings and both workspace profiles | Forced separation, airborne captor release and article-hit combinations; [capture notes](PORT_NOTES/CAPTURE_RELEASE.md) |
| Jump-squat up-B | Explicit grounded Up-slot dispatch implemented; six directed Fox/Marth gates cover A/Z and diagonal priority | Full regression/commit status in TRACKER.md; `PORT_NOTES/JUMP_SQUAT_UP_SPECIAL.md` |
| Dash defense | Dash and Run defense implemented with distinct item-throw/grab windows | `dash_escape_fd_fox`, `dash_shield_fd_marth` |
| Revival platform | Timeout and digital/analog shield+A pass five recorded schedules; no-blend entry and retained dynamic state corrected | Further input thresholds and lifecycle combinations; [notes](PORT_NOTES/REVIVAL_CAPTURE_LEDGE.md) |
| C-stick throws | Direction selection, Fox blaster/laser callbacks, captured damage and collateral hitbox lifetime implemented | Fourteen exact item/fighter/particle scenarios plus raw scratch; both workspace profiles and allocation gates pass; `PORT_NOTES/CSTICK_THROWS.md` |
| Taunt | Idle, Dash and Run entry implemented; both characters use their authored motion239 | `taunt_fd_fox`, `taunt_fd_marth` |

The added `fighter/` references are under `crates/melee-ft/src`; line numbers
are audit-time pointers and will move as packets land. Outstanding reachability
investigations: captured fighters leaving support, airborne grab release,
linked fighter death, additional shield-break orientations, FD underside
contacts, and Fox-article hits during capture. Final-stock animation timeout is
not yet a confirmed gap: the scene freezes gameplay after elimination. Likewise,
special-fall remaining-jump rejection may be unreachable because entry spends
the remaining jumps. Do not classify these solely from panic text.

Exclude CPU paths, other-character hooks, ice/cape/armored-jump responses,
ordinary random-item spawners and other stage controllers. Sudden Death Bob-ombs
are an exception: its scene spawns them even with random items disabled. Platform-drop predicates require
pass-through platforms absent on FD. Compare generic item-command rejection
against authored Fox article commands before deciding it is out of scope.

The first packet preserves native panic payloads, retains consumed controller
samples and automatically exports the first fault. Cmd-S exports the current
match, and `melee-replay` reproduces it headlessly. See
`PORT_NOTES/MATCHUP_REPLAY_AND_SHIELD.md` for the format, limits and verification.

## Exit criteria before breadth

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
