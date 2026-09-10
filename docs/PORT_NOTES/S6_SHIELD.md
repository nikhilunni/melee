# S6: shield verification and cross-lane boundaries

2026-09-10, `lane/battlefield`. **Acceptance incomplete.** Three complete scenes
were already exact on entry. This change adds their canonical/particle gates,
raw combat-scratch regressions, two zero-allocation gates, and exact prefixes
for the two blocked scenes. No production gameplay code was changed.

The task explicitly assigns `damage.rs` to S5 and Marth specials to S3, and
requires stopping when either is needed. Those boundaries are retained. All
five full acceptance gates are added and enabled; two fail visibly. No existing
test, expected word, ignore, allocation ceiling or comparator was changed.
There are no new hooks, motion rows, generic fighter instantiations or tick-path
allocations. No Git write command, commit or protected-path write was performed.
The decomp symlink type-change predates this task.

## Scene evidence

Tick ordinals below come from the current local recordings. Startup movement is
omitted from the table: Marth starts Wait 14, then Dash 20@31, Run 21@46 and
RunBrake 23@87. In the four tilt scenes it walks (15@113), attacks (53@119), and
returns to Wait (14@159). Every prefix compares both fighters and RNG, not only
Fox. The additional prefix test also checks ordered particle callsites.

| Scene | Marth attack states | Fox state sequence | First-divergence history | Final scope |
|---|---|---|---|---|
| `shieldstun_ftilt_fd_marth` | 53@119 → 14@159 | 14 → 182@111 → 179@119 → 181@125 → 179@137 → 180@171 → 14@186 | Already exact; no gameplay changes | 300 ticks exact |
| `shieldtilt_ftilt_fd_marth` | 53@119 → 14@159 | 14 → 182@111 → 179@119 → 181@125 → 179@137 → 180@171 → 14@186 | Already exact; no gameplay changes | 300 ticks exact |
| `lightshield_ftilt_fd_marth` | 53@119 → 14@159 | 14 → 178@111 → 179@119 → 181@125 → 179@138 → 180@171 → 14@186 | Already exact; no gameplay changes | 300 ticks exact |
| `powershield_ftilt_fd_marth` | 53@119 → 14@159 | 14 → 182@124 → 181@125 → 179@137 → 180@171 → 14@186 | Tick 125: `damage.rs:244`, `ftColl_80076CBC: powershield contact` | 125 exact ticks, 0..124; full gate fails |
| `shieldbreak_fd_marth` | 341@119 → 342@130 → 343@216 → 14@256 | 14 → 182@111 → 179@119 → 205@220 → 207@257 → 209@283 → 211@313 | Tick 119: `p0.cur_anim_frame`, expected 1 (`0x3F800000`), actual 6 (`0x40C00000`) | 119 exact ticks, 0..118; full gate fails |

182 is GuardReflect, 178 GuardOn, 179 Guard, 181 GuardSetOff and 180 GuardOff.
The break recording stays Furafura through its last tick, 519. It does not
exercise ShieldBreakFall 206. These later break states are recording evidence,
not a claim that the simulator reached them.

The requested release CLI commands report **62 keys**, because the existing
CLI calls `gate_items` whenever item records exist, including empty item arrays.
Its successful line is `300 ticks, 62 keys, 0 divergences`. The M5 helper calls
the existing 49-key fighter/RNG gate and additionally verifies particle order.
The comparator was not changed to manufacture the requested 49-key CLI text.

## Exact work needed across the lane boundaries

At tick 125, powershield reaches `record_shield_hit` in S5-owned
`crates/melee-ft/src/fighter/damage.rs`. Retail `ftcoll.c:450-508`
(`ftColl_80076CBC`) records attacker hitlag/pushback and defender impact even
during powershield, but omits accumulated shield damage and ordinary spark
1052. Its powershield branch clears minimum hold, grants the GuardOff attack
interrupt window (`ftCo_80094138`), requests effect 27, and plays sound 104.
This branch must be ported by the owner or reassigned before this scene can
advance. Removing the stop alone would produce the wrong health and effects.

