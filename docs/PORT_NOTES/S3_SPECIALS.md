# S3 specials: partial port and ownership boundary

S3 Part 3 is in progress. The user clarified that opcode 0xB8's old
unsupported inventory encodes a port limitation, not a retail contradiction.
The opcode is now supported and malformed operands remain rejected.
Grounded Reflector's full particle replay passes after using the fighter root's
local scale in `efLib_Create_Attach_Scale`, rather than matrix decomposition.

The rebased common damage and pose work lets grounded Illusion complete.
Special actions now carry static stale/combo move metadata. Shield Breaker,
Dolphin Slash, and S5's tumble-DI Dolphin Slash each pass the full release CLI
gate. Dolphin Slash additionally required graphics 0x3F1 and the alternate
FallSpecial gravity/capped mobility branch. Counter now also passes the full release CLI gate; full acceptance is running.

## Scene status

Tick numbers below are zero-based. State paths and target reactions come from
the recorded scenes; only the explicitly marked coverage is verified by this port.

| Scene | Recorded actor states | Target reaction | Current evidence / first boundary |
|---|---|---|---|
| illusion_fd_fox | 20 → 21 → 347 → 348 → 349 → 14 | 80 at 87 → 42 at 110 → 14 at 140 | Part 3 release CLI: 300 ticks, 62 keys, 0 divergences; former tick-125 pose boundary resolved |
| airillusion_fd_fox | 24 → 25 → 350 → 351 → 352 → 43 → 14 | idle | Full 300 ticks, 62 keys; ordered particle draws and full particle replay |
| firefox_fd_fox | 353 → 356 → 358 → 35 → 43 → 14 | idle | 300 ticks, 62 keys, 0 divergences; debug/release scene and particle gates pass; 299 measured ticks, zero simulate allocations |
| airfirefox_fd_fox | 24 → 25 → 354 → 356 → 358 → 35 → 43 → 14 | idle | 300 ticks, 62 keys, 0 divergences; debug/release scene and ordered particle RNG gates pass |
| reflector_fd_fox | 360 → 361 → 363 → 14 | idle | 300 ticks, 62 keys, 0 divergences; debug/release M5 and descriptor unit pass |
| reflectorjc_fd_fox | 360 → 361 → 24 → 25 → 42 | idle | 300 ticks, 62 keys, 0 divergences; debug/release M5 pass |
| airreflector_fd_fox | 24 → 25 → 365 → 366 → 363 → 14 | idle | 300 ticks, 62 keys, 0 divergences; debug/release M5 pass |
| shieldbreaker_fd_marth | 20 → 21 → 23 → 14 → 341 → 342 → 343 → 14 | 80 → 42 | Part 3 release CLI: 300 ticks, 62 keys, 0 divergences; former tick-165 metadata boundary resolved |
| dancingblade_fd_marth | 349 → 351 → 352 → 357 → 14 | miss | 300 ticks, 62 keys, 0 divergences; debug/release M5 pass |
| dolphinslash_fd_marth | 20 → 21 → 23 → 14 → 367 → 35 → 43 | 88 → 38 → 0 | Part 3 release CLI: 300 ticks, 62 keys, 0 divergences; metadata, launch dust and tick-178 FallSpecial gravity resolved |
| counter_fd_marth | 369 → 370 → 14 | Fox 89 → 191 → 192 | Part 3 release CLI: 300 ticks, 62 keys, 0 divergences; trigger, flash, AttackDash reaction and DownBoundD/DownWaitD verified |

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

## Part 1 validation (historical)

Both full workspace gates passed: 1,021 passed, zero failed, three pre-existing
ignores in each profile. M4 remains 261; M5 is 30. The four new tests are two
scene/prefix tests, the aerial particle replay and its zero-allocation gate.
`MELEE_ALLOW_MISSING_DATA` was unset. These green workspace totals do not turn
the nine unimplemented scenes or grounded pose boundary into completed gates. The required
Fire Fox/Counter allocation cases and Fire Fox particle replay are not available
because those moves are not ported. The new aerial Illusion allocation gate has a
zero-allocation ceiling, with zero measured allocations across ticks 1–299.

### Part 1 release scene commands (historical)

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

## Part 2 progress: Fire Fox

Grounded Fire Fox passes the full release CLI gate. Its first resource boundary
was opcode 38 (`ftAction_80071FC8`, one `HSD_Randi` at 80072014); the typed command
retains all six sound choices, range, channel, volume and pan. No captured
sound choice is used by production code.

The first particle replay had 248 field mismatches starting at launch tick 73.
The launch model callback sets Y/Z rotation after animation (`efLib_Update`);
refreshing its particle-facing joint matrices after that callback eliminates
all mismatches. The strict simulation-field replay now compares 628,334 fields
and 10,574 ordered particle draws over 300 ticks with zero mismatches. Existing
display-cache exclusions are unchanged.

