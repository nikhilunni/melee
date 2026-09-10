# Performance regression history

Run `tools/perf-gate.sh` separately from `cargo gate`, on an otherwise idle
machine. It builds the native release CLI, strips a copy, runs `size` and
`cargo bloat --release -p melee-sim --bin melee-sim --crates -n 0`, counts
duplicate melee-ft definition labels with `cargo llvm-lines -p melee-ft --release --lib`,
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
Duplicate-label tolerance is fixed at **0**, within each compiling crate and
across crates. `PERF_COPIES_TOLERANCE` is retired and rejected if set. A within-crate
duplicate is one distinct llvm-lines label whose emitted count is greater than
one, regardless of how many instances it has. An across-crate duplicate is one
label defined by two or more compiling crates. The existing `melee_ft::` label
filter stays unchanged. Unique labels and total emitted definitions per crate
are informational, with no thresholds; adding concrete functions is allowed.

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
the total labels and emitted definitions per compiling crate, alongside both
duplicate counts. No count is inferred by dividing a
shared total by roster size. See the upstream [llvm-lines interpretation](https://github.com/dtolnay/cargo-llvm-lines#multicrate-projects)
and [cargo-bloat measurement limitations](https://github.com/RazrFalcon/cargo-bloat#usage).

## C15 reference and active duplication ceilings

The C15 COMPLETE PASS at **2026-09-10T08:10:28+00:00** records the baseline, with the
concrete shell on `82459f1` plus this uncommitted working tree.
All 1,017 workspace tests pass in both profiles (three existing ignores).
Evidence: `target/perf/20260910T080958Z-957`; full report: `PORT_NOTES/C15_CONCRETE_SHELL.md`.

| Metric | C15 measurement | Next-run ceiling |
|---|---:|---:|
| Stripped bytes | 3,483,104 | 3,657,259 (also capped at P1's 3,747,632) |
| Text bytes | 3,178,496 | 3,337,421 |
| Load mean | 165.646 ms | 182.210 ms |
| 600-tick mean | 23.533 ms | 25.886 ms |
| Throughput | 25,496 ticks/s | equivalent tick-time ceiling above |
| melee-ft duplicate labels | 20 | 20 |
| melee-sim duplicate labels | 7 | 7 |
| Each compiling character crate except ft-mario | 1 | 1 |
| ft-mario / ft-fox-family duplicate labels | 0 / 0 | 0 / 0 |
| Labels defined in multiple crates | 99 | 99 |
| Audited common labels across crates | 270 | exactly one definition each |

### Duplicate-label migration (S2 follow-up, 2026-09-10)

The baseline was recounted from **committed C15 `4bb9f07`**, exported with
read-only `git archive` to `/private/tmp/s2-c15-duplicate-baseline` and compiled
with a clean temporary target, rustc 1.96.0, and the same per-crate llvm-lines
commands. The older report's `82459f1` is C15's pre-commit parent; archiving that
revision alone does not reproduce its then-uncommitted concrete-shell change.
Raw recount logs: `/private/tmp/s2-c15-census`. The checked-in baseline and log
SHA-256 values are `tools/data/c15-duplicate-baseline.json`.

| Compiling crate | Labels (info) | Emitted definitions (info) | Duplicate labels |
|---|---:|---:|---:|
| melee-ft | 809 | 845 | 20 |
| melee-sim | 101 | 128 | 7 |
| ft-captain | 43 | 66 | 1 |
| ft-falco | 44 | 67 | 1 |
| ft-fox | 44 | 67 | 1 |
| ft-fox-family | 0 | 0 | 0 |
| ft-mario | 0 | 0 | 0 |
| ft-mars | 43 | 66 | 1 |
| ft-peach | 44 | 67 | 1 |
| ft-purin | 43 | 66 | 1 |
| ft-yoshi | 45 | 68 | 1 |

There are **99 labels defined in more than one crate**. These include existing
static adapters and library helpers; the separate concrete-common audit still
requires each common body exactly once and rejects generic fighter shells or
missing pair helpers. No selector, timing/size ceiling or common-body check was
relaxed. The census is versioned as `duplicate-labels-v1`. Pre-migration PASS
blocks supply timing and size baselines; their total-copy counts are not treated
as duplication counts. The checked-in C15 duplication ceilings apply on every
run, and later PASS blocks additionally ratchet duplication down. Failed and
incomplete runs cannot raise either baseline. Label ownership is saved in
`duplicate-definitions.json`; the common subset remains in
`common-definitions.json`. Historical blocks below retain their original metrics.

## Historical P1 reference

The COMPLETE PASS at **2026-09-10T06:15:55+00:00** was the pre-C15 baseline: P1 on
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

## 2026-09-10T07:43:35+00:00 — COMPLETE — REGRESSION

Evidence: `/Users/nikhilunni/Projects/melee-lanes/combat/target/perf/20260910T074240Z-77868`. Revision `82459f14972807e09a89c41d59ec2498e68ba35b` (working tree included).

rustc 1.96.0 (ac68faa20 2026-05-25); macOS-26.2-arm64-arm-64bit-Mach-O.

Tolerance: time +10%, size +5%, copies +0 per compiling crate.
Baseline: 2026-09-10T06:15:55+00:00.

| Measurement | Value |
|---|---:|
| Stripped binary | 3,916,784 bytes |
| Text (`size`) | 3,489,792 bytes |
| load mean | 166.264 ms (95% CI 166.076..166.456 ms) |
| ticks_600 mean | 23.867 ms (95% CI 23.690..24.106 ms) |
| Headless throughput | 25,139 ticks/s |

- ft-fox melee-ft copies: 22 > 2 + 0

- ft-falco melee-ft copies: 22 > 2 + 0

- melee-sim melee-ft copies: 2112 > 2015 + 0

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
{"date": "2026-09-10T07:43:35+00:00", "metrics": {"copies": {"ft-captain": 2, "ft-falco": 22, "ft-fox": 22, "ft-fox-family": 0, "ft-mario": 0, "ft-mars": 3, "ft-peach": 8, "ft-purin": 2, "ft-yoshi": 22, "melee-ft": 561, "melee-sim": 2112}, "load_ns": 166263815.21579364, "stripped_bytes": 3916784, "text_bytes": 3489792, "ticks_600_ns": 23866916.666666664}, "platform": "macOS-26.2-arm64-arm-64bit-Mach-O", "revision": "82459f14972807e09a89c41d59ec2498e68ba35b", "rustc": "rustc 1.96.0 (ac68faa20 2026-05-25)", "status": "REGRESSION"}
-->

## 2026-09-10T08:00:06+00:00 — COMPLETE — REGRESSION

Evidence: `/Users/nikhilunni/Projects/melee-lanes/combat/target/perf/20260910T075914Z-89504`. Revision `82459f14972807e09a89c41d59ec2498e68ba35b` (working tree included).

rustc 1.96.0 (ac68faa20 2026-05-25); macOS-26.2-arm64-arm-64bit-Mach-O.

Tolerance: time +10%, size +5%, copies +0 per compiling crate.
Baseline: 2026-09-10T06:15:55+00:00.

| Measurement | Value |
|---|---:|
| Stripped binary | 3,483,104 bytes |
| Text (`size`) | 3,178,496 bytes |
| load mean | 166.885 ms (95% CI 166.700..167.081 ms) |
| ticks_600 mean | 23.957 ms (95% CI 23.652..24.321 ms) |
| Headless throughput | 25,044 ticks/s |

- ft-captain melee-ft copies: 66 > 2 + 0

- melee-ft melee-ft copies: 846 > 535 + 0

- ft-purin melee-ft copies: 66 > 2 + 0

- ft-fox melee-ft copies: 67 > 2 + 0

- ft-falco melee-ft copies: 67 > 2 + 0

- ft-mars melee-ft copies: 66 > 3 + 0

- ft-yoshi melee-ft copies: 68 > 22 + 0

- ft-peach melee-ft copies: 67 > 8 + 0

Tool: `0.12.1`.

Tool: `cargo-llvm-lines 0.4.48`.

Per-crate text contribution (cargo-bloat estimates):
```text
File  .text     Size Crate
12.8%  25.9% 614.3KiB melee_sim
 9.8%  19.7% 467.9KiB std
 7.1%  14.3% 340.0KiB melee_ft
 5.6%  11.2% 266.1KiB clap_builder
 1.7%   3.5%  82.1KiB melee_ef
 1.4%   2.8%  67.6KiB slp
 1.3%   2.7%  64.2KiB melee_mp
 1.2%   2.5%  59.3KiB melee_gr
 1.1%   2.1%  50.8KiB hsd_anim
 1.1%   2.1%  50.7KiB hsd_archive
 0.9%   1.7%  41.5KiB toml
 0.8%   1.7%  39.3KiB hsd_particle
 0.8%   1.5%  36.5KiB toml_parser
 0.7%   1.5%  35.5KiB serde_json
 0.4%   0.7%  16.9KiB serde_core
 0.3%   0.7%  15.4KiB melee_diff
 0.3%   0.6%  14.6KiB melee_lb
 0.3%   0.6%  14.6KiB ft_yoshi
 0.3%   0.6%  13.5KiB hsd_gobj
 0.2%   0.5%  11.1KiB melee_it
 0.2%   0.5%  10.8KiB melee_types
 0.2%   0.3%   8.0KiB ft_fox_family
 0.1%   0.3%   6.9KiB anyhow
 0.1%   0.2%   5.1KiB toml_datetime
 0.1%   0.2%   4.1KiB melee_cmd
 0.1%   0.1%   3.4KiB ft_peach
 0.1%   0.1%   3.4KiB melee_coll
 0.1%   0.1%   3.2KiB [Unknown]
 0.1%   0.1%   3.2KiB it_foxlaser
 0.1%   0.1%   3.1KiB ft_purin
 0.1%   0.1%   2.8KiB clap_lex
 0.1%   0.1%   2.7KiB gekko_math
 0.0%   0.1%   2.3KiB ft_mars
 0.0%   0.1%   2.2KiB ft_captain
 0.0%   0.1%   2.1KiB serde
 0.0%   0.1%   2.1KiB melee_if
 0.0%   0.1%   1.7KiB anstream
 0.0%   0.1%   1.6KiB anstyle
 0.0%   0.1%   1.6KiB ft_fox
 0.0%   0.1%   1.6KiB ft_falco
 0.0%   0.1%   1.4KiB strsim
 0.0%   0.0%   1.0KiB zmij
 0.0%   0.0%     960B itoa
 0.0%   0.0%     848B toml_writer
 0.0%   0.0%     312B winnow
 0.0%   0.0%     264B hsd_types
 0.0%   0.0%      20B __rustc
 0.0%   0.0%      16B colorchoice
49.6% 100.0%   2.3MiB .text section size, the file size is 4.7MiB

Note: numbers above are a result of guesswork. They are not 100% correct and never will be.
```

### melee-ft: 846 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 21966 | 1 | `melee_ft::fighter::assets::FighterAssets::load` |
| 2411 | 1 | `melee_ft::fighter::damage::DamageParameters::read` |
| 2091 | 1 | `melee_ft::fighter::state::common_table::common_table` |
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

### melee-sim: 128 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 478 | 7 | `melee_ft::fighter::character::CharacterState::new` |
| 282 | 1 | `melee_ft::anim::playback::FighterAnimation::advance_main` |
| 252 | 1 | `melee_ft::anim::root_motion::RootMotion::evaluate` |
| 229 | 1 | `melee_ft::anim::blend::blend_rotation` |
| 205 | 1 | `core::ptr::drop_in_place<melee_ft::fighter::assets::FighterAssets>` |
| 164 | 1 | `melee_ft::anim::playback::FighterAnimation::advance_parts` |
| 130 | 1 | `melee_ft::anim::blend::blend_pose` |
| 110 | 1 | `melee_ft::anim::playback::animate_parts` |
| 109 | 1 | `core::ptr::drop_in_place<melee_ft::fighter::FighterCore>` |
| 72 | 1 | `<melee_ft::fighter::state::FighterProc as core::fmt::Debug>::fmt` |
| 56 | 1 | `<melee_ft::input::pad::PadSample as core::cmp::PartialEq>::eq` |
| 54 | 1 | `<melee_ft::desc::animation::AnimationDescError as core::fmt::Debug>::fmt` |
| 48 | 1 | `core::ptr::drop_in_place<melee_ft::desc::bones::FighterBones>` |
| 41 | 1 | `core::ptr::drop_in_place<[core::option::Option<melee_ft::desc::bones::AnimationBoneSet>; 5]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::desc::animation::AnimationEntry]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::dynamics::DynamicSetDescriptor]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::fighter::assets::FighterAssets; 2]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::fighter::assets::FighterAssets]>` |
| 38 | 1 | `core::ptr::drop_in_place<melee_ft::anim::playback::FighterAnimation>` |
| 37 | 1 | `core::ptr::drop_in_place<melee_ft::anim::attach::MotionRemap>` |

### ft-captain: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 83 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 65 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 7 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 5 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_shield` |

### ft-falco: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 79 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 55 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 9 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |

### ft-fox: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 79 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 55 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 9 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |

### ft-fox-family: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mario: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mars: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 67 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 42 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::guard_variant` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 5 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 3 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_landing` |
| 3 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |

### ft-peach: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 65 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 64 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::check_float_input` |
| 55 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 52 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 49 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_resources_loaded` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 15 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 10 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::dynamics_first_force_bone` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |

### ft-purin: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 61 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 4 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::multi_jump_attributes` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::multi_jump_animation` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_shield` |

### ft-yoshi: 68 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 194 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 142 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 87 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_animated` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 60 | 1 | `melee_ft::fighter::escape::<impl melee_ft::fighter::FighterCore>::roll_input` |
| 54 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 49 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 41 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_entered` |
| 28 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 18 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::action_id` |
| 17 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::input_shield` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 11 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::check_hurtbox_interaction` |

<!-- perf-gate-v1
{"date": "2026-09-10T08:00:06+00:00", "metrics": {"copies": {"ft-captain": 66, "ft-falco": 67, "ft-fox": 67, "ft-fox-family": 0, "ft-mario": 0, "ft-mars": 66, "ft-peach": 67, "ft-purin": 66, "ft-yoshi": 68, "melee-ft": 846, "melee-sim": 128}, "load_ns": 166884751.40115076, "stripped_bytes": 3483104, "text_bytes": 3178496, "ticks_600_ns": 23957468.133333333}, "platform": "macOS-26.2-arm64-arm-64bit-Mach-O", "revision": "82459f14972807e09a89c41d59ec2498e68ba35b", "rustc": "rustc 1.96.0 (ac68faa20 2026-05-25)", "status": "REGRESSION"}
-->

## 2026-09-10T08:08:48+00:00 — COMPLETE — PASS

Evidence: `/Users/nikhilunni/Projects/melee-lanes/combat/target/perf/20260910T080756Z-99078`. Revision `82459f14972807e09a89c41d59ec2498e68ba35b` (working tree included).

rustc 1.96.0 (ac68faa20 2026-05-25); macOS-26.2-arm64-arm-64bit-Mach-O.

Tolerance: time +10%, size +5%, copies +0 per compiling crate.
Baseline: 2026-09-10T06:15:55+00:00.
C15 fixed ceilings: 3,747,632 stripped bytes; 1,441 total copies; one definition per common label.

| Measurement | Value |
|---|---:|
| Stripped binary | 3,483,104 bytes |
| Text (`size`) | 3,178,496 bytes |
| load mean | 166.867 ms (95% CI 166.540..167.301 ms) |
| ticks_600 mean | 23.946 ms (95% CI 23.836..24.063 ms) |
| Headless throughput | 25,056 ticks/s |
| Total melee-ft copies | 1,440 |
| Common definition labels audited across crates | 270 |

Tool: `0.12.1`.

Tool: `cargo-llvm-lines 0.4.48`.

Per-crate text contribution (cargo-bloat estimates):
```text
File  .text     Size Crate
12.8%  25.9% 614.3KiB melee_sim
 9.8%  19.7% 467.9KiB std
 7.1%  14.3% 340.0KiB melee_ft
 5.6%  11.2% 266.1KiB clap_builder
 1.7%   3.5%  82.1KiB melee_ef
 1.4%   2.8%  67.6KiB slp
 1.3%   2.7%  64.2KiB melee_mp
 1.2%   2.5%  59.3KiB melee_gr
 1.1%   2.1%  50.8KiB hsd_anim
 1.1%   2.1%  50.7KiB hsd_archive
 0.9%   1.7%  41.5KiB toml
 0.8%   1.7%  39.3KiB hsd_particle
 0.8%   1.5%  36.5KiB toml_parser
 0.7%   1.5%  35.5KiB serde_json
 0.4%   0.7%  16.9KiB serde_core
 0.3%   0.7%  15.4KiB melee_diff
 0.3%   0.6%  14.6KiB melee_lb
 0.3%   0.6%  14.6KiB ft_yoshi
 0.3%   0.6%  13.5KiB hsd_gobj
 0.2%   0.5%  11.1KiB melee_it
 0.2%   0.5%  10.8KiB melee_types
 0.2%   0.3%   8.0KiB ft_fox_family
 0.1%   0.3%   6.9KiB anyhow
 0.1%   0.2%   5.1KiB toml_datetime
 0.1%   0.2%   4.1KiB melee_cmd
 0.1%   0.1%   3.4KiB ft_peach
 0.1%   0.1%   3.4KiB melee_coll
 0.1%   0.1%   3.2KiB [Unknown]
 0.1%   0.1%   3.2KiB it_foxlaser
 0.1%   0.1%   3.1KiB ft_purin
 0.1%   0.1%   2.8KiB clap_lex
 0.1%   0.1%   2.7KiB gekko_math
 0.0%   0.1%   2.3KiB ft_mars
 0.0%   0.1%   2.2KiB ft_captain
 0.0%   0.1%   2.1KiB serde
 0.0%   0.1%   2.1KiB melee_if
 0.0%   0.1%   1.7KiB anstream
 0.0%   0.1%   1.6KiB anstyle
 0.0%   0.1%   1.6KiB ft_falco
 0.0%   0.1%   1.6KiB ft_fox
 0.0%   0.1%   1.4KiB strsim
 0.0%   0.0%   1.0KiB zmij
 0.0%   0.0%     960B itoa
 0.0%   0.0%     848B toml_writer
 0.0%   0.0%     312B winnow
 0.0%   0.0%     264B hsd_types
 0.0%   0.0%      20B __rustc
 0.0%   0.0%      16B colorchoice
49.6% 100.0%   2.3MiB .text section size, the file size is 4.7MiB

Note: numbers above are a result of guesswork. They are not 100% correct and never will be.
```

### melee-ft: 845 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 21966 | 1 | `melee_ft::fighter::assets::FighterAssets::load` |
| 2411 | 1 | `melee_ft::fighter::damage::DamageParameters::read` |
| 2091 | 1 | `melee_ft::fighter::state::common_table::common_table` |
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

### melee-sim: 128 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 478 | 7 | `melee_ft::fighter::character::CharacterState::new` |
| 282 | 1 | `melee_ft::anim::playback::FighterAnimation::advance_main` |
| 252 | 1 | `melee_ft::anim::root_motion::RootMotion::evaluate` |
| 229 | 1 | `melee_ft::anim::blend::blend_rotation` |
| 205 | 1 | `core::ptr::drop_in_place<melee_ft::fighter::assets::FighterAssets>` |
| 164 | 1 | `melee_ft::anim::playback::FighterAnimation::advance_parts` |
| 130 | 1 | `melee_ft::anim::blend::blend_pose` |
| 110 | 1 | `melee_ft::anim::playback::animate_parts` |
| 109 | 1 | `core::ptr::drop_in_place<melee_ft::fighter::FighterCore>` |
| 72 | 1 | `<melee_ft::fighter::state::FighterProc as core::fmt::Debug>::fmt` |
| 56 | 1 | `<melee_ft::input::pad::PadSample as core::cmp::PartialEq>::eq` |
| 54 | 1 | `<melee_ft::desc::animation::AnimationDescError as core::fmt::Debug>::fmt` |
| 48 | 1 | `core::ptr::drop_in_place<melee_ft::desc::bones::FighterBones>` |
| 41 | 1 | `core::ptr::drop_in_place<[core::option::Option<melee_ft::desc::bones::AnimationBoneSet>; 5]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::desc::animation::AnimationEntry]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::dynamics::DynamicSetDescriptor]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::fighter::assets::FighterAssets; 2]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::fighter::assets::FighterAssets]>` |
| 38 | 1 | `core::ptr::drop_in_place<melee_ft::anim::playback::FighterAnimation>` |
| 37 | 1 | `core::ptr::drop_in_place<melee_ft::anim::attach::MotionRemap>` |

### ft-captain: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 83 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 65 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 7 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 5 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_shield` |

### ft-falco: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 79 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 55 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 9 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |

### ft-fox: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 79 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 55 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 9 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |

### ft-fox-family: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mario: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mars: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 67 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 42 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::guard_variant` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 5 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 3 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_landing` |
| 3 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |

### ft-peach: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 65 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 64 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::check_float_input` |
| 55 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 52 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 49 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_resources_loaded` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 15 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 10 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::dynamics_first_force_bone` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |

### ft-purin: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 61 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 4 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::multi_jump_attributes` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::multi_jump_animation` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_shield` |

### ft-yoshi: 68 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 194 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 142 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 87 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_animated` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 60 | 1 | `melee_ft::fighter::escape::<impl melee_ft::fighter::FighterCore>::roll_input` |
| 54 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 49 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 41 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_entered` |
| 28 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 18 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::action_id` |
| 17 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::input_shield` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 11 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::check_hurtbox_interaction` |

<!-- perf-gate-v1
{"architecture": "concrete-shell-v1", "date": "2026-09-10T08:08:48+00:00", "metrics": {"copies": {"ft-captain": 66, "ft-falco": 67, "ft-fox": 67, "ft-fox-family": 0, "ft-mario": 0, "ft-mars": 66, "ft-peach": 67, "ft-purin": 66, "ft-yoshi": 68, "melee-ft": 845, "melee-sim": 128}, "load_ns": 166867234.74603173, "stripped_bytes": 3483104, "text_bytes": 3178496, "ticks_600_ns": 23946301.4}, "platform": "macOS-26.2-arm64-arm-64bit-Mach-O", "revision": "82459f14972807e09a89c41d59ec2498e68ba35b", "rustc": "rustc 1.96.0 (ac68faa20 2026-05-25)", "status": "PASS"}
-->

## 2026-09-10T08:10:28+00:00 — COMPLETE — PASS

Evidence: `/Users/nikhilunni/Projects/melee-lanes/combat/target/perf/20260910T080958Z-957`. Revision `82459f14972807e09a89c41d59ec2498e68ba35b` (working tree included).

rustc 1.96.0 (ac68faa20 2026-05-25); macOS-26.2-arm64-arm-64bit-Mach-O.

Tolerance: time +10%, size +5%, copies +0 per compiling crate.
Baseline: 2026-09-10T08:08:48+00:00.
C15 fixed ceilings: 3,747,632 stripped bytes; 1,440 total copies; one definition per common label.

| Measurement | Value |
|---|---:|
| Stripped binary | 3,483,104 bytes |
| Text (`size`) | 3,178,496 bytes |
| load mean | 165.646 ms (95% CI 165.459..165.846 ms) |
| ticks_600 mean | 23.533 ms (95% CI 23.455..23.621 ms) |
| Headless throughput | 25,496 ticks/s |
| Total melee-ft copies | 1,440 |
| Common definition labels audited across crates | 270 |

Tool: `0.12.1`.

Tool: `cargo-llvm-lines 0.4.48`.

Per-crate text contribution (cargo-bloat estimates):
```text
File  .text     Size Crate
12.8%  25.9% 614.3KiB melee_sim
 9.8%  19.7% 467.9KiB std
 7.1%  14.3% 340.0KiB melee_ft
 5.6%  11.2% 266.1KiB clap_builder
 1.7%   3.5%  82.1KiB melee_ef
 1.4%   2.8%  67.6KiB slp
 1.3%   2.7%  64.2KiB melee_mp
 1.2%   2.5%  59.3KiB melee_gr
 1.1%   2.1%  50.8KiB hsd_anim
 1.1%   2.1%  50.7KiB hsd_archive
 0.9%   1.7%  41.5KiB toml
 0.8%   1.7%  39.3KiB hsd_particle
 0.8%   1.5%  36.5KiB toml_parser
 0.7%   1.5%  35.5KiB serde_json
 0.4%   0.7%  16.9KiB serde_core
 0.3%   0.7%  15.4KiB melee_diff
 0.3%   0.6%  14.6KiB melee_lb
 0.3%   0.6%  14.6KiB ft_yoshi
 0.3%   0.6%  13.5KiB hsd_gobj
 0.2%   0.5%  11.1KiB melee_it
 0.2%   0.5%  10.8KiB melee_types
 0.2%   0.3%   8.0KiB ft_fox_family
 0.1%   0.3%   6.9KiB anyhow
 0.1%   0.2%   5.1KiB toml_datetime
 0.1%   0.2%   4.1KiB melee_cmd
 0.1%   0.1%   3.4KiB ft_peach
 0.1%   0.1%   3.4KiB melee_coll
 0.1%   0.1%   3.2KiB it_foxlaser
 0.1%   0.1%   3.2KiB [Unknown]
 0.1%   0.1%   3.1KiB ft_purin
 0.1%   0.1%   2.8KiB clap_lex
 0.1%   0.1%   2.7KiB gekko_math
 0.0%   0.1%   2.3KiB ft_mars
 0.0%   0.1%   2.2KiB ft_captain
 0.0%   0.1%   2.1KiB serde
 0.0%   0.1%   2.1KiB melee_if
 0.0%   0.1%   1.7KiB anstream
 0.0%   0.1%   1.6KiB anstyle
 0.0%   0.1%   1.6KiB ft_falco
 0.0%   0.1%   1.6KiB ft_fox
 0.0%   0.1%   1.4KiB strsim
 0.0%   0.0%   1.0KiB zmij
 0.0%   0.0%     960B itoa
 0.0%   0.0%     848B toml_writer
 0.0%   0.0%     312B winnow
 0.0%   0.0%     264B hsd_types
 0.0%   0.0%      20B __rustc
 0.0%   0.0%      16B colorchoice
49.6% 100.0%   2.3MiB .text section size, the file size is 4.7MiB

Note: numbers above are a result of guesswork. They are not 100% correct and never will be.
```

### melee-ft: 845 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 21966 | 1 | `melee_ft::fighter::assets::FighterAssets::load` |
| 2411 | 1 | `melee_ft::fighter::damage::DamageParameters::read` |
| 2091 | 1 | `melee_ft::fighter::state::common_table::common_table` |
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

### melee-sim: 128 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 478 | 7 | `melee_ft::fighter::character::CharacterState::new` |
| 282 | 1 | `melee_ft::anim::playback::FighterAnimation::advance_main` |
| 252 | 1 | `melee_ft::anim::root_motion::RootMotion::evaluate` |
| 229 | 1 | `melee_ft::anim::blend::blend_rotation` |
| 205 | 1 | `core::ptr::drop_in_place<melee_ft::fighter::assets::FighterAssets>` |
| 164 | 1 | `melee_ft::anim::playback::FighterAnimation::advance_parts` |
| 130 | 1 | `melee_ft::anim::blend::blend_pose` |
| 110 | 1 | `melee_ft::anim::playback::animate_parts` |
| 109 | 1 | `core::ptr::drop_in_place<melee_ft::fighter::FighterCore>` |
| 72 | 1 | `<melee_ft::fighter::state::FighterProc as core::fmt::Debug>::fmt` |
| 56 | 1 | `<melee_ft::input::pad::PadSample as core::cmp::PartialEq>::eq` |
| 54 | 1 | `<melee_ft::desc::animation::AnimationDescError as core::fmt::Debug>::fmt` |
| 48 | 1 | `core::ptr::drop_in_place<melee_ft::desc::bones::FighterBones>` |
| 41 | 1 | `core::ptr::drop_in_place<[core::option::Option<melee_ft::desc::bones::AnimationBoneSet>; 5]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::desc::animation::AnimationEntry]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::dynamics::DynamicSetDescriptor]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::fighter::assets::FighterAssets; 2]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::fighter::assets::FighterAssets]>` |
| 38 | 1 | `core::ptr::drop_in_place<melee_ft::anim::playback::FighterAnimation>` |
| 37 | 1 | `core::ptr::drop_in_place<melee_ft::anim::attach::MotionRemap>` |

### ft-captain: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 83 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 65 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 7 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 5 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_shield` |

### ft-falco: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 79 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 55 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 9 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |

### ft-fox: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 79 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 55 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 9 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |

### ft-fox-family: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mario: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mars: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 67 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 42 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::guard_variant` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 5 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 3 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_landing` |
| 3 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |

### ft-peach: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 65 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 64 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::check_float_input` |
| 55 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 52 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 49 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_resources_loaded` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 15 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 10 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::dynamics_first_force_bone` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |

### ft-purin: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 61 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 4 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::multi_jump_attributes` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::multi_jump_animation` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_shield` |

### ft-yoshi: 68 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 194 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 142 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 87 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_animated` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 60 | 1 | `melee_ft::fighter::escape::<impl melee_ft::fighter::FighterCore>::roll_input` |
| 54 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 49 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 41 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_entered` |
| 28 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 18 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::action_id` |
| 17 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::input_shield` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 11 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::check_hurtbox_interaction` |

<!-- perf-gate-v1
{"architecture": "concrete-shell-v1", "date": "2026-09-10T08:10:28+00:00", "metrics": {"copies": {"ft-captain": 66, "ft-falco": 67, "ft-fox": 67, "ft-fox-family": 0, "ft-mario": 0, "ft-mars": 66, "ft-peach": 67, "ft-purin": 66, "ft-yoshi": 68, "melee-ft": 845, "melee-sim": 128}, "load_ns": 165645617.16011906, "stripped_bytes": 3483104, "text_bytes": 3178496, "ticks_600_ns": 23533140.1}, "platform": "macOS-26.2-arm64-arm-64bit-Mach-O", "revision": "82459f14972807e09a89c41d59ec2498e68ba35b", "rustc": "rustc 1.96.0 (ac68faa20 2026-05-25)", "status": "PASS"}
-->

## 2026-09-10T08:26:32+00:00 — COMPLETE — PASS

Evidence: `/Users/nikhilunni/Projects/melee-lanes/combat/target/perf/20260910T082550Z-11098`. Revision `8d6e2eeb3926dd095ee24f8fb98d152172437569` (working tree included).

rustc 1.96.0 (ac68faa20 2026-05-25); macOS-26.2-arm64-arm-64bit-Mach-O.

Tolerance: time +10%, size +5%, copies +0 per compiling crate.
Baseline: 2026-09-10T08:10:28+00:00.
C15 fixed ceilings: 3,747,632 stripped bytes; 1,440 total copies; one definition per common label.

| Measurement | Value |
|---|---:|
| Stripped binary | 3,483,104 bytes |
| Text (`size`) | 3,178,496 bytes |
| load mean | 168.174 ms (95% CI 167.755..168.546 ms) |
| ticks_600 mean | 24.040 ms (95% CI 24.002..24.076 ms) |
| Headless throughput | 24,958 ticks/s |
| Total melee-ft copies | 1,440 |
| Common definition labels audited across crates | 270 |

Tool: `0.12.1`.

Tool: `cargo-llvm-lines 0.4.48`.

Per-crate text contribution (cargo-bloat estimates):
```text
File  .text     Size Crate
12.8%  25.9% 614.3KiB melee_sim
 9.8%  19.7% 467.9KiB std
 7.1%  14.3% 340.0KiB melee_ft
 5.6%  11.2% 266.1KiB clap_builder
 1.7%   3.5%  82.1KiB melee_ef
 1.4%   2.8%  67.6KiB slp
 1.3%   2.7%  64.2KiB melee_mp
 1.2%   2.5%  59.3KiB melee_gr
 1.1%   2.1%  50.8KiB hsd_anim
 1.1%   2.1%  50.7KiB hsd_archive
 0.9%   1.7%  41.5KiB toml
 0.8%   1.7%  39.3KiB hsd_particle
 0.8%   1.5%  36.5KiB toml_parser
 0.7%   1.5%  35.5KiB serde_json
 0.4%   0.7%  16.9KiB serde_core
 0.3%   0.7%  15.4KiB melee_diff
 0.3%   0.6%  14.6KiB melee_lb
 0.3%   0.6%  14.6KiB ft_yoshi
 0.3%   0.6%  13.5KiB hsd_gobj
 0.2%   0.5%  11.1KiB melee_it
 0.2%   0.5%  10.8KiB melee_types
 0.2%   0.3%   8.0KiB ft_fox_family
 0.1%   0.3%   6.9KiB anyhow
 0.1%   0.2%   5.1KiB toml_datetime
 0.1%   0.2%   4.1KiB melee_cmd
 0.1%   0.1%   3.4KiB ft_peach
 0.1%   0.1%   3.4KiB melee_coll
 0.1%   0.1%   3.2KiB [Unknown]
 0.1%   0.1%   3.2KiB it_foxlaser
 0.1%   0.1%   3.1KiB ft_purin
 0.1%   0.1%   2.8KiB clap_lex
 0.1%   0.1%   2.7KiB gekko_math
 0.0%   0.1%   2.3KiB ft_mars
 0.0%   0.1%   2.2KiB ft_captain
 0.0%   0.1%   2.1KiB serde
 0.0%   0.1%   2.1KiB melee_if
 0.0%   0.1%   1.7KiB anstream
 0.0%   0.1%   1.6KiB ft_falco
 0.0%   0.1%   1.6KiB ft_fox
 0.0%   0.1%   1.6KiB anstyle
 0.0%   0.1%   1.4KiB strsim
 0.0%   0.0%   1.0KiB zmij
 0.0%   0.0%     960B itoa
 0.0%   0.0%     848B toml_writer
 0.0%   0.0%     312B winnow
 0.0%   0.0%     264B hsd_types
 0.0%   0.0%      20B __rustc
 0.0%   0.0%      16B colorchoice
49.6% 100.0%   2.3MiB .text section size, the file size is 4.7MiB

Note: numbers above are a result of guesswork. They are not 100% correct and never will be.
```

### melee-ft: 845 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 21966 | 1 | `melee_ft::fighter::assets::FighterAssets::load` |
| 2411 | 1 | `melee_ft::fighter::damage::DamageParameters::read` |
| 2091 | 1 | `melee_ft::fighter::state::common_table::common_table` |
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

### melee-sim: 128 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 478 | 7 | `melee_ft::fighter::character::CharacterState::new` |
| 282 | 1 | `melee_ft::anim::playback::FighterAnimation::advance_main` |
| 252 | 1 | `melee_ft::anim::root_motion::RootMotion::evaluate` |
| 229 | 1 | `melee_ft::anim::blend::blend_rotation` |
| 205 | 1 | `core::ptr::drop_in_place<melee_ft::fighter::assets::FighterAssets>` |
| 164 | 1 | `melee_ft::anim::playback::FighterAnimation::advance_parts` |
| 130 | 1 | `melee_ft::anim::blend::blend_pose` |
| 110 | 1 | `melee_ft::anim::playback::animate_parts` |
| 109 | 1 | `core::ptr::drop_in_place<melee_ft::fighter::FighterCore>` |
| 72 | 1 | `<melee_ft::fighter::state::FighterProc as core::fmt::Debug>::fmt` |
| 56 | 1 | `<melee_ft::input::pad::PadSample as core::cmp::PartialEq>::eq` |
| 54 | 1 | `<melee_ft::desc::animation::AnimationDescError as core::fmt::Debug>::fmt` |
| 48 | 1 | `core::ptr::drop_in_place<melee_ft::desc::bones::FighterBones>` |
| 41 | 1 | `core::ptr::drop_in_place<[core::option::Option<melee_ft::desc::bones::AnimationBoneSet>; 5]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::desc::animation::AnimationEntry]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::dynamics::DynamicSetDescriptor]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::fighter::assets::FighterAssets; 2]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::fighter::assets::FighterAssets]>` |
| 38 | 1 | `core::ptr::drop_in_place<melee_ft::anim::playback::FighterAnimation>` |
| 37 | 1 | `core::ptr::drop_in_place<melee_ft::anim::attach::MotionRemap>` |

### ft-captain: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 83 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 65 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 7 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 5 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_shield` |

### ft-falco: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 79 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 55 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 9 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |

### ft-fox: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 79 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 55 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 9 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |

### ft-fox-family: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mario: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mars: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 67 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 42 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::guard_variant` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 5 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 3 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_landing` |
| 3 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |

### ft-peach: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 65 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 64 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::check_float_input` |
| 55 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 52 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 49 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_resources_loaded` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 15 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 10 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::dynamics_first_force_bone` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |

### ft-purin: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 61 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 4 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::multi_jump_attributes` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::multi_jump_animation` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_shield` |

### ft-yoshi: 68 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 194 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 142 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 87 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_animated` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 60 | 1 | `melee_ft::fighter::escape::<impl melee_ft::fighter::FighterCore>::roll_input` |
| 54 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 49 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 41 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_entered` |
| 28 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 18 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::action_id` |
| 17 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::input_shield` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 11 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::check_hurtbox_interaction` |

<!-- perf-gate-v1
{"architecture": "concrete-shell-v1", "date": "2026-09-10T08:26:32+00:00", "metrics": {"copies": {"ft-captain": 66, "ft-falco": 67, "ft-fox": 67, "ft-fox-family": 0, "ft-mario": 0, "ft-mars": 66, "ft-peach": 67, "ft-purin": 66, "ft-yoshi": 68, "melee-ft": 845, "melee-sim": 128}, "load_ns": 168173850.23817462, "stripped_bytes": 3483104, "text_bytes": 3178496, "ticks_600_ns": 24040444.5}, "platform": "macOS-26.2-arm64-arm-64bit-Mach-O", "revision": "8d6e2eeb3926dd095ee24f8fb98d152172437569", "rustc": "rustc 1.96.0 (ac68faa20 2026-05-25)", "status": "PASS"}
-->

## 2026-09-10T09:09:44+00:00 — COMPLETE — REGRESSION

Evidence: `/Users/nikhilunni/Projects/melee-lanes/combat/target/perf/20260910T090910Z-68647`. Revision `631a3835f6a7f86aed6b2f28a803eba7bb7169e3` (working tree included).

rustc 1.96.0 (ac68faa20 2026-05-25); macOS-26.2-arm64-arm-64bit-Mach-O.

Tolerance: time +10%, size +5%, copies +0 per compiling crate.
Baseline: 2026-09-10T08:26:32+00:00.
C15 fixed ceilings: 3,747,632 stripped bytes; 1,440 total copies; one definition per common label.

| Measurement | Value |
|---|---:|
| Stripped binary | 3,500,784 bytes |
| Text (`size`) | 3,194,880 bytes |
| load mean | 168.594 ms (95% CI 167.459..169.959 ms) |
| ticks_600 mean | 23.602 ms (95% CI 23.542..23.662 ms) |
| Headless throughput | 25,422 ticks/s |
| Total melee-ft copies | 1,441 |
| Common definition labels audited across crates | 270 |

- ft-fox-family melee-ft copies: 1 > 0 + 0

- C15 ft-fox-family copies: 1 > 0

- C15 total copies: 1441 > 1440

Tool: `0.12.1`.

Tool: `cargo-llvm-lines 0.4.48`.

Per-crate text contribution (cargo-bloat estimates):
```text
File  .text     Size Crate
12.7%  25.6% 613.2KiB melee_sim
 9.7%  19.6% 467.8KiB std
 7.1%  14.2% 340.4KiB melee_ft
 5.5%  11.1% 266.1KiB clap_builder
 1.7%   3.4%  82.5KiB melee_ef
 1.4%   2.8%  67.8KiB slp
 1.3%   2.7%  64.2KiB melee_mp
 1.2%   2.5%  59.3KiB melee_gr
 1.1%   2.1%  50.8KiB hsd_anim
 1.1%   2.1%  50.7KiB hsd_archive
 0.9%   1.7%  41.5KiB toml
 0.8%   1.6%  39.3KiB hsd_particle
 0.8%   1.6%  37.6KiB serde_json
 0.8%   1.5%  36.5KiB toml_parser
 0.4%   0.7%  16.9KiB serde_core
 0.3%   0.6%  15.5KiB melee_diff
 0.3%   0.6%  14.6KiB melee_lb
 0.3%   0.6%  14.6KiB ft_yoshi
 0.3%   0.6%  13.5KiB hsd_gobj
 0.2%   0.5%  11.3KiB melee_it
 0.2%   0.5%  11.3KiB melee_types
 0.2%   0.4%   8.7KiB ft_fox_family
 0.2%   0.4%   8.6KiB ft_fox
 0.2%   0.4%   8.5KiB ft_falco
 0.1%   0.3%   6.9KiB anyhow
 0.1%   0.2%   5.1KiB toml_datetime
 0.1%   0.2%   4.1KiB melee_cmd
 0.1%   0.1%   3.4KiB ft_peach
 0.1%   0.1%   3.4KiB melee_coll
 0.1%   0.1%   3.2KiB it_foxlaser
 0.1%   0.1%   3.2KiB [Unknown]
 0.1%   0.1%   3.1KiB ft_purin
 0.1%   0.1%   2.8KiB clap_lex
 0.1%   0.1%   2.7KiB gekko_math
 0.0%   0.1%   2.3KiB ft_mars
 0.0%   0.1%   2.2KiB ft_captain
 0.0%   0.1%   2.1KiB serde
 0.0%   0.1%   2.1KiB melee_if
 0.0%   0.1%   1.7KiB anstream
 0.0%   0.1%   1.6KiB anstyle
 0.0%   0.1%   1.4KiB strsim
 0.0%   0.0%   1.0KiB zmij
 0.0%   0.0%     960B itoa
 0.0%   0.0%     848B toml_writer
 0.0%   0.0%     376B it_foxillusion
 0.0%   0.0%     312B winnow
 0.0%   0.0%     264B hsd_types
 0.0%   0.0%      20B __rustc
 0.0%   0.0%      16B colorchoice
49.7% 100.0%   2.3MiB .text section size, the file size is 4.7MiB

Note: numbers above are a result of guesswork. They are not 100% correct and never will be.
```

### melee-ft: 845 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 21966 | 1 | `melee_ft::fighter::assets::FighterAssets::load` |
| 2411 | 1 | `melee_ft::fighter::damage::DamageParameters::read` |
| 2091 | 1 | `melee_ft::fighter::state::common_table::common_table` |
| 1682 | 1 | `melee_ft::fighter::shield::ShieldParameters::read` |
| 1374 | 1 | `melee_ft::desc::common::CommonFighterData::read` |
| 1231 | 1 | `melee_ft::dynamics::read_sets` |
| 1123 | 1 | `melee_ft::input::common::InputCommonData::read` |
| 1101 | 1 | `melee_ft::desc::playback::read_playback_motion` |
| 1064 | 1 | `melee_ft::fighter::spawn::<impl melee_ft::fighter::FighterCore>::prepare` |
| 1042 | 1 | `melee_ft::desc::bones::read_fighter_bones` |
| 956 | 1 | `melee_ft::fighter::commands::CommandState::step_inner` |
| 929 | 1 | `melee_ft::desc::fox_attributes::FireFoxAttributes::read` |
| 885 | 1 | `melee_ft::desc::attributes::FighterAttributes::read` |
| 844 | 1 | `melee_ft::dynamics::read_motion_starts` |
| 760 | 1 | `melee_ft::anim::attach::select_motion` |
| 695 | 1 | `melee_ft::fighter::smash::read_overlay` |
| 682 | 1 | `melee_ft::anim::playback::FighterAnimation::resume_dynamic_subtree` |
| 659 | 1 | `melee_ft::desc::fox_attributes::IllusionAttributes::read` |
| 640 | 1 | `melee_ft::desc::bones::read_ground_pose` |
| 603 | 1 | `melee_ft::desc::bones::EcbBones::read` |

### melee-sim: 128 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 478 | 7 | `melee_ft::fighter::character::CharacterState::new` |
| 282 | 1 | `melee_ft::anim::playback::FighterAnimation::advance_main` |
| 252 | 1 | `melee_ft::anim::root_motion::RootMotion::evaluate` |
| 229 | 1 | `melee_ft::anim::blend::blend_rotation` |
| 205 | 1 | `core::ptr::drop_in_place<melee_ft::fighter::assets::FighterAssets>` |
| 164 | 1 | `melee_ft::anim::playback::FighterAnimation::advance_parts` |
| 130 | 1 | `melee_ft::anim::blend::blend_pose` |
| 110 | 1 | `melee_ft::anim::playback::animate_parts` |
| 109 | 1 | `core::ptr::drop_in_place<melee_ft::fighter::FighterCore>` |
| 72 | 1 | `<melee_ft::fighter::state::FighterProc as core::fmt::Debug>::fmt` |
| 56 | 1 | `<melee_ft::input::pad::PadSample as core::cmp::PartialEq>::eq` |
| 54 | 1 | `<melee_ft::desc::animation::AnimationDescError as core::fmt::Debug>::fmt` |
| 48 | 1 | `core::ptr::drop_in_place<melee_ft::desc::bones::FighterBones>` |
| 41 | 1 | `core::ptr::drop_in_place<[core::option::Option<melee_ft::desc::bones::AnimationBoneSet>; 5]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::desc::animation::AnimationEntry]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::dynamics::DynamicSetDescriptor]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::fighter::assets::FighterAssets; 2]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::fighter::assets::FighterAssets]>` |
| 38 | 1 | `core::ptr::drop_in_place<melee_ft::anim::playback::FighterAnimation>` |
| 37 | 1 | `core::ptr::drop_in_place<melee_ft::anim::attach::MotionRemap>` |

### ft-captain: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 83 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 65 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 34 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 7 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 5 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_shield` |

### ft-falco: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 57 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 55 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 35 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 9 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 3 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |

### ft-fox: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 57 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 55 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 35 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 9 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 3 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |

### ft-fox-family: 1 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 62 | 1 | `melee_ft::physics::integrate::integrate_environment` |

### ft-mario: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mars: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 67 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 42 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 34 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::guard_variant` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 5 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 3 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_landing` |
| 3 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |

### ft-peach: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 65 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 64 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::check_float_input` |
| 55 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 52 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 49 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_resources_loaded` |
| 34 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 15 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 10 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::dynamics_first_force_bone` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |

### ft-purin: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 61 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 34 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 4 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::multi_jump_attributes` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::multi_jump_animation` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_shield` |

### ft-yoshi: 68 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 194 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 142 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 87 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_animated` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 60 | 1 | `melee_ft::fighter::escape::<impl melee_ft::fighter::FighterCore>::roll_input` |
| 54 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 49 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 41 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_entered` |
| 34 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 28 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 18 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::action_id` |
| 17 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::input_shield` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 11 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::check_hurtbox_interaction` |

<!-- perf-gate-v1
{"architecture": "concrete-shell-v1", "date": "2026-09-10T09:09:44+00:00", "metrics": {"copies": {"ft-captain": 66, "ft-falco": 67, "ft-fox": 67, "ft-fox-family": 1, "ft-mario": 0, "ft-mars": 66, "ft-peach": 67, "ft-purin": 66, "ft-yoshi": 68, "melee-ft": 845, "melee-sim": 128}, "load_ns": 168593522.25285715, "stripped_bytes": 3500784, "text_bytes": 3194880, "ticks_600_ns": 23601720.93333333}, "platform": "macOS-26.2-arm64-arm-64bit-Mach-O", "revision": "631a3835f6a7f86aed6b2f28a803eba7bb7169e3", "rustc": "rustc 1.96.0 (ac68faa20 2026-05-25)", "status": "REGRESSION"}
-->

## 2026-09-10T09:11:44+00:00 — COMPLETE — PASS

Evidence: `/Users/nikhilunni/Projects/melee-lanes/combat/target/perf/20260910T091101Z-70245`. Revision `631a3835f6a7f86aed6b2f28a803eba7bb7169e3` (working tree included).

rustc 1.96.0 (ac68faa20 2026-05-25); macOS-26.2-arm64-arm-64bit-Mach-O.

Tolerance: time +10%, size +5%, copies +0 per compiling crate.
Baseline: 2026-09-10T08:26:32+00:00.
C15 fixed ceilings: 3,747,632 stripped bytes; 1,440 total copies; one definition per common label.

| Measurement | Value |
|---|---:|
| Stripped binary | 3,500,832 bytes |
| Text (`size`) | 3,194,880 bytes |
| load mean | 167.185 ms (95% CI 166.970..167.412 ms) |
| ticks_600 mean | 23.614 ms (95% CI 23.508..23.727 ms) |
| Headless throughput | 25,409 ticks/s |
| Total melee-ft copies | 1,440 |
| Common definition labels audited across crates | 270 |

Tool: `0.12.1`.

Tool: `cargo-llvm-lines 0.4.48`.

Per-crate text contribution (cargo-bloat estimates):
```text
File  .text     Size Crate
12.7%  25.6% 613.2KiB melee_sim
 9.7%  19.6% 467.9KiB std
 7.1%  14.2% 339.9KiB melee_ft
 5.5%  11.1% 266.1KiB clap_builder
 1.7%   3.5%  82.5KiB melee_ef
 1.4%   2.8%  67.8KiB slp
 1.3%   2.7%  64.2KiB melee_mp
 1.2%   2.5%  59.3KiB melee_gr
 1.1%   2.1%  50.8KiB hsd_anim
 1.1%   2.1%  50.7KiB hsd_archive
 0.9%   1.7%  41.5KiB toml
 0.8%   1.6%  39.3KiB hsd_particle
 0.8%   1.6%  37.6KiB serde_json
 0.8%   1.5%  36.5KiB toml_parser
 0.4%   0.7%  16.9KiB serde_core
 0.3%   0.7%  15.5KiB melee_diff
 0.3%   0.6%  14.6KiB melee_lb
 0.3%   0.6%  14.6KiB ft_yoshi
 0.3%   0.6%  13.5KiB hsd_gobj
 0.2%   0.5%  11.3KiB melee_it
 0.2%   0.5%  11.3KiB melee_types
 0.2%   0.4%   8.6KiB ft_fox_family
 0.2%   0.4%   8.6KiB ft_fox
 0.2%   0.4%   8.5KiB ft_falco
 0.1%   0.3%   6.9KiB anyhow
 0.1%   0.2%   5.1KiB toml_datetime
 0.1%   0.2%   4.1KiB melee_cmd
 0.1%   0.1%   3.4KiB ft_peach
 0.1%   0.1%   3.4KiB melee_coll
 0.1%   0.1%   3.2KiB it_foxlaser
 0.1%   0.1%   3.2KiB [Unknown]
 0.1%   0.1%   3.1KiB ft_purin
 0.1%   0.1%   2.8KiB clap_lex
 0.1%   0.1%   2.7KiB gekko_math
 0.0%   0.1%   2.3KiB ft_mars
 0.0%   0.1%   2.2KiB ft_captain
 0.0%   0.1%   2.1KiB serde
 0.0%   0.1%   2.1KiB melee_if
 0.0%   0.1%   1.7KiB anstream
 0.0%   0.1%   1.6KiB anstyle
 0.0%   0.1%   1.4KiB strsim
 0.0%   0.0%   1.0KiB zmij
 0.0%   0.0%     960B itoa
 0.0%   0.0%     848B toml_writer
 0.0%   0.0%     376B it_foxillusion
 0.0%   0.0%     312B winnow
 0.0%   0.0%     264B hsd_types
 0.0%   0.0%      20B __rustc
 0.0%   0.0%      16B colorchoice
49.7% 100.0%   2.3MiB .text section size, the file size is 4.7MiB

Note: numbers above are a result of guesswork. They are not 100% correct and never will be.
```

### melee-ft: 845 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 21966 | 1 | `melee_ft::fighter::assets::FighterAssets::load` |
| 2411 | 1 | `melee_ft::fighter::damage::DamageParameters::read` |
| 2091 | 1 | `melee_ft::fighter::state::common_table::common_table` |
| 1682 | 1 | `melee_ft::fighter::shield::ShieldParameters::read` |
| 1374 | 1 | `melee_ft::desc::common::CommonFighterData::read` |
| 1231 | 1 | `melee_ft::dynamics::read_sets` |
| 1123 | 1 | `melee_ft::input::common::InputCommonData::read` |
| 1101 | 1 | `melee_ft::desc::playback::read_playback_motion` |
| 1064 | 1 | `melee_ft::fighter::spawn::<impl melee_ft::fighter::FighterCore>::prepare` |
| 1042 | 1 | `melee_ft::desc::bones::read_fighter_bones` |
| 956 | 1 | `melee_ft::fighter::commands::CommandState::step_inner` |
| 929 | 1 | `melee_ft::desc::fox_attributes::FireFoxAttributes::read` |
| 885 | 1 | `melee_ft::desc::attributes::FighterAttributes::read` |
| 844 | 1 | `melee_ft::dynamics::read_motion_starts` |
| 760 | 1 | `melee_ft::anim::attach::select_motion` |
| 695 | 1 | `melee_ft::fighter::smash::read_overlay` |
| 682 | 1 | `melee_ft::anim::playback::FighterAnimation::resume_dynamic_subtree` |
| 659 | 1 | `melee_ft::desc::fox_attributes::IllusionAttributes::read` |
| 640 | 1 | `melee_ft::desc::bones::read_ground_pose` |
| 603 | 1 | `melee_ft::desc::bones::EcbBones::read` |

### melee-sim: 128 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 478 | 7 | `melee_ft::fighter::character::CharacterState::new` |
| 282 | 1 | `melee_ft::anim::playback::FighterAnimation::advance_main` |
| 252 | 1 | `melee_ft::anim::root_motion::RootMotion::evaluate` |
| 229 | 1 | `melee_ft::anim::blend::blend_rotation` |
| 205 | 1 | `core::ptr::drop_in_place<melee_ft::fighter::assets::FighterAssets>` |
| 164 | 1 | `melee_ft::anim::playback::FighterAnimation::advance_parts` |
| 130 | 1 | `melee_ft::anim::blend::blend_pose` |
| 110 | 1 | `melee_ft::anim::playback::animate_parts` |
| 109 | 1 | `core::ptr::drop_in_place<melee_ft::fighter::FighterCore>` |
| 72 | 1 | `<melee_ft::fighter::state::FighterProc as core::fmt::Debug>::fmt` |
| 56 | 1 | `<melee_ft::input::pad::PadSample as core::cmp::PartialEq>::eq` |
| 54 | 1 | `<melee_ft::desc::animation::AnimationDescError as core::fmt::Debug>::fmt` |
| 48 | 1 | `core::ptr::drop_in_place<melee_ft::desc::bones::FighterBones>` |
| 41 | 1 | `core::ptr::drop_in_place<[core::option::Option<melee_ft::desc::bones::AnimationBoneSet>; 5]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::desc::animation::AnimationEntry]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::dynamics::DynamicSetDescriptor]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::fighter::assets::FighterAssets; 2]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::fighter::assets::FighterAssets]>` |
| 38 | 1 | `core::ptr::drop_in_place<melee_ft::anim::playback::FighterAnimation>` |
| 37 | 1 | `core::ptr::drop_in_place<melee_ft::anim::attach::MotionRemap>` |

### ft-captain: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 83 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 65 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 34 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 7 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 5 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_shield` |

### ft-falco: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 57 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 55 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 35 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 9 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 3 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |

### ft-fox: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 57 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 55 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 35 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 9 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 3 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |

### ft-fox-family: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mario: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mars: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 67 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 42 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 34 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::guard_variant` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 5 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 3 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_landing` |
| 3 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |

### ft-peach: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 65 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 64 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::check_float_input` |
| 55 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 52 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 49 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_resources_loaded` |
| 34 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 15 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 10 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::dynamics_first_force_bone` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |

### ft-purin: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 61 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 34 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 4 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::multi_jump_attributes` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::multi_jump_animation` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_shield` |

### ft-yoshi: 68 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 194 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 142 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 87 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_animated` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 60 | 1 | `melee_ft::fighter::escape::<impl melee_ft::fighter::FighterCore>::roll_input` |
| 54 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 49 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 41 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_entered` |
| 34 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 28 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 18 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::action_id` |
| 17 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::input_shield` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 11 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::check_hurtbox_interaction` |

<!-- perf-gate-v1
{"architecture": "concrete-shell-v1", "date": "2026-09-10T09:11:44+00:00", "metrics": {"copies": {"ft-captain": 66, "ft-falco": 67, "ft-fox": 67, "ft-fox-family": 0, "ft-mario": 0, "ft-mars": 66, "ft-peach": 67, "ft-purin": 66, "ft-yoshi": 68, "melee-ft": 845, "melee-sim": 128}, "load_ns": 167184600.85011905, "stripped_bytes": 3500832, "text_bytes": 3194880, "ticks_600_ns": 23613855.43333333}, "platform": "macOS-26.2-arm64-arm-64bit-Mach-O", "revision": "631a3835f6a7f86aed6b2f28a803eba7bb7169e3", "rustc": "rustc 1.96.0 (ac68faa20 2026-05-25)", "status": "PASS"}
-->

## 2026-09-10T09:30:35+00:00 — COMPLETE — REGRESSION

Evidence: `/Users/nikhilunni/Projects/melee-lanes/combat/target/perf/20260910T092946Z-465`. Revision `631a3835f6a7f86aed6b2f28a803eba7bb7169e3` (working tree included).

rustc 1.96.0 (ac68faa20 2026-05-25); macOS-26.2-arm64-arm-64bit-Mach-O.

Tolerance: time +10%, size +5%, copies +0 per compiling crate.
Baseline: 2026-09-10T09:11:44+00:00.
C15 fixed ceilings: 3,747,632 stripped bytes; 1,440 total copies; one definition per common label.

| Measurement | Value |
|---|---:|
| Stripped binary | 3,500,832 bytes |
| Text (`size`) | 3,194,880 bytes |
| load mean | 214.695 ms (95% CI 212.698..217.112 ms) |
| ticks_600 mean | 33.276 ms (95% CI 32.012..34.617 ms) |
| Headless throughput | 18,031 ticks/s |
| Total melee-ft copies | 1,440 |
| Common definition labels audited across crates | 270 |

- load_ns: 214694566.633 > 183903060.935 (previous 167184600.850, +10%)

- ticks_600_ns: 33276160.400 > 25975240.977 (previous 23613855.433, +10%)

- P1 time ceiling load_ns: 214694566.633 > 182600000

- P1 time ceiling ticks_600_ns: 33276160.400 > 25947000

Tool: `0.12.1`.

Tool: `cargo-llvm-lines 0.4.48`.

Per-crate text contribution (cargo-bloat estimates):
```text
File  .text     Size Crate
12.7%  25.6% 613.2KiB melee_sim
 9.7%  19.6% 467.9KiB std
 7.1%  14.2% 339.9KiB melee_ft
 5.5%  11.1% 266.1KiB clap_builder
 1.7%   3.5%  82.5KiB melee_ef
 1.4%   2.8%  67.8KiB slp
 1.3%   2.7%  64.2KiB melee_mp
 1.2%   2.5%  59.3KiB melee_gr
 1.1%   2.1%  50.8KiB hsd_anim
 1.1%   2.1%  50.7KiB hsd_archive
 0.9%   1.7%  41.5KiB toml
 0.8%   1.6%  39.3KiB hsd_particle
 0.8%   1.6%  37.6KiB serde_json
 0.8%   1.5%  36.5KiB toml_parser
 0.4%   0.7%  16.9KiB serde_core
 0.3%   0.7%  15.5KiB melee_diff
 0.3%   0.6%  14.6KiB melee_lb
 0.3%   0.6%  14.6KiB ft_yoshi
 0.3%   0.6%  13.5KiB hsd_gobj
 0.2%   0.5%  11.3KiB melee_it
 0.2%   0.5%  11.3KiB melee_types
 0.2%   0.4%   8.6KiB ft_fox_family
 0.2%   0.4%   8.6KiB ft_fox
 0.2%   0.4%   8.5KiB ft_falco
 0.1%   0.3%   6.9KiB anyhow
 0.1%   0.2%   5.1KiB toml_datetime
 0.1%   0.2%   4.1KiB melee_cmd
 0.1%   0.1%   3.4KiB ft_peach
 0.1%   0.1%   3.4KiB melee_coll
 0.1%   0.1%   3.2KiB it_foxlaser
 0.1%   0.1%   3.2KiB [Unknown]
 0.1%   0.1%   3.1KiB ft_purin
 0.1%   0.1%   2.8KiB clap_lex
 0.1%   0.1%   2.7KiB gekko_math
 0.0%   0.1%   2.3KiB ft_mars
 0.0%   0.1%   2.2KiB ft_captain
 0.0%   0.1%   2.1KiB serde
 0.0%   0.1%   2.1KiB melee_if
 0.0%   0.1%   1.7KiB anstream
 0.0%   0.1%   1.6KiB anstyle
 0.0%   0.1%   1.4KiB strsim
 0.0%   0.0%   1.0KiB zmij
 0.0%   0.0%     960B itoa
 0.0%   0.0%     848B toml_writer
 0.0%   0.0%     376B it_foxillusion
 0.0%   0.0%     312B winnow
 0.0%   0.0%     264B hsd_types
 0.0%   0.0%      20B __rustc
 0.0%   0.0%      16B colorchoice
49.7% 100.0%   2.3MiB .text section size, the file size is 4.7MiB

Note: numbers above are a result of guesswork. They are not 100% correct and never will be.
```

### melee-ft: 845 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 21966 | 1 | `melee_ft::fighter::assets::FighterAssets::load` |
| 2411 | 1 | `melee_ft::fighter::damage::DamageParameters::read` |
| 2091 | 1 | `melee_ft::fighter::state::common_table::common_table` |
| 1682 | 1 | `melee_ft::fighter::shield::ShieldParameters::read` |
| 1374 | 1 | `melee_ft::desc::common::CommonFighterData::read` |
| 1231 | 1 | `melee_ft::dynamics::read_sets` |
| 1123 | 1 | `melee_ft::input::common::InputCommonData::read` |
| 1101 | 1 | `melee_ft::desc::playback::read_playback_motion` |
| 1064 | 1 | `melee_ft::fighter::spawn::<impl melee_ft::fighter::FighterCore>::prepare` |
| 1042 | 1 | `melee_ft::desc::bones::read_fighter_bones` |
| 956 | 1 | `melee_ft::fighter::commands::CommandState::step_inner` |
| 929 | 1 | `melee_ft::desc::fox_attributes::FireFoxAttributes::read` |
| 885 | 1 | `melee_ft::desc::attributes::FighterAttributes::read` |
| 844 | 1 | `melee_ft::dynamics::read_motion_starts` |
| 760 | 1 | `melee_ft::anim::attach::select_motion` |
| 695 | 1 | `melee_ft::fighter::smash::read_overlay` |
| 682 | 1 | `melee_ft::anim::playback::FighterAnimation::resume_dynamic_subtree` |
| 659 | 1 | `melee_ft::desc::fox_attributes::IllusionAttributes::read` |
| 640 | 1 | `melee_ft::desc::bones::read_ground_pose` |
| 603 | 1 | `melee_ft::desc::bones::EcbBones::read` |

### melee-sim: 128 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 478 | 7 | `melee_ft::fighter::character::CharacterState::new` |
| 282 | 1 | `melee_ft::anim::playback::FighterAnimation::advance_main` |
| 252 | 1 | `melee_ft::anim::root_motion::RootMotion::evaluate` |
| 229 | 1 | `melee_ft::anim::blend::blend_rotation` |
| 205 | 1 | `core::ptr::drop_in_place<melee_ft::fighter::assets::FighterAssets>` |
| 164 | 1 | `melee_ft::anim::playback::FighterAnimation::advance_parts` |
| 130 | 1 | `melee_ft::anim::blend::blend_pose` |
| 110 | 1 | `melee_ft::anim::playback::animate_parts` |
| 109 | 1 | `core::ptr::drop_in_place<melee_ft::fighter::FighterCore>` |
| 72 | 1 | `<melee_ft::fighter::state::FighterProc as core::fmt::Debug>::fmt` |
| 56 | 1 | `<melee_ft::input::pad::PadSample as core::cmp::PartialEq>::eq` |
| 54 | 1 | `<melee_ft::desc::animation::AnimationDescError as core::fmt::Debug>::fmt` |
| 48 | 1 | `core::ptr::drop_in_place<melee_ft::desc::bones::FighterBones>` |
| 41 | 1 | `core::ptr::drop_in_place<[core::option::Option<melee_ft::desc::bones::AnimationBoneSet>; 5]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::desc::animation::AnimationEntry]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::dynamics::DynamicSetDescriptor]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::fighter::assets::FighterAssets; 2]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::fighter::assets::FighterAssets]>` |
| 38 | 1 | `core::ptr::drop_in_place<melee_ft::anim::playback::FighterAnimation>` |
| 37 | 1 | `core::ptr::drop_in_place<melee_ft::anim::attach::MotionRemap>` |

### ft-captain: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 83 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 65 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 34 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 7 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 5 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_shield` |

### ft-falco: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 57 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 55 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 35 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 9 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 3 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |

### ft-fox: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 57 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 55 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 35 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 9 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 3 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |

### ft-fox-family: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mario: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mars: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 67 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 42 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 34 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::guard_variant` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 5 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 3 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_landing` |
| 3 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |

### ft-peach: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 65 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 64 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::check_float_input` |
| 55 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 52 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 49 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_resources_loaded` |
| 34 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 15 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 10 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::dynamics_first_force_bone` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |

### ft-purin: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 61 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 34 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 4 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::multi_jump_attributes` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::multi_jump_animation` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_shield` |

### ft-yoshi: 68 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 194 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 142 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 87 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_animated` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 60 | 1 | `melee_ft::fighter::escape::<impl melee_ft::fighter::FighterCore>::roll_input` |
| 54 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 49 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 41 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_entered` |
| 34 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 28 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 18 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::action_id` |
| 17 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::input_shield` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 11 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::check_hurtbox_interaction` |

<!-- perf-gate-v1
{"architecture": "concrete-shell-v1", "date": "2026-09-10T09:30:35+00:00", "metrics": {"copies": {"ft-captain": 66, "ft-falco": 67, "ft-fox": 67, "ft-fox-family": 0, "ft-mario": 0, "ft-mars": 66, "ft-peach": 67, "ft-purin": 66, "ft-yoshi": 68, "melee-ft": 845, "melee-sim": 128}, "load_ns": 214694566.63333336, "stripped_bytes": 3500832, "text_bytes": 3194880, "ticks_600_ns": 33276160.4}, "platform": "macOS-26.2-arm64-arm-64bit-Mach-O", "revision": "631a3835f6a7f86aed6b2f28a803eba7bb7169e3", "rustc": "rustc 1.96.0 (ac68faa20 2026-05-25)", "status": "REGRESSION"}
-->
## 2026-09-10T08:41:58+00:00 — COMPLETE — REGRESSION

Evidence: `/Users/nikhilunni/Projects/melee-lanes/chars/target/perf/20260910T084103Z-29388`. Revision `631a3835f6a7f86aed6b2f28a803eba7bb7169e3` (working tree included).

rustc 1.96.0 (ac68faa20 2026-05-25); macOS-26.2-arm64-arm-64bit-Mach-O.

Tolerance: time +10%, size +5%, copies +0 per compiling crate.
Baseline: 2026-09-10T08:26:32+00:00.
C15 fixed ceilings: 3,747,632 stripped bytes; 1,440 total copies; one definition per common label.

| Measurement | Value |
|---|---:|
| Stripped binary | 3,483,344 bytes |
| Text (`size`) | 3,178,496 bytes |
| load mean | 173.444 ms (95% CI 173.204..173.695 ms) |
| ticks_600 mean | 23.698 ms (95% CI 23.638..23.753 ms) |
| Headless throughput | 25,318 ticks/s |
| Total melee-ft copies | 1,449 |
| Common definition labels audited across crates | 272 |

- melee-ft melee-ft copies: 854 > 845 + 0

- C15 melee-ft copies: 854 > 845

- C15 total copies: 1449 > 1440

Tool: `0.12.1`.

Tool: `cargo-llvm-lines 0.4.48`.

Per-crate text contribution (cargo-bloat estimates):
```text
File  .text     Size Crate
12.8%  25.8% 614.3KiB melee_sim
 9.8%  19.7% 468.8KiB std
 7.1%  14.4% 341.5KiB melee_ft
 5.6%  11.2% 266.1KiB clap_builder
 1.7%   3.5%  82.1KiB melee_ef
 1.4%   2.8%  67.6KiB slp
 1.3%   2.7%  64.2KiB melee_mp
 1.2%   2.5%  59.3KiB melee_gr
 1.1%   2.1%  50.8KiB hsd_anim
 1.1%   2.1%  50.7KiB hsd_archive
 0.9%   1.7%  41.5KiB toml
 0.8%   1.7%  39.3KiB hsd_particle
 0.8%   1.5%  36.5KiB toml_parser
 0.7%   1.5%  35.5KiB serde_json
 0.4%   0.7%  16.9KiB serde_core
 0.3%   0.6%  15.4KiB melee_diff
 0.3%   0.6%  14.6KiB melee_lb
 0.3%   0.6%  14.6KiB ft_yoshi
 0.3%   0.6%  13.5KiB hsd_gobj
 0.2%   0.5%  11.1KiB melee_it
 0.2%   0.5%  10.8KiB melee_types
 0.2%   0.3%   8.0KiB ft_fox_family
 0.1%   0.3%   6.9KiB anyhow
 0.1%   0.2%   5.1KiB toml_datetime
 0.1%   0.2%   4.1KiB melee_cmd
 0.1%   0.1%   3.4KiB ft_peach
 0.1%   0.1%   3.4KiB melee_coll
 0.1%   0.1%   3.2KiB it_foxlaser
 0.1%   0.1%   3.1KiB [Unknown]
 0.1%   0.1%   3.1KiB ft_purin
 0.1%   0.1%   2.8KiB clap_lex
 0.1%   0.1%   2.7KiB gekko_math
 0.0%   0.1%   2.3KiB ft_mars
 0.0%   0.1%   2.2KiB ft_captain
 0.0%   0.1%   2.1KiB serde
 0.0%   0.1%   2.1KiB melee_if
 0.0%   0.1%   1.7KiB anstream
 0.0%   0.1%   1.6KiB anstyle
 0.0%   0.1%   1.6KiB ft_falco
 0.0%   0.1%   1.6KiB ft_fox
 0.0%   0.1%   1.4KiB strsim
 0.0%   0.0%   1.0KiB zmij
 0.0%   0.0%     960B itoa
 0.0%   0.0%     848B toml_writer
 0.0%   0.0%     312B winnow
 0.0%   0.0%     264B hsd_types
 0.0%   0.0%      20B __rustc
 0.0%   0.0%      16B colorchoice
49.7% 100.0%   2.3MiB .text section size, the file size is 4.7MiB

Note: numbers above are a result of guesswork. They are not 100% correct and never will be.
```

### melee-ft: 854 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 21974 | 1 | `melee_ft::fighter::assets::FighterAssets::load` |
| 2411 | 1 | `melee_ft::fighter::damage::DamageParameters::read` |
| 2311 | 1 | `melee_ft::fighter::state::common_table::common_table` |
| 1682 | 1 | `melee_ft::fighter::shield::ShieldParameters::read` |
| 1374 | 1 | `melee_ft::desc::common::CommonFighterData::read` |
| 1343 | 1 | `melee_ft::input::common::InputCommonData::read` |
| 1231 | 1 | `melee_ft::dynamics::read_sets` |
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

### melee-sim: 128 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 478 | 7 | `melee_ft::fighter::character::CharacterState::new` |
| 282 | 1 | `melee_ft::anim::playback::FighterAnimation::advance_main` |
| 252 | 1 | `melee_ft::anim::root_motion::RootMotion::evaluate` |
| 229 | 1 | `melee_ft::anim::blend::blend_rotation` |
| 205 | 1 | `core::ptr::drop_in_place<melee_ft::fighter::assets::FighterAssets>` |
| 164 | 1 | `melee_ft::anim::playback::FighterAnimation::advance_parts` |
| 130 | 1 | `melee_ft::anim::blend::blend_pose` |
| 110 | 1 | `melee_ft::anim::playback::animate_parts` |
| 109 | 1 | `core::ptr::drop_in_place<melee_ft::fighter::FighterCore>` |
| 72 | 1 | `<melee_ft::fighter::state::FighterProc as core::fmt::Debug>::fmt` |
| 56 | 1 | `<melee_ft::input::pad::PadSample as core::cmp::PartialEq>::eq` |
| 54 | 1 | `<melee_ft::desc::animation::AnimationDescError as core::fmt::Debug>::fmt` |
| 48 | 1 | `core::ptr::drop_in_place<melee_ft::desc::bones::FighterBones>` |
| 41 | 1 | `core::ptr::drop_in_place<[core::option::Option<melee_ft::desc::bones::AnimationBoneSet>; 5]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::desc::animation::AnimationEntry]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::dynamics::DynamicSetDescriptor]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::fighter::assets::FighterAssets; 2]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::fighter::assets::FighterAssets]>` |
| 38 | 1 | `core::ptr::drop_in_place<melee_ft::anim::playback::FighterAnimation>` |
| 37 | 1 | `core::ptr::drop_in_place<melee_ft::anim::attach::MotionRemap>` |

### ft-captain: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 83 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 65 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 7 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 5 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_shield` |

### ft-falco: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 79 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 55 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 9 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |

### ft-fox: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 79 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 55 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 9 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |

### ft-fox-family: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mario: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mars: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 67 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 42 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::guard_variant` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 5 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 3 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_landing` |
| 3 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |

### ft-peach: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 65 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 64 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::check_float_input` |
| 55 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 52 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 49 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_resources_loaded` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 15 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 10 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::dynamics_first_force_bone` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |

### ft-purin: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 61 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 4 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::multi_jump_attributes` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::multi_jump_animation` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_shield` |

### ft-yoshi: 68 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 194 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 142 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 87 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_animated` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 60 | 1 | `melee_ft::fighter::escape::<impl melee_ft::fighter::FighterCore>::roll_input` |
| 54 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 49 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 41 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_entered` |
| 28 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 18 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::action_id` |
| 17 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::input_shield` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 11 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::check_hurtbox_interaction` |

<!-- perf-gate-v1
{"architecture": "concrete-shell-v1", "date": "2026-09-10T08:41:58+00:00", "metrics": {"copies": {"ft-captain": 66, "ft-falco": 67, "ft-fox": 67, "ft-fox-family": 0, "ft-mario": 0, "ft-mars": 66, "ft-peach": 67, "ft-purin": 66, "ft-yoshi": 68, "melee-ft": 854, "melee-sim": 128}, "load_ns": 173444365.54996032, "stripped_bytes": 3483344, "text_bytes": 3178496, "ticks_600_ns": 23698349.93333333}, "platform": "macOS-26.2-arm64-arm-64bit-Mach-O", "revision": "631a3835f6a7f86aed6b2f28a803eba7bb7169e3", "rustc": "rustc 1.96.0 (ac68faa20 2026-05-25)", "status": "REGRESSION"}
-->

## 2026-09-10T08:44:02+00:00 — COMPLETE — REGRESSION

Evidence: `/Users/nikhilunni/Projects/melee-lanes/chars/target/perf/20260910T084304Z-31386`. Revision `631a3835f6a7f86aed6b2f28a803eba7bb7169e3` (working tree included).

rustc 1.96.0 (ac68faa20 2026-05-25); macOS-26.2-arm64-arm-64bit-Mach-O.

Tolerance: time +10%, size +5%, copies +0 per compiling crate.
Baseline: 2026-09-10T08:26:32+00:00.
C15 fixed ceilings: 3,747,632 stripped bytes; 1,440 total copies; one definition per common label.

| Measurement | Value |
|---|---:|
| Stripped binary | 3,483,248 bytes |
| Text (`size`) | 3,178,496 bytes |
| load mean | 177.365 ms (95% CI 170.788..184.919 ms) |
| ticks_600 mean | 23.685 ms (95% CI 23.633..23.741 ms) |
| Headless throughput | 25,332 ticks/s |
| Total melee-ft copies | 1,445 |
| Common definition labels audited across crates | 271 |

- melee-ft melee-ft copies: 850 > 845 + 0

- C15 melee-ft copies: 850 > 845

- C15 total copies: 1445 > 1440

Tool: `0.12.1`.

Tool: `cargo-llvm-lines 0.4.48`.

Per-crate text contribution (cargo-bloat estimates):
```text
File  .text     Size Crate
12.8%  25.8% 614.3KiB melee_sim
 9.8%  19.7% 469.0KiB std
 7.2%  14.5% 344.0KiB melee_ft
 5.6%  11.2% 266.1KiB clap_builder
 1.7%   3.4%  82.1KiB melee_ef
 1.4%   2.8%  67.6KiB slp
 1.3%   2.7%  64.2KiB melee_mp
 1.2%   2.5%  59.3KiB melee_gr
 1.1%   2.1%  50.8KiB hsd_anim
 1.1%   2.1%  50.7KiB hsd_archive
 0.9%   1.7%  41.5KiB toml
 0.8%   1.7%  39.3KiB hsd_particle
 0.8%   1.5%  36.5KiB toml_parser
 0.7%   1.5%  35.5KiB serde_json
 0.4%   0.7%  16.9KiB serde_core
 0.3%   0.6%  15.4KiB melee_diff
 0.3%   0.6%  14.6KiB melee_lb
 0.3%   0.6%  14.6KiB ft_yoshi
 0.3%   0.6%  13.5KiB hsd_gobj
 0.2%   0.5%  11.1KiB melee_it
 0.2%   0.5%  10.8KiB melee_types
 0.2%   0.3%   8.0KiB ft_fox_family
 0.1%   0.3%   6.9KiB anyhow
 0.1%   0.2%   5.1KiB toml_datetime
 0.1%   0.2%   4.1KiB melee_cmd
 0.1%   0.1%   3.4KiB ft_peach
 0.1%   0.1%   3.4KiB melee_coll
 0.1%   0.1%   3.2KiB it_foxlaser
 0.1%   0.1%   3.1KiB [Unknown]
 0.1%   0.1%   3.1KiB ft_purin
 0.1%   0.1%   2.8KiB clap_lex
 0.1%   0.1%   2.7KiB gekko_math
 0.0%   0.1%   2.3KiB ft_mars
 0.0%   0.1%   2.2KiB ft_captain
 0.0%   0.1%   2.1KiB serde
 0.0%   0.1%   2.1KiB melee_if
 0.0%   0.1%   1.7KiB anstream
 0.0%   0.1%   1.6KiB ft_fox
 0.0%   0.1%   1.6KiB ft_falco
 0.0%   0.1%   1.6KiB anstyle
 0.0%   0.1%   1.4KiB strsim
 0.0%   0.0%   1.0KiB zmij
 0.0%   0.0%     960B itoa
 0.0%   0.0%     848B toml_writer
 0.0%   0.0%     312B winnow
 0.0%   0.0%     264B hsd_types
 0.0%   0.0%      20B __rustc
 0.0%   0.0%      16B colorchoice
49.7% 100.0%   2.3MiB .text section size, the file size is 4.7MiB

Note: numbers above are a result of guesswork. They are not 100% correct and never will be.
```

### melee-ft: 850 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 21974 | 1 | `melee_ft::fighter::assets::FighterAssets::load` |
| 2411 | 1 | `melee_ft::fighter::damage::DamageParameters::read` |
| 2311 | 1 | `melee_ft::fighter::state::common_table::common_table` |
| 1682 | 1 | `melee_ft::fighter::shield::ShieldParameters::read` |
| 1374 | 1 | `melee_ft::desc::common::CommonFighterData::read` |
| 1343 | 1 | `melee_ft::input::common::InputCommonData::read` |
| 1231 | 1 | `melee_ft::dynamics::read_sets` |
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

### melee-sim: 128 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 478 | 7 | `melee_ft::fighter::character::CharacterState::new` |
| 282 | 1 | `melee_ft::anim::playback::FighterAnimation::advance_main` |
| 252 | 1 | `melee_ft::anim::root_motion::RootMotion::evaluate` |
| 229 | 1 | `melee_ft::anim::blend::blend_rotation` |
| 205 | 1 | `core::ptr::drop_in_place<melee_ft::fighter::assets::FighterAssets>` |
| 164 | 1 | `melee_ft::anim::playback::FighterAnimation::advance_parts` |
| 130 | 1 | `melee_ft::anim::blend::blend_pose` |
| 110 | 1 | `melee_ft::anim::playback::animate_parts` |
| 109 | 1 | `core::ptr::drop_in_place<melee_ft::fighter::FighterCore>` |
| 72 | 1 | `<melee_ft::fighter::state::FighterProc as core::fmt::Debug>::fmt` |
| 56 | 1 | `<melee_ft::input::pad::PadSample as core::cmp::PartialEq>::eq` |
| 54 | 1 | `<melee_ft::desc::animation::AnimationDescError as core::fmt::Debug>::fmt` |
| 48 | 1 | `core::ptr::drop_in_place<melee_ft::desc::bones::FighterBones>` |
| 41 | 1 | `core::ptr::drop_in_place<[core::option::Option<melee_ft::desc::bones::AnimationBoneSet>; 5]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::desc::animation::AnimationEntry]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::dynamics::DynamicSetDescriptor]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::fighter::assets::FighterAssets; 2]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::fighter::assets::FighterAssets]>` |
| 38 | 1 | `core::ptr::drop_in_place<melee_ft::anim::playback::FighterAnimation>` |
| 37 | 1 | `core::ptr::drop_in_place<melee_ft::anim::attach::MotionRemap>` |

### ft-captain: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 83 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 65 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 7 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 5 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_shield` |

### ft-falco: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 79 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 55 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 9 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |

### ft-fox: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 79 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 55 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 9 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |

### ft-fox-family: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mario: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mars: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 67 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 42 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::guard_variant` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 5 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 3 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_landing` |
| 3 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |

### ft-peach: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 65 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 64 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::check_float_input` |
| 55 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 52 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 49 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_resources_loaded` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 15 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 10 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::dynamics_first_force_bone` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |

### ft-purin: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 61 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 4 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::multi_jump_attributes` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::multi_jump_animation` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_shield` |

### ft-yoshi: 68 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 194 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 142 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 87 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_animated` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 60 | 1 | `melee_ft::fighter::escape::<impl melee_ft::fighter::FighterCore>::roll_input` |
| 54 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 49 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 41 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_entered` |
| 28 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 18 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::action_id` |
| 17 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::input_shield` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 11 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::check_hurtbox_interaction` |

<!-- perf-gate-v1
{"architecture": "concrete-shell-v1", "date": "2026-09-10T08:44:02+00:00", "metrics": {"copies": {"ft-captain": 66, "ft-falco": 67, "ft-fox": 67, "ft-fox-family": 0, "ft-mario": 0, "ft-mars": 66, "ft-peach": 67, "ft-purin": 66, "ft-yoshi": 68, "melee-ft": 850, "melee-sim": 128}, "load_ns": 177365436.03333336, "stripped_bytes": 3483248, "text_bytes": 3178496, "ticks_600_ns": 23685220.666666668}, "platform": "macOS-26.2-arm64-arm-64bit-Mach-O", "revision": "631a3835f6a7f86aed6b2f28a803eba7bb7169e3", "rustc": "rustc 1.96.0 (ac68faa20 2026-05-25)", "status": "REGRESSION"}
-->

## 2026-09-10T09:00:54+00:00 — COMPLETE — REGRESSION

Evidence: `/Users/nikhilunni/Projects/melee-lanes/chars/target/perf/20260910T085921Z-51370`. Revision `631a3835f6a7f86aed6b2f28a803eba7bb7169e3` (working tree included).

rustc 1.96.0 (ac68faa20 2026-05-25); macOS-26.2-arm64-arm-64bit-Mach-O.

Tolerance: time +10%, size +5%, copies +0 per compiling crate.
Baseline: 2026-09-10T08:26:32+00:00.
C15 fixed ceilings: 3,747,632 stripped bytes; 1,440 total copies; one definition per common label.

| Measurement | Value |
|---|---:|
| Stripped binary | 3,483,248 bytes |
| Text (`size`) | 3,178,496 bytes |
| load mean | 224.217 ms (95% CI 220.158..227.969 ms) |
| ticks_600 mean | 76.219 ms (95% CI 64.351..86.340 ms) |
| Headless throughput | 7,872 ticks/s |
| Total melee-ft copies | 1,445 |
| Common definition labels audited across crates | 271 |

- load_ns: 224216718.100 > 184991235.262 (previous 168173850.238, +10%)

- ticks_600_ns: 76219070.850 > 26444488.950 (previous 24040444.500, +10%)

- melee-ft melee-ft copies: 850 > 845 + 0

- P1 time ceiling load_ns: 224216718.100 > 182600000

- P1 time ceiling ticks_600_ns: 76219070.850 > 25947000

- C15 melee-ft copies: 850 > 845

- C15 total copies: 1445 > 1440

Tool: `0.12.1`.

Tool: `cargo-llvm-lines 0.4.48`.

Per-crate text contribution (cargo-bloat estimates):
```text
File  .text     Size Crate
12.8%  25.8% 614.3KiB melee_sim
 9.8%  19.7% 468.8KiB std
 7.2%  14.4% 342.9KiB melee_ft
 5.6%  11.2% 266.1KiB clap_builder
 1.7%   3.5%  82.1KiB melee_ef
 1.4%   2.8%  67.6KiB slp
 1.3%   2.7%  64.2KiB melee_mp
 1.2%   2.5%  59.3KiB melee_gr
 1.1%   2.1%  50.8KiB hsd_anim
 1.1%   2.1%  50.7KiB hsd_archive
 0.9%   1.7%  41.5KiB toml
 0.8%   1.7%  39.3KiB hsd_particle
 0.8%   1.5%  36.5KiB toml_parser
 0.7%   1.5%  35.5KiB serde_json
 0.4%   0.7%  16.9KiB serde_core
 0.3%   0.6%  15.4KiB melee_diff
 0.3%   0.6%  14.6KiB melee_lb
 0.3%   0.6%  14.6KiB ft_yoshi
 0.3%   0.6%  13.5KiB hsd_gobj
 0.2%   0.5%  11.1KiB melee_it
 0.2%   0.5%  10.8KiB melee_types
 0.2%   0.3%   8.0KiB ft_fox_family
 0.1%   0.3%   6.9KiB anyhow
 0.1%   0.2%   5.1KiB toml_datetime
 0.1%   0.2%   4.1KiB melee_cmd
 0.1%   0.1%   3.4KiB ft_peach
 0.1%   0.1%   3.4KiB melee_coll
 0.1%   0.1%   3.2KiB it_foxlaser
 0.1%   0.1%   3.1KiB [Unknown]
 0.1%   0.1%   3.1KiB ft_purin
 0.1%   0.1%   2.8KiB clap_lex
 0.1%   0.1%   2.7KiB gekko_math
 0.0%   0.1%   2.3KiB ft_mars
 0.0%   0.1%   2.2KiB ft_captain
 0.0%   0.1%   2.1KiB serde
 0.0%   0.1%   2.1KiB melee_if
 0.0%   0.1%   1.7KiB anstream
 0.0%   0.1%   1.6KiB ft_falco
 0.0%   0.1%   1.6KiB anstyle
 0.0%   0.1%   1.6KiB ft_fox
 0.0%   0.1%   1.4KiB strsim
 0.0%   0.0%   1.0KiB zmij
 0.0%   0.0%     960B itoa
 0.0%   0.0%     848B toml_writer
 0.0%   0.0%     312B winnow
 0.0%   0.0%     264B hsd_types
 0.0%   0.0%      20B __rustc
 0.0%   0.0%      16B colorchoice
49.7% 100.0%   2.3MiB .text section size, the file size is 4.7MiB

Note: numbers above are a result of guesswork. They are not 100% correct and never will be.
```

### melee-ft: 850 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 21974 | 1 | `melee_ft::fighter::assets::FighterAssets::load` |
| 2411 | 1 | `melee_ft::fighter::damage::DamageParameters::read` |
| 2311 | 1 | `melee_ft::fighter::state::common_table::common_table` |
| 1682 | 1 | `melee_ft::fighter::shield::ShieldParameters::read` |
| 1374 | 1 | `melee_ft::desc::common::CommonFighterData::read` |
| 1343 | 1 | `melee_ft::input::common::InputCommonData::read` |
| 1231 | 1 | `melee_ft::dynamics::read_sets` |
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

### melee-sim: 128 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 478 | 7 | `melee_ft::fighter::character::CharacterState::new` |
| 282 | 1 | `melee_ft::anim::playback::FighterAnimation::advance_main` |
| 252 | 1 | `melee_ft::anim::root_motion::RootMotion::evaluate` |
| 229 | 1 | `melee_ft::anim::blend::blend_rotation` |
| 205 | 1 | `core::ptr::drop_in_place<melee_ft::fighter::assets::FighterAssets>` |
| 164 | 1 | `melee_ft::anim::playback::FighterAnimation::advance_parts` |
| 130 | 1 | `melee_ft::anim::blend::blend_pose` |
| 110 | 1 | `melee_ft::anim::playback::animate_parts` |
| 109 | 1 | `core::ptr::drop_in_place<melee_ft::fighter::FighterCore>` |
| 72 | 1 | `<melee_ft::fighter::state::FighterProc as core::fmt::Debug>::fmt` |
| 56 | 1 | `<melee_ft::input::pad::PadSample as core::cmp::PartialEq>::eq` |
| 54 | 1 | `<melee_ft::desc::animation::AnimationDescError as core::fmt::Debug>::fmt` |
| 48 | 1 | `core::ptr::drop_in_place<melee_ft::desc::bones::FighterBones>` |
| 41 | 1 | `core::ptr::drop_in_place<[core::option::Option<melee_ft::desc::bones::AnimationBoneSet>; 5]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::desc::animation::AnimationEntry]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::dynamics::DynamicSetDescriptor]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::fighter::assets::FighterAssets; 2]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::fighter::assets::FighterAssets]>` |
| 38 | 1 | `core::ptr::drop_in_place<melee_ft::anim::playback::FighterAnimation>` |
| 37 | 1 | `core::ptr::drop_in_place<melee_ft::anim::attach::MotionRemap>` |

### ft-captain: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 83 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 65 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 7 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 5 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_shield` |

### ft-falco: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 79 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 55 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 9 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |

### ft-fox: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 79 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 55 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 9 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |

### ft-fox-family: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mario: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mars: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 67 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 42 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::guard_variant` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 5 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 3 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_landing` |
| 3 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |

### ft-peach: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 65 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 64 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::check_float_input` |
| 55 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 52 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 49 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_resources_loaded` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 15 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 10 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::dynamics_first_force_bone` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |

### ft-purin: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 61 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 4 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::multi_jump_attributes` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::multi_jump_animation` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_shield` |

### ft-yoshi: 68 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 194 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 142 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 87 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_animated` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 60 | 1 | `melee_ft::fighter::escape::<impl melee_ft::fighter::FighterCore>::roll_input` |
| 54 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 49 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 41 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_entered` |
| 28 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 18 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::action_id` |
| 17 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::input_shield` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 11 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::check_hurtbox_interaction` |

<!-- perf-gate-v1
{"architecture": "concrete-shell-v1", "date": "2026-09-10T09:00:54+00:00", "metrics": {"copies": {"ft-captain": 66, "ft-falco": 67, "ft-fox": 67, "ft-fox-family": 0, "ft-mario": 0, "ft-mars": 66, "ft-peach": 67, "ft-purin": 66, "ft-yoshi": 68, "melee-ft": 850, "melee-sim": 128}, "load_ns": 224216718.10000005, "stripped_bytes": 3483248, "text_bytes": 3178496, "ticks_600_ns": 76219070.85}, "platform": "macOS-26.2-arm64-arm-64bit-Mach-O", "revision": "631a3835f6a7f86aed6b2f28a803eba7bb7169e3", "rustc": "rustc 1.96.0 (ac68faa20 2026-05-25)", "status": "REGRESSION"}
-->

## 2026-09-10T09:03:07+00:00 — COMPLETE — REGRESSION

Evidence: `/Users/nikhilunni/Projects/melee-lanes/chars/target/perf/20260910T090231Z-59059`. Revision `631a3835f6a7f86aed6b2f28a803eba7bb7169e3` (working tree included).

rustc 1.96.0 (ac68faa20 2026-05-25); macOS-26.2-arm64-arm-64bit-Mach-O.

Tolerance: time +10%, size +5%, copies +0 per compiling crate.
Baseline: 2026-09-10T08:26:32+00:00.
C15 fixed ceilings: 3,747,632 stripped bytes; 1,440 total copies; one definition per common label.

| Measurement | Value |
|---|---:|
| Stripped binary | 3,483,248 bytes |
| Text (`size`) | 3,178,496 bytes |
| load mean | 175.138 ms (95% CI 167.771..185.334 ms) |
| ticks_600 mean | 24.349 ms (95% CI 24.206..24.529 ms) |
| Headless throughput | 24,642 ticks/s |
| Total melee-ft copies | 1,445 |
| Common definition labels audited across crates | 271 |

- melee-ft melee-ft copies: 850 > 845 + 0

- C15 melee-ft copies: 850 > 845

- C15 total copies: 1445 > 1440

Tool: `0.12.1`.

Tool: `cargo-llvm-lines 0.4.48`.

Per-crate text contribution (cargo-bloat estimates):
```text
File  .text     Size Crate
12.8%  25.8% 614.3KiB melee_sim
 9.8%  19.7% 468.8KiB std
 7.2%  14.4% 342.9KiB melee_ft
 5.6%  11.2% 266.1KiB clap_builder
 1.7%   3.5%  82.1KiB melee_ef
 1.4%   2.8%  67.6KiB slp
 1.3%   2.7%  64.2KiB melee_mp
 1.2%   2.5%  59.3KiB melee_gr
 1.1%   2.1%  50.8KiB hsd_anim
 1.1%   2.1%  50.7KiB hsd_archive
 0.9%   1.7%  41.5KiB toml
 0.8%   1.7%  39.3KiB hsd_particle
 0.8%   1.5%  36.5KiB toml_parser
 0.7%   1.5%  35.5KiB serde_json
 0.4%   0.7%  16.9KiB serde_core
 0.3%   0.6%  15.4KiB melee_diff
 0.3%   0.6%  14.6KiB melee_lb
 0.3%   0.6%  14.6KiB ft_yoshi
 0.3%   0.6%  13.5KiB hsd_gobj
 0.2%   0.5%  11.1KiB melee_it
 0.2%   0.5%  10.8KiB melee_types
 0.2%   0.3%   8.0KiB ft_fox_family
 0.1%   0.3%   6.9KiB anyhow
 0.1%   0.2%   5.1KiB toml_datetime
 0.1%   0.2%   4.1KiB melee_cmd
 0.1%   0.1%   3.4KiB ft_peach
 0.1%   0.1%   3.4KiB melee_coll
 0.1%   0.1%   3.2KiB it_foxlaser
 0.1%   0.1%   3.1KiB [Unknown]
 0.1%   0.1%   3.1KiB ft_purin
 0.1%   0.1%   2.8KiB clap_lex
 0.1%   0.1%   2.7KiB gekko_math
 0.0%   0.1%   2.3KiB ft_mars
 0.0%   0.1%   2.2KiB ft_captain
 0.0%   0.1%   2.1KiB serde
 0.0%   0.1%   2.1KiB melee_if
 0.0%   0.1%   1.7KiB anstream
 0.0%   0.1%   1.6KiB anstyle
 0.0%   0.1%   1.6KiB ft_falco
 0.0%   0.1%   1.6KiB ft_fox
 0.0%   0.1%   1.4KiB strsim
 0.0%   0.0%   1.0KiB zmij
 0.0%   0.0%     960B itoa
 0.0%   0.0%     848B toml_writer
 0.0%   0.0%     312B winnow
 0.0%   0.0%     264B hsd_types
 0.0%   0.0%      20B __rustc
 0.0%   0.0%      16B colorchoice
49.7% 100.0%   2.3MiB .text section size, the file size is 4.7MiB

Note: numbers above are a result of guesswork. They are not 100% correct and never will be.
```

### melee-ft: 850 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 21974 | 1 | `melee_ft::fighter::assets::FighterAssets::load` |
| 2411 | 1 | `melee_ft::fighter::damage::DamageParameters::read` |
| 2311 | 1 | `melee_ft::fighter::state::common_table::common_table` |
| 1682 | 1 | `melee_ft::fighter::shield::ShieldParameters::read` |
| 1374 | 1 | `melee_ft::desc::common::CommonFighterData::read` |
| 1343 | 1 | `melee_ft::input::common::InputCommonData::read` |
| 1231 | 1 | `melee_ft::dynamics::read_sets` |
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

### melee-sim: 128 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 478 | 7 | `melee_ft::fighter::character::CharacterState::new` |
| 282 | 1 | `melee_ft::anim::playback::FighterAnimation::advance_main` |
| 252 | 1 | `melee_ft::anim::root_motion::RootMotion::evaluate` |
| 229 | 1 | `melee_ft::anim::blend::blend_rotation` |
| 205 | 1 | `core::ptr::drop_in_place<melee_ft::fighter::assets::FighterAssets>` |
| 164 | 1 | `melee_ft::anim::playback::FighterAnimation::advance_parts` |
| 130 | 1 | `melee_ft::anim::blend::blend_pose` |
| 110 | 1 | `melee_ft::anim::playback::animate_parts` |
| 109 | 1 | `core::ptr::drop_in_place<melee_ft::fighter::FighterCore>` |
| 72 | 1 | `<melee_ft::fighter::state::FighterProc as core::fmt::Debug>::fmt` |
| 56 | 1 | `<melee_ft::input::pad::PadSample as core::cmp::PartialEq>::eq` |
| 54 | 1 | `<melee_ft::desc::animation::AnimationDescError as core::fmt::Debug>::fmt` |
| 48 | 1 | `core::ptr::drop_in_place<melee_ft::desc::bones::FighterBones>` |
| 41 | 1 | `core::ptr::drop_in_place<[core::option::Option<melee_ft::desc::bones::AnimationBoneSet>; 5]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::desc::animation::AnimationEntry]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::dynamics::DynamicSetDescriptor]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::fighter::assets::FighterAssets; 2]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::fighter::assets::FighterAssets]>` |
| 38 | 1 | `core::ptr::drop_in_place<melee_ft::anim::playback::FighterAnimation>` |
| 37 | 1 | `core::ptr::drop_in_place<melee_ft::anim::attach::MotionRemap>` |

### ft-captain: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 83 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 65 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 7 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 5 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_shield` |

### ft-falco: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 79 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 55 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 9 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |

### ft-fox: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 79 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 55 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 9 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |

### ft-fox-family: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mario: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mars: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 67 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 42 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::guard_variant` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 5 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 3 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_landing` |
| 3 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |

### ft-peach: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 65 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 64 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::check_float_input` |
| 55 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 52 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 49 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_resources_loaded` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 15 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 10 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::dynamics_first_force_bone` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |

### ft-purin: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 61 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 4 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::multi_jump_attributes` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::multi_jump_animation` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_shield` |

### ft-yoshi: 68 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 194 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 142 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 87 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_animated` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 60 | 1 | `melee_ft::fighter::escape::<impl melee_ft::fighter::FighterCore>::roll_input` |
| 54 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 49 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 41 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_entered` |
| 28 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 18 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::action_id` |
| 17 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::input_shield` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 11 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::check_hurtbox_interaction` |

<!-- perf-gate-v1
{"architecture": "concrete-shell-v1", "date": "2026-09-10T09:03:07+00:00", "metrics": {"copies": {"ft-captain": 66, "ft-falco": 67, "ft-fox": 67, "ft-fox-family": 0, "ft-mario": 0, "ft-mars": 66, "ft-peach": 67, "ft-purin": 66, "ft-yoshi": 68, "melee-ft": 850, "melee-sim": 128}, "load_ns": 175137721.32416666, "stripped_bytes": 3483248, "text_bytes": 3178496, "ticks_600_ns": 24349119.5}, "platform": "macOS-26.2-arm64-arm-64bit-Mach-O", "revision": "631a3835f6a7f86aed6b2f28a803eba7bb7169e3", "rustc": "rustc 1.96.0 (ac68faa20 2026-05-25)", "status": "REGRESSION"}
-->

## 2026-09-10T09:39:52+00:00 — COMPLETE — REGRESSION

Evidence: `/Users/nikhilunni/Projects/melee-lanes/chars/target/perf/20260910T093806Z-15668`. Revision `631a3835f6a7f86aed6b2f28a803eba7bb7169e3` (working tree included).

rustc 1.96.0 (ac68faa20 2026-05-25); macOS-26.2-arm64-arm-64bit-Mach-O.

Tolerance: time +10%, size +5%, duplicate labels +0 within each compiling crate and across crates.
Baseline: 2026-09-10T08:26:32+00:00.
C15 fixed ceilings: 3,747,632 stripped bytes; C15 duplicate-label baseline; one definition per common label.

| Measurement | Value |
|---|---:|
| Stripped binary | 3,483,952 bytes |
| Text (`size`) | 3,178,496 bytes |
| load mean | 496.965 ms (95% CI 478.436..521.242 ms) |
| ticks_600 mean | 105.112 ms (95% CI 76.156..142.310 ms) |
| Headless throughput | 5,708 ticks/s |
| Total emitted definitions (informational) | 1,447 |
| Labels defined in multiple crates | 99 |
| Common definition labels audited across crates | 271 |

- load_ns: 496965185.400 > 184991235.262 (previous 168173850.238, +10%)

- ticks_600_ns: 105112208.300 > 26444488.950 (previous 24040444.500, +10%)

- P1 time ceiling load_ns: 496965185.400 > 182600000

- P1 time ceiling ticks_600_ns: 105112208.300 > 25947000

Tool: `0.12.1`.

Tool: `cargo-llvm-lines 0.4.48`.

Per-crate text contribution (cargo-bloat estimates):
```text
File  .text     Size Crate
12.8%  25.8% 614.3KiB melee_sim
 9.8%  19.7% 468.8KiB std
 7.2%  14.4% 343.3KiB melee_ft
 5.6%  11.2% 266.1KiB clap_builder
 1.7%   3.4%  82.1KiB melee_ef
 1.4%   2.8%  67.6KiB slp
 1.3%   2.7%  64.2KiB melee_mp
 1.2%   2.5%  59.3KiB melee_gr
 1.1%   2.2%  51.5KiB hsd_anim
 1.1%   2.1%  50.7KiB hsd_archive
 0.9%   1.7%  41.5KiB toml
 0.8%   1.7%  39.3KiB hsd_particle
 0.8%   1.5%  36.5KiB toml_parser
 0.7%   1.5%  35.5KiB serde_json
 0.4%   0.7%  17.3KiB melee_lb
 0.4%   0.7%  16.9KiB serde_core
 0.3%   0.6%  15.4KiB melee_diff
 0.3%   0.6%  14.6KiB ft_yoshi
 0.3%   0.6%  13.5KiB hsd_gobj
 0.2%   0.5%  11.1KiB melee_it
 0.2%   0.5%  10.8KiB melee_types
 0.2%   0.3%   8.0KiB ft_fox_family
 0.1%   0.3%   6.9KiB anyhow
 0.1%   0.2%   5.1KiB toml_datetime
 0.1%   0.2%   4.1KiB melee_cmd
 0.1%   0.1%   3.4KiB ft_peach
 0.1%   0.1%   3.4KiB melee_coll
 0.1%   0.1%   3.2KiB it_foxlaser
 0.1%   0.1%   3.1KiB [Unknown]
 0.1%   0.1%   3.1KiB ft_purin
 0.1%   0.1%   2.8KiB clap_lex
 0.1%   0.1%   2.7KiB gekko_math
 0.0%   0.1%   2.3KiB ft_mars
 0.0%   0.1%   2.2KiB ft_captain
 0.0%   0.1%   2.1KiB serde
 0.0%   0.1%   2.1KiB melee_if
 0.0%   0.1%   1.7KiB anstream
 0.0%   0.1%   1.6KiB ft_falco
 0.0%   0.1%   1.6KiB ft_fox
 0.0%   0.1%   1.6KiB anstyle
 0.0%   0.1%   1.4KiB strsim
 0.0%   0.0%   1.0KiB zmij
 0.0%   0.0%     960B itoa
 0.0%   0.0%     848B toml_writer
 0.0%   0.0%     312B winnow
 0.0%   0.0%     264B hsd_types
 0.0%   0.0%      20B __rustc
 0.0%   0.0%      16B colorchoice
49.8% 100.0%   2.3MiB .text section size, the file size is 4.7MiB

Note: numbers above are a result of guesswork. They are not 100% correct and never will be.
```

### melee-ft: 815 labels, 852 emitted definitions (informational); 20 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 21974 | 1 | `melee_ft::fighter::assets::FighterAssets::load` |
| 2411 | 1 | `melee_ft::fighter::damage::DamageParameters::read` |
| 2311 | 1 | `melee_ft::fighter::state::common_table::common_table` |
| 1682 | 1 | `melee_ft::fighter::shield::ShieldParameters::read` |
| 1374 | 1 | `melee_ft::desc::common::CommonFighterData::read` |
| 1343 | 1 | `melee_ft::input::common::InputCommonData::read` |
| 1231 | 1 | `melee_ft::dynamics::read_sets` |
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

### melee-sim: 101 labels, 128 emitted definitions (informational); 7 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 478 | 7 | `melee_ft::fighter::character::CharacterState::new` |
| 282 | 1 | `melee_ft::anim::playback::FighterAnimation::advance_main` |
| 252 | 1 | `melee_ft::anim::root_motion::RootMotion::evaluate` |
| 229 | 1 | `melee_ft::anim::blend::blend_rotation` |
| 205 | 1 | `core::ptr::drop_in_place<melee_ft::fighter::assets::FighterAssets>` |
| 164 | 1 | `melee_ft::anim::playback::FighterAnimation::advance_parts` |
| 130 | 1 | `melee_ft::anim::blend::blend_pose` |
| 110 | 1 | `melee_ft::anim::playback::animate_parts` |
| 109 | 1 | `core::ptr::drop_in_place<melee_ft::fighter::FighterCore>` |
| 72 | 1 | `<melee_ft::fighter::state::FighterProc as core::fmt::Debug>::fmt` |
| 56 | 1 | `<melee_ft::input::pad::PadSample as core::cmp::PartialEq>::eq` |
| 54 | 1 | `<melee_ft::desc::animation::AnimationDescError as core::fmt::Debug>::fmt` |
| 48 | 1 | `core::ptr::drop_in_place<melee_ft::desc::bones::FighterBones>` |
| 41 | 1 | `core::ptr::drop_in_place<[core::option::Option<melee_ft::desc::bones::AnimationBoneSet>; 5]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::desc::animation::AnimationEntry]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::dynamics::DynamicSetDescriptor]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::fighter::assets::FighterAssets; 2]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::fighter::assets::FighterAssets]>` |
| 38 | 1 | `core::ptr::drop_in_place<melee_ft::anim::playback::FighterAnimation>` |
| 37 | 1 | `core::ptr::drop_in_place<melee_ft::anim::attach::MotionRemap>` |

### ft-captain: 43 labels, 66 emitted definitions (informational); 1 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 83 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 65 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 7 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 5 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_shield` |

### ft-falco: 44 labels, 67 emitted definitions (informational); 1 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 79 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 55 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 9 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |

### ft-fox: 44 labels, 67 emitted definitions (informational); 1 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 79 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 55 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 9 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |

### ft-fox-family: 0 labels, 0 emitted definitions (informational); 0 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mario: 0 labels, 0 emitted definitions (informational); 0 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mars: 43 labels, 66 emitted definitions (informational); 1 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 67 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 42 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::guard_variant` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 5 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 3 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_landing` |
| 3 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |

### ft-peach: 44 labels, 67 emitted definitions (informational); 1 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 65 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 64 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::check_float_input` |
| 55 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 52 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 49 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_resources_loaded` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 15 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 10 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::dynamics_first_force_bone` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |

### ft-purin: 43 labels, 66 emitted definitions (informational); 1 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 61 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 4 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::multi_jump_attributes` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::multi_jump_animation` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_shield` |

### ft-yoshi: 45 labels, 68 emitted definitions (informational); 1 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 194 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 142 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 87 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_animated` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 60 | 1 | `melee_ft::fighter::escape::<impl melee_ft::fighter::FighterCore>::roll_input` |
| 54 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 49 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 41 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_entered` |
| 28 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 18 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::action_id` |
| 17 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::input_shield` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 11 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::check_hurtbox_interaction` |

<!-- perf-gate-v1
{"architecture": "concrete-shell-v1", "census": "duplicate-labels-v1", "date": "2026-09-10T09:39:52+00:00", "metrics": {"cross_crate_duplicate_labels": 99, "definitions": {"ft-captain": 66, "ft-falco": 67, "ft-fox": 67, "ft-fox-family": 0, "ft-mario": 0, "ft-mars": 66, "ft-peach": 67, "ft-purin": 66, "ft-yoshi": 68, "melee-ft": 852, "melee-sim": 128}, "duplicate_labels": {"ft-captain": 1, "ft-falco": 1, "ft-fox": 1, "ft-fox-family": 0, "ft-mario": 0, "ft-mars": 1, "ft-peach": 1, "ft-purin": 1, "ft-yoshi": 1, "melee-ft": 20, "melee-sim": 7}, "labels": {"ft-captain": 43, "ft-falco": 44, "ft-fox": 44, "ft-fox-family": 0, "ft-mario": 0, "ft-mars": 43, "ft-peach": 44, "ft-purin": 43, "ft-yoshi": 45, "melee-ft": 815, "melee-sim": 101}, "load_ns": 496965185.4, "stripped_bytes": 3483952, "text_bytes": 3178496, "ticks_600_ns": 105112208.3}, "platform": "macOS-26.2-arm64-arm-64bit-Mach-O", "revision": "631a3835f6a7f86aed6b2f28a803eba7bb7169e3", "rustc": "rustc 1.96.0 (ac68faa20 2026-05-25)", "status": "REGRESSION"}
-->

## 2026-09-10T09:42:26+00:00 — COMPLETE — REGRESSION

Evidence: `/Users/nikhilunni/Projects/melee-lanes/chars/target/perf/20260910T094147Z-17425`. Revision `631a3835f6a7f86aed6b2f28a803eba7bb7169e3` (working tree included).

rustc 1.96.0 (ac68faa20 2026-05-25); macOS-26.2-arm64-arm-64bit-Mach-O.

Tolerance: time +10%, size +5%, duplicate labels +0 within each compiling crate and across crates.
Baseline: 2026-09-10T08:26:32+00:00.
C15 fixed ceilings: 3,747,632 stripped bytes; C15 duplicate-label baseline; one definition per common label.

| Measurement | Value |
|---|---:|
| Stripped binary | 3,483,952 bytes |
| Text (`size`) | 3,178,496 bytes |
| load mean | 441.739 ms (95% CI 384.225..493.132 ms) |
| ticks_600 mean | 38.298 ms (95% CI 35.550..41.453 ms) |
| Headless throughput | 15,667 ticks/s |
| Total emitted definitions (informational) | 1,447 |
| Labels defined in multiple crates | 99 |
| Common definition labels audited across crates | 271 |

- load_ns: 441738714.500 > 184991235.262 (previous 168173850.238, +10%)

- ticks_600_ns: 38297660.400 > 26444488.950 (previous 24040444.500, +10%)

- P1 time ceiling load_ns: 441738714.500 > 182600000

- P1 time ceiling ticks_600_ns: 38297660.400 > 25947000

Tool: `0.12.1`.

Tool: `cargo-llvm-lines 0.4.48`.

Per-crate text contribution (cargo-bloat estimates):
```text
File  .text     Size Crate
12.8%  25.8% 614.3KiB melee_sim
 9.8%  19.7% 468.8KiB std
 7.2%  14.4% 343.3KiB melee_ft
 5.6%  11.2% 266.1KiB clap_builder
 1.7%   3.4%  82.1KiB melee_ef
 1.4%   2.8%  67.6KiB slp
 1.3%   2.7%  64.2KiB melee_mp
 1.2%   2.5%  59.3KiB melee_gr
 1.1%   2.2%  51.5KiB hsd_anim
 1.1%   2.1%  50.7KiB hsd_archive
 0.9%   1.7%  41.5KiB toml
 0.8%   1.7%  39.3KiB hsd_particle
 0.8%   1.5%  36.5KiB toml_parser
 0.7%   1.5%  35.5KiB serde_json
 0.4%   0.7%  17.3KiB melee_lb
 0.4%   0.7%  16.9KiB serde_core
 0.3%   0.6%  15.4KiB melee_diff
 0.3%   0.6%  14.6KiB ft_yoshi
 0.3%   0.6%  13.5KiB hsd_gobj
 0.2%   0.5%  11.1KiB melee_it
 0.2%   0.5%  10.8KiB melee_types
 0.2%   0.3%   8.0KiB ft_fox_family
 0.1%   0.3%   6.9KiB anyhow
 0.1%   0.2%   5.1KiB toml_datetime
 0.1%   0.2%   4.1KiB melee_cmd
 0.1%   0.1%   3.4KiB ft_peach
 0.1%   0.1%   3.4KiB melee_coll
 0.1%   0.1%   3.2KiB it_foxlaser
 0.1%   0.1%   3.1KiB [Unknown]
 0.1%   0.1%   3.1KiB ft_purin
 0.1%   0.1%   2.8KiB clap_lex
 0.1%   0.1%   2.7KiB gekko_math
 0.0%   0.1%   2.3KiB ft_mars
 0.0%   0.1%   2.2KiB ft_captain
 0.0%   0.1%   2.1KiB serde
 0.0%   0.1%   2.1KiB melee_if
 0.0%   0.1%   1.7KiB anstream
 0.0%   0.1%   1.6KiB ft_falco
 0.0%   0.1%   1.6KiB anstyle
 0.0%   0.1%   1.6KiB ft_fox
 0.0%   0.1%   1.4KiB strsim
 0.0%   0.0%   1.0KiB zmij
 0.0%   0.0%     960B itoa
 0.0%   0.0%     848B toml_writer
 0.0%   0.0%     312B winnow
 0.0%   0.0%     264B hsd_types
 0.0%   0.0%      20B __rustc
 0.0%   0.0%      16B colorchoice
49.8% 100.0%   2.3MiB .text section size, the file size is 4.7MiB

Note: numbers above are a result of guesswork. They are not 100% correct and never will be.
```

### melee-ft: 815 labels, 852 emitted definitions (informational); 20 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 21974 | 1 | `melee_ft::fighter::assets::FighterAssets::load` |
| 2411 | 1 | `melee_ft::fighter::damage::DamageParameters::read` |
| 2311 | 1 | `melee_ft::fighter::state::common_table::common_table` |
| 1682 | 1 | `melee_ft::fighter::shield::ShieldParameters::read` |
| 1374 | 1 | `melee_ft::desc::common::CommonFighterData::read` |
| 1343 | 1 | `melee_ft::input::common::InputCommonData::read` |
| 1231 | 1 | `melee_ft::dynamics::read_sets` |
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

### melee-sim: 101 labels, 128 emitted definitions (informational); 7 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 478 | 7 | `melee_ft::fighter::character::CharacterState::new` |
| 282 | 1 | `melee_ft::anim::playback::FighterAnimation::advance_main` |
| 252 | 1 | `melee_ft::anim::root_motion::RootMotion::evaluate` |
| 229 | 1 | `melee_ft::anim::blend::blend_rotation` |
| 205 | 1 | `core::ptr::drop_in_place<melee_ft::fighter::assets::FighterAssets>` |
| 164 | 1 | `melee_ft::anim::playback::FighterAnimation::advance_parts` |
| 130 | 1 | `melee_ft::anim::blend::blend_pose` |
| 110 | 1 | `melee_ft::anim::playback::animate_parts` |
| 109 | 1 | `core::ptr::drop_in_place<melee_ft::fighter::FighterCore>` |
| 72 | 1 | `<melee_ft::fighter::state::FighterProc as core::fmt::Debug>::fmt` |
| 56 | 1 | `<melee_ft::input::pad::PadSample as core::cmp::PartialEq>::eq` |
| 54 | 1 | `<melee_ft::desc::animation::AnimationDescError as core::fmt::Debug>::fmt` |
| 48 | 1 | `core::ptr::drop_in_place<melee_ft::desc::bones::FighterBones>` |
| 41 | 1 | `core::ptr::drop_in_place<[core::option::Option<melee_ft::desc::bones::AnimationBoneSet>; 5]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::desc::animation::AnimationEntry]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::dynamics::DynamicSetDescriptor]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::fighter::assets::FighterAssets; 2]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::fighter::assets::FighterAssets]>` |
| 38 | 1 | `core::ptr::drop_in_place<melee_ft::anim::playback::FighterAnimation>` |
| 37 | 1 | `core::ptr::drop_in_place<melee_ft::anim::attach::MotionRemap>` |

### ft-captain: 43 labels, 66 emitted definitions (informational); 1 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 83 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 65 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 7 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 5 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_shield` |

### ft-falco: 44 labels, 67 emitted definitions (informational); 1 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 79 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 55 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 9 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |

### ft-fox: 44 labels, 67 emitted definitions (informational); 1 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 79 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 55 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 9 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |

### ft-fox-family: 0 labels, 0 emitted definitions (informational); 0 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mario: 0 labels, 0 emitted definitions (informational); 0 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mars: 43 labels, 66 emitted definitions (informational); 1 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 67 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 42 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::guard_variant` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 5 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 3 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_landing` |
| 3 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |

### ft-peach: 44 labels, 67 emitted definitions (informational); 1 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 65 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 64 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::check_float_input` |
| 55 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 52 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 49 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_resources_loaded` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 15 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 10 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::dynamics_first_force_bone` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |

### ft-purin: 43 labels, 66 emitted definitions (informational); 1 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 61 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 4 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::multi_jump_attributes` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::multi_jump_animation` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_shield` |

### ft-yoshi: 45 labels, 68 emitted definitions (informational); 1 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 194 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 142 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 87 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_animated` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 60 | 1 | `melee_ft::fighter::escape::<impl melee_ft::fighter::FighterCore>::roll_input` |
| 54 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 49 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 41 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_entered` |
| 28 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 18 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::action_id` |
| 17 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::input_shield` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 11 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::check_hurtbox_interaction` |

<!-- perf-gate-v1
{"architecture": "concrete-shell-v1", "census": "duplicate-labels-v1", "date": "2026-09-10T09:42:26+00:00", "metrics": {"cross_crate_duplicate_labels": 99, "definitions": {"ft-captain": 66, "ft-falco": 67, "ft-fox": 67, "ft-fox-family": 0, "ft-mario": 0, "ft-mars": 66, "ft-peach": 67, "ft-purin": 66, "ft-yoshi": 68, "melee-ft": 852, "melee-sim": 128}, "duplicate_labels": {"ft-captain": 1, "ft-falco": 1, "ft-fox": 1, "ft-fox-family": 0, "ft-mario": 0, "ft-mars": 1, "ft-peach": 1, "ft-purin": 1, "ft-yoshi": 1, "melee-ft": 20, "melee-sim": 7}, "labels": {"ft-captain": 43, "ft-falco": 44, "ft-fox": 44, "ft-fox-family": 0, "ft-mario": 0, "ft-mars": 43, "ft-peach": 44, "ft-purin": 43, "ft-yoshi": 45, "melee-ft": 815, "melee-sim": 101}, "load_ns": 441738714.5, "stripped_bytes": 3483952, "text_bytes": 3178496, "ticks_600_ns": 38297660.4}, "platform": "macOS-26.2-arm64-arm-64bit-Mach-O", "revision": "631a3835f6a7f86aed6b2f28a803eba7bb7169e3", "rustc": "rustc 1.96.0 (ac68faa20 2026-05-25)", "status": "REGRESSION"}
-->

## 2026-09-10T09:45:54+00:00 — COMPLETE — REGRESSION

Evidence: `/Users/nikhilunni/Projects/melee-lanes/chars/target/perf/20260910T094512Z-23439`. Revision `631a3835f6a7f86aed6b2f28a803eba7bb7169e3` (working tree included).

rustc 1.96.0 (ac68faa20 2026-05-25); macOS-26.2-arm64-arm-64bit-Mach-O.

Tolerance: time +10%, size +5%, duplicate labels +0 within each compiling crate and across crates.
Baseline: 2026-09-10T08:26:32+00:00.
C15 fixed ceilings: 3,747,632 stripped bytes; C15 duplicate-label baseline; one definition per common label.

| Measurement | Value |
|---|---:|
| Stripped binary | 3,483,952 bytes |
| Text (`size`) | 3,178,496 bytes |
| load mean | 168.326 ms (95% CI 167.596..169.311 ms) |
| ticks_600 mean | 30.140 ms (95% CI 24.898..37.811 ms) |
| Headless throughput | 19,907 ticks/s |
| Total emitted definitions (informational) | 1,447 |
| Labels defined in multiple crates | 99 |
| Common definition labels audited across crates | 271 |

- ticks_600_ns: 30139720.900 > 26444488.950 (previous 24040444.500, +10%)

- P1 time ceiling ticks_600_ns: 30139720.900 > 25947000

Tool: `0.12.1`.

Tool: `cargo-llvm-lines 0.4.48`.

Per-crate text contribution (cargo-bloat estimates):
```text
File  .text     Size Crate
12.8%  25.8% 614.3KiB melee_sim
 9.8%  19.7% 468.8KiB std
 7.2%  14.4% 343.3KiB melee_ft
 5.6%  11.2% 266.1KiB clap_builder
 1.7%   3.4%  82.1KiB melee_ef
 1.4%   2.8%  67.6KiB slp
 1.3%   2.7%  64.2KiB melee_mp
 1.2%   2.5%  59.3KiB melee_gr
 1.1%   2.2%  51.5KiB hsd_anim
 1.1%   2.1%  50.7KiB hsd_archive
 0.9%   1.7%  41.5KiB toml
 0.8%   1.7%  39.3KiB hsd_particle
 0.8%   1.5%  36.5KiB toml_parser
 0.7%   1.5%  35.5KiB serde_json
 0.4%   0.7%  17.3KiB melee_lb
 0.4%   0.7%  16.9KiB serde_core
 0.3%   0.6%  15.4KiB melee_diff
 0.3%   0.6%  14.6KiB ft_yoshi
 0.3%   0.6%  13.5KiB hsd_gobj
 0.2%   0.5%  11.1KiB melee_it
 0.2%   0.5%  10.8KiB melee_types
 0.2%   0.3%   8.0KiB ft_fox_family
 0.1%   0.3%   6.9KiB anyhow
 0.1%   0.2%   5.1KiB toml_datetime
 0.1%   0.2%   4.1KiB melee_cmd
 0.1%   0.1%   3.4KiB ft_peach
 0.1%   0.1%   3.4KiB melee_coll
 0.1%   0.1%   3.2KiB it_foxlaser
 0.1%   0.1%   3.1KiB [Unknown]
 0.1%   0.1%   3.1KiB ft_purin
 0.1%   0.1%   2.8KiB clap_lex
 0.1%   0.1%   2.7KiB gekko_math
 0.0%   0.1%   2.3KiB ft_mars
 0.0%   0.1%   2.2KiB ft_captain
 0.0%   0.1%   2.1KiB serde
 0.0%   0.1%   2.1KiB melee_if
 0.0%   0.1%   1.7KiB anstream
 0.0%   0.1%   1.6KiB anstyle
 0.0%   0.1%   1.6KiB ft_falco
 0.0%   0.1%   1.6KiB ft_fox
 0.0%   0.1%   1.4KiB strsim
 0.0%   0.0%   1.0KiB zmij
 0.0%   0.0%     960B itoa
 0.0%   0.0%     848B toml_writer
 0.0%   0.0%     312B winnow
 0.0%   0.0%     264B hsd_types
 0.0%   0.0%      20B __rustc
 0.0%   0.0%      16B colorchoice
49.8% 100.0%   2.3MiB .text section size, the file size is 4.7MiB

Note: numbers above are a result of guesswork. They are not 100% correct and never will be.
```

### melee-ft: 815 labels, 852 emitted definitions (informational); 20 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 21974 | 1 | `melee_ft::fighter::assets::FighterAssets::load` |
| 2411 | 1 | `melee_ft::fighter::damage::DamageParameters::read` |
| 2311 | 1 | `melee_ft::fighter::state::common_table::common_table` |
| 1682 | 1 | `melee_ft::fighter::shield::ShieldParameters::read` |
| 1374 | 1 | `melee_ft::desc::common::CommonFighterData::read` |
| 1343 | 1 | `melee_ft::input::common::InputCommonData::read` |
| 1231 | 1 | `melee_ft::dynamics::read_sets` |
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

### melee-sim: 101 labels, 128 emitted definitions (informational); 7 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 478 | 7 | `melee_ft::fighter::character::CharacterState::new` |
| 282 | 1 | `melee_ft::anim::playback::FighterAnimation::advance_main` |
| 252 | 1 | `melee_ft::anim::root_motion::RootMotion::evaluate` |
| 229 | 1 | `melee_ft::anim::blend::blend_rotation` |
| 205 | 1 | `core::ptr::drop_in_place<melee_ft::fighter::assets::FighterAssets>` |
| 164 | 1 | `melee_ft::anim::playback::FighterAnimation::advance_parts` |
| 130 | 1 | `melee_ft::anim::blend::blend_pose` |
| 110 | 1 | `melee_ft::anim::playback::animate_parts` |
| 109 | 1 | `core::ptr::drop_in_place<melee_ft::fighter::FighterCore>` |
| 72 | 1 | `<melee_ft::fighter::state::FighterProc as core::fmt::Debug>::fmt` |
| 56 | 1 | `<melee_ft::input::pad::PadSample as core::cmp::PartialEq>::eq` |
| 54 | 1 | `<melee_ft::desc::animation::AnimationDescError as core::fmt::Debug>::fmt` |
| 48 | 1 | `core::ptr::drop_in_place<melee_ft::desc::bones::FighterBones>` |
| 41 | 1 | `core::ptr::drop_in_place<[core::option::Option<melee_ft::desc::bones::AnimationBoneSet>; 5]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::desc::animation::AnimationEntry]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::dynamics::DynamicSetDescriptor]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::fighter::assets::FighterAssets; 2]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::fighter::assets::FighterAssets]>` |
| 38 | 1 | `core::ptr::drop_in_place<melee_ft::anim::playback::FighterAnimation>` |
| 37 | 1 | `core::ptr::drop_in_place<melee_ft::anim::attach::MotionRemap>` |

### ft-captain: 43 labels, 66 emitted definitions (informational); 1 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 83 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 65 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 7 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 5 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_shield` |

### ft-falco: 44 labels, 67 emitted definitions (informational); 1 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 79 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 55 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 9 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |

### ft-fox: 44 labels, 67 emitted definitions (informational); 1 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 79 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 55 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 9 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |

### ft-fox-family: 0 labels, 0 emitted definitions (informational); 0 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mario: 0 labels, 0 emitted definitions (informational); 0 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mars: 43 labels, 66 emitted definitions (informational); 1 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 67 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 42 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::guard_variant` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 5 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 3 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_landing` |
| 3 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |

### ft-peach: 44 labels, 67 emitted definitions (informational); 1 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 65 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 64 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::check_float_input` |
| 55 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 52 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 49 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_resources_loaded` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 15 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 10 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::dynamics_first_force_bone` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |

### ft-purin: 43 labels, 66 emitted definitions (informational); 1 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 61 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 4 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::multi_jump_attributes` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::multi_jump_animation` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_shield` |

### ft-yoshi: 45 labels, 68 emitted definitions (informational); 1 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 194 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 142 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 87 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_animated` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 60 | 1 | `melee_ft::fighter::escape::<impl melee_ft::fighter::FighterCore>::roll_input` |
| 54 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 49 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 41 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_entered` |
| 28 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 18 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::action_id` |
| 17 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::input_shield` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 11 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::check_hurtbox_interaction` |

<!-- perf-gate-v1
{"architecture": "concrete-shell-v1", "census": "duplicate-labels-v1", "date": "2026-09-10T09:45:54+00:00", "metrics": {"cross_crate_duplicate_labels": 99, "definitions": {"ft-captain": 66, "ft-falco": 67, "ft-fox": 67, "ft-fox-family": 0, "ft-mario": 0, "ft-mars": 66, "ft-peach": 67, "ft-purin": 66, "ft-yoshi": 68, "melee-ft": 852, "melee-sim": 128}, "duplicate_labels": {"ft-captain": 1, "ft-falco": 1, "ft-fox": 1, "ft-fox-family": 0, "ft-mario": 0, "ft-mars": 1, "ft-peach": 1, "ft-purin": 1, "ft-yoshi": 1, "melee-ft": 20, "melee-sim": 7}, "labels": {"ft-captain": 43, "ft-falco": 44, "ft-fox": 44, "ft-fox-family": 0, "ft-mario": 0, "ft-mars": 43, "ft-peach": 44, "ft-purin": 43, "ft-yoshi": 45, "melee-ft": 815, "melee-sim": 101}, "load_ns": 168325926.61087304, "stripped_bytes": 3483952, "text_bytes": 3178496, "ticks_600_ns": 30139720.9}, "platform": "macOS-26.2-arm64-arm-64bit-Mach-O", "revision": "631a3835f6a7f86aed6b2f28a803eba7bb7169e3", "rustc": "rustc 1.96.0 (ac68faa20 2026-05-25)", "status": "REGRESSION"}
-->

## 2026-09-10T08:41:17+00:00 — COMPLETE — REGRESSION

Evidence: `/Users/nikhilunni/Projects/melee-lanes/battlefield/target/perf/20260910T084036Z-28887`. Revision `631a3835f6a7f86aed6b2f28a803eba7bb7169e3` (working tree included).

rustc 1.96.0 (ac68faa20 2026-05-25); macOS-26.2-arm64-arm-64bit-Mach-O.

Tolerance: time +10%, size +5%, copies +0 per compiling crate.
Baseline: 2026-09-10T08:26:32+00:00.
C15 fixed ceilings: 3,747,632 stripped bytes; 1,440 total copies; one definition per common label.

| Measurement | Value |
|---|---:|
| Stripped binary | 3,483,280 bytes |
| Text (`size`) | 3,178,496 bytes |
| load mean | 238.601 ms (95% CI 200.763..286.474 ms) |
| ticks_600 mean | 25.362 ms (95% CI 23.868..27.706 ms) |
| Headless throughput | 23,657 ticks/s |
| Total melee-ft copies | 1,452 |
| Common definition labels audited across crates | 274 |

- load_ns: 238600642.409 > 184991235.262 (previous 168173850.238, +10%)

- melee-ft melee-ft copies: 857 > 845 + 0

- P1 time ceiling load_ns: 238600642.409 > 182600000

- C15 melee-ft copies: 857 > 845

- C15 total copies: 1452 > 1440

Tool: `0.12.1`.

Tool: `cargo-llvm-lines 0.4.48`.

Per-crate text contribution (cargo-bloat estimates):
```text
File  .text     Size Crate
12.8%  25.8% 614.3KiB melee_sim
 9.8%  19.7% 467.7KiB std
 7.2%  14.5% 343.8KiB melee_ft
 5.6%  11.2% 266.1KiB clap_builder
 1.7%   3.5%  82.1KiB melee_ef
 1.4%   2.8%  67.6KiB slp
 1.3%   2.7%  64.3KiB melee_mp
 1.2%   2.5%  59.3KiB melee_gr
 1.1%   2.1%  50.8KiB hsd_anim
 1.1%   2.1%  50.7KiB hsd_archive
 0.9%   1.7%  41.5KiB toml
 0.8%   1.7%  39.3KiB hsd_particle
 0.8%   1.5%  36.5KiB toml_parser
 0.7%   1.5%  35.5KiB serde_json
 0.4%   0.7%  16.9KiB serde_core
 0.3%   0.6%  15.4KiB melee_diff
 0.3%   0.6%  14.6KiB melee_lb
 0.3%   0.6%  14.6KiB ft_yoshi
 0.3%   0.6%  13.5KiB hsd_gobj
 0.2%   0.5%  11.1KiB melee_it
 0.2%   0.5%  10.8KiB melee_types
 0.2%   0.3%   8.0KiB ft_fox_family
 0.1%   0.3%   6.9KiB anyhow
 0.1%   0.2%   5.1KiB toml_datetime
 0.1%   0.2%   4.1KiB melee_cmd
 0.1%   0.1%   3.4KiB ft_peach
 0.1%   0.1%   3.4KiB melee_coll
 0.1%   0.1%   3.2KiB [Unknown]
 0.1%   0.1%   3.2KiB it_foxlaser
 0.1%   0.1%   3.1KiB ft_purin
 0.1%   0.1%   2.8KiB clap_lex
 0.1%   0.1%   2.7KiB gekko_math
 0.0%   0.1%   2.3KiB ft_mars
 0.0%   0.1%   2.2KiB ft_captain
 0.0%   0.1%   2.1KiB serde
 0.0%   0.1%   2.1KiB melee_if
 0.0%   0.1%   1.7KiB anstream
 0.0%   0.1%   1.6KiB ft_falco
 0.0%   0.1%   1.6KiB ft_fox
 0.0%   0.1%   1.6KiB anstyle
 0.0%   0.1%   1.4KiB strsim
 0.0%   0.0%   1.0KiB zmij
 0.0%   0.0%     960B itoa
 0.0%   0.0%     848B toml_writer
 0.0%   0.0%     312B winnow
 0.0%   0.0%     264B hsd_types
 0.0%   0.0%      20B __rustc
 0.0%   0.0%      16B colorchoice
49.7% 100.0%   2.3MiB .text section size, the file size is 4.7MiB

Note: numbers above are a result of guesswork. They are not 100% correct and never will be.
```

### melee-ft: 857 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 21982 | 1 | `melee_ft::fighter::assets::FighterAssets::load` |
| 3187 | 1 | `melee_ft::fighter::damage::DamageParameters::read` |
| 2273 | 1 | `melee_ft::fighter::state::common_table::common_table` |
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

### melee-sim: 128 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 478 | 7 | `melee_ft::fighter::character::CharacterState::new` |
| 282 | 1 | `melee_ft::anim::playback::FighterAnimation::advance_main` |
| 252 | 1 | `melee_ft::anim::root_motion::RootMotion::evaluate` |
| 229 | 1 | `melee_ft::anim::blend::blend_rotation` |
| 205 | 1 | `core::ptr::drop_in_place<melee_ft::fighter::assets::FighterAssets>` |
| 164 | 1 | `melee_ft::anim::playback::FighterAnimation::advance_parts` |
| 130 | 1 | `melee_ft::anim::blend::blend_pose` |
| 110 | 1 | `melee_ft::anim::playback::animate_parts` |
| 109 | 1 | `core::ptr::drop_in_place<melee_ft::fighter::FighterCore>` |
| 72 | 1 | `<melee_ft::fighter::state::FighterProc as core::fmt::Debug>::fmt` |
| 56 | 1 | `<melee_ft::input::pad::PadSample as core::cmp::PartialEq>::eq` |
| 54 | 1 | `<melee_ft::desc::animation::AnimationDescError as core::fmt::Debug>::fmt` |
| 48 | 1 | `core::ptr::drop_in_place<melee_ft::desc::bones::FighterBones>` |
| 41 | 1 | `core::ptr::drop_in_place<[core::option::Option<melee_ft::desc::bones::AnimationBoneSet>; 5]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::desc::animation::AnimationEntry]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::dynamics::DynamicSetDescriptor]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::fighter::assets::FighterAssets; 2]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::fighter::assets::FighterAssets]>` |
| 38 | 1 | `core::ptr::drop_in_place<melee_ft::anim::playback::FighterAnimation>` |
| 37 | 1 | `core::ptr::drop_in_place<melee_ft::anim::attach::MotionRemap>` |

### ft-captain: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 83 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 65 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 7 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 5 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_shield` |

### ft-falco: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 79 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 55 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 9 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |

### ft-fox: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 79 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 55 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 9 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |

### ft-fox-family: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mario: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mars: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 67 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 42 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::guard_variant` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 5 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 3 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_landing` |
| 3 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |

### ft-peach: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 65 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 64 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::check_float_input` |
| 55 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 52 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 49 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_resources_loaded` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 15 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 10 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::dynamics_first_force_bone` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |

### ft-purin: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 61 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 4 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::multi_jump_attributes` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::multi_jump_animation` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_shield` |

### ft-yoshi: 68 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 194 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 142 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 87 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_animated` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 60 | 1 | `melee_ft::fighter::escape::<impl melee_ft::fighter::FighterCore>::roll_input` |
| 54 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 49 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 41 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_entered` |
| 28 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 18 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::action_id` |
| 17 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::input_shield` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 11 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::check_hurtbox_interaction` |

<!-- perf-gate-v1
{"architecture": "concrete-shell-v1", "date": "2026-09-10T08:41:17+00:00", "metrics": {"copies": {"ft-captain": 66, "ft-falco": 67, "ft-fox": 67, "ft-fox-family": 0, "ft-mario": 0, "ft-mars": 66, "ft-peach": 67, "ft-purin": 66, "ft-yoshi": 68, "melee-ft": 857, "melee-sim": 128}, "load_ns": 238600642.40916666, "stripped_bytes": 3483280, "text_bytes": 3178496, "ticks_600_ns": 25362090.333333332}, "platform": "macOS-26.2-arm64-arm-64bit-Mach-O", "revision": "631a3835f6a7f86aed6b2f28a803eba7bb7169e3", "rustc": "rustc 1.96.0 (ac68faa20 2026-05-25)", "status": "REGRESSION"}
-->

## 2026-09-10T08:43:49+00:00 — COMPLETE — REGRESSION

Evidence: `/Users/nikhilunni/Projects/melee-lanes/battlefield/target/perf/20260910T084255Z-31050`. Revision `631a3835f6a7f86aed6b2f28a803eba7bb7169e3` (working tree included).

rustc 1.96.0 (ac68faa20 2026-05-25); macOS-26.2-arm64-arm-64bit-Mach-O.

Tolerance: time +10%, size +5%, copies +0 per compiling crate.
Baseline: 2026-09-10T08:26:32+00:00.
C15 fixed ceilings: 3,747,632 stripped bytes; 1,440 total copies; one definition per common label.

| Measurement | Value |
|---|---:|
| Stripped binary | 3,483,104 bytes |
| Text (`size`) | 3,178,496 bytes |
| load mean | 255.688 ms (95% CI 235.165..281.297 ms) |
| ticks_600 mean | 71.934 ms (95% CI 49.089..96.199 ms) |
| Headless throughput | 8,341 ticks/s |
| Total melee-ft copies | 1,440 |
| Common definition labels audited across crates | 274 |

- load_ns: 255687643.200 > 184991235.262 (previous 168173850.238, +10%)

- ticks_600_ns: 71933589.550 > 26444488.950 (previous 24040444.500, +10%)

- P1 time ceiling load_ns: 255687643.200 > 182600000

- P1 time ceiling ticks_600_ns: 71933589.550 > 25947000

Tool: `0.12.1`.

Tool: `cargo-llvm-lines 0.4.48`.

Per-crate text contribution (cargo-bloat estimates):
```text
File  .text     Size Crate
12.9%  26.0% 616.2KiB melee_sim
 9.8%  19.7% 467.4KiB std
 7.0%  14.2% 335.9KiB melee_ft
 5.6%  11.2% 265.7KiB clap_builder
 1.7%   3.5%  82.1KiB melee_ef
 1.4%   2.9%  67.8KiB slp
 1.3%   2.7%  64.3KiB melee_mp
 1.2%   2.5%  59.3KiB melee_gr
 1.1%   2.1%  50.8KiB hsd_anim
 1.1%   2.1%  50.7KiB hsd_archive
 0.9%   1.7%  41.5KiB toml
 0.8%   1.7%  40.5KiB serde_json
 0.8%   1.6%  36.9KiB hsd_particle
 0.8%   1.5%  36.5KiB toml_parser
 0.4%   0.7%  16.8KiB serde_core
 0.3%   0.7%  15.4KiB melee_diff
 0.3%   0.6%  14.6KiB melee_lb
 0.3%   0.6%  14.6KiB ft_yoshi
 0.3%   0.6%  13.5KiB hsd_gobj
 0.2%   0.5%  11.3KiB melee_types
 0.2%   0.5%  11.1KiB melee_it
 0.2%   0.3%   8.0KiB ft_fox_family
 0.1%   0.3%   6.9KiB anyhow
 0.1%   0.2%   5.1KiB toml_datetime
 0.1%   0.2%   4.1KiB melee_cmd
 0.1%   0.1%   3.4KiB ft_peach
 0.1%   0.1%   3.2KiB it_foxlaser
 0.1%   0.1%   3.2KiB [Unknown]
 0.1%   0.1%   3.1KiB ft_purin
 0.1%   0.1%   2.8KiB clap_lex
 0.1%   0.1%   2.7KiB gekko_math
 0.1%   0.1%   2.5KiB melee_coll
 0.0%   0.1%   2.3KiB ft_mars
 0.0%   0.1%   2.2KiB ft_captain
 0.0%   0.1%   2.1KiB serde
 0.0%   0.1%   1.9KiB melee_if
 0.0%   0.1%   1.7KiB anstream
 0.0%   0.1%   1.6KiB ft_falco
 0.0%   0.1%   1.6KiB ft_fox
 0.0%   0.1%   1.6KiB anstyle
 0.0%   0.1%   1.4KiB strsim
 0.0%   0.0%   1.0KiB zmij
 0.0%   0.0%     960B itoa
 0.0%   0.0%     848B toml_writer
 0.0%   0.0%     312B winnow
 0.0%   0.0%     264B hsd_types
 0.0%   0.0%      20B __rustc
 0.0%   0.0%      16B colorchoice
49.6% 100.0%   2.3MiB .text section size, the file size is 4.7MiB

Note: numbers above are a result of guesswork. They are not 100% correct and never will be.
```

### melee-ft: 845 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 21980 | 1 | `melee_ft::fighter::assets::FighterAssets::load` |
| 3187 | 1 | `melee_ft::fighter::damage::DamageParameters::read` |
| 2273 | 1 | `melee_ft::fighter::state::common_table::common_table` |
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

### melee-sim: 128 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 478 | 7 | `melee_ft::fighter::character::CharacterState::new` |
| 282 | 1 | `melee_ft::anim::playback::FighterAnimation::advance_main` |
| 252 | 1 | `melee_ft::anim::root_motion::RootMotion::evaluate` |
| 229 | 1 | `melee_ft::anim::blend::blend_rotation` |
| 205 | 1 | `core::ptr::drop_in_place<melee_ft::fighter::assets::FighterAssets>` |
| 164 | 1 | `melee_ft::anim::playback::FighterAnimation::advance_parts` |
| 130 | 1 | `melee_ft::anim::blend::blend_pose` |
| 110 | 1 | `melee_ft::anim::playback::animate_parts` |
| 109 | 1 | `core::ptr::drop_in_place<melee_ft::fighter::FighterCore>` |
| 72 | 1 | `<melee_ft::fighter::state::FighterProc as core::fmt::Debug>::fmt` |
| 56 | 1 | `<melee_ft::input::pad::PadSample as core::cmp::PartialEq>::eq` |
| 54 | 1 | `<melee_ft::desc::animation::AnimationDescError as core::fmt::Debug>::fmt` |
| 48 | 1 | `core::ptr::drop_in_place<melee_ft::desc::bones::FighterBones>` |
| 41 | 1 | `core::ptr::drop_in_place<[core::option::Option<melee_ft::desc::bones::AnimationBoneSet>; 5]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::desc::animation::AnimationEntry]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::dynamics::DynamicSetDescriptor]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::fighter::assets::FighterAssets; 2]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::fighter::assets::FighterAssets]>` |
| 38 | 1 | `core::ptr::drop_in_place<melee_ft::anim::playback::FighterAnimation>` |
| 37 | 1 | `core::ptr::drop_in_place<melee_ft::anim::attach::MotionRemap>` |

### ft-captain: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 83 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 65 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 7 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 5 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_shield` |

### ft-falco: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 79 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 55 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 9 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |

### ft-fox: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 79 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 55 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 9 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |

### ft-fox-family: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mario: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mars: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 67 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 42 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::guard_variant` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 5 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 3 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_landing` |
| 3 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |

### ft-peach: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 65 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 64 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::check_float_input` |
| 55 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 52 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 49 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_resources_loaded` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 15 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 10 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::dynamics_first_force_bone` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |

### ft-purin: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 61 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 4 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::multi_jump_attributes` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::multi_jump_animation` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_shield` |

### ft-yoshi: 68 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 194 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 142 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 87 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_animated` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 60 | 1 | `melee_ft::fighter::escape::<impl melee_ft::fighter::FighterCore>::roll_input` |
| 54 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 49 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 41 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_entered` |
| 28 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 18 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::action_id` |
| 17 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::input_shield` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 11 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::check_hurtbox_interaction` |

<!-- perf-gate-v1
{"architecture": "concrete-shell-v1", "date": "2026-09-10T08:43:49+00:00", "metrics": {"copies": {"ft-captain": 66, "ft-falco": 67, "ft-fox": 67, "ft-fox-family": 0, "ft-mario": 0, "ft-mars": 66, "ft-peach": 67, "ft-purin": 66, "ft-yoshi": 68, "melee-ft": 845, "melee-sim": 128}, "load_ns": 255687643.2, "stripped_bytes": 3483104, "text_bytes": 3178496, "ticks_600_ns": 71933589.55}, "platform": "macOS-26.2-arm64-arm-64bit-Mach-O", "revision": "631a3835f6a7f86aed6b2f28a803eba7bb7169e3", "rustc": "rustc 1.96.0 (ac68faa20 2026-05-25)", "status": "REGRESSION"}
-->

## 2026-09-10T09:01:39+00:00 — COMPLETE — PASS

Evidence: `/Users/nikhilunni/Projects/melee-lanes/battlefield/target/perf/20260910T090054Z-57610`. Revision `631a3835f6a7f86aed6b2f28a803eba7bb7169e3` (working tree included).

rustc 1.96.0 (ac68faa20 2026-05-25); macOS-26.2-arm64-arm-64bit-Mach-O.

Tolerance: time +10%, size +5%, copies +0 per compiling crate.
Baseline: 2026-09-10T08:26:32+00:00.
C15 fixed ceilings: 3,747,632 stripped bytes; 1,440 total copies; one definition per common label.

| Measurement | Value |
|---|---:|
| Stripped binary | 3,483,104 bytes |
| Text (`size`) | 3,178,496 bytes |
| load mean | 168.760 ms (95% CI 168.469..169.014 ms) |
| ticks_600 mean | 23.675 ms (95% CI 23.470..23.998 ms) |
| Headless throughput | 25,343 ticks/s |
| Total melee-ft copies | 1,440 |
| Common definition labels audited across crates | 274 |

Tool: `0.12.1`.

Tool: `cargo-llvm-lines 0.4.48`.

Per-crate text contribution (cargo-bloat estimates):
```text
File  .text     Size Crate
12.9%  26.0% 616.2KiB melee_sim
 9.8%  19.7% 467.4KiB std
 7.0%  14.2% 335.9KiB melee_ft
 5.6%  11.2% 265.7KiB clap_builder
 1.7%   3.5%  82.1KiB melee_ef
 1.4%   2.9%  67.8KiB slp
 1.3%   2.7%  64.3KiB melee_mp
 1.2%   2.5%  59.3KiB melee_gr
 1.1%   2.1%  50.8KiB hsd_anim
 1.1%   2.1%  50.7KiB hsd_archive
 0.9%   1.7%  41.5KiB toml
 0.8%   1.7%  40.5KiB serde_json
 0.8%   1.6%  36.9KiB hsd_particle
 0.8%   1.5%  36.5KiB toml_parser
 0.4%   0.7%  16.8KiB serde_core
 0.3%   0.7%  15.4KiB melee_diff
 0.3%   0.6%  14.6KiB melee_lb
 0.3%   0.6%  14.6KiB ft_yoshi
 0.3%   0.6%  13.5KiB hsd_gobj
 0.2%   0.5%  11.3KiB melee_types
 0.2%   0.5%  11.1KiB melee_it
 0.2%   0.3%   8.0KiB ft_fox_family
 0.1%   0.3%   6.9KiB anyhow
 0.1%   0.2%   5.1KiB toml_datetime
 0.1%   0.2%   4.1KiB melee_cmd
 0.1%   0.1%   3.4KiB ft_peach
 0.1%   0.1%   3.2KiB [Unknown]
 0.1%   0.1%   3.2KiB it_foxlaser
 0.1%   0.1%   3.1KiB ft_purin
 0.1%   0.1%   2.8KiB clap_lex
 0.1%   0.1%   2.7KiB gekko_math
 0.1%   0.1%   2.5KiB melee_coll
 0.0%   0.1%   2.3KiB ft_mars
 0.0%   0.1%   2.2KiB ft_captain
 0.0%   0.1%   2.1KiB serde
 0.0%   0.1%   1.9KiB melee_if
 0.0%   0.1%   1.7KiB anstream
 0.0%   0.1%   1.6KiB ft_fox
 0.0%   0.1%   1.6KiB ft_falco
 0.0%   0.1%   1.6KiB anstyle
 0.0%   0.1%   1.4KiB strsim
 0.0%   0.0%   1.0KiB zmij
 0.0%   0.0%     960B itoa
 0.0%   0.0%     848B toml_writer
 0.0%   0.0%     312B winnow
 0.0%   0.0%     264B hsd_types
 0.0%   0.0%      20B __rustc
 0.0%   0.0%      16B colorchoice
49.6% 100.0%   2.3MiB .text section size, the file size is 4.7MiB

Note: numbers above are a result of guesswork. They are not 100% correct and never will be.
```

### melee-ft: 845 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 21980 | 1 | `melee_ft::fighter::assets::FighterAssets::load` |
| 3187 | 1 | `melee_ft::fighter::damage::DamageParameters::read` |
| 2273 | 1 | `melee_ft::fighter::state::common_table::common_table` |
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

### melee-sim: 128 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 478 | 7 | `melee_ft::fighter::character::CharacterState::new` |
| 282 | 1 | `melee_ft::anim::playback::FighterAnimation::advance_main` |
| 252 | 1 | `melee_ft::anim::root_motion::RootMotion::evaluate` |
| 229 | 1 | `melee_ft::anim::blend::blend_rotation` |
| 205 | 1 | `core::ptr::drop_in_place<melee_ft::fighter::assets::FighterAssets>` |
| 164 | 1 | `melee_ft::anim::playback::FighterAnimation::advance_parts` |
| 130 | 1 | `melee_ft::anim::blend::blend_pose` |
| 110 | 1 | `melee_ft::anim::playback::animate_parts` |
| 109 | 1 | `core::ptr::drop_in_place<melee_ft::fighter::FighterCore>` |
| 72 | 1 | `<melee_ft::fighter::state::FighterProc as core::fmt::Debug>::fmt` |
| 56 | 1 | `<melee_ft::input::pad::PadSample as core::cmp::PartialEq>::eq` |
| 54 | 1 | `<melee_ft::desc::animation::AnimationDescError as core::fmt::Debug>::fmt` |
| 48 | 1 | `core::ptr::drop_in_place<melee_ft::desc::bones::FighterBones>` |
| 41 | 1 | `core::ptr::drop_in_place<[core::option::Option<melee_ft::desc::bones::AnimationBoneSet>; 5]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::desc::animation::AnimationEntry]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::dynamics::DynamicSetDescriptor]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::fighter::assets::FighterAssets; 2]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::fighter::assets::FighterAssets]>` |
| 38 | 1 | `core::ptr::drop_in_place<melee_ft::anim::playback::FighterAnimation>` |
| 37 | 1 | `core::ptr::drop_in_place<melee_ft::anim::attach::MotionRemap>` |

### ft-captain: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 83 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 65 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 7 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 5 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_shield` |

### ft-falco: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 79 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 55 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 9 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |

### ft-fox: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 79 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 55 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 9 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |

### ft-fox-family: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mario: 0 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mars: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 67 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 42 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::guard_variant` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 5 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 3 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_landing` |
| 3 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |

### ft-peach: 67 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 65 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 64 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::check_float_input` |
| 55 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 52 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 49 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_resources_loaded` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 15 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 10 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::dynamics_first_force_bone` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |

### ft-purin: 66 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 61 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 4 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::multi_jump_attributes` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::multi_jump_animation` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_shield` |

### ft-yoshi: 68 melee-ft copies

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 194 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 142 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 87 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_animated` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 60 | 1 | `melee_ft::fighter::escape::<impl melee_ft::fighter::FighterCore>::roll_input` |
| 54 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 49 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 41 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_entered` |
| 28 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 28 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 18 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::action_id` |
| 17 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::input_shield` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 11 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::check_hurtbox_interaction` |

<!-- perf-gate-v1
{"architecture": "concrete-shell-v1", "date": "2026-09-10T09:01:39+00:00", "metrics": {"copies": {"ft-captain": 66, "ft-falco": 67, "ft-fox": 67, "ft-fox-family": 0, "ft-mario": 0, "ft-mars": 66, "ft-peach": 67, "ft-purin": 66, "ft-yoshi": 68, "melee-ft": 845, "melee-sim": 128}, "load_ns": 168760173.42535716, "stripped_bytes": 3483104, "text_bytes": 3178496, "ticks_600_ns": 23675379.200000003}, "platform": "macOS-26.2-arm64-arm-64bit-Mach-O", "revision": "631a3835f6a7f86aed6b2f28a803eba7bb7169e3", "rustc": "rustc 1.96.0 (ac68faa20 2026-05-25)", "status": "PASS"}
-->

## 2026-09-10T10:28:34+00:00 — COMPLETE — REGRESSION

Evidence: `/Users/nikhilunni/Projects/melee/target/perf/20260910T102741Z-95943`. Revision `77c28e8b230eec991f01e7d350ede14d3658241f` (working tree included).

rustc 1.96.0 (ac68faa20 2026-05-25); macOS-26.2-arm64-arm-64bit-Mach-O.

Tolerance: time +10%, size +5%, duplicate labels +0 within each compiling crate and across crates.
Baseline: 2026-09-10T09:01:39+00:00.
C15 fixed ceilings: 3,747,632 stripped bytes; C15 duplicate-label baseline; one definition per common label.

| Measurement | Value |
|---|---:|
| Stripped binary | 3,518,384 bytes |
| Text (`size`) | 3,211,264 bytes |
| load mean | 416.350 ms (95% CI 365.429..465.474 ms) |
| ticks_600 mean | 76.365 ms (95% CI 54.597..100.035 ms) |
| Headless throughput | 7,857 ticks/s |
| Total emitted definitions (informational) | 1,447 |
| Labels defined in multiple crates | 99 |
| Common definition labels audited across crates | 275 |

- load_ns: 416349633.250 > 185636190.768 (previous 168760173.425, +10%)

- ticks_600_ns: 76364881.200 > 26042917.120 (previous 23675379.200, +10%)

- P1 time ceiling load_ns: 416349633.250 > 182600000

- P1 time ceiling ticks_600_ns: 76364881.200 > 25947000

Tool: `0.12.1`.

Tool: `cargo-llvm-lines 0.4.48`.

Per-crate text contribution (cargo-bloat estimates):
```text
File  .text     Size Crate
12.7%  25.6% 615.2KiB melee_sim
 9.7%  19.5% 467.9KiB std
 7.1%  14.2% 341.3KiB melee_ft
 5.5%  11.1% 266.1KiB clap_builder
 1.7%   3.4%  82.5KiB melee_ef
 1.4%   2.8%  67.6KiB slp
 1.3%   2.7%  64.3KiB melee_mp
 1.2%   2.5%  59.3KiB melee_gr
 1.1%   2.1%  51.5KiB hsd_anim
 1.0%   2.1%  50.7KiB hsd_archive
 0.9%   1.7%  41.5KiB toml
 0.8%   1.7%  41.1KiB serde_json
 0.8%   1.5%  36.9KiB hsd_particle
 0.8%   1.5%  36.5KiB toml_parser
 0.4%   0.7%  17.3KiB melee_lb
 0.4%   0.7%  16.9KiB serde_core
 0.3%   0.6%  15.5KiB melee_diff
 0.3%   0.6%  14.6KiB ft_yoshi
 0.3%   0.6%  13.5KiB hsd_gobj
 0.2%   0.5%  11.3KiB melee_it
 0.2%   0.5%  11.3KiB melee_types
 0.2%   0.4%   8.6KiB ft_fox_family
 0.2%   0.4%   8.6KiB ft_falco
 0.2%   0.4%   8.5KiB ft_fox
 0.1%   0.3%   6.9KiB anyhow
 0.1%   0.2%   5.1KiB toml_datetime
 0.1%   0.2%   4.1KiB melee_cmd
 0.1%   0.1%   3.4KiB ft_peach
 0.1%   0.1%   3.2KiB it_foxlaser
 0.1%   0.1%   3.1KiB [Unknown]
 0.1%   0.1%   3.1KiB ft_purin
 0.1%   0.1%   2.8KiB clap_lex
 0.1%   0.1%   2.7KiB gekko_math
 0.1%   0.1%   2.5KiB melee_coll
 0.0%   0.1%   2.3KiB ft_mars
 0.0%   0.1%   2.2KiB ft_captain
 0.0%   0.1%   2.1KiB serde
 0.0%   0.1%   1.9KiB melee_if
 0.0%   0.1%   1.7KiB anstream
 0.0%   0.1%   1.6KiB anstyle
 0.0%   0.1%   1.4KiB strsim
 0.0%   0.0%   1.0KiB zmij
 0.0%   0.0%     960B itoa
 0.0%   0.0%     848B toml_writer
 0.0%   0.0%     376B it_foxillusion
 0.0%   0.0%     312B winnow
 0.0%   0.0%     264B hsd_types
 0.0%   0.0%      20B __rustc
 0.0%   0.0%      16B colorchoice
49.6% 100.0%   2.3MiB .text section size, the file size is 4.7MiB

Note: numbers above are a result of guesswork. They are not 100% correct and never will be.
```

### melee-ft: 817 labels, 852 emitted definitions (informational); 20 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 21980 | 1 | `melee_ft::fighter::assets::FighterAssets::load` |
| 3187 | 1 | `melee_ft::fighter::damage::DamageParameters::read` |
| 2493 | 1 | `melee_ft::fighter::state::common_table::common_table` |
| 1682 | 1 | `melee_ft::fighter::shield::ShieldParameters::read` |
| 1374 | 1 | `melee_ft::desc::common::CommonFighterData::read` |
| 1343 | 1 | `melee_ft::input::common::InputCommonData::read` |
| 1231 | 1 | `melee_ft::dynamics::read_sets` |
| 1101 | 1 | `melee_ft::desc::playback::read_playback_motion` |
| 1064 | 1 | `melee_ft::fighter::spawn::<impl melee_ft::fighter::FighterCore>::prepare` |
| 1042 | 1 | `melee_ft::desc::bones::read_fighter_bones` |
| 956 | 1 | `melee_ft::fighter::commands::CommandState::step_inner` |
| 929 | 1 | `melee_ft::desc::fox_attributes::FireFoxAttributes::read` |
| 885 | 1 | `melee_ft::desc::attributes::FighterAttributes::read` |
| 844 | 1 | `melee_ft::dynamics::read_motion_starts` |
| 760 | 1 | `melee_ft::anim::attach::select_motion` |
| 695 | 1 | `melee_ft::fighter::smash::read_overlay` |
| 682 | 1 | `melee_ft::anim::playback::FighterAnimation::resume_dynamic_subtree` |
| 659 | 1 | `melee_ft::desc::fox_attributes::IllusionAttributes::read` |
| 640 | 1 | `melee_ft::desc::bones::read_ground_pose` |
| 603 | 1 | `melee_ft::desc::bones::EcbBones::read` |

### melee-sim: 101 labels, 128 emitted definitions (informational); 7 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 478 | 7 | `melee_ft::fighter::character::CharacterState::new` |
| 282 | 1 | `melee_ft::anim::playback::FighterAnimation::advance_main` |
| 252 | 1 | `melee_ft::anim::root_motion::RootMotion::evaluate` |
| 229 | 1 | `melee_ft::anim::blend::blend_rotation` |
| 205 | 1 | `core::ptr::drop_in_place<melee_ft::fighter::assets::FighterAssets>` |
| 164 | 1 | `melee_ft::anim::playback::FighterAnimation::advance_parts` |
| 130 | 1 | `melee_ft::anim::blend::blend_pose` |
| 110 | 1 | `melee_ft::anim::playback::animate_parts` |
| 109 | 1 | `core::ptr::drop_in_place<melee_ft::fighter::FighterCore>` |
| 72 | 1 | `<melee_ft::fighter::state::FighterProc as core::fmt::Debug>::fmt` |
| 56 | 1 | `<melee_ft::input::pad::PadSample as core::cmp::PartialEq>::eq` |
| 54 | 1 | `<melee_ft::desc::animation::AnimationDescError as core::fmt::Debug>::fmt` |
| 48 | 1 | `core::ptr::drop_in_place<melee_ft::desc::bones::FighterBones>` |
| 41 | 1 | `core::ptr::drop_in_place<[core::option::Option<melee_ft::desc::bones::AnimationBoneSet>; 5]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::desc::animation::AnimationEntry]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::dynamics::DynamicSetDescriptor]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::fighter::assets::FighterAssets; 2]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::fighter::assets::FighterAssets]>` |
| 38 | 1 | `core::ptr::drop_in_place<melee_ft::anim::playback::FighterAnimation>` |
| 37 | 1 | `core::ptr::drop_in_place<melee_ft::anim::attach::MotionRemap>` |

### ft-captain: 43 labels, 66 emitted definitions (informational); 1 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 83 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 65 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 34 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 7 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 5 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_shield` |

