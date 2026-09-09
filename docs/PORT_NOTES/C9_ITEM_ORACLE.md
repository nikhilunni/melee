# C9: item recording oracle

Implemented in the chars lane, 2026-09-09; uncommitted. No Dolphin was run.
Recording `laser_fd_fox` in the main checkout remains Claude's next step.

## Files

- `harness/dolphin/trace_common.py`: complete item-list walk.
- `harness/dolphin/tick_trace.py`: item bytes, addresses, kind/name and owner slot per tick.
- `harness/gen_item_kinds.py`, `harness/item_kinds.py`: reproducible ItemKind name table.
- `harness/decode.py`: Item fields beside their preserved raw bytes.
- `harness/validate_ticks.py`: item identity, lifecycle and free-flight laser diagnostics.
- `harness/tests/test_items.py`: 34 synthetic tests, including the recording wrapper path.
- `docs/DOLPHIN_RUN.md`, this report, `TRACKER.md`: recording contract and handoff.

`record.py`, the particle/bone snippets, `crates/`, scenarios, existing
traces/ROMs and the decomp were not modified. No Git write commands ran.
The decomp symlink/typechange and ROM/trace symlinks were present on entry.

## Retail list and spawn evidence

All source paths below are relative to `third_party/melee-decomp/src/`.
Addresses are from `config/GALE01/symbols.txt`, GALE01 1.02.

| Symbol or member | Address/offset | Evidence |
|---|---|---|
| `HSD_GObj_Entities` | `0x804D782C` stores the list-array pointer | `sysdolphin/baselib/gobj.h:110-113` |
| Fighter head | `read_u32(read_u32(0x804D782C) + 0x20)` | `gobj.h:75`, p_link 8 |
| Item head | `read_u32(read_u32(0x804D782C) + 0x24)` | `gobj.h:76`, p_link 9 |
| Next GObj | `gobj + 0x08` | `gobj.h:36` |
| Item user data | `gobj + 0x2C` | `gobj.h:44` |
| `Item_8026862C` | `0x8026862C` | `melee/it/item.c:949-1012`: creates class ITEM, p_link 9; allocates Item, attaches user data |
| `Item_80268B18` | `0x80268B18` | `item.c:1015-1021`: airborne spawn wrapper |
| `Item_80267AA8` | `0x80267AA8` | `item.c:563-578`: kind, hold kind, spawn counter, self GObj, initial msid -1 |
| `it_804D6D10` | `0x804D6D10` | `item.c:570`: postincremented into Item `x1C` |
| `Item_802697D4` | `0x802697D4` | `item.c:1390-1437`: physics, velocity/nudge integration and extra movement |
| Laser physics / collision | `0x8029C9CC` / `0x8029C9EC` | `it/kinds/itfoxlaser.c:92-111` |
| `HSD_PadGameStatus` | `0x804C21CC`, 4 x 0x44 bytes | existing pad capture, unchanged |

Walk the NULL-terminated `next` chain in its actual order, reading every live
Item's **0xFCC bytes**, including `xDD4_itemVar` through byte 0xFCB
(`it/types.h:666-669`). Preserve both addresses; never sort by kind/address.
A NULL user_data is skipped as in the fighter walk. A cycle, invalid pointer
or image crossing the MEM1 boundary fails capture rather than truncating it.
The empty-list path reads only the entities pointer and item head, and does
not build an owner map or read any item payload.

There is no fighter-style arbitrary count cap: `Item_80266FCC` loads
per-hold-kind limits from `ItemCommonData` (`item.c:132-144`), and
`Item_8026784C` checks those counters (`item.c:455-528`). Item storage uses
`HSD_ObjAlloc`, not one fixed-size array. Walking every valid node to NULL
supports the retail limits without guessing a global maximum. The synthetic
suite includes 128 nodes, well beyond the eight-fighter walk cap.

`SpawnItem` is a descriptor, not the Item layout (`it/types.h:689-706`).
It carries parent GObjs, kind/hold kind, position/previous position, velocity,
facing, damage and ground/air. `Item_80267130` initializes Item position from
`SpawnItem.prev_pos`, facing and owner (`item.c:202-204`). Laser setup then
selects a state, lifetime and per-kind ray state (`itfoxlaser.c:59-69`).
The oracle reads the final live Item image instead of reconstructing it from
the descriptor. This audit confirms the requested entity-list model.

