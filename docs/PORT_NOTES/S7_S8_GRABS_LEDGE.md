# S7/S8: throw entry, pummel entry, grab mash and quick ledge options

2026-09-10, `lane/chars`; uncommitted. **Acceptance incomplete.** Four complete scenes are exact; the other four stop in S5-owned `damage.rs`. Those files were left untouched under the task's lane boundary. Full acceptance tests remain enabled, with separate exact-prefix tests. No expected values, existing ignores, scenarios, traces, ROMs or decomp files were changed.

## Scene evidence

Ticks are zero-based trace ordinals. All recordings have 300 records. Startup movement is omitted below. History lists observed failures during this implementation; no expected values were changed.

| Scene | Retail states after grab/ledge wait | First-divergence history | Final outcome |
|---|---|---|---|
| `fthrow_fd_marth` | Marth 219@141 → 14@164; Fox 239@141 → 86@151 → 29@181 → 0@221 | initial ThrowF input stop at 141; after port, S5 airborne-hit guard at 151 | 151 exact ticks; full gate blocked |
| `uthrow_fd_marth` | Marth 221@141 → 14@174; Fox 241@141 → 90@150 → 183@184 → 184@210 | initial 141 `p0.cur_anim_frame` (12 vs 1.3333334); then graphics 0x3FA; then S5 airborne-hit guard at 150 | 150 exact ticks; full gate blocked |
| `dthrow_fd_marth` | Marth 222@141 → 14@173; Fox 242@141 → 90@151 → 183@166 → 184@192 | initial 141 `p0.cur_anim_frame` (12 vs 1.3333334); then S5 airborne-hit guard at 151 | 151 exact ticks; full gate blocked |
| `pummel_fd_marth` | Marth 217@135 → 216@162 → 218@206 → 14@236; Fox 228@141 → 227@164 → 229@206 → 14@236 | initial CatchAttack input stop at 135; then captured-victim hit guard at 141 | 141 exact ticks; full gate blocked |
| `grabmash_fd_marth` | Marth 218@138 → 14@168; Fox 229@138 → 14@168 | initial 132 `p1.cur_anim_frame` (4 vs 5); timer/mash playback and reciprocal cut resolve it | 300 ticks exact |
| `ledgeattack_fd_fox` | 257@231 → 14@285 | initial CliffAttack input stop at 231; new row and shared ledge physics resolve it | 300 ticks exact |
| `ledgejump_fd_fox` | 262@231 → 263@245 → 42@282 | already exact before changes | 300 ticks exact |
| `ledgeroll_fd_fox` | 259@231 → 14@280 | already exact before changes | 300 ticks exact |

The current CLI includes item fields and prints `300 ticks, 62 keys, 0 divergences` for these recordings. `m5_gate` independently checks all 49 fighter/RNG keys and ordered particle RNG sites. The CLI/comparator was not modified to force a 49-key label. Exact-prefix tests also check the recorded items and particle order.

## S5 handoff: do not merge this as complete

- `damage.rs`, `FighterCore::prepare_damage_reaction`: its airborne-entry guard only accepts `ThrownB`. It stops forward/up/down release at 151/150/151 respectively. Retail `ftCo_8008DCE0` must accept these airborne thrown states and select DamageAir3 for the forward throw. Existing `DownBoundU`/`DownWaitU` rows are present, but the release guard prevents reaching them.
- Down throw additionally passes an animation-selection angle of 90 from `ftCo_800DE7C0` while the knockback descriptor retains the actual launch angle. The present `begin_damage_reaction(ReceivedHit, assets)` interface has no independent animation-angle override. Preserve that separation when extending the S5 entry API; do not change the throw descriptor to fabricate the animation choice.
- `damage.rs`, `detect_eligible_hit`: pummel contact at 141 stops because CaptureWaitLw is outside its supported victim-state list. Retail needs captured-hit handling and CaptureDamageLw (228) entry, preserving the pair and timer, with the retail hitlag/percent/effect behavior. `ftCo_0DC2.c` supplies that state's timer and completion callbacks. It is not installed here because execution stops before contact acceptance.
- The existing ignored raw ThrowB hitbox-phase tests are unchanged. This task does not claim to resolve that separate parked mismatch.

