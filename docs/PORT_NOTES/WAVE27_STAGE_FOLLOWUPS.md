# Wave 27: Mushroom Kingdom's `?` blocks, Kongo Jungle's own guards

Continues `docs/handoffs/WAVE26_FOLLOWUPS.md` and the "Still guarded" list
of `docs/handoffs/app-guarded-paths.md`.

## Illusion's ghost and its initial pass

it_8029CEB4 spawns Fox's and Falco's Illusion ghost through
`Item_InitSpawn`, which sets `x44_flag.b0`: Item_80268B18 runs the item's
initial air pass (it_80275E98 -> it_80276100). On a jointed floor the pass
lands and fires the joint's floor callback, so Mushroom Kingdom's lift
weighs the ghost (fn_801FC9AC, `x50 * 8`: the next tick it drops faster)
and Onett's cracking roof moves Fox a ulp. Both stage divergences of wave
26 were this (`corpus_v3_mk_fox_marth4_e19f8579b_p0`,
`corpus_v3_onett_fox_marth4_ef89b3e70_p0`).

## Mushroom Kingdom with items (grinishie1.c)

- Creation (grInishie1_801FAD84's tail, items on outside Sudden Death and
  any common kind switched on): two different bricks become the two
  waits' `?` blocks, a `?` Ground (map 2) each. The second brick is
  redrawn until it differs, so the setup draws vary: the cold path finds
  the one count the boundary seed is consistent with
  (`cold::hatena_setup_draws`; two consistent counts fail closed).
- A `?` Ground is presentation only (grInishie1_801FAC58: the blink and
  its model over the brick), but it is a scheduled GObj; each lives under
  its own scheduler key (`HATENA_KEY_BASE + brick`), with a reserved model.
- A `?` brick breaks like any other (fn_801FBEB8, fn_801FBF6C) and first
  (grInishie1_801FB0AC) drops an item: it_8026F7C8, a contents-table kind
  (it_8026C75C on it_804A0E50) created five above the brick
  (it_8026F5C8, no container) and launched; then its wait restarts.
  When a wait runs out, a random plain brick other than the last one turns
  (HATENA_APPEAR_CHECKLOOP).
- The random spawner's solidity probe (mpColl_8004D024) rises three units
  into its marker: a brick it meets takes the ceiling callback at once and
  breaks (delta_y 3 > 0.7), before the new item exists.
  `w27_start_MushroomKingdom_items_all` drops its first item that way.
- The all-`?` event (both waits ending within `all_hatena_window` ticks)
  is witnessed (`w27k_mkall_ee6ff560c_p2`); breaking one of its bricks
  is not (both recorded ones were item bumps, below) and fails closed.

## Kongo Jungle (grkongo.c, itklap.c)

- Logs: Ground_GetStageGObj makes a new GObj and model each time, so a
  map's log can come while its last one floats. The second takes a second
  scheduler key (12 for map 1, 13 for map 2). Collision joint 0 or 1
  follows the log that landed last (Ground_801C2ED0), and only that log
  hears it (mpJointSetCb1); a sinking log disables the joint and clears
  the callback whichever log it serves, and its removal disables the joint
  again (Ground_801C3128). grKongo_801D7F78's crowd test walks every other
  Ground: logs, passers-by and the path rider (its JObj's translation).
- The path rider: grKg_803B7FD4's second pair (spline 5); its drop sound
  (Ground_801C5440, no state); Ground_801C4B50's steep-tangent reference
  and both `M_PI -` corrections and the singular branch.
- The Klaptrap (itklap.c): it lets go once its child passes z 0
  (it_802E1C84: blast zones checked, falling and turning upright), a hit
  sends it tumbling (it_802E1E94), and its end removes the rider
  (itKlap_Logic10_Destroyed -> grKongo_801D8058) before its own GObj goes.

## Still guarded

- Mushroom Kingdom: an item's own map pass breaking a brick when its proc
  also draws (a bounce's spark, it_80278800). Retail breaks the brick in
  mpCollEnd's callback, inside the pass, before the bounce; the scene
  applies joint ceiling events after the proc. 172 of 600 explorer cases
  reach it (`w27k_mk_eb4935276_p1` tick 591 and `w27k_mk_ee6ff560c_p2`
  tick 982 are recorded and diverge there without the guard). Fixing it
  needs the stage callback to run from inside the item's collision.
- An all-`?` brick breaking (grinishie1.c:651-670).

- The barrel catching a Klaptrap (grKongo_801D8078, it_802E20D8) and its
  launch (it_802E2330): 3 of 360 explorer cases.
- The Klaptrap's bite switch (lb_8000B134 never ends its looping
  animation) and a rider gone before its Klaptrap.
- A third live log of one map; a passer-by created while its map's lives.