### ft-falco: 44 labels, 67 emitted definitions (informational); 1 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 57 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 55 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 35 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 9 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 3 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |

### ft-fox: 44 labels, 67 emitted definitions (informational); 1 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 57 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 55 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 35 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 9 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 3 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |

### ft-fox-family: 0 labels, 0 emitted definitions (informational); 0 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mario: 0 labels, 0 emitted definitions (informational); 0 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mars: 43 labels, 66 emitted definitions (informational); 1 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 67 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 42 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 34 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::guard_variant` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 5 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 3 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_landing` |
| 3 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |

### ft-peach: 44 labels, 67 emitted definitions (informational); 1 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 65 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 64 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::check_float_input` |
| 55 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 52 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 49 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_resources_loaded` |
| 34 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 15 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 10 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::dynamics_first_force_bone` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |

### ft-purin: 43 labels, 66 emitted definitions (informational); 1 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 61 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 34 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 4 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::multi_jump_attributes` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::multi_jump_animation` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_shield` |

### ft-yoshi: 45 labels, 68 emitted definitions (informational); 1 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 194 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 142 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 87 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_animated` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 60 | 1 | `melee_ft::fighter::escape::<impl melee_ft::fighter::FighterCore>::roll_input` |
| 54 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 49 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 41 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_entered` |
| 34 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 28 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 18 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::action_id` |
| 17 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::input_shield` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 11 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::check_hurtbox_interaction` |