## Ownership and implementation

New common rows are appended in one S7/S8 block: ThrowF/Hi/Lw, ThrownF/Hi/Lw, CatchAttack, CatchCut, CaptureCut and CliffAttackQuick. Existing common callbacks are reused; new behavior lives in `grab_escape.rs` and `ledge.rs`. There are no new generic fighters, generic phase callbacks, kind checks or character hooks. The existing `throw_variant` hook retains character-specific throw boundaries.

`grab_throw.rs` describes each throw using static captor/victim state, victim animation and weight-mask data. The shared pair traversal selects the descriptor in retail priority order, constrains all thrown states and releases through the existing damage API. Motion remapping now borrows the source animation and part tables with a fixed 140-entry stack mask. Asset command streams are shared immutable slices prepared at initialization. This removes the old throw-entry motion, remap and command-stream heap clones. The XRotN constraint slot is prepared with the fighter skeleton and retained after release, removing the remaining BTreeMap node allocation. The existing back-throw scene now has a zero-allocation regression gate alongside the two requested new scenes.

`grab_escape.rs` owns typed capture scratch and archive-derived timer/mash/cut parameters. The initial timer includes percent, handicap and current stock rank; the saved handicap is imported from StaticPlayer +4B, and stock ties share rank. Button edges cost one decrement even with several buttons; a change in either axis costs one additional decrement. Neutral retains the last signed direction. CaptureWait accelerates animation during a mash window; the victim-owned timer causes reciprocal CatchCut/CaptureCut before overlap. Two focused tests cover timer modifiers and stick/button counting. Pummel entry takes priority over throws and its completion returns to CatchWait without a second capture-flash request.

The 0x3FA up-throw graphics command uses the existing typed command/effect pipeline and retail model 0x15, prepared in the effect pool. Ledge attack uses ordinary subaction hitboxes and the existing ledge climb physics/collision. The recorded jump and roll callbacks required no behavior changes. Slow ledge options, airborne cut, C-stick throws, jump escape and unsupported character throw callbacks are outside the verified scene slice.

## Assembly and environment

The lane's `third_party/melee-decomp` directory is empty. Source and split retail assembly were read from `/Users/nikhilunni/Projects/melee/third_party/melee-decomp`; the existing main-checkout `harness/asm.py` was run read-only. Neither decomp path was modified. The baseline workspace gate fails `dynamics_c_excerpts_match_decomp` with ENOENT at `melee-lb/tests/dynamics_ref_oracle.rs:168`.

New arithmetic audited: `fn_800DA8E4` uses fmadds at 800DA9CC/800DA9D4, with a separately rounded rank term and intervening addition. `ftCommon_GrabMash`, CaptureWait animation, CatchAttack entry/completion, CatchCut entry, CaptureCut entry/physics and CliffAttack entry contain no fused sites. Throw weight scaling retains the existing audited separate multiply/divide. Existing thrown-pose and ledge FMA sites remain unchanged. Graphics 0x3FA follows ftCo_8009F834's existing audited randomized-offset block and efAsync_Dispatch's positional model branch.

## Final validation

Commands use the checked-out uncommitted tree. `--no-fail-fast` lets workspace validation continue after the known blockers; it does not skip any test.

