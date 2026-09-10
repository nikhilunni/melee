# C13 — recording-derived fixtures

2026-09-09, battlefield lane. Supported Fox fixtures regenerated; command,
constant repairs and tests complete. C12 fixtures are unchanged. No git commands
or TRACKER edits were made during this continuation; the earlier stopped-turn
TRACKER entry predates the user's latest ruling and has been left alone.
No game data, decomp, melee-ft production source or initial_state files were edited.

## Command and hook

`melee-sim fixture-spawns <scenario.toml> --out <path.json> [--ticks N]` shares
`gate_with_recording` with `gate`: same import, input script, scheduler, schema,
field and first-divergence checks. It opens the output only after the full gate
succeeds. Prefix export still gates the entire scene. The ledge integration test
checks full parsed-JSON equality, prefix equality and failure preserving an
existing target. No fixture is read by production simulation.

`Effects` owns a concrete `EventSink(Option<Recording>)`, normally `None`.
Every event method branches on that option before inspecting/validating event
payloads, converting floats, constructing JSON or growing storage. Disabled
calls do only the option check; no event buffer, allocation, serialization,
trait-object dispatch or shared owner is created. This is zero event-processing
cost, not a claim that a dynamic option check executes zero CPU instructions.
The allocator test invokes every disabled method for 600 ticks (including a
velocity override that enabled export rejects) and measures **zero allocations**.

Hooks sit at external effect, dust, egg, stage-animation, Story puff and HUD
callers, immediately beside spawn/joint/flag operations. Tick labels precede
scheduler execution. Slash orientation and post-particle HUD requests retain
their phase/order. Interpreter child requests are never logged. The exporter
filters unused/detached joint updates and unchanged duplicate matrices, keeping
all retained call order. Spawn matrices do not suppress a later update to older
generators sharing a joint. The JSON schema remains the existing common schema.

All 12 supported exports gate successfully: six FD movement scenes, jump, dash,
FD start, BF start, DL start and Fox jab. The approved recording change moves
ledge events 246→245 and 283→282. Removing 32 redundant unchanged joint updates
also explains the ledge event-count reduction; replay remains bit-exact.
Fox jab's new random hit-spark choice reaches kind 6. Retail efLib's case 6 is
in the existing 2/306/307 root-scale group (eflib.c:904–922); that same scale and
flag path now includes 6. Its 300×49 gate and independent particle replay pass.

Exact one-command-per-file recipes, including deferred fixtures:

```sh
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/shield_fd_fox.toml --out crates/hsd-particle/tests/data/shield_fd_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/spotdodge_fd_fox.toml --out crates/hsd-particle/tests/data/spotdodge_fd_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/roll_fd_fox.toml --out crates/hsd-particle/tests/data/roll_fd_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/airdodge_fd_fox.toml --out crates/hsd-particle/tests/data/airdodge_fd_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/wavedash_fd_fox.toml --out crates/hsd-particle/tests/data/wavedash_fd_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/ledge_fd_fox.toml --out crates/hsd-particle/tests/data/ledge_fd_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/jump_fd_fox.toml --out crates/hsd-particle/tests/data/jump_fd_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/dash_fd_fox.toml --out crates/hsd-particle/tests/data/dash_fd_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/start_fd_fox.toml --out crates/hsd-particle/tests/data/start_fd_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/start_bf_fox.toml --out crates/hsd-particle/tests/data/start_bf_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/start_dl_fox.toml --out crates/hsd-particle/tests/data/start_dl_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/jab_fd_fox.toml --out crates/hsd-particle/tests/data/jab_fd_fox_spawns.json
```

## Allocation investigation

The 27,698 count reproduced with all recorder bodies changed temporarily to
compile-time `None::<&mut Recording>`: **27,698 again**. The optional recorder
therefore accounts for none of that increase. The temporary backtrace probe used
the same measured ticks as the previous lane probe (1–9 and 599); counting was
suspended during backtrace capture/storage. Both temporary edits and probe test
were removed afterward.

The old/new sampled allocation differences occur only at particle population
allocation sites: `Particle::new` 171→174, `ParticleSystem::emit` 4→5,
`sort_for_display` 1→3, `DrawLog::draw` 5→6. Other sampled project call sites,
including effects/frame, are unchanged. The replacement recording changes
particle populations and thereby existing Vec growth and stable-sort scratch
allocation. The deleted old recording prevents a full per-site accounting of
all 397 allocations; the sampled evidence is not such an accounting.

Two concrete allocation sources were removed without changing evaluation order:

1. `sort_for_display` checks whether buckets are already ordered before calling
   the same stable sort. Rust's stable sort allocates scratch before recognizing
   an ordered large slice. Equal-key ties remain unchanged. This removes 318
   allocations in this scene: 27,698→27,380.
2. `BackgroundAnimation` caches its fixed model traversal at load time;
   `for_each_matrix` visits/setup matrices in that same order. The frame callback
   consumes that visitor instead of rebuilding joint/matrix vectors each tick.
   Compatibility `matrices()` still returns the same collected result for callers
   that need ownership. This removes another 4,790–4,791 measured allocations.

Final measured simulate counts are **22,589–22,590**, snapshot **97,463**;
latest exact output is below. One allocation varies between repeated debug runs.
The unchanged ceilings are 27,301/102,175, now asserted as upper bounds, as the
acceptance explicitly permits lower counts. No budget was increased. Existing
stable-bucket/tie/link-mask tests, strict particle replay and M4 gate comparisons
validate ordering. The absent-sink test separately proves zero hook allocation.
Probe logs: `/tmp/c13-alloc-disabled.log`, `/tmp/c13-alloc-probe.log`,
`/tmp/battlefield-alloc-probe.log`, `/tmp/c13-alloc-sorted.log`.

