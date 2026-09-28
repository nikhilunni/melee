# Yoshi's Story: Shy Guys and terrain footsteps

Witnesses (recorded 2026-09-27, machine-local traces), all from the
`start_ys_fox_marth4` boundary (Fox at -42, Marth at +42 on the side
platforms):

- `start_ys_fox_marth4` / `_cold` (600 ticks): the first Shy Guy group
  spawns at tick 121 and walks across; registered in `harness/boundaries.toml`.
- `stage_ys_shyguy_firefox` (660): a Fire Fox knocks a Shy Guy away (state 2,
  fire hit effect, a Randi(3) spin each frame, removed by the bottom blast
  zone); Fox then walks the stage's terrain.
- `stage_ys_shyguy_nair` (780): a late nair stuns one (state 3, it_8027B798's
  launch) and it flees at double gait rate (state 4) out of the left side.
- `stage_ys_shyguy_marth` (620): Marth's up-air knocks one away.

Gated by `m5_gate::yoshis_story_shy_guys_match_retail`.

## Spawner (grStory_801E3418)

Map 3's proc (grStory_801E3334) runs the spawner before the map's collision
update, then lb_800115F4. While any `It_Kind_Heiho` item lives nothing
happens; otherwise the 120-frame timer runs out and a group of one or three to
five spawns at one of six heights, each through it_8027B5B0: ftLib_800864A8's
facing (a Randi(2) draw on a tie) before the item exists, then
`Item_80268B18`, whose spawned callback (it_802D8688) clears xDCC b3 so the
blast zones do not remove a Shy Guy entering from off screen. The jitter draw
for the next Shy Guy follows each spawn.

## The Shy Guy (itheiho.c)

The Article comes from the stage file's `itemdata` table (Ground_801C0800 ->
it_8026B40C). Its walk is authored as joint 1's Y translation, read after
every animation step and zeroed (itUpdateVelocityFromBone). The joint's AObj
restarts at frame zero with each state change, so `melee_it::bone_motion`
samples each article state's readings at load for the three rate schedules
the code produces (rate 1; rate 2 from the first step when the flee restarts
inside its animation callback; rate 2 after one step when it starts from a
physics or collision callback).

`heiho.x2C` (the late-flight climb, `x2C > 960`) is only ever cleared, so the
climb never runs. Food (it_8028FAF4) needs items switched on.

Hits: hold kind 4 items show the hit element's effect (it_80270E30):
normal as the plain spark, fire and electric as positional generators, slash
as model 8 with a random Z turn (ported, not yet witnessed; Marth's up-air is
not a slash hit).

## Terrain footsteps (ftAction_80072CD8)

Opcode 54 footsteps consult the floor's material row (ft_80084BFC ->
mpLib_80056A1C/A54, the shared table every supported stage uses): a
replacement sound, whether the command's own sound still plays, and a foot
effect (none in the shared rows). Each sound goes through ft_PlaySFX, which
draws HSD_Randi(200) for a pitch when the id is 332..=370, so the footstep
sounds are part of the RNG stream. Opcode 17 sounds never consult the terrain.

## Next

Walking onto a slope reaches `ft_0899.c:109-232` (the ground pose's body
tilt on a non-flat floor): the explorer from `start_ys_fox_marth4` stops
there in 34 of 36 cases.
