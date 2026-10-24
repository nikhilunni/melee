> **Branch** `worktree-agent-a215182eba8447ed8` (tip `b40348b6`), not merged into main. Based on 26fee4c7. No full sweep or replay-batch has run since the rebase (both needed; the replay runner changed). cd9625be leaves a diff3 marker in boundaries.toml: squash it with 3227638a. **dae25e9f is WIP** (save-at-end savestate tooling, unused). To merge, rebase onto main, sweep, replay-batch (before: a main build).

# Handoff: match flow (FFA, teams, doubles) and boundary import

Branch `worktree-agent-a215182eba8447ed8`, rebased on main 26fee4c7.

## Task and scope (wave 13)

docs/PARITY.md work items 32 (match flow: final stock/elimination, a second
fighter's revival at its own spawn, more than nine camera subjects; 3- and
4-player matches) and 26 (boundary import: saved article GObjs in
FighterVars, charged specials in a savestate, scheduler/camera/emitter
states), then retail's match flow (team battles and friendly fire, stamina,
coin, handicap, damage ratio, time mode with more than two players), with
doubles in the Slippi replay runner as the stretch goal.

## Done and verified (gates exact against retail)

Commits:
- `cd9625be` Three- and four-player free-for-all: `MatchConfig`/`Setup`
  hold 2-4 players; fn_8016DEEC's initial facing (note the stale f26
  register: the first candidate takes the previous distance, not its own);
  one ifStatus HUD proc / percent display / stock display per player;
  gm_GetFFAOutcome (`eliminated + 1 >= players`, not "first player out");
  gm_80167320: a stock player out of stocks sleeps on (closes both
  `life.rs` "final stock / elimination" sites); camera subjects in a fixed
  pool of 64 slots (closes `frame.rs` "more than nine camera subjects";
  melee-cm now takes `&mut [Option<&mut Subject>]`); explorer, recording
  config, boundaries and `replay_to_scenario.py` handle 2-4 players;
  `record.py` plugs a controller per player.
