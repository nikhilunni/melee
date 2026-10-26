# Big Blue (grbigblue.c): what remains

Wave 27 ported Big Blue (`melee_gr::bigblue`, `scene_stage/bigblue.rs`,
notes in docs/PORT_NOTES/WAVE27_FZERO_PURA.md). It is in `Stage::PORTED`;
items and CPUs stay off there. Witnesses gate exact: the start boundaries
`start_bb_fox_marth4` and `start_bb_fox_marth_time8` (warm and cold),
`bb27_idle_fox_marth4` and nine `corpus_v3_bb_fox_marth4_*` explorer cases.

## Guarded branches (unimplemented!, C line in the message)

- The Falcon Flyer coming back (grbigblue.c:1607). The flyers call it
  from the seventh launch on, after ~5000 ticks; no explorer case got
  there. The removed code is the `if out <= 1 { ... }` body of
  801E93D8's case 0 (spawn at blast_left - 50 on the road, the
  801E8794/801EAB50 leftward checks, the highest car, 30 over the camera
  top when blocked). `harness/scenarios/bb27_long_idle_time8.toml` (26000
  ticks idle from the registered timed boundary) would witness it; its
  first recording failed on a full disk (a partial
  `traces/bb27_long_idle_time8.tick.raw.jsonl` is left) and needs
  re-recording with `record_many --jobs 1`.
- The route platform overlapping a flyer's spawn (2190): recorded,
  ungated: `corpus_v3_bb_fox_marth4_e1b05b110_p0` and
  `corpus_v3_bb_fox_marth4_ee133b82f_p1` (traces in the main checkout,
  TOMLs in harness/scenarios). Restore `result = true;` in
  `Flyers::route_near`, gate both, register them.
- The road rising above x8 (2488), turning back up (2822); a car-bone
  rank running dry (3173, 3192); a car below -2000 (3924); a flyer past
  +-65 degrees (1076, 1079). Not reached in 360 explorer cases.

## Other

- Two explorer cases stop at ftCo_800DC920 (life.rs, "a fighter dying
  pinned to its captor is moved by its release"): a fighter path, not the
  stage's (`corpus_v3_bb_fox_marth4_e19f8579b_p1`, `e00f160ee_p1`,
  recorded).
- `start_bb_fox_marth99` (99 stocks) was made but the port refuses more
  than five stocks (ifStock_802F8298); its savestate stays unregistered.
- The full sweep against the baseline and replay-batch were not run
  before the wave closed.
