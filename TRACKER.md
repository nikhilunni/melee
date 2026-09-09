# Project tracker

Status legend: `[ ]` todo, `[~]` in progress (add date), `[x]` done,
`[!]` blocked (say on what), `[-]` out of scope for now.

Keep this file honest. A task is `[x]` only when its tests pass in
`cargo gate` and it is committed. When you finish a session, update the
task lines you touched and add one line to the session log.

## Current focus

**The disc is present, extracted, and the oracle works.** `harness/roms/idle_ys_fox.sav` + `harness/traces/idle_ys_fox.expected.jsonl` are the first real-game trace (two idle Foxes, Yoshi's Story, 600 frames). Fusion audit complete workspace-wide. Fox's Wait1 animation plays through the port. **Milestone 2 gate passed.** Next: Milestone 3 groundwork (melee-gr Yoshi's Story, melee-ft init and frame order, melee-sim loop).

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
- [-] `particle.c`, `generator.c`, `spline.c` effects (needed for `ef` later; visual only)

## Milestone 3: One fighter idle (`melee-lb`, `melee-mp`, `melee-ft`, `ft-fox`)

Gate: `harness/scenarios/idle_fd_fox.toml`, 600 frames bit-exact.

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
- [~] (2026-09-08) `melee-gr`: `grlast.c` Final Destination decorations, first pass committed (procs, direct RNG sites, desc readers). **Finding:** the direct grlast.c logic draws on only 2 of 600 idle ticks; the 6k+1 draws per tick come from elsewhere (likely the HSD particle generator via a particle-spawn animation track on map 4). Next: RNG ledger from Dolphin (PC/LR at each `seed` write) to attribute every draw before porting more.
- [ ] `melee-gr`: Dolphin capture of the FD `Ground` struct at the `idle_fd_fox` savestate (timers/phases) to seed the decoration state; then a 600-tick RNG-draw-count test against `idle_fd_fox.tick.expected.jsonl`
- [ ] `melee-ft`: `fighter.c` init and per-frame update order, `ftcommon.c`, `ftcoll.c`, `ftanim.c`, `ftlib.c`
- [ ] `melee-ft`: `ftCo_*` action states for standing, squat, and turn only
- [ ] `melee-ft`: physics (`ft_08A1.c` etc): gravity, friction, ground snap
- [x] `ft-fox`: attributes (Fox special block, 0xD4 bytes) read from PlFx.dat (Codex T4); init and Wait wiring still to do
- [x] `melee-ft`: `desc/` reads ftData attributes (0x184 bytes, grouped by concept), part table (54 entries, TopN/TransN/XRotN/YRotN=0/1/2/3), ECB joints [41,55,25,13,7,4], bone lists, PlCo common subset (`docs/FOX_DATA.md`)
- [ ] `melee-sim`: scenario loading, asset loading, frame loop, trace emit
- [x] Record savestate and `expected.jsonl`: **`idle_ys_fox`** (Yoshi's Story) instead of FD, two idle Foxes, 600 frames, byte-identical on rerun (`docs/DOLPHIN_RUN.md`)
- [x] `idle_fd_fox` recorded (2026-09-08): FD unlocked via `poke-or 0x8045BF2A u16 0xC0` (save data `gmMainLib_804D3EE0->thing.x186A`), Stock 1 via `GameRules` bytes, items were already NONE. Savestate `harness/roms/idle_fd_fox.sav` at frame 50841, both Foxes in Wait at (+-60, 0.0001, 0). Tick trace `harness/traces/idle_fd_fox.tick.expected.jsonl`, 600 ticks, deterministic.
- [ ] `pl/player.c` and `gm` match setup: only what spawning one fighter needs

## Milestone 4: Movement (`melee-ft`)

Gate: scripted input scenarios for walk, dash, run, jump, double jump,
fast fall, ledge grab, ledge options, platform drop, wavedash.

- [ ] Input system: `fighter.c` input parsing, buffers, deadzones (`ftCo_800A2040` CPU vs pad path)
- [ ] `ftCo_*` ground movement states (walk, dash, run, turn, skid)
- [ ] Jumps, air movement, fast fall, landing, L-cancel
- [ ] Ledge states (`ftcliffcommon.c` and `ftCo_Cliff*`)
- [ ] Platform pass-through and drop
- [ ] Shield, roll, spot dodge, air dodge (no hit interaction yet)
- [ ] Scenarios for each, recorded from Dolphin
- [ ] Battlefield and Yoshi's Story stages (platforms)

## Milestone 5: Combat (`melee-ft`, `melee-lb`)

Gate: two-fighter scenarios with hits, shields, grabs, KOs.

- [ ] Hitbox/hurtbox system (`ftcoll.c`, `lbcollision.c`, `ftcolanim.c`)
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
| Fox | `ft-fox` | 3829 | `[ ]` (M5) |
| Marth | `ft-mars` | 2184 | `[ ]` (M5) |
| Mario | `ft-mario` | 1257 | `[ ]` |
| Dr. Mario | `ft-drmario` | 399 | `[ ]` |
| Luigi | `ft-luigi` | 1922 | `[ ]` |
| Captain Falcon | `ft-captain` | 1705 | `[ ]` |
| Falco | `ft-falco` | 503 | `[ ]` |
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

- 2026-09-08 (evening): Disc extracted (1,209 files), real .dat tests, retail asm lookup (`harness/asm.py`), fusion audit complete across gekko-math/melee-lb/hsd-anim/melee-mp (MWCC fused most sites; sinf ~18% of inputs differ), JObjDesc->JObjTree glue, coll_data reader + real FD collision test, lbanim FigaTree attach + ftData reader (Fox Wait1 plays), Dolphin oracle booted: `idle_ys_fox` savestate + 600-frame trace at 131 fps. Delegation switched to Codex (`tools/codex-task.sh`). M2 gate tooling built (Codex) and **M2 gate passed**: 0 mismatches on 3,080 bone words vs Dolphin. FD unlocked, tick-boundary sampling fixed, `idle_fd_fox` tick trace recorded (deterministic). Next: M3 tasks from `docs/M3_PLAN.md` on FD (fighter fields first, `rng.seed` after grlast.c).
- 2026-09-08 (late): Disc arrived and verified; main.dol placed for decomp tooling. No disc-dependent work run yet. Session paused by user. Next: the four items under "Milestone 0: disc arrived".

- 2026-09-08: Third batch: frsqrte/fres hardware-exact (M1 complete), JObj scene graph + DObj/MObj, typed descriptor readers + FigaTree, Snapshot/RecordSink/SchemaCoverage, melee-mp mplib+mpcoll. Gate: 55 suites green, clippy clean. Next: JObjDesc→JObjTree glue, melee-gr Final Destination, melee-ft skeleton (fighter.c init/update order), melee-sim frame loop, Dolphin savestate workflow doc.

- 2026-09-08: Second batch: hsd-gobj scheduler (35 tests), hsd-anim mtx/quat + SDK PS kernels and aobj/fobj (all native-C oracle clean), melee-lb lbtrigf + expf/powf, Dolphin scripting fork built on arm64, gekko_probe captured frsqrte/fres behaviour. Gate: 46 suites green. Next: hsd-anim jobj/dobj/mobj, hsd-archive typed node readers, Snapshot trait, melee-mp, user decision on estimate tables.

- 2026-09-08: Parallel batch: hsd-archive parser, type enums, MSL math with native oracle, harness walk + schema generator, slp parser, Dolphin research. Fixed melee-diff float equality. Next: hsd-gobj, hsd-anim start, lbtrigf into melee-lb, Dolphin build.
- 2026-09-08: Scaffolded workspace, gekko-math RNG/FMA, melee-diff, harness skeleton, docs. Decomp pinned as submodule.