After that dependency, S6 must finish the separate existing stop in
`shield.rs::take_shield_hit`: powershield keeps shieldstun, omits effect 1049,
and omits the ordinary 0.6 pushback multiplier (Guard.c:659-717).
The recorded tick-125 Fox ground velocity is `1.2100000381469727`, compared
with `0.7260000705718994` for ordinary shielding. Its health is
`59.720001220703125`: ordinary frame drain happened, contact damage did not.
Both have animation rate `3.3223140239715576` and six hitlag frames.

Projectile powershield is a separate path: Guard.c:859-887 builds the reflect
volume and responds via `ftCo_80093790`; `fighter.c:2941-2947` routes reflect
overflow to shield break or invokes the callback. The current reflect response
and item shield response remain explicit port gaps. None of these five scenes
contains a projectile, so they cannot certify projectile reflection.
The loaded descriptor uses radius 0.75, damage multiplier 0.5, speed multiplier
0.7 and current shield health as maximum damage. The reflect timer starts at 1
and the physical powershield timer at 3; Guard.c:980-1002 expires each only
after its decremented value becomes negative.

Shield Breaker entry is missing from the checked-out Marth implementation.
The first divergence at tick 119 is Marth's animation, before Fox's break at
220. Under the task's explicit fallback, the enabled prefix test stops before
119; it does not drive Fox using later oracle state, fabricate a Marth special,
or claim coverage of Fox's break/down/stand/dizzy path. S3 must supply the
341 → 342 → 343 sequence before that full-scene port can be validated.

## Shield formulas and analog mapping

Sources below are the current pinned decomp under
`third_party/melee-decomp/src/melee/ft/`. Numbers are decoded from the local
PlCo `ftLoadCommonData` slot 0; decimal constants denote their stored `f32`
values. These explanatory formulas preserve the implementation's rounding
boundaries; `fma(a,c,b)` denotes the Gekko fused operation in that order.
Use the loaded parameters, not these decimal summaries, in production code.

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
break response. Those exhaustion branches are not implemented in this task.

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
and intangible hurt status. Jigglypuff's branch must use a character hook.
Fly completion enters 206; landing selects 207/208 from the existing HipN
orientation test (`ftCo_ShieldBreakDown.c:19-35`). Down completion enters
209/210, then 211 (`ftCo_ShieldBreakStand.c:15-34`). Furafura sets shield health
to 30 and initializes `max(400 - percent, 0) + 90` timer units. Every animation
tick subtracts 1 plus the input mash contribution at scale 3; expiry returns to
Wait (`ftCo_Furafura.c:16-47`). None of this post-blocker behavior is claimed as
implemented or validated by this change.

## Assembly and architecture audit

Read-only `python3 harness/asm.py <symbol> --fused` checks confirmed:

- Size: `ftCo_80091D58`, 80091DAC/80091DB4 `fmadds`.
- Drain: `ftCo_800925A4`, 80092624 `fmadds`.
- Health/attacker push: `Fighter_ProcessHit_8006D1EC`,
  8006D2AC/8006D2CC/8006D8D8 `fmadds`.
- Stun/size: `ftCo_80092F2C`, 80093038/8009305C and 80093164/8009316C.
- Tilt: `ftCo_80091BC4`, 80091C78/80091D3C `fmadds`; the square-root Newton
  steps retain the double `fnmsub` operations at 80091CE0/CF0/D00.
- Trigger initialization and `ftColl_80076CBC` have no fused instructions.

The existing 178..182 rows remain in the single concrete `state::COMMON`.
They dispatch through installed phase callbacks and existing shield hooks in
the static `CharacterTable`. No alternate callback dispatch or character-kind
branch was added. Hitboxes remain supplied by melee-cmd/melee-coll.

## Validation

