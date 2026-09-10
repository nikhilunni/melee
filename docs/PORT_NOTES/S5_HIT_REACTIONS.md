# S5: hit reactions, DI and getups

2026-09-10, battlefield/perf lane. Uncommitted. **Seven of eight full scenes
are exact; acceptance remains blocked by S3 at tick 119 of Dolphin Slash.**
All eight full tests are enabled in `m5_gate.rs`; none is ignored, shortened or
weakened. No Git write command, game-data write, scenario change, decomp edit,
expected-value edit or other active lane's source edit was made.

## Scene evidence

State sequences below come from the recorded tick traces. Movement before the
attack and intermediate attacker Wait states are omitted. History records
successive first failures, without changing expected values. Both sides of
all seven completed scenes pass the 49-key and ordered-particle M5 helper.
The existing CLI also compares item keys and therefore prints **62 keys**, not
49, even when the recorded item list is empty. That S4 behavior was preserved.

| Scene | Recorded reaction states (tick:state) | First-divergence history | Final release CLI |
|---|---|---|---|
| `di_upaway_fsmash_fd_marth` | P2 149:87 → 191:38 → 232:0 | SDI/ASDI guard → exact | 300 ticks, 62 keys, 0 divergences |
| `di_downin_fsmash_fd_marth` | P2 149:87 → 157:183 → 183:196 → 218:14 | SDI/ASDI guard → tick 151 animation (premature landing during hitlag) → stay-airborne collision → exact | 300 ticks, 62 keys, 0 divergences |
| `sdi_fsmash_fd_marth` | P2 149:87 → 191:38 → 229:0 | SDI/ASDI guard → exact | 300 ticks, 62 keys, 0 divergences |
| `cc_ftilt_fd_marth` | P2 101:39 → 108:40 → 125:76 → 129:42 → 133:40 → 171:41 → 181:14 | crouch modifier guard → tick 129 animation (hitlag two ticks too long) → crouch hitlag scaling → exact | 300 ticks, 62 keys, 0 divergences |
| `tumbledi_dolphinslash_fd_marth` | P1 119:367 → 164:35 → 185:43; P2 123:88 → 130:183 → 144:29 → 200:0 | tick 119 P1 animation, unchanged: S3 special entry missing | BLOCKED: 119 ticks matched |
| `getupattack_fd_fox` | P2 113:90 → 144:183 → 170:184 → 177:187 → 230:14; P1 193:88 → 215:183 → 241:184 | tick 177 P2 animation → recovery rows/input → attack scratch invariant after attacker hitlag → support prone attack scratch → exact | 300 ticks, 62 keys, 0 divergences |
| `getupstand_fd_fox` | P2 113:90 → 144:183 → 170:184 → 177:186 → 207:14 | tick 177 P2 animation → recovery row/input → exact | 300 ticks, 62 keys, 0 divergences |
| `getuproll_fd_fox` | P2 113:90 → 144:183 → 170:184 → 177:188 → 212:14 | tick 177 P2 animation → recovery row/input → exact | 300 ticks, 62 keys, 0 divergences |

The neutral `fsmashcharge_fd_marth` comparison has the same launch velocity
at tick 149: `(1.9327497482299805, 1.8664348125457764)`. At tick 157 its
position is `(61.89606475830078, 1.6011072397232056)`; up-away is
`(68.19588470458984, 7.901285171508789)`, with post-decay velocity
`(1.895891547203064, 1.831185221672058)`. The nearly parallel diagonal stick
produces a small DI rotation alongside its SDI/ASDI displacement. The SDI scene
moves X from 60 to 66, 72 and 78 at ticks 151, 153 and 155, exactly three
six-unit taps during hitlag; its neutral-input launch at tick 157 retains the
reference velocity and adds 18 units to X. Down-in is floor-clamped during
hitlag and enters DownBoundU on tick 157 after the final ASDI displacement.

All three getup recordings contain **Fox versus Fox** (`p0.kind = p1.kind = 1`),
not Marth as the first fighter. The getup attack does hit and knock down P1;
that entire reaction is gated. The required down-in path goes directly from
DownBoundU to DownFowardD: retail's roll selector tests specifically DownWaitU,
including when called by DownBound_Anim. This counterintuitive state choice is
preserved (`ftCo_Down.c:39-49`).