## Record and decoded offset contract

New tick and RNG-ledger raw records always have `items`, including `[]`.
Each item is `{gobj, base, kind, kind_name, owner, bytes}`. Addresses are
uppercase `0x` hex strings; bytes use the same lowercase hex as fighters.
`kind_name` is the full enum spelling (Fox laser: `It_Kind_Fox_Laser`, 54).
Unknown numeric kinds are retained with name `?`. Regenerate/check with:

```sh
python harness/gen_item_kinds.py
python harness/gen_item_kinds.py --check
```

Ownership maps `Item.owner` to `Fighter.gobj` at +0 and then
`Fighter.player_id` at +0xC (`ft/types.h:1127,1130`), using the fighter byte
images already captured in this same CPU callback. It is the player slot,
not fighter-list index. NULL, stale/unresolved and item-owned pointers yield
`null`; the raw owner pointer is always retained in the decoded state.

The decoder preserves the entry and adds `items[i].state`. Numeric fields
use the existing typed canonical values; f32 fields preserve bits and an
approximation. Vectors expand to `.x/.y/.z`. Nothing is added to the existing
top-level 49-key fighter/RNG state. Old raw records without `items` decode
exactly as before, with no new field.

Every offset below is relative to Item*, big-endian. `types.h` in this table
means `melee/it/types.h` unless a different directory is named.

| Key | Offset | Type | Source |
|---|---:|---|---|
| `entity` | 0x004 | ptr | types.h:216-217 |
| `spawn_kind` | 0x00C | s32 | types.h:221-222 |
| `kind` | 0x010 | s32 | types.h:224-225 |
| `spawn_id` | 0x01C | u32 bits | types.h:231 (`s32 x1C`); item.c:570 |
| `motion_id` | 0x024 | s32 | types.h:240-241 (`msid`) |
| `anim_id` | 0x028 | s32 | types.h:243-244 |
| `facing_dir` | 0x02C | f32 | types.h:246-247 |
| `vel` | 0x040 | vec3 | types.h:261-262 |
| `pos` | 0x04C | vec3 | types.h:264-265 |
| `external_vel` | 0x058 | vec3 | types.h:267-268; added to position at item.c:1423 |
| `ground_vel` | 0x064 | vec3 | types.h:270-271; platform displacement, item.c:1424-1434 |
| `nudge` | 0x070 | vec3 | types.h:273-274 |
| `ground_or_air` | 0x0C0 | s32 | types.h:281-284, following two pointers |
| `prev_pos` | 0x388 | vec3 | types.h:290 + CollData 0x10, lb/types.h:202-206 |
| `env_flags` | 0x4AC | u32 bits | types.h:290 + CollData 0x134, lb/types.h:232 |
| `owner` | 0x518 | ptr | types.h:293-295 |
| `hitbox0.state` | 0x5D4 | s32 | types.h:315-320 + HitCapsule 0, lb/types.h:32 |
| `hitbox0.damage` | 0x5E0 | f32 | types.h:316 + HitCapsule 0xC, lb/types.h:35 |
| `hitbox1.state` | 0x710 | s32 | types.h:316 |
| `hitbox2.state` | 0x84C | s32 | types.h:316 |
| `hitbox3.state` | 0x988 | s32 | types.h:316 |
| `hitbox_count` | derived | u32 | count states != Disabled (0), lb/forward.h:71-78; four slots, types.h:320 |
| `reflect_gobj` | 0xC64 | ptr | types.h:377 |
| `hitlag_frames` | 0xCBC | f32 | types.h:397 |
| `atk_victim` | 0xD04 | ptr | types.h:420-422 |
| `physics_callback` | 0xD18 | ptr | types.h:437-438 |
| `collision_callback` | 0xD1C | ptr | types.h:440-441 |
| `life_timer` | 0xD44 | f32 | types.h:478 |
| `flags` | 0xDC8 | u32 | types.h:530; bit layout types.h:34-64 |
| `laser.prev_pos` (Fox laser only) | 0xDE0 | vec3 | types.h:568,667 union +0xC; it/itCharItems.h:240 |