## Fighter boundary repair

`idle_fox_600` and the shared full replay import Fighter bytes, joint SRT/matrices,
flags, dynamics state and both trees' exact AObj/FObj cursors from the savestate.
A scalar frame alone cannot reconstruct the saved blend pose. They complete the
remaining scheduler procs before comparing tick zero; between-ticks Wait saves
run a complete pass, Entry skips the pending scheduler, and the supported link-4
resume skips already-integrated P0 physics. This follows melee-sim's boundary
interpretation. The current local save observed by these tests has the
between-ticks scheduler sentinel; the helper also supports the reported link-4
boundary. No initial bone-oracle row is injected anymore.

The saved-vs-completed cross-check is restricted to +0x24 (Wait-animation data
pointer), +0x28 (secondary animation table pointer), and +0x5E8 (bone-parts array
pointer). These resources retain their identities during the pending procs;
+0x894..+0x8AC animation scalars, position and counters may advance. Definitions
are in retail Fighter/types.h. Every downstream compared fighter/bone word
remains exact; no pose exclusions were added. `real_fox_wait_playback` derives
initial frames and total Wait draws from its trace/ledger.

## Recording-constant audit

Searched all workspace Rust integration tests for large decimal/hex literals and
`to_bits()` comparisons, then inline `#[cfg(test)]` sections/test modules, Python
harness/tools tests, and small seed/frame/draw assertions. The regex deliberately
has no trailing word boundary, so suffixed integers such as `0xDEADBEEFu32` are
included. Comments and synthetic fixtures were inspected for provenance too.
Generated spawn JSON is the explicitly requested recording-derived input artifact;
it is not duplicated as Rust constants. Deferred C12 JSON is intentionally retained.

```sh
rg -n --glob '*.rs' '(\b[0-9][0-9_]{5,}|\b0x[0-9a-fA-F_]{7,}|to_bits\(\))' crates/*/tests
rg -n '(\b[0-9][0-9_]{5,}|\b0x[0-9a-fA-F_]{7,})' harness/tests tools/tests
rg -n --glob '*.rs' 'assert_eq!.*(seed|draw|frame)|to_bits\(\)' crates
```

Locations below identify the replacement in the final source; deleted files use
the original location. Tables and native oracle inputs from the retail executable
or owned DAT archives stay. Test-length contracts (600 ticks, 49 keys, 24 fighter
fields), synthetic seeds, IEEE special values and performance ceilings stay.

| File:line (under crates/) | Old literal or copied assumption | Replacement / disposition |
|---|---|---|
| `hsd-particle/tests/live_fd.rs:50` | `1_286_746_018` | Initial capture seed, metadata cross-check; no replacement numeric seed |
| `hsd-particle/tests/live_fd.rs:133` | `one initial generator / matrix` | Metadata-derived generator count and joint matrices |
| `hsd-particle/tests/support/dash_fd_spawns.rs:41` | `34/42/49/56; joint 65538; x words 3248823986/3232382294, y 953267991` | Deleted Rust schedule; generated dash_fd_spawns.json |
| `hsd-particle/tests/support/start_fd_spawns.rs:30` | `nine REQUESTS with copied tick/address/matrix schedule` | Deleted; generated start_fd_spawns.json and common replay |
| `hsd-particle/tests/support/extract_start_fd_joints.py:1` | `0x80C378E0, 0x80D417E0, 0x80D41CA0, 0x80D48FA0, 0x80D49420; tick windows` | Deleted extractor and captured start_fd_joints.json |
| `hsd-particle/tests/support/jump_fd_spawns.rs:1` | `alternate-schema support/jump_fd_spawns.json` | Deleted legacy reader/JSON; generated common data/jump_fd_spawns.json |
| `hsd-particle/tests/live_fd_start.rs:13` | `old manual schedule and unclassified caller panic` | Common strict replay; shared retail caller classification |
| `melee-ft/tests/real_fox_wait_playback.rs:122` | `[5.0f32.to_bits(), 0]` | tick-zero frame_bits from trace |
| `melee-ft/tests/real_fox_wait_playback.rs:201` | `total draws 9` | Ledger count of exact Wait caller; ordered per-tick comparison retained |
| `melee-ft/tests/idle_fox_600.rs:209` | `total draws 9` | Ledger count including completed tick zero |
| `melee-ft/tests/fighter_support/saved_pose.rs:139` | `+0x894..+0x8AC equality; old 3212 bone-row size` | Invariant resource pointers; oracle-row restore removed; full saved streams restored |
| `melee-ft/tests/fighter_support/replay.rs:341` | `start total draws 16` | Matching ledger sites over replayed ticks |
| `melee-ft/tests/fighter_support/replay.rs:383` | `bone field count 3212` | Owned bone count ×22-field schema |
| `melee-ft/tests/start_fox_states.rs:140` | `total draws 16` | Exact fighter-site count in ledger |
| `melee-ft/tests/movement_fox_states.rs:162` | `catch tick70 / take71; initial facing1.0` | First CliffCatch in raw trace; raw initial facing bits |
| `melee-sim/src/frame.rs:792` | `initial seed0xCC51_A0A5; no tick-zero draws; included old joint fixture` | Sidecar seed; current particle metadata matrices; full particle-state comparisons |
| `melee-sim/tests/m2_gate.rs:47` | `position(-42,23.450098,0), facing1, animation6, inverse scale(67,1.0416667), dirty[67,71,72]` | Capture metadata; model scale/scaled_joint from owned DAT; metadata-derived dirty fields/coverage |
| `melee-sim/tests/m5_gate.rs:8` | `particle draw total9373` | Nonempty invariant; helper already compares each exact ordered draw to ledger |
| `melee-sim/tests/slippi_oracle.rs:93` | `0xcc51_a0a5 / 0xc37a8245; fixed first-two VI equality` | Sidecar initial seed, every ledger LCG transition and trace seed; monotonic VI permits repeats |
| `melee-sim/tests/slippi_oracle.rs:161` | `f32::from_bits(0x65000c80)` | f32::MAX synthetic invalid analog input |
| `melee-sim/tests/slippi_replay.rs:320` | `online Slippi seed0x3AAE` | Parsed replay.start.random_seed in netplay formula |
| `melee-sim/tests/alloc_gate.rs:89` | `27301 / 102175 exact totals` | Unchanged upper-bound performance budgets |
| `melee-ft/tests/real_fox_wait_playback.rs:92` | `0x3f75_c28f model scale; blend6/0` | Retained retail PlFx/ftData attributes, not savestate frames |
| `melee-gr/tests/real_fd.rs:56` | `3_884_216_597 seed` | Retained four hand-LCG steps from synthetic seed1 |
| `hsd-particle/tests/start_paths.rs:57` | `2_745_024 seed` | Retained one hand-LCG step from synthetic seed1 |
| `melee-ft/tests/input_support/mod.rs:29` | `0x3F5F66F3` | Retained retail PlCo tilt-angle attribute |

