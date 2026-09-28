# Dream Land: Fox/Marth boundary, Whispy's wind and the Bronto Burt flyby

Stage bring-up for `start_dl_fox_marth4` (2026-09-27). It extends
[DREAM_LAND.md](../DREAM_LAND.md), whose 600-tick scenes never reached a
flyby or let a moving fighter meet the wind.

## Boundary

`make_boundary.py --stage DreamLand --players Fox Marth --stocks 4` made
`start_dl_fox_marth4` (seed 1783163052). Its cold scenario gated exactly
before any change, so it is registered in `harness/boundaries.toml`.

## What was missing

1. **Flyby** (`grOldPupupu_80210D10`, groldpupupu.c:351-436). The map-8
   timer expired near tick 3700 and hit `unimplemented!`. It now draws
   `HSD_Randi(10)` for one of ten flight paths (grOp_803E67B0), `HSD_Randi(5)`
   for a group of three on paths that allow it, `HSD_Randf` after
   Camera_800307D0 (which draws nothing), then the next delay. The map-2
   models have no DPtcl keys (checked over the whole animation), and their
   proc (grOldPupupu_80210C34) only deletes them, so the scene keeps the
   placement as data (`melee_gr::pupupu::flyby::Flyby`). Placement arithmetic
   follows the asm (fnmsubs at 0x80210E04/E50, fmsubs 0x80210E90, fmadds
   0x80210EA8). Presentation does not draw the Bronto Burts yet.
2. **Wind sample point.** ftColl_GetWindOffsetVec (0x8007B924) runs inside
   Fighter_procUpdate after the velocity integration, so fn_802112F4 sees
   the post-velocity `cur_pos`, and the offset is added after the
   moving-floor term. The scene used the pre-update position, one frame
   early for a fighter crossing the gust rectangle (first seen as a 0.2
   x error on a jump through y = 40). Fighters now receive a
   `melee_gr::wind::Wind` and sample it in `integrate_environment`.
3. **Dynamics gusts** (lb_80011A50). Every tenth wind tick Whispy adds a
   directional field (priority 0, 15 pool ticks, strength 0.5) over the
   same rectangle. Fox's tail chain carries a hurtbox (joint 18), so the
   gust changed which Marth jab hitbox connected. The field pool
   (`melee_lb::radial_force`) now keeps both kinds with lb_800100B0's
   priority rule, and Dream Land ticks it from grOldPupupu_80210BC0 (map 5);
   it was never aged on this stage before.
4. **Wind state** (lb_804D63B4 via lb_80011ABC). lb_800115F4 sums
   directional strength before decay (`> 0.1`, double constant) into
   started/blowing/stopped/calm. Marth and Roy (`CommonBehavior::
   stage_wind_dynamics`) react: ftCo_8009E614 hands every chain to the solver
   and reattaches the motion without blend on the first windy tick, and
   restores the motion's ownership (ftCo_8009E4A8, per-set
   ftAnim_8006EED4) on the first calm tick; ftCo_8009E7B4 selects "all
   dynamic" for motion entries while it blows. Bones match retail across
   whole wind cycles (`bones-diff` on `stage_dl_windright_fox_marth4` and
   `corpus_v3_dl_fox_marth4_edb2b114a_p2`, ticks 100 onward).
5. **Loop quake** (Camera_RequestQuake(QuakeKind_Loop), grLib_801C9BC8).
   Requested on every wind tick; one looping quake model plays at s_link 1
   until Camera_UpdateQuakes' countdown ends it.

## Witnesses

| Scenario | Ticks | Covers |
|---|---:|---|
| `stage_dl_idle_fox_marth4` | 5000 | Idle; left gust pushes Marth off the left platform; tie vote; flyby draws |
| `stage_dl_windright_fox_marth4` | 2400 | Both walk right; Whispy turns right; right gust pushes both to the edge (Teeter) |
| `corpus_v3_dl_fox_marth4_edb2b114a_p2` | 2534 | Jump through the gust's top edge; tail-hurtbox hit |
| `corpus_v3_dl_fox_marth4_e75fb4a9a_p0` | 6001 | Full explorer match |

Nine bridged explorer samples (seeds 75fb4a9a, db2b114a, f89b3e70,
005a4f43, 19f8579b, 0dee256e, c3145eb3, 726cfdde, eda0d0fc) gate exactly;
75 explorer cases raised no port fault.

## Not modelled

Bronto Burt rendering; Whispy's sound requests; the frozen-fighter flag
(x2227_b6) in the Marth wind response; a mid-wind savestate import.
