# Explorer campaign, second round: the unexplored boundaries (2026-10-18)

The breadth round `EXPLORER_CAMPAIGN_2026_10.md` left open: the fifteen
start boundaries added at its end, `start_dl_link_roy4` (added since), and
the eight Kirby boundaries. Every character explored only on Final
Destination in the first round (Donkey Kong, Dr. Mario, Luigi, Mewtwo, Ness,
Pichu, Roy, Sheik, Young Link, Zelda, Mr. Game & Watch) now has explorer
matches on another stage. Explored from `main` at c5d78e56 (the Kirby hats) plus
the bridge change below; every gate result below was checked again on
`main` at 9fa4e7da (item-teardown generator positions).

## First: explorer bridges now render every tick

`replay_to_scenario.py` writes `render_every_tick = true`, as
`slippi_to_scenario.py` does (`--dolphin-cadence` leaves it out). The
explorer's `Match` (melee-lib) never skips a display pass: `Runtime` starts
at `DisplayPass::Rendered` and only `melee-sim`'s scenario runner calls
`set_display_pass`. A bridge recorded at Dolphin's two ticks per render was
therefore another game wherever the display pass matters.

The two samples the first round could not explain were bridged again from
fresh explorer recordings (seed 0xbe7703ed, profile 0; the pads are
identical to the old scenarios'):

| Scenario | Explorer | Old bridge (two ticks per render) | New bridge (`_ret`) |
|---|---:|---:|---:|
| `corpus_v3_fod_fox_marth4_ebe7703ed_p0_ret` | 4709 ticks, P2 wins | retail ends at 3837 | retail plays 4710 records, P2 wins; gate exact, particles exact |
| `corpus_v3_dl_captainfalcon_jigglypuff4_ebe7703ed_p0_ret` | 5809 ticks, P2 wins | retail ends at 5014 | retail plays 5810 records; gate exact, particles exact |

`replay_to_scenario.py --verify` passes for both. So display cadence was the
cause, and every bridge below carries `render_every_tick = true`.

## What ran

- 1,656 explorer cases (8.9 million port ticks) from 24 boundaries, 23 input
  seeds each (seeds 30023 to 30045 of the xorshift sequence, `skip` 30023),
  three profiles per seed. Two `explore_batch.py` runs: the sixteen new
  stage boundaries (`--samples 16`), the eight Kirby ones (`--samples 8`),
  both `--per-fault 2`.
- 43 cases bridged and recorded: 19 shortest faulted cases and 24 clean
  samples.
- 95 faulted cases, 13 distinct messages (one, `ftCo_800AC5A0`'s fighter
  address, is an explorer-only limit).

| Boundary | Cases | Finished | Playing at 6000 | Faulted |
|---|---:|---:|---:|---:|
| `start_bf_mewtwo_peach4` | 69 | 2 | 65 | 2 |
| `start_bf_sheik_iceclimbers4` | 69 | 22 | 46 | 1 |
| `start_bf_gameandwatch_link4` | 69 | 3 | 66 | 0 |
| `start_ys_drmario_samus4` | 69 | 2 | 45 | 22 |
| `start_ys_ness_marth4` | 69 | 14 | 55 | 0 |
| `start_ys_younglink_peach4` | 69 | 3 | 66 | 0 |
| `start_ys_samus_falco4` | 69 | 52 | 17 | 0 |
| `start_dl_donkeykong_peach4` | 69 | 1 | 68 | 0 |
| `start_dl_luigi_iceclimbers4` | 69 | 19 | 48 | 2 |
| `start_dl_roy_link4` | 69 | 6 | 63 | 0 |
| `start_dl_yoshi_sheik4` | 69 | 54 | 15 | 0 |
| `start_dl_link_roy4` | 69 | 3 | 66 | 0 |
| `start_fod_ganondorf_link4` | 69 | 6 | 57 | 6 |
| `start_fod_pichu_samus4` | 69 | 37 | 32 | 0 |
| `start_fod_zelda_marth4` | 69 | 30 | 39 | 0 |
| `start_fod_captainfalcon_zelda4` | 69 | 42 | 25 | 2 |
| `start_fd_kirby_fox4` | 69 | 31 | 37 | 1 |
| `start_ys_kirby_fox4` | 69 | 18 | 1 | 50 |
| `start_fd_kirby_captainfalcon4` | 69 | 22 | 46 | 1 |
| `start_fd_kirby_falco4` | 69 | 19 | 49 | 1 |
| `start_fd_kirby_jigglypuff4` | 69 | 6 | 62 | 1 |
| `start_fd_kirby_donkeykong4` | 69 | 6 | 62 | 1 |
| `start_fd_kirby_mewtwo4` | 69 | 3 | 63 | 3 |
| `start_fd_kirby_gameandwatch4` | 69 | 2 | 65 | 2 |

## Faults, by case count

"Stops" means the port, fed retail's own recording, panics with the same
message at the explorer's fault tick (`melee-sim gate` stops at the first
divergence, so the panic also says the port was exact until then). Ticks
are the explorer's fault tick; the scenario's `frames` is two past it.

