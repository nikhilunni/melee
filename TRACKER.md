# Project tracker

Status legend: `[ ]` todo, `[~]` in progress (add date), `[x]` done,
`[!]` blocked (say on what), `[-]` out of scope for now.

Keep this file honest. A task is `[x]` only when its tests pass in
`cargo gate` and it is committed. When you finish a session, update the
task lines you touched and add one line to the session log.

## Current focus

**The disc is present, extracted, and the oracle works.** `harness/roms/idle_ys_fox.sav` + `harness/traces/idle_ys_fox.expected.jsonl` are the first real-game trace (two idle Foxes, Yoshi's Story, 600 frames). Fusion audit complete workspace-wide. Fox's Wait1 animation plays through the port. **Milestones 1-3 gates passed, plus the match-start scenario.** The port reproduces a Fox vs Fox match on Final Destination from its first initialised frame (entry warp, fall, landing, idle) for 600 ticks bit-exactly, RNG included, and the bone oracle matches all 73 bones. **Milestone 4 has started (2026-09-09):** the tick tracer injects scripted inputs and records the pad each tick consumed; five movement scenarios (walk, dash, jump, turn, squat; 300 ticks each from `idle_fd_fox.sav`) are recorded with RNG ledgers; the port replays their pads and stops at each first transition with an explicit `unimplemented`. Squat/Turn/Walk (M4-T1), Dash/Run/RunBrake with dust effects (M4-T2) jumps/fast fall/landing (M4-T3), shield/spot dodge/roll (M4-T4) air dodge/wavedash/backward jump/ledge (M4-T5) and TurnRun/WalkFast/ledge climb+escape (M4-T7) are ported and gated: 17 scenario gates; dash and jump need dust effects (new particle draw sites) next. A `melee-ef` crate for the effect layer is still pending. Older: Milestone 3 groundwork (melee-gr Yoshi's Story, melee-ft init and frame order, melee-sim loop).

Older note: `harness/roms/GALE01.iso` (gitignored)
matches both hashes in `docs/ISO.md`; `harness/roms/sys/main.dol` matches
the decomp's `build.sha1`; a copy of `main.dol` is at
`third_party/melee-decomp/orig/GALE01/sys/main.dol` where the decomp's
`dtk`/`objdiff` tooling expects it (that path is gitignored in the
submodule). The Dolphin scripting build is at
`~/Projects/dolphin-scripting/build/Binaries/Dolphin.app` (`docs/DOLPHIN_BUILD.md`).

Nothing disc-dependent has been run yet. The next session should start
with the three items under "Milestone 0: disc arrived" below; they are
independent and can run in parallel. Every one of them is the first time
the port meets the real game, so expect surprises and record them here.

## Blockers

- ~~Oracle sampling point~~ **fixed 2026-09-08**: `harness/dolphin/tick_trace.py` samples on the memcheck of the scheduler tick counter (`gm_80479D58`, store at retail 0x801A4FB8, right after `HSD_GObj_80390CFC` returns). `run_scenario.py --tick-trace` records, decodes and validates; `harness/validate_ticks.py` checks +1 animation steps and LCG-consistent seeds. `idle_fd_fox.tick.expected.jsonl`: 600 ticks, 0 violations, rerun byte-identical.
- **RNG consumers in an idle match.** User chose FD (unlocked 2026-09-08 by OR-ing bits 6-7 into the save's stage unlock mask via `drive.py poke-or`; Battlefield came with it). But FD is *not* RNG-quiet: `idle_fd_fox` draws 1..61 times per tick (histogram in the run output; pattern 6k+1), from `gr/grlast.c` background lights/flicker (lines 440-515, 652-658), plus the Wait1/Wait2 choice. Matching `rng.seed` on FD means porting grlast.c's per-tick decoration logic (1,035 lines, no particles or items). User decision (2026-09-08): port FD's decorations now so `rng.seed` is part of the M3 gate; stay on FD; keep the other stages' RNG dependencies tracked in Milestone 6.

## Decisions

| Date | Decision | Why |
|---|---|---|
| 2026-09-08 | Layered workspace, one crate per subsystem and per character | Build time. 435k lines of game C cannot be one crate. |
| 2026-09-08 | Internal state need not match retail memory layout; `Snapshot` + schema handles comparison | Lets the port use enums and real types. |
| 2026-09-08 | Floats in traces compare by bit pattern only | JSON round trip of the decimal caused false divergences. |
| 2026-09-08 | Oracle reads retail memory via Dolphin scripting; never a modified DOL | Keeps Slippi replays and community tools valid as test inputs. |
| 2026-09-08 | Target Felk's Dolphin scripting fork; GDB stub as fallback | Only option with savestate, frame hook, memory read, input in one process. |
| 2026-09-08 | Slippi fixtures from hohav/peppi (MIT), not slippi-js (LGPL) | License. |
| 2026-09-08 | Commit the empirically captured `frsqrte`/`fres` tables (user decision) | Captured from executing the instructions in Dolphin with no Dolphin source consulted (`harness/gekko_probe/README.md`). Table values describe hardware behaviour, but Dolphin's emulation is GPLv2 code; user to decide. Model: frsqrte 32 (base, slope) entries by exp parity + top 4 mantissa bits; fres 32 entries by top 5 bits, single-range clamp. |
| 2026-09-08 | Bulk porting delegated to Codex (`gpt-6-astra`) via `tools/codex-task.sh`; Fable subagents kept to one or two | User: Fable fan-out too expensive. Guardrail is the bit-exact test suite; Claude reviews and commits. |
| 2026-09-08 | Melee-specific on-disc structs (`coll_data`, `ftData`, `map_head`) are read by the owning game crate (`melee-mp`, `melee-ft`, `melee-gr`) in a `desc`-style module that depends on `hsd-archive`; `hsd-archive` stays HSD-only and never depends on `melee-types` | Keeps the archive crate a leaf; game crates already know their own types. |
| 2026-09-09 | Scripted scenarios: the TOML `inputs` schedule drives Dolphin at VI frames; the port replays `HSD_PadGameStatus` as recorded at each tick boundary (`inputs.pN` beside the expected record, `melee-sim/src/inputs.rs`) | VI-to-tick alignment is not modelled; pads are inputs, never compared state. Several scenarios from one savestate share its particle capture (`Scenario::boundary_path`). |
| pending | Retail asm workflow once disc arrives | `dtk` disassembly vs `objdiff`; how agents look up a function's asm. |

## Milestone 0: Infrastructure

- [x] Cargo workspace, layered, dev profile tuned (2026-09-08)
- [x] `melee-diff` canonical trace format and first-divergence report
- [x] Harness: symbol resolver over decomp `symbols.txt`
- [x] Harness: schema-driven memory decoder
- [x] Harness: `gen_schema.py` extracting struct layouts from header comments (692 Fighter fields, 35 Item fields)
- [x] Harness: GObj fighter-list walk with fake-memory tests
- [x] Harness: Dolphin script skeleton (`trace_scenario.py`)
- [x] Docs: ISO, oracle design, Dolphin options, plan
- [x] Decomp pinned as submodule
- [x] Build Dolphin scripting fork on macOS arm64 (`docs/DOLPHIN_BUILD.md`; binary under `~/Projects/dolphin-scripting/build/Binaries/`)
- [x] Verify `frameadvance` fires with Null video backend (yes, even with no disc, via `-e probe.dol`)
- [x] Measure oracle frames/sec with a booted game: 60 fps realtime, 131 fps unlimited (600 frames in 4.6 s; 6.8 s wall incl. boot + savestate load)

### Milestone 0: disc arrived (do these first)

- [x] (2026-09-08) **Extract the disc filesystem.** Write `harness/extract_fst.py` (GameCube FST: offset at 0x424, size at 0x428, 12-byte entries, string table after) to `harness/roms/files/` + `sys/`. Expect ~1200 files incl. `PlFxNr.dat`, `PlFx.dat`, `PlFxAJ.dat`, `PlCo.dat`, `GrNLa.dat` (Final Destination). Then `crates/hsd-archive/tests/real_dat.rs`: parse those five, assert file_size/relocs/publics, read the Fox root `JObjDesc` and count joints; document the `PlFxAJ.dat` animation layout and the `GrNLa.dat` `map_head`/`coll_data` offsets. Tests must skip cleanly when the disc is absent.
- [x] (2026-09-08) **Boot the game in Dolphin scripting and get the first real trace.** Launch `Dolphin -e harness/roms/GALE01.iso --script harness/dolphin/trace_scenario.py` with the config flags from `docs/DOLPHIN_BUILD.md`; confirm `on_frameadvance` fires and `memory.read_u32(seed)` changes. Navigate to a Vs match (Fox, Final Destination, 1 stock, no items, CPU off or a second human slot idle), save a savestate at the first playable frame to `harness/roms/idle_fd_fox.sav`, run the `idle_fd_fox` scenario for 600 frames, `decode.py` it to `harness/traces/idle_fd_fox.expected.jsonl`. Verify the fighter-list walk finds one fighter and `p0.cur_pos` is sane. Record frames/sec. Note: `walk.py` assumes `HSD_GObj_Entities` layout from gobj.h; this is its first real test.
- [~] (2026-09-08) **Retail assembly lookup.** In the decomp submodule run its tooling (`python configure.py` then `ninja` may be heavy; `dtk dol split` or `objdiff` per `docs/getting_started.md`) to get per-function disassembly from `orig/GALE01/sys/main.dol`. Deliver `harness/asm.py <symbol>` that prints a function's retail asm. Then start the **fusion audit**: `grep -rn "FUSION AUDIT PENDING" crates/` lists every site (gekko-math 25, melee-lb 14, hsd-anim 92, melee-mp many); for each, read the asm, replace with `fmadds`/`fmsubs`/`fnmsubs` or confirm unfused, and remove the marker. Start with gekko-math `sqrtf` and `sinf` since everything depends on them, then `lbtrigf`, then mtx.
- [x] (2026-09-08) **Gate Milestone 2 — PASSED.** 0 mismatches over 3,080 bone-matrix/SRT words vs the real game (`docs/M2_GATE.md`, `crates/melee-sim/tests/m2_gate.rs`). Original text: With the trace from the Dolphin item and the skeleton from the extraction item: read Fox's bone matrices from the oracle (JObj `mtx` at each joint via `HSD_JObjGetMtxPtr`; find the Fighter's root JObj through `fp->x28_jobj` or equivalent in `ft/types.h`) and compare against `hsd-anim` evaluating the Wait animation at the same frame. This is the first bit-exact comparison against the real game.
- [x] Verify savestate load is synchronous with next frame (yes; memory is frozen during a callback, load lands on the saved boundary)
- [ ] Small C++ patch or debugger workflow to set code breakpoints for intra-frame phases
- [x] `Snapshot` trait in `melee-types`, `RecordSink` in `melee-diff`, `SchemaCoverage` in `melee-sim` with stale-exclusion detection
- [x] RNG ledger: `harness/dolphin/rng_ledger.py` + `harness/rng_ledger_report.py` (memcheck on `seed`, PC/LR via patched fork) attribute every draw per tick to its `bl` site
- [ ] Golden fixture recorder: break on function entry/exit, dump args and touched memory to `harness/goldens/`
- [ ] Retail asm lookup tool: given a symbol, print its disassembly from `main.dol` (needs disc)
- [ ] CI: `cargo gate`, clippy `-D warnings`, harness pytest, `gen_schema.py --check`

## Milestone 1: Math and RNG (`gekko-math`)

- [x] HSD RNG, bit-exact with tests
- [x] FMA wrappers in PowerPC operand order
- [x] MSL `sinf`, `cosf`, `tanf`, `logf`, `frexp`, `fmodf`, classify, fabs
- [x] Gekko int/float conversion semantics (`fctiwz`, `__cvt_*`)
- [x] Native-C reference oracle test (bit-exact vs decomp C, `-ffp-contract=off`)
- [x] `frsqrte` / `fres` hardware-exact from captured tables (golden fixture 10,600 pairs; full captures 148k pairs, 0 mismatches; `sqrtf` bit-exact vs native C)
- [ ] FMA audit of the 25 marked sites against retail asm (blocked on disc)
- [ ] Int-conversion audit of the 1 marked site (blocked on disc)
- [x] Paired-single matrix routines: transcribed from asm in `hsd-anim::mtx`; fusion audited against retail (`crates/hsd-anim/tests/ref/FUSION_AUDIT.md`)
- [ ] `fmuls` on a double-width estimate result (Gekko truncates frC to 25 bits): needed once frsqrte is exact, for `PSVECMag`/`PSVECNormalize`

## Milestone 2: HSD engine (`hsd-archive`, `hsd-gobj`, `hsd-anim`)

Gate: load one character and one stage archive, evaluate the wait
animation, match bone matrices from the oracle. **Gate passed 2026-09-08**
(Fox Wait1 frame 6 and 7 on the `idle_ys_fox` savestate, bit-exact on all
recomputed bones). Remaining items below are breadth, not gate blockers.

- [x] `hsd-archive`: header, relocs, publics, externs, strings; synthetic tests
- [x] `hsd-archive`: typed readers for JObjDesc/DObj/MObj/AObj/FObj/AnimJoint/MatAnimJoint/ShapeAnimJoint and Melee FigaTree (20 tests)
- [x] `hsd-archive`: test against a real `.dat` (7 tests; Fox 73 joints, GrNLa coll_data 16 verts/16 lines; `docs/DISC.md`)
- [x] `hsd-gobj`: `gobj.c`, `gobjproc.c`, `gobjplink.c`, `gobjgxlink.c` (link/priority only, no GX), `gobjobject.c`, `gobjuserdata.c` (~800 lines)
- [x] `hsd-anim`: `mtx.c`, `quatlib.c`, and the SDK `PSMTX*`/`PSVEC*` paired-single kernels; all 31 fusion sites audited (Codex), ~22% of mtx/quat sweep inputs differ from unfused
- [ ] `hsd-anim`: `PSMTXRotAxisRad` (used by jobj.c, psdisp.c, cobj.c), `C_MTXLookAt`
- [ ] Wire `hsd_anim::mtx::InverseTrig` to `melee_lb::trigf` in `melee-sim` (hsd-anim must not depend on melee-lb)
- [x] `hsd-anim`: `aobj.c`, `fobj.c` keyframe evaluation (retail-faithful native-C oracle, 0 mismatches; Hermite and linear tracks are fused in retail)
- [ ] `hsd-anim`: confirm via Dolphin whether retail data ever hits the uninitialised single-key FObj path (see fobj.rs `FOBJ_UNINITIALISED_VALUE`)
- [x] `hsd-anim`: `jobj.c` hierarchy, matrix setup, dirty flags, SRT setters, anim application, ftparts bone lookup (52 tests)
- [x] `hsd-anim`: `load.rs` converts `JObjDesc`/`AnimJoint` trees into `JObjTree` (Codex; 12 synthetic tests + real Fox skeleton test)
- [x] `hsd-anim`: `dobj.c`, `mobj.c` data and anim plumbing, no render
- [ ] `hsd-anim`: `robj.c` (942) constraints. jobj.rs lists exactly which RObj entry points it calls (all currently no-ops, matching `robj == NULL`); check whether fighter skeletons carry RObjs before porting
- [ ] `hsd-anim`: `cobj.c` (1406) camera object, needed by `cm` later
- [ ] HSD `class.c`/`object.c` object model: decide Rust representation (traits vs enums)
- [-] Rendering: `tobj`, `pobj`, `lobj`, `tev`, `texp*`, `psdisp*`, `displayfunc`, `video`, `shadow`, `fog` (Milestone 8)
- [-] Audio: `axdriver`, `synth`, `hsd_3A94` and other sound files (Milestone 8)
- [x] (2026-09-08) `hsd-particle`: **matches the real game**: restored from the live dump at the `idle_fd_fox` savestate, 600/600 ticks reproduce the RNG draw counts, draw-site order and per-tick seed; 8/8 dumped frames match all 10,120 particle/generator fields bit-for-bit (`tests/live_fd.rs`). Remaining breadth (nested spawning, tornado/rectangle emitters, more opcodes) tracked in `docs/PARTICLES.md`. History: first pass committed (Codex): bank parsing, generator emission shapes, particle bytecode interpreter subset, list ordering, six RNG sites with gating conditions (`docs/PARTICLES.md`); 35 tests incl. real FD bank. Cold-start draw counts 43,19,1,1,61,... vs live ledger 61,7,7,19,25,...: the live generator/particle state at the savestate must be captured (`harness/dolphin_particle_snippet.py`, `docs/PARTICLES_DUMP.md`) and restored before the sequences can be compared. Remaining: nested spawning, AppSRT callbacks, tornado/rectangle emitters, more opcodes. Original note: `generator.c` (1,244) and `particle.c` (3,095) lifetime/emission logic. **Not visual-only**: the RNG ledger shows every idle FD tick draws once in `hsd_8039EE24` (generator update) plus six per live particle (2 sites in `hsd_8039DAD4`, 4 in `hsd_8039930C`); this is the 6k+1 pattern. Required for `rng.seed` parity on every stage. `psdisp.c` rendering stays out of scope.
- [-] `spline.c` (visual only)

## Milestone 3: One fighter idle (`melee-lb`, `melee-mp`, `melee-ft`, `ft-fox`)

Gate: `harness/scenarios/idle_fd_fox.toml`, 600 frames bit-exact.
**Gate passed 2026-09-09**: `melee-sim gate harness/scenarios/idle_fd_fox.toml` reproduces all 600 ticks x 49 keys (both Foxes' 24 fields and `rng.seed`) from the imported savestate boundary, with the real scheduler (`hsd-gobj::World`), stage, particle system and fighters; no per-tick oracle inputs. Remaining items below are breadth toward Milestone 4.

- [x] `melee-lb`: `lbtrigf.c` (atan2f, atanf, asinf, acosf, lb_sqrtf) and `lb_00CE.c` expf/powf
- [x] `melee-lb`: `lbanim.c` FigaTree -> JObj attachment (Codex; real Fox Wait1: 120 frames, 73 nodes, 49 animated joints); translation-filtered path `fn_8001E60C`/`lbAnim_8001E7E8` deliberately skipped, see anim.rs header
- [x] `melee-ft`: `desc.rs` reads `ftData.xC` animation table from PlFx.dat and slices sub-archives out of PlFxAJ.dat (Codex)
- [ ] `melee-lb`: `lbvector.c`, `lbcollision.c`, `lbarchive.c`, `lbfile.c` (headless file access)
- [ ] `melee-lb`: remaining `lb_*` files as needed by callers (17k lines total)
- [x] `melee-mp`: `mplib.c` (103 fns) and `mpcoll.c` (complete) as `CollMap`; 42 tests on synthetic FD; all 39 fusion sites audited against retail (33 fused), geometry native-C oracle 200k inputs 0 mismatches (`tests/ref/FUSION_AUDIT.md`)
- [ ] `melee-mp`: `mpisland.c` (626 lines; feeds CPU AI and Link hookshot) — `CollMap::island_update` is the hook
- [ ] `melee-mp`: terrain sound-id tables (`mpLib_803BD3D8..`) once an sfx layer exists
- [x] `melee-mp`: `desc.rs` reads `coll_data` from an archive (Codex); real GrNLa.dat loads into `CollMap`, floor/ledge queries verified (3 real-stage tests)
- [ ] `melee-gr`: `ground.c`, `grlib.c`, `grdatfiles.c` plumbing for one stage (Final Destination)
- [~] (2026-09-08) `melee-gr`: `grlast.c` Final Destination decorations, first pass committed (procs, direct RNG sites, desc readers). **Finding:** the direct grlast.c logic draws on only 2 of 600 idle ticks; the 6k+1 draws per tick come from elsewhere (likely the HSD particle generator via a particle-spawn animation track on map 4). RNG ledger (`harness/dolphin/rng_ledger.py`, needs the fork patch `docs/patches/0002-scripting-read-pc-lr.patch`) attributes every draw: 120 idle ticks = 3,716 draws: 120 from `hsd_8039EE24+0xDC`, 599 each from six particle sites, 1 from `grLast_8021ADD0+0x270`, 1 from `ftCo_8008A7A8+0x114` (Wait anim choice). So the stage's own logic is nearly RNG-free; the particle system is the dependency.
- [ ] `melee-gr`: Dolphin capture of the FD `Ground` struct at the `idle_fd_fox` savestate (timers/phases) to seed the decoration state; then a 600-tick RNG-draw-count test against `idle_fd_fox.tick.expected.jsonl`
- [x] `melee-ft`: `ftanim.c` playback/attach/blend + `ftwaitanim.c` Wait1/Wait2 choice (Codex T7): both Foxes' animation state matches the FD trace 600/600 ticks; all 9 idle-choice RNG draws match the ledger (`src/anim/README.md`)
- [x] `melee-ft`: human input path `Fighter_Spaghetti_8006AD10` + 21 `ftCo_Wait_IASA` predicates (Codex T9): 600 ticks x 2 fighters, 108 input bytes + 7 CPU fields match (`src/input/README.md`)
- [ ] `melee-ft`: `fighter.c` init and per-frame update order, `ftcommon.c`, `ftcoll.c`, `ftlib.c`
- [x] `melee-ft`: `fighter/` composes data+physics+collision+animation+input into `Fighter` with spawn/reset, the Wait callback table, all 15 scheduler proc methods and the 24-key Snapshot (Codex T10): `tests/idle_fox_600.rs` replays both Foxes 600/600 ticks, all 24 fields each, bit-exact. Cold spawn at FD's y=10 markers enters Fall (retail-derived: one jump used, 10-frame ECB lock, one CPU-init draw). Non-idle branches are explicit `unimplemented` with C lines.
- [ ] `melee-ft`: `ftCo_*` action states for squat and turn (Wait done)
- [ ] `melee-ft`: physics (`ft_08A1.c` etc): gravity, friction, ground snap
- [x] `ft-fox`: attributes (Fox special block, 0xD4 bytes) read from PlFx.dat (Codex T4); init and Wait wiring still to do
- [x] `melee-ft`: `desc/` reads ftData attributes (0x184 bytes, grouped by concept), part table (54 entries, TopN/TransN/XRotN/YRotN=0/1/2/3), ECB joints [41,55,25,13,7,4], bone lists, PlCo common subset (`docs/FOX_DATA.md`)
- [x] `melee-sim`: scenario/asset loading, `InitialState::from_savestate_traces`, `World`-driven frame loop, 49-key trace, `run`/`gate` CLI (Codex T13; `crates/melee-sim/src/M3.md` lists imported vs computed state)
- [x] Record savestate and `expected.jsonl`: **`idle_ys_fox`** (Yoshi's Story) instead of FD, two idle Foxes, 600 frames, byte-identical on rerun (`docs/DOLPHIN_RUN.md`)
- [x] `idle_fd_fox` recorded (2026-09-08): FD unlocked via `poke-or 0x8045BF2A u16 0xC0` (save data `gmMainLib_804D3EE0->thing.x186A`), Stock 1 via `GameRules` bytes, items were already NONE. Savestate `harness/roms/idle_fd_fox.sav` at frame 50841, both Foxes in Wait at (+-60, 0.0001, 0). Tick trace `harness/traces/idle_fd_fox.tick.expected.jsonl`, 600 ticks, deterministic.
- [~] `pl/player.c` and `gm` match setup: the slot->kind/costume subset spawning needs is in `fighter/spawn.rs`; stocks/percent bookkeeping still to do

## Milestone 4: Movement (`melee-ft`)

Gate: scripted input scenarios for walk, dash, run, jump, double jump,
fast fall, ledge grab, ledge options, platform drop, wavedash.

- [x] (2026-09-09) **Match start gate passed** `melee-sim gate harness/scenarios/start_fd_fox.toml`: 600 ticks x 49 keys, 0 divergences, all RNG produced by the port (effect id -> particle spawn mapping from eflib.c/efasync.c lives in `melee-sim/src/effects/` pending a `melee-ef` crate; FD's tick-1 spawn comes from melee-gr's animation track). `harness/scenarios/start_fd_fox.toml`: savestate at the first initialised frame (both Foxes in Entry 322 at y=10); tick trace, RNG ledger and 600-frame particle dump recorded. Sequence: 322 -> 323 (t6) -> 324 (t35, anim frame holds at 10 until GO) -> Fall 29 (t65) -> Landing 42 (t75) -> Wait 14 (t105); P1 +5 ticks. Entry warp effects spawn new particle generators (new RNG sites). Done: hsd-particle matches 600/600 (1,051,650 fields, 23,092 draws; 9 external spawn requests as a fixture); melee-ft Entry/Fall/Landing + airborne physics match the 24 fields 600/600 in the state-callback replay. Dynamic-bone solver `lb_8001044C` + `ftdynamics.c` ported (Fox tail, bones 17-20); `start_fox_600` passes 600/600 x 24 fields; **bone oracle passes**: all 73 bones' SRT for both fighters over 130 start ticks + 8 idle ticks (tick-aligned dump), and 202,020 matrix words vs post-render dumps (matrices are render-time caches, so they are compared against VI-sampled captures, see `melee-ft/src/dynamics/README.md`). Fixed en route: part-animation ownership must be cleared before the pose reset at motion entry. Next: melee-sim `gate start_fd_fox` (wire fighter/stage effect requests into hsd-particle spawns; import the entry-state boundary).

- [x] (2026-09-09) Scripted-input recording: `tick_trace.py` issues per-port holds from the scenario `inputs` (a top-level TOML key; the tracer and `Scenario` reject it under a `[[fighters]]` table) and records `HSD_PadGameStatus[4]` per tick; `decode.py` emits `record["inputs"]`; `validate_ticks --scripted` (run_scenario adds it automatically). Gotcha: Dolphin's `StickX 0.5` is raw 64, which HSD scales to 0.8 (dash threshold); `0.35` gives WalkMiddle.
- [x] (2026-09-09) Scenarios recorded from `idle_fd_fox.sav` (300 ticks, P1 scripted, P2 idle) with tick traces and RNG ledgers (`<name>.ledger.raw.jsonl`): `squat_fd_fox` (39 Squat t31, 40 SquatWait t38, 41 SquatRv t91, Wait t101), `turn_fd_fox` (18 Turn t31, 15 WalkSlow t42, 16 WalkMiddle t55, Wait t61), `walk_fd_fox` (15 t31, 16 t41, Wait t71), `dash_fd_fox` (20 Dash t31, 21 Run t42, 23 RunBrake t56, Wait t74), `jump_fd_fox` (24 KneeBend t31, 25 JumpF t34, 42 Landing t55, Wait t85; then 24/25 t101-104, 27 JumpAerialF t113 with fast fall from t135, 42 t152, 40/41 SquatWait/Rv t156-177, Wait t187). RNG: squat/turn/walk draw nothing new; dash and jump add particle sites (`hsd_8039DAD4+0x710/+0x900`, `hsd_8039930C+0x22D4/+0x1504/+0x31EC/+0x3280/+0x3564`, `hsd_8039F05C+0x1F4`) and `ftCo_8009F834+0x4A8/+0x4CC/+0x4F0` (3 draws per dust effect).
- [~] (2026-09-09) Codex M4-T1 implemented and verified, left uncommitted as requested: Squat/SquatWait/SquatRv, Turn, WalkSlow/Middle/Fast; all three movement gates 300x49, zero divergences; raw-field state replays pass; existing gates pass; `cargo gate` 550 passed / 0 failed / 1 pre-existing ignored; clippy clean. See `crates/melee-ft/src/fighter/M4_FOX.md`.
- [x] (2026-09-09) Codex M4-T2: Dash/Run/RunBrake (`ftCo_Dash.c`, `ftCo_Run.c`, `ftCo_RunBrake.c`) + the subaction GFX command and dust spawner `ftCo_8009F834` (fmadds at 8009FCF8/FD1C/FD44 audited): Dash GFX 0x3FF -> async kind 6 -> effect 5 -> particles 9 -> 7/8; Run 0x3FE -> kind 5 -> generator 263 -> 264/265; RunBrake 0x401 -> kind 5 -> generator 90. The missing particle behaviour was shared AppSRT inheritance (child generators keep the parent's transform until the last child dies). Gate `dash_fd_fox` 300x49; ordered particle RNG sites match all 300 ticks; report `melee-ft/src/fighter/M4_DASH.md`. Follow-up M4-T2b: `hsd-particle/tests/live_fd_dash.rs` replays the 300-tick dash particle dump with the port's own spawn requests as a fixture: 441,857 generator/particle/AppSRT fields match, 0 mismatches. Excluded, documented in the test: 532 AppSRT *display-cache* fields (`psDispSubAppSRT`, psdisp.c:1400-1439: cached model matrix, camera-view product, column lengths, status bits, `psFrameNum`), which need the camera and render schedule (Milestone 8); `hsd-particle/src/appsrt.rs` ports that display update with explicit view/frame inputs for later.
- [!] (2026-09-09) M4-T2 particle-dump follow-up: strict 300-tick replay compares 442389 fields; generator/particle and AppSRT SRT/ownership fields match, but 425 display-cache values differ from tick 50. Await display-time camera view matrices and psFrameNum/pass order, absent from the particle capture. New test and cargo gate remain red; existing M4/idle/start/live gates and clippy pass. No commit. See `crates/melee-ft/src/fighter/M4_DASH.md`.
- [x] (2026-09-09) Codex M4-T3: KneeBend/JumpF/JumpAerialF (Fox's basic double jump via the new `CharacterCallbacks::aerial_jump_style` hook; Ness/Yoshi/Peach/Mewtwo styles named, unported), fast fall (`ftCommon_CheckFallFast` 8007D528), Landing into SquatWait; dust: jump GFX 0x402 -> particle 89 and 0x403 -> 94 (async kind 0, live root joint, no offset RNG), landing 0x404 -> effect 0x18 -> particle 10. Gate `jump_fd_fox` 300x49; `hsd-particle/tests/live_fd_jump.rs` matches 489,588 dump fields with no exclusions (shared `tests/support/dust_replay.rs`); report `melee-ft/src/fighter/M4_JUMP.md`. Backward jumps, jump cancels and relaxed/C-stick jump entry stay explicit `unimplemented`.
- [x] (2026-09-09) Second scenario batch recorded from `idle_fd_fox.sav` (tick traces + ledgers): `shield_fd_fox` (182 GuardOn t31, 179 Guard t39, 180 GuardOff t91, Wait t106; 1 extra particle draw pair), `spotdodge_fd_fox` (182 t31, 235 EscapeN t35, 178 GuardSetOff t57, 179, 180, Wait t96; one dust spawn), `roll_fd_fox` (182 t31, 233 EscapeF t35 to x=-26.4, 178 t66, 179, 180, Wait t96; two dust spawns), `airdodge_fd_fox` (full hop, 236 EscapeAir at the apex t141, 43 LandingFallSpecial t186, Wait t196; the first hop's L during jump squat is too early to wavedash), `ledge_fd_fox` (420 ticks: backward full hop 26 JumpB t34 drifting past the left edge, 252 CliffCatch t70, 253 CliffWait t77, 262/263 CliffJumpSlow t232/t246 from stick up, 42 Landing t283, Wait t313). Lesson: a double jump does not turn the fighter around, so grabbing a ledge after running off it needs a backward hop (facing stays toward the ledge). A stock-1 death ends the scene and resets the tick counter (the tracer reports a discontinuity).
- [x] (2026-09-09) Codex M4-T4: GuardOn/Guard/GuardOff/GuardSetOff (`ftCo_Guard.c`: shield size/health/tilt as typed state, collision volumes, shield pose, attached shield particle) and EscapeN/EscapeF (`ftCo_Escape.c`: invincibility as typed state, roll velocity, ground collision); Yoshi/Marth/Samus branches are `CharacterCallbacks::guard_variant`/`escape_variant` hooks. Gates `shield_fd_fox`, `spotdodge_fd_fox`, `roll_fd_fox` 300x49; report `melee-ft/src/fighter/M4_SHIELD.md`. Shield stun/damage/powershield/break and grab out of shield stay explicit `unimplemented` (M5). Follow-up M4-T6: particle dumps of shield/spotdodge/roll/airdodge/wavedash/ledge replayed field-by-field (`hsd-particle/tests/live_fd_*.rs`, 3,014,914 fields, 0 mismatches; only the documented AppSRT display-cache exclusion); one fix in the snapshot adapter's AppSRT generator reference.
- [x] (2026-09-09) `wavedash_fd_fox` recorded: L + diagonal-down on the first airborne frame (VI 34, Fox's 3-frame jump squat): 25 JumpF t34 (y=2.1), 236 EscapeAir t35, 43 LandingFallSpecial t36, Wait t46 having slid from x=-60 to -44.4; repeated at t121-138.
- [x] (2026-09-09) Codex M4-T5: air dodge (`ftCo_EscapeAir.c`) and LandingFallSpecial (43); gates `airdodge_fd_fox`, `wavedash_fd_fox` 300x49. FallSpecial 35 is never reached in these scenes.
- [x] (2026-09-09) Codex M4-T5: JumpB, ledge grab during airborne collision (`ftcliffcommon.c`), CliffCatch/CliffWait, CliffJumpQuick1/2 (262/263 are the *Quick* pair; the dust at t246 is the launch); gate `ledge_fd_fox` 420x49; report `melee-ft/src/fighter/M4_LEDGE.md`. CliffClimb/CliffAttack/CliffEscape stay explicit `unimplemented`.
- [ ] Input system: `fighter.c` input parsing, buffers, deadzones (`ftCo_800A2040` CPU vs pad path)
- [x] (2026-09-09) Third scenario batch recorded (tick traces + ledgers): `turnrun_fd_fox` (20 Dash t31, 21 Run t42, 19 TurnRun t61 at x=5.6, 21 Run left t91, 23 RunBrake t101, Wait t119; six dust spawns), `walkfast_fd_fox` (gradual tilt 0.35/0.6/0.95: 15 t31, 16 t41, 17 WalkFast t54, Wait t76; RNG-quiet), `ledgeclimb_fd_fox` (420 ticks: ledge grab as in ledge_fd_fox, 255 CliffClimbQuick t232, 15 WalkSlow t266, Wait t272), `ledgeescape_fd_fox` (420: 259 CliffEscapeQuick t232, Wait t281 at x=-50.7).
- [x] (2026-09-09) Codex M4-T7: TurnRun (`ftCo_TurnRun.c`), WalkFast verified against its first retail trace, CliffClimbQuick/CliffEscapeQuick (`ftCo_CliffClimb.c`, `ftCo_CliffEscape.c`); gates `turnrun_fd_fox`, `walkfast_fd_fox` 300x49, `ledgeclimb_fd_fox`, `ledgeescape_fd_fox` 420x49 (17 scenario gates green); report `melee-ft/src/fighter/M4_TURNRUN.md`.
- [x] (2026-09-09) `airjumpb_fd_fox` recorded: full hop, then stick back + X in the air: 28 JumpAerialB t51 (y=35.5, facing kept), drifted past the left edge into 32 FallAerial t101 and a ledge grab 252/253 at t104/t111; RNG only particle updates.
- [x] (2026-09-09) JumpAerialB and FallAerial; gate `airjumpb_fd_fox` (M4-T9)
- [ ] RunDirect, skid, CliffClimbSlow/CliffEscapeSlow (the slow variants are chosen at 100%+ damage, so they wait for Milestone 5 combat)
- [ ] Jumps backward, air movement, L-cancel
- [ ] Ledge states (`ftcliffcommon.c` and `ftCo_Cliff*`)
- [ ] Shield, roll, spot dodge, air dodge (no hit interaction yet)
- [ ] Scenarios for each, recorded from Dolphin
- [x] (2026-09-09) Battlefield (Codex lane B1): `melee-gr/src/battle/`, `melee-sim/src/scene_stage.rs` stage dispatch, HSD light reader, particle opcode 0xB3; gates `idle_bf_fox`, `start_bf_fox` 600x49; particle replays 6,409,194 fields, 0 mismatches; `docs/BATTLEFIELD.md`.
- [x] (2026-09-09) Codex lane B2: platform landing, `ftCo_Pass` drop-through and one-way lines (`platform_bf_fox` 300x49); Yoshi's Story idle (`melee-gr/src/story/`, Randall and the Shy Guy schedule for the idle interval, spline support; `idle_ys_fox` 600x49, particle replay 51,303 fields). `docs/YOSHIS_STORY.md`. Full Shy Guy motion/hit/escape and a YS match start remain.
- [ ] Platform drop-through with an opponent on the platform; Yoshi's Story match start (needs a start_ys_fox recording)

- [x] (2026-09-09) **Second character: Marth (Codex M4-T8).** Characters and FD unlocked in RAM (`poke-or 0x8045BF28 u16 0xFFFF`; rules re-poked to stock 1 each session). Savestates `harness/roms/{idle,start}_fd_marth.sav` (P1 Marth kind 18 at -60, P2 Fox idle at +60) with tick traces, ledgers, particle and bone dumps. New `ft-mars` crate; `melee-ft` loaders take a `CharacterDescriptor` (files, symbols, part/animation counts); `melee-sim` scene holds a closed `SceneFighter` enum of boxed `Fighter<C>`; `melee-gr/src/music.rs` models the match-start alternate-music `HSD_Randi(100)` that only fires when all characters are unlocked (found via the ledger: `grLast`/`Ground_801C24F8+0x1B4`). Gates `idle_fd_marth`, `start_fd_marth` 600x49; bone oracle 90 Marth + 73 Fox bones, 202,672 SRT words. Report `melee-ft/src/fighter/M4_MARTH.md`, data `docs/MARTH_DATA.md`. Next: Marth variants of the 16 movement scenarios (recording).

- [x] (2026-09-09) **Marth variants of all 16 movement scenarios** recorded from `idle_fd_marth.sav` with the Fox input scripts (`<name>_fd_marth.toml`; airjumpb needed an earlier drift back, Marth's longer air drift carried the Fox timing to a death). **Without any code change 8 of 15 pass**: squat, turn, walk, dash, jump, wavedash, turnrun, walkfast (300x49 each). The rest stop at explicit boundaries: shield/spotdodge/roll at the Marth shield-offset hook (`ftCo_Guard.c:342-346`), airdodge at FallSpecial 35 (Marth's full-hop air dodge lands in special fall), ledge/ledgeclimb/ledgeescape at directional Fall (FallF/FallB from a residual x velocity after JumpB), and airjumpb (both characters) at JumpAerial animation end -> FallAerial 32.
- [x] (2026-09-09) Codex M4-T9: FallF/FallB (directional falls blend a secondary animation while keeping the Fall state), FallAerial, FallSpecial + LandingFallSpecial from an airborne air dodge, Marth's `guard_variant` override (ft-mars: shield model + sound selection, no custom offset). All 16 Marth movement scenes and both airjumpb scenes pass; `m4_gate` 65 tests; report `melee-ft/src/fighter/M4_FALLS.md`.

- [x] (2026-09-09) **Parallel lanes prepared.** `harness/record.py` records tick trace, ledger, particles and bones in one command; melee-sim characters register in one line (`scene_characters!`); three git worktrees under `../melee-lanes/{combat,battlefield,chars}` (CLAUDE.md "Parallel lanes"). Recorded: `jab_fd_marth` (first hit: Marth dash into Fox, push apart, jab at t121, Fox damage state 79 at t124 for 4%, Wait t150; ledger adds `ifStatus_802F4B84+0x1C0/+0x1D0`, the HUD percent shake), Battlefield `idle_bf_fox`/`start_bf_fox` (Fox vs Fox spawn at (0,0) and (0,54.4); idle ledger adds `hsd_8039DAD4+0x900` x232 from the stage's own generators), and Falco (kind 22), Captain Falcon (kind 2) and Peach (kind 9) idle/start plus all sixteen movement scenes each (airjumpb needs the Marth drift timing for all three; Falcon's three ledge scenes need the drift back at VI 60). Lanes launched: A1 combat (jab), B1 Battlefield, C1 Falco; Falcon and Peach queued in the character lane.

## Milestone 5: Combat (`melee-ft`, `melee-lb`)

Gate: two-fighter scenarios with hits, shields, grabs, KOs.

- [x] (2026-09-09) **First hit (Codex lane A1):** `jab_fd_marth` gate 300x49: Marth Attack11 into idle Fox with fighter overlap push before it; typed hitboxes from the subaction commands, hurtboxes, hit detection in retail pair order, hitlag, damage, knockback, hitstun, Fox DamageN2 -> Wait; slash effects; new `melee-if` crate for the HUD percent-shake RNG (`ifstatus.c`, s_link 17). Particle replay 467,132 fields. Report `melee-ft/src/fighter/M5_HIT.md`. Next (A2, scenes scripted, recording pending): attacker swap `jab_fd_fox`, `fsmash_fd_marth` launch/tumble, `shieldhit_fd_marth`, `grab_fd_marth`.
- [ ] Hitbox/hurtbox system breadth (`ftcoll.c`, `lbcollision.c`, `ftcolanim.c`): item and projectile collision, multi-hitbox priority
- [ ] Damage, knockback, hitlag, hitstun, DI, SDI, ASDI
- [ ] Shield damage and stun, powershield, shield break
- [ ] Grabs, throws, grab escape
- [ ] Tech, missed tech, getups
- [ ] Death and respawn, stock counting
- [ ] Attack action states for the common set (jab, tilts, smashes, aerials)
- [ ] Fox full moveset (`ft-fox`, 3829 lines) including specials
- [ ] Second character for matchup testing: Marth (`ft-mars`, 2184 lines)
- [ ] Item-free CPU AI (`melee-cpu`): `ftCo_0A01.c` (8.5k), `ftcpuattack.c` (2.7k), `ftcmdscript.c` (394)

## Milestone 6: Breadth

Gate: per-character and per-stage scenarios; every playable character idle,
moving, and fighting bit-exact.

Characters (decomp lines, `ft/kinds/`). Check off when the character's own
crate passes its scenarios.

| Character | Crate | Lines | Status |
|---|---|---|---|
| Fox | `ft-fox` | 3829 | `[x]` idle/start/17 movement scenes on FD and Battlefield; jab defender (A1) |
| Marth | `ft-mars` | 2184 | `[x]` idle/start/16 movement scenes; first hit as attacker (M4-T8/T9, A1) |
| Mario | `ft-mario` | 1257 | `[ ]` |
| Dr. Mario | `ft-drmario` | 399 | `[ ]` |
| Luigi | `ft-luigi` | 1922 | `[ ]` |
| Captain Falcon | `ft-captain` | 1705 | `[x]` idle/start/16 movement scenes (2026-09-09, lane C2) |
| Falco | `ft-falco` | 503 | `[x]` idle/start/16 movement scenes (2026-09-09, lane C1) |
| Sheik | `ft-seak` | 2858 | `[ ]` |
| Zelda | `ft-zelda` | 2218 | `[ ]` (transform pair with Sheik) |
| Peach | `ft-peach` | 2195 | `[ ]` |
| Jigglypuff | `ft-purin` | 2533 | `[ ]` |
| Pikachu | `ft-pikachu` | 2377 | `[ ]` |
| Pichu | `ft-pichu` | 421 | `[ ]` |
| Samus | `ft-samus` | 1846 | `[ ]` |
| Link | `ft-link` | 1865 | `[ ]` |
| Young Link | `ft-clink` | 553 | `[ ]` |
| Donkey Kong | `ft-donkey` | 2156 | `[ ]` |
| Bowser | `ft-koopa` | 1988 | `[ ]` |
| Ness | `ft-ness` | 5677 | `[ ]` |
| Yoshi | `ft-yoshi` | 3364 | `[ ]` |
| Mewtwo | `ft-mewtwo` | 2703 | `[ ]` |
| Mr. Game & Watch | `ft-gamewatch` | 3738 | `[ ]` |
| Roy | `ft-emblem` | 454 | `[ ]` |
| Ganondorf | `ft-ganon` | 354 | `[ ]` |
| Ice Climbers | `ft-popo`, `ft-nana` | 2415 + 1078 | `[ ]` (Nana AI in `melee-cpu`) |
| Kirby | `ft-kirby` | 18190 | `[ ]` (copy abilities; largest by far, split by hat) |
| Master Hand | `ft-masterhand` | 4067 | `[ ]` |
| Crazy Hand | `ft-crazyhand` | 3879 | `[ ]` |
| Giga Bowser | `ft-gigakoopa` | 391 | `[ ]` |
| Sandbag, Wireframes | `ft-sandbag`, `ft-zako` | 132 + 135 | `[ ]` |

Common fighter code shared by all: `ft/kinds/ftCommon` (30.4k lines) and
`ft/*.c` (35.3k lines). Split `melee-ft` when it passes 30k.

Stages (`gr/`, 56k lines across 77 files). Every stage's decorations must
be ported for `rng.seed` parity, not just its collision; known per-tick RNG
consumers are listed so they are not forgotten:

| Stage | File | Status | Known RNG / dynamic elements |
|---|---|---|---|
| Final Destination | `grlast.c` | `[~]` (M3) | background lights/flicker, transitions (1-61 draws/tick) |
| Battlefield | `grbattle.c` | `[ ]` | background; unlocked alongside FD |
| Yoshi's Story | `grstory.c` | `[ ]` | Randall (moving platform + puff particle generators, `ef`), Shy Guys (`itheiho`, `it/itzako`), see `docs/M3_PLAN.md` §3; `idle_ys_fox` savestate + traces exist |
| Dream Land | `grpura.c` | `[ ]` | Whispy wind timing, Bronto Burts |
| Fountain of Dreams | `grizumi.c` | `[ ]` | platform height schedule, background |
| Pokemon Stadium | `grpstadium.c` | `[ ]` | transformations, background screen |
| Others (Kongo Jungle, Corneria, Brinstar, ...) | | `[ ]` | per stage |
| Target stages `grt*`, Adventure routes `gr*route` | | `[ ]` | last |

Items (`it/`, 17k core + 57.5k in `it/kinds` across 166 files):
`[ ]` core item system (`item.c`, `itcoll.c`, `ithitbox.c`, `itdrop.c`),
`[ ]` character projectiles needed by M5 movesets (Fox laser, etc), then
`[ ]` common items by family. Split `melee-it` into `it-<family>` crates.

CPU AI (`melee-cpu`): `[ ]` full port including item logic and all
character-specific branches.

## Milestone 7: Replay corpus

Gate: zero divergence over thousands of Slippi replays.

- [x] `slp` crate: parse replays, emit scenarios and expected traces
- [ ] Resolve `rng.seed` timing (Slippi records at frame start; we dump at frame end)
- [ ] Map Slippi ports to fighter-list order when ports have gaps
- [ ] Verify `self_vel`/`kb_vel` field mapping against Slippi's recording code
- [ ] Batch runner: N replays in parallel, aggregate first divergences by function
- [ ] Triage tooling: given a divergence, print the phase, the fighter's action state, and the likely decomp file

## Milestone 8: Platform (out of scope until M7 is green)

- [-] Rendering via wgpu, emulating enough GX for HSD display lists
- [-] Audio
- [-] Menus (`mn/`, 32.8k), HUD (`if/`, 10k), trophies (`ty/`, 12k), cutscenes/video (`vi/`), camera (`cm/`, 4.7k), effects (`ef/`, 4.2k), debug (`db/`)
- [-] Game modes beyond Versus (`gm/`, 54k): Classic, Adventure, All-Star, Event, Target Test, Home Run Contest

## Session log

Newest first. One line per session: date, what landed, what is next.

- 2026-09-09 (dash particle follow-up): added strict full-field replay, production spawn fixture, AppSRT ownership adapter and display-cache API; 425 display-cache mismatches remain because camera/frame inputs were not captured. Need those external display inputs to complete the red gate; no comparisons weakened or commits made.

- 2026-09-09 (M4-T1): ported Squat/Turn/Walk families, shared grounded physics/collision, walk command-loop and footstep/rumble requests; three movement gates 300x49 exact, state scratch/rate/command replay exact, all regressions and clippy pass. No commit requested. Next: review, then dash/run and jump/effect ports.

- 2026-09-09 (night): match-start scenario `start_fd_fox` recorded (tick trace, ledger, particles, tick-aligned + post-render bone dumps); Entry/Fall/Landing states, airborne physics, dynamic-bone solver (Fox tail), effect-id -> spawn mapping, FD animation spawn; bone oracle 73/73 bones; **start_fd_fox gate passed** 600x49 with produced RNG. Lesson: cached JObj matrices are render-time caches, compare them post-render. Next: M4 movement with scripted inputs.
- 2026-09-09 (later): T5 physics/collision, T10 Fighter composition, T13 melee-sim loop. **Milestone 3 gate passed**: 600 ticks x 49 keys, 0 divergences, seed included. Next: match-start scenario (spawn/Fall/landing), then M4 movement.
- 2026-09-09: T7 animation playback and T9 human input both match the FD trace for 600 ticks; hsd-particle matches all 600 dumped frames field-for-field; Docker/IDE memory pressure killed two concurrent Codex runs (resumed; rule: one at a time unless memory is free). Next: T5 grounded physics + collision for a standing fighter, T10 spawn/state machine, T13 melee-sim frame loop, then the M3 gate `idle_fd_fox_600`.
- 2026-09-08 (night): FD unlocked (save unlock mask poke), `idle_fd_fox` savestate + tick-boundary 600-tick trace (deterministic), Dolphin fork patched for PC/LR, RNG ledger attributes all 18,318 idle draws (particle system: 1 + 6 per emission per tick), M3 T4 Fox data readers, melee-gr FD first pass, hsd-particle first pass + live state dump. In flight: Codex replaying live particle state against the ledger. Next: fighter-side M3 tasks (T5 physics, T6/T7 animation playback, T9 input, T10 spawn) once particles match.
- 2026-09-08 (evening): Disc extracted (1,209 files), real .dat tests, retail asm lookup (`harness/asm.py`), fusion audit complete across gekko-math/melee-lb/hsd-anim/melee-mp (MWCC fused most sites; sinf ~18% of inputs differ), JObjDesc->JObjTree glue, coll_data reader + real FD collision test, lbanim FigaTree attach + ftData reader (Fox Wait1 plays), Dolphin oracle booted: `idle_ys_fox` savestate + 600-frame trace at 131 fps. Delegation switched to Codex (`tools/codex-task.sh`). M2 gate tooling built (Codex) and **M2 gate passed**: 0 mismatches on 3,080 bone words vs Dolphin. FD unlocked, tick-boundary sampling fixed, `idle_fd_fox` tick trace recorded (deterministic). Next: M3 tasks from `docs/M3_PLAN.md` on FD (fighter fields first, `rng.seed` after grlast.c).
- 2026-09-08 (late): Disc arrived and verified; main.dol placed for decomp tooling. No disc-dependent work run yet. Session paused by user. Next: the four items under "Milestone 0: disc arrived".

- 2026-09-08: Third batch: frsqrte/fres hardware-exact (M1 complete), JObj scene graph + DObj/MObj, typed descriptor readers + FigaTree, Snapshot/RecordSink/SchemaCoverage, melee-mp mplib+mpcoll. Gate: 55 suites green, clippy clean. Next: JObjDesc→JObjTree glue, melee-gr Final Destination, melee-ft skeleton (fighter.c init/update order), melee-sim frame loop, Dolphin savestate workflow doc.

- 2026-09-08: Second batch: hsd-gobj scheduler (35 tests), hsd-anim mtx/quat + SDK PS kernels and aobj/fobj (all native-C oracle clean), melee-lb lbtrigf + expf/powf, Dolphin scripting fork built on arm64, gekko_probe captured frsqrte/fres behaviour. Gate: 46 suites green. Next: hsd-anim jobj/dobj/mobj, hsd-archive typed node readers, Snapshot trait, melee-mp, user decision on estimate tables.

- 2026-09-08: Parallel batch: hsd-archive parser, type enums, MSL math with native oracle, harness walk + schema generator, slp parser, Dolphin research. Fixed melee-diff float equality. Next: hsd-gobj, hsd-anim start, lbtrigf into melee-lb, Dolphin build.
- 2026-09-08: Scaffolded workspace, gekko-math RNG/FMA, melee-diff, harness skeleton, docs. Decomp pinned as submodule.
- 2026-09-09: scripted-input tracer + per-tick pad replay in melee-sim; five movement scenarios and ledgers recorded; Codex M4-T1 launched (squat/turn/walk).
