# Function-level parity inventory

How far the port is from all of retail Melee, measured per retail function.
Measured 2026-10-24 on main at `969a065e`. The objective here is breadth: every
function retail runs, not only what the gate exercises.

```sh
cargo run -q --release -p melee-lib --example motion_rows > target/parity/motion_rows.tsv
cd harness && uv run python parity_inventory.py --motion-rows ../target/parity/motion_rows.tsv --markdown
```

From a worktree, set `MELEE_DATA_ROOT` to the main checkout (the decomp
submodule is read from there). Outputs, keyed by retail address so other
tools can join on it:

- `target/parity/inventory.tsv`: one row per retail function (address, size,
  name, source file, subsystem, area, scope, status, and the Rust locations
  behind the status).
- `target/parity/fail_closed.tsv`: one row per fail-closed site (retail
  address and function, subsystem, Rust location, macro, C line, how it was
  resolved, the enclosing Rust fn, the message and the comment above it).

## Method

**Retail side.** Every `type:function` symbol in the `.init` and `.text`
sections of `config/GALE01/symbols.txt`: 19,827 functions, 3,783 KiB of code.
`splits.txt` attributes each to its source file; the directory is its
subsystem. Sizes come from the symbol table.

**Port side.** Every `.rs` file under `crates/`, split by a small lexer into
code, comments and string literals. A token in a comment or string is a
citation when it is a function's exact name, a name ending in an address
(`ftCo_800B63D8`, resolved by address, so renamed symbols still match), or a
bare address (`0x8006B0E0`, `800D8EC8`). An address inside a function's range
cites that function. Rust `fn` names equal to a symbol (`sinf`, `atan2f`) also
count.

**Statuses**, first match wins:

