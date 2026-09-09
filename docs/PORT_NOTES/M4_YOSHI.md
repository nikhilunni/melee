# Lane C: Yoshi idle, match start and movement

Implemented against the 2026-09-09 recordings, left uncommitted. The eighteen
Yoshi scenes preserve the 49-key contract: 24 fighter fields per slot plus
the produced RNG seed. Each also has an ordered particle-draw ledger test.
Archive details and the complete attribute grouping are in
[YOSHI_DATA.md](../../../../docs/YOSHI_DATA.md).

## Shared assumptions removed

| Location | Previous assumption / resulting behavior |
| --- | --- |
| `desc/playback.rs`, `fighter/{assets,procs}.rs` | Every character has a Wait-choice table. A null table now means no alternate Wait choice or choice RNG. Existing Fox readers explicitly unwrap their present table. |
| `fighter/assets.rs` | Every character has common shield-pose animations and all requested motion rows. Valid null pose tables become empty, and zero-size AJ rows remain absent. Yoshi Guard hold uses SM_None. |
| `fighter/{state,snapshot,spawn}.rs` | Shared callback identity was also the live action number. `MotionState.id` remains the semantic family, while `action_id` comes from the character hook and is emitted in Snapshot. |
| `fighter/{mod,shield}.rs` | Guard startup skipped its animation and all shield IASAs shared an order. Optional character callbacks own entry, Anim, IASA, hold and exit; defaults retain existing behavior. `animated_shield` controls only animation attachment. |
| `fighter/{mod,escape}.rs` | Spot dodge and roll only needed common entry/completion. Entry, completion and Anim hooks now retain Yoshi's egg model/body and guard scratch. Ordinary spot dodge leaves the egg before entry; roll completion can return directly to hold. |
| `fighter/{mod,jump}.rs` | Basic and Peach were the implemented aerial-jump styles. Yoshi selects always-forward animation, root-motion vertical movement, armor and a character-owned turn countdown. |
| `fighter/commands.rs`, `assets.rs` | The selected scripts did not need counted command loops or article visibility. Decode Command_03/04 with a typed loop stack and opcode 36 as article visibility. |
| `initial_state/saved_pose.rs` | Reconstructing the main motion was enough to restore independent part poses. Part-owned primary/blend joints now import their AObj/FObj stream state from the saved boundary. |
| `scene_fighter.rs`, sim `assets.rs` | OnLoad only needed fighter resources. A generic costume accessor/callback supplies material animations. Character registration itself is one registry entry plus the crate dependency. |
| `fighter/effects.rs`, sim `effects.rs` | Effects always had an AnimJoint. Static shell models permit null animation; egg-exit requests carry the outgoing bone matrix through deferred flush. Twelve fragments run the retail spawn and velocity callbacks. |

No character-kind test was added to shared gameplay. `ft-yoshi` overrides
`action_id`, `animated_shield`, `enter_shield`, `animate_shield`,
`input_shield`, `enter_guard_hold`, `enter_guard_off`, `escape_variant`,
`escape_finished`, `escape_animated`, `aerial_jump_style`,
`aerial_jump_entered`, `aerial_jump_animated`, `on_costume_loaded`,
`on_resources_loaded` and the explicit egg hurtbox interaction boundary.
The pre-existing unported Samus escape branch remains explicit.

## Recorded behavior and corrections

Start matches the supplied sequence, with no timing constants added:

| Tick | Yoshi action |
| ---: | --- |
| 0 | 322 Entry |
| 6 | 323 EntryStart |
| 35 | 324 EntryEnd |
| 65 | 29 Fall |
| 80 | 42 Landing |
| 110 | 14 Wait |

`jump_fd_yoshi` enters 27 JumpAerialF at tick 113, Y approximately
10.6470985, and 32 FallAerial at tick 183. `airjumpb_fd_yoshi` also uses
27, at tick 51, and enters 32 at 121; the reversal rotates the root and
flips facing halfway through twelve frames. Both scenes pin armor against
the raw retail Fighter word +18B4 on all 300 ticks.

The task's shield numbering misidentifies **341 as GuardSetOff**. The decomp
and recording show 341 = GuardOn_0, 342 = GuardHold, 343 = GuardOff,
344 = GuardDamage, and 345 = GuardOn_1 (Reflect startup). Common action
numbers are 178/179/180/181/182 respectively.

| Scene | Observed Yoshi transitions (`tick:action`) |
| --- | --- |
| shield | `31:345, 37:342, 92:343, 108:14` |
| spotdodge | `31:345, 35:235, 57:341, 63:342, 82:343, 98:14` |
| roll | `31:345, 35:233, 69:342, 82:343, 98:14` |

