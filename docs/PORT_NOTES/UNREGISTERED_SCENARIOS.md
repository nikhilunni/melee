# Untracked local scenarios: what each one is (2026-10-12)

The main checkout's `harness/scenarios` held 231 scenario files that were not
in git when this pass began (`main` at a62763c6). This is the account of
each, re-checked on `main` at edcb8b8b. Nothing was deleted, moved or
re-recorded.

How each was checked: `melee-sim gate` over its full `frames` (a recording
with `gate = "<twin>"` loads as its cold twin, so a bridge is one check), then
`melee-sim particle-sites --to <frames - 1>` against its RNG ledger
(`ledger600` for a start boundary), `melee-sim triage` where the gate failed,
and `harness/replay_vs_trace.py` where it matters whether retail in Dolphin
played the console's game. Ticks below are trace ticks (replay frame N is
trace tick N + 1).

| Class | Scenario files |
|---|---|
| Registered in `m5_gate.rs` by this pass (37 single scenes, 26 bridges with their cold twins) | 89 |
| Boundaries committed beside the bridges that start from them (25, each with its cold twin) | 50 |
| Fixed, registered and committed on `main` by other agents during the pass | 21 |
| Exact, not registered: duplicate or superseded | 23 |
| Exact on every key, not registrable (no ledger; particle order) | 4 |
| Not exact: known open faults (one diagnosed here: the Luigi bridge) | 8 |
| Not exact: assertions with no owner | 4 |
| Not exact: wrong recording | 4 |
| No retail trace when checked | 8 |
| Boundaries left local (no registered scenario starts from them) | 20 |