| # | Fault | Cases | Where | On retail | Shortest retail witness (ticks) | Likely owner |
|---|---|---:|---|---|---|---|
| 1 | `it_80270E30`: "Coin hit effect on a stage enemy" (`melee-it` `hurt.rs:268`) | 22 | Dr. Mario vs Samus, YS: Dr. Tornado (347) hits a Shy Guy | stops at 561 | `corpus_v3_ys_drmario_samus4_e052f4d33_p2` (563); also `_e170dc03b_p1` (638) | `melee-it` (stage-enemy hit effect), `it-*` Shy Guy |
| 2 | `itKirbycutterbeam_UnkMotion0_Phys` (itkirbycutterbeam.c:93-97): "the beam descending a sloped floor" (`it-kirby` `cutter_beam.rs:229`) | 46 | Kirby vs Fox, YS only (its sloped floors) | stops at 182 | `corpus_v3_ys_kirby_fox4_e1ffd6e62_p0` (184); also `_e170dc03b_p0` (191) | `it-kirby` |
| 3 | `grab_escape.rs:406` `unreachable!()` in `release`: the victim's `state_data` is not `Capture` | 7 | Kirby vs Mewtwo (3), Game & Watch (2), DK (1), Fox (1), FD: Kirby's inhale (353-356) takes a fighter (Mewtwo out of 341 or 264, into 288) | stops at 524 | `corpus_v3_fd_kirby_mewtwo4_ef540a761_p1` (526); also `_e052f4d33_p2` (907) | `melee-ft` `grab_escape.rs` / `capture_kirby.rs` (the inhaled victim's release) |
| 4 | "`ftCo_8009750C`: a captured fighter holding an item" (`melee-ft` `capture_captain.rs:65`) | 6 | Ganondorf vs Link, FoD: Gerudo Dragon (353) catches Link holding an item (359 to CaptureCaptain 275) | stops at 982 | `corpus_v3_fod_ganondorf_link4_e5909cbff_p2` (984); also `_e96382eb6_p0` (1305) | `melee-ft` `capture_captain.rs` (ftCo_8009750C's item drop) |
| 5 | `ftCo_Landing.c:41-50`: "scratch inheritance from unsupported source ActionId(360)" (`melee-ft` `landing.rs:362`) | 3 | Sheik's Vanish air end (360) landing into 43: Sheik vs Ice Climbers on BF, Zelda-as-Sheik vs Captain Falcon on FoD | stops at 1670 | `corpus_v3_fod_captainfalcon_zelda4_e5909cbff_p1` (1672); also `_e86fddb60_p1` (1955) | `ft-seak` (360's scratch) / `melee-ft` `landing.rs` |
| 6 | `ftKb_Init_803C9DD0` (ftkirby.c:3777): "a copied neutral special" (`ft-kirby` `init.rs:204`) | 3 | Kirby vs Falco, Jigglypuff, Captain Falcon, FD | stops at 1570 | `corpus_v3_fd_kirby_falco4_ed4320157_p0` (1572); also `corpus_v3_fd_kirby_jigglypuff4_e86fddb60_p2` (3755) | `ft-kirby` (copy abilities: known, TRACKER Next 2) |
| 7 | `ftCo_800BD6EC` (ftCo_CaptureWaitKirby.c:56-62): "the held fighter drops Kirby through a platform" (`melee-ft` `capture_kirby.rs:394`) | 3 | Kirby vs Fox, YS | stops at 686 | `corpus_v3_ys_kirby_fox4_e4e54007e_p2` (688); also `_ef41f983a_p1` (1624) | `melee-ft` `capture_kirby.rs`, `ft-kirby` SpecialN |
| 8 | "`ftCo_800AC5A0`: r30 with no knockback is the Fighter's address, unknown for this scene" (`melee-cpu` `tumble.rs:100`) | 2 | Luigi vs Ice Climbers, DL (Nana) | **not a port fault**: `corpus_v3_dl_luigi_iceclimbers4_e86fddb60_p1` gates exact over all 4073 ticks (the recorded scene knows Nana's address) | none needed | explorer limit (TRACKER: a table for recorded setups) |
| 9 | `ftKb_SpecialAirHi1_Coll` (ftkirbyspecialhi.c:447-453): "the aerial start landing into ftKb_MS_SpecialHi1 at the current frame" (`ft-kirby` `special_hi.rs:345`) | 1 | Kirby vs Fox, YS | stops at 429 | `corpus_v3_ys_kirby_fox4_e119262ea_p1` (431) | `ft-kirby` |
| 10 | "holding an item: motion state CaptureMewtwoAir is not audited" (`melee-ft` `item_pickup.rs:528`) | 1 | Mewtwo vs Peach, BF: Confusion (352) catches Peach, item in hand (Wait to 304) | stops at 2232 | `corpus_v3_bf_mewtwo_peach4_e0a079375_p1` (2234) | `melee-ft` `item_pickup.rs` (held-item audit) |
| 11 | "`ftCo_800C44CC`: x2071_b0_3 of motion 365 is not carried" (`melee-ft` `damage_bind.rs:29`) | 1 | Mewtwo vs Peach, BF: Disable hits Peach in Toad (365) | stops at 4400 | `corpus_v3_bf_mewtwo_peach4_e86fddb60_p2` (4402) | `ft-peach` (365's motion flags) / `melee-ft` `damage_bind.rs` |

One more distinct fault in the raw counts is the Kirby batch's catch-all
"entered unreachable code", which is fault 3 (all seven panics are at
`grab_escape.rs:406`).

## Silent divergences

### In a faulted case: a thrown Popo hits Nana, and her launch is mirrored

`corpus_v3_dl_luigi_iceclimbers4_e052f4d33_p0`: the explorer faults with
fault 8 at 4925, but retail's match ends at 4393, so the bridge diverges
earlier. First divergence at 3292: Luigi throws Popo (Luigi 222, Popo 242
from 3277), Popo's thrown body hits Nana (Nana 26 to 86 at 3292, 26 to 32
percent): retail `p2.facing_dir` -1, the port +1, and `p2.kb_vel.x` has
the opposite sign (1.3166 vs -1.3166). The direction of a hit by a thrown
fighter's body (its hitbox's owner, or the thrower's facing) is decided
differently. Likely owner: `melee-ft` thrown-fighter hit / `melee-coll`
knockback direction. Only witness: this scenario (exact for 3291 ticks).

### In 24 clean samples: one fighter fault, one particle fault

Base gate (fighters, items, seed): 23 of 24 exact over all retail played.
With `gate --particles`: 23 exact on `main` at 9fa4e7da. At c5d78e56 three
more had particle faults of known classes, which 9fa4e7da fixed (struck
through below).

| Scenario | First divergence | Class |
|---|---|---|
| `corpus_v3_fod_pichu_samus4_ef41f983a_p0` | **fighter**, tick 1057: `p0.percent` retail 5, port 4. Pichu in aerial Quick Attack's end (358), animation frame 46: retail applies a 1% self-damage there, the port does not. Retail's match then ends at 3976 where the explorer played 6000 | **New**. `ft-pikachu-family` `special_hi.rs` / Pichu's 358 script (self-damage command, `melee-cmd`) |
| same | particles, tick 913: bank 30 program 30007 generator position off by 8 ulps (y 0x3F97244E vs 0x3F972456), Pichu respawning (12) | **New** particle class (Pichu bank 30) |
| `corpus_v3_ys_ness_marth4_e170dc03b_p1` | ~~particles, tick 2264: bank 0 program 294 at position 0 on retail, the port at Ness's PK Fire (item 29)~~ exact at 9fa4e7da | Known: `PARTICLE_GATE_2026_10.md` program 294 |
| `corpus_v3_ys_younglink_peach4_ed4320157_p1` | ~~particles, tick 3810: bank 0 program 64, x -7.05 vs -4.93, Young Link in 344~~ exact at 9fa4e7da | Known: program 64 |
| `corpus_v3_dl_roy_link4_ea56d29fc_p0` | ~~particles, tick 493: bank 0 program 64, x -38.17 vs -40.70, Link in 344 hitting Roy~~ exact at 9fa4e7da | Known: program 64 |

The other twenty samples gate exact with particles over their full
length: `corpus_v3_bf_mewtwo_peach4_ea56d29fc_p0`, `_ea40991a0_p2`,
`corpus_v3_bf_sheik_iceclimbers4_ea40991a0_p0`,
`corpus_v3_bf_gameandwatch_link4_ee403ad76_p0`,
`corpus_v3_ys_samus_falco4_ef540a761_p1`,
`corpus_v3_dl_donkeykong_peach4_e0a079375_p1`,
`corpus_v3_dl_luigi_iceclimbers4_ea56d29fc_p1`,
`corpus_v3_dl_yoshi_sheik4_ee403ad76_p0`, `corpus_v3_dl_link_roy4_ef41f983a_p0`,
`corpus_v3_dl_roy_link4_ea40991a0_p0`,
`corpus_v3_fod_ganondorf_link4_ef41f983a_p2`,
`corpus_v3_fod_zelda_marth4_e1ffd6e62_p0`, and all eight Kirby samples
(`corpus_v3_fd_kirby_fox4_ea56d29fc_p0`, `_ef41f983a_p2`,
`corpus_v3_fd_kirby_captainfalcon4_ee42e8dad_p0`,
`corpus_v3_fd_kirby_falco4_e6eb56344_p2`,
`corpus_v3_fd_kirby_jigglypuff4_e7da23597_p0`,
`corpus_v3_fd_kirby_donkeykong4_e170dc03b_p2`,
`corpus_v3_fd_kirby_mewtwo4_ef540a761_p2`,
`corpus_v3_fd_kirby_gameandwatch4_ea56d29fc_p2`). With the three fixed
by 9fa4e7da, 23 samples are candidates for `CORPUS_V3_MATCHES`; nothing was
registered here.

## Recorder notes

- The ledger gap of the first round recurs: when retail's match ends before
  `frames` (the Pichu and Luigi cases above), the RNG-ledger pass fails and
  the traces stay uncompressed. Recording again with `frames` at the trace
  length (which `explore_batch.py` writes) completes; done for both.
- `corpus_v3_dl_roy_link4_ea40991a0_p0` failed twice with "no scheduler
  tick for 120 VI fields", once in the tick pass and once in the ledger pass
  at record 777, under a load average near 50; the third attempt, at a
  load near 8, completed. A host-load flake, not the scenario.

## Not done

Nothing was fixed or registered. The faults' witnesses and the clean
samples sit in the main checkout's `harness/scenarios` with their traces;
their TOMLs are committed in this branch's `harness/scenarios/`.