The roll returns directly to hold; it does not take 341. Ordinary startup
retains the prior reflect/powershield countdown words. Roll's interrupt
integer overwrites the same retail union word later used as guard elapsed;
the port preserves that word through typed retained guard state. Raw-ledger
tests compare health, elapsed, minimum hold, reflect and powershield timers.

Idle has no Yoshi Wait-choice draw. The supplied ledge scenes and backward
jump pad scripts are replayed unchanged, including their recorded drift.
No scenario-specific tick branches or future oracle state feed the runtime.

## Bone boundary and arithmetic evidence

The idle save contains independent part animations that differ from the
current main Wait motion. Their saved AObj/FObj state must be imported;
merely restoring SRT and reattaching the current motion changes later part
joints. Restoration reads the initial savestate heap only. Both primary and
blend part-owned joints retain saved stream cursors and interpolation state.

| Capture | Ticks | Local SRT words | Rendered matrix words |
| --- | ---: | ---: | ---: |
| `start_fd_yoshi.bones.jsonl` | 130 | 167,508 | 0 |
| `idle_fd_yoshi.bones.jsonl` | 8 | 10,296 | 0 |
| Total | 138 | 177,804 | 0 |

All 70 Yoshi joints and 73 Fox joints are checked. Yoshi has no dynamic
chains. As in the Marth test, unused Euler W is excluded and active
quaternion W is exact. Tick matrix caches are not rendered oracles.

Retail audits (all commands run from the repository root):

```sh
python3 harness/asm.py ftYs_JumpAerial_Enter
python3 harness/asm.py ft_800CB6EC
python3 harness/asm.py ftYs_Init_8012B8A4 --fused
python3 harness/asm.py efSync_Spawn --fused
python3 harness/asm.py efLib_Cb_SetOffset_FromParams --fused
```

`ftYs_JumpAerial_Enter` (800CBE98) uses separate horizontal multiplication
at 800CBEF8 and direction multiplication at 800CBFC4. Turning uses `fdivs`
at 800CB768 and **`fnmsubs` at 800CB76C**. Shield material frame math,
`efSync_Spawn` (8005FDDC) and shell motion (8005E950) have no fused sites.
Shell rotation retains double TAU promotion before rounding; velocity uses
audited MSL sin/cos and separately rounded multiplies. Spawn chooses twelve
shell variants with three random draws each, then drains the animation queue
in reverse creation order. Idle/start particle draw sites remain unchanged.

## Validation

Commands ran from the lane root on 2026-09-09.
All local data and captures were present; oracle runs were not asset skips.

| CLI command | Literal final line |
| --- | --- |
| `cargo run -q -p melee-sim -- gate harness/scenarios/idle_fd_yoshi.toml` | `600 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/start_fd_yoshi.toml` | `600 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/squat_fd_yoshi.toml` | `300 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/turn_fd_yoshi.toml` | `300 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/walk_fd_yoshi.toml` | `300 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/walkfast_fd_yoshi.toml` | `300 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/dash_fd_yoshi.toml` | `300 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/turnrun_fd_yoshi.toml` | `300 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/jump_fd_yoshi.toml` | `300 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/airjumpb_fd_yoshi.toml` | `300 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/shield_fd_yoshi.toml` | `300 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/spotdodge_fd_yoshi.toml` | `300 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/roll_fd_yoshi.toml` | `300 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/airdodge_fd_yoshi.toml` | `300 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/wavedash_fd_yoshi.toml` | `300 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/ledge_fd_yoshi.toml` | `420 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/ledgeclimb_fd_yoshi.toml` | `420 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/ledgeescape_fd_yoshi.toml` | `420 ticks, 49 keys, 0 divergences` |

All eighteen are registered in `tests/m4_gate.rs`, together with eighteen
ordered particle-draw tests. Aggregate test counts below sum the per-binary
Cargo results; the workspace baseline was **798 passed, zero failed, one
ignored**. This change adds 46 tests: 36 scene/ledger, two bone, five raw state
and three attribute/reset tests.