<!-- perf-gate-v1
{"architecture": "concrete-shell-v1", "census": "duplicate-labels-v1", "date": "2026-09-10T10:28:34+00:00", "metrics": {"cross_crate_duplicate_labels": 99, "definitions": {"ft-captain": 66, "ft-falco": 67, "ft-fox": 67, "ft-fox-family": 0, "ft-mario": 0, "ft-mars": 66, "ft-peach": 67, "ft-purin": 66, "ft-yoshi": 68, "melee-ft": 852, "melee-sim": 128}, "duplicate_labels": {"ft-captain": 1, "ft-falco": 1, "ft-fox": 1, "ft-fox-family": 0, "ft-mario": 0, "ft-mars": 1, "ft-peach": 1, "ft-purin": 1, "ft-yoshi": 1, "melee-ft": 20, "melee-sim": 7}, "labels": {"ft-captain": 43, "ft-falco": 44, "ft-fox": 44, "ft-fox-family": 0, "ft-mario": 0, "ft-mars": 43, "ft-peach": 44, "ft-purin": 43, "ft-yoshi": 45, "melee-ft": 817, "melee-sim": 101}, "load_ns": 416349633.25, "stripped_bytes": 3518384, "text_bytes": 3211264, "ticks_600_ns": 76364881.2}, "platform": "macOS-26.2-arm64-arm-64bit-Mach-O", "revision": "77c28e8b230eec991f01e7d350ede14d3658241f", "rustc": "rustc 1.96.0 (ac68faa20 2026-05-25)", "status": "REGRESSION"}
-->

