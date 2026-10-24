# Retail coverage: which retail functions real play executes

The dynamic half of the port's completeness measure. The static inventory
(`target/parity/inventory.tsv`) lists every retail function and whether the port
has it; this measures which of them retail actually runs during the scenarios
the gate replays, and how often. Joined on the retail address, the two give
"unported code that real play runs", ranked by calls.

## Method

**Mechanism: Dolphin's Branch Watch, driven from the tick tracer.** Dolphin's
JITs can record every executed branch, keyed by (origin, destination,
instruction) with a hit count, in a taken and a not-taken collection (the
Qt debugger's Branch Watch tool, `Core/Debugger/BranchWatch.h`). The code is
emitted only in debug mode (`Dolphin.Interface.DebugModeEnabled`) and checks a
recording flag at run time. The scripting fork had no way to reach it, so
`docs/patches/0004-scripting-branch-watch.patch` (branch `retail-coverage` in
`~/Projects/dolphin-scripting/src`, commit 97aabd6, about 85 added lines in
`Scripting/Python/Modules/emulationmodule.cpp` and its stub) adds
`emulation.branch_watch_start/stop/clear/save(path)`.

What was considered first:

- JIT block profiling (`Debug.JitEnableProfiling`) counts block runs, but a
  block runs past conditional exits and, with branch following, into callees,
  so "block ran" over-reports functions; it is also lost when a block is
  invalidated. No Python access either.
- Code breakpoints: the scripting API has memory breakpoints only, and a
  breakpoint per function (~20k) would be a patch anyway and far slower.
- Branch Watch is exact for this question: every function leaves through a
  branch (`blr`, a tail call, a back edge), so a function executed iff a branch
  inside it ran; its entries are the taken branches landing on its first
  instruction (calls with LK, or jumps from outside it); and each conditional
  site's taken / not-taken counts give block-level coverage.

`harness/dolphin/coverage_trace.py` is the tick tracer (`tick_trace.py`) plus
the watch: cleared and started at install (right after the savestate load),
stopped inside the memcheck callback that takes the last tick's record, saved
before `.done`. So coverage spans exactly the recorded ticks, from the same
savestate, inputs, Gecko codes and ports as `record.py`'s tick pass.

`harness/retail_coverage.py` runs it over scenarios in parallel (record_many's
pattern: a child process per scenario with a private Dolphin user folder),
maps each dump onto `symbols.txt` functions, `splits.txt` units and the DOL's
branch sites, and writes the tables. The coverage Dolphin is a separate build
(`tools/build-coverage-dolphin.sh`: `src-coverage/`, `build-coverage/`, 98 MB,
no Qt); the recording build is untouched.

```sh
tools/build-coverage-dolphin.sh
export MELEE_GECKO_DIR=~/melee-data/gecko      # Slippi bridges and UCF scenes need their codes
cd harness && uv run python retail_coverage.py --m5 --run m5 --jobs 8 --resume   # the rest of the m5 list
cd harness && uv run python retail_coverage.py --scope --run scope --jobs 3      # out-of-scope CPU matches
cd harness && uv run python retail_coverage.py --aggregate --run m5             # tables only
```

(From a worktree, add `MELEE_DATA_ROOT=<main checkout>`. `--resume` skips what
a run already has, so the measured run continues where it stopped: the 462
explorer corpus scenes it skipped and the five that failed. Do not replace
the coverage binary in place while a run is going: macOS kills new launches
of a rewritten executable; the build script renames into place.)

## Determinism

Every coverage run's raw tick trace is compared record by record with the
scenario's own recording in `harness/traces` (made by the recording build,
without debug mode or Branch Watch). Only `vi_frame` is excluded (host timing,
AGENTS.md), and keys only one side has (a recording older than the
`pad_queue_*` fields). The raw records hold every fighter's and item's full
struct bytes, the seed, the pads and `ps_frame`, so equality there is stronger
than a `melee-sim gate` of the decoded trace (which is derived from them; a
decoded smoke run differed from the stored expected trace only in the
`pad_queue_*` keys its older recording lacks).

