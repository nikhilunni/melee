# P1: retained revival owners, Yoshi codegen and performance baseline

2026-09-10, chars lane, uncommitted. Base `cd6cfada40bc577865ae540a34aa34faf12d28f3`.
Main advanced to `7020449dac96982dead483cc8e6aa349367d0096` during this task; its
only additions relative to this base are TRACKER text and 21 scenario files.
Production code at the base therefore matches that main revision. Measurements
include P1's working-tree changes. No Git write commands, commits, game-data
writes, scenario edits, decomp edits, protected attack/damage/item/Fox edits,
or m5 gate-list edits were made. The pre-existing decomp symlink is unchanged.

The requested `C6_PERF_GATES.md` does not exist in this tree. Its implementation
report is `C5_C6_C10_PERF_LANE.md`; that report, the C2/C5-b/C8 notes,
CLAUDE, STEEL_THREAD, TRACKER, PERF and the actual perf scripts were inspected.

## Revival ownership and behavior

The old scheduler path called `archive.model(costume)`, decoding a new JObj tree.
`reset_for_revival` then called `C::from_archive` and `Fighter::spawn`, which
allocated a replacement character, skeleton owners, blend/rest trees, animation
parts, per-joint track buffers, dynamics and collision vectors. Finally,
`enter_revival` cloned the animated platform from assets and replaced the whole
fighter. C8 measured a 668-allocation spike, 690 allocations across 479 ticks.

The scheduler now calls the existing fighter directly with assets, arena and
spawn services. `reset_life` resets its live state; `initialize_spawn_geometry`
shares the existing support probe, dynamics selection, scale and placement work
with initial spawning. The small generic `initialize_spawn` preserves the death
hook, thrown-capsule initialization, two CPU RNG draws and ordinary motion entry
in their original order. The action then enters Rebirth as before.

| Owner | Lifetime / reset now |
|---|---|
| Scene's Fighter and typed character payload | Same owner. Archive/costume parsing, on_load and on_resources_loaded run at construction. Revival calls on_reset on the existing character, preserving loaded attributes, item registrations and costume resources. |
| Primary JObj tree | Same node arena and DObj owners. Existing joint animation tracks are returned to their reserved buffers; pose/cache values reset from the already-owned rest pose. |
| FighterAnimation | Same parts vector, blend tree, rest tree and track buffers. Clocks, part-animation scratch, masks and root-translation histories reset in place. |
| Dynamic sets and first-bone vector | Same sets, nested spring storage and vector capacities. Rest parameters stay loaded; existing selection resets enabled joints' position/angular velocity and lock state. |
| Attributes, bone maps and capabilities | Retained loaded owners; no resource import during revival. |
| Hurt capsules and dynamic colliders | Existing slices receive asset defaults with clone_from_slice; no Vec replacement. Thrown capsule resets as a scalar owner and is sampled at the existing callback boundary. |
| Physics, input, ECB, combat, shield, status, commands, effect requests, camera and mirrors | Reset as inline state. Fixed command/effect storage stays inline. Player identity, costume, control and remaining stock count survive. Spawn number advances through the existing counter. |
| Revival platform | Its animated tree is cloned once during FighterCore preparation, including savestate construction. An activity flag gates accessory ticks. Revival requests frame zero on retained tracks, sets the original scale/translation, and activates the same tree. |

Retail references: `ftlib.c:ftLib_80087140`,
`fighter.c:Fighter_UnkInitReset_80067C98` / `Fighter_UnkProcessDeath_80068354`,
`ft_0D31.c` death dispatch and `ft_0D4D.c:ftCo_800D4FF4`, Rebirth/RebirthWait
callbacks. The pinned source groups the Dead/Rebirth functions in these files,
rather than separate ftCo_Dead*.c / ftCo_Rebirth*.c files.

One premise needs qualification: retail clearly resets the *same Fighter*, but
this decomp's `ftCo_800D4FF4` calls `ftCommon_SetAccessory`, and
`ftcommon.c:977-994` calls `HSD_JObjLoadJoint` / `HSD_JObjAddAnimAll` on accessory
attachment. Retaining the platform for the entire fighter lifetime is this port's
allocation-free ownership choice, not evidence that retail allocates it only
once across all stocks. Animation restart and visible/gated behavior are retained.
No new float formula, reassociation, fusion or expected word was introduced.

## Allocation census and remaining backtraces

One warm-up tick, then every remaining recorded tick, unchanged from C8. Both
allocation and reallocation are counted. Snapshot allocation stays diagnostic.

