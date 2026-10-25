# Hyrule Temple, Fourside, Brinstar Depths

Wave 26. Three Versus stages, each with a Fox/Marth start boundary
(`start_ht_fox_marth4`, `start_fs_fox_marth4`, `start_bd_fox_marth4`, warm
and cold gated in `m4_gate`) and explorer matches in `CORPUS_V3_MATCHES`.

## Shared changes

- The dynamics field pool (lb_804D63B0, lb_00F9.c) is match state:
  `InitialState::radial_forces` carries it into the runtime. A cold setup
  seeds it from the stage (Hyrule Temple's permanent field); a warm
  boundary imports the active list and the wind state (lb_804D63B4).
- Ground_801C24F8's music rules 3 and 4 test one character's unlock bit
  (Marth, Young Link: lbl_803B78C8 indices 2 and 6) before their
  HSD_Randi(100). The pending music carries the save data's unlock mask.
- A stage whose StageData lists static collision joints
  (`grFs_803E3CE8`, `grKr_803E4C78`; Ground_801C2ED0 reads them after the
  archive's) keeps them as `procs::*_BINDINGS` and binds them in its own
  creation, before any animation.
- `BackgroundAnimation::request_frame` (grAnime_801C7B24) and
  `subtree_animation_ended` (grAnime_801C83D0 from a descendant).

## Hyrule Temple (grshrine.c)

Three static maps and no draws. grShrine_80201C64's
`lb_80011A50(&{0.5,0,0}, -1, 0.5, 0, M_PI/3, -10000, 10000, 10000, -10000)`
never expires, so lb_800115F4 reports wind from the first tick and Marth's
cape belongs to the solver for the whole match.

## Fourside (grfourside.c)

Creation order 0, 4, 6, 2, 1, 3, 5; two setup draws (the crane's and the
helicopter's waits). The three controllers read each other's Grounds:
the helicopter waits while the UFO is out over buildings 0 or 1, and the
UFO's first two buildings wait while the helicopter is busy
(grFourside_801F3F10). The UFO's odds come from collision joint 0's
callback (fighters on the roof, counted between UFO steps) or from
`ufo_challenge` failed waits. While it arrives it is a camera subject
(Camera_80029020); a subject that would follow a framed item's fails
closed (the subject list's order across items is not modelled).

Display-only: the render callback fn_801F3F74 turns background joints with
the camera (none carries collision), Ground_801C4E70's fog positions, and
the sounds.

## Brinstar Depths (grkraid.c)

Creation order 0, 2, 3, 4, 1; one setup draw (Kraid's wait). Kraid rises
at one of six places (the outermost two after a swipe missed, any other
place after one landed), roars (HSD_Randi(3)) and swipes; a hand landing
in its window turns map 3 (all six collision joints on bone 12) at a
speed from the hand's distance to the middle, braking after
`rand_range(map_time_max, map_time_min)` ticks, with a Loop quake every
turning tick and a Medium one at the hand.

grKraid_801FE2D0, map 1's proc, is debug code left in retail: the second
port's A press (HSD_PadMasterStatus[1].trigger) steps map 1 through its
seven animations, frame-zero particle keys and all. The port computes that
trigger from the second port's pad and keeps the counter (grKr_804D6A0C)
and the last buttons as stage state; warm boundaries import both.

Warm boundaries for all three stages import their controllers at the
creation phases only; a boundary inside a moving phase fails closed.

## Yoshi's Island (gryorster.c)

Maps 0 and 1, no draws. Map 1 binds its archive joints from the bind pose,
plays animation 4 and animation 3 on bones 23 and 26 (a tick behind, so a
warm boundary replays the clocks rather than restoring one). The nine
blocks (bones 10..18, collision joints 1..9) are created by the deferred
grYorster_80202160 when the countdown releases the stage (Ground_801C0FB8
runs on_start, the deferred list, then Ground_801C0C2C's GObj): one stage
material item each, with grMaterial_801C8DE0's radius-5 capsule.

- A hit on a block's item reaches grYorster_80202428 (yaku.x18) once the
  item's DmgReceived ran: the attacking fighter gets dmg.x1958 (a freeze
  request ProcessHit turns into dmg.x1954, `CombatState::freeze_request`),
  the block gathers the damage.
- A bump from below reaches grYorster_802024F0 through the joint's ceiling
  callback (cb_1). The port records ceiling calls (`record_ceiling_events`)
  and applies them when the collider's proc returns, because the callback
  reads the fighter's `pos_delta.y`. An item's bump past the threshold
  (it_8026B718) fails closed.
- grLib_801C9EE8's PointInsideColl (fmadds halves) decides whether a
  block may come back.

Warm boundaries are imported only before the stage starts (xC4 = 1).

## Mushroom Kingdom (grinishie1.c)

Maps 0 and 3; with items off nothing draws at creation (the `?` blocks of
grInishie1_801FAD84 and grInishie1_801FB3F0 need items on and stay
fail-closed). Map 3 binds the archive's joints and grI1_803E48C8's
(joints 13..19), makes nineteen brick material items at creation (capsule
(0,-1,0)-(0,1,0), radius 6) and records the bricks' ceiling calls and the
lifts' floor calls.

- A brick breaks on any hit on its item (fn_801FBF6C) or a ceiling contact
  faster than 0.7 (fn_801FBEB8, compared in double): wait drawn by
  `rand_range(400, 500)`, JObj hidden (its joint drops out at the next
  Ground_801C2FE0), dust 0x442 (efLib_CreateGenerator 0x1DC) and a Small
  quake. It returns when nothing is within 15 of it (grLib_801C9EE8), then
  flickers. Phases 3 and 4 of grInishie1_801FB3F0 have no writer.
- The lifts (joints 0x14, 0x15) sum their callers' weights (fn_801FC9AC:
  `x50 * 8` standing, 0x801FC9F4 fmadds), tip (0x801FC230 fmadds), fall
  past the blast zone at lift_max_offset, stay away 180 ticks and settle.
- Generic fixes the stage exposed: a fighter KO'd standing on a falling
  lift rides it (procUpdate's mpGetSpeed in the dead states); a fighter in
  x2219_b1 is not pushed (ftCommon_8007E0E4); nothing is magnified before
  the HUD shows (ifMagnify_IsHUDVisible).
- Open: corpus_v3_mk_fox_marth4_e19f8579b_p0 diverges at tick 238 by
  0.0015 in y as Fox's Illusion carries him off a descending lift (pos_delta
  and the lift match retail); a moving-floor collision case, not stage code.