`prev_pos` is the collision routine's previous **step**, not universally the
previous tick. In particular, +0x58 is additive movement, not previous
position. Fox laser physics saves its pre-integration position in its own
union, so `laser.prev_pos` is the appropriate independent tick check.
`life_timer` is recorded as stored; its decrement/enabling is kind-dependent.
There is no active-hitbox count word: +0xAC8 is the **hurtbox** count and is
not used as the hitbox count. First damage always means slot zero, including
its stored value when disabled.

## Validation and compatibility

Every item gets finite position/velocity, unique identity/address and
metadata-vs-state checks. A spawn ID surviving adjacent ticks must keep its
GObj, Item pointer and kind. The same address with a new spawn ID is a
despawn plus spawn, not continuity. Reordering the list is allowed. A first
observation is reported at its observed tick; initial items are explicitly
labelled. Despawn is the first absent tick; the last sample does not imply
destruction. An item created and destroyed entirely between two snapshots
cannot be observed by this tick-boundary oracle.

The motion diagnostic is deliberately source-scoped to **Fox laser free
flight**, not an assertion that every Item moves by velocity alone.
It requires unchanged motion/owner/facing/velocity, the known laser callback
addresses, airborne state, no recorded hitlag/held/hit flags, environment or
hit/reflection references, no extra displacement, and lifetime >1. The laser
collision callback sets lifetime to 1 on a terrain hit. Eligible transitions
check new position against old position plus current recorded velocity and
the saved laser previous position against the old position. Tolerances are
`rel_tol=1e-6`, `abs_tol=1e-5`, reading canonical bits, never approximations.
All other continuing-item transitions are counted as motion-skipped; their
identity checks still run. Other item kinds need a callback audit before
this motion diagnostic can cover them. The future simulator comparator
remains out of scope; full raw bytes are available for its chosen keys.

`--items` only adds validator event/coverage output; item validation itself
is automatic. Legacy traces report `items: not recorded (legacy trace)`
when asked for events. The normal validator output is unchanged for legacy
and empty-item scenes.

Read-only Rust inspection:

- `melee-sim/src/initial_state/mod.rs:66-74` parses the raw boundary into
  `serde_json::Value`; its fighter selection ignores extra top-level keys.
- `melee-sim/src/inputs.rs:69-85` likewise parses each decoded record into
  `serde_json::Value` and selects `inputs`.
- `melee-sim/src/trace.rs` calls `melee_diff::read_trace`.
  `melee-diff/src/lib.rs:17-22,165-176` deserializes `Record {frame, phase,
  state}` without `deny_unknown_fields`, so absent, empty and populated
  `items` are all tolerated and ignored by today's comparator. Its 49-key
  state schema remains unchanged. Scenario TOML's `deny_unknown_fields`
  applies to scenarios, not trace JSON.
- The existing Rust comparator was also run on 300-record temporary copies
  with both empty and populated item fields: both returned
  `OK: 300 records match`. These are compatibility fixtures, not recordings.

`record.py` already invokes `run_scenario.py --tick-trace`; that runner
decodes then validates (with `--scripted` for input scenarios and a 256-draw
RNG bound). The new test replaces only subprocess execution with a fake
capture, then exercises the actual wrapper, decoder and validator and
checks the lifecycle output. Particle and bone snippet files are untouched.

## Proposed scenario for Claude (not written under scenarios)

Proposed `harness/scenarios/laser_fd_fox.toml`:

```toml
# idle_fd_fox.sav: P1 Fox (-60, facing right), P2 Fox (+60, idle).
# Inputs are VI callbacks from load; actual consumed pads are in the trace.
name = "laser_fd_fox"
savestate = "harness/roms/idle_fd_fox.sav"
frames = 300
seed = 1
stage = "FinalDestination"

inputs = [
  { frame = 30, port = 0, buttons = { B = true } },
  { frame = 31, port = 0, buttons = {} },
]

[[fighters]]
slot = 0
kind = "Fox"
controller = "scripted"

[[fighters]]
slot = 1
kind = "Fox"
controller = "idle"
```

This uses the existing Fox-vs-Fox savestate; selecting Marth in TOML does
not change a savestate's fighters. For a Marth defender, Claude must first
record a matching Fox-vs-Marth savestate. B is held for one VI and released;
the remaining ticks are neutral to observe the laser's crossing/hit/despawn.
Verify a single **kind 54** projectile (the blaster is itself another item),
its owner's slot 0, rightward travel and the defender's damage change.
The hit timing and capture throughput are not yet Dolphin-verified.

