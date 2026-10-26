# Brinstar (wave 27)

Wave 27's stage work. Each stage has start boundaries (warm and cold gated in
`m4_gate`) and explorer matches in `CORPUS_V3_MATCHES`.

## Brinstar (grzebes.c, GrZe.dat, StKind 0x06)

Boundaries `start_brin_fox_marth4` and `start_brin_falco_peach4`. Creation
order 0, 1, 6 (whose creation makes 7, then the bubbles' map 3 models), 4,
8; setup draws: map 4's `HSD_Randi(600)`, the acid's first delay
(`rand_range(800, 0)`) and height (`HSD_Randf`). The bubbles settle for a
hundred ticks at creation (no draws). Collision joints come from
grZe_803E1A10 (1, 4, 3 on map 6's bones 21, 14, 1; 2, 5 on map 7's 6, 1);
joint 0 is the chain's line, whose six vertices map 6's proc sets.

- The acid (map 8, grZebes_801D99E0): waits a row's delay (a Loop quake for
  its last 120 ticks), draws a height in the table's range, accelerates
  under a top speed that brakes in time (inline sqrtf, 0x801D9C6C fnmsub),
  brakes, takes the next row. Its surface, `-55 + level`, is the stage's
  floor (stage_info.x724, Ground_801C438C): the camera frames above it and
  keeps the eye 5 over it (`scene_stage::live_camera`, grZebes_801DCCC8).
- The acid's device (ftCo_800C07F8 type 3, from the stage's start): in
  Fighter_8006CB94 after the item hits, a fighter below the surface takes the
  record at yakumono_param +0x2C (14%, 90 degrees, fire) as a damage-log
  entry of kind 3 (`HitSource::Stage`, ftCo_800C08A0: no hit effect, its own
  facing, no credit) when ftColl_GetHitStatus is 0 and bury_timer_2 is out
  (PlCo +5E4 after each hit). One coming from above (its previous position
  moved with the surface) splashes: a map 5 model whose frame-zero key
  spawns its generator, at most ten, each following the surface until its
  animation ends.
- The plates (grZebes_801DA528, maps 6 and 7): hits on the plate's material
  item add their damage and set the wobble to the launch's x (it_8027B798);
  the damage heals a point every 20 ticks and breaks the plate past 30; the
  plate's collision bone plays animation 1 from frame 30 as it breaks,
  2 as it grows back after a 1200-1800 tick wait.
- The bubbles (grZe_8049F170): grZebes_801DB3CC's forces (chain links pull
  toward their neighbours, pairs push apart or attract, the loose ones stay
  inside the parts' bounds), the capsule of their shared material item
  (grZebes_801DBB60: any hit bursts the first held link), a loose bubble every
  20-50 ticks while fewer than fifteen are out, a burst link replaced by the
  nearest loose bubble or the chain dropping. The chain's line covers the
  held bubbles' tops. When the chain is gone the top platform (map 6's bone
  14, map 7's bone 1, their own tracks: grAnime_801C8098) sinks, waits 600,
  rises and grows a new chain from both anchors every 15 ticks.
- The pulse (grZebes_801DAA08, every 200-300 ticks): map 2's model hung on a
  random held bubble, which may not pulse again until its material animation
  ends (`zebes::pulse_ticks`, from map 2's material clock).
- Map 6's pulse clock (xF8) is never written at creation:
  `alloc_user_data_ground` returns the block uncleared. Both retail
  boundaries (different rosters, from the menus) hold -849, which cold
  construction uses, so the first tick pulses.
- Display only: the fog and light colours that follow the acid
  (grZebes_801DA0C4, 801DA254), map 1's shakes (ported, no gameplay), the
  bubbles' and pulses' models (not drawn).

Guarded: a chain line column no held bubble reaches (grzebes.c:613-632
leaves its x to the stack; 2400 explorer cases never reached it); warm
boundaries inside a pulse, a broken plate, the sunk platform or with
splashes out.

Map 4 (grZebes_801D95B8) switches animations with grAnime_801C8138, which
ends with HSD_JObjAnimAll: frame zero is evaluated at the switch, so its end
(and the next `HSD_Randi(600)` wait) comes on the right tick.

Witnesses: 17 explorer matches in `CORPUS_V3_MATCHES` (632-6001 ticks, every
key and particle exact), found by hunting the port for each event.
