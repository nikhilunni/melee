# S6: shield stun, powershield, shield break and dizzy

2026-09-10, `lane/battlefield`, following the S3/S4/S5/S9/S10 integration.
All five requested scene CLI gates now pass. Performance passes; full workspace
and merge acceptance remain blocked by the lane's empty decomp directory.
No scenario, recording, game data, decomp or expected value was changed. No
commit or Git write command was run by this session.

## Scene evidence and first-divergence history

Ticks are zero based. Marth's approach is Wait 14 → Dash 20@31 → Run 21@46 →
RunBrake 23@87. The four tilt scenes add WalkSlow 15@113 and AttackS3S 53@119.

| Scene | Marth attack states | Fox shield/reaction states | First-divergence history | Final release CLI |
|---|---|---|---|---|
| `shieldstun_ftilt_fd_marth` | 53@119 → 14@159 | 182@111 → 179@119 → 181@125 → 179@137 → 180@171 → 14@186 | Exact before and after the port | `300 ticks, 62 keys, 0 divergences` |
| `shieldtilt_ftilt_fd_marth` | 53@119 → 14@159 | 182@111 → 179@119 → 181@125 → 179@137 → 180@171 → 14@186 | Exact before and after the port | `300 ticks, 62 keys, 0 divergences` |
| `lightshield_ftilt_fd_marth` | 53@119 → 14@159 | 178@111 → 179@119 → 181@125 → 179@138 → 180@171 → 14@186 | Exact before and after the port | `300 ticks, 62 keys, 0 divergences` |
| `powershield_ftilt_fd_marth` | 53@119 → 14@159 | 182@124 → 181@125 → 179@137 → 180@171 → 14@186 | Original contact stop at 125 → all fighter scratch exact, RNG `C40288F8` vs `4EEE4E83` → color 118 graphics decoded, RNG `3FC52AE7` → old shield destruction moved before new graphics → exact | `300 ticks, 62 keys, 0 divergences` |
| `shieldbreak_fd_marth` | 341@119 → 342@130 → 343@216 → 14@256 | 182@111 → 179@119 → 205@220 → 207@257 → 209@283 → 211@313 | Original Marth divergence at 119; resolved by S3 → exhaustion stop at 220 → break effect timing (`AE9BF8A7` vs `3802DEC5`) → script effects 0x3E9/0x515/0x429 and sound behavior 6 → exact | `520 ticks, 62 keys, 0 divergences` |

182 is GuardReflect, 178 GuardOn, 179 Guard, 180 GuardOff and 181 GuardSetOff.
Fox remains dizzy through tick 519. ShieldBreakFall 206 is unrecorded and
explicitly unimplemented. The 520-tick M5 gate is enabled; its former S3 ignore
has been removed. The additional 118-tick prefix test is retained.

The existing CLI includes 13 item keys, even for an empty item list, and prints
62 keys. The M5 helper verifies the requested 49 fighter/RNG keys and ordered
particle RNG callsites independently. CLI output was not changed to claim 49.

## Powershield contact and effect order

`ftColl_80076CBC` retains attacker hitlag/pushback and defender impact. Its
physical powershield branch omits accumulated shield damage and spark 1052,
clears minimum hold and installs PlCo +0x2B8 (4) as the GuardOff attack-interrupt
window (`ftCo_80094138`). It requests effect 27, color animation 118 and sound
104. `take_shield_hit` retains shieldstun while omitting effect 1049 and the
ordinary 0.6 pushback multiplier. At tick 125 the raw recording and port agree
on velocity **1.2100000381469727** and shield health **59.720001220703125**.

Color animation 118 is not solely renderer output: its PlCo program contains
effect 0x404. `ftCo_800C0408`/`lb_80014258` invoke the ordinary graphics command,
which consumes three offset draws even for zero ranges. The existing decoded
overlay interpreter now executes this program, including during hitlag; no
recording-derived random value or schedule is inserted into production code.

Motion entry destroys the old shield before executing the new overlay's
synchronous dust. The effect queue's `BeforeGraphics` boundary drains the
already sealed outgoing requests through `DestroyOwned`, preserving the new
dust. Previously the delayed destruction also removed the new dust, losing
its particle draws. Powershield effect 27 itself uses the existing positional
particle generator path.

Projectile powershield reflection remains the existing explicit unsupported
branch; none of the five recordings exercises a reflected projectile.

## Exhaustion, break states and typed scratch