| Scene | Measured ticks | Before | New measured ceiling | Peak / allocating ticks |
|---|---:|---:|---:|---:|
| start_fd_fox | 599 | 17 | 17 | 5 / 9 |
| idle_fd_fox | 599 | 1 | 1 | 1 / 1 |
| jab_fd_marth | 299 | 12 | 12 | 2 / 10 |
| ko_fd_marth | 479 | 690 | **22** | 6 / 13 |
| start_bf_fox | 599 | 17 | 17 | 6 / 11 |

All 668 allocations in the former revival spike are gone. The KO scene improves
96.81%; the whole-scene single-digit target is **not reached**. Remaining calls
all happen by zero-based tick 214, before revival. They are not hidden by warming
up through revival or excluding any measured tick. Owned snapshots still add
exactly 125 allocations per tick in every scene.

`MELEE_ALLOC_BACKTRACES=1 cargo test -p melee-sim --test alloc_gate ko_fd_marth -- --nocapture`
prints each remaining stack with its tick. Backtrace collection temporarily
turns off counting of its own diagnostic allocations; the final census also ran
without this option and measured the same 22. Full stacks are preserved in
`target/p1-evidence/alloc-backtraces.log`.

| Allocations | Zero-based ticks | Captured allocating stack, outer to inner |
|---:|---|---|
| 9 | 43; 209 x3; 210 x3; 211; 214 | ParticleSystem::update_generators → emit (`hsd-particle/src/system.rs:474`) → Vec::insert → RawVec growth |
| 9 | 47, 55, 63, 70, 79, 86, 89, 124, 209 | Effects::flush / Effect::animate → spawn_particle → ParticleSystem::spawn (`system.rs:157`) → Option::map → Arc::new → allocation of AppSRT |
| 2 | 47, 209 | ParticleSystem::spawn → insert_generator (`system.rs:117`) → Vec::insert → RawVec growth |
| 1 | 55 | Generator::disc → DrawLog::draw (`rng_sites.rs:47`) → Vec::push → RawVec growth |
| 1 | 209 | Runtime::dispatch → StockDisplay::tick (`melee-if/src/lib.rs:103`) → Vec::push → RawVec growth |

These require particle/AppSRT/diagnostic-storage provisioning work separate from
retaining fighter owners. No guessed particle pool size or changed exhaustion
semantics was added to meet a numeric target.

A new behavior test repeats revival three times each for Fox and Marth. Each
reset plus platform tick allocates **zero**, preserves eight owner addresses,
keeps the stock count, increments the spawn number, consumes exactly the two
known CPU draws and reproduces both the fighter snapshot and restarted platform
state. It dirties physics, timers and accessory animation between resets.
This covers repeat reuse beyond the single revival in the KO oracle. It passes
in both profiles as part of the workspace gates.

## Why Yoshi instantiated the common graph

Yoshi's non-generic shield overrides call generic shell methods:
`shield::{enter,hold,off,animate,escape_finished}` reach
`Fighter<Yoshi>::change_motion_state`; `shield::input` also calls `enter_escape`.
The original `Fighter<C>::row` indexed `C::COMMON` at runtime and called
`C::special_rows`. Materializing that associated common table in ft-yoshi takes
the addresses of all common callbacks. Those callbacks recursively reach the
rest of the common transition graph, including attacks, damage, jumps, ledges,
walking and revival. The five egg rows also reused common callbacks. The
archive/init readers themselves are not responsible for hundreds of templates.

Moving only leaf arithmetic to FighterCore would not sever this table-reference
edge: shield transitions really do need hooks and row installation. The contained
fix binds immutable common and special table references once in Fighter::prepare.
Runtime row lookup reads those references. `CharacterCallbacks::SPECIAL_ROWS`
provides a const-backed default while preserving the existing special_rows API;
Yoshi uses that const instead of an out-of-line function materializing the rows.
The entire common graph is therefore instantiated at scene construction in
melee-sim, while Yoshi retains only the few shell methods its overrides use.
This adds one array reference and one slice reference (24 bytes on this 64-bit
host) per shell, with no heap owner or added fn-pointer invocation during lookup.
Existing installed-row dispatch and per-character action-ID mapping are preserved.

The original code was independently built from a read-only `git archive HEAD`
export under `/tmp/p1-before`, without assets. Each library was compiled with
`cargo rustc -p <crate> --release --lib -- --emit=llvm-ir -C no-prepopulate-passes`.
Demangling concrete symbols with rustc-demangle 0.1.28 identifies **189 identical
namespace-owned Yoshi specializations defined in both compiling libraries**.
These include `Fighter<ft_yoshi::init::Yoshi>::enter_shield`, `ledge_input`,
`shield_input` and `change_motion_state_with_options`. Their LLVM definitions
have internal linkage. This proves duplicate codegen rather than inferring
Yoshi's presence by dividing a seven-copy aggregate. It does not claim all IR
survives optimization or occupies duplicate final machine-code bytes.