The task's referenced `docs/PORT_NOTES/S2_AERIALS_LCANCEL.md` is absent from this
checkout. C15's concrete shell migration, the available prior reports, and the
current code were used. The damage-level-2 stop mentioned in the prompt is
already absent from this revision: S1 supplies the ground damage levels and
DamageFly rows. Existing initially-airborne-hit boundaries are retained.

## DI, SDI, ASDI and crouch formulas

All values below are read from `PlCo.dat` common data, through `DamageParameters`.
Offsets describe archive input only; gameplay uses named fields. The status
phase has no asset argument, so each `DamageState` retains a small typed copy
of the influence parameters from damage entry. No character union or heap
allocation is introduced.

| Parameter | Common offset | Value read (f32 bits where relevant) |
|---|---|---|
| Minimum SDI/ASDI stick magnitude | 0x4B0 | 0.699999988079071 (`3f333333`) |
| SDI tap age, strict less-than | 0x4B4 | 4 |
| SDI displacement scale | 0x4B8 | 6.0 (`40c00000`) |
| ASDI displacement scale | 0x4BC | 3.0 (`40400000`) |
| Maximum DI angle, degrees | 0x1A8 | 18.0 (`41900000`) |
| Held-shield launch-speed scale | 0x1AC | 1.0 (`3f800000`) |
| Crouch knockback multiplier | 0x124 | 0.6666666865348816 (`3f2aaaab`) |
| Crouch hitlag multiplier | 0x1A0 | 0.6666666865348816 (`3f2aaaab`) |
| Tumble stick-exit threshold / tap age | 0x210 / 0x214 | 0.800000011920929 (`3f4ccccd`) / 1 |
| Getup stand / roll threshold | 0x244 / 0x248 | 0.20000000298023224 (`3e4ccccd`) |
| Bounce attack input buffer | 0x24C | 60.0 (`42700000`) |
| C-stick getup attack threshold | 0x7F4 | 0.6625000238418579 (`3f29999a`) |

**SDI:** `ftCo_Damage.c:569-589`, `ftCo_Damage_OnEveryHitlag` (8008E4F0).
While hitlag is active, require `sx*sx + sy*sy >= minimum*minimum` and either
axis tilt age `< tap_window`. Add `(sx*distance, sy*distance)` to position and
reset both tilt ages to 254. Products and sums round separately. The callback
runs in Fighter_procUpdate after combo separation (`fighter.c:2374-2378`),
after this tick's input sampling. Damage collision uses `mpColl_800477E0`
(stay airborne) during hitlag, as `ft_081B.c:125-165` requires. This clamps SDI
against the floor without entering DownBound early.

**ASDI:** `ftCo_Damage.c:624-648`, `ftCo_Damage_OnExitHitlag` (8008E714).
At hitlag expiry in the status phase, use C-stick if its squared magnitude
passes the same radius (`ft_0DF1.c:117-125`); otherwise use the main stick.
If that chosen stick passes, add `asdi_distance * stick` to position, with
separate products and sums (8008E7A8..8008E7DC). This consumes the prior sampled
input, before the current tick's input callback. ASDI precedes launch DI.

**DI:** `ftCo_Damage.c:591-622`, `ftCo_8008E5A4` (8008E5A4), is the actual
launch-rotation helper in this pinned decomp. For velocity `(vx, vy)` and main
stick `(sx, sy)`, compute:

```text
q = fmadds(-vx, -vx, vy * vy)
p = fmadds(vy, sx, (-vx) * sy)
f = (p * p) / q
if cross(velocity, stick).z < 0: f = -f
speed = sqrtf(fmadds(vx, vx, vy * vy))
angle = fmadds((PI_f32 / 180) * maximum_degrees, f, atan2f(vy, vx))
(vx, vy) = (speed * cosf(angle), speed * sinf(angle))
```

A neutral main stick leaves velocity untouched; the retail squared-speed
zero guard is `q < 0.00001f` (a named constant, not a stick threshold).
The cross-product sign uses the SDK's rounded product then `fnmsubs`, matching
`PSVECCrossProduct` at 80342E58, including rounded-result negation. Fused sites
are 8008E5FC, 8008E624, 8008E660 and 8008E6CC. Sqrt uses the existing audited
MSL refinement. Held shield then rescales speed using common 0x1AC
(`ftCo_Damage.c:655-663`, fused magnitude at 8008E860).

