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
