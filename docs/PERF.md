# Performance regression history

Run `tools/perf-gate.sh` separately from `cargo gate`, on an otherwise idle
machine. It builds the native release CLI, strips a copy, runs `size` and
`cargo bloat --release -p melee-sim --bin melee-sim --crates -n 0`, counts
melee-ft instantiations with `cargo llvm-lines -p melee-ft --release --lib`,
the equivalent command for melee-sim and every character crate, and runs Criterion.
Missing cargo-bloat/cargo-llvm-lines tools are installed under
`target/perf-tools` (or the configured Cargo target directory). Failed installs
or missing census results produce an INCOMPLETE block and a nonzero exit.

`cargo bench -p melee-sim --bench ticks` measures two independent costs:
loading assets, saved fighter/stage/particle state and pads into a Simulation;
and 600 `tick_without_snapshot()` calls on a freshly loaded scene. The tick
measurement excludes setup, destruction, snapshots and comparison. Every
iteration starts at the same boundary. Neither measurement renders.

The initial 2026-09-09 main-checkout reference is **3,866,720 stripped bytes**,
**3,604,480 text bytes**, and **0.25 seconds CPU** for `gate start_fd_fox`
including load and comparison. That CPU timing is historical context, not a
baseline for the new wall-clock load and simulate-only measurements.

Environment variables configure regression limits:

- `PERF_TIME_TOLERANCE=10`: percent increase for load and 600-tick mean times.
- `PERF_SIZE_TOLERANCE=5`: percent increase for stripped and text sizes.
- `PERF_COPIES_TOLERANCE=0`: absolute increase in melee-ft copies per compiling crate.

The first complete passing run establishes timing and instantiation baselines;
its sizes must already pass the historical reference. Later runs compare to
the previous complete PASS block. INCOMPLETE and REGRESSION blocks remain in
history but never raise the baseline. Invalid measurements fail closed.
Raw logs and Criterion estimates live under `target/perf/<UTC timestamp>-<pid>`.