**Crouch cancel:** `ftCo_Damage.c:118-131`, `ftCo_Damage_CalcKnockback`
(8008D930), multiplies applied knockback by common 0x124 for exactly Squat and
SquatWait, before reaction-level, launch and hitstun selection. The product is
8008D974 `fmuls`. `ftCommon_CalcHitlag`, `ftcommon.c:640-648`, also truncates
`normal_hitlag * common[0x1A0]` to an integer when the **pre-hit** motion is
Squat/SquatWait (8007DAF8 `fmuls`, 8007DAFC `fctiwz`). Keeping that pre-transition
motion is what advances the crouch scene's landing from tick 131 to tick 129.

## State and allocation ownership

Eight static common rows add face-up/down stand, attack, forward roll and
back roll. DownWait rows gain their own IASA callback; downbound animation
checks buffered attack, then roll, before entering DownWait. DownWait checks
fresh attack, roll, then stand; its timer expires into stand. Stand starts
at animation frame zero, while attack/roll perform retail's immediate step.
The existing down and grounded/root-motion physics callbacks are reused.
DamageFall gains its common-data horizontal stick-tap exit after the existing
aerial input checks. No `Fighter<C>` or generic common callback was added.

**Character hooks added: none.** These paths are common behavior; all numeric
choices come from descriptors/common data. Getup hitboxes are loaded through
the existing melee-cmd interpreter and resolved through melee-coll. No attack
geometry, future trace state or RNG result is injected into gameplay.

The new SDI allocation gate first found one allocation at the final KO.
`StockDisplay::tick` built a Vec for up to five stock-icon effect positions.
It now returns an iterator over a fixed five-slot option array, preserving
slot order and completing all timer mutations before consumption. This small
melee-if change is required by the new zero-allocation scene, affects no active
lane-owned file, and also removes the pre-existing KO scene's last allocation.
Neither allocation ceiling nor warm-up/counting procedure was changed.

Adding common getup resources initially raised the broad melee-ft census from
845 to 857. A concrete `motion_indices` asset helper now collects base, idle,
character and getup resource lists using the same sorted set type; it avoids
nested chain/map drop instantiations while preserving identical indices and
load order. The focused census returns to 845; common gameplay remains concrete.

## Blocking S3 dependency and retained limits

`tumbledi_dolphinslash_fd_marth` first differs at frame 119, `p0.cur_anim_frame`:
expected `1 (0x3F800000)`, actual `6 (0x40C00000)`. Retail enters Marth SpecialHi
367 there; this checkout has no Marth special rows/entry override. S3 must
supply that entry, its attack and subsequent FallSpecial/LandingFallSpecial
behavior. No S5 damage is reached before the failure. The full eight-scene
test remains enabled and failing, rather than hiding the dependency behind
an ignore or prefix-only acceptance test. Re-run after S3 integration to
verify the recorded tick-130 bounce, tick-144 edge departure and tick-200 KO.

Unrecorded DownDamage, neutral/forward tech, wall/ceiling tech, meteor cancel,
initially-airborne follow-up hits and other damage modifiers remain outside
this verified slice. The face-down getup rows share retail's callbacks and
resource preparation; only the down-in forward-roll branch exercises that
orientation here. No contradiction with an existing retail test was found.

## Final validation

The initial unchanged workspace baseline passed **1,017 / 0 / 3 ignored**.
After S5, complete coverage in **each profile** is **1,026 passed, 1 failed,
3 pre-existing ignored**. The eight new scene tests add seven passes and the
S3 dependency failure; the two new allocation tests both pass. No existing
oracle test fails. `cargo gate` and `cargo gate --release` exit 101 at M5;
Cargo stops scheduling remaining targets after that failure. Those remaining
four melee-sim integration targets, melee-test-support, melee-types, slp and
all workspace doctests were run separately in both profiles and pass. The
aggregate above deduplicates the repeated doctest runs.

| Command | Result / exact result line |
|---|---|
| `cargo gate` | exit 101; M5: `test result: FAILED. 35 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 25.58s` |
| `cargo gate --release` | exit 101; M5: `test result: FAILED. 35 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.32s` |
| `cargo test -p melee-sim --test m4_gate` | exit 0; `test result: ok. 261 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 182.94s` |
| Release M4 inside workspace gate | `test result: ok. 261 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.48s` |
| Allocation gates, debug | `test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.33s` |
| Allocation gates, release | `test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.20s` |
| `tools/check-release-math.sh` | exit 0; 74 tests, including all six opt levels; final line `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s` |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0; ``Finished `dev` profile [unoptimized + debuginfo] target(s) in 44.20s`` |
| `cargo fmt --all -- --check` | exit 0, no output |
| `tools/merge-check.sh lane/battlefield` | exit 1 at `[FAIL] gate`; same sole M5 failure, `test result: FAILED. 35 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 47.08s` |

