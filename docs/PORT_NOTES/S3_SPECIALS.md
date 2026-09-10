# S3 specials: partial port and ownership boundary

S3 is **not complete**. The implementation covers Fox/Falco's six Illusion /
Phantasm rows and the ghost article. Aerial Illusion passes its full 300-tick
fighter/item oracle and particle replay. Grounded Illusion matches ticks 0–124,
including the target reaction, then reaches the existing common ground-pose
`LegCorrection` stop at tick 125. Fire Fox, Reflector and Marth's specials remain
unimplemented. No acceptance claim is made for those nine scenes.

## Scene status

Tick numbers below are zero-based. State paths and target reactions come from
the recorded scenes; only the explicitly marked coverage is verified by this port.

| Scene | Recorded actor states | Target reaction | Current evidence / first boundary |
|---|---|---|---|
| illusion_fd_fox | 20 → 21 → 347 → 348 → 349 → 14 | 80 at 87 → 42 at 110 → 14 at 140 | Ticks 0–124 exact, including item keys and particle RNG sites; tick 125 common `LegCorrection` |
| airillusion_fd_fox | 24 → 25 → 350 → 351 → 352 → 43 → 14 | idle | Full 300 ticks, 62 keys; ordered particle draws and full particle replay |
| firefox_fd_fox | 353 → 356 → 358 → 35 → 43 → 14 | idle | Up-special entry unimplemented |
| airfirefox_fd_fox | 24 → 25 → 354 → 356 → 358 → 35 → 43 → 14 | idle | Up-special entry unimplemented |
| reflector_fd_fox | 360 → 361 → 363 → 14 | idle | Down-special entry unimplemented |
| reflectorjc_fd_fox | 360 → 361 → 24 → 25 → 42 | idle | Down-special entry unimplemented |
| airreflector_fd_fox | 24 → 25 → 365 → 366 → 363 → 14 | idle | Down-special entry unimplemented |
| shieldbreaker_fd_marth | 20 → 21 → 23 → 14 → 341 → 342 → 343 → 14 | 80 → 42 | Marth special entry unimplemented |
| dancingblade_fd_marth | 349 → 351 → 352 → 357 → 14 | miss | Marth special entry unimplemented |
| dolphinslash_fd_marth | 20 → 21 → 23 → 14 → 367 → 35 → 43 | 88 → 38 → 0 | Marth special entry unimplemented |
| counter_fd_marth | 369 → 370 → 14 | Fox 89 → 191 → 192 | Marth special entry unimplemented; 191/192 are `DownBoundD`/`DownWaitD` in melee-types |

The grounded prefix is an additional boundary test, not a replacement for a full
scene gate. None of the pre-existing tests or expected values were weakened.
The common pose dependency is outside the task's listed S3 file ownership;
`collision/pose.rs`, `damage.rs`, `down.rs`, attack/aerial rows and the normal
landing/fall implementation were not modified. The shared landing change only
exposes `enter_special_landing` to character callbacks.

## First-divergence history

1. Resource decoding reached fighter subaction opcode 37 (`ftAction_80071FA0`):
   the fighter visibility bit is now a typed command.
2. Falco's ghost article uses `ftData.items[3]`, whereas Fox uses `[2]`.
   `FoxFamily::GHOST_ARTICLE_INDEX` expresses this difference. Reading Falco row
   2 as the ghost was a resource-selection error, not an unsupported item opcode.
3. Grounded tick 63: side-special entry from Run lacked
   `ftCo_SpecialS`'s ground-speed retention. Retail 80096614/1C/24 uses subtraction,
   multiply and `fmadds`; applying it before the startup divisor removes the
   position mismatch (expected 8.1733446, previously 9.3466778).
4. Tick 82: synchronous trail creation ran before queued dust effects. Retail
   `Fighter_8006C80C` flushes efAsync before accessory4. Moving the existing
   character accessory hook to that boundary and introducing `SyncAttached`
   restores generator insertion and RNG order. The existing laser gate remains
   exact with this correction.
