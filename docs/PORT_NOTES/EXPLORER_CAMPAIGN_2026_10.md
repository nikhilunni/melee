# Explorer campaign across the roster (2026-10-12)

One breadth round of `explore_batch.py` over every registered boundary but
Kirby's, then the start of a second round over seventeen new boundaries. The
campaign was wound down after that: fifteen of the new boundaries have had
no explorer match yet (listed at the end). Built from `main` at 595a009b;
every result below was checked again on `main` at de6702a0.

## What ran

- 3,837 explorer cases (19.3 million port ticks) from 54 boundaries: 23
  input seeds (69 cases) per boundary, seeds 30000 to 30022 of the xorshift
  sequence, and 60 seeds (180 cases) of Sudden Death.
- 45 cases bridged to retail and recorded (13 faults, 32 clean samples): 41
  gate exact with 0 differing particle ticks, 4 do not (three faults below).
- 14 faulted cases in all, five distinct messages; one silent divergence in
  32 samples.

Cases per character and stage (a case counts for both its characters):

| Character | FD | BF | YS | DL | FoD | PS | Total |
|---|---:|---:|---:|---:|---:|---:|---:|
| Bowser | 69 | 69 | 0 | 0 | 0 | 0 | 138 |
| Captain Falcon | 138 | 0 | 0 | 69 | 0 | 0 | 207 |
| Donkey Kong | 69 | 0 | 0 | 0 | 0 | 0 | 69 |
| Dr. Mario | 69 | 0 | 0 | 0 | 0 | 0 | 69 |
| Falco | 207 | 69 | 0 | 69 | 0 | 0 | 345 |
| Fox | 1905 | 69 | 138 | 69 | 69 | 276 | 2526 |
| Mr. Game & Watch | 138 | 0 | 0 | 0 | 0 | 0 | 138 |
| Ganondorf | 69 | 69 | 0 | 0 | 0 | 0 | 138 |
| Ice Climbers | 138 | 69 | 69 | 0 | 0 | 69 | 345 |
| Jigglypuff | 69 | 0 | 0 | 69 | 0 | 69 | 207 |
| Link | 69 | 0 | 0 | 69 | 0 | 69 | 207 |
| Luigi | 69 | 0 | 0 | 0 | 0 | 0 | 69 |
| Mario | 138 | 0 | 0 | 69 | 69 | 0 | 276 |
| Marth | 456 | 138 | 69 | 138 | 69 | 138 | 1008 |
| Mewtwo | 69 | 0 | 0 | 0 | 0 | 0 | 69 |
| Ness | 69 | 0 | 0 | 0 | 0 | 0 | 69 |
| Peach | 414 | 0 | 69 | 0 | 0 | 138 | 621 |
| Pichu | 69 | 0 | 0 | 0 | 0 | 0 | 69 |
| Pikachu | 69 | 138 | 0 | 0 | 69 | 0 | 276 |
| Roy | 69 | 0 | 0 | 0 | 0 | 0 | 69 |
| Samus | 138 | 0 | 0 | 0 | 0 | 69 | 207 |
| Sheik | 69 | 0 | 0 | 0 | 0 | 0 | 69 |
| Yoshi | 69 | 69 | 69 | 0 | 0 | 0 | 207 |
| Young Link | 69 | 0 | 0 | 0 | 0 | 0 | 69 |
| Zelda | 69 | 0 | 0 | 0 | 0 | 0 | 69 |

By stage: Final Destination 2,457 (180 of them Sudden Death), Pokémon
Stadium 414, Battlefield 345, Dream Land 276, Yoshi's Story 207, Fountain of
Dreams 138.

## Faults, by case count

Ticks are trace ticks. "Stops" means the port panics on retail's own game
inside the named recording; the scenario's `frames` is two past the
explorer's fault.

