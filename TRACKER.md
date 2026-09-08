# Project tracker

Status legend: `[ ]` todo, `[~]` in progress (add date), `[x]` done,
`[!]` blocked (say on what), `[-]` out of scope for now.

Keep this file honest. A task is `[x]` only when its tests pass in
`cargo gate` and it is committed. When you finish a session, update the
task lines you touched and add one line to the session log.

## Current focus

Milestone 2 (HSD engine) and ISO-independent ports, while waiting on the
disc and a Dolphin build. See blockers.

## Blockers

- `[!]` **Disc image.** NTSC-U 1.02 (`GALE01`), owned and self-dumped. Needed
  for: retail asm (FMA audit), `.dat` assets for the simulator, savestates,
  the first real oracle trace. Owner: user. `docs/ISO.md`.
- `[!]` **Dolphin scripting build for macOS arm64.** No binaries exist.
  Recipe in `docs/DOLPHIN.md`. Can be built before the disc arrives.
- `[!]` **`frsqrte`/`fres` hardware tables.** Placeholders return IEEE
  values. Blocks bit-exactness of anything using `sqrtf` (90 call sites in
  game code). See Decisions.

## Decisions

| Date | Decision | Why |
|---|---|---|
| 2026-09-08 | Layered workspace, one crate per subsystem and per character | Build time. 435k lines of game C cannot be one crate. |
| 2026-09-08 | Internal state need not match retail memory layout; `Snapshot` + schema handles comparison | Lets the port use enums and real types. |
| 2026-09-08 | Floats in traces compare by bit pattern only | JSON round trip of the decimal caused false divergences. |
| 2026-09-08 | Oracle reads retail memory via Dolphin scripting; never a modified DOL | Keeps Slippi replays and community tools valid as test inputs. |
| 2026-09-08 | Target Felk's Dolphin scripting fork; GDB stub as fallback | Only option with savestate, frame hook, memory read, input in one process. |
| 2026-09-08 | Slippi fixtures from hohav/peppi (MIT), not slippi-js (LGPL) | License. |
| pending | How to obtain `frsqrte`/`fres` tables | Options: derive from 750CL manual; extract empirically via homebrew DOL in Dolphin (preferred); port Dolphin's under GPLv2. |
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
- [ ] Build Dolphin scripting fork on macOS arm64 (`docs/DOLPHIN.md` recipe)
- [ ] Verify `frameadvance` fires with Null video backend; measure frames/sec
- [ ] Verify savestate load is synchronous with next frame
- [ ] Small C++ patch or debugger workflow to set code breakpoints for intra-frame phases
- [ ] `Snapshot` trait in `melee-types` + test that Rust emitters cover every schema path
- [ ] Golden fixture recorder: break on function entry/exit, dump args and touched memory to `harness/goldens/`
- [ ] Retail asm lookup tool: given a symbol, print its disassembly from `main.dol` (needs disc)
- [ ] CI: `cargo gate`, clippy `-D warnings`, harness pytest, `gen_schema.py --check`

## Milestone 1: Math and RNG (`gekko-math`)

- [x] HSD RNG, bit-exact with tests
- [x] FMA wrappers in PowerPC operand order
- [x] MSL `sinf`, `cosf`, `tanf`, `logf`, `frexp`, `fmodf`, classify, fabs
- [x] Gekko int/float conversion semantics (`fctiwz`, `__cvt_*`)
- [x] Native-C reference oracle test (bit-exact vs decomp C, `-ffp-contract=off`)
- [ ] `frsqrte` / `fres` hardware-exact (blocked on Decisions)
- [ ] FMA audit of the 25 marked sites against retail asm (blocked on disc)
- [ ] Int-conversion audit of the 1 marked site (blocked on disc)
- [ ] Paired-single matrix routines (`PSMTX*`) used by HSD `mtx.c`: decide whether Rust needs to emulate paired-single rounding or whether results are identical to scalar f32

## Milestone 2: HSD engine (`hsd-archive`, `hsd-gobj`, `hsd-anim`)

Gate: load one character and one stage archive, evaluate the wait
animation, match bone matrices from the oracle.