Damage exhaustion resets health to PlCo +0x280 (30), enters 205 and applies
hitlag from the shield impact (`fighter.c:2835-2841,2910-2920` in the available
reference; the response is `2941-2947` in the task's source revision).
Continuous drain exhaustion (`Guard.c:428-436`) clamps health to
zero, enters the same launch and requests its distinct sound 129. Break entry
uses the character's initial vertical velocity, performs the retail immediate
animation step, emits effect 1051 and sets intangible hurt status. Down and
stand retain that status; dizzy entry clears it.

| Recording tick | State | Shield health | Vertical velocity | Hurt status | Dizzy timer |
|---|---|---|---|---|---|
| 220 | ShieldBreakFly 205 | 30 | 3.299999952316284 | 2 | — |
| 257 | ShieldBreakDownU 207 | 32.589988708496094 | -2.799999952316284 | 2 | — |
| 283 | ShieldBreakStandU 209 | 34.40998077392578 | 0 | 2 | — |
| 313 | Furafura 211 | 30.06999969482422 | 0 | 0 | 490 |
| 519 | Furafura 211 | 30.06999969482422 | 0 | 0 | 284 |

The unit test replays real inputs and compares these milestone states plus raw
combat scratch, health, hurt status and dizzy timer against the recording.
Subsequent trace bytes are assertions only. The health at dizzy frame end is
30 plus one regeneration step because Anim writes 30 before ProcessHit.

`DizzyState` owns the remaining timer and two signed stick directions. Returning
to neutral retains the last direction; crossing to a new signed direction
counts one mash, even if both axes change. Button and stick mashes can each
subtract 3 in one tick (`ftCommon_GrabMash`). State callbacks live in the new
`shield_break.rs`; four rows are appended to the one common table.
Animations 286/288/290/205 are prepared during initialization.

New descriptor data: `CommonBehavior::shield_break_top_exit` supplies retail's
Purin difference. `Status::unconditional_top_exit` represents x2222_b3, is
cleared on motion change (`fighter.c:1075`), and replaces S9's ShieldBreakFly
stop in the top-exit test. No new character callback function or generic
fighter shell was introduced. Fox's false flag is checked by the unit test;
no new Jigglypuff shield-break recording is claimed.

The break burst uses generator 0x31 with the shield joint's local scale and
world translation. Dizzy stars use generator 0xCE attached through an AppSRT,
with character attribute +0x168. Script effect 0x3E9 maps to generator 0xC;
0x515 records quake 4. Sound behaviors 4 and 6 retain their distinct channels
as output requests. All new queues and state use fixed storage.

## Retail formulas

**Trigger and size.** `fighter.c:1832-1894` takes the maximum normalized
shoulder trigger, applies the 0.3 deadzone, and forces it to 1 for digital L/R.
Guard.c:310-336, 405-439 then computes `L = (T - deadzone) / (1 - deadzone)`.
A negative result preserves the previous amount (initially zero). Despite the
field name `lightshield_amount`, increasing L makes the shield smaller.
Guard.c:191-215 computes:

```text
size_blend = fma(L, size_range[1] - size_range[0], size_range[0])
health_fraction = (health / 60) * size_blend
size = fma(1 - minimum_size, health_fraction, minimum_size) * character_size
size_range = [1, 0.5]; minimum_size = 0.15
```

Yoshi's existing character-owned egg shield retains its separate fixed size.
Digital L enters GuardReflect through the fresh digital-edge/window test;
analog-only input enters GuardOn (Guard.c:70-86, 957-978).

The scenario's `TriggerLeft 0.5` is a Dolphin input setting, not the normalized
fighter trigger. Its captured HSD pad at tick 111 has raw analogL 128 and
normalized analogL `0.9142857193946838`; Fighter +0x650 agrees. Therefore the
recorded L is **`0.8775510191917419`**, not `(0.5 - 0.3) / 0.7`. Both pad input
and raw fighter comparisons confirm this without altering scenario data.

**Decay, damage and regeneration.** Guard.c:405-447 drains active shielding:

```text
health -= drain * fma(L, drain_range[1] - drain_range[0], drain_range[0])
drain = 0.14; drain_range = [0.1, 2]
```

`ftcoll.c:487-492` accumulates ordinary contact damage
`D += max(integer_hit_damage + hitbox_shield_damage, 0)`, using command-derived
hitbox data. `fighter.c:2816-2844` applies it in ProcessHit:

```text
light_damage = fma(L, damage_lightshield[1] - damage_lightshield[0],
                  damage_lightshield[0])
health -= fma(damage_multiplier, D * (1 - light_damage), frame_damage)
damage_lightshield = [0.1, 0.3]; damage_multiplier = 1; frame_damage = 0
```

Without the shield-enabled flag, health below 60 regenerates by 0.07, capped at
60 (`fighter.c:2817-2826`). Drain exhaustion clamps health to zero and enters
ShieldBreakFly; damage exhaustion resets it to PlCo +0x280 (30) and marks the
break response. Both exhaustion branches are implemented.

**Stun and pushback.** Guard.c:650-717 uses integer hit damage `d`:

```text
light_stun = fma(L, stun_lightshield[1] - stun_lightshield[0], stun_lightshield[0])
stun = fma(stun_multiplier, d * (1 - light_stun), stun_base)
animation_rate = (0.1 + GuardDamage_animation_length) / stun
ordinary_push = min((stun * pushback_multiplier) * ordinary_multiplier, maximum)
powershield_push = min(stun * pushback_multiplier, maximum)
stun_lightshield = [0.05, 0.7]; stun_multiplier = 1.5; stun_base = 2
pushback_multiplier = 0.2; ordinary_multiplier = 0.6; maximum = 2
```

The defender moves away from the attacker. Cape is a separate response.
`ftcoll.c:454-469` records attacker `L*d` and relative direction;
`fighter.c:3009-3015` sets attacker shield knockback magnitude to
`fma(L*d, 0.07, 0.02)` when that recorded product is nonzero. Subsequent grounded
friction is handled by the existing physics path. Hitlag uses the existing
archive-driven `ftCommon_CalcHitlag` calculation.

The light-shield recording keeps GuardSetOff one tick longer (through 137),
with contact rate `2.821254253387451` and push `0.8549389243125916`. These are
read-only observations verified by the added scratch replay, not baked-in
expected constants in tests.

**Tilt.** Guard.c:138-188 maps the facing-relative stick angle into 0..359
degrees, smooths the shortest wraparound delta by PlCo +0x44C (0.5), and adds
10 for the pose frame. Stick magnitude is capped at 1 and smoothed by the same
factor. Guard.c:230 onward blends the tilt pose into the shield startup pose;
the existing model-bone scale supplies shield collision geometry. The added
canonical and combat-scratch tests do not constitute a new rendered-bone oracle.

**Break and dizzy follow-up.** `ftCo_ShieldBreakFly.c:22-37` enters airborne
205, sets vertical velocity from character attributes, requests effect 1051
and intangible hurt status. The descriptor supplies Jigglypuff's top-exit flag;
common gameplay has no kind check. Fly completion remains an explicit
unimplemented 206; landing selects orientation from HipN
(`ftCo_ShieldBreakDown.c:19-35`). Face-up landing enters 207, then
209, then 211 (`ftCo_ShieldBreakStand.c:15-34`). Furafura sets shield health
to 30 and initializes `max(400 - percent, 0) + 90` timer units. Every animation
tick subtracts 1 plus the input mash contribution at scale 3; expiry returns to
Wait (`ftCo_Furafura.c:16-47`). The recording and new exhaustion test cover
205, 207, 209 and 211. The unrecorded face-down break entries remain explicit
stops.

## Assembly audit

Read-only disassembly checks used the main checkout's decomp reference because
the rebased lane's decomp directory is empty; it was not modified. Size uses
80091DAC/80091DB4 `fmadds`; drain uses 80092624; ProcessHit uses
8006D2AC/8006D2CC/8006D8D8; stun uses 80093038/8009305C and size
80093164/8009316C; tilt uses 80091C78/80091D3C. `ftColl_80076CBC`,
`ftCo_80098B20`, `ftCo_80099010`, `ftCo_Furafura_Anim`, `ftCommon_GrabMash`
and `ft_80084EEC` have no fused sites. Existing audited gravity, friction,
collision and matrix helpers are reused.

## Changed files

- `crates/melee-ft/src/fighter/shield_break.rs` (new), `shield.rs`,
  `state/common_table.rs`: exhaustion and the four common state rows.
- `crates/melee-ft/src/fighter/damage.rs` (shield function only), `smash.rs`,
  `procs.rs`: powershield color program and its advancement.
- `crates/melee-ft/src/fighter/assets.rs`, `mod.rs`, `spawn.rs`, `life.rs`:
  archive preparation, typed scratch and the motion-cleared top-exit flag.
- `crates/melee-ft/src/fighter/commands.rs`, `effects.rs`;
  `crates/melee-ef/src/{lib,request,tables}.rs`; `crates/melee-sim/src/frame.rs`:
  effect generators, sound channels and outgoing-effect ordering.
- `crates/melee-sim/src/frame/combat.rs`, `tests/{m5_gate,alloc_gate}.rs`:
  milestone assertions, enabled full gate and zero-allocation budgets.
- This report, `TRACKER.md` and the generated performance entry in `docs/PERF.md`.

## Final validation

The five release CLI commands were rerun on the final code with
`cargo run -q --release -p melee-sim -- gate harness/scenarios/<scene>.toml`.
Their exact final lines appear in the scene table above.

| Check | Result |
|---|---|
| `cargo build -q` | PASS |
| `cargo gate` | FAIL: the rebased lane's decomp source directory is empty |
| `cargo gate --no-fail-fast` | 1,098 passed, 1 failed, 3 existing ignores; the sole failure is the missing-source oracle below |
| `cargo gate --release --no-fail-fast` | Same 1,098 / 1 / 3; same sole failure |
| M4 in both workspace profiles | `261 passed; 0 failed; 0 ignored` |
| M5 in both workspace profiles | `75 passed; 0 failed; 0 ignored` |
| Allocation gate in both workspace profiles | `24 passed; 0 failed; 0 ignored`; powershield 299 measured ticks: 0 allocations; shield break 519 measured ticks: 0 allocations |
| `tools/check-release-math.sh` | PASS: both profiles and opt levels 0, 1, 2, 3, s, z |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo fmt --all -- --check` | PASS |
| `tools/perf-gate.sh` | PASS on the warm-cache rerun; first timing failure retained below and in `docs/PERF.md` |
| `tools/merge-check.sh lane/battlefield` | Data, oracle presence, ancestry and build PASS; `[FAIL] gate` at the same missing-source oracle; later steps not reached |

The environmental failure is
`melee-lb::dynamics_ref_oracle::dynamics_c_excerpts_match_decomp`, at line 168:

```text
called `Result::unwrap()` on an `Err` value: Os { code: 2, kind: NotFound, message: "No such file or directory" }
error: 1 target failed:
    `-p melee-lb --test dynamics_ref_oracle`
```

It requires `third_party/melee-decomp/src/melee/lb/lb_00F9.c` under this lane.
There is no supported external path override for this test. The user was asked
to restore the reference checkout; the protected directory was left untouched.
Other optional source-excerpt checks also need that checkout to exercise their
source comparisons. No test expectation, ignore or gate threshold was weakened.

Standalone `cargo test -p melee-sim --test m4_gate` also passed:

```text
test result: ok. 261 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 195.84s
```

### Performance census

Command: `CARGO_HOME=/private/tmp/p1-cargo-home CARGO_NET_OFFLINE=true
tools/perf-gate.sh` (the existing local dependency cache). The first run's
evidence is `target/perf/20260910T140500Z-45450`, also appended by the tool to
`docs/PERF.md`. All size and duplicate-definition limits pass, including one
definition for each of the 303 common-shell labels. Totals are informational:

| Compiling crate | Labels | Definitions | Duplicate labels |
|---|---:|---:|---:|
| melee-ft | 879 | 914 | 20 |
| melee-sim | 102 | 129 | 7 |
| ft-captain | 43 | 66 | 1 |
| ft-falco | 44 | 67 | 1 |
| ft-fox | 44 | 67 | 1 |
| ft-fox-family | 0 | 0 | 0 |
| ft-mario | 0 | 0 | 0 |
| ft-mars | 44 | 67 | 1 |
| ft-peach | 44 | 67 | 1 |
| ft-purin | 43 | 66 | 1 |
| ft-yoshi | 45 | 68 | 1 |

Total definitions: **1,511**. Cross-crate duplicate labels: **100**, equal to
the reviewed S3 ceiling. No `Fighter<C>` shell, new common callback
specializations or relaxed budgets were introduced.

The first timing measurement failed. During its benchmark, `os.getloadavg()`
reported `(386.46, 183.74, 94.83)` on a 12-CPU host; after it finished the
one-minute average fell from 320.80 to 182.25. This supports host contention as
a cause, but does not turn a failed timing gate into a pass. Exact final line:

```text
[REGRESSION] perf-gate: 3620912 stripped bytes, 3309568 text bytes; load 296.516 ms; ticks_600 56.169 ms
```

The warm-cache rerun at `2026-09-10T14:10:13+00:00` passed, with unchanged
code, thresholds, size and census. Its initial build took 0.07 seconds; the
load average during collection had fallen to `(117.67, 165.46, 109.07)`.
Evidence: `target/perf/20260910T140939Z-51950`. Exact final line:

```text
[PASS] perf-gate: 3620912 stripped bytes, 3309568 text bytes; load 172.185 ms; ticks_600 24.185 ms
```

Full acceptance still requires restoring the lane's decomp reference externally
and rerunning `cargo gate`, `cargo gate --release`, and
`tools/merge-check.sh lane/battlefield`. The reference was still absent at the
final check. No code/test failure or retail-vs-test contradiction was found.