Unloaded, the coverage pass is identical: the first runs (and the alternating
overhead runs below) matched their recordings record for record. Over the
whole set, run while other agents held the machine at load 200-290 on 12
cores, 1,348 of 1,472 runs are identical and 124 differ, almost all at
record 4 in `ps_frame` (the display-pass counter) and then fighter bytes;
109 of the 124 are tick-clock scenes, the rest Slippi bridges (19) and UCF
shield-drop scenes. **The recording build differs the same way under the
same load:** re-recording five of them with the plain headless Dolphin
(private user folder, no debug mode, no coverage) three times each gave
`samus_x_e9943b4ab_p0` 3/3 different at record 4, `corpus_v3_bf_marth_fox4_edb2b114a_p0`
2/3, `samus_missile_air_fd_fox4` 1/3, `bf_reflectorpass_loop_fox` 1/3,
`taunt_start_fd_peach_fox4` 1/3, each at the same record as the coverage run.
So the differences are the harness's sensitivity to host timing under heavy
load (when a display pass falls relative to the injected pads), not the
coverage build; it is worth knowing for anyone re-recording on a loaded
machine. The verdict per run is in `m5.runs.tsv`; coverage of a run that took
another display cadence is still retail code running.

## Cost

- Overhead, same scenario alternating under the same load
  (`corpus_v3_s0_e6cc80d32_p1`, 957 ticks): recording build 106 and 80 ticks/s,
  coverage build without debug mode 114 and 83, coverage pass 75 and 64. Branch
  Watch costs about 1.3-1.4x; unloaded a pass runs ~190 ticks/s.
- Boot and savestate load: ~2 s per scenario; mapping and the determinism
  compare a few seconds more (the compare streams the stored `.zst` trace).
- The measured run: 1,472 scenarios, 2.17 M ticks, 63,200 emulation-seconds
  (34 ticks/s per job at load ~250), about 3 h wall at 6-7 jobs. The full m5
  list is 1,943 scenarios and 3.69 M ticks: ~1.7x that, or roughly 20 minutes
  wall at 8 jobs on an idle machine (3.69 M / (8 x ~190 ticks/s) + boots).
- Space: the per-scenario tables are ~0.5 MB a scenario (782 MB for the run);
  raw traces and branch dumps are deleted after each run unless `--keep`.

## Results (m5 gate scenarios)

Scenario set (`--m5 --corpus-every 5`, run `m5`): every scenario the m5 gate
lists except four in five of the 717 explorer corpus scenes (255 of them ran):
all 557 matrix witnesses, the character and UCF witnesses, the 48 + v2 corpus
matches, and **all 109 recorded Slippi replay bridges (499 k ticks)**, 1,472
scenarios and 2.17 M ticks in all. Five did not run (`slp_bf_marth_peach_t6500`,
`corpus_v3_fd_donkeykong_fox4_e3e98e6dc_p0`, `bowser_thrown_lw_fd_fox4`,
`drmario_held_turnip_airhi_ps_peach`, `corpus_v3_fd_iceclimbers_peach4_e0696dc6c_p1`:
killed or timed out under load; `--resume` reruns them).

**7,919 of 19,827 retail functions execute** (40%). In them, 64,512 of 84,256
branch sites ran (77%), and 13,176 of 34,815 conditional sites went both ways
(38%): executed functions are mostly but not wholly exercised.

| subsystem | functions | executed (m5) | executed (scope runs) | only in scope runs |
|---|---:|---:|---:|---:|
| `melee/ft` | 6808 | 4593 | 2858 | 326 |
| `melee/it` | 4277 | 1224 | 1711 | 975 |
| `melee/gr` | 2617 | 273 | 327 | 135 |
| `melee/gm` | 1645 | 111 | 99 | 2 |
| `sysdolphin/baselib` | 1078 | 615 | 610 | 16 |
| `melee/lb` | 535 | 221 | 221 | 13 |
| `melee/mn` | 431 | 2 | 2 | 0 |
| `melee/if` | 321 | 75 | 71 | 3 |
| `melee/pl` | 313 | 173 | 169 | 10 |
| `melee/mp` | 229 | 177 | 171 | 8 |
| `dolphin/os` | 207 | 71 | 71 | 0 |
| `dolphin/gx` | 154 | 103 | 110 | 7 |
| `melee/ty` | 151 | 0 | 0 | 0 |
| `MetroTRK` | 133 | 0 | 0 | 0 |
| `melee/cm` | 127 | 49 | 53 | 6 |
| `melee/vi` | 84 | 0 | 0 | 0 |
| `melee/db` | 80 | 6 | 9 | 3 |
| `dolphin/dvd` | 73 | 19 | 19 | 0 |
| `melee/ef` | 65 | 53 | 54 | 6 |
| `dolphin/ax` | 57 | 43 | 41 | 0 |
| `MSL` | 52 | 23 | 25 | 3 |
| `dolphin/mtx` | 27 | 25 | 25 | 0 |
| `melee/sfx` | 21 | 20 | 17 | 0 |

(Grouped by the first two path components of the `splits.txt` unit; the full
per-unit table is `m5.units.tsv`. The menus, trophies (`ty`), cutscenes
(`vi`) and the debugger stub are outside match play, as expected.)