## 2026-09-10T10:59:52+00:00 — COMPLETE — PASS

Evidence: `/Users/nikhilunni/Projects/melee/target/perf/20260910T105920Z-35455`. Revision `122fec531b05b8564ce76fcb5caa3af0d2cb51a6` (working tree included).

rustc 1.96.0 (ac68faa20 2026-05-25); macOS-26.2-arm64-arm-64bit-Mach-O.

Tolerance: time +10%, size +5%, duplicate labels +0 within each compiling crate and across crates.
Baseline: 2026-09-10T09:01:39+00:00.
C15 fixed ceilings: 3,747,632 stripped bytes; C15 duplicate-label baseline; one definition per common label.

| Measurement | Value |
|---|---:|
| Stripped binary | 3,518,384 bytes |
| Text (`size`) | 3,211,264 bytes |
| load mean | 168.606 ms (95% CI 168.311..168.930 ms) |
| ticks_600 mean | 23.881 ms (95% CI 23.751..24.007 ms) |
| Headless throughput | 25,125 ticks/s |
| Total emitted definitions (informational) | 1,447 |
| Labels defined in multiple crates | 99 |
| Common definition labels audited across crates | 275 |

Tool: `0.12.1`.

Tool: `cargo-llvm-lines 0.4.48`.

Per-crate text contribution (cargo-bloat estimates):
```text
File  .text     Size Crate
12.7%  25.6% 615.2KiB melee_sim
 9.7%  19.5% 467.9KiB std
 7.1%  14.2% 341.3KiB melee_ft
 5.5%  11.1% 266.1KiB clap_builder
 1.7%   3.4%  82.5KiB melee_ef
 1.4%   2.8%  67.6KiB slp
 1.3%   2.7%  64.3KiB melee_mp
 1.2%   2.5%  59.3KiB melee_gr
 1.1%   2.1%  51.5KiB hsd_anim
 1.0%   2.1%  50.7KiB hsd_archive
 0.9%   1.7%  41.5KiB toml
 0.8%   1.7%  41.1KiB serde_json
 0.8%   1.5%  36.9KiB hsd_particle
 0.8%   1.5%  36.5KiB toml_parser
 0.4%   0.7%  17.3KiB melee_lb
 0.4%   0.7%  16.9KiB serde_core
 0.3%   0.6%  15.5KiB melee_diff
 0.3%   0.6%  14.6KiB ft_yoshi
 0.3%   0.6%  13.5KiB hsd_gobj
 0.2%   0.5%  11.3KiB melee_it
 0.2%   0.5%  11.3KiB melee_types
 0.2%   0.4%   8.6KiB ft_fox_family
 0.2%   0.4%   8.6KiB ft_falco
 0.2%   0.4%   8.5KiB ft_fox
 0.1%   0.3%   6.9KiB anyhow
 0.1%   0.2%   5.1KiB toml_datetime
 0.1%   0.2%   4.1KiB melee_cmd
 0.1%   0.1%   3.4KiB ft_peach
 0.1%   0.1%   3.2KiB it_foxlaser
 0.1%   0.1%   3.1KiB [Unknown]
 0.1%   0.1%   3.1KiB ft_purin
 0.1%   0.1%   2.8KiB clap_lex
 0.1%   0.1%   2.7KiB gekko_math
 0.1%   0.1%   2.5KiB melee_coll
 0.0%   0.1%   2.3KiB ft_mars
 0.0%   0.1%   2.2KiB ft_captain
 0.0%   0.1%   2.1KiB serde
 0.0%   0.1%   1.9KiB melee_if
 0.0%   0.1%   1.7KiB anstream
 0.0%   0.1%   1.6KiB anstyle
 0.0%   0.1%   1.4KiB strsim
 0.0%   0.0%   1.0KiB zmij
 0.0%   0.0%     960B itoa
 0.0%   0.0%     848B toml_writer
 0.0%   0.0%     376B it_foxillusion
 0.0%   0.0%     312B winnow
 0.0%   0.0%     264B hsd_types
 0.0%   0.0%      20B __rustc
 0.0%   0.0%      16B colorchoice
49.6% 100.0%   2.3MiB .text section size, the file size is 4.7MiB

Note: numbers above are a result of guesswork. They are not 100% correct and never will be.
```