| Command | Result |
|---|---|
| `cargo gate --no-fail-fast` | DEBUG_RESULT_PENDING |
| `cargo gate --release --no-fail-fast` | 1,065 passed, 5 failed, 4 ignored across 180 unit/integration/doc targets; only `dynamics_ref_oracle` and `m5_gate` fail |
| `cargo test -p melee-sim --test m4_gate` | 261 passed, 0 failed |
| M5 in both workspace profiles | M5_RESULT_PENDING |
| `cargo test --release -p melee-sim --test alloc_gate` | 17 passed, 0 failed; new mash, ledge attack and existing back-throw scene each allocate zero during simulate-only ticks |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo fmt --all --check`; `git diff --check` | PASS |
| `tools/check-release-math.sh` | PASS: math suite in debug/release and fused math at opt levels 0, 1, 2, 3, s, z |
| `python3 -m unittest discover -s tools/tests -p test_perf_gate.py` | 11 passed |
| `tools/perf-gate.sh` | PERF_RESULT_PENDING |
| `tools/merge-check.sh lane/chars` | Stops before build at ancestry check; no Git writes performed |

Exact CLI output for each of `grabmash_fd_marth`, `ledgeattack_fd_fox`, `ledgejump_fd_fox`, `ledgeroll_fd_fox`:

```text
300 ticks, 62 keys, 0 divergences
```

Forward/up/down throw CLI exits 101 with:

```text
not implemented: ftCo_Damage.c:346: airborne hit
```

Pummel CLI exits 101 with:

```text
not implemented: ftColl_80079AB0: crouch/other damage modifiers outside idle victim
```

Exact M4 and allocation test summaries:

```text
test result: ok. 261 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 202.14s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.65s
```

Exact release prefix output (`m5_gate s7_throws_and_pummel_before_s5_damage_boundaries -- --nocapture`):

```text
fthrow_fd_marth: 151 ticks, 49 keys, 0 divergences; S5 damage boundary prefix
uthrow_fd_marth: 150 ticks, 49 keys, 0 divergences; S5 damage boundary prefix
dthrow_fd_marth: 151 ticks, 49 keys, 0 divergences; S5 damage boundary prefix
pummel_fd_marth: 141 ticks, 49 keys, 0 divergences; S5 damage boundary prefix
```

The release workspace ends with:

```text
error: 2 targets failed:
    `-p melee-lb --test dynamics_ref_oracle`
    `-p melee-sim --test m5_gate`
```

`merge-check` ends with:

```text
[PASS] data: no tracked game data or protected lane changes
[PASS] data: oracle traces present
[FAIL] rebase: main is not an ancestor of lane/chars (or ref lookup failed)
```

Raw command output is in `/tmp/s7s8-gate-{debug,release}-final.log`, `/tmp/s7s8-m4-debug.log`, `/tmp/s7s8-alloc-final.log`, `/tmp/s7s8-clippy-verified.log`, `/tmp/s7s8-release-math.log`, `/tmp/s7s8-perf-parser-tests.log`, `/tmp/s7s8-merge-check.log`, and `/tmp/s7s8-cli-<scene>.log`.

## Changed files

| Files | Change |
|---|---|
| `crates/melee-ft/src/fighter/grab_escape.rs` (new) | Capture scratch, timer/mash, pummel and cut callbacks; timer/input tests |
| `crates/melee-ft/src/fighter/grab.rs`, `grab_throw.rs`, `ledge.rs` | Capture initialization, static throw descriptors and borrowed victim motions, quick ledge attack |
| `crates/melee-ft/src/fighter/state/common_table.rs` | One appended S7/S8 row block |
| `crates/melee-ft/src/fighter/mod.rs`, `spawn.rs`, `assets.rs` | Typed scratch, saved handicap, prepared constraint storage and immutable command assets |
| `crates/melee-ft/src/anim/attach.rs`, `playback.rs` | Borrowed motion remapping; existing owned API preserved |
| `crates/hsd-anim/src/jobj.rs` | Prepared constraint slots survive activation/deactivation |
| `crates/melee-ft/src/fighter/effects.rs`; `crates/melee-ef/src/lib.rs`, `pool.rs`, `tables.rs` | Up-throw graphics model 0x15 through the existing command/effect pipeline |
| `crates/melee-sim/src/frame.rs`, `frame/grab_pairs.rs`, `initial_state/mod.rs` | Pair escape order, throw dispatch/geometry, stock rank and handicap |
| `crates/melee-sim/tests/m5_gate.rs`, `alloc_gate.rs` | Eight full acceptance scenes, four exact prefixes, three zero-allocation scenes |
| `tools/perf_report.py` | Keep the concrete-pair census tracking the renamed throw helpers; no budget changes |
| `docs/PERF.md` | Repair two pre-existing missing JSON comment terminators; append generated performance evidence |
| `TRACKER.md`, this report | Partial completion and exact S5 handoff |
