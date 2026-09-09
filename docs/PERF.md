# Performance regression history

Run `tools/perf-gate.sh` separately from `cargo gate`, on an otherwise idle
machine. It builds the native release CLI, strips a copy, runs `size` and
`cargo bloat --release -p melee-sim --bin melee-sim --crates -n 0`, counts
melee-ft instantiations with `cargo llvm-lines -p melee-sim --release --lib`
and the equivalent command for every character crate, and runs Criterion.
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
The sim library instantiates shared scene dispatch; each `ft-*` crate is also
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
