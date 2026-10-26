# Versus rules: what is left (wave 27)

Ported and witnessed: Time (2-4 players, 2v2), the team stock steal,
handicap and Damage Ratio, timed stock team battles, Lightning, Slo-Mo,
Super Sudden Death, Stamina including the HP-out KO, Single Button,
Invisible, Giant, Tiny, Coin, and a Sudden Death among only the tied
players. Details and witnesses: `docs/PORT_NOTES/VERSUS_RULES.md`.

Still guarded, in the order I would take them:

1. **Camera mode** (`SpecialMelee::Camera`, GM_CAMERA_MODE 0x0A). The
   boundary exists: `w27_start_fd_sp_camera_fox_marth4` (Fox vs Marth on
   FD, seed 3929830345, 600 idle ticks recorded; `_cold` written). Its CSS
   starts only with a controller in port 4, so it was made with
   `MELEE_SI_PORTS=0,1,3`, and every recording from it (record_many,
   explore_batch) needs the same environment; scenarios have no field for
   it yet (the Rust scenario and boundary parsers deny unknown fields).
   What the port needs (gm_PrepCameraModeVSScene, gmcameramode.c:190-231):
   - an untimed Time match: timer_enabled off, x5_0 (gm_GetMatchOutcome
     returns NONE, gm_16AE.c:878), so the cold path must accept Time
     without a time limit for this mode;
   - x1_2 and x1_3: the countdown (gm_Scene_Vs_OnEnter) and GO
     (gm_16AE.c:495, 2140) are banner element 8, not 3 and 4
     (`melee-lib::banner` and the importer's `scene_flow.rs` know 1, 3, 4).
     ifStatus_802F6EA4(8, ..) creates nothing and calls both callbacks at
     once (if_2F6E.c:36): at scene enter, before the boundary, fn_8016B7F8
     releases every input freeze and starts the stage
     (Ground_801C0FB8: FD's deferred GObjs), and fn_8016B784 enables the
     HUD. A camera boundary therefore has no banner, the HUD on, input
     released and the stage started, which the match-start import
     (`match_start_countdown`, the FD GObj registration) assumes are still
     to come;
   - x2_5 off: no score popups; x2_6, x3_1, x4_0 off (pause-related and
     ifStatus_802F6508);
   - every player xD_b3 -> Player_SetMoreFlagsBit4 (gm_16AE.c:1647), which
     ftLib_80086ED0 reads: no magnifier for anyone (ifmagnify.c:323), so
     no off-screen damage;
   - gmCamera_801A31FC at match start: cmSnap_800316B4, two UI GObjs
     (GObj_Create(0xE, 0x10, 0) and IfCamera_Top's) and the SIS texts,
     which a cold start's GObj addresses must include;
   - each frame gmCamera_801A3098 / 30E4 read port 4's pad (Z: snapshot,
     pausing through gm_801A4634(3); B: UI toggle) and only change what is
     drawn otherwise: guard any port-4 input. Start from any port pauses
     (gm_CameraModeVSGetPauser).
2. **Coin**, the guarded corners: it_802E6658 removing the last coin when
   58 are out (random two-player play never gets there; a directed witness
   needs much more damage in 280 frames, or four players);
   a container's coins (it_8026F8B4, items on: Coin is refused with items
   in `MatchConfig` and at construction); the spill of every other
   Fighter_UnkTakeDamage_8006CC30 caller (`refuse_unwitnessed_coin_spill`:
   CaptureCut, Thrown's ftCo_800DE854, a launched captor's ftCo_800DE2F0,
   the barrel, DK's cargo), each needing its dmg.x1854 position; a Coin
   team battle (ranks by team, untested).
3. **Bonus** matches (`gm_8016B0B4` is false; their own scoring).
