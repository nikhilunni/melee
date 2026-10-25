# Function-level parity inventory

How far the port is from all of retail Melee, measured per retail function.
Measured 2026-10-24 after the two wave-16 breadth picks were integrated
(production source `93683489`). Evidence is in
`target/parity/wave16-integrated/`. The objective here is breadth: every
function retail runs, not only what the gate exercises. The current full
release checkpoint passes; runtime gates and performance budget results
are tracked separately in `TRACKER.md`.

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

**Fail-closed sites.** The current inventory has **269 sites**: 250
`unimplemented!` and 19 `bail!`. The tool also detects `todo!` in non-test,
non-tooling code and `Error::UnsupportedFeature` in `hsd-particle`; neither
adds a current site. `bail!` is counted in simulation crates and `melee-lib`,
while other crates' input validation is excluded. `unreachable!` is an
invariant, not a gap, and is left out. A site resolves by a symbol/address
or C line in its message/comment, then its enclosing Rust function's doc.
Current resolution: 211 by message/C line, 13 by enclosing context, 45
unresolved dispatch/import sites. New explicit guards can increase this
count even while more retail behavior is ported.

## Accuracy

**Citations.** 24,116 of 24,970 function-citation candidates resolve to
retail functions (96.6%); 546 name decomp inline helpers without their own
retail symbol, and 308 names remain unresolved. Stale names include
`Fighter_ProcessHit`, `Fighter_UnkProcessDeath` and `Fighter_UnkInitReset`;
other unresolved tokens are state names or fields rather than functions.
Not counted as function citations: 475 data references and 2,594 other
decomp identifiers. Counts come from the current generated summary, not
the earlier calibration sample below.

**Historical spot check.** Three calibration rounds (64 random functions) found the
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

## Current totals

All functions (19,827; 3,783 KiB): **4,208 ported, 201 partial, 1,362 body,
1,381 row, 229 referenced, 12,446 not started**. By retail bytes,
**29.3% ported/partial, 38.0% with body/row**.

The historical in-game measurement group (14,399 functions; 2,432 KiB) is
**44.9% / 57.0%**. The remaining 5,428 functions are still in scope; their
reporting groups appear below. These figures count citations and code bytes,
not directly witnessed branches or a percentage of gameplay completeness.
Three Kongo functions move from partial to ported; Party Ball adds three
ported citations, three partial citations and explicit unsupported guards. The site count increases even as its witnessed FALL/REST path and
Kongo barrel use become exact; site counts do not measure gameplay parity.

| area | functions | ported | partial | body | row | referenced | not started | KiB | bytes ported+partial | with body+row |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| it items | 3802 | 803 | 23 | 133 | 117 | 38 | 2688 | 473 | 39.2% | 44.7% |
| ft characters | 3672 | 1715 | 35 | 411 | 921 | 53 | 537 | 445 | 62.5% | 87.5% |
| gr | 1760 | 149 | 8 | 89 | 0 | 28 | 1486 | 340 | 17.9% | 20.6% |
| ft common states | 1471 | 514 | 80 | 191 | 322 | 8 | 356 | 248 | 62.0% | 77.8% |
| ft engine | 1108 | 341 | 25 | 213 | 21 | 43 | 465 | 237 | 54.7% | 69.2% |
| HSD (sysdolphin) | 499 | 141 | 2 | 27 | 0 | 15 | 314 | 180 | 35.4% | 45.4% |
| mp | 229 | 212 | 1 | 2 | 0 | 2 | 12 | 102 | 92.5% | 92.9% |
| gm | 617 | 15 | 2 | 28 | 0 | 6 | 566 | 98 | 6.0% | 13.9% |
| it engine | 475 | 127 | 15 | 108 | 0 | 11 | 214 | 88 | 47.3% | 62.3% |
| lb | 211 | 61 | 2 | 36 | 0 | 1 | 111 | 58 | 47.2% | 60.1% |
| ef | 65 | 14 | 5 | 33 | 0 | 0 | 13 | 49 | 72.7% | 93.1% |
| if | 163 | 5 | 0 | 10 | 0 | 1 | 147 | 49 | 5.6% | 27.2% |
| cm | 127 | 21 | 1 | 13 | 0 | 4 | 88 | 35 | 30.2% | 37.5% |
| pl | 191 | 9 | 1 | 19 | 0 | 6 | 156 | 29 | 8.5% | 17.4% |
| MSL | 9 | 4 | 0 | 5 | 0 | 0 | 0 | 1 | 82.0% | 100.0% |
| **total** | 14399 | 4131 | 200 | 1318 | 1381 | 216 | 7153 | 2432 | 44.9% | 57.0% |

### Every reporting group remains in scope

