# Wave 27 stages: Brinstar, Venom

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

## Venom (grvenom.c, GrVe.dat, StKind 0x16)

Boundary `start_venom_fox_marth4`. Creation order 4, 0, 5, 9, 7; no setup
draw (the first Arwing's wait is `yakumono_param` x0..x4 = 600..600). Map 5
(the Great Fox) carries collision joints 0 and 1 on its root and bobs with
its looping subtree animations; its proc runs the spawner. Two constant
dynamics winds (lb_80011A50, one per half of the stage).

- The flight (map 7, grVenom_8020454C): one 9999-frame loop whose first
  AObj's frame raises flags at set frames. Frames 3800..6100 forbid passes;
  6810..9200 allow only the near passes 1..4; at 4400 the tunnel floor
  (collision joint 2 on map 9's identity root) comes and at 6100 it goes.
  Map 9's init binds the joint under the map-scale wrapper (0.54) and then
  sets the wrapper's x and y scale to 1: the tunnel's lines stand at the
  file's own coordinates, 1/0.54 of the bind pose (a warm boundary restores
  the wrapper's saved scale).
- The spawner (grVenom_8020362C): while a pass lives the wait is drawn
  again every tick (rand_range(960, 1800)); afterwards it counts down and a
  pass of a new type (1..11, or 1..4 near-only) comes, an Arwing unless
  `HSD_Randf() <= 0.15` and the last pass was not a Wolfen.
- A pass (grVenom_80204F20 and the Star Fox inlines): the path (map 2,
  animation `type - 1`, started on its first proc, the pass ends with it),
  the ship (map 3 Arwing / 6 Wolfen) and for types 1..7 the platform
  (map 10, joint 4 / map 13, joint 3) on the ship's bone 1. A near ship
  banks every 120 ticks (one of four maneuvers on bone 1, grVe_803E5644);
  a far ship (types 8..11) is held to the path's bone 5 (lb_8000C2F8's
  RObjs) and, inside its type's window, fires at a fighter on its side
  every eighth tick with a one-in-eight draw.
- The laser (it-arwinglaser, itarwinglaser.c types 1 and 3): flies
  straight at the target's position in depth too at x4_throw_speed_mul *
  x4 (20 units a tick), tested against joint 0's lines only near the
  stage's plane (|z| < 30), 400 ticks of life, no blast-zone test.
  ArwingLaserAttr x0 points at an ItemAttr of its own (not the article's).

Display only, not ported: lights and fog, map 7's culling and light
restarts, the ships' colour overlay, the sounds, the shadow test. Guarded:
the Star Fox taunt (Fox and Falco on Corneria or Venom, ftfoxappeals.c:55),
the 1P stage's three slots (grVe_804D6A40), Corneria's cannon laser types
0, 2, 4 and 5, a warm boundary with a pass out or inside the tunnel.

## Princess Peach's Castle (grcastle.c)

Creation order 0, 4, 3, 6, no setup draws; the block sets' joints (0..2,
6..14) start disabled. grCastle_801CD37C's lb_80011A50 field (direction
(1, 0, 0), strength 0.5) is steered every tick by grCastle_801D0BBC (one
degree a tick, clamped to 60, the turn flipped on a coin every 30 ticks,
strong gusts toggled every 300) and re-gusted once per flag
(grCastle_801D0D24, four flags): those draws are the stage's every tick. The
two statics (grCs_804D45E0/E4) outlive matches; a warm boundary reads them.
The flags' own dynamics are drawing only.

- The deferred grCastle_801CD9B4 hooks the platforms' floor callbacks; then
  grCastle_801CD610 registers the moat (ftCo_800C06E8 type 6: below 14 *
  scale a falling fighter is lifted 0.1; crossing it splashes), makes the
  switches (maps 20, 19, 18, each its block set 17, 7, 5) and draws the
  switch and Bill countdowns.
- Switches (grCastle_801CF308): a weighted choice (the last one's weight /
  x6) and HSD_Randi(11) of the places on the roof or a platform; a blink
  table read a byte at a time; then a material item (state 1) whose hit
  (yaku.x18) or touch (x1C, it_802E7054) presses it and freezes the fighter
  (ftLib_80086A4C). The press fills the block set: its blocks grow by x48
  (staying hidden while a Bill flies within 40, grCastle_801CDF54), their
  joints enabled, stay `rand_range(x42, x40)`, blink, shrink. A `?` block's
  material item and item launch need items on and fail closed
  (grcastle.c:1608).
- The Bullet Bill (maps 8..16, grCastle_801CE578): any flight but the last
  three; its material item (state 3) and an Auto camera subject; the
  cannon (map 2, four animations, a Loop quake every tick) partway; at the
  flight's end a fuse, then the explosion (map 1, material state 4,
  lb_800119DC). The explosion's frame-zero key (common bank 0xF2) goes
  through efLib_Cb_DPtcl -> efLib_SpawnParticleEffect, which attaches it
  with an AppSRT carrying the root's scale.
- The colour script grMaterial_801C9604 starts on a landed Bill (drawing
  only). The eye never goes below 50 * scale (grCastle_801D0FF0).

## Green Greens (grgreens.c)

Creation order 0, 2, 6, 5, 4. Setup draws 21: grGreens_80214B58 places
eighteen of the thirty block slots (grGr_8049F9E0, a draw each), the first
block wait, then Whispy's side and wait. The blocks and Whispy start when
the countdown ends.

- Blocks (map 6, grGreens_8021479C): two grids of three columns by five
  rows. A falling block (a map-1 or map-3 GObj with a stage material item
  carrying its hurt capsule) picks its column by the weights of the
  columns' heights, stacks, lands on its spot and stitches its lines to
  its landed neighbours (mpLib_800581DC). A hit (yaku.x18, from the item's
  hit proc, item link 14) or a head bump while it falls breaks it: a block
  quakes Small and breaks (efSync 1032, 0x408 -> generator 0x3E AppSRT); a
  bomb's item enters state 1, its blast hitbox placed at once when the
  strike came from link 14 (it_802790C0's s_link > 11), the blast efSync
  1039.
- Whispy (maps 5 and 4): idle animations drawn by HSD_Randi(8), the blow
  (a fighter wind device and a directional field per blowing loop, toward
  the fighters' side) and the apples (it_802EE200: kind and fate drawn,
  HSD_Randi(2) facing). grAnime_801C8138 resets the rest pose
  (lb_8000B804) and evaluates frame zero, and the particles on Whispy's
  JObjs read that pose in the particle proc, so both maps republish after
  his proc.
- Apples (it-whispyapple): the enter state waits for the stage plane
  (pos.z >= 0), then bounces with a drawn sound (it_803F9250[HSD_Randi(3)])
  and rests; ECB rotation follows the spin, whose landing rate is ItCo +74
  (it_8026DC24). A healing apple needs items on and fails closed
  (itwhispyapple.c:110).