### melee-ft: 817 labels, 852 emitted definitions (informational); 20 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 21980 | 1 | `melee_ft::fighter::assets::FighterAssets::load` |
| 3187 | 1 | `melee_ft::fighter::damage::DamageParameters::read` |
| 2493 | 1 | `melee_ft::fighter::state::common_table::common_table` |
| 1682 | 1 | `melee_ft::fighter::shield::ShieldParameters::read` |
| 1374 | 1 | `melee_ft::desc::common::CommonFighterData::read` |
| 1343 | 1 | `melee_ft::input::common::InputCommonData::read` |
| 1231 | 1 | `melee_ft::dynamics::read_sets` |
| 1101 | 1 | `melee_ft::desc::playback::read_playback_motion` |
| 1064 | 1 | `melee_ft::fighter::spawn::<impl melee_ft::fighter::FighterCore>::prepare` |
| 1042 | 1 | `melee_ft::desc::bones::read_fighter_bones` |
| 956 | 1 | `melee_ft::fighter::commands::CommandState::step_inner` |
| 929 | 1 | `melee_ft::desc::fox_attributes::FireFoxAttributes::read` |
| 885 | 1 | `melee_ft::desc::attributes::FighterAttributes::read` |
| 844 | 1 | `melee_ft::dynamics::read_motion_starts` |
| 760 | 1 | `melee_ft::anim::attach::select_motion` |
| 695 | 1 | `melee_ft::fighter::smash::read_overlay` |
| 682 | 1 | `melee_ft::anim::playback::FighterAnimation::resume_dynamic_subtree` |
| 659 | 1 | `melee_ft::desc::fox_attributes::IllusionAttributes::read` |
| 640 | 1 | `melee_ft::desc::bones::read_ground_pose` |
| 603 | 1 | `melee_ft::desc::bones::EcbBones::read` |