Remaining matches were retained with these classifications. The per-file inventory
below lists every matching final line; `R` means retail/archive values or code/layout
addresses, `S` synthetic numeric/IEEE/native-oracle inputs, `D` dynamic trace/ledger
comparisons, `B` explicit budget/schema/test-length contracts. No recording seed,
frame, address or draw expectation remains hardcoded in the audited Rust tests.
Python memory addresses are constructed mock-memory fixtures; assembly snippets
are retail instruction/address data. They do not read the re-recorded corpus.

<details><summary>Per-file retained-match audit inventory</summary>

| File | Matching lines | Classification |
|---|---|---|
| `crates/ft-captain/tests/attributes.rs` | 13, 14, 23, 24, 27, 28, 31, 74, 75, 76, 77, 78, 80, 83, 86 | R owned archive/layout values; S synthetic reader fixtures |
| `crates/ft-falco/tests/attributes.rs` | 10, 35, 67, 71, 73, 77, 80 | R owned archive/layout values; S synthetic reader fixtures |
| `crates/ft-fox/tests/attributes.rs` | 12, 14, 15, 26, 27, 28, 29, 30, 31, 32, 36, 41, 46, 51, 56, 61, 66, 71, 76, 81, 86, 88, 92, 94, 98, 100, 104, 106, 107, 108, 112, 117, 119, 120, 124, 129, 131, 132, 133, 134, 135, 139, 141, 145, 150, 155, 157, 161, 166, 170, 209, 213, 218, 223, 225, 226, 230, 235, 237, 241, 244 | R owned archive/layout values; S synthetic reader fixtures |
| `crates/ft-mars/tests/attributes.rs` | 13, 14, 22, 23, 51, 52, 53, 54, 55, 57, 88 | R owned archive/layout values; S synthetic reader fixtures |
| `crates/ft-peach/tests/attributes.rs` | 13, 14, 22, 23, 30, 57, 70, 105, 106, 107, 108, 109, 110, 111, 113, 168, 169, 172, 173 | R owned archive/layout values; S synthetic reader fixtures |
| `crates/ft-purin/tests/attributes.rs` | 13, 14, 15, 23, 24, 25, 51, 53, 55 | R owned archive/layout values; S synthetic reader fixtures |
| `crates/ft-yoshi/tests/attributes.rs` | 14, 15, 17, 23, 24, 26, 29, 30, 57, 58, 59, 60, 88, 89, 91, 94, 95, 97, 122, 123 | R owned archive/layout values; S synthetic reader fixtures |
| `crates/gekko-math/tests/golden.rs` | 71, 79, 166 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/gekko-math/tests/ref_oracle.rs` | 157, 158, 174, 175, 177, 178, 190, 191, 193, 194, 195, 196, 203, 205, 206, 217, 228, 229, 232, 235, 237, 241, 245, 261, 262, 263, 264, 275, 281, 284, 287, 312, 313, 316, 341, 353, 354, 376, 385, 424, 430, 436, 442, 465, 475, 490, 500, 506, 507, 508, 533, 554, 558 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/hsd-anim/tests/anim_aobj.rs` | 76, 91 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/hsd-anim/tests/anim_common/mod.rs` | 55, 105, 106 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/hsd-anim/tests/anim_fobj.rs` | 18, 74, 143, 154, 155, 157, 162, 164, 166, 168, 202, 203, 213, 224, 230, 231, 366, 367, 371, 386, 388, 389, 432, 436, 458, 464, 484, 485, 498, 521, 541, 560, 831, 832 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/hsd-anim/tests/anim_ref_oracle.rs` | 143, 146, 161, 175, 197, 208, 209, 246, 247, 255, 349, 374, 375, 376, 377, 378, 636 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/hsd-anim/tests/jobj_anim.rs` | 83, 84, 476, 494, 521 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/hsd-anim/tests/jobj_matrix.rs` | 21, 22 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/hsd-anim/tests/load.rs` | 110, 148, 187, 245, 246, 400, 401 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/hsd-anim/tests/load_common/mod.rs` | 41 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/hsd-anim/tests/mtx_oracle.rs` | 170, 192, 193, 966, 1014, 1027, 1028, 1071, 1093 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/hsd-anim/tests/real_fox.rs` | 52, 53, 54 | R owned archive/layout values; S synthetic reader fixtures |
| `crates/hsd-archive/tests/desc.rs` | 55, 407, 408, 424, 453, 483, 520, 551, 583, 584, 631, 632, 640, 652, 907, 1070, 1078, 1079 | R owned archive/layout values; S synthetic reader fixtures |
| `crates/hsd-archive/tests/real_dat.rs` | 91, 98, 105, 112, 251, 378 | R owned archive/layout values; S synthetic reader fixtures |
| `crates/hsd-archive/tests/synthetic.rs` | 44, 117, 124, 184, 185, 194, 307 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/hsd-particle/tests/lifecycle.rs` | 28, 37, 85 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/hsd-particle/tests/live_fd.rs` | 16, 17, 121, 122 | D; R callback/layout constants; S test arithmetic |
| `crates/hsd-particle/tests/live_fd_start.rs` | 59 | D; R callback/layout constants; S test arithmetic |
| `crates/hsd-particle/tests/opcodes.rs` | 188, 193, 207, 263, 294, 342, 344 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/hsd-particle/tests/real_fd_bank.rs` | 36, 37, 38, 39, 57, 97, 98 | R owned archive/layout values; S synthetic reader fixtures |
| `crates/hsd-particle/tests/real_fd_particles.rs` | 101 | R owned archive/layout values; S synthetic reader fixtures |
| `crates/hsd-particle/tests/start_paths.rs` | 56, 57, 92, 149, 163, 208, 216, 434, 435 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/hsd-particle/tests/support/dust_replay.rs` | 41, 42, 43, 44, 45, 49, 50, 201, 212, 217, 220, 251, 254, 255, 257 | D; R callback/layout constants; S test arithmetic |
| `crates/hsd-particle/tests/support/fixture_spawns.rs` | 59 | D; R callback/layout constants; S test arithmetic |
| `crates/melee-ft/tests/airborne_ref_oracle.rs` | 65, 69, 72, 121, 122, 123, 135 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/melee-ft/tests/animation_ref_oracle.rs` | 87, 89, 93, 97, 102, 110, 125, 126, 127, 128, 129, 130, 131, 184, 185, 186, 199 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/melee-ft/tests/bone_desc.rs` | 75, 76, 77, 78, 121, 122, 123, 124 | R owned archive/layout values; S synthetic reader fixtures |
| `crates/melee-ft/tests/common_desc.rs` | 10, 35 | R owned archive/layout values; S synthetic reader fixtures |
| `crates/melee-ft/tests/cpu_initialization.rs` | 9, 13 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/melee-ft/tests/fighter_animation.rs` | 146, 264, 268, 269 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/melee-ft/tests/fighter_attributes.rs` | 13, 15, 16, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 40, 42, 45, 49, 53, 55, 56, 57, 58, 60, 61, 62, 63, 64, 65, 66, 69, 71, 72, 73, 74, 75, 76, 79, 81, 82, 83, 84, 87, 91, 95, 99, 103, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 125, 129, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140, 141, 142, 143, 146, 148, 151, 153, 154, 155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165, 168 | R owned archive/layout values; S synthetic reader fixtures |
| `crates/melee-ft/tests/fighter_support/rendered_pose.rs` | 42 | D; R callback/layout constants; S test arithmetic |
| `crates/melee-ft/tests/fighter_support/replay.rs` | 170, 177, 353, 420, 434, 435, 496 | D; R callback/layout constants; S test arithmetic |
| `crates/melee-ft/tests/fighter_support/saved_pose.rs` | 36, 46, 47, 52, 56, 57, 168, 169, 170 | D; R callback/layout constants; S test arithmetic |
| `crates/melee-ft/tests/fox_spawn_native.rs` | 21, 36, 39, 42, 103, 109, 110, 123, 124, 128, 129, 138, 142, 144 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/melee-ft/tests/grounded_physics_native.rs` | 70, 92, 105, 108, 109, 110, 120, 122, 125, 126, 131, 152, 207 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/melee-ft/tests/human_input.rs` | 24, 25, 28, 31, 32, 50, 55, 57, 68, 69, 76, 77, 101, 125, 126, 150 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/melee-ft/tests/idle_fox_600.rs` | 74, 174, 175, 176, 179, 180, 181, 192, 197, 216 | D; R callback/layout constants; S test arithmetic |
| `crates/melee-ft/tests/idle_ground_fields_600.rs` | 100, 101, 183, 248, 315, 316, 344, 373, 389, 393 | D; R callback/layout constants; S test arithmetic |
| `crates/melee-ft/tests/input_oracle.rs` | 68, 70, 74, 88, 105, 121, 128 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/melee-ft/tests/input_support/mod.rs` | 29, 60, 61, 64 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/melee-ft/tests/movement_fox_states.rs` | 95, 96, 100, 101, 106, 107, 199 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/melee-ft/tests/real_fox_data.rs` | 39, 44, 49, 51, 55, 60, 65, 67, 71, 76, 78, 82, 84, 85, 86, 87, 88, 89, 92, 150, 151, 152, 154, 189, 190, 191, 199, 200, 201, 202, 203, 204, 205, 206, 207, 208, 209, 210, 211, 212, 213, 214, 216 | R owned archive/layout values; S synthetic reader fixtures |
| `crates/melee-ft/tests/real_fox_wait_playback.rs` | 92, 122, 132, 170, 192, 209 | D; R callback/layout constants; S test arithmetic |
| `crates/melee-ft/tests/start_fox_states.rs` | 49, 151 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/melee-gr/tests/real_battlefield.rs` | 19, 55, 68, 69, 71 | R owned archive/layout values; S synthetic reader fixtures |
| `crates/melee-gr/tests/real_fd.rs` | 37, 56 | R owned archive/layout values; S synthetic reader fixtures |
| `crates/melee-gr/tests/real_pupupu.rs` | 19, 32, 49, 50, 62 | R owned archive/layout values; S synthetic reader fixtures |
| `crates/melee-gr/tests/real_story.rs` | 21, 32 | R owned archive/layout values; S synthetic reader fixtures |
| `crates/melee-lb/tests/dynamics_ref_oracle.rs` | 71, 74, 75, 76, 82, 152, 157 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/melee-lb/tests/figatree_attach.rs` | 97, 110, 116, 156, 157, 158, 178, 179, 199, 200 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/melee-lb/tests/real_fox_wait.rs` | 41, 42, 97 | R owned archive/layout values; S synthetic reader fixtures |
| `crates/melee-lb/tests/ref_oracle.rs` | 144, 145, 163, 182, 183, 184, 186, 187, 188, 208, 220, 221, 222, 223, 224, 225, 247, 248, 250, 251, 255, 258, 260, 264, 268, 302, 304, 308, 315, 317, 319, 323, 346, 347, 349, 350, 354, 359, 408, 412, 422, 423, 459, 471, 472, 494, 503, 511, 521, 543, 546, 558, 565, 576, 580, 595, 601, 607 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/melee-mp/tests/desc.rs` | 23, 33, 109, 124 | R owned archive/layout values; S synthetic reader fixtures |
| `crates/melee-mp/tests/geom_oracle.rs` | 155, 156, 166, 167, 175, 201, 238 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/melee-mp/tests/real_stage.rs` | 14, 15, 16, 33, 34, 65, 66, 93, 94, 108, 109, 122, 123, 128, 129, 148, 149, 155, 156, 171 | R owned archive/layout values; S synthetic reader fixtures |
| `crates/melee-sim/tests/alloc_gate.rs` | 89, 93 | B unchanged allocation ceilings |
| `crates/melee-sim/tests/fixture_spawns.rs` | 59 | D; R RNG callers/LCG formula; B schema; S disabled-sink inputs |
| `crates/melee-sim/tests/m4_gate.rs` | 77 | D; R RNG callers/LCG formula; B schema; S disabled-sink inputs |
| `crates/melee-sim/tests/m5_gate.rs` | 78, 133 | D; R RNG callers/LCG formula; B schema; S disabled-sink inputs |
| `crates/melee-sim/tests/slippi_oracle.rs` | 27, 28, 29, 30, 31, 32, 108, 120, 121, 122, 123, 124, 125, 126, 127 | D; R RNG callers/LCG formula; B schema; S disabled-sink inputs |
| `crates/melee-sim/tests/slippi_replay.rs` | 172 | D; R RNG callers/LCG formula; B schema; S disabled-sink inputs |
| `crates/slp/tests/parse_fixtures.rs` | 128 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/gekko-math/src/estimate.rs` | 259, 260, 261, 262, 263, 264, 267, 273, 274, 275, 278, 279, 285, 288, 289, 291, 292, 297, 298, 299, 304, 307, 309, 310, 311, 312, 315, 316, 317, 318, 319, 330, 332, 333, 334, 335, 336, 337, 338, 339, 340, 344, 349, 356, 357 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/gekko-math/src/fma.rs` | 74 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/gekko-math/src/msl.rs` | 548, 551, 562, 565, 566, 577, 627, 630, 654, 656, 657, 679, 685, 700, 736, 737, 739, 740 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/gekko-math/src/rng.rs` | 73, 76, 81, 82, 83, 84, 92, 96, 97 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/hsd-anim/src/jobj.rs` | 1598 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/hsd-anim/src/mobj.rs` | 299 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/hsd-anim/src/mtx.rs` | 1020, 1025, 1163 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/hsd-anim/src/quat.rs` | 429 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/hsd-archive/src/reader.rs` | 147, 159, 161 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/hsd-particle/src/bank.rs` | 240, 241, 253, 254 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/melee-diff/src/lib.rs` | 201, 221, 222, 225, 226, 230, 231, 242 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/melee-diff/src/snapshot.rs` | 112, 113 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/melee-ft/src/anim/wait_choice.rs` | 66, 68, 70, 73, 74, 75, 76, 77, 78, 79, 97 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/melee-ft/src/collision/tests.rs` | 165, 167, 168 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/melee-ft/src/fighter/entry.rs` | 211, 218 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/melee-ft/src/fighter/walk.rs` | 217, 222 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/melee-gr/src/battle/mod.rs` | 87 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/melee-gr/src/last/background.rs` | 102, 106, 107 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/melee-gr/src/last/procs.rs` | 272, 275 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/melee-gr/src/pupupu/mod.rs` | 263, 264 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/melee-lb/src/collision.rs` | 222 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/melee-lb/src/trigf.rs` | 511, 512, 513, 515, 518, 522, 526, 530, 537, 538, 545, 547, 548, 585, 586, 623, 624 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/melee-mp/src/tests.rs` | 1509, 1698, 1704, 1705 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/melee-sim/src/bones.rs` | 195, 208 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/melee-sim/src/effects.rs` | 323, 415 | Production tail after cfg declaration; false-positive scan rows, not test assertions |
| `crates/melee-sim/src/frame.rs` | 635, 636, 648, 655, 656, 661, 681, 847 | D; R callback/layout constants; S test arithmetic |
| `crates/melee-sim/src/initial_state/mod.rs` | 136, 137, 158, 161, 170, 176, 177, 188, 222, 245, 279, 351, 367, 368 | Production tail after cfg declaration; false-positive scan rows, not test assertions |
| `crates/melee-sim/src/inputs.rs` | 230, 237 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/melee-types/src/mp.rs` | 638 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/melee-types/src/snapshot.rs` | 215, 225, 235, 236 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `crates/slp/src/event.rs` | 525, 528 | S deterministic fixtures/native oracle/IEEE cases; R formulas and layout constants |
| `harness/tests/test_asm.py` | 18, 22, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 36, 38, 39, 40, 41, 42, 44, 46, 47, 48, 49, 50, 51, 52, 53, 54, 65, 66, 67, 70, 71, 74, 84, 96, 103, 104, 105, 106, 107, 117, 118, 120, 126, 127, 129, 130, 133, 139, 140, 141, 148, 149, 150, 151, 152, 153, 154, 171, 172, 173 | S constructed mock-memory/protocol inputs; R addresses/assembly |
| `harness/tests/test_extract_fst.py` | 182 | S constructed mock-memory/protocol inputs; R addresses/assembly |
| `harness/tests/test_items.py` | 18, 19, 27, 49, 96, 120, 121, 122, 123, 160, 276 | S constructed mock-memory/protocol inputs; R addresses/assembly |
| `harness/tests/test_jobjdump.py` | 15, 16, 51, 53, 54, 59, 65, 66, 74, 78, 111, 122, 135, 160, 161, 171 | S constructed mock-memory/protocol inputs; R addresses/assembly |
| `harness/tests/test_particle_dump.py` | 17, 18, 19, 78, 98, 99, 101, 104, 105, 106, 109, 199, 204, 212 | S constructed mock-memory/protocol inputs; R addresses/assembly |
| `harness/tests/test_remote.py` | 18, 19, 36, 37, 63, 64, 68, 69, 175, 179, 206, 209, 222, 223, 225, 241, 290, 311, 372, 373, 374, 375, 376, 406 | S constructed mock-memory/protocol inputs; R addresses/assembly |
| `harness/tests/test_tick_trace.py` | 195, 220 | S constructed mock-memory/protocol inputs; R addresses/assembly |
| `harness/tests/test_validate_ticks.py` | 23, 60, 83, 85, 86 | S constructed mock-memory/protocol inputs; R addresses/assembly |
| `harness/tests/test_walk.py` | 13, 14, 15, 16, 17, 18, 96, 100, 116, 117, 120, 121, 122, 145, 209, 225, 239 | S constructed mock-memory/protocol inputs; R addresses/assembly |
| `tools/tests/test_perf_gate.py` | 33 | S constructed mock-memory/protocol inputs; R addresses/assembly |

</details>

## Validation

All Cargo commands used `CARGO_HOME=/tmp/c13-cargo-home`; the default registry
source directory is read-only, so missing packages were unpacked into that temporary
home using the existing readable cache. No normal Cargo-home files were changed.

```text
CARGO_HOME=/tmp/c13-cargo-home cargo test -p hsd-particle --test live_fd --test live_fd_ledge --test live_jab_fd_fox --test live_fd_start --test live_fd_airdodge --test live_fd_dash --test live_fd_jump --test live_fd_roll --test live_fd_shield --test live_fd_spotdodge --test live_fd_wavedash --test live_bf_start --test live_dl_start
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.50s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.54s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.83s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.27s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.14s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.31s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.79s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.34s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.21s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.26s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.08s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.33s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.28s
```

```text
CARGO_HOME=/tmp/c13-cargo-home cargo test -p melee-ft
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.27s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.37s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.27s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.26s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.58s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.22s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.86s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.64s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.18s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.71s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

