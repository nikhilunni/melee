# Handoff: the guarded paths app players hit first (after wave 27)

Wave 27 finished wave 26's branch: the Party Ball, 3-4 player CPUs and
random items on every ported stage but Mushroom Kingdom now ship, with
witnesses in `m5_gate` (`WIDENED_LAYOUT_BOUNDARIES`,
`WIDENED_LAYOUT_MATCHES`). Details: `docs/PORT_NOTES/WAVE27_ITEMS_EVERYWHERE.md`.

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