| Command | Result |
| --- | --- |
| `cargo test -p melee-sim --test m4_gate` | 217 passed, zero failed |
| `cargo test -p hsd-particle` | 64 passed, zero failed |
| `cargo test -p melee-ft` | 90 passed, zero failed |
| `cargo test -p melee-sim --lib yoshi_bones -- --nocapture` | 2 passed, 177804 SRT words exact |
| `cargo test -p melee-sim --lib yoshi_state -- --nocapture` | 5 passed, all 300 ticks per scene |
| `cargo test -p ft-yoshi -- --nocapture` | 3 passed; six costume material frame counts each equal 100 |
| `cargo gate` | 844 passed, zero failed, one pre-existing ignored doctest |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0; clean |
| `cargo fmt --all` | exit 0; no output |
| `git -c core.fsmonitor=false diff --check` | exit 0; no output |

Selected literal final output:

```text
cargo test -p melee-sim --test m4_gate:
test result: ok. 217 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 50.21s
cargo test -p melee-sim --lib yoshi_bones -- --nocapture:
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 32 filtered out; finished in 2.38s
cargo test -p melee-sim --lib yoshi_state -- --nocapture:
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 29 filtered out; finished in 1.41s
cargo clippy --workspace --all-targets -- -D warnings:
Finished `dev` profile [unoptimized + debuginfo] target(s) in 14.60s
```

Logs: `/tmp/yoshi-final-scenes.log`, `/tmp/yoshi-final-checks.log`,
`/tmp/yoshi-final-{gate,m4,particle,ft,clippy,bones,state,attributes}.log`.
`/tmp/yoshi-baseline.log` records the initial workspace gate. The final gate
and the separately requested suites all exited zero. Existing test edits
only adapt Fox's present Wait table to the optional reader return type;
no expected bits, test filters or comparisons were loosened.


## Limits

Special bodies, items, egg shield damage/break, delayed egg powershield,
grabs, combat against the egg capsule and armor damage resolution stay
explicit `unimplemented!`. The armor value and egg hurtbox/model state are
retained, but no unsupported hit response is approximated. Costume material
freezes, model selections and article visibility are output state for later
rendering. No rendered Yoshi matrix oracle exists. New Yoshi particle-field
dumps are not replayed field by field; all eighteen ordered particle ledgers
and resulting scene seeds pass, alongside existing particle-field tests.

No test expected values or tolerances changed. No harness files, captures,
scenarios, ROMs or decomp source were changed. Harness Python tests were not
run because no harness code changed. No Dolphin, commits or pushes.
The lane's pre-existing decomp/ROM/trace symlinks remain in place.

## Changed files

- `Cargo.lock`
- `Cargo.toml`
- `TRACKER.md`
- `crates/ft-yoshi/Cargo.toml`
- `crates/ft-yoshi/src/attributes.rs`
- `crates/ft-yoshi/src/init.rs`
- `crates/ft-yoshi/src/lib.rs`
- `crates/ft-yoshi/src/material.rs`
- `crates/ft-yoshi/src/shield.rs`
- `crates/ft-yoshi/tests/attributes.rs`
- `crates/ft-yoshi/tests/support/mod.rs`
- `crates/melee-ft/src/anim/playback.rs`
- `crates/melee-ft/src/desc/playback.rs`
- `crates/melee-ft/src/fighter/M4_YOSHI.md`
- `crates/melee-ft/src/fighter/README.md`
- `crates/melee-ft/src/fighter/assets.rs`
- `crates/melee-ft/src/fighter/commands.rs`
- `crates/melee-ft/src/fighter/damage.rs`
- `crates/melee-ft/src/fighter/effects.rs`
- `crates/melee-ft/src/fighter/escape.rs`
- `crates/melee-ft/src/fighter/jump.rs`
- `crates/melee-ft/src/fighter/mod.rs`
- `crates/melee-ft/src/fighter/procs.rs`
- `crates/melee-ft/src/fighter/shield.rs`
- `crates/melee-ft/src/fighter/snapshot.rs`
- `crates/melee-ft/src/fighter/spawn.rs`
- `crates/melee-ft/src/fighter/state.rs`
- `crates/melee-ft/tests/idle_ground_fields_600.rs`
- `crates/melee-ft/tests/real_fox_wait_playback.rs`
- `crates/melee-sim/Cargo.toml`
- `crates/melee-sim/src/assets.rs`
- `crates/melee-sim/src/effects.rs`
- `crates/melee-sim/src/effects/egg_shell.rs`
- `crates/melee-sim/src/frame.rs`
- `crates/melee-sim/src/frame/yoshi_bones.rs`
- `crates/melee-sim/src/frame/yoshi_state.rs`
- `crates/melee-sim/src/initial_state/saved_pose.rs`
- `crates/melee-sim/src/scene_fighter.rs`
- `crates/melee-sim/tests/m4_gate.rs`
- `docs/YOSHI_DATA.md`
