# Mushroom Kingdom II, Great Bay

Wave 27. Two Versus stages, each with a Fox/Marth start boundary
(`start_mk2_fox_marth4`, `start_gb_fox_marth4`, warm and cold gated in
`m4_gate` with their particle ledgers), a full idle match in `m5_gate`
(`mk2w_idle_6000`, `gbw_idle_6000`) and explorer matches in
`CORPUS_V3_MATCHES`.

## Mushroom Kingdom II (grinishie2.c, GrI2.dat)

- Map 2's on_init (grInishie2_801FCE04) draws four times: the two lift
  waits, Birdo's and the carpet's (`test_random` draws nothing for an empty
  range). Its controller counts the four down; each one spent makes an
  event map: the lifts (maps 3, 4) out of the waterfall, Pidgit's carpet
  (map 1), and Birdo's platform (map 15), which makes Birdo. An event map
  that ends removes itself and rearms its timer. Ground_804D6950's
  `released` flags gate a map's re-creation until its GObj is gone.
- Joints 2..11 sit on maps 5..14, which are never made: they stay disabled
  (`DISABLED_AT_SETUP`).
- Birdo (`it-kyasarin`, itkyasarin.c) walks in from a blast zone, stops,
  steps back and forth, lays egg volleys, and leaves on her timer or a
  heavy hit; past her endurance she flies at the screen or tumbles. She
  has no map collision while walking. Her eggs (`egg.rs`) are throwable
  items; a burst egg leaves its shell (`melee-ef` egg_shell, models
  0x1C/0x1D, 36 creation draws). Birdo's removal (`birdo_gone`) tells the
  stage through `ItemEvent::StageNotice`.
- The warm boundary restores the controller timers and any live event map
  (`initial_state/stage_inishie2.rs`).

Guarded (`unimplemented!` with the C line, never reached by 900 explorer
matches with branch tags): a tumbling Birdo meeting the map
(it_8027C79C, itzako.c:462) and a reflected egg (itkyasarinegg.c:184).

## Great Bay (grgreatbay.c, GrGb.dat)

- Creation order 0, 3, 4, 2, 1, 9, 10; map 9's on_init (the moon) makes the
  four giants (maps 5..8) between its GObj and its proc. Map 10 is
  Tingle's platform, an identity JObj past the archive's ten models, whose
  proc runs at s_link 5. Seven setup draws, among them the moon's fall
  wait and Tingle's creation (it_802ECA70: the facing
  coin, his place and his wait) taken before the item pool exists
  (`tingle_draws`).
- The floating platform (map 2, bone 49) weighs its riders and slants,
  slides and sinks (grGreatBay_801F60C4); the turtle (map 1) surfaces,
  dives and rises at one of three places with its shell vertices following
  its bones (grGb_803B8048, mpVtxSetPos) and then reclassified
  (mpLib_80055E24: a shell line can turn from wall to floor as the turtle
  rises, which decides what pushes an item off it), a wake generator while it swims
  (killed by grLib_801C9874) and splashes from its bones (grLib_801C97DC).
- The water is a fighter device (fn_801F4520): a fighter crossing y = the
  surface from above, off a ledge, makes a splash scaled by its camera
  radius. It runs in hit detection before the hit logs resolve, as retail.
- Tingle (`it-tincle`, ittincle.c) floats down on his balloon and bobs;
  riders sink him (their weight fed through it_802ECC98) and he rises
  back; a hit pops the balloon and he falls, lands, lies, gets up and
  floats back up. it_802EB5C8 clears xD5C, so he never slides on a slope
  (he rides the turtle's shell at its line speed). Falling past -50 tears him down and grGreatBay_801F5600
  makes him again at once (`respawn_tingle`, the facing query and two
  it_802EB6DC resets around the map's scale).
- The moon falls once: map 9's loop flag is 0 in GrGb.dat, so
  grAnime_801C8138 never sets AOBJ_LOOP and its 4690-frame fall ends on
  AOBJ_NO_ANIM, never AOBJ_REWINDED; grgreatbay.c:834's rewind branch is
  unreachable (a 30000-tick port run confirms it) and stays guarded.
- The turtle's item draw (grGreatBay_801F63F4) is ported; spawning the item
  with items on (grgreatbay.c:1349) fails closed: no items-on Great Bay
  boundary exists yet.
