# M4-T8: Marth idle and match start on Final Destination

Implemented and verified against the supplied local captures; left uncommitted.
Both `idle_fd_marth` and `start_fd_marth` report **600 ticks, 49 keys,
0 divergences**. P0 is Marth, P1 Fox; Snapshot still emits the same 24 keys
per fighter. Data details are in [MARTH_DATA.md](../../../../docs/MARTH_DATA.md).

## What assumed Fox

| Location | Assumption removed / resulting behavior |
| --- | --- |
| `melee-ft/fighter/assets.rs` | `FighterAssets::fox`, `ftDataFox`, fixed kind/part count: now `load(CharacterDescriptor, ...)`, with per-character kind, files, symbols and counts |
| `melee-ft/desc/animation.rs` | `read_fox_animations` and `FOX_ANIMATION_COUNT`: named-symbol/count reader; callers identify their own data |
| `melee-ft/desc/bones.rs` | Fox forward-count constant and five archive part groups: caller supplies counts; runtime still reserves five slots |
| `melee-ft/fighter/assets.rs` collider reader | Required a collider pointer even with count zero: empty collider arrays now permit null |
| `melee-ft/fighter/spawn.rs` | Rejected any kind except Fox: now verifies callback kind equals loaded resource kind |
| `melee-ft/fighter/mod.rs`, `ft-mars/init.rs` | Marth landing and guard kind guards: implemented landing reset and explicit unsupported guard in Marth callbacks |
| `melee-ft/fighter/commands.rs`, `assets.rs` | Fox scripts never required opcode 31 here: decode signed model group/selection and retain DObj output state |
| `ft-fox/init.rs`, new `ft-mars/init.rs` | Archive and costume metadata now belongs to each character crate |
| `melee-sim/scenario.rs`, `assets.rs` | Fox-only validation, archive/AJ resources and two costume files: ordered supported character slots and per-slot archives/costume tables |
| `melee-sim/initial_state/{mod,fighter}.rs` | Raw kind 1 and costume == slot, one Fox resource set: check kind per slot, validate costume independently, generic boundary import |
| `melee-sim/initial_state/mod.rs` | Every idle save paused at link 14 and matched raw tick zero: additionally handle completed idle boundaries by importing saved Fighter bytes and executing the first full tick |
| `melee-sim/scene_fighter.rs`, `frame.rs` | `[Fighter<Fox>; 2]`: enum of boxed concrete fighters, generic proc dispatcher, unchanged scheduler registrations |
| `melee-sim/effects.rs` | Fox-typed flush and attachments: generic flush and character-independent bone-matrix access |
| `melee-gr/{desc,music}.rs`, sim setup | Original start save did not draw for FD music: saved unlock mask and StageParam now drive the pending setup selection |
| Fox tests and `melee-sim/bones.rs` | Call generalized readers with explicit Fox metadata; existing expected values and comparisons retained |

`SceneFighter` is an enum because the supported character set is closed and
small. Boxing avoids copying large states; generic dispatch retains each
character's monomorphised `Fighter<C>` without duplicating its gameplay API
as an object-safe trait. Only scene composition selects variants. Shared
states access character behavior through `CharacterCallbacks`, or numeric
differences through owned data. The rationale is also on the enum itself.

## Retail behavior and corrections to the notes

Marth's observed sequence matches: Entry 322 at 0, EntryStart 323 at 6,
EntryEnd 324 at 35, Fall 29 at 65, Landing 42 at 82, Wait 14 at 112.
P1 Fox reaches those states at 0,11,40,70,80,110. The slower Marth fall follows
its gravity, terminal speed, scale and ECB descriptors; no state timing was
hardcoded. EntryStart's animation holds at 10; Entry and EntryEnd have SM_None
and frame -1, as in the earlier Fox report.

The entry request remains **0x43E -> common effect 0x24** for both characters
(`ft_0C31.c:129-131`, `efasync.c:750-756`). Landing is **0x404 -> effect 0x18**.
Marth's different scale/data exercises existing particle paths; no new effect
ID, particle opcode, emitter arithmetic or dynamic-bone solver arithmetic was
needed. Both full 600-tick ordered particle RNG ledgers match independently
of the 49-key seed comparison.

Three boundary details were absent or different in the task notes:

- Both captures use costume **0 for P1 and P2**, not costume == slot.
- Idle Marth's savestate is between ticks (current proc null, link 24).
  Marth's saved animation frame is 46, raw tick zero is 47. Import now uses
  saved memory and executes a complete first tick; tick zero remains gated.
  The old idle Fox partial-tick path remains supported and tested.
- Start tick zero has one RNG call at **801C26AC**,
  `Ground_801C24F8+0x1B4`. This is music selection, not a fighter effect:
  `gm_80164ABC` tests all eleven unlockable-character bits. The new save has
  them unlocked; the older Fox save does not. FD's StageParam selects rule 6
  and a 12-percent alternate-music chance. Read the saved unlock mask through
  `gmMainLib_804D3EE0 + 0x1868`, never through fighter kind. Retail instructions
  at 8015CC44 and 8015ED90 confirm **0x1868**; gm/types.h's `thing` comment
  says +1898 and is stale. The saved seed 64080096 advances to F5F87501 by
  the actual `HSD_Randi(100)` call before the first observation.

## Bones and arithmetic audit

All **90 Marth bones** and **73 Fox bones** match at every dumped boundary.
Marth has three dynamic chains, roots **55,45,50**, each four joints, and
zero dynamic collision spheres. All twelve dynamic joints pass through the
existing ported solver. No claim is made about specific accessory names not
encoded in the descriptor's numeric joint references.

