# S4: item engine and Fox laser

The S4 lane adds the concrete item engine, Fox/Falco item implementations and shared SpecialN motion rows. The complete acceptance sequence passes in both profiles; this lane remains uncommitted.

## Crate layout and dispatch

- `melee-it`: owned `SpawnItem`, fixed item storage, `ItemCore`, archive descriptors, shared `melee-cmd` script timing, `melee-coll` capsules, `melee-mp` collision, static state/event rows. No dependency on `it-foxlaser` or fighter character crates.
- `it-foxlaser`: Fox/Falco lasers share the two-row `it_803F67D0` state table; Fox/Falco blasters share the separate eleven-row `it_803F6CA8` table. The blaster table is not the laser table.
- `ft-fox-family`: `FoxFamily` constants/accessors and typed `SpecialNeutral`; six `rows::<C>()` for actions 341..346, animations 295..300. Fox and Falco implement the trait independently. Falco constants compile but its SpecialN rows have no scene oracle in this task.
- `melee-sim::scene_items`: one `item_kinds!` inventory closes over the four implemented kinds. The engine dispatches static `ItemLogicRow` data by generated `ItemKind`.

`ItemStateRow` contains an animation id and animation/physics/collision function pointers with separate phase context types. `ItemLogic` mirrors the per-kind row, including spawn/destruction, pickup/drop/throw, damage dealt/received, clank/reflection/absorption, shield bounce/hit and owner removal. Event callbacks default to no reaction. Per-kind scratch is typed, and the simulation uses no trait objects for item gameplay.

`SpawnItem` preserves the meanings of the retail 0x4C descriptor (`it/types.h:689`), replacing GObj owner references with player slots. It is an owned host representation, not a claim of ABI-compatible host pointers.

## Pool and list evidence

Retail `Item_80266FCC` (`item.c:133-143`) maps `ItemCommonData` offsets 0x00,0x04,0x08,0x0C,0x10,0x18,0x1C,0x20,0x24,0x28 to hold categories 0,1,2,9,10,7,5,12,11,3. The loader reads these admission limits. Category 8, used by lasers and blasters, has **no retail admission bound** (`Item_8026784C`, `item.c:456-529`); retail allocation can grow. Consequently a fixed pool sized solely from those limits cannot be justified for this scene. The port uses an explicit checked capacity of 128, distinct from retail's limits, and fails on exhaustion. Storage is reserved once on the heap; a capacity assertion precedes every insertion, so the item buffer cannot grow during a tick. This avoids large inline construction temporaries overflowing Rust test-thread stacks. The scene also owns effect storage in an initialization-time box, keeping cold-start constructor frames within the default test stack; no test-stack override is used.

`Item_8026862C` creates ITEM class 6, p-link 9, priority 0 (`item.c:958`); equal-priority GObjs append in `gobjplink.c`. The recording follows this discipline: blaster 74/owner 0 at frame 31, laser 54/owner 0 appended at frame 42, laser removed at frame 57, blaster removed at frame 71. Survivor order is stable. The task's list-order stop condition was not triggered.

## Scheduler and exact arithmetic

`item.c:991-1000` registers item processes at s-links 0,1,4,5,9,11,12,13,14,16, all priority 0. The simulator creates actual item GObjs and tagged processes at spawn; its GObj/proc slabs and free lists are provisioned before ticking. Item animation runs at 1, physics at 4, environment collision at 5, blaster muzzle effects at 9, capsule refresh at 11, and damage callbacks at 14. Fighter collision records dealt damage at fighter link 13; `Item_8026A294` consumes it at item link 14, then destruction removes the GObj. Particle passes remain at 15 and percent display at 17.

`Item_802697D4` applies velocity, nudge, environment movement and platform movement in retail order. Spawn collision (`Item_80267130` -> `it_80275E98` -> `it_80276100` -> `mpColl_800471F8`) is required even in free space: subdivision/reintegration rounds the owner-to-muzzle displacement. `ftLib_80086990` supplies the fighter ECB midpoint as the starting position, with separately rounded add/multiply/add operations. Omitting either step caused the one-ULP spawn mismatches found and fixed during this port. The laser script's frame-one capsule clear immediately refreshes surviving capsules, followed by link 11, explaining captured hitbox state 3 on the first visible tick.

## Compared item keys