5. Tick 86: the trail must be destroyed on entry to the end motion.
   `efLib_DestroyAll` also walks the parent fighter's joints, not just owned
   effect models. The effect engine now expires those attachments too.
6. Tick 86: `Fighter_ChangeMotionState` clamps ground speed on leaving root
   motion (`fighter.c:1363–1368`). Without that branch, speed remained 4.0 rather
   than clamping to dash speed before friction; position was 68.720016 instead
   of 66.920013. The shared concrete transition now performs that clamp.
7. Ticks 0–124 then match, including the ghost hit and target DamageN3/Landing.
   Tick 125 stops in `FighterCore::proc_pose_with_map`, `ft_0899.c:109–232:
   LegCorrection`. At the last completed tick P0 is in 349 at x=85.665695;
   P1 is in Landing 42 at x=53.511879. A common pose implementation is needed
   before the remaining grounded ticks can be evaluated.

The dependency is `crates/melee-ft/src/collision/pose.rs`: its flat-floor adapter
rejects a missing foot probe or a nonzero correction. Retail `fn_8008998C`
(`ft_0899.c:18–77`) falls back to the floor endpoint, limits the correction slope,
then `ft_80089B08` invokes `lbBgFlash_80021410` when correction is required.
Those common pose/IK paths must be implemented before rerunning grounded tick
125; bypassing the error would not establish an exact port.

## Typed scratch and family split

`Fox` and `Falco` each own `SpecialSide` inside the C15 checked character payload.
Callbacks use `fighter.character.get::<C>()` / `get_mut::<C>()` and release the
borrow before transitions. The shared shell and all phase signatures still use
concrete `Fighter`. `rows::<C>()` assembles six existing laser rows and six new
side-special rows into static character tables; no common attack row was added.

| SpecialSide field | Retail Fighter offset | Meaning |
|---|---|---|
| gravity_delay | +0x2340 | Startup/end gravity countdown |
| ghost_positions | +0x2344 through +0x2370 | Four position samples, newest first |
| ghost_rotations | +0x2374 through +0x2380 | Four TopN rotation-X samples |
| ghost_present | +0x2384 | Logical presence of the requested ghost GObj |
| trail_pending | accessory4_cb | Pending one-shot `ftFx_SpecialS_CreateGFX` |

Numeric motion attributes come from the existing shared `FoxAttributes` reader.
The family trait supplies Fox kind 56 versus Falco kind 57 and their different
article indices. Falco's rows compile and its existing tests pass; no new Falco
oracle is claimed. Fire Fox, Reflector and Marth scratch layouts are not yet
implemented.

## Ghost item and effects

`it-foxillusion` implements the S4 `ItemLogic` trait for Fox Illusion and Falco
Phantasm. `melee-it` does not depend on the kind crate. The scene's existing
`item_kinds!` dispatch registers both kinds. The three static item rows select
grounded/aerial travelling scripts and the final trailing state. Lifetime and
owner removal follow `itfoxillusion.c`; physics follows owner history sample 1.
The primary item's motion, position, lifetime, hitbox state and ordered list
membership are covered by the S4 comparator's 13 additional keys. Kind 56 is
present for six ticks: 84–89 grounded, 66–71 aerial.

The secondary ghost **display-only** JObj is not evaluated, matching the existing
item engine's headless scope. No claim is made about its visual pose. Start/travel
support changes that require preserved ground/air motion flags remain explicit
unsupported branches. Ordinary end-state departure uses the common Fall path;
aerial end landing uses existing LandingFallSpecial, and ledge checks call the
existing concrete common helper.

Trail effect 0x48D creates Fox-bank generator 0xBC0 attached to TopN, with an
AppSRT facing rotation and retail flags `(type & ~0x600) | 0x800`. The new aerial
fixture contains only production effect-boundary inputs; particle outputs still
come from the retail capture. Three `ftCo_8009F834` fighter-side offset draws
(8009F930/954/978) are explicitly classified outside the particle interpreter.
The replay compares all existing simulation fields and ordered particle RNG
sites, retaining only the replay helper's pre-existing display-cache exclusions.

