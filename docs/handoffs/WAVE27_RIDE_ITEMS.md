# Wave 27: ridden and worn items (Warp Star, Lip's Stick, Parasol, Flipper, Barrel Cannon, Poké Ball)

The task was these six random items through their whole lifecycles, in that
order. The first three are ported and gated. The last three are not started.

## Ported (exact, registered in `m5_gate` `RANDOM_ITEM_KIND_WITNESSES`)

- **Warp Star** (`it-wstar`, `melee-ft` `warp_star.rs`): pickup, ride, path
  (`it_804D6D00`), blast, landing, KO during the ride (`ftCo_800C48AC`), the
  drop after a hit in LightGet, and the dropped star breaking. Witnesses:
  15 explorer matches, 8 `wstar_ride_*` and `wstar_ko_e68833afa`.
- **Lip's Stick** (`it-sword` `lipstick.rs`, `melee-ft` `flower.rs`): spores,
  swings, the thrown stick's contact spore, the flower (planting, growth,
  damage ticks, mashing, wilting, loss at death). Witnesses:
  `start_fd_items_lipstick` warm and cold, 13 explorer matches.
- **Parasol** (`it-sword` `parasol.rs`, `melee-ft` `parasol.rs`): the item,
  its swings, and the fighter's ItemParasol states opened from Fall, Jump,
  the end of hitstun and special fall. Witnesses: `start_fd_items_parasol`
  warm and cold, 13 explorer matches. Parasol is **not** in
  `PORTED_RANDOM_ITEMS` yet: only one explorer batch has run, so faults may
  remain.

## Open divergence: `wstar_hit_e06ad0b5e` (not registered)

Particle generator 8 (program 431) has the wrong position at tick 646. A
LightGet hit drops the star and it breaks. Retail then reads bone 3's local
SRT (-0, -1.4269, -1). The port keeps the bone's last world matrix because
`joints_dirty` is false on that path. The fix is probably to dirty the joints
(or recompute the free pose) on the drop and break path before the generator
reads the bone. The scenario TOML and traces are in the main checkout.

## Still guarded (`unimplemented!` with the C line)

- it-wstar: EnteredAir and slide (itwstar.c:230/242); `ftCo_800C4858`, which
  drops the star when its rider is hit (the rider is intangible, so the
  explorer has not reached it).
- Lip's Stick and Star Rod empty puff (efAsync 1032); a flower on Pikachu
  (`ftPk_Init_UnkMotionStates1`); a flower kept through a transformation
  (ftcommon.c:1350); the flower's thrown and slide states; a contact spore
  from a stick with no owner.
- Parasol: the PassiveWall hook (ftCo_PassiveWall.c:170); the canopy rotation
  is kept in scratch only and is not applied to bones;
  `ftCommon_8007E83C` with no special parasol may still fire.

## Not started

- **Flipper** (itflipper.c): its bounce reads the victim's and attacker's
  fighter position and velocity (xCF4/xCEC fighter gobj, `ftLib_800866DC`,
  `ftLib_80086BEC`, xC38/xCD4, xCB4/xCE0). `ItemEventContext` must carry them
  first.
- **Barrel Cannon** (TaruCann, ittarucann.c) and the **Poké Ball** with its
  Pokémon (itmball.c and the `itpokemon*` files): not begun.

Boundaries `start_fd_items_wstar`, `start_fd_items_lipstick` and
`start_fd_items_parasol` exist (make_boundary.py `--items <Kind>
--no-containers`). Copy their pattern for the next three items.