| # | Tag | Cases | Characters, stage | First divergence | Shortest retail witness | Status |
|---|---|---:|---|---|---|---|
| 1 | `ftCo_800AC5A0`: "steering with no knockback (uninitialised r5/r30)" | 6 | Ice Climbers vs Fox and vs Peach, FD and PS | stops at 1186 | `corpus_v3_ps_iceclimbers_peach4_ed9a5198f_p0` (1187) | Known, owned; **fixed on `main`** during the campaign. All five recorded cases gate exact now; three are registered |
| 2 | "the tether's hang countdown outside AirCatchHit" (`melee-ft` `air_catch.rs:55`) | 4 (2 on retail) | Link vs Fox, Samus vs Fox, PS | stops at 5238 (Link), 5639 (Samus) | `corpus_v3_ps_link_fox4_e23bd2614_p2` (5239); `corpus_v3_ps_samus_fox4_e51dc0146_p0` (5640) | **New** |
| 3 | `itPeachToadSpore_Logic68_Reflected`: "it_80273030 keeping the lifetime" | 2 | Mario vs Peach, FD | stops at 2999 | `corpus_v3_fd_mario_peach4_ef4e3c329_p1` (3000) | Known (`UNREGISTERED_SCENARIOS.md`); **fixed on `main`** during the campaign. Both cases gate exact and are registered |
| 4 | "mv+4 written from outside during motion ActionId(14)" (`melee-ft` `landing.rs`, `overwrite_common_scratch_word`) | 1 | Pikachu vs Mario, FoD | stops at 1664 | `corpus_v3_fod_pikachu_mario4_eac9514d6_p1` (1665) | **New** |
| 5 | Mr. Game & Watch's second torch (silent) | 1 of 32 samples | Mr. Game & Watch vs Samus, FD | 5749, `items.0.kind` (retail 116, the port 96; `items.count` 2 and 1) | `corpus_v3_fd_gameandwatch_samus4_ebe7703ed_p0` (6001; exact for 5749) | **New** |
| 6 | "holding an item: motion state FlyReflectWall is not audited" | 1 (0 on retail) | Link vs Fox, PS | none: retail does not reach it | none (`corpus_v3_ps_link_fox4_e276b41d4_p2` is exact for the 4644 ticks retail played) | Known (tracker, Next 1; `COVERAGE_AUDIT.md`); still no retail witness |

### 2. The tether lands before its chain reaches the wall