## Validation

Both full workspace gates passed: 1,021 passed, zero failed, three pre-existing
ignores in each profile. M4 remains 261; M5 is 30. The four new tests are two
scene/prefix tests, the aerial particle replay and its zero-allocation gate.
`MELEE_ALLOW_MISSING_DATA` was unset. These green workspace totals do not turn
the nine unimplemented scenes or grounded pose boundary into completed gates. The required
Fire Fox/Counter allocation cases and Fire Fox particle replay are not available
because those moves are not ported. The new aerial Illusion allocation gate has a
zero-allocation ceiling, with zero measured allocations across ticks 1–299.

### Release scene commands

Each command used `cargo run -q --release -p melee-sim -- gate
harness/scenarios/<scene>.toml`. Full failure output is reproduced below (the
process/thread wrapper and backtrace hint are omitted).

**illusion_fd_fox**, exit 101:

```text
not implemented: ft_0899.c:109-232: LegCorrection
```

**airillusion_fd_fox**, exit 0:

```text
300 ticks, 62 keys, 0 divergences
```

**firefox_fd_fox**, exit 101:

```text
not implemented: Fox family Up
```

**airfirefox_fd_fox**, exit 101:

```text
not implemented: Fox family Up
```

**reflector_fd_fox**, exit 101:

```text
not implemented: Fox family Down
```

**reflectorjc_fd_fox**, exit 101:

```text
not implemented: Fox family Down
```

**airreflector_fd_fox**, exit 101:

```text
not implemented: Fox family Down
```

**shieldbreaker_fd_marth**, exit 1:

```text
Error: first divergence at frame 119 phase frame_end
  field:    p0.cur_anim_frame
  expected: 1 (0x3F800000)
  actual:   6 (0x40C00000)
119 ticks matched; RNG-writing procs this tick: [("ParticlesMain", 2079789175)]
```

**dancingblade_fd_marth**, exit 1:

```text
Error: first divergence at frame 31 phase frame_end
  field:    p0.cur_anim_frame
  expected: 1 (0x3F800000)
  actual:   31 (0x41F80000)
31 ticks matched; RNG-writing procs this tick: [("ParticlesMain", 3810178733)]
```

**dolphinslash_fd_marth**, exit 1:

```text
Error: first divergence at frame 119 phase frame_end
  field:    p0.cur_anim_frame
  expected: 1 (0x3F800000)
  actual:   6 (0x40C00000)
119 ticks matched; RNG-writing procs this tick: [("ParticlesMain", 2079789175)]
```

**counter_fd_marth**, exit 1:

```text
Error: first divergence at frame 56 phase frame_end
  field:    p0.cur_anim_frame
  expected: 1 (0x3F800000)
  actual:   56 (0x42600000)
56 ticks matched; RNG-writing procs this tick: [("ParticlesMain", 4083522474)]
```

### Compiler, allocation and math checks

- `cargo gate`: 1,021 passed, 0 failed, 3 existing ignored; M4 261, M5 30.
- `cargo gate --release`: same counts, including all ft-falco tests.
- `cargo clippy --workspace --all-targets -- -D warnings`: exit 0.
- `cargo fmt --all --check`: exit 0.
- `tools/check-release-math.sh`: exit 0, native math in both profiles and
  all three fused tests at optimization levels 0, 1, 2, 3, s and z.
- `git diff --check`: exit 0.

```text
airillusion_fd_fox: 299 measured ticks; simulate-only 0 (0.000000/tick), peak 0, allocating ticks 0; with snapshot 37375 (125.000000/tick), peak 125; snapshot overhead 37375 (125.000000/tick)
```

```text
300 ticks, 62 keys, 0 divergences; fixture crates/hsd-particle/tests/data/airillusion_fd_fox_spawns.json
```

The fixture replay passed in both profiles. No no-data or test-stack override
was used. No commits, Git write commands or changes to protected harness/decomp
paths were made. The decomp symlink type difference preceded this task.
### Instantiation, size and timing