Instantiation attribution means **the crate compiling the function**, not a
claim that generic `Fighter<C>` names reveal their concrete character types.
The concrete melee-ft library owns C2's shared core bodies. The sim library
instantiates shared scene dispatch; each `ft-*` crate is also
measured independently (including zero melee-ft contributions). Full logs retain
all functions; dated blocks show the top 20 melee-ft functions by IR lines and
the sum of copies per compiling crate. No count is inferred by dividing a
shared total by roster size. See the upstream [llvm-lines interpretation](https://github.com/dtolnay/cargo-llvm-lines#multicrate-projects)
and [cargo-bloat measurement limitations](https://github.com/RazrFalcon/cargo-bloat#usage).

## Active baseline after P1

The COMPLETE PASS at **2026-09-10T06:15:55+00:00** is the active baseline: P1 on
`cd6cfad`, production-equivalent to main `7020449` before the P1 edits. The
intervening main commit adds only scenarios and TRACKER text. Default tolerances
remain time +10%, size +5%, and **zero** additional copies per compiling crate.
The report script automatically uses the latest complete PASS; no historical
failed block was edited or promoted.

| Metric | Active measurement | Next-run limit with default tolerances |
|---|---:|---:|
| Stripped bytes | 3,747,632 | 3,935,013 |
| Text bytes | 3,342,336 | 3,509,452 |
| Load mean | 166.000 ms | 182.600 ms |
| 600-tick mean | 23.588 ms | 25.947 ms |
| Throughput, equivalent to the tick-time check | 25,436 ticks/s | at least 23,123.82 ticks/s |
| melee-ft copies | 535 | 535 |
| melee-sim copies | 2,015 | 2,015 |
| ft-yoshi copies | 22 | 22 |
| ft-captain / ft-falco / ft-fox / ft-mario | 2 / 2 / 2 / 0 | unchanged |
| ft-mars / ft-peach / ft-purin | 3 / 8 / 2 | unchanged |

P1's explicitly requested baseline refresh used the documented
`PERF_COPIES_TOLERANCE=3` for one successful calibration: four new concrete
reset definitions minus removed platform-Option drop glue increase melee-ft
532→535 while total charged copies fall 2,796→2,591. This is an intentional
core accounting change, not a new permanent tolerance. The following complete
run passed with the default **PERF_COPIES_TOLERANCE=0**. Neither timing nor size
tolerance was widened. An earlier INCOMPLETE cache failure and a 28.325 ms timing
REGRESSION remain in the history. Subsequent complete samples were 23.397 and
23.588 ms. Full mechanism and evidence: `PORT_NOTES/P1_REVIVAL_YOSHI_BASELINE.md`.

In this restricted lane, cargo-bloat's full cargo-metadata call needed to unpack
cached `crunchy` sources. The global Cargo registry is read-only, so complete
runs used `CARGO_HOME=/tmp/p1-cargo-home CARGO_NET_OFFLINE=true`: a writable
registry index/source overlay with existing package/cache links. No network,
compiler options, benchmark parameters or repository dependencies changed.
Normal unrestricted checkouts can continue to run `tools/perf-gate.sh` directly.

## 2026-09-09T23:08:12+00:00 — INCOMPLETE

Evidence: `/Users/nikhilunni/Projects/melee-lanes/battlefield/target/perf/20260909T230733Z-39876`. Revision `579d1464179c5e68932b442f2d72356e542e0194` (working tree included).

rustc 1.96.0 (ac68faa20 2026-05-25); macOS-26.2-arm64-arm-64bit-Mach-O.

Tolerance: time +10%, size +5%, copies +0 per compiling crate.
Baseline: 2026-09-09 main size measurements; timing/copies not yet baselined.

| Measurement | Value |
|---|---:|
| Stripped binary | 3,866,784 bytes |
| Text (`size`) | 3,604,480 bytes |

- Criterion load: benchmark command failed

- Criterion ticks_600: benchmark command failed

- cargo-llvm-lines unavailable; installation failed

- cargo-bloat unavailable or failed

<!-- perf-gate-v1
{"date": "2026-09-09T23:08:12+00:00", "metrics": {"copies": {}, "stripped_bytes": 3866784, "text_bytes": 3604480}, "platform": "macOS-26.2-arm64-arm-64bit-Mach-O", "revision": "579d1464179c5e68932b442f2d72356e542e0194", "rustc": "rustc 1.96.0 (ac68faa20 2026-05-25)", "status": "INCOMPLETE"}
-->

## 2026-09-10T02:45:33+00:00 — INCOMPLETE

Evidence: `/Users/nikhilunni/Projects/melee-lanes/battlefield/target/perf/20260910T024316Z-11500`. Revision `3a2643e509db8dbbbf847fb797c412d6f2659149` (working tree included).

rustc 1.96.0 (ac68faa20 2026-05-25); macOS-26.2-arm64-arm-64bit-Mach-O.

Tolerance: time +10%, size +5%, copies +0 per compiling crate.
Baseline: 2026-09-09 main size measurements; timing/copies not yet baselined.

| Measurement | Value |
|---|---:|
| Stripped binary | 3,796,320 bytes |
| Text (`size`) | 3,375,104 bytes |
| load mean | 585.287 ms (95% CI 564.987..605.612 ms) |
| ticks_600 mean | 75.880 ms (95% CI 64.405..88.066 ms) |
| Headless throughput | 7,907 ticks/s |

- cargo-bloat unavailable or failed

Tool: `0.12.1`.

Tool: `cargo-llvm-lines 0.4.48`.

Per-crate text contribution (cargo-bloat estimates):
```text

```

### melee-ft: 615 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 21431 | 1 | `melee_ft::fighter::assets::FighterAssets::load` |
| 1723 | 1 | `melee_ft::fighter::damage::DamageParameters::read` |
| 1682 | 1 | `melee_ft::fighter::shield::ShieldParameters::read` |
| 1389 | 1 | `melee_ft::fighter::assets::read_script` |
| 1374 | 1 | `melee_ft::desc::common::CommonFighterData::read` |
| 1231 | 1 | `melee_ft::dynamics::read_sets` |
| 1130 | 6 | `melee_ft::anim::playback::for_each_aobj` |
| 1123 | 1 | `melee_ft::input::common::InputCommonData::read` |
| 1101 | 1 | `melee_ft::desc::playback::read_playback_motion` |
| 1042 | 1 | `melee_ft::desc::bones::read_fighter_bones` |
| 969 | 1 | `melee_ft::fighter::spawn::<impl melee_ft::fighter::FighterCore>::prepare` |
| 929 | 1 | `melee_ft::desc::fox_attributes::FireFoxAttributes::read` |
| 923 | 1 | `melee_ft::fighter::commands::CommandState::step_inner` |
| 885 | 1 | `melee_ft::desc::attributes::FighterAttributes::read` |
| 878 | 1 | `melee_ft::anim::attach::attach_motion` |
| 844 | 1 | `melee_ft::dynamics::read_motion_starts` |
| 659 | 1 | `melee_ft::desc::fox_attributes::IllusionAttributes::read` |
| 640 | 1 | `melee_ft::desc::bones::read_ground_pose` |
| 603 | 1 | `melee_ft::desc::bones::EcbBones::read` |
| 577 | 1 | `melee_ft::desc::fox_attributes::BlasterAttributes::read` |

### melee-sim: 2109 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 8673 | 49 | `melee_ft::fighter::grab::capture_pair` |
| 8036 | 49 | `melee_ft::fighter::grab_throw::enter_back_throw` |
| 4851 | 49 | `melee_ft::fighter::damage::detect_hit` |
| 3630 | 7 | `melee_ft::fighter::spawn::<impl melee_ft::fighter::Fighter<C>>::create` |
| 2548 | 49 | `melee_ft::fighter::grab_throw::release_back_throw` |
| 2450 | 7 | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::enter_shield` |
| 2422 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_input` |
| 2310 | 7 | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::shield_input` |
| 2198 | 7 | `melee_ft::fighter::walk::<impl melee_ft::fighter::Fighter<C>>::walk_input` |
| 2191 | 7 | `melee_ft::fighter::turn::<impl melee_ft::fighter::Fighter<C>>::turn_input` |
| 2114 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_animation` |
| 2107 | 7 | `melee_ft::fighter::squat::<impl melee_ft::fighter::Fighter<C>>::squat_input` |
| 1843 | 7 | `melee_ft::fighter::life::<impl melee_ft::fighter::Fighter<C>>::reset_for_revival` |
| 1820 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_collision` |
| 1764 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::enter_cliff_catch` |
| 1652 | 7 | `melee_ft::fighter::multi_jump::<impl melee_ft::fighter::Fighter<C>>::enter_multi_jump` |
| 1617 | 7 | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::damage_collision` |
| 1596 | 7 | `melee_ft::fighter::dash::<impl melee_ft::fighter::Fighter<C>>::dash_input` |
| 1540 | 7 | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::process_damage` |
| 1505 | 7 | `melee_ft::fighter::state::callbacks::collision::fall_collision` |

### ft-captain: 2 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 83 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 65 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::from_archive` |

### ft-falco: 2 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 71 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 55 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_load` |

### ft-fox: 2 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 71 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 55 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_load` |

### ft-mario: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mars: 3 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 67 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 42 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 22 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::guard_variant` |

### ft-peach: 8 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 65 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 64 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::check_float_input` |
| 55 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 52 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 49 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_resources_loaded` |
| 15 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_landing` |

### ft-purin: 2 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 61 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 13 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |

### ft-yoshi: 234 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 350 | 1 | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::enter_shield` |
| 346 | 1 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_input` |
| 330 | 1 | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::shield_input` |
| 314 | 1 | `melee_ft::fighter::walk::<impl melee_ft::fighter::Fighter<C>>::walk_input` |
| 313 | 1 | `melee_ft::fighter::turn::<impl melee_ft::fighter::Fighter<C>>::turn_input` |
| 302 | 1 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_animation` |
| 301 | 1 | `melee_ft::fighter::squat::<impl melee_ft::fighter::Fighter<C>>::squat_input` |
| 260 | 1 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_collision` |
| 252 | 1 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::enter_cliff_catch` |
| 236 | 1 | `melee_ft::fighter::multi_jump::<impl melee_ft::fighter::Fighter<C>>::enter_multi_jump` |
| 231 | 1 | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::damage_collision` |
| 228 | 1 | `melee_ft::fighter::dash::<impl melee_ft::fighter::Fighter<C>>::dash_input` |
| 215 | 1 | `melee_ft::fighter::state::callbacks::collision::fall_collision` |
| 212 | 1 | `melee_ft::fighter::down::<impl melee_ft::fighter::Fighter<C>>::enter_down_bound` |
| 209 | 1 | `melee_ft::fighter::jump::<impl melee_ft::fighter::Fighter<C>>::knee_bend_animation` |
| 207 | 1 | `melee_ft::fighter::attack::<impl melee_ft::fighter::Fighter<C>>::enter_ground_attack` |
| 205 | 1 | `melee_ft::fighter::jump::<impl melee_ft::fighter::Fighter<C>>::enter_aerial_jump` |
| 194 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 193 | 1 | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::shield_animation` |
| 186 | 1 | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::damage_input` |

<!-- perf-gate-v1
{"date": "2026-09-10T02:45:33+00:00", "metrics": {"copies": {"ft-captain": 2, "ft-falco": 2, "ft-fox": 2, "ft-mario": 0, "ft-mars": 3, "ft-peach": 8, "ft-purin": 2, "ft-yoshi": 234, "melee-ft": 615, "melee-sim": 2109}, "load_ns": 585286583.4, "stripped_bytes": 3796320, "text_bytes": 3375104, "ticks_600_ns": 75880179.1}, "platform": "macOS-26.2-arm64-arm-64bit-Mach-O", "revision": "3a2643e509db8dbbbf847fb797c412d6f2659149", "rustc": "rustc 1.96.0 (ac68faa20 2026-05-25)", "status": "INCOMPLETE"}
-->

## 2026-09-10T02:48:01+00:00 — COMPLETE — PASS

Evidence: `/Users/nikhilunni/Projects/melee-lanes/battlefield/target/perf/20260910T024544Z-13503`. Revision `3a2643e509db8dbbbf847fb797c412d6f2659149` (working tree included).

Timing caveat: these samples overlapped the initial workspace gate. This is a complete tool census, but its timing baseline is noisy. The planned isolated repeat was stopped after a newly restored M2 oracle exposed a test-initialization mismatch; see `PORT_NOTES/C5B_ZERO_ALLOC.md`.


rustc 1.96.0 (ac68faa20 2026-05-25); macOS-26.2-arm64-arm-64bit-Mach-O.

Tolerance: time +10%, size +5%, copies +0 per compiling crate.
Baseline: 2026-09-09 main size measurements; timing/copies not yet baselined.

| Measurement | Value |
|---|---:|
| Stripped binary | 3,796,320 bytes |
| Text (`size`) | 3,375,104 bytes |
| load mean | 748.969 ms (95% CI 642.717..874.996 ms) |
| ticks_600 mean | 90.577 ms (95% CI 70.107..110.217 ms) |
| Headless throughput | 6,624 ticks/s |

Tool: `0.12.1`.

Tool: `cargo-llvm-lines 0.4.48`.

Per-crate text contribution (cargo-bloat estimates):
```text
File  .text     Size Crate
18.6%  37.9% 963.9KiB melee_sim
 8.1%  16.5% 419.2KiB std
 5.5%  11.3% 287.9KiB melee_ft
 5.2%  10.6% 270.4KiB clap_builder
 1.3%   2.7%  67.8KiB slp
 1.2%   2.5%  64.3KiB melee_mp
 1.1%   2.3%  58.4KiB melee_gr
 1.1%   2.2%  55.0KiB hsd_archive
 0.9%   1.8%  45.6KiB hsd_anim
 0.8%   1.7%  42.8KiB serde_json
 0.8%   1.7%  42.7KiB toml
 0.8%   1.6%  41.7KiB ft_yoshi
 0.7%   1.4%  36.5KiB toml_parser
 0.6%   1.2%  31.4KiB hsd_particle
 0.3%   0.7%  16.9KiB serde_core
 0.3%   0.6%  16.4KiB melee_lb
 0.3%   0.6%  15.8KiB melee_diff
 0.3%   0.5%  13.0KiB hsd_gobj
 0.2%   0.4%  11.2KiB melee_types
 0.1%   0.3%   6.9KiB anyhow
 0.1%   0.2%   5.1KiB toml_datetime
 0.1%   0.2%   4.4KiB ft_mars
 0.1%   0.1%   3.7KiB ft_peach
 0.1%   0.1%   3.2KiB [Unknown]
 0.1%   0.1%   3.0KiB ft_purin
 0.1%   0.1%   2.8KiB clap_lex
 0.0%   0.1%   2.2KiB gekko_math
 0.0%   0.1%   2.1KiB serde
 0.0%   0.1%   2.1KiB ft_captain
 0.0%   0.1%   2.1KiB melee_if
 0.0%   0.1%   1.7KiB anstream
 0.0%   0.1%   1.6KiB anstyle
 0.0%   0.1%   1.4KiB strsim
 0.0%   0.0%   1.0KiB zmij
 0.0%   0.0%     960B itoa
 0.0%   0.0%     848B toml_writer
 0.0%   0.0%     680B ft_fox
 0.0%   0.0%     680B ft_falco
 0.0%   0.0%     312B winnow
 0.0%   0.0%     132B hsd_types
 0.0%   0.0%      20B __rustc
 0.0%   0.0%      16B colorchoice
49.0% 100.0%   2.5MiB .text section size, the file size is 5.1MiB

Note: numbers above are a result of guesswork. They are not 100% correct and never will be.
```

### melee-ft: 615 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 21431 | 1 | `melee_ft::fighter::assets::FighterAssets::load` |
| 1723 | 1 | `melee_ft::fighter::damage::DamageParameters::read` |
| 1682 | 1 | `melee_ft::fighter::shield::ShieldParameters::read` |
| 1389 | 1 | `melee_ft::fighter::assets::read_script` |
| 1374 | 1 | `melee_ft::desc::common::CommonFighterData::read` |
| 1231 | 1 | `melee_ft::dynamics::read_sets` |
| 1130 | 6 | `melee_ft::anim::playback::for_each_aobj` |
| 1123 | 1 | `melee_ft::input::common::InputCommonData::read` |
| 1101 | 1 | `melee_ft::desc::playback::read_playback_motion` |
| 1042 | 1 | `melee_ft::desc::bones::read_fighter_bones` |
| 969 | 1 | `melee_ft::fighter::spawn::<impl melee_ft::fighter::FighterCore>::prepare` |
| 929 | 1 | `melee_ft::desc::fox_attributes::FireFoxAttributes::read` |
| 923 | 1 | `melee_ft::fighter::commands::CommandState::step_inner` |
| 885 | 1 | `melee_ft::desc::attributes::FighterAttributes::read` |
| 878 | 1 | `melee_ft::anim::attach::attach_motion` |
| 844 | 1 | `melee_ft::dynamics::read_motion_starts` |
| 659 | 1 | `melee_ft::desc::fox_attributes::IllusionAttributes::read` |
| 640 | 1 | `melee_ft::desc::bones::read_ground_pose` |
| 603 | 1 | `melee_ft::desc::bones::EcbBones::read` |
| 577 | 1 | `melee_ft::desc::fox_attributes::BlasterAttributes::read` |

### melee-sim: 2109 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 8673 | 49 | `melee_ft::fighter::grab::capture_pair` |
| 8036 | 49 | `melee_ft::fighter::grab_throw::enter_back_throw` |
| 4851 | 49 | `melee_ft::fighter::damage::detect_hit` |
| 3630 | 7 | `melee_ft::fighter::spawn::<impl melee_ft::fighter::Fighter<C>>::create` |
| 2548 | 49 | `melee_ft::fighter::grab_throw::release_back_throw` |
| 2450 | 7 | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::enter_shield` |
| 2422 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_input` |
| 2310 | 7 | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::shield_input` |
| 2198 | 7 | `melee_ft::fighter::walk::<impl melee_ft::fighter::Fighter<C>>::walk_input` |
| 2191 | 7 | `melee_ft::fighter::turn::<impl melee_ft::fighter::Fighter<C>>::turn_input` |
| 2114 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_animation` |
| 2107 | 7 | `melee_ft::fighter::squat::<impl melee_ft::fighter::Fighter<C>>::squat_input` |
| 1843 | 7 | `melee_ft::fighter::life::<impl melee_ft::fighter::Fighter<C>>::reset_for_revival` |
| 1820 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_collision` |
| 1764 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::enter_cliff_catch` |
| 1652 | 7 | `melee_ft::fighter::multi_jump::<impl melee_ft::fighter::Fighter<C>>::enter_multi_jump` |
| 1617 | 7 | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::damage_collision` |
| 1596 | 7 | `melee_ft::fighter::dash::<impl melee_ft::fighter::Fighter<C>>::dash_input` |
| 1540 | 7 | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::process_damage` |
| 1505 | 7 | `melee_ft::fighter::state::callbacks::collision::fall_collision` |

### ft-captain: 2 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 83 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 65 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::from_archive` |

### ft-falco: 2 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 71 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 55 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_load` |

### ft-fox: 2 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 71 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 55 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_load` |

### ft-mario: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mars: 3 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 67 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 42 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 22 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::guard_variant` |

### ft-peach: 8 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 65 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 64 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::check_float_input` |
| 55 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 52 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 49 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_resources_loaded` |
| 15 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_landing` |

### ft-purin: 2 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 61 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 13 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |

### ft-yoshi: 234 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 350 | 1 | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::enter_shield` |
| 346 | 1 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_input` |
| 330 | 1 | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::shield_input` |
| 314 | 1 | `melee_ft::fighter::walk::<impl melee_ft::fighter::Fighter<C>>::walk_input` |
| 313 | 1 | `melee_ft::fighter::turn::<impl melee_ft::fighter::Fighter<C>>::turn_input` |
| 302 | 1 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_animation` |
| 301 | 1 | `melee_ft::fighter::squat::<impl melee_ft::fighter::Fighter<C>>::squat_input` |
| 260 | 1 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_collision` |
| 252 | 1 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::enter_cliff_catch` |
| 236 | 1 | `melee_ft::fighter::multi_jump::<impl melee_ft::fighter::Fighter<C>>::enter_multi_jump` |
| 231 | 1 | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::damage_collision` |
| 228 | 1 | `melee_ft::fighter::dash::<impl melee_ft::fighter::Fighter<C>>::dash_input` |
| 215 | 1 | `melee_ft::fighter::state::callbacks::collision::fall_collision` |
| 212 | 1 | `melee_ft::fighter::down::<impl melee_ft::fighter::Fighter<C>>::enter_down_bound` |
| 209 | 1 | `melee_ft::fighter::jump::<impl melee_ft::fighter::Fighter<C>>::knee_bend_animation` |
| 207 | 1 | `melee_ft::fighter::attack::<impl melee_ft::fighter::Fighter<C>>::enter_ground_attack` |
| 205 | 1 | `melee_ft::fighter::jump::<impl melee_ft::fighter::Fighter<C>>::enter_aerial_jump` |
| 194 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 193 | 1 | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::shield_animation` |
| 186 | 1 | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::damage_input` |

<!-- perf-gate-v1
{"date": "2026-09-10T02:48:01+00:00", "metrics": {"copies": {"ft-captain": 2, "ft-falco": 2, "ft-fox": 2, "ft-mario": 0, "ft-mars": 3, "ft-peach": 8, "ft-purin": 2, "ft-yoshi": 234, "melee-ft": 615, "melee-sim": 2109}, "load_ns": 748969133.5, "stripped_bytes": 3796320, "text_bytes": 3375104, "ticks_600_ns": 90577087.6}, "platform": "macOS-26.2-arm64-arm-64bit-Mach-O", "revision": "3a2643e509db8dbbbf847fb797c412d6f2659149", "rustc": "rustc 1.96.0 (ac68faa20 2026-05-25)", "status": "PASS"}
-->

## 2026-09-10T05:12:56+00:00 — COMPLETE — REGRESSION

Evidence: `/Users/nikhilunni/Projects/melee-lanes/combat/target/perf/20260910T051158Z-30459`. Revision `97b2c264190910995bfdb1b39458c5d3341ac774` (working tree included).

rustc 1.96.0 (ac68faa20 2026-05-25); macOS-26.2-arm64-arm-64bit-Mach-O.

Tolerance: time +10%, size +5%, copies +0 per compiling crate.
Baseline: 2026-09-10T02:48:01+00:00.

| Measurement | Value |
|---|---:|
| Stripped binary | 4,029,424 bytes |
| Text (`size`) | 3,588,096 bytes |
| load mean | 167.757 ms (95% CI 165.364..171.492 ms) |
| ticks_600 mean | 23.284 ms (95% CI 23.130..23.499 ms) |
| Headless throughput | 25,769 ticks/s |

- stripped_bytes: 4029424.000 > 3986136.000 (previous 3796320.000, +5%)

- text_bytes: 3588096.000 > 3543859.200 (previous 3375104.000, +5%)

Tool: `0.12.1`.

Tool: `cargo-llvm-lines 0.4.48`.

Per-crate text contribution (cargo-bloat estimates):
```text
File  .text     Size Crate
18.9%  38.1%   1.0MiB melee_sim
 8.2%  16.5% 448.4KiB std
 5.1%  10.2% 278.4KiB melee_ft
 4.9%   9.8% 266.6KiB clap_builder
 1.5%   2.9%  80.0KiB melee_ef
 1.3%   2.5%  68.7KiB slp
 1.2%   2.4%  64.2KiB melee_mp
 1.1%   2.2%  61.2KiB melee_gr
 0.9%   1.9%  51.4KiB hsd_anim
 0.9%   1.9%  50.8KiB hsd_archive
 0.8%   1.5%  41.7KiB serde_json
 0.8%   1.5%  41.7KiB toml
 0.7%   1.5%  39.7KiB ft_yoshi
 0.7%   1.3%  36.5KiB toml_parser
 0.6%   1.3%  34.4KiB hsd_particle
 0.3%   0.7%  18.8KiB serde_core
 0.3%   0.6%  15.5KiB melee_diff
 0.3%   0.6%  15.4KiB melee_lb
 0.2%   0.5%  13.6KiB hsd_gobj
 0.2%   0.4%  10.8KiB melee_types
 0.1%   0.3%   7.0KiB anyhow
 0.1%   0.2%   5.1KiB toml_datetime
 0.1%   0.1%   3.9KiB melee_cmd
 0.1%   0.1%   3.7KiB ft_peach
 0.1%   0.1%   3.1KiB [Unknown]
 0.1%   0.1%   3.0KiB ft_purin
 0.1%   0.1%   2.8KiB clap_lex
 0.0%   0.1%   2.7KiB gekko_math
 0.0%   0.1%   2.5KiB melee_coll
 0.0%   0.1%   2.3KiB ft_mars
 0.0%   0.1%   2.1KiB serde
 0.0%   0.1%   2.1KiB ft_captain
 0.0%   0.1%   2.1KiB melee_if
 0.0%   0.1%   1.7KiB anstream
 0.0%   0.1%   1.6KiB anstyle
 0.0%   0.0%   1.4KiB strsim
 0.0%   0.0%   1.0KiB zmij
 0.0%   0.0%     960B itoa
 0.0%   0.0%     848B toml_writer
 0.0%   0.0%     680B ft_falco
 0.0%   0.0%     680B ft_fox
 0.0%   0.0%     312B winnow
 0.0%   0.0%     132B hsd_types
 0.0%   0.0%      20B __rustc
 0.0%   0.0%      16B colorchoice
49.7% 100.0%   2.7MiB .text section size, the file size is 5.4MiB

Note: numbers above are a result of guesswork. They are not 100% correct and never will be.
```

### melee-ft: 532 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 21452 | 1 | `melee_ft::fighter::assets::FighterAssets::load` |
| 1723 | 1 | `melee_ft::fighter::damage::DamageParameters::read` |
| 1682 | 1 | `melee_ft::fighter::shield::ShieldParameters::read` |
| 1374 | 1 | `melee_ft::desc::common::CommonFighterData::read` |
| 1231 | 1 | `melee_ft::dynamics::read_sets` |
| 1123 | 1 | `melee_ft::input::common::InputCommonData::read` |
| 1101 | 1 | `melee_ft::desc::playback::read_playback_motion` |
| 1042 | 1 | `melee_ft::desc::bones::read_fighter_bones` |
| 980 | 1 | `melee_ft::fighter::spawn::<impl melee_ft::fighter::FighterCore>::prepare` |
| 929 | 1 | `melee_ft::desc::fox_attributes::FireFoxAttributes::read` |
| 885 | 1 | `melee_ft::desc::attributes::FighterAttributes::read` |
| 844 | 1 | `melee_ft::dynamics::read_motion_starts` |
| 760 | 1 | `melee_ft::anim::attach::select_motion` |
| 702 | 1 | `melee_ft::fighter::commands::CommandState::step_inner` |
| 682 | 1 | `melee_ft::anim::playback::FighterAnimation::resume_dynamic_subtree` |
| 659 | 1 | `melee_ft::desc::fox_attributes::IllusionAttributes::read` |
| 640 | 1 | `melee_ft::desc::bones::read_ground_pose` |
| 603 | 1 | `melee_ft::desc::bones::EcbBones::read` |
| 577 | 1 | `melee_ft::desc::fox_attributes::BlasterAttributes::read` |
| 550 | 1 | `melee_ft::desc::animation::read_entry` |

### melee-sim: 2017 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 8673 | 49 | `melee_ft::fighter::grab::capture_pair` |
| 8036 | 49 | `melee_ft::fighter::grab_throw::enter_back_throw` |
| 3458 | 7 | `melee_ft::fighter::spawn::<impl melee_ft::fighter::Fighter<C>>::create` |
| 3087 | 49 | `melee_ft::fighter::damage::detect_hit` |
| 2548 | 49 | `melee_ft::fighter::grab_throw::release_back_throw` |
| 2450 | 7 | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::enter_shield` |
| 2422 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_input` |
| 2310 | 7 | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::shield_input` |
| 2198 | 7 | `melee_ft::fighter::walk::<impl melee_ft::fighter::Fighter<C>>::walk_input` |
| 2191 | 7 | `melee_ft::fighter::turn::<impl melee_ft::fighter::Fighter<C>>::turn_input` |
| 2114 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_animation` |
| 2107 | 7 | `melee_ft::fighter::squat::<impl melee_ft::fighter::Fighter<C>>::squat_input` |
| 1843 | 7 | `melee_ft::fighter::life::<impl melee_ft::fighter::Fighter<C>>::reset_for_revival` |
| 1764 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::enter_cliff_catch` |
| 1666 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_collision` |
| 1652 | 7 | `melee_ft::fighter::multi_jump::<impl melee_ft::fighter::Fighter<C>>::enter_multi_jump` |
| 1617 | 7 | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::damage_collision` |
| 1596 | 7 | `melee_ft::fighter::dash::<impl melee_ft::fighter::Fighter<C>>::dash_input` |
| 1540 | 7 | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::process_damage` |
| 1505 | 7 | `melee_ft::fighter::state::callbacks::collision::fall_collision` |

### ft-captain: 2 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 83 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 65 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::from_archive` |

### ft-falco: 2 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 71 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 55 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_load` |

### ft-fox: 2 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 71 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 55 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_load` |

### ft-mario: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mars: 3 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 67 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 42 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 22 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::guard_variant` |

### ft-peach: 8 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 65 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 64 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::check_float_input` |
| 55 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 52 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 49 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_resources_loaded` |
| 15 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_landing` |

### ft-purin: 2 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 61 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 13 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |

### ft-yoshi: 228 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 350 | 1 | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::enter_shield` |
| 346 | 1 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_input` |
| 330 | 1 | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::shield_input` |
| 314 | 1 | `melee_ft::fighter::walk::<impl melee_ft::fighter::Fighter<C>>::walk_input` |
| 313 | 1 | `melee_ft::fighter::turn::<impl melee_ft::fighter::Fighter<C>>::turn_input` |
| 302 | 1 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_animation` |
| 301 | 1 | `melee_ft::fighter::squat::<impl melee_ft::fighter::Fighter<C>>::squat_input` |
| 252 | 1 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::enter_cliff_catch` |
| 238 | 1 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_collision` |
| 236 | 1 | `melee_ft::fighter::multi_jump::<impl melee_ft::fighter::Fighter<C>>::enter_multi_jump` |
| 231 | 1 | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::damage_collision` |
| 228 | 1 | `melee_ft::fighter::dash::<impl melee_ft::fighter::Fighter<C>>::dash_input` |
| 215 | 1 | `melee_ft::fighter::state::callbacks::collision::fall_collision` |
| 212 | 1 | `melee_ft::fighter::down::<impl melee_ft::fighter::Fighter<C>>::enter_down_bound` |
| 209 | 1 | `melee_ft::fighter::jump::<impl melee_ft::fighter::Fighter<C>>::knee_bend_animation` |
| 207 | 1 | `melee_ft::fighter::attack::<impl melee_ft::fighter::Fighter<C>>::enter_ground_attack` |
| 205 | 1 | `melee_ft::fighter::jump::<impl melee_ft::fighter::Fighter<C>>::enter_aerial_jump` |
| 194 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 193 | 1 | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::shield_animation` |
| 186 | 1 | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::damage_input` |

<!-- perf-gate-v1
{"date": "2026-09-10T05:12:56+00:00", "metrics": {"copies": {"ft-captain": 2, "ft-falco": 2, "ft-fox": 2, "ft-mario": 0, "ft-mars": 3, "ft-peach": 8, "ft-purin": 2, "ft-yoshi": 228, "melee-ft": 532, "melee-sim": 2017}, "load_ns": 167756802.57757935, "stripped_bytes": 4029424, "text_bytes": 3588096, "ticks_600_ns": 23283934.666666664}, "platform": "macOS-26.2-arm64-arm-64bit-Mach-O", "revision": "97b2c264190910995bfdb1b39458c5d3341ac774", "rustc": "rustc 1.96.0 (ac68faa20 2026-05-25)", "status": "REGRESSION"}
-->

## 2026-09-10T05:24:01+00:00 — COMPLETE — PASS

Evidence: `/Users/nikhilunni/Projects/melee-lanes/combat/target/perf/20260910T052304Z-40654`. Revision `97b2c264190910995bfdb1b39458c5d3341ac774` (working tree included).

rustc 1.96.0 (ac68faa20 2026-05-25); macOS-26.2-arm64-arm-64bit-Mach-O.

Tolerance: time +10%, size +5%, copies +0 per compiling crate.
Baseline: 2026-09-10T02:48:01+00:00.

| Measurement | Value |
|---|---:|
| Stripped binary | 3,864,176 bytes |
| Text (`size`) | 3,424,256 bytes |
| load mean | 169.320 ms (95% CI 167.766..171.616 ms) |
| ticks_600 mean | 23.606 ms (95% CI 23.433..23.848 ms) |
| Headless throughput | 25,417 ticks/s |

Tool: `0.12.1`.

Tool: `cargo-llvm-lines 0.4.48`.

Per-crate text contribution (cargo-bloat estimates):
```text
File  .text     Size Crate
16.7%  34.6% 888.4KiB melee_sim
 8.4%  17.5% 448.2KiB std
 5.2%  10.9% 279.1KiB melee_ft
 5.0%  10.4% 266.1KiB clap_builder
 1.5%   3.1%  80.0KiB melee_ef
 1.3%   2.6%  67.8KiB slp
 1.2%   2.5%  64.2KiB melee_mp
 1.2%   2.4%  61.2KiB melee_gr
 1.0%   2.0%  50.8KiB hsd_archive
 1.0%   2.0%  50.8KiB hsd_anim
 0.8%   1.6%  41.7KiB toml
 0.7%   1.5%  39.7KiB ft_yoshi
 0.7%   1.5%  37.3KiB serde_json
 0.7%   1.4%  36.5KiB toml_parser
 0.6%   1.3%  34.4KiB hsd_particle
 0.3%   0.7%  17.4KiB serde_core
 0.3%   0.6%  15.5KiB melee_diff
 0.3%   0.6%  15.4KiB melee_lb
 0.3%   0.5%  13.6KiB hsd_gobj
 0.2%   0.4%  11.2KiB melee_types
 0.1%   0.3%   7.0KiB anyhow
 0.1%   0.2%   5.1KiB toml_datetime
 0.1%   0.2%   3.9KiB melee_cmd
 0.1%   0.1%   3.7KiB ft_peach
 0.1%   0.1%   3.1KiB [Unknown]
 0.1%   0.1%   3.0KiB ft_purin
 0.1%   0.1%   2.8KiB clap_lex
 0.1%   0.1%   2.7KiB gekko_math
 0.0%   0.1%   2.5KiB melee_coll
 0.0%   0.1%   2.3KiB ft_mars
 0.0%   0.1%   2.1KiB serde
 0.0%   0.1%   2.1KiB ft_captain
 0.0%   0.1%   2.1KiB melee_if
 0.0%   0.1%   1.7KiB anstream
 0.0%   0.1%   1.6KiB anstyle
 0.0%   0.1%   1.4KiB strsim
 0.0%   0.0%   1.0KiB zmij
 0.0%   0.0%     960B itoa
 0.0%   0.0%     848B toml_writer
 0.0%   0.0%     680B ft_fox
 0.0%   0.0%     680B ft_falco
 0.0%   0.0%     312B winnow
 0.0%   0.0%     132B hsd_types
 0.0%   0.0%      20B __rustc
 0.0%   0.0%      16B colorchoice
48.3% 100.0%   2.5MiB .text section size, the file size is 5.2MiB

Note: numbers above are a result of guesswork. They are not 100% correct and never will be.
```

### melee-ft: 532 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 21452 | 1 | `melee_ft::fighter::assets::FighterAssets::load` |
| 1723 | 1 | `melee_ft::fighter::damage::DamageParameters::read` |
| 1682 | 1 | `melee_ft::fighter::shield::ShieldParameters::read` |
| 1374 | 1 | `melee_ft::desc::common::CommonFighterData::read` |
| 1231 | 1 | `melee_ft::dynamics::read_sets` |
| 1123 | 1 | `melee_ft::input::common::InputCommonData::read` |
| 1101 | 1 | `melee_ft::desc::playback::read_playback_motion` |
| 1042 | 1 | `melee_ft::desc::bones::read_fighter_bones` |
| 980 | 1 | `melee_ft::fighter::spawn::<impl melee_ft::fighter::FighterCore>::prepare` |
| 929 | 1 | `melee_ft::desc::fox_attributes::FireFoxAttributes::read` |
| 885 | 1 | `melee_ft::desc::attributes::FighterAttributes::read` |
| 844 | 1 | `melee_ft::dynamics::read_motion_starts` |
| 760 | 1 | `melee_ft::anim::attach::select_motion` |
| 702 | 1 | `melee_ft::fighter::commands::CommandState::step_inner` |
| 682 | 1 | `melee_ft::anim::playback::FighterAnimation::resume_dynamic_subtree` |
| 659 | 1 | `melee_ft::desc::fox_attributes::IllusionAttributes::read` |
| 640 | 1 | `melee_ft::desc::bones::read_ground_pose` |
| 603 | 1 | `melee_ft::desc::bones::EcbBones::read` |
| 577 | 1 | `melee_ft::desc::fox_attributes::BlasterAttributes::read` |
| 550 | 1 | `melee_ft::desc::animation::read_entry` |

### melee-sim: 2017 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 8673 | 49 | `melee_ft::fighter::grab::capture_pair` |
| 8036 | 49 | `melee_ft::fighter::grab_throw::enter_back_throw` |
| 3458 | 7 | `melee_ft::fighter::spawn::<impl melee_ft::fighter::Fighter<C>>::create` |
| 3087 | 49 | `melee_ft::fighter::damage::detect_hit` |
| 2548 | 49 | `melee_ft::fighter::grab_throw::release_back_throw` |
| 2450 | 7 | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::enter_shield` |
| 2422 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_input` |
| 2310 | 7 | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::shield_input` |
| 2198 | 7 | `melee_ft::fighter::walk::<impl melee_ft::fighter::Fighter<C>>::walk_input` |
| 2191 | 7 | `melee_ft::fighter::turn::<impl melee_ft::fighter::Fighter<C>>::turn_input` |
| 2114 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_animation` |
| 2107 | 7 | `melee_ft::fighter::squat::<impl melee_ft::fighter::Fighter<C>>::squat_input` |
| 1843 | 7 | `melee_ft::fighter::life::<impl melee_ft::fighter::Fighter<C>>::reset_for_revival` |
| 1764 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::enter_cliff_catch` |
| 1666 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_collision` |
| 1652 | 7 | `melee_ft::fighter::multi_jump::<impl melee_ft::fighter::Fighter<C>>::enter_multi_jump` |
| 1617 | 7 | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::damage_collision` |
| 1596 | 7 | `melee_ft::fighter::dash::<impl melee_ft::fighter::Fighter<C>>::dash_input` |
| 1540 | 7 | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::process_damage` |
| 1505 | 7 | `melee_ft::fighter::state::callbacks::collision::fall_collision` |

### ft-captain: 2 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 83 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 65 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::from_archive` |

### ft-falco: 2 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 71 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 55 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_load` |

### ft-fox: 2 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 71 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 55 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_load` |

### ft-mario: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mars: 3 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 67 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 42 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 22 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::guard_variant` |

### ft-peach: 8 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 65 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 64 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::check_float_input` |
| 55 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 52 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 49 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_resources_loaded` |
| 15 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_landing` |

### ft-purin: 2 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 61 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 13 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |

### ft-yoshi: 228 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 350 | 1 | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::enter_shield` |
| 346 | 1 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_input` |
| 330 | 1 | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::shield_input` |
| 314 | 1 | `melee_ft::fighter::walk::<impl melee_ft::fighter::Fighter<C>>::walk_input` |
| 313 | 1 | `melee_ft::fighter::turn::<impl melee_ft::fighter::Fighter<C>>::turn_input` |
| 302 | 1 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_animation` |
| 301 | 1 | `melee_ft::fighter::squat::<impl melee_ft::fighter::Fighter<C>>::squat_input` |
| 252 | 1 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::enter_cliff_catch` |
| 238 | 1 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_collision` |
| 236 | 1 | `melee_ft::fighter::multi_jump::<impl melee_ft::fighter::Fighter<C>>::enter_multi_jump` |
| 231 | 1 | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::damage_collision` |
| 228 | 1 | `melee_ft::fighter::dash::<impl melee_ft::fighter::Fighter<C>>::dash_input` |
| 215 | 1 | `melee_ft::fighter::state::callbacks::collision::fall_collision` |
| 212 | 1 | `melee_ft::fighter::down::<impl melee_ft::fighter::Fighter<C>>::enter_down_bound` |
| 209 | 1 | `melee_ft::fighter::jump::<impl melee_ft::fighter::Fighter<C>>::knee_bend_animation` |
| 207 | 1 | `melee_ft::fighter::attack::<impl melee_ft::fighter::Fighter<C>>::enter_ground_attack` |
| 205 | 1 | `melee_ft::fighter::jump::<impl melee_ft::fighter::Fighter<C>>::enter_aerial_jump` |
| 194 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 193 | 1 | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::shield_animation` |
| 186 | 1 | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::damage_input` |

<!-- perf-gate-v1
{"date": "2026-09-10T05:24:01+00:00", "metrics": {"copies": {"ft-captain": 2, "ft-falco": 2, "ft-fox": 2, "ft-mario": 0, "ft-mars": 3, "ft-peach": 8, "ft-purin": 2, "ft-yoshi": 228, "melee-ft": 532, "melee-sim": 2017}, "load_ns": 169320193.10178572, "stripped_bytes": 3864176, "text_bytes": 3424256, "ticks_600_ns": 23605858.333333336}, "platform": "macOS-26.2-arm64-arm-64bit-Mach-O", "revision": "97b2c264190910995bfdb1b39458c5d3341ac774", "rustc": "rustc 1.96.0 (ac68faa20 2026-05-25)", "status": "PASS"}
-->

## 2026-09-10T06:01:06+00:00 — INCOMPLETE

Evidence: `/Users/nikhilunni/Projects/melee-lanes/chars/target/perf/20260910T060020Z-75512`. Revision `cd6cfada40bc577865ae540a34aa34faf12d28f3` (working tree included).

rustc 1.96.0 (ac68faa20 2026-05-25); macOS-26.2-arm64-arm-64bit-Mach-O.

Tolerance: time +10%, size +5%, copies +0 per compiling crate.
Baseline: 2026-09-10T05:24:01+00:00.

| Measurement | Value |
|---|---:|
| Stripped binary | 3,747,632 bytes |
| Text (`size`) | 3,342,336 bytes |
| load mean | 164.673 ms (95% CI 164.022..165.415 ms) |
| ticks_600 mean | 23.227 ms (95% CI 23.160..23.293 ms) |
| Headless throughput | 25,832 ticks/s |

- cargo-bloat unavailable or failed

- melee-ft melee-ft copies: 535 > 532 + 0

Tool: `0.12.1`.

Tool: `cargo-llvm-lines 0.4.48`.

Per-crate text contribution (cargo-bloat estimates):
```text

```

### melee-ft: 535 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 21452 | 1 | `melee_ft::fighter::assets::FighterAssets::load` |
| 1723 | 1 | `melee_ft::fighter::damage::DamageParameters::read` |
| 1682 | 1 | `melee_ft::fighter::shield::ShieldParameters::read` |
| 1374 | 1 | `melee_ft::desc::common::CommonFighterData::read` |
| 1231 | 1 | `melee_ft::dynamics::read_sets` |
| 1123 | 1 | `melee_ft::input::common::InputCommonData::read` |
| 1101 | 1 | `melee_ft::desc::playback::read_playback_motion` |
| 1042 | 1 | `melee_ft::desc::bones::read_fighter_bones` |
| 1000 | 1 | `melee_ft::fighter::spawn::<impl melee_ft::fighter::FighterCore>::prepare` |
| 929 | 1 | `melee_ft::desc::fox_attributes::FireFoxAttributes::read` |
| 885 | 1 | `melee_ft::desc::attributes::FighterAttributes::read` |
| 844 | 1 | `melee_ft::dynamics::read_motion_starts` |
| 760 | 1 | `melee_ft::anim::attach::select_motion` |
| 702 | 1 | `melee_ft::fighter::commands::CommandState::step_inner` |
| 682 | 1 | `melee_ft::anim::playback::FighterAnimation::resume_dynamic_subtree` |
| 659 | 1 | `melee_ft::desc::fox_attributes::IllusionAttributes::read` |
| 640 | 1 | `melee_ft::desc::bones::read_ground_pose` |
| 603 | 1 | `melee_ft::desc::bones::EcbBones::read` |
| 577 | 1 | `melee_ft::desc::fox_attributes::BlasterAttributes::read` |
| 550 | 1 | `melee_ft::desc::animation::read_entry` |

### melee-sim: 2015 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 8673 | 49 | `melee_ft::fighter::grab::capture_pair` |
| 8036 | 49 | `melee_ft::fighter::grab_throw::enter_back_throw` |
| 3087 | 49 | `melee_ft::fighter::damage::detect_hit` |
| 2548 | 49 | `melee_ft::fighter::grab_throw::release_back_throw` |
| 2450 | 7 | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::enter_shield` |
| 2422 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_input` |
| 2310 | 7 | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::shield_input` |
| 2198 | 7 | `melee_ft::fighter::walk::<impl melee_ft::fighter::Fighter<C>>::walk_input` |
| 2191 | 7 | `melee_ft::fighter::turn::<impl melee_ft::fighter::Fighter<C>>::turn_input` |
| 2114 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_animation` |
| 2107 | 7 | `melee_ft::fighter::squat::<impl melee_ft::fighter::Fighter<C>>::squat_input` |
| 1764 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::enter_cliff_catch` |
| 1666 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_collision` |
| 1652 | 7 | `melee_ft::fighter::multi_jump::<impl melee_ft::fighter::Fighter<C>>::enter_multi_jump` |
| 1624 | 7 | `melee_ft::fighter::spawn::<impl melee_ft::fighter::Fighter<C>>::initialize_spawn` |
| 1617 | 7 | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::damage_collision` |
| 1596 | 7 | `melee_ft::fighter::dash::<impl melee_ft::fighter::Fighter<C>>::dash_input` |
| 1540 | 7 | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::process_damage` |
| 1505 | 7 | `melee_ft::fighter::state::callbacks::collision::fall_collision` |
| 1484 | 7 | `melee_ft::fighter::down::<impl melee_ft::fighter::Fighter<C>>::enter_down_bound` |

### ft-captain: 2 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 83 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 65 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::from_archive` |

### ft-falco: 2 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 71 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 55 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_load` |

### ft-fox: 2 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 71 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 55 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_load` |

### ft-mario: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mars: 3 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 67 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 42 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 22 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::guard_variant` |

### ft-peach: 8 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 65 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 64 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::check_float_input` |
| 55 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 52 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 49 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_resources_loaded` |
| 15 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_landing` |

### ft-purin: 2 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 61 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 13 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |

### ft-yoshi: 22 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 194 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 179 | 1 | `melee_ft::fighter::escape::<impl melee_ft::fighter::Fighter<C>>::enter_escape` |
| 142 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 112 | 1 | `melee_ft::fighter::state::row::<impl melee_ft::fighter::Fighter<C>>::row` |
| 84 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_animated` |
| 72 | 1 | `melee_ft::fighter::spawn::<impl melee_ft::fighter::Fighter<C>>::change_motion_state_with_options` |
| 60 | 1 | `melee_ft::fighter::escape::<impl melee_ft::fighter::FighterCore>::roll_input` |
| 54 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 49 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 40 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_entered` |
| 28 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 18 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::action_id` |
| 16 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::input_shield` |
| 11 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::check_hurtbox_interaction` |
| 9 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::animate_shield` |
| 9 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::enter_guard_hold` |
| 9 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::enter_guard_off` |
| 9 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::enter_shield` |
| 6 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::escape_variant` |
| 2 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::escape_finished` |

<!-- perf-gate-v1
{"date": "2026-09-10T06:01:06+00:00", "metrics": {"copies": {"ft-captain": 2, "ft-falco": 2, "ft-fox": 2, "ft-mario": 0, "ft-mars": 3, "ft-peach": 8, "ft-purin": 2, "ft-yoshi": 22, "melee-ft": 535, "melee-sim": 2015}, "load_ns": 164673477.9965873, "stripped_bytes": 3747632, "text_bytes": 3342336, "ticks_600_ns": 23226988.966666665}, "platform": "macOS-26.2-arm64-arm-64bit-Mach-O", "revision": "cd6cfada40bc577865ae540a34aa34faf12d28f3", "rustc": "rustc 1.96.0 (ac68faa20 2026-05-25)", "status": "INCOMPLETE"}
-->

## 2026-09-10T06:14:21+00:00 — COMPLETE — REGRESSION

Evidence: `/Users/nikhilunni/Projects/melee-lanes/chars/target/perf/20260910T061325Z-88408`. Revision `cd6cfada40bc577865ae540a34aa34faf12d28f3` (working tree included).

rustc 1.96.0 (ac68faa20 2026-05-25); macOS-26.2-arm64-arm-64bit-Mach-O.

Tolerance: time +10%, size +5%, copies +3 per compiling crate.
Baseline: 2026-09-10T05:24:01+00:00.

| Measurement | Value |
|---|---:|
| Stripped binary | 3,747,632 bytes |
| Text (`size`) | 3,342,336 bytes |
| load mean | 166.188 ms (95% CI 165.843..166.523 ms) |
| ticks_600 mean | 28.325 ms (95% CI 25.740..31.138 ms) |
| Headless throughput | 21,183 ticks/s |

- ticks_600_ns: 28325143.033 > 25966444.167 (previous 23605858.333, +10%)

Tool: `0.12.1`.

Tool: `cargo-llvm-lines 0.4.48`.

Per-crate text contribution (cargo-bloat estimates):
```text
File  .text     Size Crate
16.6%  34.2% 858.3KiB melee_sim
 8.6%  17.8% 446.9KiB std
 5.5%  11.3% 283.4KiB melee_ft
 5.2%  10.6% 266.1KiB clap_builder
 1.5%   3.2%  80.0KiB melee_ef
 1.3%   2.7%  67.8KiB slp
 1.2%   2.6%  64.2KiB melee_mp
 1.1%   2.4%  59.3KiB melee_gr
 1.0%   2.0%  50.8KiB hsd_anim
 1.0%   2.0%  50.7KiB hsd_archive
 0.8%   1.7%  41.7KiB toml
 0.7%   1.5%  37.2KiB serde_json
 0.7%   1.5%  36.5KiB toml_parser
 0.7%   1.4%  34.4KiB hsd_particle
 0.3%   0.7%  17.6KiB serde_core
 0.3%   0.6%  15.5KiB melee_diff
 0.3%   0.6%  15.4KiB melee_lb
 0.3%   0.6%  14.5KiB ft_yoshi
 0.3%   0.5%  13.6KiB hsd_gobj
 0.2%   0.4%  11.1KiB melee_types
 0.1%   0.3%   7.0KiB anyhow
 0.1%   0.2%   5.1KiB toml_datetime
 0.1%   0.2%   3.9KiB melee_cmd
 0.1%   0.1%   3.7KiB ft_peach
 0.1%   0.1%   3.1KiB [Unknown]
 0.1%   0.1%   3.0KiB ft_purin
 0.1%   0.1%   2.8KiB clap_lex
 0.1%   0.1%   2.7KiB gekko_math
 0.0%   0.1%   2.5KiB melee_coll
 0.0%   0.1%   2.3KiB ft_mars
 0.0%   0.1%   2.1KiB serde
 0.0%   0.1%   2.1KiB ft_captain
 0.0%   0.1%   2.1KiB melee_if
 0.0%   0.1%   1.7KiB anstream
 0.0%   0.1%   1.6KiB anstyle
 0.0%   0.1%   1.4KiB strsim
 0.0%   0.0%   1.0KiB zmij
 0.0%   0.0%     960B itoa
 0.0%   0.0%     848B toml_writer
 0.0%   0.0%     680B ft_fox
 0.0%   0.0%     680B ft_falco
 0.0%   0.0%     312B winnow
 0.0%   0.0%     132B hsd_types
 0.0%   0.0%      20B __rustc
 0.0%   0.0%      16B colorchoice
48.6% 100.0%   2.5MiB .text section size, the file size is 5.0MiB

Note: numbers above are a result of guesswork. They are not 100% correct and never will be.
```

### melee-ft: 535 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 21452 | 1 | `melee_ft::fighter::assets::FighterAssets::load` |
| 1723 | 1 | `melee_ft::fighter::damage::DamageParameters::read` |
| 1682 | 1 | `melee_ft::fighter::shield::ShieldParameters::read` |
| 1374 | 1 | `melee_ft::desc::common::CommonFighterData::read` |
| 1231 | 1 | `melee_ft::dynamics::read_sets` |
| 1123 | 1 | `melee_ft::input::common::InputCommonData::read` |
| 1101 | 1 | `melee_ft::desc::playback::read_playback_motion` |
| 1042 | 1 | `melee_ft::desc::bones::read_fighter_bones` |
| 1000 | 1 | `melee_ft::fighter::spawn::<impl melee_ft::fighter::FighterCore>::prepare` |
| 929 | 1 | `melee_ft::desc::fox_attributes::FireFoxAttributes::read` |
| 885 | 1 | `melee_ft::desc::attributes::FighterAttributes::read` |
| 844 | 1 | `melee_ft::dynamics::read_motion_starts` |
| 760 | 1 | `melee_ft::anim::attach::select_motion` |
| 702 | 1 | `melee_ft::fighter::commands::CommandState::step_inner` |
| 682 | 1 | `melee_ft::anim::playback::FighterAnimation::resume_dynamic_subtree` |
| 659 | 1 | `melee_ft::desc::fox_attributes::IllusionAttributes::read` |
| 640 | 1 | `melee_ft::desc::bones::read_ground_pose` |
| 603 | 1 | `melee_ft::desc::bones::EcbBones::read` |
| 577 | 1 | `melee_ft::desc::fox_attributes::BlasterAttributes::read` |
| 550 | 1 | `melee_ft::desc::animation::read_entry` |

### melee-sim: 2015 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 8673 | 49 | `melee_ft::fighter::grab::capture_pair` |
| 8036 | 49 | `melee_ft::fighter::grab_throw::enter_back_throw` |
| 3087 | 49 | `melee_ft::fighter::damage::detect_hit` |
| 2548 | 49 | `melee_ft::fighter::grab_throw::release_back_throw` |
| 2450 | 7 | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::enter_shield` |
| 2422 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_input` |
| 2310 | 7 | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::shield_input` |
| 2198 | 7 | `melee_ft::fighter::walk::<impl melee_ft::fighter::Fighter<C>>::walk_input` |
| 2191 | 7 | `melee_ft::fighter::turn::<impl melee_ft::fighter::Fighter<C>>::turn_input` |
| 2114 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_animation` |
| 2107 | 7 | `melee_ft::fighter::squat::<impl melee_ft::fighter::Fighter<C>>::squat_input` |
| 1764 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::enter_cliff_catch` |
| 1666 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_collision` |
| 1652 | 7 | `melee_ft::fighter::multi_jump::<impl melee_ft::fighter::Fighter<C>>::enter_multi_jump` |
| 1624 | 7 | `melee_ft::fighter::spawn::<impl melee_ft::fighter::Fighter<C>>::initialize_spawn` |
| 1617 | 7 | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::damage_collision` |
| 1596 | 7 | `melee_ft::fighter::dash::<impl melee_ft::fighter::Fighter<C>>::dash_input` |
| 1540 | 7 | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::process_damage` |
| 1505 | 7 | `melee_ft::fighter::state::callbacks::collision::fall_collision` |
| 1484 | 7 | `melee_ft::fighter::down::<impl melee_ft::fighter::Fighter<C>>::enter_down_bound` |

### ft-captain: 2 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 83 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 65 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::from_archive` |

### ft-falco: 2 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 71 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 55 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_load` |

### ft-fox: 2 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 71 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 55 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_load` |

### ft-mario: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mars: 3 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 67 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 42 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 22 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::guard_variant` |

### ft-peach: 8 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 65 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 64 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::check_float_input` |
| 55 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 52 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 49 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_resources_loaded` |
| 15 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_landing` |

### ft-purin: 2 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 61 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 13 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |

### ft-yoshi: 22 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 194 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 179 | 1 | `melee_ft::fighter::escape::<impl melee_ft::fighter::Fighter<C>>::enter_escape` |
| 142 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 112 | 1 | `melee_ft::fighter::state::row::<impl melee_ft::fighter::Fighter<C>>::row` |
| 84 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_animated` |
| 72 | 1 | `melee_ft::fighter::spawn::<impl melee_ft::fighter::Fighter<C>>::change_motion_state_with_options` |
| 60 | 1 | `melee_ft::fighter::escape::<impl melee_ft::fighter::FighterCore>::roll_input` |
| 54 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 49 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 40 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_entered` |
| 28 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 18 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::action_id` |
| 16 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::input_shield` |
| 11 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::check_hurtbox_interaction` |
| 9 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::animate_shield` |
| 9 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::enter_guard_hold` |
| 9 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::enter_guard_off` |
| 9 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::enter_shield` |
| 6 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::escape_variant` |
| 2 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::escape_finished` |

<!-- perf-gate-v1
{"date": "2026-09-10T06:14:21+00:00", "metrics": {"copies": {"ft-captain": 2, "ft-falco": 2, "ft-fox": 2, "ft-mario": 0, "ft-mars": 3, "ft-peach": 8, "ft-purin": 2, "ft-yoshi": 22, "melee-ft": 535, "melee-sim": 2015}, "load_ns": 166188352.81162697, "stripped_bytes": 3747632, "text_bytes": 3342336, "ticks_600_ns": 28325143.03333333}, "platform": "macOS-26.2-arm64-arm-64bit-Mach-O", "revision": "cd6cfada40bc577865ae540a34aa34faf12d28f3", "rustc": "rustc 1.96.0 (ac68faa20 2026-05-25)", "status": "REGRESSION"}
-->

## 2026-09-10T06:15:23+00:00 — COMPLETE — PASS

Evidence: `/Users/nikhilunni/Projects/melee-lanes/chars/target/perf/20260910T061451Z-89933`. Revision `cd6cfada40bc577865ae540a34aa34faf12d28f3` (working tree included).

rustc 1.96.0 (ac68faa20 2026-05-25); macOS-26.2-arm64-arm-64bit-Mach-O.

Tolerance: time +10%, size +5%, copies +3 per compiling crate.
Baseline: 2026-09-10T05:24:01+00:00.

| Measurement | Value |
|---|---:|
| Stripped binary | 3,747,632 bytes |
| Text (`size`) | 3,342,336 bytes |
| load mean | 174.864 ms (95% CI 168.809..181.865 ms) |
| ticks_600 mean | 23.397 ms (95% CI 23.099..23.840 ms) |
| Headless throughput | 25,645 ticks/s |

Tool: `0.12.1`.

Tool: `cargo-llvm-lines 0.4.48`.

Per-crate text contribution (cargo-bloat estimates):
```text
File  .text     Size Crate
16.6%  34.2% 858.3KiB melee_sim
 8.6%  17.8% 446.9KiB std
 5.5%  11.3% 283.4KiB melee_ft
 5.2%  10.6% 266.1KiB clap_builder
 1.5%   3.2%  80.0KiB melee_ef
 1.3%   2.7%  67.8KiB slp
 1.2%   2.6%  64.2KiB melee_mp
 1.1%   2.4%  59.3KiB melee_gr
 1.0%   2.0%  50.8KiB hsd_anim
 1.0%   2.0%  50.7KiB hsd_archive
 0.8%   1.7%  41.7KiB toml
 0.7%   1.5%  37.2KiB serde_json
 0.7%   1.5%  36.5KiB toml_parser
 0.7%   1.4%  34.4KiB hsd_particle
 0.3%   0.7%  17.6KiB serde_core
 0.3%   0.6%  15.5KiB melee_diff
 0.3%   0.6%  15.4KiB melee_lb
 0.3%   0.6%  14.5KiB ft_yoshi
 0.3%   0.5%  13.6KiB hsd_gobj
 0.2%   0.4%  11.1KiB melee_types
 0.1%   0.3%   7.0KiB anyhow
 0.1%   0.2%   5.1KiB toml_datetime
 0.1%   0.2%   3.9KiB melee_cmd
 0.1%   0.1%   3.7KiB ft_peach
 0.1%   0.1%   3.1KiB [Unknown]
 0.1%   0.1%   3.0KiB ft_purin
 0.1%   0.1%   2.8KiB clap_lex
 0.1%   0.1%   2.7KiB gekko_math
 0.0%   0.1%   2.5KiB melee_coll
 0.0%   0.1%   2.3KiB ft_mars
 0.0%   0.1%   2.1KiB serde
 0.0%   0.1%   2.1KiB ft_captain
 0.0%   0.1%   2.1KiB melee_if
 0.0%   0.1%   1.7KiB anstream
 0.0%   0.1%   1.6KiB anstyle
 0.0%   0.1%   1.4KiB strsim
 0.0%   0.0%   1.0KiB zmij
 0.0%   0.0%     960B itoa
 0.0%   0.0%     848B toml_writer
 0.0%   0.0%     680B ft_fox
 0.0%   0.0%     680B ft_falco
 0.0%   0.0%     312B winnow
 0.0%   0.0%     132B hsd_types
 0.0%   0.0%      20B __rustc
 0.0%   0.0%      16B colorchoice
48.6% 100.0%   2.5MiB .text section size, the file size is 5.0MiB

Note: numbers above are a result of guesswork. They are not 100% correct and never will be.
```

### melee-ft: 535 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 21452 | 1 | `melee_ft::fighter::assets::FighterAssets::load` |
| 1723 | 1 | `melee_ft::fighter::damage::DamageParameters::read` |
| 1682 | 1 | `melee_ft::fighter::shield::ShieldParameters::read` |
| 1374 | 1 | `melee_ft::desc::common::CommonFighterData::read` |
| 1231 | 1 | `melee_ft::dynamics::read_sets` |
| 1123 | 1 | `melee_ft::input::common::InputCommonData::read` |
| 1101 | 1 | `melee_ft::desc::playback::read_playback_motion` |
| 1042 | 1 | `melee_ft::desc::bones::read_fighter_bones` |
| 1000 | 1 | `melee_ft::fighter::spawn::<impl melee_ft::fighter::FighterCore>::prepare` |
| 929 | 1 | `melee_ft::desc::fox_attributes::FireFoxAttributes::read` |
| 885 | 1 | `melee_ft::desc::attributes::FighterAttributes::read` |
| 844 | 1 | `melee_ft::dynamics::read_motion_starts` |
| 760 | 1 | `melee_ft::anim::attach::select_motion` |
| 702 | 1 | `melee_ft::fighter::commands::CommandState::step_inner` |
| 682 | 1 | `melee_ft::anim::playback::FighterAnimation::resume_dynamic_subtree` |
| 659 | 1 | `melee_ft::desc::fox_attributes::IllusionAttributes::read` |
| 640 | 1 | `melee_ft::desc::bones::read_ground_pose` |
| 603 | 1 | `melee_ft::desc::bones::EcbBones::read` |
| 577 | 1 | `melee_ft::desc::fox_attributes::BlasterAttributes::read` |
| 550 | 1 | `melee_ft::desc::animation::read_entry` |

### melee-sim: 2015 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 8673 | 49 | `melee_ft::fighter::grab::capture_pair` |
| 8036 | 49 | `melee_ft::fighter::grab_throw::enter_back_throw` |
| 3087 | 49 | `melee_ft::fighter::damage::detect_hit` |
| 2548 | 49 | `melee_ft::fighter::grab_throw::release_back_throw` |
| 2450 | 7 | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::enter_shield` |
| 2422 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_input` |
| 2310 | 7 | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::shield_input` |
| 2198 | 7 | `melee_ft::fighter::walk::<impl melee_ft::fighter::Fighter<C>>::walk_input` |
| 2191 | 7 | `melee_ft::fighter::turn::<impl melee_ft::fighter::Fighter<C>>::turn_input` |
| 2114 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_animation` |
| 2107 | 7 | `melee_ft::fighter::squat::<impl melee_ft::fighter::Fighter<C>>::squat_input` |
| 1764 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::enter_cliff_catch` |
| 1666 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_collision` |
| 1652 | 7 | `melee_ft::fighter::multi_jump::<impl melee_ft::fighter::Fighter<C>>::enter_multi_jump` |
| 1624 | 7 | `melee_ft::fighter::spawn::<impl melee_ft::fighter::Fighter<C>>::initialize_spawn` |
| 1617 | 7 | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::damage_collision` |
| 1596 | 7 | `melee_ft::fighter::dash::<impl melee_ft::fighter::Fighter<C>>::dash_input` |
| 1540 | 7 | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::process_damage` |
| 1505 | 7 | `melee_ft::fighter::state::callbacks::collision::fall_collision` |
| 1484 | 7 | `melee_ft::fighter::down::<impl melee_ft::fighter::Fighter<C>>::enter_down_bound` |

### ft-captain: 2 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 83 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 65 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::from_archive` |

### ft-falco: 2 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 71 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 55 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_load` |

### ft-fox: 2 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 71 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 55 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_load` |

### ft-mario: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mars: 3 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 67 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 42 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 22 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::guard_variant` |

### ft-peach: 8 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 65 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 64 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::check_float_input` |
| 55 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 52 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 49 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_resources_loaded` |
| 15 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_landing` |

### ft-purin: 2 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 61 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 13 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |

### ft-yoshi: 22 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 194 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 179 | 1 | `melee_ft::fighter::escape::<impl melee_ft::fighter::Fighter<C>>::enter_escape` |
| 142 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 112 | 1 | `melee_ft::fighter::state::row::<impl melee_ft::fighter::Fighter<C>>::row` |
| 84 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_animated` |
| 72 | 1 | `melee_ft::fighter::spawn::<impl melee_ft::fighter::Fighter<C>>::change_motion_state_with_options` |
| 60 | 1 | `melee_ft::fighter::escape::<impl melee_ft::fighter::FighterCore>::roll_input` |
| 54 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 49 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 40 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_entered` |
| 28 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 18 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::action_id` |
| 16 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::input_shield` |
| 11 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::check_hurtbox_interaction` |
| 9 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::animate_shield` |
| 9 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::enter_guard_hold` |
| 9 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::enter_guard_off` |
| 9 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::enter_shield` |
| 6 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::escape_variant` |
| 2 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::escape_finished` |

<!-- perf-gate-v1
{"date": "2026-09-10T06:15:23+00:00", "metrics": {"copies": {"ft-captain": 2, "ft-falco": 2, "ft-fox": 2, "ft-mario": 0, "ft-mars": 3, "ft-peach": 8, "ft-purin": 2, "ft-yoshi": 22, "melee-ft": 535, "melee-sim": 2015}, "load_ns": 174864065.50984126, "stripped_bytes": 3747632, "text_bytes": 3342336, "ticks_600_ns": 23396708.333333332}, "platform": "macOS-26.2-arm64-arm-64bit-Mach-O", "revision": "cd6cfada40bc577865ae540a34aa34faf12d28f3", "rustc": "rustc 1.96.0 (ac68faa20 2026-05-25)", "status": "PASS"}
-->

## 2026-09-10T06:15:55+00:00 — COMPLETE — PASS

Evidence: `/Users/nikhilunni/Projects/melee-lanes/chars/target/perf/20260910T061523Z-90501`. Revision `cd6cfada40bc577865ae540a34aa34faf12d28f3` (working tree included).

rustc 1.96.0 (ac68faa20 2026-05-25); macOS-26.2-arm64-arm-64bit-Mach-O.

Tolerance: time +10%, size +5%, copies +0 per compiling crate.
Baseline: 2026-09-10T06:15:23+00:00.

| Measurement | Value |
|---|---:|
| Stripped binary | 3,747,632 bytes |
| Text (`size`) | 3,342,336 bytes |
| load mean | 166.000 ms (95% CI 165.645..166.411 ms) |
| ticks_600 mean | 23.588 ms (95% CI 23.259..23.978 ms) |
| Headless throughput | 25,436 ticks/s |

Tool: `0.12.1`.

Tool: `cargo-llvm-lines 0.4.48`.

Per-crate text contribution (cargo-bloat estimates):
```text
File  .text     Size Crate
16.6%  34.2% 858.3KiB melee_sim
 8.6%  17.8% 446.9KiB std
 5.5%  11.3% 283.4KiB melee_ft
 5.2%  10.6% 266.1KiB clap_builder
 1.5%   3.2%  80.0KiB melee_ef
 1.3%   2.7%  67.8KiB slp
 1.2%   2.6%  64.2KiB melee_mp
 1.1%   2.4%  59.3KiB melee_gr
 1.0%   2.0%  50.8KiB hsd_anim
 1.0%   2.0%  50.7KiB hsd_archive
 0.8%   1.7%  41.7KiB toml
 0.7%   1.5%  37.2KiB serde_json
 0.7%   1.5%  36.5KiB toml_parser
 0.7%   1.4%  34.4KiB hsd_particle
 0.3%   0.7%  17.6KiB serde_core
 0.3%   0.6%  15.5KiB melee_diff
 0.3%   0.6%  15.4KiB melee_lb
 0.3%   0.6%  14.5KiB ft_yoshi
 0.3%   0.5%  13.6KiB hsd_gobj
 0.2%   0.4%  11.1KiB melee_types
 0.1%   0.3%   7.0KiB anyhow
 0.1%   0.2%   5.1KiB toml_datetime
 0.1%   0.2%   3.9KiB melee_cmd
 0.1%   0.1%   3.7KiB ft_peach
 0.1%   0.1%   3.1KiB [Unknown]
 0.1%   0.1%   3.0KiB ft_purin
 0.1%   0.1%   2.8KiB clap_lex
 0.1%   0.1%   2.7KiB gekko_math
 0.0%   0.1%   2.5KiB melee_coll
 0.0%   0.1%   2.3KiB ft_mars
 0.0%   0.1%   2.1KiB serde
 0.0%   0.1%   2.1KiB ft_captain
 0.0%   0.1%   2.1KiB melee_if
 0.0%   0.1%   1.7KiB anstream
 0.0%   0.1%   1.6KiB anstyle
 0.0%   0.1%   1.4KiB strsim
 0.0%   0.0%   1.0KiB zmij
 0.0%   0.0%     960B itoa
 0.0%   0.0%     848B toml_writer
 0.0%   0.0%     680B ft_fox
 0.0%   0.0%     680B ft_falco
 0.0%   0.0%     312B winnow
 0.0%   0.0%     132B hsd_types
 0.0%   0.0%      20B __rustc
 0.0%   0.0%      16B colorchoice
48.6% 100.0%   2.5MiB .text section size, the file size is 5.0MiB

Note: numbers above are a result of guesswork. They are not 100% correct and never will be.
```

### melee-ft: 535 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 21452 | 1 | `melee_ft::fighter::assets::FighterAssets::load` |
| 1723 | 1 | `melee_ft::fighter::damage::DamageParameters::read` |
| 1682 | 1 | `melee_ft::fighter::shield::ShieldParameters::read` |
| 1374 | 1 | `melee_ft::desc::common::CommonFighterData::read` |
| 1231 | 1 | `melee_ft::dynamics::read_sets` |
| 1123 | 1 | `melee_ft::input::common::InputCommonData::read` |
| 1101 | 1 | `melee_ft::desc::playback::read_playback_motion` |
| 1042 | 1 | `melee_ft::desc::bones::read_fighter_bones` |
| 1000 | 1 | `melee_ft::fighter::spawn::<impl melee_ft::fighter::FighterCore>::prepare` |
| 929 | 1 | `melee_ft::desc::fox_attributes::FireFoxAttributes::read` |
| 885 | 1 | `melee_ft::desc::attributes::FighterAttributes::read` |
| 844 | 1 | `melee_ft::dynamics::read_motion_starts` |
| 760 | 1 | `melee_ft::anim::attach::select_motion` |
| 702 | 1 | `melee_ft::fighter::commands::CommandState::step_inner` |
| 682 | 1 | `melee_ft::anim::playback::FighterAnimation::resume_dynamic_subtree` |
| 659 | 1 | `melee_ft::desc::fox_attributes::IllusionAttributes::read` |
| 640 | 1 | `melee_ft::desc::bones::read_ground_pose` |
| 603 | 1 | `melee_ft::desc::bones::EcbBones::read` |
| 577 | 1 | `melee_ft::desc::fox_attributes::BlasterAttributes::read` |
| 550 | 1 | `melee_ft::desc::animation::read_entry` |

### melee-sim: 2015 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 8673 | 49 | `melee_ft::fighter::grab::capture_pair` |
| 8036 | 49 | `melee_ft::fighter::grab_throw::enter_back_throw` |
| 3087 | 49 | `melee_ft::fighter::damage::detect_hit` |
| 2548 | 49 | `melee_ft::fighter::grab_throw::release_back_throw` |
| 2450 | 7 | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::enter_shield` |
| 2422 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_input` |
| 2310 | 7 | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::shield_input` |
| 2198 | 7 | `melee_ft::fighter::walk::<impl melee_ft::fighter::Fighter<C>>::walk_input` |
| 2191 | 7 | `melee_ft::fighter::turn::<impl melee_ft::fighter::Fighter<C>>::turn_input` |
| 2114 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_animation` |
| 2107 | 7 | `melee_ft::fighter::squat::<impl melee_ft::fighter::Fighter<C>>::squat_input` |
| 1764 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::enter_cliff_catch` |
| 1666 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_collision` |
| 1652 | 7 | `melee_ft::fighter::multi_jump::<impl melee_ft::fighter::Fighter<C>>::enter_multi_jump` |
| 1624 | 7 | `melee_ft::fighter::spawn::<impl melee_ft::fighter::Fighter<C>>::initialize_spawn` |
| 1617 | 7 | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::damage_collision` |
| 1596 | 7 | `melee_ft::fighter::dash::<impl melee_ft::fighter::Fighter<C>>::dash_input` |
| 1540 | 7 | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::process_damage` |
| 1505 | 7 | `melee_ft::fighter::state::callbacks::collision::fall_collision` |
| 1484 | 7 | `melee_ft::fighter::down::<impl melee_ft::fighter::Fighter<C>>::enter_down_bound` |

### ft-captain: 2 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 83 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 65 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::from_archive` |

### ft-falco: 2 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 71 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 55 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_load` |

### ft-fox: 2 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 71 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 55 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_load` |

### ft-mario: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mars: 3 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 67 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 42 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 22 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::guard_variant` |

### ft-peach: 8 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 65 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 64 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::check_float_input` |
| 55 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 52 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 49 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_resources_loaded` |
| 15 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_landing` |

### ft-purin: 2 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 61 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 13 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |

### ft-yoshi: 22 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 194 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 179 | 1 | `melee_ft::fighter::escape::<impl melee_ft::fighter::Fighter<C>>::enter_escape` |
| 142 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 112 | 1 | `melee_ft::fighter::state::row::<impl melee_ft::fighter::Fighter<C>>::row` |
| 84 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_animated` |
| 72 | 1 | `melee_ft::fighter::spawn::<impl melee_ft::fighter::Fighter<C>>::change_motion_state_with_options` |
| 60 | 1 | `melee_ft::fighter::escape::<impl melee_ft::fighter::FighterCore>::roll_input` |
| 54 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 49 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 40 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_entered` |
| 28 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 18 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::action_id` |
| 16 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::input_shield` |
| 11 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::check_hurtbox_interaction` |
| 9 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::animate_shield` |
| 9 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::enter_guard_hold` |
| 9 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::enter_guard_off` |
| 9 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::enter_shield` |
| 6 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::escape_variant` |
| 2 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::escape_finished` |

<!-- perf-gate-v1
{"date": "2026-09-10T06:15:55+00:00", "metrics": {"copies": {"ft-captain": 2, "ft-falco": 2, "ft-fox": 2, "ft-mario": 0, "ft-mars": 3, "ft-peach": 8, "ft-purin": 2, "ft-yoshi": 22, "melee-ft": 535, "melee-sim": 2015}, "load_ns": 166000202.4844841, "stripped_bytes": 3747632, "text_bytes": 3342336, "ticks_600_ns": 23588423.56666667}, "platform": "macOS-26.2-arm64-arm-64bit-Mach-O", "revision": "cd6cfada40bc577865ae540a34aa34faf12d28f3", "rustc": "rustc 1.96.0 (ac68faa20 2026-05-25)", "status": "PASS"}
-->

## 2026-09-10T07:23:11+00:00 — COMPLETE — REGRESSION

Evidence: `/Users/nikhilunni/Projects/melee-lanes/combat/target/perf/20260910T072233Z-63983`. Revision `d999eadeb67b1182f81936309b6a4393b1b0762c` (working tree included).

rustc 1.96.0 (ac68faa20 2026-05-25); macOS-26.2-arm64-arm-64bit-Mach-O.

Tolerance: time +10%, size +5%, copies +0 per compiling crate.
Baseline: 2026-09-10T06:15:55+00:00.

| Measurement | Value |
|---|---:|
| Stripped binary | 3,916,784 bytes |
| Text (`size`) | 3,489,792 bytes |
| load mean | 169.185 ms (95% CI 168.640..169.853 ms) |
| ticks_600 mean | 24.047 ms (95% CI 23.968..24.130 ms) |
| Headless throughput | 24,951 ticks/s |

- melee-sim melee-ft copies: 2112 > 2015 + 0

- ft-falco melee-ft copies: 22 > 2 + 0

- melee-ft melee-ft copies: 561 > 535 + 0

- ft-fox melee-ft copies: 22 > 2 + 0

Tool: `0.12.1`.

Tool: `cargo-llvm-lines 0.4.48`.

Per-crate text contribution (cargo-bloat estimates):
```text
File  .text     Size Crate
16.9%  35.0% 916.1KiB melee_sim
 8.3%  17.2% 450.2KiB std
 5.5%  11.5% 299.8KiB melee_ft
 4.9%  10.2% 266.2KiB clap_builder
 1.5%   3.1%  82.1KiB melee_ef
 1.3%   2.6%  67.8KiB slp
 1.2%   2.5%  64.2KiB melee_mp
 1.1%   2.3%  59.3KiB melee_gr
 0.9%   1.9%  50.8KiB hsd_anim
 0.9%   1.9%  50.7KiB hsd_archive
 0.8%   1.6%  41.7KiB toml
 0.7%   1.4%  37.6KiB serde_json
 0.7%   1.4%  37.1KiB hsd_particle
 0.7%   1.4%  36.5KiB toml_parser
 0.3%   0.7%  17.6KiB serde_core
 0.3%   0.6%  15.4KiB melee_lb
 0.3%   0.6%  15.4KiB melee_diff
 0.3%   0.6%  14.5KiB ft_yoshi
 0.3%   0.5%  13.7KiB hsd_gobj
 0.2%   0.4%  11.3KiB melee_types
 0.2%   0.4%  11.1KiB melee_it
 0.1%   0.3%   6.9KiB anyhow
 0.1%   0.3%   6.6KiB ft_fox_family
 0.1%   0.2%   5.1KiB toml_datetime
 0.1%   0.2%   4.1KiB melee_cmd
 0.1%   0.1%   3.7KiB ft_peach
 0.1%   0.1%   3.2KiB it_foxlaser
 0.1%   0.1%   3.1KiB [Unknown]
 0.1%   0.1%   3.0KiB ft_purin
 0.1%   0.1%   2.8KiB clap_lex
 0.0%   0.1%   2.7KiB gekko_math
 0.0%   0.1%   2.5KiB melee_coll
 0.0%   0.1%   2.3KiB ft_mars
 0.0%   0.1%   2.1KiB serde
 0.0%   0.1%   2.1KiB ft_captain
 0.0%   0.1%   2.1KiB melee_if
 0.0%   0.1%   1.7KiB anstream
 0.0%   0.1%   1.6KiB anstyle
 0.0%   0.1%   1.4KiB strsim
 0.0%   0.0%   1.3KiB ft_fox
 0.0%   0.0%   1.3KiB ft_falco
 0.0%   0.0%   1.0KiB zmij
 0.0%   0.0%     960B itoa
 0.0%   0.0%     848B toml_writer
 0.0%   0.0%     312B winnow
 0.0%   0.0%     264B hsd_types
 0.0%   0.0%      20B __rustc
 0.0%   0.0%      16B colorchoice
48.4% 100.0%   2.6MiB .text section size, the file size is 5.3MiB

Note: numbers above are a result of guesswork. They are not 100% correct and never will be.
```

### melee-ft: 561 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 21966 | 1 | `melee_ft::fighter::assets::FighterAssets::load` |
| 2411 | 1 | `melee_ft::fighter::damage::DamageParameters::read` |
| 1682 | 1 | `melee_ft::fighter::shield::ShieldParameters::read` |
| 1374 | 1 | `melee_ft::desc::common::CommonFighterData::read` |
| 1231 | 1 | `melee_ft::dynamics::read_sets` |
| 1123 | 1 | `melee_ft::input::common::InputCommonData::read` |
| 1101 | 1 | `melee_ft::desc::playback::read_playback_motion` |
| 1064 | 1 | `melee_ft::fighter::spawn::<impl melee_ft::fighter::FighterCore>::prepare` |
| 1042 | 1 | `melee_ft::desc::bones::read_fighter_bones` |
| 950 | 1 | `melee_ft::fighter::commands::CommandState::step_inner` |
| 929 | 1 | `melee_ft::desc::fox_attributes::FireFoxAttributes::read` |
| 885 | 1 | `melee_ft::desc::attributes::FighterAttributes::read` |
| 844 | 1 | `melee_ft::dynamics::read_motion_starts` |
| 760 | 1 | `melee_ft::anim::attach::select_motion` |
| 695 | 1 | `melee_ft::fighter::smash::read_overlay` |
| 682 | 1 | `melee_ft::anim::playback::FighterAnimation::resume_dynamic_subtree` |
| 659 | 1 | `melee_ft::desc::fox_attributes::IllusionAttributes::read` |
| 640 | 1 | `melee_ft::desc::bones::read_ground_pose` |
| 603 | 1 | `melee_ft::desc::bones::EcbBones::read` |
| 577 | 1 | `melee_ft::desc::fox_attributes::BlasterAttributes::read` |

### melee-sim: 2112 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 8673 | 49 | `melee_ft::fighter::grab::capture_pair` |
| 8036 | 49 | `melee_ft::fighter::grab_throw::enter_back_throw` |
| 3087 | 49 | `melee_ft::fighter::damage::detect_hit` |
| 2548 | 49 | `melee_ft::fighter::grab_throw::release_back_throw` |
| 2450 | 7 | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::enter_shield` |
| 2422 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_input` |
| 2310 | 7 | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::shield_input` |
| 2198 | 7 | `melee_ft::fighter::walk::<impl melee_ft::fighter::Fighter<C>>::walk_input` |
| 2191 | 7 | `melee_ft::fighter::turn::<impl melee_ft::fighter::Fighter<C>>::turn_input` |
| 2135 | 7 | `melee_ft::fighter::attack::<impl melee_ft::fighter::Fighter<C>>::jab_input` |
| 2114 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_animation` |
| 2107 | 7 | `melee_ft::fighter::squat::<impl melee_ft::fighter::Fighter<C>>::squat_input` |
| 1988 | 7 | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::damage_collision` |
| 1764 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::enter_cliff_catch` |
| 1715 | 7 | `melee_ft::fighter::attack::<impl melee_ft::fighter::Fighter<C>>::enter_ground_attack` |
| 1680 | 7 | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::damage_animation` |
| 1680 | 7 | `melee_ft::fighter::dash::<impl melee_ft::fighter::Fighter<C>>::dash_input` |
| 1666 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_collision` |
| 1652 | 7 | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::process_damage` |
| 1652 | 7 | `melee_ft::fighter::multi_jump::<impl melee_ft::fighter::Fighter<C>>::enter_multi_jump` |

### ft-captain: 2 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 83 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 65 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::from_archive` |

### ft-falco: 22 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 152 | 1 | `melee_ft::fighter::state::callbacks::collision::finish_ground` |
| 130 | 1 | `melee_ft::fighter::state::callbacks::collision::air_catch_hit` |
| 112 | 1 | `melee_ft::fighter::state::row::<impl melee_ft::fighter::Fighter<C>>::row` |
| 107 | 1 | `melee_ft::fighter::fall::<impl melee_ft::fighter::Fighter<C>>::enter_special_fall` |
| 79 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 72 | 1 | `melee_ft::fighter::spawn::<impl melee_ft::fighter::Fighter<C>>::change_motion_state_with_options` |
| 67 | 1 | `melee_ft::fighter::landing::<impl melee_ft::fighter::Fighter<C>>::enter_landing` |
| 55 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 50 | 1 | `melee_ft::fighter::state::callbacks::collision::ground_action` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 8 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::state::callbacks::camera::follow_fighter` |
| 2 | 1 | `melee_ft::fighter::state::callbacks::physics::guard_on` |
| 2 | 1 | `melee_ft::fighter::state::callbacks::physics::pass` |
| 1 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::descriptor` |
| 1 | 1 | `melee_ft::fighter::CharacterCallbacks::action_id` |

### ft-fox: 22 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 152 | 1 | `melee_ft::fighter::state::callbacks::collision::finish_ground` |
| 130 | 1 | `melee_ft::fighter::state::callbacks::collision::air_catch_hit` |
| 112 | 1 | `melee_ft::fighter::state::row::<impl melee_ft::fighter::Fighter<C>>::row` |
| 107 | 1 | `melee_ft::fighter::fall::<impl melee_ft::fighter::Fighter<C>>::enter_special_fall` |
| 79 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 72 | 1 | `melee_ft::fighter::spawn::<impl melee_ft::fighter::Fighter<C>>::change_motion_state_with_options` |
| 67 | 1 | `melee_ft::fighter::landing::<impl melee_ft::fighter::Fighter<C>>::enter_landing` |
| 55 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 50 | 1 | `melee_ft::fighter::state::callbacks::collision::ground_action` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 8 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::state::callbacks::camera::follow_fighter` |
| 2 | 1 | `melee_ft::fighter::state::callbacks::physics::guard_on` |
| 2 | 1 | `melee_ft::fighter::state::callbacks::physics::pass` |
| 1 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::descriptor` |
| 1 | 1 | `melee_ft::fighter::CharacterCallbacks::action_id` |

### ft-fox-family: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mario: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mars: 3 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 67 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 42 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 22 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::guard_variant` |

### ft-peach: 8 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 65 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 64 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::check_float_input` |
| 55 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 52 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 49 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_resources_loaded` |
| 15 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_landing` |

### ft-purin: 2 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 61 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 13 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |

### ft-yoshi: 22 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 194 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 179 | 1 | `melee_ft::fighter::escape::<impl melee_ft::fighter::Fighter<C>>::enter_escape` |
| 142 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 112 | 1 | `melee_ft::fighter::state::row::<impl melee_ft::fighter::Fighter<C>>::row` |
| 84 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_animated` |
| 72 | 1 | `melee_ft::fighter::spawn::<impl melee_ft::fighter::Fighter<C>>::change_motion_state_with_options` |
| 60 | 1 | `melee_ft::fighter::escape::<impl melee_ft::fighter::FighterCore>::roll_input` |
| 54 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 49 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 40 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_entered` |
| 28 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 18 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::action_id` |
| 16 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::input_shield` |
| 11 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::check_hurtbox_interaction` |
| 9 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::animate_shield` |
| 9 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::enter_guard_hold` |
| 9 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::enter_guard_off` |
| 9 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::enter_shield` |
| 6 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::escape_variant` |
| 2 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::escape_finished` |

<!-- perf-gate-v1
{"date": "2026-09-10T07:23:11+00:00", "metrics": {"copies": {"ft-captain": 2, "ft-falco": 22, "ft-fox": 22, "ft-fox-family": 0, "ft-mario": 0, "ft-mars": 3, "ft-peach": 8, "ft-purin": 2, "ft-yoshi": 22, "melee-ft": 561, "melee-sim": 2112}, "load_ns": 169184575.1818254, "stripped_bytes": 3916784, "text_bytes": 3489792, "ticks_600_ns": 24047193.199999996}, "platform": "macOS-26.2-arm64-arm-64bit-Mach-O", "revision": "d999eadeb67b1182f81936309b6a4393b1b0762c", "rustc": "rustc 1.96.0 (ac68faa20 2026-05-25)", "status": "REGRESSION"}
-->

## 2026-09-10T07:37:08+00:00 — COMPLETE — REGRESSION

Evidence: `/Users/nikhilunni/Projects/melee-lanes/combat/target/perf/20260910T073631Z-74693`. Revision `dc0b51ece8f9aece02b7dafbcbbd81afe9d8b35f` (working tree included).

rustc 1.96.0 (ac68faa20 2026-05-25); macOS-26.2-arm64-arm-64bit-Mach-O.

Tolerance: time +10%, size +5%, copies +0 per compiling crate.
Baseline: 2026-09-10T06:15:55+00:00.

| Measurement | Value |
|---|---:|
| Stripped binary | 3,916,784 bytes |
| Text (`size`) | 3,489,792 bytes |
| load mean | 167.691 ms (95% CI 166.843..168.739 ms) |
| ticks_600 mean | 24.580 ms (95% CI 23.988..25.295 ms) |
| Headless throughput | 24,410 ticks/s |

- ft-fox melee-ft copies: 22 > 2 + 0

- melee-sim melee-ft copies: 2112 > 2015 + 0

- ft-falco melee-ft copies: 22 > 2 + 0

- melee-ft melee-ft copies: 561 > 535 + 0

Tool: `0.12.1`.

Tool: `cargo-llvm-lines 0.4.48`.

Per-crate text contribution (cargo-bloat estimates):
```text
File  .text     Size Crate
16.9%  35.0% 916.1KiB melee_sim
 8.3%  17.2% 450.2KiB std
 5.5%  11.5% 299.8KiB melee_ft
 4.9%  10.2% 266.2KiB clap_builder
 1.5%   3.1%  82.1KiB melee_ef
 1.3%   2.6%  67.8KiB slp
 1.2%   2.5%  64.2KiB melee_mp
 1.1%   2.3%  59.3KiB melee_gr
 0.9%   1.9%  50.8KiB hsd_anim
 0.9%   1.9%  50.7KiB hsd_archive
 0.8%   1.6%  41.7KiB toml
 0.7%   1.4%  37.6KiB serde_json
 0.7%   1.4%  37.1KiB hsd_particle
 0.7%   1.4%  36.5KiB toml_parser
 0.3%   0.7%  17.6KiB serde_core
 0.3%   0.6%  15.4KiB melee_lb
 0.3%   0.6%  15.4KiB melee_diff
 0.3%   0.6%  14.5KiB ft_yoshi
 0.3%   0.5%  13.7KiB hsd_gobj
 0.2%   0.4%  11.3KiB melee_types
 0.2%   0.4%  11.1KiB melee_it
 0.1%   0.3%   6.9KiB anyhow
 0.1%   0.3%   6.6KiB ft_fox_family
 0.1%   0.2%   5.1KiB toml_datetime
 0.1%   0.2%   4.1KiB melee_cmd
 0.1%   0.1%   3.7KiB ft_peach
 0.1%   0.1%   3.2KiB it_foxlaser
 0.1%   0.1%   3.1KiB [Unknown]
 0.1%   0.1%   3.0KiB ft_purin
 0.1%   0.1%   2.8KiB clap_lex
 0.0%   0.1%   2.7KiB gekko_math
 0.0%   0.1%   2.5KiB melee_coll
 0.0%   0.1%   2.3KiB ft_mars
 0.0%   0.1%   2.1KiB serde
 0.0%   0.1%   2.1KiB ft_captain
 0.0%   0.1%   2.1KiB melee_if
 0.0%   0.1%   1.7KiB anstream
 0.0%   0.1%   1.6KiB anstyle
 0.0%   0.1%   1.4KiB strsim
 0.0%   0.0%   1.3KiB ft_falco
 0.0%   0.0%   1.3KiB ft_fox
 0.0%   0.0%   1.0KiB zmij
 0.0%   0.0%     960B itoa
 0.0%   0.0%     848B toml_writer
 0.0%   0.0%     312B winnow
 0.0%   0.0%     264B hsd_types
 0.0%   0.0%      20B __rustc
 0.0%   0.0%      16B colorchoice
48.4% 100.0%   2.6MiB .text section size, the file size is 5.3MiB

Note: numbers above are a result of guesswork. They are not 100% correct and never will be.
```

### melee-ft: 561 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 21966 | 1 | `melee_ft::fighter::assets::FighterAssets::load` |
| 2411 | 1 | `melee_ft::fighter::damage::DamageParameters::read` |
| 1682 | 1 | `melee_ft::fighter::shield::ShieldParameters::read` |
| 1374 | 1 | `melee_ft::desc::common::CommonFighterData::read` |
| 1231 | 1 | `melee_ft::dynamics::read_sets` |
| 1123 | 1 | `melee_ft::input::common::InputCommonData::read` |
| 1101 | 1 | `melee_ft::desc::playback::read_playback_motion` |
| 1064 | 1 | `melee_ft::fighter::spawn::<impl melee_ft::fighter::FighterCore>::prepare` |
| 1042 | 1 | `melee_ft::desc::bones::read_fighter_bones` |
| 950 | 1 | `melee_ft::fighter::commands::CommandState::step_inner` |
| 929 | 1 | `melee_ft::desc::fox_attributes::FireFoxAttributes::read` |
| 885 | 1 | `melee_ft::desc::attributes::FighterAttributes::read` |
| 844 | 1 | `melee_ft::dynamics::read_motion_starts` |
| 760 | 1 | `melee_ft::anim::attach::select_motion` |
| 695 | 1 | `melee_ft::fighter::smash::read_overlay` |
| 682 | 1 | `melee_ft::anim::playback::FighterAnimation::resume_dynamic_subtree` |
| 659 | 1 | `melee_ft::desc::fox_attributes::IllusionAttributes::read` |
| 640 | 1 | `melee_ft::desc::bones::read_ground_pose` |
| 603 | 1 | `melee_ft::desc::bones::EcbBones::read` |
| 577 | 1 | `melee_ft::desc::fox_attributes::BlasterAttributes::read` |

### melee-sim: 2112 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 8673 | 49 | `melee_ft::fighter::grab::capture_pair` |
| 8036 | 49 | `melee_ft::fighter::grab_throw::enter_back_throw` |
| 3087 | 49 | `melee_ft::fighter::damage::detect_hit` |
| 2548 | 49 | `melee_ft::fighter::grab_throw::release_back_throw` |
| 2450 | 7 | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::enter_shield` |
| 2422 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_input` |
| 2310 | 7 | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::shield_input` |
| 2198 | 7 | `melee_ft::fighter::walk::<impl melee_ft::fighter::Fighter<C>>::walk_input` |
| 2191 | 7 | `melee_ft::fighter::turn::<impl melee_ft::fighter::Fighter<C>>::turn_input` |
| 2135 | 7 | `melee_ft::fighter::attack::<impl melee_ft::fighter::Fighter<C>>::jab_input` |
| 2114 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_animation` |
| 2107 | 7 | `melee_ft::fighter::squat::<impl melee_ft::fighter::Fighter<C>>::squat_input` |
| 1988 | 7 | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::damage_collision` |
| 1764 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::enter_cliff_catch` |
| 1715 | 7 | `melee_ft::fighter::attack::<impl melee_ft::fighter::Fighter<C>>::enter_ground_attack` |
| 1680 | 7 | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::damage_animation` |
| 1680 | 7 | `melee_ft::fighter::dash::<impl melee_ft::fighter::Fighter<C>>::dash_input` |
| 1666 | 7 | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_collision` |
| 1652 | 7 | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::process_damage` |
| 1652 | 7 | `melee_ft::fighter::multi_jump::<impl melee_ft::fighter::Fighter<C>>::enter_multi_jump` |

### ft-captain: 2 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 83 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 65 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::from_archive` |

### ft-falco: 22 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 152 | 1 | `melee_ft::fighter::state::callbacks::collision::finish_ground` |
| 130 | 1 | `melee_ft::fighter::state::callbacks::collision::air_catch_hit` |
| 112 | 1 | `melee_ft::fighter::state::row::<impl melee_ft::fighter::Fighter<C>>::row` |
| 107 | 1 | `melee_ft::fighter::fall::<impl melee_ft::fighter::Fighter<C>>::enter_special_fall` |
| 79 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 72 | 1 | `melee_ft::fighter::spawn::<impl melee_ft::fighter::Fighter<C>>::change_motion_state_with_options` |
| 67 | 1 | `melee_ft::fighter::landing::<impl melee_ft::fighter::Fighter<C>>::enter_landing` |
| 55 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 50 | 1 | `melee_ft::fighter::state::callbacks::collision::ground_action` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 8 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::state::callbacks::camera::follow_fighter` |
| 2 | 1 | `melee_ft::fighter::state::callbacks::physics::guard_on` |
| 2 | 1 | `melee_ft::fighter::state::callbacks::physics::pass` |
| 1 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::descriptor` |
| 1 | 1 | `melee_ft::fighter::CharacterCallbacks::action_id` |

### ft-fox: 22 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 152 | 1 | `melee_ft::fighter::state::callbacks::collision::finish_ground` |
| 130 | 1 | `melee_ft::fighter::state::callbacks::collision::air_catch_hit` |
| 112 | 1 | `melee_ft::fighter::state::row::<impl melee_ft::fighter::Fighter<C>>::row` |
| 107 | 1 | `melee_ft::fighter::fall::<impl melee_ft::fighter::Fighter<C>>::enter_special_fall` |
| 79 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 72 | 1 | `melee_ft::fighter::spawn::<impl melee_ft::fighter::Fighter<C>>::change_motion_state_with_options` |
| 67 | 1 | `melee_ft::fighter::landing::<impl melee_ft::fighter::Fighter<C>>::enter_landing` |
| 55 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 50 | 1 | `melee_ft::fighter::state::callbacks::collision::ground_action` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 8 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::state::callbacks::camera::follow_fighter` |
| 2 | 1 | `melee_ft::fighter::state::callbacks::physics::guard_on` |
| 2 | 1 | `melee_ft::fighter::state::callbacks::physics::pass` |
| 1 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::descriptor` |
| 1 | 1 | `melee_ft::fighter::CharacterCallbacks::action_id` |

### ft-fox-family: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mario: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mars: 3 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 67 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 42 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 22 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::guard_variant` |

### ft-peach: 8 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 65 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 64 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::check_float_input` |
| 55 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 52 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 49 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_resources_loaded` |
| 15 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_landing` |

### ft-purin: 2 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 61 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 13 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |

### ft-yoshi: 22 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 194 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 179 | 1 | `melee_ft::fighter::escape::<impl melee_ft::fighter::Fighter<C>>::enter_escape` |
| 142 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 112 | 1 | `melee_ft::fighter::state::row::<impl melee_ft::fighter::Fighter<C>>::row` |
| 84 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_animated` |
| 72 | 1 | `melee_ft::fighter::spawn::<impl melee_ft::fighter::Fighter<C>>::change_motion_state_with_options` |
| 60 | 1 | `melee_ft::fighter::escape::<impl melee_ft::fighter::FighterCore>::roll_input` |
| 54 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 49 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 40 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_entered` |
| 28 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 18 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::action_id` |
| 16 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::input_shield` |
| 11 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::check_hurtbox_interaction` |
| 9 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::animate_shield` |
| 9 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::enter_guard_hold` |
| 9 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::enter_guard_off` |
| 9 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::enter_shield` |
| 6 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::escape_variant` |
| 2 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::escape_finished` |

<!-- perf-gate-v1
{"date": "2026-09-10T07:37:08+00:00", "metrics": {"copies": {"ft-captain": 2, "ft-falco": 22, "ft-fox": 22, "ft-fox-family": 0, "ft-mario": 0, "ft-mars": 3, "ft-peach": 8, "ft-purin": 2, "ft-yoshi": 22, "melee-ft": 561, "melee-sim": 2112}, "load_ns": 167691471.24309525, "stripped_bytes": 3916784, "text_bytes": 3489792, "ticks_600_ns": 24580394.433333334}, "platform": "macOS-26.2-arm64-arm-64bit-Mach-O", "revision": "dc0b51ece8f9aece02b7dafbcbbd81afe9d8b35f", "rustc": "rustc 1.96.0 (ac68faa20 2026-05-25)", "status": "REGRESSION"}
-->