The existing `trace::gate` contract and M4 assertions remain the 49 fighter keys. The CLI and new laser test use `trace::gate_items`, which additionally compares the recorded item list. This explicit extension avoids changing legacy fighter-only scenes into unported stage-item gates (Yoshi's Story records Shy Guys). The item schema adds `items.count` and these 12 keys per list entry: `kind`, `owner`, `pos.x`, `pos.y`, `pos.z`, `vel.x`, `vel.y`, `vel.z`, `facing_dir`, `motion_id`, `life_timer`, `hitbox0.state`. Thus the reported schema count is 62; repeated item entries use the same schema in list order. Float values compare bit patterns. Owner is the recorded player slot or null; missing owner data is rejected. Item divergences use the same frame/path/expected/actual report as fighter divergences. A regression test covers order, missing owners, list length and a one-ULP change.

The existing generated `melee-types::ItemKind` was regenerated into temporary output and checked byte-identical; all 238 recorded ids match `harness/item_kinds.py`. No harness or decomp files were changed.

## Effects and sounds

Muzzle effect 0x48E routes to model 0xBBD, Fox effect table row 5 (`gfx_id % 1000`), with opcode-selected common/Fox particle banks. The effect belongs to the blaster owner and expires on gun removal. Hit sparks use the existing normal spark path plus the conditional second draw at `ftColl_80078538+0x94`; existing percent-shake draws run after particles. Fox fire sounds are 110103/110106, Falco 100099/100102. `it_802AE538` emits the opening sound once per item (Fox 0x1AE05, Falco 0x186F1); `it_802ADDD0` emits the holster sound on each visibility transition to 2 (Fox 0x1AE14, Falco 0x18700). Item sound requests drain into the same headless request sink as fighter sounds, with volume 127 and pan 64. A behavior test checks repeated opens, closing/reopening, and repeated visibility requests. Sound playback itself is not a recorded item/fighter key.

## Coverage boundaries

The 300-tick Fox scene is the item/fighter oracle. Other recorded stage items are not part of the legacy M4 fighter oracle. Falco and aerial SpecialN rows compile; this task has no recorded Falco laser or aerial laser scene. Reflection, absorption and shield-bounce callbacks exist, but the scene does not exercise them; broader item shield/event routing remains explicit unsupported scope. Item hitbox transforms currently support the root bone; non-root item hitbox bones fail explicitly. Blaster recoil/open counters are modeled, while visual-only gun joint animation is not evaluated. The external-owner blaster row 10 remains explicitly unsupported. Muzzle coordinates and all replayed particle simulation fields are verified separately from those visual-only gun joints.

Validation is against this lane based on `cd6cfada40bc577865ae540a34aa34faf12d28f3`; main's later P1 work was not rebased or merged here. No commits, git write commands, or changes to the protected harness/decomp paths were made. The pre-existing decomp symlink type change is not part of S4.

## Validation

The final acceptance commands exited 0 on 2026-09-10. `MELEE_ALLOW_MISSING_DATA` was unset; no test-stack override was used. Counts for multi-target commands are sums of their printed test-result lines.

| Check | Debug | Release |
|---|---:|---:|
| Laser CLI | 300 ticks, 62 keys, 0 divergences | 300 ticks, 62 keys, 0 divergences |
| M4 | 261 passed | 261 passed |
| M5 (8 existing + laser) | 9 passed | 9 passed |
| Requested package suites | 176 passed | 176 passed |
| Allocation gate | 6 passed | 6 passed |
| Full workspace | 986 passed, 0 failed, 3 existing ignored | 986 passed, 0 failed, 3 existing ignored |

The unchanged ignores are two pre-existing raw throw/tech scratch tests (`back_throw_and_tech_match_retail_scratch`, `capture_back_throw_and_missed_tech_match_retail_scratch`) and the schema documentation example. No new ignore, missing-data opt-out, expected-value edit, or looser float comparison was introduced.

The allocation ceilings remain 17/1/12/690/17 for start FD/idle FD/Marth jab/Marth KO/start BF. Laser measures 32 across 299 counted ticks in both profiles (peak 4, 19 allocating ticks). Its fixed item and scheduler storage is reserved during initialization. The allocation test reports the existing fighter snapshot overhead separately; strict item comparison is also diagnostic work outside the simulate-only count.

`tools/check-release-math.sh` passes the native math suites in both profiles and all three fused tests at each of optimization levels 0, 1, 2, 3, s and z. `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all` and `cargo fmt --all --check` pass. The three requested crates are listed in CLAUDE.md's repository map.

### Exact command output

For package/workspace commands, Cargo's last printed test-result line belongs to the final documentation target; the aggregate counts are listed above.

```text
$ cargo run -q -p melee-sim -- gate harness/scenarios/laser_fd_fox.toml
300 ticks, 62 keys, 0 divergences
```

```text
$ cargo test -p melee-sim --test m4_gate
test result: ok. 261 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 165.36s
```

```text
$ cargo test -p melee-sim --test m5_gate
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.08s
```

```text
$ cargo test -p hsd-particle -p melee-ft -p melee-it -p it-foxlaser -p ft-fox-family -p ft-fox -p ft-falco
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

```text
$ cargo test -p melee-sim --test alloc_gate -- --nocapture
laser_fd_fox: 299 measured ticks; simulate-only 32 (0.107023/tick), peak 4, allocating ticks 19; with snapshot 37407 (125.107023/tick), peak 129; snapshot overhead 37375 (125.000000/tick)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.01s
```

```text
$ cargo gate
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

```text
$ cargo run -q -p melee-sim --release -- gate harness/scenarios/laser_fd_fox.toml
300 ticks, 62 keys, 0 divergences
```

```text
$ cargo test -p melee-sim --test m4_gate --release
test result: ok. 261 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 13.51s
```

```text
$ cargo test -p melee-sim --test m5_gate --release
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.19s
```

```text
$ cargo test -p hsd-particle -p melee-ft -p melee-it -p it-foxlaser -p ft-fox-family -p ft-fox -p ft-falco --release
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

```text
$ cargo test -p melee-sim --test alloc_gate --release -- --nocapture
laser_fd_fox: 299 measured ticks; simulate-only 32 (0.107023/tick), peak 4, allocating ticks 19; with snapshot 37407 (125.107023/tick), peak 129; snapshot overhead 37375 (125.000000/tick)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.88s
```

```text
$ cargo gate --release
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

```text
$ tools/check-release-math.sh
Fused math: opt-level=0
Fused math: opt-level=1
Fused math: opt-level=2
Fused math: opt-level=3
Fused math: opt-level=s
Fused math: opt-level=z
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

```text
$ cargo clippy --workspace --all-targets -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.22s
```

```text
$ cargo fmt --all --check
(no output; exit 0)
```

```text
$ cargo test -p hsd-particle --test live_laser_fd_fox -- --nocapture
Compared 481555 fields across 300 ticks and 9803 ordered particle draws; mismatches: 0; AppSRT display-cache fields skipped: 812
laser_fd_fox matched 300/300 ticks: 481555 fields, 9803 ordered particle draws, all final seeds; final seed 0x8c77d25d
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.13s
```

```text
$ cargo test -p hsd-particle --test live_laser_fd_fox --release -- --nocapture
Compared 481555 fields across 300 ticks and 9803 ordered particle draws; mismatches: 0; AppSRT display-cache fields skipped: 812
laser_fd_fox matched 300/300 ticks: 481555 fields, 9803 ordered particle draws, all final seeds; final seed 0x8c77d25d
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
```

Fixture generation used the production caller-input recorder:

```text
$ cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/laser_fd_fox.toml --out crates/hsd-particle/tests/data/laser_fd_fox_spawns.json
300 ticks, 62 keys, 0 divergences; fixture crates/hsd-particle/tests/data/laser_fd_fox_spawns.json
```

The replay retains the existing AppSRT display-cache exclusions (812 fields); every other captured particle simulation field and all ordered particle RNG sites/final seeds match. The new M5 assertion separately compares the direct hit-effect sites 0x80063990 and 0x800785CC/0x800785FC in ledger order. No oracle output is fed into item gameplay.


## Changed files

```text
CLAUDE.md
Cargo.lock
Cargo.toml
TRACKER.md
crates/ft-falco/Cargo.toml
crates/ft-falco/src/init.rs
crates/ft-falco/tests/attributes.rs
crates/ft-fox-family/Cargo.toml
crates/ft-fox-family/src/lib.rs
crates/ft-fox-family/src/special_n.rs
crates/ft-fox/Cargo.toml
crates/ft-fox/src/init.rs
crates/hsd-particle/tests/data/README.md
crates/hsd-particle/tests/data/laser_fd_fox_spawns.json
crates/hsd-particle/tests/live_laser_fd_fox.rs
crates/hsd-particle/tests/support/dust_replay.rs
crates/it-foxlaser/Cargo.toml
crates/it-foxlaser/src/lib.rs
crates/it-foxlaser/tests/flight.rs
crates/melee-cmd/src/decode.rs
crates/melee-cmd/src/lib.rs
crates/melee-ef/src/lib.rs
crates/melee-ef/src/pool.rs
crates/melee-ef/src/request.rs
crates/melee-ef/src/tables.rs
crates/melee-ft/Cargo.toml
crates/melee-ft/src/fighter/commands.rs
crates/melee-ft/src/fighter/damage.rs
crates/melee-ft/src/fighter/fall.rs
crates/melee-ft/src/fighter/mod.rs
crates/melee-ft/src/fighter/spawn.rs
crates/melee-ft/src/fighter/state/callbacks/collision.rs
crates/melee-ft/src/fighter/state/special.rs
crates/melee-ft/tests/fox_spawn_native.rs
crates/melee-it/Cargo.toml
crates/melee-it/src/desc.rs
crates/melee-it/src/engine.rs
crates/melee-it/src/lib.rs
crates/melee-it/src/logic.rs
crates/melee-it/src/spawn.rs
crates/melee-sim/Cargo.toml
crates/melee-sim/src/assets.rs
crates/melee-sim/src/frame.rs
crates/melee-sim/src/initial_state/cold.rs
crates/melee-sim/src/initial_state/mod.rs
crates/melee-sim/src/lib.rs
crates/melee-sim/src/main.rs
crates/melee-sim/src/scenario.rs
crates/melee-sim/src/scene_items.rs
crates/melee-sim/src/trace.rs
crates/melee-sim/src/trace_items.rs
crates/melee-sim/tests/alloc_gate.rs
crates/melee-sim/tests/m5_gate.rs
docs/PORT_NOTES/S4_ITEMS_LASER.md
```