Fire Fox scratch: +2340 gravity delay (`i32`), +2344 relative launch angle
(`f32`), +2348 remaining travel frames (`i32`), +234C elapsed physics ticks
(`i32`), +2350 grounded collision ticks (`i32`), plus the typed pending
accessory effect. Shared rows 353..358 compile for Fox and Falco.

### Reflector progress

Grounded Reflector and aerial Reflector pass the 300-tick release comparator.
The grounded first-stop history was effect 0x3FA (common model 0x15),
particle kind 372, then particle instruction 0xB8 at tick 37. The latter now
implements the retail point-joint force/proximity branch with audited fused
arithmetic; unbound joints consume the instruction without changing velocity.
The descriptor and contact-direction reaction unit and particle force/proximity
unit pass in debug and release. Reflector scratch is +2340 release lag (i32),
+2344 turn countdown (i32), +2348 released latch (bool), +234C gravity delay
(i32), plus owned defense data and an accessory callback effect latch.

### Dolphin Slash progress

Both profiles pass ticks 0–122, including ordered particle draws. The release
CLI stops at tick 123 in `damage.rs:705`: `GROUND_MOVES[attacker.motion_state.id
as usize]` receives `CommonMotionState::None` for action 367. S5 needs the
character special move ID at the combo-recording boundary (SpecialHi); this
lane has left damage.rs unchanged. Recorded target state 88 starts at 123,
38 at 170, DeadDown 0 at 208. Actor FallSpecial starts at 164, landing 43 at 185;
those later transitions remain unverified. The first resource stop before this
was command 25 mode 2, now ported as the typed `AirUseAllJumps` conversion
(five locked ECB ticks). Dolphin scratch stores retail +6BC lstick_angle;
command vars +2200/+2204/+2208 retain their script-owned meanings.

### Shield Breaker progress

Ticks 0–164 pass in both profiles, including ordered particle draws; first
contact at tick 165 reaches the same `damage.rs:705` special move lookup as
Dolphin Slash. The required move here is SpecialN. Startup enters at 119,
charge at 130, release at 161. First resource boundary was sound behavior 3,
now represented as the retained +2150 effect sound channel. The synchronous
release uses Marth bank 16 and models 0x3E80/0x3E81, loaded once into the pool.
Scratch +2340 is the integer charge counter, plus an accessory effect latch.

### Dancing Blade progress

Release CLI passes all 300 ticks. The initial frame-47 continuation mismatch
was a missing explicit reset of command vars 0 and 1 at entry, now matched
to retail entry and continuation callbacks. The recorded sequence selects
351, 352, then 357 from the live stick threshold and AB press window.
Scratch +2340 retains retail specials.x0; windows remain script command vars.

### Counter progress

Release CLI reaches tick 60, the recorded 369→370 trigger, then stops explicitly
at `CharacterCallbacks::check_hurtbox_interaction`. The C8 boundary is called
per candidate in S5-owned `damage.rs`, but currently takes only `&Character`.
It needs mutable Fighter, attacker/hit-capsule context, and assets, plus a
consumed-contact result, to test the Counter defense volume before ordinary
shield/hurt contact and call `ftMs_SpecialLw_80139140`. No damage/down code was
changed. Target 89 begins at 73, DownBoundD 191 at 96, DownWaitD 192 at 122;
these remain unverified. Scratch +2340 is scaled damage (Roy consumes it;
Marth uses script damage), with owned AbsorbDescriptor-layout Counter volume
and the attribute +60 collision multiplier. Subaction 58 is decoded as its
four-word signed wind payload; executing its dynamic-wind callback remains
an explicit later boundary, beyond the first contact.

## Part 2 implementation scope and remaining boundaries

Fox/Falco share `rows::<C>()`, attributes and typed `SpecialHi`/`SpecialLw`
accessors through `FoxFamily`. Falco compiles the same callbacks; only Fox has
new scenario evidence. Reflector's descriptor is `melee_coll::defense` data;
its unit tests preserve the descriptor scalars and verify ground/air reaction
state, facing and impulse parameters. No reflected-projectile scene is claimed.
Reflector turns, platform drops, aerial jump cancel and unsupported support
transitions stop explicitly. Fire Fox's ground-directed travel, rebound and
preserved support transitions remain explicit ungated branches.

Marth owns its special table and four scratch types in `ft-mars`; there is no
kind dispatch or generic fighter shell. Dolphin Slash and Shield Breaker's
post-contact behavior, Counter's trigger/flash/knockdown and a full Counter
allocation gate await the documented incoming-hit boundaries. Wind/visual
impulses from lb_800119DC are not simulated; they are not RNG sites. The later
subaction-58 dynamic-wind execution is an explicit stop. Marth aerial neutral,
side and down-special entries are outside these recorded scenes and remain
explicit stops. No full Marth particle replay is claimed.

