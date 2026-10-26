# Corneria (grcorneria.c, GrCn.dat): not started

Wave 27's stage agent ported Brinstar and Venom; Corneria is unported (it
stays out of `Stage::PORTED`). What is ready for it:

- The boundary `start_cor_fox_marth4` exists (savestate backed up, seed
  3309739787, traces recorded in the main checkout) but is not registered in
  `harness/boundaries.toml`: register it once the cold and warm gates pass.
- The Arwing machinery Venom uses is shared code to extend, not copy:
  `melee_gr::venom::arwings` (spawner, ship maneuvers and fire windows,
  platform placement), `scene_stage::venom::passes` (nested map creation in
  GObj order, RObj constraint resolved at fire time and at each display
  pass, platform joints), and `it-arwinglaser` (aimed lasers 1 and 3).
- The Star Fox taunt is guarded on both stages
  (`ftfoxappeals.c:55`, `melee_ft::input::iasa`).

What Corneria adds beyond Venom (read grcorneria.c in full first):

- The Great Fox (map 3, joints 3 and 4 on its root via grCn_803E1D38) moves:
  grCorneria_801E1970 draws three HSD_Randf per wait and drifts the ship
  (offset_x/offset_y with velocity clamps, yakumono_param x0..x14), and
  grCorneria_801E2228 lowers it (xD0) on background 4 with HSD_Randi(3).
  Map 0 copies its translation (grCorneria_801E1878); the markers follow.
- Its two cannons are material items (grMaterial_801C8CFC with capsules,
  fn_801E12D4 counts beam damage to x38 = 200, then destroys them, a large
  quake and joint 4 off). grCorneria_801E1348's cycle fires the Great Fox
  laser (It_Kind_GreatFox_Laser, itgreatfoxlaser.c, it_802EAF34) when a
  fighter is within 10 units of the cannon's height.
- Backgrounds 8, 9 and 4 scroll by x88 a tick and recreate one another
  (grCorneria_801E03C8); which one the camera's interest is over
  (grCorneria_801E08CC, Camera_GetTransformInterest) picks the spawner's
  pass types and the Great Fox's descent.
- Arwings: types 1..9 near (rotated -pi/2, positions from bone 4 with
  lbl_803E2068 offsets and a floor of y 100), 10..13 far; near ships also
  fire the cannon-laser types 0/2 (it_802E72E0 with side beams 4 and 5),
  which `it-arwinglaser` guards. Platform joints grCn_803E2190 {0,0,1,2,5}.
- The laser's Corneria offset (grCorneria_801DDCF0) in
  itArwinglaser_UnkMotion3_Phys.
- The lowest eye height grCorneria_801E2FCC (assets.rs refuses Corneria in
  `stage_camera` until it is ported) and the shadow test 801E2EEC.
- Params: GrCn.dat yakumono_param x0..x88 dumped during the wave (x74/x78
  are s32 fire-window period and one-in-N draw).
