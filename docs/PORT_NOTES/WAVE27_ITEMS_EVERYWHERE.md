# Wave 27: items on every stage, 3-4 player CPUs, the Party Ball, the Goomba

Continues `docs/handoffs/app-guarded-paths.md` (wave 26).

## What the app may start now

- `Stage::items_supported`: 2-4 players on every ported stage but Mushroom
  Kingdom (its `?` blocks hand out items, grinishie1.c:465, unported).
  Two-player boundaries on each stage (`w26_start_<Stage>_items_all`,
  `w27_start_{YoshisIsland,Onett}_items_all`), three and four players on
  FD, BF (`w26_start_{fd,bf}_items_all_fox_marth_falco[_peach]`) and Kongo
  Jungle (`w27_start_KongoJungle_items_all_fox_marth_falco_peach`).
- `Stage::cpu_max_players`: 4 on FD and Battlefield, 2 elsewhere. A CPU's
  tumble reads its Fighter's address (`initial_state::heap`), known for
  three and four players only there. CPUs with items:
  `w27_start_fd_items_all_fox_cpu9marth`,
  `w27_start_bf_items_all_fox_cpu9marth_cpu9falco_cpu9peach`.

## Uninitialised memory as a scenario input

itHarisen_UnkMotion8_Coll (0x80292AE8) lands a dropped Fan on a nonzero
xD4C and removes it on zero, and nothing writes xD4C for the Fan: the word
is whatever the heap held where HSD_ObjAlloc put the Item (fresh blocks
come from HSD_MemAlloc, so retail runs from one boundary differ). The pool
gives each new Fan the next word of `Setup::heap_ammo_words`; a scenario
names its recording's words (`heap_ammo_words`, `fan_heap_words.py`), and
the port assumes nonzero past the list (three Fans in four in the
recordings). The explorer and app play with that default.

## Explorer method

Branches ported without a witness were found with temporary probes
(`unimplemented!` behind `MELEE_PROBE`) in a port-only explorer scan over
every layout; the shortest cases per probe were bridged from the unprobed
scan and recorded to the probe tick plus 240 (`w27p_*`). Faults fixed in
this wave were witnessed the same way from the scans that hit them
(`w27q_*`, `w27g_*` for the Goomba), and one clean case per layout was
recorded to 1500 ticks (`w27s_*`).

## Ported

- Party Ball whole lifecycle (wave 26) plus the thrown ball at a wall or
  ceiling and the rising ball at a ceiling.
- Goomba out of a crate (itoldkuri.c): stand, paced walks turning at the
  cycle's end, at walls and when an item pushes it (xDC0 bit 2,
  it_802750E8), falls, carry/drop/throw, stomps (it_802D8098), reels,
  tumbles and screen flights.
- Box: thrown into a wall (breaks), under a ceiling (stops rising), dropped
  hard (breaks). Bob-omb walking/turning/standing off a floor and its
  slope slide; Capsule, Box and Mr. Saturn slides and Mr. Saturn's falls.
- Items pressed between two walls (it_80276D9C's box squeeze); items
  bumping Yoshi's Island's blocks (it_8026B718, xCBC only).
- Held items through FlyReflectWall/Ceil, DownReflect and BarrelWait.
- A right-facing ReDead's birth; the reflected, absorbed and
  shield-bounced callbacks get the RNG (a reflected container opens).
- CPU: ftCo_800A53DC's standings ranking, item throws (ftCo_800AD7FC),
  swing weapons (ftCo_800B52AC, Peach's parasol included), a held
  Bob-omb's fuse (it_8026B960 via `ItemLogic::fuse`).

## Still guarded (counts per 5040 explorer cases, scan `g`)

Octarock (12) and Ottosea (22) constructors and lifecycles; the CPU
attacking a stage enemy (ftCo_800B5AB0, 8); barrel ownership by an
owned or non-resting barrel (it_8027236C, 5); Peach's pull with a full
hand (3); doAirEnd0 with an item (3); LightGet leaving the ground (2); a
fast tool hit (1); Lift's Fall with an empty hand (1); a new item's
bump on Yoshi's Island at spawn (1). Kongo Jungle's own guards
(audio envelope, log Sink, path choice) occur with or without items.