Raw IR and the 189 demangled names are in
`target/p1-evidence/{before-ft_yoshi.ll,before-melee_sim.ll,duplicate-yoshi-symbols.txt}`.
The ordinary llvm-lines census uses its own grouping/mangling options and the
unchanged PERF substring rule; diagnostic v0 symbols are only the type-identity
cross-check, not a substitute count.

| Compiling crate | Before copies / IR lines | After copies / IR lines |
|---|---:|---:|
| ft-yoshi | 228 / 14,783 | **22 / 1,105** |
| melee-sim | 2,017 / 137,843 | 2,015 / 135,809 |
| melee-ft | 532 / 77,080 | 535 / 77,989 |
| All other character crates | 19 copies | 19 copies |
| Total copies | 2,796 | **2,591** |

The three-copy core increase is four new definitions (animation reset, life
reset, shared spawn geometry and its one concrete probe closure), minus the
obsolete Option<RevivalPlatform> drop glue. It is shared code, not per-character
multiplication. Sim loses two net copies. Yoshi loses 206 copies / 92.53% of its
charged IR. Its remaining 22 include 16 character trait implementations, one restore closure, three
shell methods, one concrete roll-input helper and the default on_grounded_motion
implementation; the full per-function files are retained with the evidence.

Before/after for the original top 20 labels:
| Function label (common prefix shortened) | Before copies / IR lines | After copies / IR lines |
|---|---:|---:|
| `shield::enter_shield` | 1 / 350 | 0 / 0 |
| `ledge::ledge_input` | 1 / 346 | 0 / 0 |
| `shield::shield_input` | 1 / 330 | 0 / 0 |
| `walk::walk_input` | 1 / 314 | 0 / 0 |
| `turn::turn_input` | 1 / 313 | 0 / 0 |
| `ledge::ledge_animation` | 1 / 302 | 0 / 0 |
| `squat::squat_input` | 1 / 301 | 0 / 0 |
| `ledge::enter_cliff_catch` | 1 / 252 | 0 / 0 |
| `ledge::ledge_collision` | 1 / 238 | 0 / 0 |
| `multi_jump::enter_multi_jump` | 1 / 236 | 0 / 0 |
| `damage::damage_collision` | 1 / 231 | 0 / 0 |
| `dash::dash_input` | 1 / 228 | 0 / 0 |
| `state::callbacks::collision::fall_collision` | 1 / 215 | 0 / 0 |
| `down::enter_down_bound` | 1 / 212 | 0 / 0 |
| `jump::knee_bend_animation` | 1 / 209 | 0 / 0 |
| `attack::enter_ground_attack` | 1 / 207 | 0 / 0 |
| `jump::enter_aerial_jump` | 1 / 205 | 0 / 0 |
| `<ft_yoshi::init::Yoshi as CharacterCallbacks>::from_archive` | 1 / 194 | 1 / 194 |
| `shield::shield_animation` | 1 / 193 | 0 / 0 |
| `damage::damage_input` | 1 / 186 | 0 / 0 |

## COMPLETE baseline and thresholds

Final evidence: `target/perf/20260910T061523Z-90501`;
`docs/PERF.md` COMPLETE PASS `2026-09-10T06:15:55+00:00`.
rustc 1.96.0 (ac68faa20 2026-05-25), macOS 26.2 arm64;
cargo-llvm-lines 0.4.48 and cargo-bloat 0.12.1.

| Metric | Previous COMPLETE PASS | P1 final COMPLETE PASS |
|---|---:|---:|
| Stripped binary | 3,864,176 bytes | 3,747,632 bytes (-3.02%) |
| Text | 3,424,256 bytes | 3,342,336 bytes (-2.39%) |
| Load mean | 169.320 ms | 166.000 ms (95% CI 165.645..166.411) |
| 600 simulate-only ticks | 23.606 ms | 23.588 ms (95% CI 23.259..23.978) |
| Throughput | 25,417 ticks/s | 25,436 ticks/s |

Tick throughput is effectively unchanged within measurement noise. The final
baseline yields 3,935,013 stripped bytes, 3,509,452 text bytes, 182.600 ms load,
25.947 ms per 600 ticks (at least 23,123.82 ticks/s), and the exact per-crate copy
limits above for the next default run. The default time +10% and size +5%
tolerances were not changed. The KO allocation limit tightens to 22; all other
allocation ceilings retain their measured values.

The first run was INCOMPLETE because cargo-bloat's full metadata call tried to
unpack cached crunchy sources into the sandbox's read-only global Cargo registry.
Complete runs use a writable temporary Cargo home overlay and offline mode.
A subsequent complete run failed timing at 28.325 ms (interval 25.740..31.138);
that REGRESSION remains in PERF history. Process inspection was sandbox-blocked,
so interference was not positively attributed to another process. Two subsequent
full runs passed with 23.397 and 23.588 ms; no timings were fabricated or bounds
widened to pass the failed sample.

