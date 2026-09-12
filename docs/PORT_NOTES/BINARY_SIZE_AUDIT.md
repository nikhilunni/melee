# Binary-size audit, 2026-09-11

The saved measurements show growth across gameplay and rendering work as well
as the library extraction. They do not isolate the extraction's cost. The
size debt remains open; no ceiling or baseline was relaxed, and no size
optimization is claimed by this audit.

## Verified saved artifacts

These are the sizes of the retained `melee-sim.stripped` files, measured again
from disk. The source trees may have contained uncommitted changes; a recorded
revision alone does not reconstruct each measured tree.

| Performance run under `target/perf/` | Stripped bytes | Change |
| --- | ---: | ---: |
| `20260910T105920Z-35455` | 3,518,384 | Earlier combat baseline |
| `20260911T023813Z-10015` | 3,757,952 | +239,568 |
| `20260912T022555Z-78051` | 3,926,592 | +168,640 |

The first interval includes increased authored gameplay: the saved
`bloat.txt` attribution for `melee_ft` grows from 341.3 to 399.5 KiB, Fox and
Falco each grow from about 8.5 to 15.3 KiB, and Marth from 2.3 to 12.0 KiB.
Combined `melee_sim` plus `melee_lib` attribution grows from 615.2 to 691.6 KiB.
This is not an extraction-only A/B comparison.

In the second interval, `melee_ft` is approximately flat (399.5 to 399.0 KiB).
The largest attributed increases include `hsd_archive` (+29.9 KiB), `std`
(+23.8 KiB), `hsd_anim` (+19.9 KiB), `melee_lib` (+14.9 KiB) and `melee_gr`
(+12.4 KiB). These are approximate code-section attributions, not an exact
partition of total file growth or proof of duplicated code.

The last recorded performance run retains the existing duplicate-label
counts of 19 in `melee-ft`, 7 in `melee-lib`, and 100 across crates. Those
counts cover the selected gameplay labels, not all generic serialization and
collection code. Passing that census does not imply all code-size growth is
necessary. Full historical metrics and unchanged ceilings remain in
`docs/PERF.md`.

## Small measured experiment to perform next

`crates/melee-lib/src/initial_state/mod.rs` has the only two production
`serde_json::from_reader(File)` sites in the library/simulator: particle
boundary metadata and the saved-state sidecar. Other JSON paths already use
string/slice parsing. A saved symbol inventory confirms the release executable
contains both `IoRead<File>` and string/slice parser code.

Read those two small setup files into buffers and use the existing string or
slice parser. Preserve full-document validation, including trailing-data
rejection. This changes setup only and requires no per-tick allocation. It
may remove a second parser backend; its actual net saving must be measured
before claiming a result. It is not expected to repay the whole size debt.

For that experiment: save the current stripped executable; change only the
loader calls; rebuild and compare stripped/code sizes; run saved-boundary
oracles and the load benchmark without competing tests. Use separate target
directories for separate source trees, or a strictly sequential build, so
Cargo metadata and benchmark evidence cannot collide.

The saved per-symbol inventory also identifies `FighterAssets::load` as a
large cold function (82,208 bytes in that snapshot). Any proposal to outline
repeated decoding/error paths needs a measured implementation and load-time
check. Its size alone is not proof that it is the cause of recent inflation.
