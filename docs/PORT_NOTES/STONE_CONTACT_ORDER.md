# Ordinary fighter contacts replace pending phantoms before shell handling

`ftColl_80076ED8` clears `dmg_log1_idx` at the beginning of its ordinary
contact branch (retail 0x800771FC..0x80077200), before victim invincibility
and Stone-shell absorption can return without logging damage. The Rust
collision path used to clear it only inside `CombatState::log_hit`, so an
earlier phantom contact survived a later ordinary contact whose damage the
shell absorbed. This could start hidden hitlag, delaying movement and IASA.

The shared fighter-contact path now clears only its pending `phantom_log`
at that boundary. Hitbox ordinary/phantom victim histories, stored phantom
identity, active lockout, damage bookkeeping and all Stone timers, arithmetic
and geometry remain intact. The analogous item-contact path is outside this
change's witnessed scope.

Retail bytes establish actual same-tick mixed contacts in two complete
captures. `kirby_stone_phantom_fd_fox4` has 360 ticks: Fox hitID0 phantoms at 276,
then hitID1 hits the shell normally; Kirby continues the nudge at 277.
`corpus_v3_ys_kirby_fox4_e45211ed5_p0` has 699 ticks: the mixed contact occurs
at 513 and Kirby's timer reaches0 at 641, exiting Stone at 642. The historical
scenario headers are not descriptions of the final retail contact response.
Both hitbox victim lists retain Kirby afterward even though the global
pending phantom log is empty.

Tests replay every original raw-clock row to compare hidden hitlag, lockout,
phantom knockback, Stone timer/shell and positions, then isolate the actual
contact pose to test phantom-only, shell-held ordinary and invincible
ordinary contacts. A genuine phantom-only control and an existing 1505-tick
Stone capture preserve active lockout and delayed shell damage after a later
ordinary hit. Strict oracle checks use full fields/items/particles, ordered
particle call sites, ledger-cadence seeds and a separate raw LCG-edge audit
from genuine initial seed observations, including directed captures' row-zero
setup draw. Public API allocation/clone/capture tests continue both complete
scenarios under public live-step display behavior; clone construction itself
is unmeasured. Raw cadence is verified separately by the raw replay tests.

Validation logs and final provenance are retained under
`target/wave19/stone-contact/` in the coordinator checkout. No new recordings,
truncated scenarios, raw expectation changes or newly admitted item branches
are part of this change.

Final release proof on docs parent `74a18a5e`: all four strict captures and
raw-state regressions pass; API step/capture checks pass; touched all-target
clippy is clean. The one 3,490-scenario sweep has zero regressions and two
fixes (3,316 particle-exact). Both replay corpora retain every path's category,
matched frames and length: 108/108 completed, 964,782 matched; 486/502
completed, 5,102,652 matched. Nine library tests cannot access their hardcoded
worktree-local data paths; the other 195 and all 38 melee-ft tests pass.

## Coordinator checkpoint (2026-10-25)

Integrated production `72aa5d9a` independently passes the full release
workspace: 1,974 tests, zero failures, one existing ignore (m4 345; m5 305).
All-target clippy, 407 harness tests, schema, WASM pure suites/four expected
spline guards/6,003 native-WASM hash lines and release web checks pass.
This full MAIN run covers the worker-local data-path limitations above;
no missing-data opt-out or data copy was used.

The fresh coordinator 3,490-scenario sweep has 3,316 particle-exact cases,
115 not comparable, 58 existing failures and one existing particle fault;
zero regressions and two fixes versus `2c10dd58`, 249 fixes versus startup.
Both replay output files are byte-identical to their prior baseline:
108/108 and 486/502 complete, 964,782 and 5,102,652 matched frames. Evidence
and executable/source/scenario provenance are `target/wave19/main/`;
canonical artifacts are `target/sweeps/72aa5d9a43d9c64d16e1f66b7a7e1e45f65cc75f.*`.

Fresh inventory remains 4,209 ported/201 partial/1,363 body/1,381 row/
229 referenced/12,444 not started, with 268 explicit guards. Retail-byte
coverage is 29.3% conservative/38.0% including bodies and rows; historical
in-game subtotal is 45.0%/57.0%. These are citation proxies, with no scope
exclusions. Quiet performance is 7,640,128 stripped/6,881,280 text bytes,
55.080 ms load and 18.351 ms per 600 ticks. Timing/shared duplication pass;
existing size/text/C15 and Kirby duplicate-label budgets still fail. The
complete report is appended to `docs/PERF.md`; ceilings remain unchanged.