```text
CARGO_HOME=/tmp/c13-cargo-home cargo test -p melee-sim --test m4_gate -- fox
test result: FAILED. 22 passed; 4 failed; 0 ignored; 0 measured; 235 filtered out; finished in 5.25s
```

```text
CARGO_HOME=/tmp/c13-cargo-home cargo test -p melee-sim --test alloc_gate --test fixture_spawns -- --nocapture
599 measured ticks: simulate-only 22590 allocations (37.712855/tick); with snapshot 97463 allocations (162.709516/tick); snapshot overhead 74873 (124.996661/tick)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.85s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.15s
```

```text
CARGO_HOME=/tmp/c13-cargo-home cargo test -p melee-sim --test fixture_spawns --test slippi_oracle --test slippi_replay
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.29s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.40s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.45s
```

```text
CARGO_HOME=/tmp/c13-cargo-home cargo test -p melee-sim --lib start_effect_matrices_and_particle_state
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 47 filtered out; finished in 4.76s
```

Supported particle coverage is 15 tests in 13 binaries. `live_fd_idle` does not
exist; `live_fd` contains idle FD coverage. Full melee-ft reports 90 passed,
zero failed/ignored. Existing optional local-corpus early returns remain: the
legacy M2 bone/VI captures are absent, so their rewritten metadata initialization
is compiled but not live-validated. The active idle/start SRT and start particle
matrix tests ran and passed.

