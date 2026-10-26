# Big Blue (grbigblue.c): handoff

Wave 27 ported Mute City and Poké Floats (docs/PORT_NOTES/WAVE27_FZERO_PURA.md)
and left Big Blue unported: `Stage::BigBlue` is not in `Stage::PORTED`.
This note keeps what the wave learned so the next port starts from it.

## Data already recorded (main checkout)

- Boundary `start_bb_fox_marth4` (make_boundary.py --stage BigBlue --players
  Fox Marth, not registered in harness/boundaries.toml): savestate
  `harness/roms/start_bb_fox_marth4.sav` (backed up to `~/melee-data/roms`),
  seed 3657601642, the 600-tick `ledger600` recording, and the
  `start_bb_fox_marth4{,_cold}.toml` scenarios.
- `bb27_idle_fox_marth4`: 3800 ticks of both fighters idle (the road, the
  cars, the flyers and the platforms over a minute), recorded and
  compressed; its TOML is in harness/scenarios.

## What is shared and already ported

- `melee_gr::fzero::CarJoints` (grFZeroCar_801CAFBC: thirty identity car
  joints scaled by the ground scale) and `orientation` (Ground_801C5AEC).
- Cardinal splines, splGetSplinePoint and lbShadow_8000E9F0's tangent.
- `stage_limits::StageLimits` for camera and blast-zone limits that move.
- Line hazards (`LineHazard`, `scene_stage::line_hazard`, ftCo_800C0B20)
  and `scene_items::tear_down` for a stage material item's death.
- `melee_gr::desc::read_big_blue` (GrBb.dat, fog row 1; the lights are
  row 0x23's) reads the archive. Map scale 0.5.

## The stage (from the decomp and asm)

- grBigBlue_801E57C0: `unk8C.b5 = 1`; creation order 0x1F, 0, 1, 2, 0x22,
  0x21 (whose on_init creates map 4, the cars), 0x20 (hidden), 0x23, 0x24
  (hidden); then Ground_801C39C0/3BB4, mpLib_80058044 on joints
  0x21..0x41 (disabled), and the permanent wind lb_80011A50((-1, 0, 0),
  -1, 1.0, 0.0, pi/3, +-100000).
- grBb_803E2938 (StageData.joints): 28 extra collision bindings on map 34.
- Map 0x1F (grBigBlue_801E5B20): the position-marker wrapper, translated
  by yakumono_param x134 (0, 60, 60) and scaled by x140 (0.8); markers
  4..7 are on map 31's joints 4..7 (so revival markers move with it), 0..3
  offset by the flyer (grBigBlue_801EF7D8).
- Map 0x21 (grBigBlue_801E6364): 30 car collision JObjs (grBb_803E2DC0),
  the cars (map 4, grFZeroCar_801CAFBC with grBb_803E2D84) reparented
  under them, rotated pi/2 about y and scaled by x0C; car 9's child at x
  10; grBigBlue_801EC6C0 then HSD_Randi spawn timer from x10..x14. Its
  proc grBigBlue_801EF424 drives them.
- Map 0x20 (grBigBlue_801E6904): three riding platforms on bones 1, 6, 11
  (collision joints 0..2, mpJointSetCb1 fn_801E8560: needs a fighter's
  y, which `JointEvent` does not carry yet); grBigBlue_801E6C60's state
  machine (cases 0..2, lane speeds grBb_803B8114 (56, 40, 24) times the
  ground scale) with x88/x8C HSD_Randi waits.
- Map 0x22 (grBigBlue_801E8D64, proc 801E93D8) and 0x23 (801E9F3C, proc
  801EA05C): the road's segments and the flyers; many HSD_Randi draws at
  creation (setup_draws needs the count, possibly variable).
- yakumono_param dumped 0x0..0x140: x0 -60, x18 4 cars, x24 0.9, x28 0.04,
  x6C 10 (scroll), x134 (0, 60, 60), x140 0.8.
- Functions compiled with fp_contract: check every multiply-add in
  801E6C60, 801E8B84, 801E93D8, 801EA05C, 801EAB50, 801EB4AC, 801EBAF8,
  801EC58C, 801ECB50, 801ED694, 801EDF44, 801EE398, 801EEF00 and
  fn_801E8560 against `asm.py --fused`.
- grLib_801C9E60 (the stage's drift, -10) is read by items and the CPU.

## Suggested shape

`melee_gr::bigblue::{mod, road, cars, flyers, platforms, procs}` and
`scene_stage/bigblue.rs` as Mute City's, a warm restore that rebuilds the
creation and imports the saved Ground vars, then `bb27_idle_fox_marth4`
and an explorer batch from `start_bb_fox_marth4` once registered.
