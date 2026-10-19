# World-matrix leads from bones-diff (wave 12)

`melee-sim bones-diff` compares each joint's world matrix where the port holds
it clean (fa132102). Dumps recorded before `dirty_bones` existed could not tell
a real fault from a joint only retail held dirty. Nine scenarios passed the
local-SRT check but showed world mismatches with those older dumps. Each was
re-recorded with
`record_many.py scenarios/<name>.toml --jobs 2 -- --bones N`, N being the
scenario's `frames`. (The sudden-death dumps used to be 1545 ticks long, past
the end of their 1471-tick tick traces. `record.py` now refuses that: the bone
pass's seed check fails at the tick trace's last record.)

| Scenario | Old lead | With `dirty_bones` |
|---|---|---|
| `sheikzelda_chain_hit_fd_sheik` | p0 bone 9 1 ulp, tick 263 | retail held bones 9 and 15 dirty at 263 (the chain arm's guard pose moved them after their last build); bones match |
| `firefox_charge_hit_fd_marth` | bone 8, tick 157 | dirty only in retail; bones match |
| `capture_jump_up_release_fd_marthfox_candidate` | Marth bone 8, ~128 | dirty only in retail; bones match |
| `capture_jump_xy_latch_fd_marthfox_candidate` | Marth bone 8, ~128 | dirty only in retail; bones match |
| `grabair_bf_marth_fox4_e75fb4a9a_bones` | Marth bone 8 (and tick 2238) | dirty only in retail; bones match |
| `sudden_death_pummelcaptor_bomb_fd_marth` | Marth bone 8, ~128 | dirty only in retail; bones match |
| `sudden_death_bombchain_fd_marth` | root translation | dirty only in retail (a dead fighter's root is never rebuilt); bones match |
| `sudden_death_thrownbhold_bomb_fd_marth` | root translation | as bombchain; bones match |
| `sudden_death_counterbomb_fd_marth` | bone 88 | **real fault**: see below |

Eight leads were stale caches: joints the port had rebuilt and retail had not.
Neither side's game state depends on such a joint until something asks for its
matrix, and then both rebuild it.

## The real fault: gust order (counterbomb)

With the new dump, the first difference is local, not world: Marth's
`bone[52].rotate` is 20 ulps off at tick 1294. That bone is the third joint of
the cape chain 50-53. Ticks 1289-1293 match, while four radial force fields act
on the chain.

The solver keeps state that no SRT shows: each node's position (`+2C`),
velocity axis (`+38`) and angular velocity (`+44`). The bone snippet can now
dump it (`MELEE_BONES_DYNAMICS=1`, docs/DOLPHIN_RUN.md). A probe copy of the
scenario (`probe_w12_counterbomb_dyn`, ticks 1270-1296) showed every node
bit-exact through 1293. At 1294, node 52's axis and angular velocity were
1-3 ulps off, so the fault came from that single solver step.

The solver asm (`lb_8001044C`, `lb_800101C8`, `lb_800100B0`, `lb_800115F4`)
matched the port, including every FMA site. What remained was the order in
which `lb_800101C8` sums the four fields. Trying all 24 orders, only the ones
that swap the two newest fields stay exact past 1294. Those two fields were
both inserted at tick 1278:

- Counter's hit impulse (`ftMs_SpecialLw`, `lb_800119DC(&hip, 120, 0.9, 0.02,
  M_PI / 3)`), made *before* `Fighter_ChangeMotionState(372)`;
- SpecialAirLwHit's script wind (ftAction_80073118 -> ftCo_8009E714 ->
  `lb_800119DC`), made during that motion change.

The pool prepends (`lb_800100B0`), so retail's list is [wind, counter]. The
port kept two queues and drained the winds first, which gave [counter, wind].
Floating-point addition is not associative, so the sum came out a few ulps
apart. The difference first reached an SRT word 16 ticks after the insertion,
once the counter's field had left its `cos(5 pi) = -1` zero.

Fix: `melee_ft::fighter::commands::GustRequests` keeps one queue of
`GustRequest::{Wind, Impulse}` in issue order, which melee-lib drains into the
pool. The character crates push impulses as before (`commands.gusts.push`).
`counter_gusts_match_retail_bones` (m5_gate) holds the scenario's bones exact.

The probe's traces (`harness/traces/probe_w12_counterbomb_dyn.*`) were scratch
diagnostics, not a registered scenario, and were deleted on 2026-10-19;
record the probe again with `MELEE_BONES_DYNAMICS=1` to reproduce them.
