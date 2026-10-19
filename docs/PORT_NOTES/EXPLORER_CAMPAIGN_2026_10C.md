# Explorer campaign, third round: Fountain of Dreams, Stadium, Dream Land (2026-10-19)

The first two rounds (`EXPLORER_CAMPAIGN_2026_10.md`, `_10B.md`) left the
three stages with disc reads or creation draws thin: Bowser, Dr. Mario,
Mr. Game & Watch, Mewtwo, Ness and Young Link had no boundary on any of
them, and Pikachu, Pichu and Sheik one each. This round adds six
boundaries for those characters, explores them and six older boundaries
with fresh seeds, and fixes the four faults retail reproduces. Built from
`main` at 123198d9 (wave 12); every bridge renders every tick.

## Boundaries added

Six, made with `make_boundary.py --no-register` and entered in
`boundaries.toml`; each start scene and its cold twin gate exact over 600
ticks with particles. Savestates are in `harness/roms` and backed up to
`~/melee-data/roms`.

| Stage | Boundary | Seed |
|---|---|---:|
| Pokémon Stadium | `start_ps_bowser_ness4` | 512840711 |
| Pokémon Stadium | `start_ps_mewtwo_younglink4` | 512840711 |
| Fountain of Dreams | `start_fod_drmario_gameandwatch4` | 1899183411 |
| Fountain of Dreams | `start_fod_sheik_ness4` (Zelda on the CSS, A held) | 1801721350 |
| Dream Land | `start_dl_gameandwatch_mewtwo4` | 1032949840 |
| Dream Land | `start_dl_pikachu_drmario4` | 3933258060 |

The two Stadium boundaries share a seed: the menu drive reaches the match
on the same frame for both.

## What ran

Input seeds continue the xorshift sequence at `skip` 30046 (the second
round ended at 30045), three profiles per seed.

- **New boundaries:** `explore_batch.py <out> 23 30046 --per-fault 2
  --samples 6`, 414 cases, 2,385,083 port ticks; 16 faulted.
- **Older boundaries, fresh seeds:** `explore_batch.py <out> 12 30046
  --per-fault 1 --samples 4` over `start_fod_pikachu_mario4`,
  `start_fod_fox_marth4`, `start_ps_marth_jigglypuff4`,
  `start_ps_iceclimbers_peach4`, `start_dl_mario_falco4` and
  `start_dl_link_marth4`: 216 cases, 1,173,085 ticks, **no fault**.
- **After the fixes:** the explorer again over the four new boundaries that
  faulted (same seeds, 276 cases, 1,625,103 ticks): one fault left (open
  fault 5 below).

| Boundary | Cases | Ticks | Finished | Playing at 6000 | Faulted (before fixes) |
|---|---:|---:|---:|---:|---:|
| `start_ps_bowser_ness4` | 69 | 412,314 | 1 | 68 | 0 |
| `start_ps_mewtwo_younglink4` | 69 | 407,108 | 0 | 63 | 6 |
| `start_fod_drmario_gameandwatch4` | 69 | 383,790 | 2 | 59 | 8 |
| `start_fod_sheik_ness4` | 69 | 386,129 | 19 | 49 | 1 |
| `start_dl_gameandwatch_mewtwo4` | 69 | 406,955 | 3 | 65 | 1 |
| `start_dl_pikachu_drmario4` | 69 | 388,787 | 21 | 48 | 0 |
| `start_fod_pikachu_mario4` | 36 | 180,275 | 21 | 15 | 0 |
| `start_fod_fox_marth4` | 36 | 151,071 | 34 | 2 | 0 |
| `start_ps_marth_jigglypuff4` | 36 | 212,040 | 2 | 34 | 0 |
| `start_ps_iceclimbers_peach4` | 36 | 216,000 | 0 | 36 | 0 |
| `start_dl_mario_falco4` | 36 | 197,699 | 15 | 21 | 0 |
| `start_dl_link_marth4` | 36 | 216,000 | 0 | 36 | 0 |

Recorded on retail: 17 bridged cases (7 faults, 10 clean samples), four
witnesses past a fix (three full-length re-runs of the fixed cases, one
extension with neutral pads), one re-recording, and the six boundaries:
28 scenes.

## Faults

"Stops" means the port, fed retail's own recording, failed with the
explorer's message at the explorer's fault tick (exact until then).

| # | Fault | Cases | Where | On retail | Witness | Status |
|---|---|---:|---|---|---|---|
| 1 | `FixedVec::insert`: "assertion failed: index <= self.len" in `resolve_terrain_footsteps` | 8 | Dr. Mario vs Mr. Game & Watch, FoD: Game & Watch's TurnRun (19) into Run (21) | stops at 472 | `corpus_v3_fod_drmario_gameandwatch4_eb0b8bd2c_p1` (474), `_efbfb6a67_p0` (654) | **Fixed** (`melee-ft`) |
| 2 | `efLib_Cb_AccumOffset_FromParams` "on a freed item JObj" | 5 | Mewtwo vs Young Link, Stadium: a stuck fire arrow leaves with its line | stops at 4374 | `corpus_v3_ps_mewtwo_younglink4_ef3cf9943_p0` (4375) | **Fixed** (`melee-ef`) |
| 3 | `ftCo_Landing.c:41-50`: "scratch inheritance from unsupported source ActionId(354)" | 1 | Sheik vs Ness, FoD: Zelda lands out of Farore's Wind's air end | stops at 5032 | `corpus_v3_fod_sheik_ness4_ed7b67fb9_p0` (5034) | **Fixed** (`ft-zelda`) |
| 4 | `ftGw_AttackS4_ItemTorchSetFlag` (ftgamewatchattacks4.c:52): "a superseded torch ends while x2254_fireGObj's torch may be in hitlag" | 1 | Mr. Game & Watch vs Mewtwo, DL | stops at 921 | `corpus_v3_dl_gameandwatch_mewtwo4_e1c18df30_p2` (923) | **Fixed** (`ft-gamewatch`) |
| 5 | "fn_800DA004 / fn_800DA440 / fn_800DD684 (ftCo_Throw.c:225-234): a grab pair losing its floor (ftCo_800DC920, both to Fall)" | 1 | Mewtwo vs Young Link, Stadium, tick 5092 | **not reached**: `corpus_v3_ps_mewtwo_younglink4_e69d5406a_p1` gates exact over all 5094 retail ticks | none | **Open**, no retail witness |