### melee-sim: 101 labels, 128 emitted definitions (informational); 7 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 478 | 7 | `melee_ft::fighter::character::CharacterState::new` |
| 282 | 1 | `melee_ft::anim::playback::FighterAnimation::advance_main` |
| 252 | 1 | `melee_ft::anim::root_motion::RootMotion::evaluate` |
| 229 | 1 | `melee_ft::anim::blend::blend_rotation` |
| 205 | 1 | `core::ptr::drop_in_place<melee_ft::fighter::assets::FighterAssets>` |
| 164 | 1 | `melee_ft::anim::playback::FighterAnimation::advance_parts` |
| 130 | 1 | `melee_ft::anim::blend::blend_pose` |
| 110 | 1 | `melee_ft::anim::playback::animate_parts` |
| 109 | 1 | `core::ptr::drop_in_place<melee_ft::fighter::FighterCore>` |
| 72 | 1 | `<melee_ft::fighter::state::FighterProc as core::fmt::Debug>::fmt` |
| 56 | 1 | `<melee_ft::input::pad::PadSample as core::cmp::PartialEq>::eq` |
| 54 | 1 | `<melee_ft::desc::animation::AnimationDescError as core::fmt::Debug>::fmt` |
| 48 | 1 | `core::ptr::drop_in_place<melee_ft::desc::bones::FighterBones>` |
| 41 | 1 | `core::ptr::drop_in_place<[core::option::Option<melee_ft::desc::bones::AnimationBoneSet>; 5]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::desc::animation::AnimationEntry]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::dynamics::DynamicSetDescriptor]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::fighter::assets::FighterAssets; 2]>` |
| 41 | 1 | `core::ptr::drop_in_place<[melee_ft::fighter::assets::FighterAssets]>` |
| 38 | 1 | `core::ptr::drop_in_place<melee_ft::anim::playback::FighterAnimation>` |
| 37 | 1 | `core::ptr::drop_in_place<melee_ft::anim::attach::MotionRemap>` |