Link's aerial Hookshot (360) meets a wall (AirCatchHit 361 at 5237) and he
lands the next tick (Landing 42 at 5238); Samus the same (357, AirCatchHit
358 at 5628, Landing at 5637). The article is still paying out from the
wall, and when it arrives it starts the hang: `hookshot::wall_step`
(`ft-link-family` `hookshot.rs:509`) and `grapple::wall` (`ft-samus`
`grapple/mod.rs:540`) write the hang countdown through
`Fighter::air_catch_hang_frames`, which panics in any state but AirCatchHit.
Retail writes `mv.co.aircatchhit.x4` whatever the fighter's state is: mv+4
of Landing here. The port has the mechanism for such a write
(`overwrite_common_scratch_word`, Landing's `retained_drop_timer`); what is
not checked against the asm is whether the article should have left its
wall state when the fighter landed. Needs a wall within a tether's length of
a floor, so only Stadium among the six stages showed it (4 of 138 Link and
Samus cases there, none in 207 on Final Destination).

### 4. Thunder's lead bolt ends while Pikachu stands

Pikachu is hit out of Thunder (360 to Damage 78 at 1642), is in Wait (14)
from 1656, and the lead bolt ends at 1664. it_2725_Logic39_Destroyed
(802B2020) then runs ftPk_SpecialLw_SetState_Unk0, which stores 3 at mv+4 of
whatever state Pikachu is in (`ft-pikachu-family` `special_lw.rs`
`lead_gone`). The port models that write for Jump, Fall, Run, Squat, Walk,
the aerials, Landing and the attacks, and fails closed for Wait, whose
scratch it does not model. Wait reads no scratch of its own; the open
question is what the next state inherits (`inherited_scratch_word` answers
`None` out of Wait today). Small and local once that is decided from the asm.

### 5. Mr. Game & Watch grabbed out of a second forward smash

He forward smashes (346) and starts another out of the first one's
interrupt window (frame 41 to 1 at tick 5717). Retail and the port then both
hold two torches (item kind 116) through 5748. Samus's Grapple Beam catches
him at 5749 (346 to CapturePulledHi 226): retail removes one torch on that
tick and the other on 5750; the port removes both on 5749. So the first
torch, which the second smash's setup no longer points at, outlives the
motion change by a tick on retail. An ordinary end later in the same recording shows a
lag too (the torch of 5867 leaves at 5923, a tick after the motion change
at 5922). Not looked at in the asm.

### 6. FlyReflectWall with an item in hand

The explorer's own Stadium game launches a Link holding an item into a wall
at tick 5858. Retail fed the same pads ends that match at 4644: a Stadium
transformation follows its disc read, and an explorer recording holds the
port's read timing. The gap stays where `COVERAGE_AUDIT.md` has it.

## Not port faults: what the bridge showed

- **Two samples where retail plays a shorter game than the explorer.**
  `corpus_v3_dl_captainfalcon_jigglypuff4_ebe7703ed_p0` ends on retail at
  5014 (Captain Falcon's fourth KO at 4899) and in the explorer's own run at
  5809; `corpus_v3_fod_fox_marth4_ebe7703ed_p0` ends at 3837 and 4709. The
  port gates exact against the whole retail trace in both, from the
  savestate and (the first one, checked) from a cold twin, and
  `replay_to_scenario.py --verify` confirms retail consumed the explorer's
  pads. So the port fed retail's recording is retail, and the port run by
  the explorer is another game. Not diagnosed. The likely cause is display
  cadence: an explorer bridge is recorded without `render_every_tick`, so
  retail renders every second tick, the gate follows the recording's
  cadence, and the explorer's `Match` renders every tick (the magnifier's
  damage tick is the known display-pass effect). If so, bridged explorer
  scenarios should carry `render_every_tick = true`, as Slippi bridges do,
  or a faulted case can leave the explorer's game before its fault.
- **A recorder gap those two cases hit.** When retail's match ends before
  the scenario's `frames`, the tick pass accepts it ("match ended early")
  but the ledger pass still demands the requested count
  (`record.py`: "the RNG ledger pass wrote 5014 records of 5810"), so the
  recording fails and its traces stay uncompressed (about 1 GB each, 3.7 GB
  for the Fountain of Dreams one). `explore_batch.py` then sets `frames` to
  the trace length, and recording the scenario again completes. Done here
  for the three cases affected; nothing is left uncompressed.
- **Stadium.** Of six Stadium faults bridged, three were not retail's game
  by the fault tick (exact for all retail played: 4644, 4917 and 5524
  ticks). The other three (fault 2 twice, fault 1 once) are.

## Boundaries added

Seventeen, each with a cold twin that gates exact over 600 ticks
(`make_boundary.py --no-register`, entries in `boundaries.toml`). Every
character but Kirby now has a boundary on a stage with platforms and one
against an opponent other than Fox.

| Stage | Boundaries |
|---|---|
| Battlefield | `start_bf_bowser_marth4`, `start_bf_mewtwo_peach4`, `start_bf_sheik_iceclimbers4`, `start_bf_gameandwatch_link4`, `start_bf_pikachu_ganondorf4` |
| Yoshi's Story | `start_ys_drmario_samus4`, `start_ys_ness_marth4`, `start_ys_younglink_peach4`, `start_ys_samus_falco4` |
| Dream Land | `start_dl_donkeykong_peach4`, `start_dl_luigi_iceclimbers4`, `start_dl_roy_link4`, `start_dl_yoshi_sheik4` |
| Fountain of Dreams | `start_fod_ganondorf_link4`, `start_fod_pichu_samus4`, `start_fod_zelda_marth4`, `start_fod_captainfalcon_zelda4` |

Explored so far: `start_bf_bowser_marth4` (69 cases, no fault, two samples
exact) and `start_bf_pikachu_ganondorf4` (69 cases, no fault, no sample
recorded). **The other fifteen have had no explorer match.** The next batch
is theirs:

    cd harness && uv run python explore_batch.py <out> 23 30000 --boundary <name> --samples 2 --jobs 4

## Registered

38 scenarios in `CORPUS_V3_MATCHES` (647 entries), each exact over its full
`frames` with 0 differing particle ticks on `main` at de6702a0: 30 clean
samples (one per boundary, two for Bowser on Battlefield, one Sudden
Death), three of fault 1, both of fault 3, and three Stadium matches of
Link and Samus. Left unregistered, in the main checkout's
`harness/scenarios` with their traces:

| Scenario | Why |
|---|---|
| `corpus_v3_ps_link_fox4_e23bd2614_p2`, `corpus_v3_ps_samus_fox4_e51dc0146_p0` | fault 2 |
| `corpus_v3_fod_pikachu_mario4_eac9514d6_p1` | fault 4 |
| `corpus_v3_fd_gameandwatch_samus4_ebe7703ed_p0` | fault 5 |
| `corpus_v3_fd_iceclimbers_fox4_e23bd2614_p2`, `corpus_v3_fd_iceclimbers_peach4_ed3951905_p2` | exact; second cases of fault 1 on boundaries that already have one |