### 1. A footstep's effect after a motion change's flush

ftAction_80072CD8 (the terrain footstep) records how many graphics
commands precede it (`TerrainStep::graphics_before`) so its foot effect
joins them in script order. When the proc changed motion first, the
commands issued before that change were taken by the change's
efAsync_QueueFlush (fighter.c:951); the port resolves and removes them
(`resolve_graphics_commands_before_seal`) before the footsteps, so the
recorded place pointed past the queue. The place now counts from the
commands left (`graphics_resolved`). A footstep issued before the change
would belong to the sealed batch; it fails closed
(`unimplemented!`, ftaction.c:1164-1177). Fountain of Dreams is the stage
whose floor has a footstep effect that Game & Watch's TurnRun-to-Run tick
reaches.

### 2. The fire arrow's flame outlived its arrow

efSync 0x448 (itlinkarrow.c:643, efasync.c:830-837) creates the flame with
the arrow's gobj as its parent, so the arrow's Item_8026A8EC
(efLib_DestroyAll, item.c:1991) removes it. The port's
`spawn_item_following_generator` left the model without an owner. A
stuck arrow normally outlives the flame's 0xB4 ticks; here a Stadium
transformation deactivated its line (`stuck_collision`), so the arrow left
first. Confirmed past the old fault by
`corpus_v3_ps_mewtwo_younglink4_ef3cf9943_p0_ext`: the same pads, then
neutral, to 4700 ticks, exact with particles. The `unimplemented!` for a
freed JObj stays for any other following model.

### 3. Farore's Wind's mv+4

The travels write `mv.zd.specialhi.x4` (the stick, ftzeldaspecialhi.c:423,
508, 513) and the endings keep it, as Sheik's Vanish does (326192ee). Zelda
now reports it through `RETAINED_SCRATCH_WORD` for 350, 351, 353 and 354.

### 4. A superseded torch ending in hitlag

ftGw_AttackS4_ItemTorchSetFlag (8014A904) thaws whatever x2254 names
(ItemTorchExitHitlag), then clears x2254, whichever torch ended. The port
sends the thaw and `Superseded` to the torch no newer one replaced: the
newer torch thaws at once and hears nothing more from its owner.

### 5. Open: a grab pair losing its floor on Stadium

The explorer's Mewtwo/Young Link game reaches ftCo_800DC920's floor-loss
branch at 5092, almost certainly under a transformation's moving or
vanishing floor. Retail fed the same pads plays another game by then
(Stadium transformations follow their disc reads; see
`COVERAGE_AUDIT.md`), so the bridge is exact for all 5094 ticks and
witnesses nothing. A witness needs a directed scenario (a grab held on a
transformation floor as it leaves) recorded on retail. Owner: `melee-ft`
throw states (fn_800DA004 / fn_800DA440 / fn_800DD684).

## Clean samples

All gate exact with 0 differing particle ticks over every tick retail
played:

| Scenario | Ticks |
|---|---:|
| `corpus_v3_fod_pikachu_mario4_e51868d7e_p0` | 6001 |
| `corpus_v3_fod_fox_marth4_ebbc08847_p0` | 4956 (P2 wins) |
| `corpus_v3_ps_iceclimbers_peach4_e51868d7e_p0` | 5461 (the explorer played 6000: a transformation) |
| `corpus_v3_dl_mario_falco4_ebbc08847_p0` | 6001 |
| `corpus_v3_ps_bowser_ness4_e51868d7e_p0`, `_e404fe6bc_p0` | 6001 each |
| `corpus_v3_fod_drmario_gameandwatch4_e51868d7e_p0` | 5346 |
| `corpus_v3_fod_sheik_ness4_e8508a02a_p2` | 6001 |
| `corpus_v3_dl_gameandwatch_mewtwo4_e8508a02a_p0` | 6001 |
| `corpus_v3_dl_pikachu_drmario4_e51868d7e_p1` | 6001 |
| `corpus_v3_ps_mewtwo_younglink4_e69d5406a_p1`, `_efbfb6a67_p0` | 5094, 4370 (bridged faults retail did not reach) |

No silent divergence: every bridged case is exact for as long as retail
played.

## Recorder note

The ledger gap of the first round recurs: `corpus_v3_ps_iceclimbers_peach4_e51868d7e_p0`
ended on retail at 5461 of 6001 requested ticks, the ledger pass failed
and the traces stayed uncompressed (about 3.9 GB). Recording it again with
`frames` at the trace length completed and compressed.

## Registered

Twenty-one scenarios in `CORPUS_V3_MATCHES`: the twelve samples above,
the seven fault witnesses (short ones and the three full-length re-runs:
`corpus_v3_fod_drmario_gameandwatch4_eb0b8bd2c_p1_full`,
`corpus_v3_fod_sheik_ness4_ed7b67fb9_p0_full`,
`corpus_v3_dl_gameandwatch_mewtwo4_e1c18df30_p2_full`), and the fire-arrow
witness with its extension.
