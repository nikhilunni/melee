# Mute City, Big Blue, Poké Floats

Wave 27. The F-Zero stages share grfzerocar.c (`melee_gr::fzero`: the
thirty car joints of grFZeroCar_801CAFBC and Ground_801C5AEC's orientation).

## Shared changes

- `stage_limits::StageLimits` holds stage_info's camera limits and blast
  zone as match state; Mute City's script moves them (Ground_801C3880 ..
  Ground_801C39B0). Fighters, items, the camera and the CPU read it.
- GrMc.dat's map 0x1E animation has an unrelocated `0xFFFFFFFF` child
  (AnimJoint and MatAnimJoint) under the spline joints' parents.
  grAnime_801C6C0C reads the children's payloads through it; Dolphin's
  reads there return zero, and the boundary's RAM shows those joints
  without an AObj. The readers mark it (`unrelocated_child`) and the
  joints below take no animation.
- Cardinal splines: splGetSplinePoint's cardinal branch is witnessed (the
  car splines), and lbShadow_8000E9F0's cardinal tangent is ported
  (`hsd_anim::spline::tangent`, 0x8000EC68..0x8000ED48).
- `ParticleSystem::kill_joint` is grLib_801C98A0 (type 0x80 on every
  generator following a joint, then hsd_8039D4DC).

## Mute City (grmutecity.c)

Creation order 0, 0x1E, 0x1D, 2 (map 0x1E's on_init makes the platform and
the cars before its proc is registered, so its proc runs first at s_link 4).
No setup draws. The route is map 0x1E's 3600-frame loop
(grAnime_801C775C); grMc_803E34E0's commands run against its frame: they
stop it where the platform sets down (rate 0, bone 0x11 and the platform's
bone 6 keep playing), swap the platform's collision joints, move the camera
limits (scaled) and the blast zone (unscaled, past the stage's own), raise
and lower the track's floor and set its ends' shape.

- The track's floor (joint 4, B10, its callback fn_801F2B58 recorded and
  failing closed for an item on it): grMuteCity_801F094C steps each spline's
  parameter by 0.0004 until it straddles z = 0; 801F0D20 lays lines 0x31,
  0x33 and 0x35 there (0x801F0E0C..64, 0x801F0F04..14 fmadds).
- The cars (grMuteCity_801F1328, 801F106C, 801F1A34): an insertion sort by
  distance that compares slot 0 with the word before grMc_8049F440 (the last
  word of Brinstar's grZe_8049F170, zero on a fresh boot; the sort can swap
  a leader out into it), close pairs, the five leaders and trailers, a
  HSD_Randi(4) per car close behind another; placement between the splines
  (the 0.93..0.01 wrap, the pit's zeroed tangent, 0x801F1FBC..801F2004
  fmadds and a double fmadd of 3 along the up vector), sparks (0x11A) and a
  sound's HSD_Randi(3) for touching pairs, the Loop quake and a HSD_Randi(3)
  for a car passing the stage, a stage material item (state 2, capsule
  (0,0,-12)..(0,0,2), radius 15) within x, y +-200 and z -100..50. A car's
  item damage (grMc_803E3C6C) smokes it past 8 (0x116) and wrecks it past 16
  (0xE3, HSD_Randf for the wreck's turn); a wreck smokes (0x119) in front of
  the camera.
- The platform's tilt (grMuteCity_801F0290) scales bone 6's animated
  rotation by a factor stepping 0.001 toward commands 22's targets.
- Display only: the hazard sign, overlays, lights, the car models.
- Warm boundaries are the creation: rebuilt and checked against the saved
  car table and order; the platform's collision is bound from its pose
  before animation (the saved JObj matrices are the animated ones).
- The track's line hazards (grMuteCity_801F2BBC, Ground_801C5700's
  on_touch_line): while grMc_804D69D4 is set, end lines 0x31 and 0x35 give
  yakumono_param x8's hit (8 damage, electric, 90 degrees) and lines
  0x32..0x34 xC's (18, normal, 80). ftCo_800C0B20 applies it in the hit
  detection pass, before the hit logs resolve: a kind-3 damage-log entry
  with no attacker, gated by the hurt status, the revival and ledge
  intangibility and PlCo +5DC's cooldown (bury_timer_1, x2328).
- A wrecked car's item goes through Item_8026A8EC's whole teardown
  (`scene_items::tear_down`, the frame loop's flush): the generators on
  its bones die with it. The stage leaves `InitialState` while the cars
  step so the teardown can reach the rest.
- `unk8C.b5` is clear on Mute City (grmutecity.c:368) and Hyrule Temple
  (never set): fn_8016719C records the revival marker on the player
  (Player_80032FA4) and the fighter keeps it (smash_attrs.x2135).
  ftCo_Rebirth_Coll then maps nothing (Fighter_procMap's ECB-lock
  countdown still runs), ftCo_800D5600 resets the collision positions a
  second time, and from the second revival on Fighter_UnkProcessDeath
  skips the spawn's floor probe. Both stages' marker 4 is on the static
  map 0, so the per-tick re-aim (ftCo_Rebirth_Phys) is the entry's.
- The revival reads the live camera top (`stage_limits`), which Mute
  City's script moves.

## Poké Floats (grpura.c)

Creation: maps 0, 1, 4; map 4's on_init (stageGObj4_OnInit) binds its
collision joints, plays its animation, makes 25 camera subjects on the
first 25 bones under its root (grPura_80212CD4) and creates every Pokémon
(grPura_802125F0: maps 2, 3, 5..26, map 13 three times under keys 28 and
29), placing each Ground's JObj on its bone (Ground_801C33C0(4, joint)).
Map 4's proc follows the Pokémon's. Map 1's on_init draws the sky's two
colours (HSD_Randi(4) until they differ): a variable setup draw count the
cold start inverts by searching the count whose creation draws it.

- The match camera is fixed (grGroundParam x4C_fixed_cam, so
  Ground_EnableMatchCamera picks CAMERA_FIXED): `melee_cm::fixed`,
  Camera_8002DDC4 with Stage_80224CAC's interest and grGroundParam
  +0x50..+0x64. The 25 subjects are still linked and updated
  (grPura_80212EF4), after the fighters'.
- grPura_80211D00 scales the camera limits by 0.8 about the camera's
  centre at creation (in `Assets::stage_camera`).
- Each tick: the Pokémon copy their bones' translation, rotation, scale
  and hidden flag (stageGObj2_GObjProc); map 4 moves the subjects, the
  collision joints (Ground_801C2FE0), 41 vertices onto joints of maps 2,
  8, 12 and 25 (grPura_80213030) with the joints' bounds, then
  mpLib_80055E24(0x18) and lb_800115F4. Map 1 counts its 3600-tick fade
  and draws the next colour.
- A fighter KO'd standing on a rising Pokémon star-KOs while still
  grounded: DeadUpStar rides the floor (mpGetSpeed) like the side deaths,
  and the twinkle (efAsync 0x42D, no JObj) spawns at the position queued,
  not the root's at the flush.
- Display only: the toon texture, the sky colour (Ground_801C205C,
  Camera_SetBackgroundColor).
