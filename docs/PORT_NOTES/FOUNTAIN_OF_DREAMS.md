# Fountain of Dreams (grizumi.c)

Stage bring-up for Fox vs Marth (STAGE-FOD). Boundary `start_fod_fox_marth4`
(make_boundary.py; seed 594180171). Gates: `start_fod_fox_marth4`, its cold
scenario, `start_fod_fox`, `idle_fod_fox` (600 ticks each, particle states
and particle RNG sites exact), `stage_fod_idle_fox_marth4` (6000 ticks, both
platforms through steps, a submerge and the resurfacing, fighters riding
them) and six whole explorer matches.

## Ground

- grIzumi_801CBB88 creates maps 0, 1, 3. Map 3's on_init (grIzumi_801CBE64)
  binds the three collision joints of grIz_803E0D60 (joint 0/1/2 follow map
  3's descendants 1/2/3) from the unanimated pose, evaluates map 3's frame
  zero, spawns bank-30 generators 30004 and 30006 through grLib_801C96F8
  (new AppSRT, status 1, map scale 0.75), creates the star (map -1, no
  animation), map 2 and the two map-4 platforms. Scheduler keys: star 13,
  right platform 14 (`melee_gr::izumi::procs`).
- Setup draws: two (each platform's first controller step draws its wait).
- grIzumi_801CC358 (`melee_gr::izumi::Platform::tick`): Arrived -> Waiting
  (rand_range(fctiwz x3C, fctiwz x38)), decision (weights 4 submerge / 10
  wait / 8 step), Moving (0.15 up, 0.1 down, landing exactly on the target),
  Sunk (hide, collision JObj to origin - 1 in double), Submerged (resurface
  to the rest height 25). Every step ends with mpLib_80055E9C on its joint.
  Fused sites: 0x801CC4C4 and 0x801CC8AC (fmadds); 0x801CC87C is a double
  fdiv; 0x801CCD38 fsubs then fdivs.
- The map-3 animation's lights are animated LObjs (presentation only; not
  loaded).

## Savestate import

- Collision joints are transformed from the saved JObj matrices; the
  vertices' previous positions are imported (the setup's last transform
  moved them), then every vertex is checked against `groundCollVtx`.
- The platforms' animations are requested but not evaluated at the match
  start (AOBJ_FIRST_PLAY).
- Initial generators own AppSRTs: `particles::restore` now rebuilds them.
- Mid-tick cursors key the star and the second platform by Ground map id
  and collision joint.

## Terrain

FoD's list (mpLib_803BDFD8) swaps material 10, the fountain's floor, for
mpLib_803BD7A0: footsteps, landings and down-bounds spawn stage-bank effects
30007/30008/30011 (ftCo_8009F834 block_11: zero-range jitter, efAsync kind 2
from bank 30). `melee_mp::terrain_effects`; the effect flush takes the
stage bank.

## Explorer

60 cases from `start_fod_fox_marth4` (20 seeds): two faults, both fighter
scratch inheritance (ftFx_SpecialLw turnFrames, ftMars specials mv+4).
