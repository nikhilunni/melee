# Icicle Mountain

Wave 27. The Versus stage (gricemt.c, GrIm.dat) with a Fox/Marth start
boundary (`start_im_fox_marth4`, warm and cold, `m4_gate`), an idle match
to near its end (`imw_idle_3800`, `m5_gate`) and explorer matches in
`CORPUS_V3_MATCHES`.

## The stage

- grIceMt_801F686C: maps 0 (the base; stageGObj0_OnInit sets the respawn
  bones 18..21 to grIm_803B8220[1..4]) and 8 (the background),
  Ground_801C3260 on maps 1..6 (their joints off, marked released), then
  map 9, the scroll controller: stageGObj9_OnInit draws the first two field
  segments (HSD_Randi(6) until one is not cooling down) and
  grIceMt_801FA0BC makes them, the upper one in view at `-20 * scale` and
  the under one above it (grIceMt_801F993C's spacing). GObj order 0, 8, 9,
  under, upper.
- The controller (grIceMt_801FA364, fn_801F8E58) holds a speed
  `rand_range(360, 900)` frames, then draws one of the twelve speeds not
  cooling down and eases to it over 60 frames; both segments move by it
  every frame (grIceMt_801F96E0). grIceMt_801F9ACC swaps a segment out once
  the view's segment has moved a whole spacing (down) or past zero (up),
  making the next one (fn_801F9038) beyond the other: placed, snapshotted
  (Ground_801C32AC) before Ground_801C2FE0, so its lines' first motion is
  from its unplaced pose.
- Segments' blocks are material items (grMaterial_801C8CFC) made in the
  on_init before the segment is placed: they start at the unplaced bone
  and follow it from their first physics step. A hit hides the bone (its
  collision joint hides at the next Ground_801C2FE0 and stays hidden: no
  mpJointUnhide) and spawns efSync 0x445 (generator 0x209, newly in the
  dust table). Removing a segment frees its blocks at once.
- Maps 2 and 4 carry platforms that give way under a fighter
  (fn_801F9338/9448/9558): their bones play once, the joint leaves at 265
  frames and returns at 638. The floor callbacks run when the fighter's
  collision proc returns (`apply_ceiling_events`).
- grIceMt_801F7080 (on_load, after the fighters): Ground_801C2FE0 on the
  segments and the falling snow (grLib_801C96F8(0x7530)).
- The cold setup's controller draws vary (a second segment repeating the
  first draws again): `icemt_controller_draws` finds the one count whose
  own draws replay consistently from the boundary seed, and refuses an
  ambiguous seed.

## Engine changes it needed

The fixed camera (Camera_8002DDC4), tracked revival markers (x2135: no
Rebirth map pass, no floor probe on the next revival) and a star KO riding
its floor with its twinkle at the copied point came from main (Poké Floats,
Mute City); Icicle Mountain turns on the fixed camera through its
GroundParam and the tracked markers in `Arena` (grIceMt_801F686C clears
stage_info.unk8C.b5), and its witnesses match with them as they are.

- Snow (material 14) takes mpLib_803BE2F8's row mpLib_803BDA08: stage-bank
  footsteps 30001, landings 30002 and down-bounds 30003.
- CPUs: grLib_801C9E60 is the scroll (grIceMt_801FA728, `Scene::stage_drift`)
  and ftCo_800A9CB4 recovers with the up special while the field scrolls
  down; the floor-island lists hold 256 (Icicle Mountain has more than 64).
  Kirby's Stone slide holds its angle and speed to +F8 and +100
  (ftkirbyspeciallw.c:308-352), which random play on these slopes reaches.
- The HUD's s_link 17 procs follow their GObjs: a revival's
  ifStatus_802F6508 makes the player's stock HUD again at the end of the
  list (`hud::HudOrder`), which orders two players' stock-loss particles
  in one tick.

## Not ported

- Adventure's branches (Stage_80225194() == 76: map 10, the zako
  generator, the burst speed) are not reachable in Versus.
- Items on Icicle Mountain have no witness: `items_supported` leaves it
  out, with Mushroom Kingdom II and Great Bay.