Both failed workspace commands end with:

```text
error: test failed, to rerun pass `-p melee-sim --test m5_gate`
```

The failing test names only `tumbledi_dolphinslash_fd_marth`; its exact
first-divergence lines are:

```text
first divergence at frame 119 phase frame_end
  field:    p0.cur_anim_frame
  expected: 1 (0x3F800000)
  actual:   6 (0x40C00000)
119 ticks matched; RNG-writing procs this tick: [("ParticlesMain", 2079789175)]
```

Seven final executions of `cargo run -q --release -p melee-sim -- gate
harness/scenarios/<scene>.toml` exit 0 and print exactly:

```text
300 ticks, 62 keys, 0 divergences
```

Their M5 helpers print `300 ticks, 49 keys, 0 divergences` and additionally
check ordered particle RNG against the independent ledger. The eighth CLI
command exits 1 at the boundary above; no 300-tick pass is claimed.

Both new allocation scenes report these exact measurements in both profiles:

```text
sdi_fsmash_fd_marth: 299 measured ticks; simulate-only 0 (0.000000/tick), peak 0, allocating ticks 0; with snapshot 37375 (125.000000/tick), peak 125; snapshot overhead 37375 (125.000000/tick)
getupattack_fd_fox: 299 measured ticks; simulate-only 0 (0.000000/tick), peak 0, allocating ticks 0; with snapshot 37375 (125.000000/tick), peak 125; snapshot overhead 37375 (125.000000/tick)
```

Existing ceilings remain unchanged; the KO allocation count decreases from
one to zero after the stock-HUD fix. Snapshot allocations remain separately
reported and unchanged.

Command manifests and full logs are `/private/tmp/s5-validation.json`,
`/private/tmp/s5-validation-tail.json`, `/private/tmp/s5-final-release-scenes.json`,
and `/private/tmp/s5-final-*.log`. The exact per-scene debug history is in
`/private/tmp/s5-<scene>-{before,v1,v2}.log` where applicable. Final aggregate
counts are not represented as a fabricated Cargo summary line.

Merge-check passed its protected-data, trace-presence, ancestry and build
checks. Main is an ancestor of this lane. It then stopped at the same enabled
Dolphin Slash test; its later checks were not reached. The separately run
checks above provide their own evidence, without claiming merge-check passed.

`tools/perf-gate.sh` exits **0 / PASS** after the other S5 validation processes
finished, with unchanged budgets and tolerances. Exact final line:

```text
[PASS] perf-gate: 3483104 stripped bytes, 3178496 text bytes; load 168.760 ms; ticks_600 23.675 ms
```

Evidence: `target/perf/20260910T090054Z-57610`, automatically appended to
`docs/PERF.md`; full command log `/private/tmp/s5-perf-idle-final.log`.
The census is flat at **1,440 copies**, with each of **274 common labels**
defined once. Per-crate counts remain at their C15 ceilings:

| Compiling crate | Copies |
|---|---:|
| melee-ft | 845 |
| melee-sim | 128 |
| ft-captain | 66 |
| ft-falco | 67 |
| ft-fox | 67 |
| ft-fox-family | 0 |
| ft-mario | 0 |
| ft-mars | 66 |
| ft-peach | 67 |
| ft-purin | 66 |
| ft-yoshi | 68 |

Earlier runs are retained in the append-only performance log. The first
reported 857 melee-ft copies / 1,452 total before the asset helper correction.
The second passed the corrected census and size checks but measured load
255.688 ms and ticks_600 71.934 ms while S5 debug tests/builds were running.
The final run above removes that known competing S5 workload; no benchmark,
baseline or ceiling was weakened.

## Changed files

- `TRACKER.md`
- `crates/melee-ft/src/fighter/assets.rs`
- `crates/melee-ft/src/fighter/damage.rs`
- `crates/melee-ft/src/fighter/down.rs`
- `crates/melee-ft/src/fighter/procs.rs`
- `crates/melee-ft/src/fighter/state/common_table.rs`
- `crates/melee-if/src/lib.rs`
- `crates/melee-sim/tests/alloc_gate.rs`
- `crates/melee-sim/tests/m5_gate.rs`
- `docs/PERF.md` (automatic append-only performance evidence)
- `docs/PORT_NOTES/S5_HIT_REACTIONS.md`