The complete final commands used `--no-fail-fast` on both workspace profiles
so the two known S6 failures did not hide later suites or doctests. Both finish
with **1,065 passed, 2 failed, 4 pre-existing ignored**. Only the new full
powershield and shield-break gates fail. All pre-existing suites pass unchanged.
M5 is **59 passed / 2 failed / 1 pre-existing ignored** in both profiles.
The four existing ignores are two raw throw probes, the S3-blocked Dolphin
Slash scene, and the schema doctest. No new ignore was added.

| Command | Result |
|---|---|
| Each requested release scene CLI | Three exact 300-tick scenes; powershield stops at 125; break diverges at 119 |
| `cargo gate --no-fail-fast` | 1,065 passed / 2 known failures / 4 existing ignores |
| `cargo gate --release --no-fail-fast` | Same results |
| `cargo test -p melee-sim --test m4_gate` | 261 passed |
| Allocation suite, both workspace profiles | 16 passed; both S6 scenes zero allocations |
| Three new raw-scratch tests, both profiles | All passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `tools/check-release-math.sh` | PASS |
| `cargo fmt --all -- --check` | PASS, no output |
| `tools/merge-check.sh lane/battlefield` | BLOCKED at ancestry prerequisite |

Exact final M5 lines, debug then release:

```text
test result: FAILED. 59 passed; 2 failed; 1 ignored; 0 measured; 0 filtered out; finished in 34.86s
test result: FAILED. 59 passed; 2 failed; 1 ignored; 0 measured; 0 filtered out; finished in 5.40s
```

Exact standalone M4, clippy and math final lines:

```text
test result: ok. 261 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 105.87s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 33s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Both new allocation checks, in both profiles:

```text
lightshield_ftilt_fd_marth: 299 measured ticks; simulate-only 0 (0.000000/tick), peak 0, allocating ticks 0; with snapshot 37375 (125.000000/tick), peak 125; snapshot overhead 37375 (125.000000/tick)
shieldtilt_ftilt_fd_marth: 299 measured ticks; simulate-only 0 (0.000000/tick), peak 0, allocating ticks 0; with snapshot 37375 (125.000000/tick), peak 125; snapshot overhead 37375 (125.000000/tick)
```

Exact release prefix lines:

```text
powershield_ftilt_fd_marth: 125 ticks, 49 keys, 0 divergences; cross-lane boundary prefix only
shieldbreak_fd_marth: 119 ticks, 49 keys, 0 divergences; cross-lane boundary prefix only
```

The initial merge-check also emitted a sandbox fsmonitor IPC diagnostic.
Repeating with the process-only settings `GIT_CONFIG_COUNT=1`,
`GIT_CONFIG_KEY_0=core.fsmonitor`, `GIT_CONFIG_VALUE_0=false` removed that
unrelated diagnostic and retained the same exit 1:

```text
[PASS] data: no tracked game data or protected lane changes
[PASS] data: oracle traces present
[FAIL] rebase: main is not an ancestor of lane/battlefield (or ref lookup failed)
```

No rebase or ancestry bypass was attempted. Build/test steps inside merge-check
did not run; the independent validation above is not a merged-tree pass.
The first `cargo gate` was started before additions and discovered the new
S6 tests while still running; it is not reported as a pristine baseline.
The final complete profile runs above were started after all Rust edits.

Logs: `/tmp/s6-validation.json`, `/tmp/s6-final-{debug,release,m4,clippy,math,fmt}.log`,
`/tmp/s6-focused-release.log` and `/tmp/s6-final-merge-no-fsmonitor.log`.
The first isolated perf run produced fresh timing/census evidence but exited 1:
`perf-gate: invalid or missing evidence: Extra data: line 2 column 1 (char 567)`.
Inspection found two missing `-->` delimiters in pre-existing `docs/PERF.md`
history comments (after original lines 4784 and 7070). Two earlier REGRESSION
records swallowed following Markdown and the next JSON block. Adding only the
two closing delimiters made all 29 history records parse. Every existing JSON
evidence line was verified byte-identical; no result, cap, tolerance or passing
baseline was altered. This was a report-formatting defect, not a retail-test
contradiction. The first run and its failure remain in
`/tmp/s6-final-perf.log`; its evidence directory is
`target/perf/20260910T103805Z-6246`.

The final isolated command was:

```sh
CARGO_HOME=/private/tmp/p1-cargo-home CARGO_NET_OFFLINE=true tools/perf-gate.sh
```

It exits **1 (REGRESSION)**. Duplication and size pass; load and tick timing
exceed both the previous PASS tolerance and fixed P1 caps. Exact final census:

```text
Duplicate-definition census (tolerance +0; label/definition totals informational):
  ft-captain: 1 duplicate labels; 43 labels; 66 definitions
  ft-falco: 1 duplicate labels; 44 labels; 67 definitions
  ft-fox: 1 duplicate labels; 44 labels; 67 definitions
  ft-fox-family: 0 duplicate labels; 0 labels; 0 definitions
  ft-mario: 0 duplicate labels; 0 labels; 0 definitions
  ft-mars: 1 duplicate labels; 43 labels; 66 definitions
  ft-peach: 1 duplicate labels; 44 labels; 67 definitions
  ft-purin: 1 duplicate labels; 43 labels; 66 definitions
  ft-yoshi: 1 duplicate labels; 45 labels; 68 definitions
  melee-ft: 20 duplicate labels; 817 labels; 852 definitions
  melee-sim: 7 duplicate labels; 101 labels; 128 definitions
  across crates: 99 duplicate labels