The first performance run found one compiler-generated cross-crate copy of
`melee_ft::physics::integrate::integrate_environment` in ft-fox-family. Its
explicit no-inline boundary now keeps that concrete body in melee-ft. This
changes compiler placement only; no arithmetic or gate ceiling was changed.
The next full perf gate passed. The full test/check chain was rerun after this
adjustment and the final visibility-bit correction; both workspace profiles,
clippy, formatting and release-math passed. The final performance refresh,
however, regressed on timing while retaining the same size and copy counts.

| Compiling crate | Final charged copies | Ceiling |
|---|---:|---:|
| ft-captain | 66 | 66 |
| ft-falco | 67 | 67 |
| ft-fox | 67 | 67 |
| ft-fox-family | 0 | 0 |
| ft-mario | 0 | 0 |
| ft-mars | 66 | 66 |
| ft-peach | 67 | 67 |
| ft-purin | 66 | 66 |
| ft-yoshi | 68 | 68 |
| melee-ft | 845 | 845 |
| melee-sim | 128 | 128 |
| **Total** | **1440** | **1,440** |

```text
[PASS] perf-gate: 3500832 stripped bytes, 3194880 text bytes; load 167.185 ms; ticks_600 23.614 ms
```

Final refresh, exit 1:

```text
[REGRESSION] perf-gate: 3500832 stripped bytes, 3194880 text bytes; load 214.695 ms; ticks_600 33.276 ms
```

The final run exceeded both the previous-run timing limits and P1's load
182.600 ms / 600-tick 25.947 ms ceilings. Immediately afterwards, `uptime`
reported load averages 24.22, 36.01 and 31.59. Process inspection was denied by
the sandbox. Host contention is a possible explanation, not an established
cause; the final performance gate is **not passing**. No threshold was changed.

The raw performance runs are retained in `docs/PERF.md`; the failed census was
not hidden or promoted into a higher budget.

## Changed files

- `CLAUDE.md`
- `Cargo.lock`
- `Cargo.toml`
- `TRACKER.md`
- `crates/ft-falco/src/init.rs`
- `crates/ft-fox-family/Cargo.toml`
- `crates/ft-fox-family/src/lib.rs`
- `crates/ft-fox-family/src/special_n.rs`
- `crates/ft-fox-family/src/special_s.rs`
- `crates/ft-fox/src/init.rs`
- `crates/hsd-particle/tests/data/README.md`
- `crates/hsd-particle/tests/data/airillusion_fd_fox_spawns.json`
- `crates/hsd-particle/tests/live_airillusion_fd_fox.rs`
- `crates/hsd-particle/tests/support/dust_replay.rs`
- `crates/it-foxillusion/Cargo.toml`
- `crates/it-foxillusion/src/lib.rs`
- `crates/melee-cmd/src/decode.rs`
- `crates/melee-cmd/src/lib.rs`
- `crates/melee-ef/src/lib.rs`
- `crates/melee-ef/src/request.rs`
- `crates/melee-ft/src/fighter/commands.rs`
- `crates/melee-ft/src/fighter/landing.rs`
- `crates/melee-ft/src/fighter/ledge.rs`
- `crates/melee-ft/src/fighter/mod.rs`
- `crates/melee-ft/src/fighter/run.rs`
- `crates/melee-ft/src/fighter/spawn.rs`
- `crates/melee-ft/src/fighter/state/special.rs`
- `crates/melee-ft/src/physics/integrate.rs`
- `crates/melee-it/src/engine.rs`
- `crates/melee-it/src/spawn.rs`
- `crates/melee-sim/Cargo.toml`
- `crates/melee-sim/src/frame.rs`
- `crates/melee-sim/src/lib.rs`
- `crates/melee-sim/src/scene_items.rs`
- `crates/melee-sim/tests/alloc_gate.rs`
- `crates/melee-sim/tests/m5_gate.rs`
- `docs/PERF.md`
- `docs/PORT_NOTES/S3_SPECIALS.md`