Ordered particle sites are compared on every gated tick. Canonical RNG seeds
cover effect/sound draw counts; Fire Fox's random sound site is 80072014, its
range command is seven words, and seeking consumes no draw. The Fire Fox
particle replay separately excludes that identified nonparticle Randi and
retains the existing particle display-cache exclusions.

### Part 2 changed files

- `ft-fox-family/src/{lib,special_s,special_hi,special_lw}.rs`, its manifest,
  and `ft-{fox,falco}/src/init.rs`: shared rows and typed scratch.
- `ft-mars/src/{lib,init,special_hi,special_n,special_s,special_lw}.rs` and
  manifest: character-owned table, callbacks and explicit incoming-hit stop.
- `melee-cmd/src/{lib,decode}.rs`, `melee-ft/src/fighter/{assets,commands,
  effects,procs,jump,spawn}.rs`, `melee-ft/src/input/{human,iasa}.rs`: typed
  command extensions, execution and exposed concrete transition/input helpers.
- `melee-ef/src/{lib,pool,request,tables}.rs`,
  `hsd-particle/src/{particle,system}.rs`: synchronous effects, orientation,
  Marth bank, Reflector light and point-joint force instruction.
- `melee-sim/src/{assets,frame,initial_state/mod,initial_state/cold}.rs`:
  initialization-only effect loading and command resolution.
- `melee-sim/tests/{m5_gate,alloc_gate}.rs`,
  `hsd-particle/tests/{live_firefox_fd_fox,support/dust_replay}.rs`,
  Fire Fox spawn fixture and its README: scene, allocation and particle proof.
- `Cargo.lock`, `CLAUDE.md`, this report and `TRACKER.md`: dependencies and
  progress. No protected game-data or decomp files were edited.

## Part 2 final validation and required stop

`cargo clippy --workspace --all-targets -- -D warnings` passed. `cargo fmt
--all --check` passed. All nine added scene tests pass individually in debug
and release: six cover 300 ticks, three cover the documented prefixes.
Grounded Fire Fox's particle replay passes both profiles: 628,334 fields,
10,574 ordered particle draws, zero mismatches. Its allocation gate reports
299 measured ticks, zero simulate allocations. Counter's available allocation
prefix reports 59 measured ticks, zero simulate allocations.

The full `cargo gate` stopped at the existing particle opcode test (18 passed,
1 failed in that integration suite). Exact failure:

```text
thread 'unported_opcodes_and_malformed_programs_fail_explicitly' panicked at crates/hsd-particle/tests/opcodes.rs:401:9:
assertion `left == right` failed
  left: Err(TruncatedProgram { pc: 1 })
 right: Err(UnsupportedOpcode { opcode: 184, pc: 0 })
test result: FAILED. 18 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
error: test failed, to rerun pass `-p hsd-particle --test opcodes`
```

`opcodes.rs:391–395` omits 0xB8 from the supported set, so its one-byte input
expects UnsupportedOpcode. Retail `sysdolphin/baselib/particle.c:1635` defines
that instruction, reached by grounded Reflector at tick 37. Now that it is
implemented, the same input is missing operands and returns TruncatedProgram.
This is a support-inventory expectation that conflicts with the requested new
retail behavior. Per the user's explicit rule, work stopped and the existing
test/expected values were left untouched. Updating the inventory and adding a
truncated-0xB8 case require resolution of that stop instruction.

The chained full release gate and release-math script did not run after the
debug failure. The final M4 261 suite had not been reached; the earlier baseline
M4 passed before these changes, which is not final-tree proof. No Part 2
performance/copy census is claimed: the requested idle-host perf refresh was
not run after the mandatory stop. The 1,440-copy table and perf lines above
are explicitly Part 1 historical results. A fresh census remains necessary,
particularly for the new concrete random-sound resolver and newly exposed
input helpers. No budget was raised.

Latest release CLI final lines for each fully passing Part 2 scene:

```text
firefox_fd_fox:       300 ticks, 62 keys, 0 divergences
airfirefox_fd_fox:    300 ticks, 62 keys, 0 divergences
reflector_fd_fox:     300 ticks, 62 keys, 0 divergences
airreflector_fd_fox:  300 ticks, 62 keys, 0 divergences
reflectorjc_fd_fox:   300 ticks, 62 keys, 0 divergences
dancingblade_fd_marth: 300 ticks, 62 keys, 0 divergences
```

No commits or git write commands were run. The initial decomp type-change was
pre-existing and left untouched. Protected scenario, trace and ROM files were
only read.