The requested baseline promotion deliberately accounts for melee-ft's +3 concrete
copies using documented PERF_COPIES_TOLERANCE=3 on the successful calibration.
The final separate script run uses the default +0 copy tolerance and PASSes.
Only this explicit architectural accounting changed; no oracle assertions or
existing expected values were weakened. PERF's latest COMPLETE PASS automatically
ratchets size, timings and each compiling crate to the new measured baseline.

## Validation and exact final lines

The initial unmodified `cargo gate` passed 977 tests, failed 0, ignored 3.
Both final full workspace profiles pass **978**, fail **0**, ignore the same
**3** existing tests (sums of every test-result line). The repeat-revival test
accounts for the additional pass. The pre-existing ignores remain untouched.
The KO acceptance includes all 49 keys and ordered particle RNG draws for 480
frames; M4/M5 gate lists and expected records were not edited.

Commands and raw logs are preserved in `target/p1-evidence/`. Selected exact
command-ending/result lines follow; the final workspace result belongs to the
last doc-test suite, so the full workspace tallies above are provided separately.

`cargo test -p melee-sim --test alloc_gate -- --nocapture` (exit 0):

```text
start_fd_fox: 599 measured ticks; simulate-only 17 (0.028381/tick), peak 5, allocating ticks 9; with snapshot 74892 (125.028381/tick), peak 130; snapshot overhead 74875 (125.000000/tick)
start_bf_fox: 599 measured ticks; simulate-only 17 (0.028381/tick), peak 6, allocating ticks 11; with snapshot 74892 (125.028381/tick), peak 131; snapshot overhead 74875 (125.000000/tick)
idle_fd_fox: 599 measured ticks; simulate-only 1 (0.001669/tick), peak 1, allocating ticks 1; with snapshot 74876 (125.001669/tick), peak 126; snapshot overhead 74875 (125.000000/tick)
jab_fd_marth: 299 measured ticks; simulate-only 12 (0.040134/tick), peak 2, allocating ticks 10; with snapshot 37387 (125.040134/tick), peak 127; snapshot overhead 37375 (125.000000/tick)
ko_fd_marth: 479 measured ticks; simulate-only 22 (0.045929/tick), peak 6, allocating ticks 13; with snapshot 59897 (125.045929/tick), peak 131; snapshot overhead 59875 (125.000000/tick)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.93s
```

`cargo gate` (exit 0):

```text
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`cargo gate --release` (exit 0):

```text
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`cargo test -p melee-sim --test m4_gate` (exit 0):

```text
test result: ok. 261 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 104.72s
```

`cargo test -p melee-sim --test m4_gate --release` (exit 0):

```text
test result: ok. 261 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.14s
```

`cargo test -p melee-sim --test m5_gate` (exit 0):

```text
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.47s
```

`cargo test -p melee-sim --test m5_gate --release` (exit 0):

```text
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.73s
```

`tools/check-release-math.sh` (exit 0):

```text
Fused math: opt-level=0
Fused math: opt-level=1
Fused math: opt-level=2
Fused math: opt-level=3
Fused math: opt-level=s
Fused math: opt-level=z
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`cargo clippy --workspace --all-targets -- -D warnings` (exit 0):

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 50.52s
```

`cargo fmt --all -- --check`: exit 0, no output.

`CARGO_HOME=/tmp/p1-cargo-home CARGO_NET_OFFLINE=true tools/perf-gate.sh`
(default performance tolerances, exit 0):

```text
[PASS] perf-gate: 3747632 stripped bytes, 3342336 text bytes; load 166.000 ms; ticks_600 23.588 ms
```

## Files changed and limits

Production:

- `crates/melee-ft/src/fighter/{life.rs,spawn.rs,mod.rs,state/row.rs}`
- `crates/melee-ft/src/anim/playback.rs`
- `crates/melee-sim/src/frame.rs`
- `crates/ft-yoshi/src/init.rs`

Tests:

- `crates/melee-sim/tests/alloc_gate.rs`
- `crates/melee-sim/tests/alloc_gate/revival.rs`

Documentation:

- `docs/PERF.md`
- `docs/PORT_NOTES/P1_REVIVAL_YOSHI_BASELINE.md`
- `docs/PORT_NOTES/C15_DESIGN_CONCRETE_SHELL.md`
- `TRACKER.md`

No acceptance command remains blocked. The whole KO scene still has the 22
reported non-revival allocations; zero allocations for the whole scene is not
claimed. No new oracle was recorded, no unported death variant/platform timeout
was implemented, and C15 remains a design task to schedule after combat merges.
The final main merge must rerun its gates if sibling production changes arrive.