Written by other agents while the pass ran and not classified here: fifteen
`kirby_cutter_*` scenes (the Kirby agent's; recorded by the end, and stopping
at `unimplemented!` "ftKb_SpecialHi_Enter / ftKb_SpecialAirHi_Enter
(ftkirbyspecialhi.c): Final Cutter" on `main`) and the boundaries
`start_ps_link_fox4` and `start_ps_samus_fox4` with their cold twins.

## Not exact

### Since fixed (2026-10-12, later the same day)

The Luigi bridge and the four assertions below gate exact and are
registered: `slp_bf_luigi_iceclimbers_t2300_cold` in
`SLIPPI_REPLAY_WITNESSES` (the cold scene carries the boundary's stale word,
`docs/PORT_NOTES/STALE_HEAP.md`; its boundary and cold twin are committed
with it), and the two "cliff camera scratch missing" and two
`ftCo_800DC920` explorer cases in `CORPUS_V3_MATCHES`. The sections below
describe them as this pass found them.

Recorded the same day and left local: `corpus_v3_ps_iceclimbers_peach4_eae02b66b_p1`
(4562 ticks, every tick rendered) gates exact on the build before the day's
fixes too. It was bridged for a frozen Peach holding a turnip (the explorer's
tick 4320), which retail fed the same pads never reaches: a Stadium
transformation follows its disc read, and the explorer's recording carries
the port's read timing (`stage_reads`), not retail's. An explorer case from
a Stadium boundary is retail's game only up to the first read whose timing
differs (`..._ec3145eb3_p2_edgeguard` is the explorer's game past tick
5198 and not at 5559).

Explorer faults with no owner, each recorded on retail from a Final
Destination boundary (every tick rendered, 120 ticks past the port's fault)
and left local; the port stops at the fault on retail's own game:

| Scenario | Tick | The port's stop |
|---|---|---|
| `corpus_v3_fd_iceclimbers_peach4_ecad2716f_p0` | 949 | `ftCo_800AC5A0`: the tumble steer with no knockback. Retail's Nana (Fighter at 0x80C9D2A0) steers y = -96, x = 0 on 950 and 951: the low byte of her address and a leftover r5 (`melee-cpu` `tumble.rs`) |
| `corpus_v3_fd_iceclimbers_peach4_e59d49e21_p0` | 599 | `ftCo_800ADE48`: CPU item use (Nana holding a turnip) |
| `corpus_v3_fd_iceclimbers_peach4_e15946934_p0` | 605 | "ftPe specials: mv+4 inherited from an unmodelled scratch word" |
| `corpus_v3_fd_mario_peach4_efb86521f_p1` | 330 | `itPeachToadSpore_Logic68_Reflected`: Mario's cape and Toad's spores |

### Newly diagnosed

No scenario of the 231 shows a port fault that was not already known or
is not fixed on `main` by now. One open item has its cause:

- `slp_bf_luigi_iceclimbers_t2300`: tick 540, `p0.cur_pos.y`, retail 1.9461 and the port 1.1461 (`self_vel.y` 0.031 and -0.769). `docs/SLIPPI.md` ("Nana takes no magnifier damage") has it as not diagnosed. It is now: tick 540 is the first frame of Luigi's first aerial Cyclone (358), whose entry reads Fighter +0x222C (`x222C_cycloneCharge`), a word Fighter_Create leaves as stale heap (fighter.c:855-858, as `ft-luigi` `init.rs` notes). The recording's raw fighter bytes hold 0x0932038D there from tick 0, so retail in Dolphin takes the charged branch (no `tap_y_vel_max` subtracted, 0.8 higher); the cold twin builds Luigi with zero, which is also what the console had (its replay shows the port's value). Not a fault in the Cyclone: a cold scene cannot know the boundary's heap. To gate this bridge the cold twin would need the boundary's word (a scenario field read at construction), which is a sim change and was not made here.

### Shown against retail by this pass, fixed on main meanwhile

Five recordings of the night of 2026-10-11 were untracked and undocumented
when the pass began; four did not gate. `main` fixed and registered all five
while it ran (4f1f52fd, 6231ce70, 7926e1d1, ad5da25a, edcb8b8b), and they
gate exact with 0 differing particle ticks on the rebased build. What the
triage here showed, for the record (it agrees with those commits):

- `slp_dl_fox_falco_r1_t1600`: tick 1482, `p0.percent`, retail 28.58 and the port 28.61: Falco's laser dealt 3 x 0.96 on retail and 3 x 0.97 in the port, one step of the stale-move table, with every earlier hit equal. Retail keeps the console's seed on every frame (`replay_vs_trace.py`).
- `slp_dl_donkeykong_ganondorf_gs_r1_t4900`: tick 4770, `rng.seed`: nine particle draws on retail, six in the port, a generator's creation (0x8039E1E4, 0x8039E3D4, 0x8039C8D4) missing as Donkey Kong and Ganondorf both took 10%. Retail keeps the console's seed on every frame.
- `slp_fod_level_falco_zelda_r1_t9100`: tick 8970, `p2.cur_pos.x`: Sheik, grounded in the needle throw, moved 0.025 on retail a tick after her needle hit Falco. Retail keeps the console's seed on every frame.
- `slp_ps_mario_pikachu_r1_t6400`: tick 6298, `p1.motion_id`, retail 366 and the port 364 (Pikachu's Thunder loop ending). This retail game leaves the console's seed at 4070.
- `slp_dl_captainfalcon_pichu_r1_t2300`: exact already. Retail in Dolphin leaves the console's recording at the replay's stop itself (position from frame 2159, seed from 2162), as the port does.

`kirby_inhale_whiff_fd_fox4` stopped at `unimplemented!` (tick 100,
"ftData_SpecialNeutral[Kirby]") and is exact and registered since f7e2ff07.

### Known open faults

- `slp_ps_marth_peach_wide_r1_t4600`, `slp_ps_marth_peach_wide9263_r1_t4600`, `slp_ps_marth_peach_wide9263_r1_t5400`: tick 3710, `items.0.pos.y`, retail 2.7089 and the port 4.5641. Mr. Saturn's bounce and walk, unported in `it-dosei` (tracker, Next 1: "Mr. Saturn's idle and slide states (two recordings leave the port there at tick 3710)"; `docs/SLIPPI.md`, the HNC 6 row). The three are one fault; `_t5400` contains `_t4600`.
- `slp_bf_yoshi_samus_t1000` gates exact on every key and item; `particle-sites` differs on tick 870 (the Egg Throw's and the Run dust's generators created in the other order, the same draws). Open in `docs/SLIPPI.md` ("One Yoshi bridge is a local probe"); the registered `slp_bf_yoshi_samus_t860` is its first 860 ticks.

### Assertions with no owner

- `corpus_v3_fd_iceclimbers_fox4_ed7f7dd2b_p0` (exact for 553 ticks) and `corpus_v3_ys_iceclimbers_fox4_ec6468ea6_p1` (811): panic on the next tick, "cliff camera scratch missing" (`melee-ft` `procs.rs:263`, `proc_camera_with_map`: `camera.on_ledge` set while the state data is not `Cliff`). Tracker, Next 1: explorer faults with recorded witnesses and no owner.
- `corpus_v3_bf_iceclimbers_fox4_ea1a1c9aa_p1` (exact for 2506 ticks) and `corpus_v3_bf_iceclimbers_fox4_ec1d4b39b_p0` (1200): assertion "ftCo_800DC920: a thrown fighter's constraint release on death" (`life.rs:727`, reached from `partner.rs:79` `vanish_with_leader`). The same tracker item.

### Wrong recordings

- `slp_ps_fox_drmario_wide9263_r1_t7200`: tick 1671, `p0.cur_anim_frame` (and motion, seed). Retail paused: the bridge fed the replay's Start press at 1671 and the console had Pause off (`docs/SLIPPI.md`, "Tooling found on the way"). Superseded by the registered `slp_ps_fox_drmario_wide9263_nopause_r1_t7200`.
- `start_fd_slippi73_ns_p24_marth0_sheik1_4_r232813` (and its cold twin): refused at setup, "game_start_seed 2187173367 reaches boundary seed 3530190225 through the setup draws, not 1763022367". The savestate was taken before the sleeping Zelda existed (`docs/SLIPPI.md`); superseded by the tracked `..._r232813b`.

## Exact but not registered

Duplicates and superseded recordings (each gates exact with 0 differing
particle ticks; registering them would add no tick of retail the lists do
not already hold, or the author set them aside):

- `corpus_v3_fd_link_fox4_e3e74affa_p2`, `_e4213e6a5_p2`, `_e50f774a0_p2`, `_e89a89d0e_p2`, `_ec13743d5_p0`: the first 86 to 125 ticks of the registered `link_x_*` scenes of the same names (the same pads, extended by 240 neutral ticks).
- `ps_match_cam_e9943b4ab` (770 ticks) and `ps_match_cam2_e9943b4ab` (2915): the pads of the registered `ps_match_screen_e9943b4ab_p0` (4100) and `corpus_v3_ps_fox_marth4_e9943b4ab_p0` (6001) from the same boundary, cut short. Camera probes; no particle dump.
- `slp_ys_samus_jigglypuff_oc_t2900`: the boundary and pads of the registered `slp_ys_samus_jigglypuff_t2900`, recorded again at two ticks per render on another host cadence (the game differs from tick 121). Superseded by `slp_ys_samus_jigglypuff_r1_t2900`.
- `slp_dl_captainfalcon_pichu_lag2158_t2300`, `slp_dl_captainfalcon_pichu_lag2159_t2300`: lag-frame probes of the `_r1_` recording `main` registered. Their fighters, items and seeds equal it on all 2300 ticks: the dropped display pass changes nothing there.
- `slp_dl_peach_younglink_lag8502_t8700`: equals the registered `_r1_` recording on all 6599 ticks retail ran; the match ended before the lag frame at 8502.
- `slp_fd_samus_pikachu_t4250`, `slp_fod_samus_marth_t7900` (its match ended at 3298), `slp_fod_level_fox_jigglypuff_t13500` (ended at 6914): two-ticks-per-render recordings beside registered `_r1_` ones of the same boundary and pads.
- `slp_fod_sheik_pikachu_lag4576_t4700`: the lag frame one record early; `docs/SLIPPI.md` calls it the wrong record. The registered `_lag4577_` is the right one.

Not registrable as they are:

- `slp_bf_marth_peach_t7300`: exact for 7300 ticks, but a tick-trace probe with no RNG ledger, which `SLIPPI_REPLAY_WITNESSES` requires. It extends the registered `slp_bf_marth_peach_t6500` by 800 ticks; a ledger pass (`record_many.py`) would make it registrable.
- `slp_bf_yoshi_samus_t1000`: above (particle order at 870).

## No retail trace

- `kirby_cutter_air_fd_fox4`, `_beam_hit_`, `_edge_`, `_ground_`, `_ledge_`, `_reverse_`, `_sword_hit_`: the Kirby agent's Final Cutter scenes, written within the hour and recorded since (see the top).
- `damage_level1_jab_fd_fox`: no trace, and the file does not parse as TOML (line 13: `raw = { button = 256,  }`, a trailing comma in an inline table). Written 2026-09-27; `damage_level1_dancingblade_fd_marth` is the recorded scene of that family.
- Tracked, for completeness: `puff_base_fd_fox4` has no trace (a `search` base).

## Boundaries left local

Each cold twin gates exact over 600 ticks with 0 differing particle ticks
against `ledger600`, as does the savestate scene where it does not name the
twin as its `gate`.

- No scenario starts from them: `start_bf_captainfalcon3_pikachu2_4`, `start_bf_neutral_captainfalcon3_pikachu2_4`, `start_bf_slippi_captainfalcon3_pikachu2_4`, `start_bf_slippi_marth1_peach1_4`, `start_bf_slippi_p24_marth1_peach1_4`, `start_ps_slippi_fox3_falco3_4`, `start_ps_slippi_p14_fox3_falco3_4`, `start_ps_wide_fox_marth4`.
- Only unregistered scenarios above start from them: `start_bf_slippi74_ns_p14_luigi2_iceclimbers0_4` (the Luigi bridge), `start_ps_slippi74_ns_frozen_wide_p12_marth0_peach0_4` (Mr. Saturn). Commit each with its scenario when that gates.

Four boundaries were untracked although registered bridges start from them;
they are committed now: `start_bf_slippi73ns_p24_falco0_sheik0_4`
(`slp_bf_falco_sheik_t3100`), `start_fd_slippi73ns_p12_fox0_link2_4_g14`
(`slp_fd_fox_link_t7200`), `start_fod_slippi73_p13_fox0_falco2_4_g04`
(`slp_fod_fox_falco_t2400`), `start_ps_slippi74pre_ns_p14_fox2_yoshi4_4`
(`slp_ps_fox_yoshi_t1300`).

## Registered

`CORPUS_V3_MATCHES` 9, `DONKEY_WITNESSES` 14, `MEWTWO_WITNESSES` 4,
`MATRIX_WITNESSES` 8, `widescreen_code_matches_retail` 2,
`SLIPPI_REPLAY_WITNESSES` 26; each entry's reason is in `m5_gate.rs`.

Twelve bridges had `frames` longer than their recording. In each the
recorder's tick and ledger passes both report `ended_early` at the same
record and one player's fourth KO falls just before it: retail's match ended
there. Nine are registered at the recorded length, with the reason in the
scenario's header (the copies in the main checkout keep the written length):

| Bridge | Written | Retail ran |
|---|---|---|
| `slp_dl_peach_younglink_r1_t8700` | 8700 | 6599 |
| `slp_fd_captainfalcon_iceclimbers_r001727_t17300` | 17300 | 7510 |
| `slp_fd_captainfalcon_samus_t10400` | 10400 | 3948 |
| `slp_fod_sheik_marth_t10300` | 10300 | 3145 |
| `slp_ps_falco_marth_t3200` | 3200 | 2856 |
| `slp_ps_falco_younglink_t4700` | 4700 | 3345 |
| `slp_ps_link_luigi_t10500` | 10500 | 7902 |
| `slp_ys_falco_iceclimbers_t7250` | 7250 | 3421 |
| `slp_ys_iceclimbers_captainfalcon_t2450` | 2450 | 2274 |

(`slp_bf_falco_iceclimbers_t5600`, 4699, `slp_dl_peach_fox_t4500`, 3547, and
`slp_bf_drmario_fox_t8628`, 8121, already carried the recorded length. The
other three with a longer `frames` are among the superseded recordings
above.)

Most registered bridges are not their replay's game to the end; the reason
beside each entry says where retail in Dolphin left the console.

## Traces with no scenario file

93 trace stems in `harness/traces` have no scenario of that name (62 hold a
tick trace only, 28 a recording with ledger and particle passes, 3 others). Early search
candidates and sweeps, left as they are:

`am_probe_capture_moving_platform`, `bowser_klaw_air_throw_fd_fox4`,
`capture_edge_fox_outward_stop{31,32,40,43}_candidate`,
`capture_edge_fox_outward_stop43_{grab110,grab112,jump108_grab108}_candidate`,
`clank_airborne_{fox,marth}_fd_foxmarth`, `clank_jab_fd_foxmarth`,
`clank_jab_s{74_f123,76_f122,76_f123,78_f122,78_f123,80_f122,80_f123}_fd_foxmarth`,
`clank_jab_spacing_fd_foxmarth`, `clank_priority_{fox,marth}_fd_foxmarth`,
`clank_smash_norebound_fd_foxmarth`,
`damage_fly_roll_{dtilt,dtilt_late,dtilt_t129,dtilt_t130,dtilt_t131,jab,late}_fd_fox_candidate`,
`damage_fly_roll_fd_fox_candidate`, `drmario_sheetturn_r{162,166,170,174}`,
`firefox_end_{departure,landing}_fd_fox`, `firefox_shallow_end_landing_fd_fox`,
`fod_idle_9000_sheik_marth`, `fox_ys`, `grab_high_fd_{foxmarth,marthfox}`,
`human_smoke_fd_marth_replaycheck`, `illusion_inward_sweep_{42,44,46,48,50}`,
`illusion_landing_sweep_{48,50,51,52,53}`, `jab200_fd_marth`, `laser`,
`laser_reflect_delayed_fd_marth`, `laser_reflect_delayed_full_trigger_fd_marth`,
`laser_reflect_{headless,null}_check`,
`laser_reflect_overflow_{air207,air208,air209,crouched,crouched_timed,half_tilt,held_tilt,tilt}_fd_marth`,
`laser_reflect_overflow_fd_marth`, `laser_reflect_pilot{55,56,57,58}_fd_marth`,
`laser_reflect_return_{late,reflector}_fd_marth`,
`match2_fd_foxmarth_replaycheck`, `match_fd_foxmarth_replaycheck`,
`meteor_dair_fd_marth`, `meteorcancel_dair_fd_marth`,
`stopceil_inward_fd_fox_candidate`, `stopceil_latejump{262,264}_fd_fox_candidate`,
`stopceil_left_latejump{266,273,276,280}_fd_marth_candidate`,
`stopceil_shine{274,280,286}_fd_fox_candidate`,
`stopceil_underside_fd_fox_candidate`, `topko_{dolphinslash,utilt}_fd_marth`,
`walljump_right_shallow_fd_fox_candidate`, `yoshi_hold_mouth_escape_bones`.

## The pytest fixtures `x.toml` and `bridged.toml`

They came from `harness/tests/test_replay_bridge.py`
(`test_scenario_uses_tick_clock_and_only_emits_changes` writes `bridged`,
`test_first_sample_drives_the_tick_after_the_boundary` writes `x`):
`replay_to_scenario.write_scenario` writes under `data_root.SCENARIOS`, which
with `MELEE_DATA_ROOT` set is the main checkout. a9848305 (2026-10-11)
pointed both tests at `tmp_path`; a worktree branched before it still leaked
until rebased. On `main` no test writes outside its temporary directory (the
suite run against an empty scratch data root: 333 passed, no file created),
and `harness/tests/conftest.py` now gives every test empty `SCENARIOS` and
`TRACES` directories of its own, so a test that forgets cannot reach a
checkout.
