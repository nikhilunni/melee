# Fox–Marth on Final Destination: interaction coverage

Approved next milestone, 2026-09-11. The recorded steel thread established
exactness for particular matches. It did not establish completeness for all
legal inputs in this matchup. Keep breadth paused while closing that gap.

## Scope

Two human-controlled players, Fox and Marth, Final Destination, four stocks,
eight minutes, ordinary versus rules, random item spawning off. Character
articles (lasers and blasters) remain in scope. Include both port assignments,
both facings, full analog controller inputs, and varied seeds. The current
keyboard adapter is only a subset of this input space. Inventory timeout and
tie resolution explicitly; do not silently redefine a complete match as only
one ending through stock exhaustion. CPU, other stages/characters, random
items, and exact rendered pixels are separate milestones.

Current API limitation: `MatchRules` explicitly supports no timer. Adding the
eight-minute rule and its terminal behavior is part of this milestone, not a
configuration already available to the UI.

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
| Laser contacts | Ordinary shield, lightshield, airborne hit and grazing deflection implemented in the first packet; projectile reflection and item phantom contact remain unsupported | Powershield, shield depletion and phantom contact |
| Marth Counter | Aerial entry, hit/miss, landing and offstage completion implemented and gated; counterpart transitions source-audited | Deliberate support loss during both phases; projectile contacts |
| Fox Reflector | Turn, button/tap aerial jump cancel and preserved turn landing implemented; seven directed gates pass | Remaining phase preservation and edge departures; `PORT_NOTES/REFLECTOR_INPUT.md` |
| Fox recovery | `ft-fox-family/src/special_hi.rs`, `special_s.rs`: preserved air/ground and recovery transitions panic | Charge/launch landing, floor-directed launch, rebound and lost support |
| Shared contacts | `melee-ft/src/fighter/damage.rs`: phantom, invincible and simultaneous contact branches panic | Retail reachability and contact-order fixtures for this matchup |
| Shield exits | `melee-ft/src/fighter/shield.rs`: grab out of shield, delayed powershield and projectile reflection panic | Standing/dash grab exits, shield timing and laser reflection |
| Ledge variants | `melee-ft/src/fighter/ledge.rs`: slow options, C-stick options and hang timeout panic | Percent-dependent options, full controller inputs and prolonged hanging |
| Match endings | Tracker retains a screen-KO follow-up | Audit all KO variants, respawn, final stock, timeout and ties |
| Diagonal smash | Authored fallback and stick priority implemented; four directed Fox/Marth gates pass | Full regression/commit status in TRACKER.md |
| Hitstun exit | Attack entry, tumble/ordinary input priority and air-dodge knockback decay implemented; seven directed gates pass | Full regression/commit status in TRACKER.md; `PORT_NOTES/POST_HITSTUN_INPUT.md` |
| Airborne grab victim | `fighter/grab.rs:194`: CapturePulledHi panics | `grab_airborne_fd_foxmarth`, `grab_airborne_fd_marthfox` |
| Jump-squat up-B | Explicit grounded Up-slot dispatch implemented; six directed Fox/Marth gates cover A/Z and diagonal priority | Full regression/commit status in TRACKER.md; `PORT_NOTES/JUMP_SQUAT_UP_SPECIAL.md` |
| Dash defense | `fighter/dash.rs`: early Escape and later Shield rejected | `dash_escape_fd_fox`, `dash_shield_fd_marth` |
| Revival platform | `fighter/life.rs`: timeout and an overbroad shield+A predicate panic | `rebirth_timeout_fd_fox`, `rebirth_timeout_fd_marth`, `rebirth_shield_a_fd_fox` |
| C-stick throws | `fighter/grab_throw.rs:96`: C-stick throw selection panics | `cstick_throws_fd_foxmarth`, `cstick_throws_fd_marthfox` |
| Taunt | `fighter/walk.rs`, `input/iasa.rs`, `fighter/dash.rs`: selected input lacks transition body | `taunt_fd_fox`, `taunt_fd_marth` |

The added `fighter/` references are under `crates/melee-ft/src`; line numbers
are audit-time pointers and will move as packets land. Outstanding reachability
investigations: captured fighters leaving support, airborne grab release,
shield-break flight expiring before landing, linked fighter death, FD underside
contacts, and Fox-article hits during capture. Final-stock animation timeout is
not yet a confirmed gap: the scene freezes gameplay after elimination. Likewise,
special-fall remaining-jump rejection may be unreachable because entry spends
the remaining jumps. Do not classify these solely from panic text.

Exclude CPU paths, other-character hooks, ice/cape/armored-jump responses,
random-item kinds and other stage controllers. Platform-drop predicates require
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