- [x] `hsd-archive`: header, relocs, publics, externs, strings; synthetic tests
- [ ] `hsd-archive`: typed readers for JObj/DObj/MObj/AObj/FObj node graphs (offset-linked structs to owned trees)
- [ ] `hsd-archive`: test against a real `.dat` (blocked on disc)
- [ ] `hsd-gobj`: `gobj.c`, `gobjproc.c`, `gobjplink.c`, `gobjgxlink.c` (link/priority only, no GX), `gobjobject.c`, `gobjuserdata.c` (~800 lines)
- [ ] `hsd-anim`: `mtx.c` (509) and `quatlib.c` (199) via gekko-math
- [ ] `hsd-anim`: `aobj.c` (550), `fobj.c` (496) keyframe evaluation
- [ ] `hsd-anim`: `jobj.c` (1578) hierarchy, local/world matrices, flags
- [ ] `hsd-anim`: `dobj.c` (349), `mobj.c` (591) data only, no render
- [ ] `hsd-anim`: `robj.c` (942) constraints if fighters use them (check)
- [ ] `hsd-anim`: `cobj.c` (1406) camera object, needed by `cm` later
- [ ] HSD `class.c`/`object.c` object model: decide Rust representation (traits vs enums)
- [-] Rendering: `tobj`, `pobj`, `lobj`, `tev`, `texp*`, `psdisp*`, `displayfunc`, `video`, `shadow`, `fog` (Milestone 8)
- [-] Audio: `axdriver`, `synth`, `hsd_3A94` and other sound files (Milestone 8)
- [-] `particle.c`, `generator.c`, `spline.c` effects (needed for `ef` later; visual only)

## Milestone 3: One fighter idle (`melee-lb`, `melee-mp`, `melee-ft`, `ft-fox`)

Gate: `harness/scenarios/idle_fd_fox.toml`, 600 frames bit-exact.

- [ ] `melee-lb`: `lbtrigf.c` (atan2f, atanf, asinf, acosf, lb_sqrtf with lookup table)
- [ ] `melee-lb`: `lbvector.c`, `lbcollision.c`, `lbarchive.c`, `lbfile.c` (headless file access), `lbanim.c`
- [ ] `melee-lb`: remaining `lb_*` files as needed by callers (17k lines total)
- [ ] `melee-mp`: `mplib.c`, `mpcoll.c`, `mpisland.c` (12k lines): floor/wall/ceiling queries, ledge detection
- [ ] `melee-gr`: `ground.c`, `grlib.c`, `grdatfiles.c`, `grlast.c` (Final Destination only for this milestone)
- [ ] `melee-ft`: `fighter.c` init and per-frame update order, `ftcommon.c`, `ftcoll.c`, `ftanim.c`, `ftlib.c`
- [ ] `melee-ft`: `ftCo_*` action states for standing, squat, and turn only
- [ ] `melee-ft`: physics (`ft_08A1.c` etc): gravity, friction, ground snap
- [ ] `ft-fox`: init, attributes, Wait animation; nothing else
- [ ] `melee-sim`: scenario loading, asset loading, frame loop, trace emit
- [ ] Record `idle_fd_fox` savestate and produce `expected.jsonl` (blocked on disc + Dolphin)
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

Stages (`gr/`, 56k lines across 77 files): `[ ]` Final Destination (M3),
`[ ]` Battlefield, `[ ]` Yoshi's Story, `[ ]` Dream Land, `[ ]` Fountain of
Dreams, `[ ]` Pokemon Stadium, then the rest. Target stages (`grt*`) and
Adventure routes (`gr*route`) last.

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

- 2026-09-08: Parallel batch: hsd-archive parser, type enums, MSL math with native oracle, harness walk + schema generator, slp parser, Dolphin research. Fixed melee-diff float equality. Next: hsd-gobj, hsd-anim start, lbtrigf into melee-lb, Dolphin build.
- 2026-09-08: Scaffolded workspace, gekko-math RNG/FMA, melee-diff, harness skeleton, docs. Decomp pinned as submodule.