M4 has exactly the expected 22 passes / four C12 failures: `idle_bf_fox_600`,
`platform_bf_fox_300` (link1 callback8006A360), `idle_dl_fox_600` and
`idle_ys_fox_600` (link6 callback8006C27C). No new M4 failure.

```text
CARGO_HOME=/tmp/c13-cargo-home cargo gate
battlefield_idle_600_particles_and_ordered_rng ... FAILED
tick 161 field coverage: left 5564, right 5456
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.05s
error: test failed, to rerun pass `-p hsd-particle --test live_bf_idle`
```

This is the known unregenerated C12 fixture. Full workspace gate remains red;
no C12 test was loosened or skipped. `cargo fmt --all` exits 0. Final
`cargo clippy --workspace --all-targets -- -D warnings` exits 0 (all targets).

## Regenerate after C12

Unchanged: idle BF/DL/YS, start YS, Marth jab/utilt/shieldhit/grab-startup/grab/tech/KO.
Other non-Fox character scenes remain blocked even without a checked-in fixture.

```sh
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/idle_bf_fox.toml --out crates/hsd-particle/tests/data/idle_bf_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/idle_dl_fox.toml --out crates/hsd-particle/tests/data/idle_dl_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/idle_ys_fox.toml --out crates/hsd-particle/tests/data/idle_ys_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/start_ys_fox.toml --out crates/hsd-particle/tests/data/start_ys_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/jab_fd_marth.toml --out crates/hsd-particle/tests/data/jab_fd_marth_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/utilt_fd_marth.toml --out crates/hsd-particle/tests/data/utilt_fd_marth_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/shieldhit_fd_marth.toml --out crates/hsd-particle/tests/data/shieldhit_fd_marth_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/grab_fd_marth.toml --out crates/hsd-particle/tests/data/grab_fd_marth_startup_spawns.json --ticks 127
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/grab_fd_marth.toml --out crates/hsd-particle/tests/data/grab_fd_marth_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/tech_fd_marth.toml --out crates/hsd-particle/tests/data/tech_fd_marth_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/ko_fd_marth.toml --out crates/hsd-particle/tests/data/ko_fd_marth_spawns.json
```