[REGRESSION] perf-gate: 3518384 stripped bytes, 3211264 text bytes; load 722.268 ms; ticks_600 133.791 ms
```

Exact timing failures:

```text
load_ns: 722267783.500 > 185636190.768 (previous 168760173.425, +10%)
ticks_600_ns: 133790725.300 > 26042917.120 (previous 23675379.200, +10%)
P1 time ceiling load_ns: 722267783.500 > 182600000
P1 time ceiling ticks_600_ns: 133790725.300 > 25947000
```

Both isolated runs built the **same byte-identical stripped executable**:
3,518,384 bytes, SHA-256
`aa6ddd8c2d04a2dc7ef44ad23a42377b6e4d7f0a6fbe428f247725ab083c6e89`.
The first run's Criterion JSON reports load **167.650 ms**, 600 ticks
**23.803 ms**; the second reports **722.268 ms** and **133.791 ms**. This rules
out a changed executable between the two measurements. It does not establish
which host resource caused the slowdown or turn the final failure into a pass.
No measurements were discarded and no threshold was loosened. A stable-host
performance rerun remains necessary for full acceptance.

Final evidence: `target/perf/20260910T104031Z-10332`,
`/tmp/s6-final-perf-repaired.log`, and the appended REGRESSION in `docs/PERF.md`.
Only the two delimiter additions alter historical report text. The new census
retains the C15 duplicate counts: **20 / 7 / 1 per compiling character / 99
across crates**; total definitions are informational (**1,447**).

The initial three successful release CLI commands each printed exactly:

```text
300 ticks, 62 keys, 0 divergences
```

Their matching M5 helper lines in `/tmp/s6-focused-release.log` each print:

```text
300 ticks, 49 keys, 0 divergences
```

The final full workspace logs end with the same nonzero-target diagnostic:

```text
error: 1 target failed:
    `-p melee-sim --test m5_gate`
```

S6 is not ready to merge: two full-scene gates, the timing gate and the
merge-check ancestry prerequisite remain unresolved.

## Changed files

- `crates/melee-sim/tests/m5_gate.rs`: one appended S6 block; five full gates
  plus one two-scene prefix test.
- `crates/melee-sim/tests/alloc_gate.rs`: one appended S6 block; tilted and
  analog shield ceilings are zero.
- `crates/melee-sim/src/frame/combat.rs`: appended calls to the unchanged raw
  scratch comparator for the three complete scenes.
- `TRACKER.md`: S6 status and session evidence.
- `docs/PERF.md`: two missing comment terminators restored without altering
  historical JSON; the performance tool appends its fresh result.
- `docs/PORT_NOTES/S6_SHIELD.md`: this report.