Most executed (entries over all scenarios): the math and animation core,
every one hit by all 1,472 scenarios: `fabsf` 2.92 G, `PSMTXConcat` 2.32 G,
`HSD_AObjInterpretAnim` 2.12 G, `parseFloat` (fobj.c) 1.98 G, `sinf` 1.49 G,
`cosf` 1.44 G, `WriteMTXPS4x3` 1.33 G, `HSD_FObjGetState` 1.22 G,
`HSD_FObjInterpretAnim` 1.18 G, `FObjUpdateAnim` 1.18 G, `GXLoadPosMtxImm`
1.09 G, `HSD_Index2PosNrmMtx` 0.89 G, `JObjUpdateFunc` 0.80 G,
`HSD_MtxInverseTranspose` 0.74 G. The ranking for any subset is a sort of
`m5.tsv` by `entries`.

## Out-of-scope runs: what breadth requires

Six retail matches outside the port's scope, played by level-9 CPUs with no
inputs (`--scope`; savestates made by `make_boundary.make_savestate` with two
coverage-only boundary-script keys, `item_frequency` and `cpu_players`, under
`target/coverage/scope/`, never registered): 7,200 ticks each.

| run | stage | players | items | functions | not in m5 |
|---|---|---|---|---:|---:|
| `scope_fd_falcon_puff_cpu` | Final Destination | Falcon, Jigglypuff | off | 2,795 | 19 |
| `scope_corneria_4cpu_items` | Corneria | Fox, Falco, Pikachu, Mario | very high | 3,944 | 489 |
| `scope_brinstar_4cpu_items` | Brinstar | Samus, Link, Kirby, DK | very high | 4,361 | 790 |
| `scope_hyrule_4cpu_items` | Hyrule Temple | Ness, Peach, Yoshi, Ice Climbers | very high | 4,059 | 488 |
| `scope_ps_4cpu_items` | Pokemon Stadium | Mewtwo, G&W, Ganondorf, Pichu | very high | 4,272 | 667 |
| `scope_mutecity_4cpu_items` | Mute City | Roy, Young Link, Dr. Mario, Bowser | very high | 4,104 | 720 |

Together they execute 6,706 functions, **1,513 of them never run by the m5
set** (union 9,432, 48% of retail). Of the 1,513: 975 are items (`melee/it`,
mostly `it/kinds`: `itflipper.c` 36, `itkusudama.c` 35, `itbox.c` 32,
`itcapsule.c` 29, `itrshell.c` 29, `ittaru.c` 29, `itlgun.c` 27,
`itlucky.c` 27, `itmball.c` 27, `itkamex.c` 26, `itsscope.c` 25,
`itparasol.c` 24, `itmarumine.c` 23, `itgshell.c` 23, `itscball.c` 21,
`itharisen.c` 21, `itfflower.c` 20, `ithammer.c` 20, `itwstar.c` 19,
`itbombhei.c` 19, ...), 135 stages (Corneria 47,
Brinstar 33, Mute City, Hyrule Temple, `ground.c` 19), 326 fighter code
(item use and the common states around it: `ftCo_0A01.c` 30, Kirby's other
copy abilities, character specials of the eleven characters these matches
add). The FD CPU match adds only 19: level-9 CPU play on a tournament stage
runs almost nothing the gate set does not (the CPU AI itself, `ftcpuattack.c`,
is already reached through Nana's follow AI). Item-heavy, non-tournament,
four-player play is where breadth lies: one two-minute CPU match per stage
lights up several hundred functions the gate never runs.

## Files

- `target/coverage/<run>.tsv`: one row per retail function (19,827, hit or
  not): address, name, unit, size, entries, branch executions, branch sites
  hit / total, conditional sites seen both ways / total, scenarios hit (count
  and the first eight names).
- `<run>.hits.tsv`: address x scenario, entries and branch executions.
- `<run>.units.tsv`: per decomp source file.
- `<run>.runs.tsv`: per scenario: ticks, wall and emulation time, input
  clock, determinism verdict, functions hit.
- `<run>/<name>.functions.tsv`, `<name>.sites.txt`: the per-scenario tables
  the aggregate is rebuilt from (`--aggregate`).

## Caveats

- Functions are `symbols.txt`'s; a symbol whose size is wrong mis-assigns a
  branch to its neighbour (a few one-function hits in unrelated stage units
  look like this).
- Entries miss a function entered by falling through from the previous one
  (none known in Melee) and count a recursive `bl` as an entry.
- Dolphin HLE-patches a few SDK functions (OSReport and friends); those do not
  appear.
- Coverage is of the ticks recorded: menus, the match-start construction
  before the boundary savestate, and the results screen are not in it.