## Changed files

- Recorder/CLI: `melee-sim/src/{fixture_spawns.rs,lib.rs,main.rs,trace.rs}`;
  `melee-sim/tests/fixture_spawns.rs`.
- External caller hooks: `melee-sim/src/{effects.rs,frame.rs,scene_stage/pupupu.rs}`,
  `effects/{dust.rs,egg_shell.rs}`. Kind-6 retail dispatch in effects.rs.
- Allocation sources: `hsd-particle/src/system.rs`,
  `melee-gr/src/last/animation.rs`; unchanged ceilings in `melee-sim/tests/alloc_gate.rs`.
- Particle tests: `live_fd.rs`, `live_fd_dash.rs`, `live_fd_jump.rs`, `live_fd_start.rs`,
  `support/dust_replay.rs`; 12 generated data JSON files listed above.
- Deleted obsolete support: `dash_fd_spawns.rs`, `jump_fd_spawns.rs`,
  `jump_fd_spawns.json`, `start_fd_spawns.rs`, `start_fd_joints.json`,
  `extract_start_fd_joints.py`.
- Fighter tests only: `idle_fox_600.rs`, `real_fox_wait_playback.rs`,
  `start_fox_states.rs`, `movement_fox_states.rs`, `fighter_support/{replay.rs,saved_pose.rs}`.