| scope | functions | ported | partial | body | row | referenced | not started | KiB | bytes ported+partial | with body+row |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| in-game | 14399 | 4131 | 200 | 1318 | 1381 | 216 | 7153 | 2432 | 44.9% | 57.0% |
| single-player and modes | 1520 | 5 | 0 | 2 | 0 | 1 | 1512 | 278 | 0.1% | 0.2% |
| menus | 530 | 1 | 0 | 10 | 0 | 2 | 517 | 260 | 0.1% | 4.8% |
| rendering | 795 | 42 | 1 | 11 | 0 | 6 | 735 | 206 | 4.5% | 13.6% |
| OS, hardware, loading | 746 | 27 | 0 | 12 | 0 | 3 | 704 | 165 | 2.0% | 3.7% |
| results | 352 | 0 | 0 | 5 | 0 | 0 | 347 | 102 | 0.0% | 2.0% |
| trophies | 151 | 0 | 0 | 0 | 0 | 0 | 151 | 97 | 0.0% | 0.0% |
| audio | 336 | 1 | 0 | 2 | 0 | 1 | 332 | 84 | 0.2% | 0.5% |
| single-player fighters | 557 | 1 | 0 | 0 | 0 | 0 | 556 | 51 | 0.3% | 0.3% |
| memory card | 168 | 0 | 0 | 0 | 0 | 0 | 168 | 44 | 0.0% | 0.0% |
| debug | 153 | 0 | 0 | 2 | 0 | 0 | 151 | 37 | 0.0% | 0.5% |
| cutscenes | 84 | 0 | 0 | 0 | 0 | 0 | 84 | 20 | 0.0% | 0.0% |
| pause | 36 | 0 | 0 | 0 | 0 | 0 | 36 | 7 | 0.0% | 0.0% |
| **total** | 19827 | 4208 | 201 | 1362 | 1381 | 229 | 12446 | 3783 | 29.3% | 38.0% |

Character-family percentages are sensitive to shared code and citation
style: Fox/Falco, Marth/Roy and other family crates can implement behavior
without naming each clone symbol. All 26 registered characters have recorded
witness families; that does not close their partial/unwitnessed branches.
Kirby now has 24/25 copied neutrals, with Sheik still unported. Do not reuse
the older copy-ability percentages as the current state.

## Largest remaining breadth areas

1. **Stages:** ten Versus stages are supported, with explicit guards for
   unwitnessed hazards; 19 remain. Large untouched files include Big Blue
   (41.2 KiB), Corneria (24.8), Brinstar (18.1), Icicle Mountain (16.2),
   Peach's Castle (15.2), Green Greens (14.5) and Venom (14.0). The whole
   historical in-game `gr` group is 17.9% / 20.6% by bytes.
2. **Items:** 2,688 of 3,802 item-kind functions are not started; the group
   is 39.2% / 44.7%. Capsule/Box/Barrel and random spawning are integrated
   within witnessed setup bounds, now including whole empty Box/Barrel
   blasts and narrow primary-article owner transfer to an unowned REST
   Barrel. Twenty-three ground Sword recordings admit only common held
   AttackLw3 (57), AttackHi4 (63) and witnessed throws; they end at entry.
   Party Ball now has neutral left-facing FALL/REST coverage; its other
   states, other random kinds, shooting items, heavy carry/throws,
   stage/enemy items and many fighter use rows remain.
3. **Modes, menus and match shell:** the historical in-game `gm` group is
   6.0% / 13.9%; other mode, menu, results and pause families are reported
   separately above. FFA and witnessed teams/doubles are integrated;
   stock sharing and timed-team ranking remain guarded. The app's own
   menus do not count as retail ports.
4. **HSD and camera:** 314 of 499 historical in-game HSD functions are not
   started. The unnamed `hsd_3A94.c` and `hsd_3B34.c` remain wholly
   uncited, while `camera.c` is 60% not started. Display, single-player,
   pause and debug behavior all remain in scope.
5. **Kirby and CPU edges:** copied Sheik Needle Storm remains unported;
   its start boundary is not an ability witness. CPU players now run in
   the guarded FD/Battlefield Versus envelope, with static character
   profiles and 95 registered witnesses. Remaining CPU stage, command,
   item and recovery paths stay explicit guards; CPU replay setup lacks
   a recorded level. The CPU crate still has 39 fail-closed sites.
6. **Shared engine sequencing:** the container Resume phase correction is
   integrated, with 37 strict random-item witnesses including combined
   Sword/container cases. Party Ball FALL/REST and Kongo saved AObj/controller-clock
   restoration with complete held/launch witnesses are integrated. Next
   existing-capture tasks are log-standing and revival-floor entry. New call sites need
   direct retail branch witnesses; monster births, heavy entries and the
   25 held aerial entry captures do not authorize later continuations.
   No new recordings are authorized in this task.

The current `summary.md` and `inventory.tsv` list not-started source files
and functions; `fail_closed.tsv` lists exact current guards. The historical
packet map below is a discovery aid only.

## Current fail-closed backlog

**269 current sites** are enumerated in
`target/parity/wave16-integrated/fail_closed.tsv` (250 unimplemented,
19 bail). A site may guard an entire family of motion rows or setup states;
site count is not branch count. Do not infer current row totals or agent
scope from the older packets below. Check current source and complete
recordings before enabling anything.

### Historical 32-packet map (before the six handoff integrations)

The following map was measured at `969a065e`: **331 historical sites**,
including 258 unimplemented motion rows counted through shared dispatch.
Its counts, line numbers, capability descriptions and reachability are
historical and are not reconciled to the 269-site inventory above. Several
packets have now landed partly or fully (CPU, copied specials, match flow,
random items, Kongo/Japes and common grab edges). Keep it as an index of
retail families, not a current assignment list or completion claim.

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

These 32 rows accounted for the old 331-site snapshot. The current
not-started inventory remains broader than known guards: whole stages,
items and modes can be absent without a dedicated fail-closed site. No
historical measurement group or packet excludes that work from the goal.