Only Claude should run these after writing the scenario in the main checkout:

```sh
cd harness
uv run python record.py scenarios/laser_fd_fox.toml
uv run python validate_ticks.py traces/laser_fd_fox.tick.expected.jsonl --scripted --max-draws 256 --items
```

## Commands and final results

Commands ran from `/Users/nikhilunni/Projects/melee-lanes/chars`. The shell
has no `python` executable and system `python3` lacks pytest, so harness
commands used the existing main checkout's interpreter without installing
anything:

```sh
export PATH=/Users/nikhilunni/Projects/melee/harness/.venv/bin:$PATH
python -m pytest harness/tests
python harness/gen_item_kinds.py --check
cargo gate
cargo clippy --workspace --all-targets -- -D warnings
```

Final harness output: `220 passed in 0.88s` (186 existing + 34 new).
Kind table: `PASS: ItemKind table matches it/forward.h`.
`cargo gate` passed both before and after: 149 completed suites, 941 passed,
0 failed, 3 pre-existing ignored tests; both commands exited 0. Final
nonblank output line:

```text
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Clippy exited 0 with:

```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in 20.60s
```

Logs: `/private/tmp/c9-pytest-final.log`,
`/private/tmp/c9-cargo-gate-before.log`, `/private/tmp/c9-cargo-gate-after.log`,
`/private/tmp/c9-clippy.log`. `git -c core.fsmonitor=false diff --check`
also exited 0 with no output. The read-only scope check
`git -c core.fsmonitor=false diff --exit-code -- harness/record.py harness/dolphin_particle_snippet.py harness/dolphin_bones_tick_snippet.py crates/ harness/scenarios/`
exited 0 with no output.

Before editing, the decoder and both validator modes were saved under
`/private/tmp/c9-jab-before.jsonl`, `c9-validate-before.log`, and
`c9-validate256-before.log`. After editing:

```sh
python harness/decode.py harness/traces/jab_fd_marth.tick.raw.jsonl /private/tmp/c9-jab-after.jsonl
cmp /private/tmp/c9-jab-before.jsonl /private/tmp/c9-jab-after.jsonl
python harness/validate_ticks.py /private/tmp/c9-jab-after.jsonl --scripted > /private/tmp/c9-validate-after.log
cmp /private/tmp/c9-validate-before.log /private/tmp/c9-validate-after.log
python harness/validate_ticks.py /private/tmp/c9-jab-after.jsonl --scripted --max-draws 256 > /private/tmp/c9-validate256-after.log
cmp /private/tmp/c9-validate256-before.log /private/tmp/c9-validate256-after.log
```

All three `cmp` commands: exit 0, no output (byte-identical). The default
64-draw validator exits **1 both before and after**, ending:

```text
ordinal 124: rng.seed 3519480427 -> 380938109 not reachable in 0..64 LCG draws
FAIL: 1 violations
```

With the recording runner's existing `--max-draws 256`, both versions exit 0
and end `PASS: 0 violations`. No RNG bound or expected value was changed.

The temporary compatibility copies were decoded with `items: []` and one
synthetic Item entry respectively; the raw source trace remained untouched:

```sh
cargo run -q -p melee-diff -- /private/tmp/c9-jab-before.jsonl /private/tmp/c9-jab-empty.jsonl
cargo run -q -p melee-diff -- /private/tmp/c9-jab-before.jsonl /private/tmp/c9-jab-populated.jsonl
```

Each exited 0 with `OK: 300 records match`.

The synthetic four-record lifecycle fixture from `test_items.item_trace`
was written under `/private/tmp` and checked with:

```sh
python harness/validate_ticks.py /private/tmp/c9-items-synthetic.jsonl --scripted --items
```

Exit 0, final lines:

```text
item spawn tick 101 ordinal 1: 0x80500000 id=123 It_Kind_Fox_Laser kind=54
item despawn tick 103 ordinal 3: 0x80500000 id=123 It_Kind_Fox_Laser kind=54
items: 1 spawns; 1 despawns; motion checked=1, skipped=0
PASS: 0 violations
```
