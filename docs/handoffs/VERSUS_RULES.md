> **Branch** `worktree-agent-a2cd0a235fdb06398` (tip `fcee686a`), not merged into main. Stamina, Single Button, Invisible, Giant and Tiny are merged into main (5528fef4 and before). WIP commit 12f8338a holds the partial Coin mode; the rest of this note is what remains.

# Versus rules: what is left (wave 26)

Ported and witnessed: Time (2-4 players, 2v2), the team stock steal,
handicap and Damage Ratio, timed stock team battles, Lightning, Slo-Mo,
Super Sudden Death, Stamina including the HP-out KO, Single Button,
Invisible, Giant and Tiny. Details and witnesses:
`docs/PORT_NOTES/VERSUS_RULES.md`. Every boundary below exists in the main
checkout (`harness/roms`, `harness/scenarios`, traces recorded).

Still guarded, in the order I would take them:

1. **Coin** (boundary `start_fd_coin1_fox_marth`, seed 2751777771, time
   limit 60; it and its `_cold` twin gate exact over their 600 idle
   ticks). A `WIP:` commit holds the parts written so far; the match runs
   until the first damage or fall, where `guard_coin_spills`
   (`crates/melee-lib/src/frame.rs`) stops it.
   - Done, compiled, not yet exercised by a witness:
     `crates/melee-it/src/coin.rs` (coin attributes, `plan_spill` =
     it_802E61C4/it_802E6380/it_802E628C with its HSD_Randf draws,
     `coin_floor_pass` = it_802E5AC4, `tick_coin_timers` = it_802E657C,
     `follow_coin_sphere`), `ItemScratch::Coin`, ItemCore
     `bounce_spark_off` (xDCF b0), `crates/it-coin` (states of
     it_803F8BD8, `enter_flight` = it_802E66A0), `crates/melee-lib/src/coins.rs`
     (purses, StaticPlayer +90..+9C), coin score in `standings`
     (fn_8016588C x5 == 2).
   - Written, not wired: `crates/melee-ft/src/fighter/coins.rs` (no `mod`
     line yet). It needs FighterCore fields `coin_rules`, `coins_held`,
     `coin_spills: FixedVec<CoinSpill, N>`, `coin_spheres: [CoinSphere; 2]`;
     FighterAssets `coin_spheres` (ftData x38) and `coin_spill` (PlCo
     +5C0/+5C8/+5CC); a striker id for `SpillCause::Hit` (the file calls
     `ko_source.player()`, which does not exist).
   - Still to do: call `spill_coins_for_hit` from process_damage at the
     knockback branch's start and in the no-knockback block (angle from
     ftCo_Damage_CalcAngle, dmg.facing_dir_1, x1854 collision position);
     the death payout from the death entries (ft_0D31.c: left
     x = blast left angle 0, right x = blast right angle pi, down
     y = blast bottom angle pi/2, up y = blast top 3pi/2; star and screen
     KOs at cur_pos, not collectable, pi/2); the scene registers Unk4's
     assets from ItCo x8 bank 0x73 (`from_article_slot_states`), dispatches
     `Unk4: it_coin::Coin`, drains `coin_spills` after the fighter procs
     (purses, it_802E609C creation, it_802750F8, the it_802E6658 cap);
     pickup ft_8007C77C in hit detection between the fighter and item
     loops (on alternate frames by the parity of gm_80479D58.unk_8 at
     0x80479D60; lbColl_80007B78 / lbColl_800067F8; efSync 0x432 at
     contact); spheres ft_8007C630 at spawn and ft_8007C6DC in
     Fighter_procUpdate; container coins (it_8026F8B4). Then witnesses from
     the explorer on the registered boundary.
2. A **Sudden Death among only some players** (3-4 player Time ties):
   `Match::sudden_death` refuses it. Needs a 3-4 player Time boundary whose
   match ends in a partial tie.
3. **Camera** mode: the boundary driver could not reach its CSS.
