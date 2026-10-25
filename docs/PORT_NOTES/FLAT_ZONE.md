# Flat Zone (grflatzone.c, ittools.c)

Stage: `melee_gr::flatzone` (logic), `melee_lib::scene_stage::flatzone`
(engine half, setup), `initial_state::stage_flatzone` (savestate import),
`it-tools` (the falling tools), `melee_mp::dynamic_attr` (the oil spill).

- Creation (grFlatzone_80216E78): maps 0, 1, 2, 3, 4, 5, 6 and 8 (map 7 is
  never made; map 8 is past GrFz.dat's eight models and gets an empty JObj).
  Maps 2..6 and 8 bind their archive collision joints (Ground_801C2ED0) from
  the visible, unanimated model, then 3..6 and 8 hide. Two setup draws: the
  controller's event wait (`rand_int(3000, 1800)`) and, after creation,
  grFlatzone_80218060(1)'s variant wait. The bottom blast zone drops by 50
  through Ground_801C3990, which stores an absolute height in the relative
  field (the camera centre is added twice).
- Every fighter gets `x34_scale.z = p_ftCommonData->x7E4` before OnLoad
  (Fighter_80068E64), and ftCo_8009DD94 solves the dynamic bones with the
  root's x scale at `x34_scale.y` (both witnessed by `w25_fz_start600_v01`'s
  bone dump).
- Controller (grFlatzone_802171D4): `{3, 4}[HSD_Randi(2)]`, redrawn while it
  equals the last event, so cases 1 and 2 (maps 4 and 6, grFlatzone_80218260)
  are unreachable and stay `unreachable!`. The uncleared xC5 is 0 at the
  boundary; only a stale 3 or 4 would change the first event's draws.
- Platforms (grFlatzone_802174EC): `grFz_803E7A68` rows; each animation
  change re-stitches the joints (mpLib_80058560). grAnime_801C8138 resets the
  pose (lb_8000B804, `BackgroundAnimation::reset_pose`) before the new
  animation's frame zero.
- Oil man (grFlatzone_802176BC): the spill is grDynamicAttr material 0x11
  (traction 0.1), radius 22, on the joint-50 floor below (mpCheckMultiple),
  for 1200 frames, aged by mpLib_800587FC (registered at s_link 4, p_link 6)
  and read by mpCollEnd. Retail's early release tests `xCC == NULL` and so
  never releases (kept). The controller is freed as soon as he pours.
- Tools (grFlatzone_80217EF0, it_802EEFA8): one every 60 frames for 1200,
  x = `HSD_Randf() * 100 - 60`, kind `HSD_Randi(5)`, facing `HSD_Randi(2)`,
  colliding with joint 50 only. it_802EF548 reads the attacker's
  ftLib_800866DC position, which the scene fills before Item_8026A294.
- Savestate import: a boundary before the first stage proc (map 5's xC6 still
  0xFF) rebuilds the bind-time collision; a later one takes the saved JObj
  matrices and stitches. A live oil man, spill or falling tools at the
  boundary fail closed.