### ft-captain: 43 labels, 66 emitted definitions (informational); 1 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 83 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 65 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 34 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 7 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 5 | 1 | `<ft_captain::init::CaptainFalcon as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_shield` |

### ft-falco: 44 labels, 67 emitted definitions (informational); 1 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 57 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 55 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 35 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 9 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 3 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_falco::init::Falco as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |

### ft-fox: 44 labels, 67 emitted definitions (informational); 1 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 57 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 55 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 35 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 29 | 1 | `<melee_ft::fighter::state::action::SpecialSlot as core::fmt::Debug>::fmt` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 9 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_muzzle` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 3 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::accessory` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::enter_special` |
| 2 | 1 | `<ft_fox::init::Fox as melee_ft::fighter::CharacterCallbacks>::item_owner` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |

### ft-fox-family: 0 labels, 0 emitted definitions (informational); 0 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mario: 0 labels, 0 emitted definitions (informational); 0 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|

### ft-mars: 43 labels, 66 emitted definitions (informational); 1 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 67 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 42 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 34 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::guard_variant` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 5 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |
| 3 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_landing` |
| 3 | 1 | `<ft_mars::init::Marth as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |

### ft-peach: 44 labels, 67 emitted definitions (informational); 1 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 65 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 64 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::check_float_input` |
| 55 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 54 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 52 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 49 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_resources_loaded` |
| 34 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 15 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 10 | 1 | `<ft_peach::init::Peach as melee_ft::fighter::CharacterCallbacks>::dynamics_first_force_bone` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::multi_jump_animation` |
| 5 | 1 | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` |

### ft-purin: 43 labels, 66 emitted definitions (informational); 1 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 61 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 34 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 19 | 1 | `melee_ft::fighter::CharacterCallbacks::escape_variant` |
| 13 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 4 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::multi_jump_attributes` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::multi_jump_animation` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 3 | 1 | `<ft_purin::init::Jigglypuff as melee_ft::fighter::CharacterCallbacks>::on_reset` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::animate_shield` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` |
| 2 | 1 | `melee_ft::fighter::CharacterCallbacks::enter_shield` |

### ft-yoshi: 45 labels, 68 emitted definitions (informational); 1 duplicate labels

| IR lines | Copies | Function (top 20 by IR lines) |
|---:|---:|---|
| 194 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::from_archive` |
| 142 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_costume_loaded` |
| 87 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_animated` |
| 76 | 24 | `melee_ft::fighter::character::CharacterTable::new::{{closure}}` |
| 60 | 1 | `melee_ft::fighter::escape::<impl melee_ft::fighter::FighterCore>::roll_input` |
| 54 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved::{{closure}}` |
| 49 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::on_load` |
| 41 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::aerial_jump_entered` |
| 34 | 1 | `melee_ft::fighter::CharacterCallbacks::item_owner` |
| 28 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::restore_saved` |
| 22 | 1 | `melee_ft::fighter::character::CharacterState::get` |
| 21 | 1 | `melee_ft::fighter::character::CharacterState::get_mut` |
| 18 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::action_id` |
| 17 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::input_shield` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::jab_variant` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::on_landing` |
| 13 | 1 | `melee_ft::fighter::CharacterCallbacks::throw_variant` |
| 12 | 1 | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` |
| 11 | 1 | `<ft_yoshi::init::Yoshi as melee_ft::fighter::CharacterCallbacks>::check_hurtbox_interaction` |

<!-- perf-gate-v1
{"architecture": "concrete-shell-v1", "census": "duplicate-labels-v1", "date": "2026-09-10T10:59:52+00:00", "metrics": {"cross_crate_duplicate_labels": 99, "definitions": {"ft-captain": 66, "ft-falco": 67, "ft-fox": 67, "ft-fox-family": 0, "ft-mario": 0, "ft-mars": 66, "ft-peach": 67, "ft-purin": 66, "ft-yoshi": 68, "melee-ft": 852, "melee-sim": 128}, "duplicate_labels": {"ft-captain": 1, "ft-falco": 1, "ft-fox": 1, "ft-fox-family": 0, "ft-mario": 0, "ft-mars": 1, "ft-peach": 1, "ft-purin": 1, "ft-yoshi": 1, "melee-ft": 20, "melee-sim": 7}, "labels": {"ft-captain": 43, "ft-falco": 44, "ft-fox": 44, "ft-fox-family": 0, "ft-mario": 0, "ft-mars": 43, "ft-peach": 44, "ft-purin": 43, "ft-yoshi": 45, "melee-ft": 817, "melee-sim": 101}, "load_ns": 168606196.3245238, "stripped_bytes": 3518384, "text_bytes": 3211264, "ticks_600_ns": 23880843.000000004}, "platform": "macOS-26.2-arm64-arm-64bit-Mach-O", "revision": "122fec531b05b8564ce76fcb5caa3af0d2cb51a6", "rustc": "rustc 1.96.0 (ac68faa20 2026-05-25)", "status": "PASS"}
-->