| Capture | Ticks | Local SRT words matched | Rendered matrix words |
| --- | ---: | ---: | ---: |
| `start_fd_marth.bones.jsonl` | 130 | 190,936 | 0 |
| `idle_fd_marth.bones.jsonl` | 8 | 11,736 | 0 |
| Total | 138 | 202,672 | 0 |

The tests run under `melee-sim` to include the real mixed-character scheduler
and all RNG consumers. Like the Fox SRT test, they exclude only unused Euler
W; quaternion W is exact when enabled. No later pose or ledger seed feeds the
runtime. The matrix half is deliberately absent because only tick-aligned
SRT captures exist; cached tick matrices cannot substitute for rendered ones.

New behavior consists of data reads, integer decode/reset and one conditional
HSD RNG call. No new floating-point arithmetic needs a fusion port. Audited
with `harness/asm.py`: `Ground_801C24F8` (including 801C26AC),
`gmMainLib_GetSaveData`, and `gmMainLib_GetUnlockedCharactersBitmaskPtr`.
Existing gravity, scaling, effects and springs retain their audited arithmetic.

## Validation

All local assets were present; these were actual oracle runs, not asset skips.

| Command | Result |
| --- | --- |
| `cargo run -q -p melee-sim -- gate harness/scenarios/idle_fd_marth.toml` | 600 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/start_fd_marth.toml` | 600 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/idle_fd_fox.toml` | 600 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/start_fd_fox.toml` | 600 ticks, 49 keys, 0 divergences |
| `cargo test -p melee-sim --test m4_gate` | 31 passed, zero failed (including both Marth gates and ordered ledgers) |
| `cargo test -p melee-sim --lib marth_bones -- --nocapture` | 2 passed; all 202,672 SRT words |
| `cargo test -p melee-ft --test start_fox_bones_130` | 4 passed, zero failed |
| `cargo test -p hsd-particle` | 58 passed, zero failed |
| `cargo test -p melee-sim --test m2_gate` | 1 passed, zero failed |
| `cargo gate` | 609 passed, zero failed, one pre-existing ignored doctest |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo fmt --all` | success |
| `git -c core.fsmonitor=false diff --check` | clean |

Baseline was 600 passed, zero failed, one ignored. Nine tests were added:
two attribute/reset tests, two Marth SRT tests, four scene/ledger tests and
one music RNG test. Existing test changes only pass explicit metadata to the
generalized reader APIs; no expected bits or coverage changed.

Logs are under `/tmp/melee-m4-t8-{final-gate,clippy}.log` and
`/tmp/melee-m4-t8-acceptance-{0..8}.log`. Harness Python tests were not run
because no harness code changed.

## Limits

Marth special-move bodies, combat, items and shield model behavior remain
unported. All four special capabilities are present; unsupported transitions
still stop explicitly. Marth registers no items and cannot walljump.
OnLoadForRoy shares the attribute layout, not a new playable Roy implementation.
DObj selection is retained renderer output, like the pre-existing texture
requests; full mesh/sword-trail rendering remains M8. New Marth particle-field
dumps were not replayed field-by-field; their ordered RNG sites and resulting
scene seeds are gated, alongside all existing particle-field regressions.

No tests were weakened. No changes under harness, roms, traces, scenarios or
the decomp submodule. No Dolphin invocation, commits or pushes.

## Changed files

- `Cargo.lock`
- `Cargo.toml`
- `TRACKER.md`
- `crates/ft-fox/src/init.rs`
- `crates/ft-mars/Cargo.toml`
- `crates/ft-mars/src/attributes.rs`
- `crates/ft-mars/src/init.rs`
- `crates/ft-mars/src/lib.rs`
- `crates/ft-mars/tests/attributes.rs`
- `crates/ft-mars/tests/support/mod.rs`
- `crates/melee-ft/src/desc/animation.rs`
- `crates/melee-ft/src/desc/bones.rs`
- `crates/melee-ft/src/fighter/M4_MARTH.md`
- `crates/melee-ft/src/fighter/README.md`
- `crates/melee-ft/src/fighter/assets.rs`
- `crates/melee-ft/src/fighter/commands.rs`
- `crates/melee-ft/src/fighter/mod.rs`
- `crates/melee-ft/src/fighter/procs.rs`
- `crates/melee-ft/src/fighter/spawn.rs`
- `crates/melee-ft/tests/animation_desc.rs`
- `crates/melee-ft/tests/bone_desc.rs`
- `crates/melee-ft/tests/fighter_support/mod.rs`
- `crates/melee-ft/tests/idle_ground_fields_600.rs`
- `crates/melee-ft/tests/real_fox_data.rs`
- `crates/melee-ft/tests/real_fox_wait_playback.rs`
- `crates/melee-gr/src/desc.rs`
- `crates/melee-gr/src/lib.rs`
- `crates/melee-gr/src/music.rs`
- `crates/melee-lb/tests/real_fox_wait.rs`
- `crates/melee-sim/Cargo.toml`
- `crates/melee-sim/src/assets.rs`
- `crates/melee-sim/src/bones.rs`
- `crates/melee-sim/src/effects.rs`
- `crates/melee-sim/src/frame.rs`
- `crates/melee-sim/src/frame/marth_bones.rs`
- `crates/melee-sim/src/initial_state/fighter.rs`
- `crates/melee-sim/src/initial_state/mod.rs`
- `crates/melee-sim/src/lib.rs`
- `crates/melee-sim/src/scenario.rs`
- `crates/melee-sim/src/scene_fighter.rs`
- `crates/melee-sim/tests/m4_gate.rs`
- `docs/MARTH_DATA.md`
