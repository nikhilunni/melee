# Handoff: the guarded paths app players hit first (after wave 27)

Wave 27 finished wave 26's branch: the Party Ball, 3-4 player CPUs and
random items on every ported stage but Mushroom Kingdom now ship, with
witnesses in `m5_gate` (`WIDENED_LAYOUT_BOUNDARIES`,
`WIDENED_LAYOUT_MATCHES`). Details: `docs/PORT_NOTES/WAVE27_ITEMS_EVERYWHERE.md`.

## Open divergences (found, not registered)

- `w27g_FlatZone_items_all_e6cc80d32_p1`, tick 1200: a walking Goomba's
  hitbox reaches Fox as his side taunt (AppealSR) ends into Turn; the port
  lands the hit at 1200, retail only at 1209. The Goomba's position and
  both hitbox capsules equal retail's raw Item bytes through 1200; retail's
  frame-end hurt capsules overlap the hitbox at 1201 (capsule 7, bone 26)
  without a hit. Suspect the fighter's hurt capsules at hit detection
  after an animation-end motion change, not the Goomba. Record with bones
  and compare with `bones-diff` next.
- `w27s_bf_items_all_fox_cpu9marth_cpu9falco_cpu9peach_e89a89d0e_p2`,
  tick 1422: a thrown Barrel in hitlag keeps pushing a resting Sword. The
  Barrel's push box (xBEC) shrinks from 7.0273 to 6.7952 the tick after
  its hit in retail (its model's rotation reaches it_80271A58 a proc
  later); the port keeps the earlier box for the whole hitlag.

## Still guarded (explorer scan of 5040 cases over every widened layout)

- Mushroom Kingdom with items: the `?` blocks (grinishie1.c:465; the
  boundary's creation draws and the hatena ground objects).
- Octarock (12) and Ottosea (22) out of containers: constructors
  (it_8026F6BC) and lifecycles (itoctarock.c, itoctarockstone.c,
  itoldottosea.c, itwhitebea.c; the Ottosea pushes a Freezie only with the
  Freezie switched on).
- A CPU attacking a stage enemy (ftCo_800B5AB0, 8 cases with Goombas).
- Barrel ownership by an owned or non-resting barrel (it_8027236C, 5).
- Peach's turnip pull with a full hand (ftPe_8011D518, 3); doAirEnd0 with
  an item (3); LightGet leaving the ground (2); a fast tool hit (1);
  Lift's Fall with an empty hand (1); a new item's bump on Yoshi's
  Island's blocks during its spawn pass (1).
- 3-4 player CPUs elsewhere than FD and Battlefield: a CPU's tumble reads
  its Fighter's address, which `initial_state::heap` knows only there
  (and only for the recorded rosters).
- Kongo Jungle's own guards (audio envelope, log Sink, path choice) occur
  with or without items.