| status | rule |
|---|---|
| ported | Named before the first `: ` of the first comment line above a `fn`, `const`, `static`, `impl` or `trait` (the `sym (addr): what it does` convention), or the first line of a module's `//!` doc. A possessive (`sym's`) does not count. Tooling crates (`melee-sim`, `melee-replay`, `slp`, ...) and tests never port. |
| partial | Ported (or `body`/`row`), and a fail-closed site sits in the citing item's body or names the function (its symbol, or a `file.c:line` inside its C body). |
| body | Cited anywhere else inside a fn/const/static item: its doc block below the first line, or a comment or string in its body. Usually a callee the port inlines. |
| row | A callback of a motion-state row the port implements (`MotionRow::implemented`, dumped by the `motion_rows` example and matched against the decomp's `MotionStateTable` arrays), or of an item state row (`ItemStateTable`) with another callback ported. |
| referenced | Cited only outside items: a field, enum or module comment, or tooling. Also a function only named by a fail-closed message. |
| not started | Never cited. |

`ported + partial` is the conservative measure; `+ body + row` is the
estimate. Byte totals weight each function by its retail code size. That is a
proxy: a 4-byte empty callback and a 4 KiB physics routine differ in size far
more than in porting effort, and a large function ported for one branch counts
whole.

**Measurement groups.** Every function is kept in the totals. The objective
is 100% retail parity with no scope exclusions (user, 2026-10-24). The
inventory retains its historical `scope` tags so measurements remain
comparable; these are reporting groups, not permission to omit work. The
groups outside the historical in-game subtotal are menus (`mn`, menu scenes
in `gm`), results (`gmresult`,
`gmreg*`, `pl/plbonus*`), single-player and modes (Classic, Adventure, events,
Home-Run, Targets, trophy and route stages, Sandbag), single-player fighters
(Master Hand, Crazy Hand, wireframes, Giga Bowser), trophies (`ty`),
cutscenes (`vi`), debug (`db`, HSD debug), pause (`gmpause`), memory card,
audio (`lbaudio_ax`, `sfx`, AX/DSP, synth), rendering (GX, HSD display
objects, text, shadows; the port's renderer in `melee-platform` still counts
when it cites them) and OS, hardware and loading (Dolphin SDK, MetroTRK,
runtime, MSL outside the math library, archive and DVD loading). The rules
are regexes on the source path in `parity_inventory.py` (`_OUT_OF_SCOPE`).
Unnamed `gm_*` and `hsd_*` files are assigned by neighbours where the decomp
does not name them; `gm_1601.c` (match flow), `gm_16AE.c`, `gmmain_lib.c`
(save data and rules) and the unnamed `hsd_3A94`/`hsd_3B*` files stay in game
logic.

**Fail-closed sites.** Every `unimplemented!` and `todo!` in non-test,
non-tooling code (300), every `bail!` in a simulation crate or `melee-lib`
(20; `bail!` elsewhere is input validation) and every
`Error::UnsupportedFeature` in `hsd-particle` (11): 331 sites. `unreachable!`
is an invariant, not a gap, and is left out. Each resolves to a retail
function by, in order: a symbol or address in its message (238), a
`file.c:line` in its message or the comment above it, mapped to the C
function whose body holds that line (included in the 238), or the comment
block of the enclosing Rust fn (32). 61 do not resolve: generic dispatch arms
(`item control for this kind`) and the boundary importer.

## Accuracy

**Citations.** 22,671 function citations; 96.7% resolve to a function in the
symbol table. The rest are 480 decomp `static inline` helpers (no retail
function of their own, e.g. `ftCommon_GroundToAirStateChange`) and 268 stale
or invented names: `Fighter_ProcessHit` (47), `Fighter_UnkProcessDeath`
(11), `Fighter_UnkInitReset` (9), `Fighter_UnkProcessGrab` (4) are older
decomp names since renamed; others are state names written as symbols
(`ftNs_SpecialAirHi`, `ftCo_Cliff`), struct fields (`unknown_float`) and one
run-together pair. Not counted as function citations: 394 data symbols and
2,458 decomp identifiers that are types, fields, enums or macros.

**Spot check.** Three calibration rounds (64 random functions) found the
systematic errors the `body` and `row` statuses and the module-doc rule now
fix: state callbacks and one-call wrappers the port implements through a row
without naming them, and callees named only inside the caller's body (with
only `ported` and `referenced`, 9 of 12 sampled `referenced` functions were
in fact ported inline). A final round of 30 random in-game
functions (6 ported, 3 partial, 5 body, 4 row, 4 referenced, 8 not started),
each checked against the decomp and the port, found 4 wrong (13%):

- `grCorneria_801E2E50` is `partial` but not ported: a field comment counted
  as `body`, and fail-closed messages name it.
- `it_802BE5D8` is `body` but not ported: the comment says its material
  animation only draws.
- `Stage_GetCamBoundsTopOffset` and `grLast_8021A9A4` are `referenced` but
  effectively ported (a 24-byte getter the port samples, an empty callback).

All 18 in the ported, row and not-started buckets were right. Errors sit in
the soft buckets (`body`, `referenced`), which is why the tables report both
measures. Known undercount: of 30 sampled `not started` functions across all
rounds, four were in fact ported. Two are now `row` (an item state's empty
callback, a capture state's IASA wrapper); two remain, uncited helpers
outside state tables (`ftCo_8009856C`, a DownAttack entry helper; Jigglypuff's
hat getter). Families whose crates cite with wildcards (`ftMs_SpecialS*_Coll`)
undercount most.

## Totals

All functions (19,827; 3,783 KiB): 3,864 ported, 246 partial, 1,207 body,
1,154 row, 235 referenced, 13,121 not started. By bytes, **27.6% ported or
partial, 34.8% with body and row**.

In-game logic only (14,399 functions; 2,432 KiB): **41.8% ported or partial,
52.2% with body and row**. The excluded families (5,428 functions, 1,351 KiB)
are 2.0% done (3.6% with body and row), mostly rendering ported for the app.

| area (in-game) | functions | ported | partial | body | row | referenced | not started | KiB | ported+partial | with body+row |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| it items (kinds) | 3802 | 711 | 35 | 121 | 99 | 32 | 2804 | 473 | 36.9% | 41.8% |
| ft characters | 3672 | 1613 | 45 | 348 | 714 | 64 | 888 | 445 | 58.8% | 77.8% |
| gr stages | 1760 | 101 | 3 | 62 | 0 | 26 | 1568 | 340 | 12.0% | 14.0% |
| ft common states | 1471 | 482 | 92 | 181 | 320 | 8 | 388 | 248 | 57.9% | 72.8% |
| ft engine | 1108 | 302 | 41 | 196 | 21 | 45 | 503 | 237 | 50.2% | 64.1% |
| HSD (sysdolphin) | 499 | 136 | 4 | 28 | 0 | 16 | 315 | 180 | 34.6% | 45.2% |
| mp collision | 229 | 212 | 1 | 2 | 0 | 2 | 12 | 102 | 92.5% | 92.9% |
| gm match flow | 617 | 12 | 1 | 18 | 0 | 4 | 582 | 98 | 4.9% | 10.4% |
| it engine | 475 | 113 | 8 | 107 | 0 | 10 | 237 | 88 | 41.0% | 56.5% |
| lb | 211 | 60 | 4 | 33 | 0 | 1 | 113 | 58 | 47.2% | 59.9% |
| ef effects | 65 | 14 | 6 | 30 | 0 | 0 | 15 | 49 | 73.2% | 91.3% |
| if HUD | 163 | 5 | 0 | 6 | 0 | 4 | 148 | 49 | 5.6% | 20.6% |
| cm camera | 127 | 20 | 1 | 13 | 0 | 4 | 89 | 35 | 30.0% | 37.3% |
| pl player | 191 | 9 | 1 | 17 | 0 | 6 | 158 | 29 | 8.5% | 16.6% |
| MSL math | 9 | 4 | 0 | 5 | 0 | 0 | 0 | 1 | 82.0% | 100.0% |
| **total** | 14399 | 3794 | 242 | 1167 | 1154 | 222 | 7820 | 2432 | 41.8% | 52.2% |

| family outside historical in-game subtotal | functions | KiB | ported+partial | with body+row |
|---|---:|---:|---:|---:|
| single-player and modes | 1520 | 278 | 0.0% | 0.0% |
| menus | 530 | 260 | 0.1% | 4.8% |
| rendering | 795 | 206 | 11.4% | 13.6% |
| OS, hardware, loading | 746 | 165 | 2.0% | 3.7% |
| results | 352 | 102 | 0.0% | 2.0% |
| trophies | 151 | 97 | 0.0% | 0.0% |
| audio | 336 | 84 | 0.2% | 0.5% |
| single-player fighters | 557 | 51 | 0.3% | 0.3% |
| memory card | 168 | 44 | 0.0% | 0.0% |
| debug | 153 | 37 | 0.0% | 0.4% |
| cutscenes | 84 | 20 | 0.0% | 0.0% |
| pause | 36 | 7 | 0.0% | 0.0% |

Characters, by the estimate (ported, partial, body, row): Nana, Mewtwo,
Jigglypuff, Mario, Bowser, Captain Falcon, Sheik, Samus, Link and Zelda
93-99%; Ice Climbers, Pikachu, Ness, Peach, Luigi, Game & Watch and Yoshi
90-93%; Fox (with the Falco code it shares) 81%; Donkey Kong 75%; Marth (with
Roy's shared code) 72%; Kirby 47% (the copy abilities, 79 KiB, are 38%).
Fox, Donkey Kong and Marth are fully playable and exact on their witnesses, so
their lower numbers are mostly uncited helpers (`ftMs_SpecialS_801376E8`
and its siblings), the undercount the spot check describes. The clone files
(Falco, Ganondorf, Roy, Pichu, Dr. Mario, Young Link: 32-59%) are small and
hold their own load and attribute hooks.

## Biggest not-started areas

1. **Stages other than the six tournament ones**: 2,190 functions, 342 KiB at
   2% (Big Blue 41 KiB, Corneria 25, Brinstar 18, Icicle Mountain 16, Peach's
   Castle 15, Green Greens 15, Venom 14, Kongo Jungle 13, Mute City 12,
   Rainbow Cruise 11, Great Bay 10, Onett 8, ...). The six
   tournament stages are 68% (72%); the shared stage code (`ground.c`,
   `grlib.c`, `granime.c`, ...) 22% (29%).
2. **Item kinds never touched**: 100 of 166 item source files, 2,611
   functions, 255 KiB at 0%: every random item (shells, Bob-omb launcher
   aside, Hammer, Star Rod, Ray Gun, Super Scope, Fire Flower, Mushrooms,
   Poké Ball and its 30-odd Pokémon, Party Ball, barrels and crates), stage
   enemies and the adventure enemies (ReDead, Like Like). The 66 item files
   the port has started are 80% (91%).
3. **Match flow (`gm`, `pl`, `if`)**: 971 functions, 176 KiB at 5-9%. The port
   runs its own match shell; retail's `gm_1601.c` (36 KiB: time, stock and
   KO bookkeeping, sudden death), `gm_16F1`/`gm_16A2`/`gm_16AE`, `player.c`
   (23 KiB) and the HUD (`ifstock`, `ifstatus`, `iftime`, the magnifier) are
   mostly not ported function by function; what the gate needs is
   reproduced inside `melee-lib`.
4. **HSD**: 315 of 499 in-game functions not started, notably `cobj.c`
   (camera objects, 78% not started), `robj.c`, the unnamed `hsd_3A94.c`
   (37 KiB) and `hsd_3B34.c`/`hsd_3B5C.c`.
5. **Kirby's copy abilities**: 579 functions, 79 KiB at 9% (38%): 17 of the
   25 copied neutral specials, 94 of Kirby's 102 unimplemented rows.
6. **Item use by fighters**: 112 of 341 common motion rows are
   unimplemented, nearly all item states: heavy items (HeavyGet, HeavyThrow
   x8), five swing items x4 rows, Ray Gun, Fire Flower, Super Scope (16
   rows), Screw Attack, Parasol item, Hammer (7), Mushrooms (8), Warp Star,
   barrels, being carried (Lift), and captures by enemies and bosses.
7. **CPU players**: `ftCo_0A01.c`, `ftcpuattack.c` and `ftcmdscript.c` are
   111 KiB at 56%; what exists serves Nana. 44 fail-closed sites.
8. **Camera** (`camera.c`, 35 KiB): 60% not started (single-player framing,
   pause and debug cameras, quake helpers).

## Fail-closed backlog

331 sites, clustered into work items a single worktree agent can take. Each
row names its sites' files; `fail_closed.tsv` has every line, message and
retail function. The 258 unimplemented motion rows fail closed through one
site (`state/row.rs:110`) and are counted as "+N rows" in the items they
belong to: 132 item and status rows (19 + 30 + 22 + 4 + 57; the common
table's 112 plus Donkey Kong's heavy carry and Falcon's and Ganondorf's
swing rows), 102 of Kirby's (94 + 6 + 2) and 24 other character rows.

| # | work item | sites | where | retail |
|---|---|---:|---|---|
| 1 | CPU attack choice: ranged, edge guard, roll-in, item swing/shoot, per-kind exclusions, motion ranges, special follow-ups | 9 | `melee-cpu/attack.rs`, `arrived.rs` | `ftCo_800B9CBC`, `800B732C`, `800BA2E8`, `800A5A90`, `800B77E8`, `800B630C`, `800B658C`, `800ACD5C` |
| 2 | CPU stage awareness and routes for the other stages (Corneria ceilings, Temple, Icicle Mountain, Rainbow Cruise, Fourside, Mushroom Kingdom, Big Blue, Kongo barrels, stage drift) | 13 | `melee-cpu/awareness.rs`, `route.rs`, `island_route.rs`, `movement.rs`, `recover.rs`, `select.rs`, `world.rs`, `behave.rs:112`, `attack.rs:175` | `ftCo_800A1CC4`, `800A2718`, `800AAF48`, `800A9CB4`, `800A2C80`, `800A1B38`, `grLib_801C9E60` |
| 3 | CPU modes and behaviours beyond Nana's (mode dispatch, behaviour table, charge release, Zelda transform, level wait, approach, defence, item pickup) | 13 | `melee-cpu/behave.rs`, `lib.rs`, `select.rs`, `hold.rs`, `attack.rs:109,136` | `ftCo_800B2AFC`, `800ABA34`, `800B2790`, `800ADC28`, `800A08F0`, `800AB224`, `800BB9B4`, `800A3710`, `800B683C` |
| 4 | CPU recovery specials, scaled Yoshi branches, partner-gone and tumble register cases | 9 | `melee-cpu/movement.rs`, `recover.rs`, `partner.rs`, `tumble.rs` | `ftCo_800A96B8`, `800A0508`, `800A3234`, `800AEFB8`, `800AC5A0` |
| 5 | Kirby's copied neutral specials, one family per agent: Mario/Dr. Mario/Luigi, Link/Young Link, Samus, Ness, Mewtwo, Yoshi, Pikachu/Pichu, Peach, Bowser/Giga Bowser, DK, Zelda/Sheik, Ice Climbers | 3 (+94 rows) | `ft-kirby/copied/mod.rs`, `init.rs:148` | `ftKb_SpecialN_Enter`, `ftKb_Init_803C9DD0`, `fn_80105978`, `ftCo_800BDA50` |
| 6 | Kirby's Inhale edges: an item mouthful (SpecialNCapture1/Drink1/Spit1 rows), a pull with no victim, mouthful walk/jump edges | 6 (+6 rows) | `ft-kirby/mouthful.rs` | `ftKb_SpecialN_800F5EA8`, `ftKb_EatWait_IASA`, `ftWalkCommon_800DFDDC`, `fn_800F697C`, `fn_800F6AC8` |
| 7 | Kirby's Final Cutter and Stone edges, the Cutter beam's clank/absorb/shield bounce and floor/wall cases, the ability star against walls; the SpecialHi2/3 rows | 13 (+2 rows) | `ft-kirby/special_hi.rs`, `special_lw.rs`, `it-kirby/` | `ftKb_SpecialHi_800F3570`, `800F37EC`, `800F3B28`, `ftKb_SpecialLwEnd_*`, `itKirbyCutterBeam_*`, `it_8026DFB0` |
| 8 | Fighters out of Kirby's mouth: star landing, ceiling, slow star, hits after a star, pairs parted mid-Inhale, a victim holding a heavy item or another fighter | 10 | `melee-ft/capture_kirby.rs` | `ftCo_800BDA74`, `800BE494`, `800BE7C0`, `800DC920`, `800DDDE4`, `ftCo_CaptureCut_Enter` |
| 9 | Model scale other than 1 (Mushrooms, Giga Bowser, metal): scaled knockback, hurt matrices, attribute modifiers, Bowser's flame, grapple, blaster, Kirby's mouthful take-off | 16 | `ftColl_8007ABD0` in nine special crates, `damage.rs:3147`, `caches.rs:226`, `spawn.rs:368` | `ftCo_CalcYScaledKnockback`, `lbColl_80008248`, `ftCo_800D105C` |
| 10 | Heavy items: HeavyGet/HeavyThrow rows, DK's heavy carry rows, every "with a heavy item" status entry | 8 (+19 rows) | `bury.rs`, `capture_*.rs`, `damage_bind.rs`, `damage_song.rs`, `frozen.rs`, `item_pickup.rs:605` | `ftCo_8009750C`, `ftpickupitem_80094694` |
| 11 | Swing items: Bat, Parasol, Fan, Star Rod, Lipstick (20 common rows, Falcon/Ganon swing4) and smash/tilt/C-stick item attacks | 8 (+30 rows) | `attack.rs`, `item_swing.rs` | `fn_800CCEC4`, `ftCo_Attack_800CCF58`, `checkItemThrow`, `ftCo_800CED30` |
| 12 | Shooting items: Ray Gun, Fire Flower, Super Scope (22 rows), Peach's float aerial with a gun | 1 (+22 rows) | `ft-peach/float_attack.rs` | `ftCo_800CDDA0` |
| 13 | The Parasol item and Peach's parasol table and opening | 10 (+4 rows) | `melee-ft/parasol.rs`, `character.rs:543`, `ft-peach/articles.rs`, `special_hi.rs:166` | `ftCommon_8007E83C`, `ftCo_800CEFE0`, `800CF3C8`, `800CF280`, `80095744` |
| 14 | Other held-item paths: squat, light-item pickup in the air, transform with an item, Samus, Game & Watch Manhole/Greenhouse with an item, hammer thaw | 8 | `squat.rs`, `item_pickup.rs`, `transform.rs`, `frozen.rs:336`, `ft-samus/special_lw.rs`, `ft-gamewatch/` | `ftCo_800D638C`, `ftpickupitem_80094D90`, `ftCommon_8007EFC8`, `ft_800895E0` |
| 15 | The other common rows: Screw Attack (4), Hammer (7), Mushrooms (8), Warp Star (2), barrels (2), Lift (4), captures by Bowser, Master and Crazy Hand, ReDead, Like Like and Kirby-as-Yoshi (17), DeadUp variants (4), directional Fall rows (6), RunDirect, DownSpot (2); one agent per family, rows first, then the item kinds | 0 (+57 rows) | `state/row.rs:110` | `ftCo_ItemScrew*`, `ftCo_Hammer*`, `ftCo_Kinoko*`, `ftCo_WarpStar*`, `ftCo_Capture*` |
| 16 | Peach's rarer pulls: Bob-omb walking/turning/sliding and Mr. Saturn's unheld states; the pull table's other kinds | 14 | `it-bombhei`, `it-dosei`, `ft-peach/special_lw.rs:95` | `it_3F14_Logic6_EnteredAir`, `fn_8027FCA8`, `it_8027F8E0`, `itDosei_Logic7_EnteredAir`, `it_802BD4AC` |
| 17 | Article control messages: owner hitlag for Toad/Parasol, arrow/boomerang/bow/hookshot/milk, Disable, Shadow Ball, Thunder, charge shot, grapple, egg; shield bounces | 22 | `it-link`, `it-mewtwo`, `it-pikachu`, `it-samus`, `it-peach`, `it-yoshi*`, `it-drmariopill`, `melee-it/logic.rs` | `it_802A8398`, `it_802BDFA0`, `it_802B1DF8`, `itColl_BounceOffShield`, ... |
| 18 | Item engine: the item script opcodes, item commands, collision categories, wall pinch, floor launch, item-vs-item hitboxes, item effects, a full item table | 12 | `melee-it/desc.rs`, `engine.rs`, `hurt.rs`, `logic.rs`, `map.rs`, `melee-lib/frame.rs` | `it_802799E4`, `it_80275E98`, `it_80276D9C`, `it_8027B798`, `it_802706D0`, `it_80278800` |
| 19 | Grab, throw and capture edges: Egg Lay catch hits, cape on a frozen fighter, mouth-hold lift, hookshot reel, throw landings, kept grab timers | 14 | `grab.rs`, `grab_damage.rs`, `grab_escape.rs`, `throw_flight.rs`, `damage.rs:1050,1125`, `capture_yoshi.rs`, `cargo.rs`, `life.rs:810` | `ftCo_8008EC90`, `fn_800DD568`, `fn_800DD5EC`, `ftCo_800DE3FC`, `fn_800DB5D8` |
| 20 | Grab pairs inside status effects (bound, asleep, Confusion while holding) | 3 | `damage_bind.rs`, `damage_song.rs`, `capture_mewtwo.rs` | `ftCo_800DD168` |
| 21 | Common-state edges: relaxed/C-stick jump entry, Fall and Damage IASA arms, special fall with jumps left, DownReflect, tether hang, bury on a moving floor, freeze by throw, frozen top KO, phantom vs shell, revival platform, walk tier, dash throw off a ledge | 15 | `melee-ft/fighter/` (`jump.rs`, `state/callbacks/`, `damage*.rs`, `down.rs`, `air_*.rs`, `bury.rs`, `life.rs:352`, `hit_log.rs`, `walk.rs`) | `fn_800CAF78`, `ftCo_Fall_IASA_Inner`, `ftCo_Damage_IASA`, `ftCo_800C7CA0`, `ftColl_8007BE3C`, ... |
| 22 | Fighter proc branches and motion-change flags: held-item lifetime, capture/grab, death/entry, status effects, accessory callbacks, stage collision lists, overlap nudge, coin-match attachment | 11 | `fighter/mod.rs:1004-1019`, `spawn.rs`, `state/row.rs` | `Fighter_8006A360`, `Fighter_Create`, `ftCommon_8007E0E4`, `Fighter_ChangeMotionState` |
| 23 | Character hook defaults no registered character reaches yet (taunt entry, throw callback, jab/smash entry, landing reset, multijump table, article requests, special grab callbacks) and Samus's morph-ball roll; audit which are now unreachable and document them | 16 | `fighter/character.rs`, `fighter/mod.rs:217-660`, `state/special.rs` | `ftCo_800DEA28`, `Fighter_UnkProcessGrab_8006CA5C`, `ftCo_800992A8` |
| 24 | Per-character special edges: DK hitboxes, Judge with no face, Nana's flags, the Klaw losing its floor, Link aerial re-enable and arrow failure, Ness yo-yo reel-in, Peach SpecialHiEnd/SSJump rows and doAirEnd0, Jigglypuff's Stadium sleep and Rollout capsule, cape turnarounds of rolling specials; plus the 24 character rows still unported (Link AppealS, Luigi SpecialS2, Ness Lw turn, Peach, Pikachu and Pichu SpecialS1, Yoshi egg variants, Ice Climbers Belay variants, DK ThrowFWait2 and SpecialLwEnd1, two of Mario's) | 18 (+24 rows) | `ft-*` crates | `ftColl_8007B8A8`, `ftGw_SpecialS_GetRandomInt`, `ftNn_Init_*`, `ftKp_SpecialS_801332C4`, `lwOnAnim`, `itNessyoyo_UnkMotion3_Phys`, `ftPr_SpecialN_8014222C`, `fn_8013295C` |
| 25 | Scratch words carried across motion changes (`mv+4`, `turnFrames`, landing inheritance) | 11 | `ft-*/lib.rs`, `init.rs`, `special_*.rs`, `landing.rs` | `ftCo_Landing_Enter`, `ftPk_SpecialLw_SetState_Unk0` |
| 26 | Boundary import: saved article GObjs in FighterVars, charged specials in a savestate, scheduler/camera/emitter states, cold setup for other stages | 22 | `ft-*/init.rs`, `melee-lib/initial_state/`, `setup.rs` | port tooling, no single retail function |
| 27 | Particles: AppSRT, tornado, rectangle emission, camera-facing generators, A9 opcode, point joints, child banks | 11 | `hsd-particle/` | `psDispParticles`, `hsd_8039DAD4`, `hsd_8039930C`, `calcTornadoLastPos` |
| 28 | Effects: unsupported dust and effect ids, item-following generators, ef animation events, footstep effects issued before a motion change, the graphics dispatch | 14 | `melee-ef/`, `fighter/effects.rs`, `procs.rs:467` | `efAsync_Dispatch`, `efSync_Spawn`, `efAsync_QueueProcessDeferred`, `ftCo_8009F834` |
| 29 | Subaction opcodes, sound behaviours, capsule states and colour animations | 7 | `melee-cmd`, `melee-coll`, `fighter/commands.rs`, `caches.rs`, `color_overlay.rs` | `ftAction_80073240`, `ftAction_80071B50`, `ftColl_80078C70`, `ft_8007C224` |
| 30 | Bone and part trees: instanced parents, insertion types, part order | 5 | `melee-ft/desc/bones.rs` | `ftParts_80075304`, `ftParts_800753D4` |
| 31 | Stage and camera leftovers: Battlefield light kinds, animated stage lights, stage spline animation, Battlefield demo background, zoomed single-player camera | 5 | `melee-gr/`, `scene_stage/battle.rs`, `melee-cm/standard.rs` | `LObjLoad`, `Ground_801C466C`, `JObjSortAnim`, `grBattle_OnLoad`, `Camera_8002B1F8` |
| 32 | Match flow: final stock and elimination, a second fighter's revival at its own spawn, more than nine camera subjects | 4 | `fighter/life.rs`, `frame/partner_fighters.rs`, `frame.rs:3424` | `gm_80167320`, `Player_80032070` |

The first 32 rows hold all 331 sites. Rows 2, 15 and the CPU and stage
items in 1-4 only become reachable once the stages and items they depend on
exist; closing the not-started stages and items (the first two areas above)
is the larger part of the remaining work and is not in this list, which
covers only code the port already knows it lacks.