- `a1286a80` Team battles and doubles: `PlayerSlot.team`/`friendly_fire`
  with `teammate_of`/`spares`; team checks in grabs (ftcoll.c:1584),
  fighter hits (1677-1685, thrown attacker uses the grabber's team),
  item-vs-fighter (2032), fighter-vs-item (itcoll.c:425-433),
  item-vs-item (561-563); facing skips teammates; gm_GetTeamBattleOutcome
  (`frame::stock_outcome_decided`, `MatchOutcome::TeamWinner`); CPU
  ftCo_IsAlly; Samus missile lock-on skips the owner's team
  (`LockOnTargets::opposes`); team KO tally for gm_8016C75C; saved
  boundaries read rules at lbl_8046B6A0+24C8 and StaticPlayer +47;
  `make_boundary.py --teams 0 1 0 1 [--friendly-fire]` (pokes CSS rules,
  door teams, start-data teams and save GameRules +9); Slippi runner
  accepts 2-4 leaders and teams, NeutralSpawn.asm teams rows
  (`slippi::teams_order`, `neutral_spawn_teams`), neutral pad for an
  eliminated player's absent frames. FFA follow-ups: `release_pair` sets
  down a Falcon Dive captor that hangs from its victim (x221B_b7 role
  swap); fighter pools sized 8 (ledge holders, lock-on fighters);
  CpuFighter +4C/+50 words that name no live Item are stale memory
  (closes part of `initial_state/cpu.rs:42`).
- `3227638a` fixes a diff3 marker left in boundaries.toml by the rebase
  (the intermediate commit cd9625be has a broken boundaries.toml; squash
  them when merging).
- `46afa741` clippy and a test literal after the rebase.

Witnesses (all in `CORPUS_V3_MATCHES`, m5_gate.rs, TOMLs copied into the
branch):
- FFA: corpus_v3_bf_fox_marth_falco2_ef89b3e70_p0,
  corpus_v3_fd_fox_falco_marth_captainfalcon2_ef89b3e70_p0,
  corpus_v3_bf_fox_marth_falco1_{ec13743d5_p0,e00f31913_p1,e19f8579b_p2},
  corpus_v3_fd_fox_iceclimbers_marth_falco1_{ec13743d5_p0,e00f31913_p1,e19f8579b_p2},
  corpus_v3_bf_fox_marth_falco2_{ec13743d5_p0,e00f31913_p1,e19f8579b_p2},
  corpus_v3_fd_fox_falco_marth_captainfalcon2_{ec13743d5_p1,e00f31913_p1},
  corpus_v3_fd_fox_falco_marth_captainfalcon2_ec13743d5_p0 (Falcon Dive
  pair parted by a third fighter).
- Teams: corpus_v3_bf_teams_fox_marth_falco_peach2_{e75fb4a9a,e89a89d0e,edb2b114a,ef89b3e70}_p0
  (friendly fire off), corpus_v3_fd_doubles_ff_fox_falco_marth_captainfalcon2_{same four}_p0
  (friendly fire on).
- Slippi doubles matched in full (not yet registered as
  SLIPPI_REPLAY_WITNESSES): public-v3.7-b/CPTFALCON/18_34_44 ... (DL), 9190/9190;
  public-v3.7-b/FALCO/18_15_55 ... (BF), 12382/12382.

New boundaries (all in harness/boundaries.toml; savestates in the main
checkout's harness/roms, traces in harness/traces): start_bf_fox_marth_falco2,
start_fd_fox_falco_marth_captainfalcon2, start_bf_fox_marth_falco1,
start_fd_fox_iceclimbers_marth_falco1,
start_bf_iceclimbers_iceclimbers_iceclimbers_iceclimbers1,
start_fd_doubles_ff_fox_falco_marth_captainfalcon2 (teams 0 1 0 1, FF on),
start_bf_teams_fox_marth_falco_peach2 (teams 0 0 1 1, FF off). Each start
and `_cold` scenario gates 600 ticks exact. They were not yet copied to
~/melee-data/roms.

## NOT done: the full sweep

The final sweep against target/sweeps/26fee4c7.tsv was interrupted. An
earlier sweep ran with a stale binary (pre-rebase) and so showed 18 false
"regressions" (Kirby copy scenarios from main, two cape-turn scenarios).
With the rebuilt binary kirby_copy_shadowball_fd_mewtwo4 gates exact. Run
the sweep first (below); replay-batch before/after too (the runner
changed: leaders, teams, spawn rule, eliminated frames). A baseline binary
was copied to target/melee-sim-main (the main checkout's 11:16 build).

## WIP commit

- `dae25e9f` WIP: `MELEE_SAVE_AT_END` in dolphin/tick_trace.py saves a
  savestate (and `.json` sidecar with the seed) when a tick trace ends;
  `harness/save_midmatch.py --pairs <leadin.toml> <name> ...` records a
  lead-in with record_many and saves roms/<name>.sav. A lead-in must not
  use `render_every_tick` (the one-entry pad queue would be saved; the
  tracer refuses). Four lead-ins exist and gate exact
  (leadin_fd_donkeykong_fox4_full_punch, leadin_fd_samus_fox4_charge_stored,
  leadin_fd_mewtwo_fox4_ball_stored, leadin_fd_samus_fox4_missile_flight,
  scenarios in the main checkout only), and their savestates
  mid_fd_*.sav were saved. The four warm scenarios that play on
  (mid_fd_donkeykong_fox4_full_punch_release,
  mid_fd_samus_fox4_charge_stored_release,
  mid_fd_mewtwo_fox4_ball_stored_release,
  mid_fd_samus_fox4_missile_flight_idle; main checkout scenarios) were
  never recorded successfully with the final savestates: record them,
  then gate. Expect the importer to fail closed at ft-donkey/init.rs
  (full Giant Punch: the secondary colour overlay slot x488 is not
  imported at all; initial_state imports no colour overlays), ft-samus
  (charge shot / missile GObj in FighterVars), ft-mewtwo (Shadow Ball).
  Those are item 26's real porting work.

## Next steps

1. `cargo build --release -p melee-sim` (NOT `-p ... --examples`, which
   skips the binary), then the full sweep:
   `target/release/melee-sim sweep --scenarios /Users/nikhilunni/Projects/melee/harness/scenarios --out target/after.tsv --baseline /Users/nikhilunni/Projects/melee/target/sweeps/<main>.tsv --lock /Users/nikhilunni/Projects/melee/target/sweep.lock`;
   replay-batch over ~/melee-data/replays/public-v3.7 before and after.
2. Port the team stock steal (fails closed in frame.rs, "a team battle's
   stock steal"): fn_8016B918 (gm_16AE.c:533-590) in the scene frame before
   gm_GetMatchOutcome; fn_8016B918_inline picks the teammate (stocks > 1);
   ifStock_802F7EFC sets up the flying stock icon (anim slots 5/6, start/
   mid/end via lbVector_8000DE38) and ifStock_802F8298 spawns efSync 0x475
   at anim 1 and 0x476 at anim 10 (RNG); Player_LoseStock, SetStocks,
   gm_8016C658, fn_8016719C(i, 0) (respawn, reuse complete_death's revival).
   Witnesses: public-v3.7-b FALCO/18_57_46 (FD) steals at tick 7804 and
   diverges at 7803; also check 18_13_54 (YS, 6807), 19_14_10 (PS, 6309),
   18_52_22 (PS, 9236).
3. Record the four mid-match warm scenarios and port the importer sites
   they reach (item 26).
4. Register the two fully matched doubles replays as Slippi witnesses.
5. Player_80032070(slot, 1) (partner_fighters.rs: Nana revives at her own
   spawn while Popo is on his platform) is still open; the 4-IC boundary
   is the place to look for it with the explorer.

## Commands

- Gate: `target/release/melee-sim gate /Users/nikhilunni/Projects/melee/harness/scenarios/<name>.toml`
- Explore and bridge: `cd harness && MELEE_DATA_ROOT=/Users/nikhilunni/Projects/melee uv run python explore_batch.py ../target/<out> 4 0 --boundary <name> --samples 8`
- Doubles replay: `MELEE_DATA_ROOT=/Users/nikhilunni/Projects/melee target/release/melee-sim replay "<slp>" --all-characters-unlocked true`
- `crates/slp` presence of a port's frames: an ad-hoc example (not
  committed) printed when frames go absent/present, stocks and Start.

## Gotchas

- Slippi writes no frames for an eliminated player; a teammate who steals
  a stock reappears (Rebirth) with Start held in the first new Pre Frame.
- `cargo fmt` reformats many untouched files here; do not run it.
- make_boundary's friendly fire must be set in save GameRules (+9); the
  CSS rules byte alone is overwritten.
- I deleted, at the time of creation, savestates I had just made that
  were unusable (a team boundary with friendly fire off, and four
  mid_fd_*.sav saved with the one-entry pad queue); they were remade.

## Divergences outside this area

- start_bf_iceclimbers x4 explorer (seed 89a89d0e profile2): melee-cpu
  tumble.rs:100 "ftCo_800AC5A0: r30 with no knockback is the Fighter's
  address" (work item 4).