- Other audited tests: `melee-sim/tests/{m2_gate.rs,m5_gate.rs,slippi_oracle.rs,slippi_replay.rs}`;
  inline start-matrix test in `melee-sim/src/frame.rs`.
- Documentation: `hsd-particle/tests/data/README.md`, `hsd-particle/START_FD.md`, this report.

Paths in this list are under `crates/` unless prefixed with `docs/`.

## Post-merge follow-up — ef RNG prefix ownership (2026-09-09)

The two Marth replay failures were caused by consuming a fixture-owned RNG
draw twice. The reported decimal value `2147892080` is **0x80063B70**, not
0x80063770; `2151280384` is **0x8039EF00**, not 0x8039F000.
`symbols.txt:1378` assigns 0x80063930..0x8006729B to `efAsync_Dispatch`;
0x80063B70 is its +0x240 call to `HSD_Randf`. The retail assembly explicitly
shows `80063B70: bl HSD_Randf` after `efLib_Create_Attach_Pos(8, ...)`.
This is efasync.c case 0x3EC, line 133, through the inline
`efAsync_SetEffectRandomRotationZ` helper at lines 34–36. The hex address
0x80063770 in the question instead falls inside `efAlt_Spawn` (symbols.txt:1377);
it is not the call recorded by either failing ledger.

At jab tick 124, the ledger begins with 0x80063B70 then 0x8039EF00. At up-tilt
tick 126, three landing-offset draws precede that same pair. Both fixtures
already contain the orientation `external_randf` event, correctly ordered before
the slash spawn. The old prefix scan consumed all non-particle draws before the
first particle draw, including that event; replaying the fixture consumed it
again and shifted the draw comparison and RNG state.

`dust_replay.rs` now stops the external-prefix scan at a particle draw **or**
the exact fixture-owned site 0x80063B70. The latter must have Randf PC 0x8038054C.
The existing fixture reader consumes it once and includes it in the exact
ordered site/count comparison. Particle classification, every particle draw,
LCG verification of every ledger entry, final seeds, field comparisons and
unknown-site/interleaving rejection are retained. There is no broad ef range
exclusion and no production-code or fixture-format change. Existing KO coverage
also exercises orientation events interleaved with particle constructor draws.

Changed files in this follow-up: `crates/hsd-particle/tests/support/dust_replay.rs`,
`crates/hsd-particle/tests/data/README.md`, and this report. README now records
that C12 and the eleven deferred regenerations have landed, and explains event
ownership at the prefix boundary. No git commands or TRACKER edits.

The temporary re-exports confirm that the existing format and fixture contents
are correct; neither checked-in JSON was overwritten:

```text
CARGO_HOME=/tmp/c13-cargo-home cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/jab_fd_marth.toml --out /tmp/c13-followup-jab.json
300 ticks, 49 keys, 0 divergences; fixture /tmp/c13-followup-jab.json
CARGO_HOME=/tmp/c13-cargo-home cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/utilt_fd_marth.toml --out /tmp/c13-followup-utilt.json
300 ticks, 49 keys, 0 divergences; fixture /tmp/c13-followup-utilt.json
jab_fd_marth: regenerated JSON equals checked-in fixture
utilt_fd_marth: regenerated JSON equals checked-in fixture

CARGO_HOME=/tmp/c13-cargo-home cargo test -p melee-sim --test m5_gate
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 47.47s

CARGO_HOME=/tmp/c13-cargo-home cargo fmt --all
exit 0
CARGO_HOME=/tmp/c13-cargo-home cargo clippy --workspace --all-targets -- -D warnings
exit 0
```

Full particle validation (`CARGO_HOME=/tmp/c13-cargo-home cargo test -p
hsd-particle`) passed: 75 tests, zero failures/ignored, across all 29 unit/integration
binaries plus doc tests. Exact result lines per binary follow:

```text
Running unittests src/lib.rs (target/debug/deps/hsd_particle-db66684c16b83668)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
Running tests/lifecycle.rs (target/debug/deps/lifecycle-436a00bf7b585947)
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
Running tests/live_bf_idle.rs (target/debug/deps/live_bf_idle-fea824b16b9c7543)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 53.86s
Running tests/live_bf_start.rs (target/debug/deps/live_bf_start-71330e181852a515)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 30.25s
Running tests/live_dl_idle.rs (target/debug/deps/live_dl_idle-f8637d625a6a6db5)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
Running tests/live_dl_start.rs (target/debug/deps/live_dl_start-a245c385bd0130bf)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.76s
Running tests/live_fd.rs (target/debug/deps/live_fd-c68dcb718e95abed)
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.38s
Running tests/live_fd_airdodge.rs (target/debug/deps/live_fd_airdodge-1652ce294d48c8df)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.59s
Running tests/live_fd_dash.rs (target/debug/deps/live_fd_dash-18689f5ce16a1f1f)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.51s
Running tests/live_fd_jab_marth.rs (target/debug/deps/live_fd_jab_marth-cc0c27399e234fc5)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.83s
Running tests/live_fd_jump.rs (target/debug/deps/live_fd_jump-575d5856db92a222)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.21s
Running tests/live_fd_ledge.rs (target/debug/deps/live_fd_ledge-b24e5bd0a4ac5856)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.63s
Running tests/live_fd_roll.rs (target/debug/deps/live_fd_roll-89d3680f45569c11)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.36s
Running tests/live_fd_shield.rs (target/debug/deps/live_fd_shield-b6d3074ad2824617)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.14s
Running tests/live_fd_spotdodge.rs (target/debug/deps/live_fd_spotdodge-3cf8020c312d67c7)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.40s
Running tests/live_fd_start.rs (target/debug/deps/live_fd_start-4f8109f3a14bafaf)
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.93s
Running tests/live_fd_wavedash.rs (target/debug/deps/live_fd_wavedash-58e297c312b6a69c)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.54s
Running tests/live_grab_fd_marth.rs (target/debug/deps/live_grab_fd_marth-cbe796f7f3c0458f)
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.83s
Running tests/live_jab_fd_fox.rs (target/debug/deps/live_jab_fd_fox-dcee3d6a81e15500)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.50s
Running tests/live_ko_fd_marth.rs (target/debug/deps/live_ko_fd_marth-3b0dd151ee66136a)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.58s
Running tests/live_shieldhit_fd_marth.rs (target/debug/deps/live_shieldhit_fd_marth-85eccb3bf2338080)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.93s
Running tests/live_tech_fd_marth.rs (target/debug/deps/live_tech_fd_marth-201256787c7342f5)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.73s
Running tests/live_utilt_fd_marth.rs (target/debug/deps/live_utilt_fd_marth-fb7b5bcf7752aae4)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.99s
Running tests/live_ys_idle.rs (target/debug/deps/live_ys_idle-c3f8ed14d2a7d0ad)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
Running tests/live_ys_start.rs (target/debug/deps/live_ys_start-aa617242786dba9b)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.01s
Running tests/opcodes.rs (target/debug/deps/opcodes-1110184e69b1bc41)
test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
Running tests/real_fd_bank.rs (target/debug/deps/real_fd_bank-3f4fefa104e2c14d)
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
Running tests/real_fd_particles.rs (target/debug/deps/real_fd_particles-0b8f58405da803ab)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
Running tests/start_paths.rs (target/debug/deps/start_paths-3ffb5f1800c1bffe)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

The additional workspace `cargo gate` is **not green**. It reaches an unrelated
unchanged melee-sim test, `frame::falcon_bones::idle_falcon_partial_emission_particles_600`,
and fails at `src/frame/falcon_bones.rs:138` on
`assert!(initial.pending_emission.is_some())`, before simulation begins. This
unit test imports the Falcon savestate directly and does not compile or call
`hsd-particle/tests/support/dust_replay.rs`. Its boundary-shape assumption was
not changed or weakened for this follow-up. The focused rerun reproduces it:

```text
CARGO_HOME=/tmp/c13-cargo-home cargo gate
test result: FAILED. 50 passed; 1 failed; 2 ignored; 0 measured; 0 filtered out; finished in 31.92s
error: test failed, to rerun pass `-p melee-sim --lib`

CARGO_HOME=/tmp/c13-cargo-home cargo test -p melee-sim --lib frame::falcon_bones::idle_falcon_partial_emission_particles_600
assertion failed: initial.pending_emission.is_some()
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 52 filtered out; finished in 4.93s
error: test failed, to rerun pass `-p melee-sim --lib`
```
