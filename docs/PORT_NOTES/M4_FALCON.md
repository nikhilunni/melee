# Lane C: Captain Falcon idle, match start and movement

Implemented in `ft-captain`, with all eighteen recorded FD scene gates and
ordered particle RNG ledgers passing. Left uncommitted. Data, archive counts,
callbacks and costume details are in [FALCON_DATA.md](../../../../docs/FALCON_DATA.md).

## Changes and character boundaries

`CaptainAttributes` reads the complete 0x8C `ftData.ext_attr` block into named
Falcon Punch, Raptor Boost, Falcon Dive and Falcon Kick groups. Signed fields,
unknown values and float payloads are retained. `CaptainFalcon` implements
OnLoad, saved-state import and OnDeath; its descriptor contains the six costume
entries, 318 animation rows, 54 semantic parts and three part-animation groups.
The skeleton has 63 joints and no dynamic chains or collision spheres.
OnLoad enables walljumping, registers no items, and all four specials are
available in the retail dispatch tables. Their bodies remain unported.

Registration is one line in `scene_fighter.rs` plus the crate dependency.
The workspace layering is unchanged. Falcon takes Basic double jump and
ordinary guard/escape paths; no new `CharacterCallbacks` hook, override of
those defaults, or shared character-kind check was needed. The existing
explicit unsupported branches remain in place.

Two shared assumptions were exposed:

| Location | Finding and change |
| --- | --- |
| `melee-ft/fighter/assets.rs` | Only Wait1/Wait2 were loaded. Falcon's Wait table includes Wait3 (motion 4, weight 10). Load all non-sentinel Wait/SquatWait choices and their scripts from data. |
| `melee-sim/initial_state`, particle dispatch | Earlier idle boundaries were between ticks or inside Fighter ProcessHit. This save is inside the main particle emitter. Import its unfinished emission from CPU/stack state and complete it once at the main particle callback. |

## Interrupted particle emission

The first idle run rejected the saved boundary: scheduler link **15**, callback
**8005C9A4** (`efLib_particles_proc_main`). The saved CPU has PC **80326290**,
inside `cosf`; the stack's return links are **8039EC84** (spherical emission)
and **8039EF2C** (generator update). This is the final `cosf(latitude)` call
before allocation. The current generator has one remaining emission, the
geometry RNG calls have already occurred, and the particle is not yet linked.

`SavedPose::cpu_general_registers` locates the unique serialized GPR record
using retail r2/r13 SDA bases. It checks a valid stack/PC, and the continuation
checks the scheduler registers, both stack frames, callback return addresses
and captured generator identity. The serialization order was read locally
from Dolphin `PowerPCManager::DoState` (`PowerPC.cpp:83-85`); no emulator code
or math tables were copied.

The importer retains the already-stored direction X/Y, the emitter's matrix,
and the cosine argument at callee SP+8. It completes the last cosine and the
existing paired-single matrix transform, then applies the audited velocity and
position operations. The importer uses the ordinary particle constructor; the runtime continuation
calls its interpreter, updates ownership and decrements count/lifetime.
After that, ordinary scheduled updates own every subsequent tick.

No later oracle row, RNG seed replacement, rewind, draw-count shortcut or
character identity selects this path. Tick zero consumes the remaining four
ordinary primary-color draws. The independent **600-tick full particle replay
matches 858792 fields without exclusions**, proving the particle's SRT,
bytecode state, population, generator count/lifetime and produced RNG state.
All eighteen ordered ledgers also match.

The continuation deliberately requires the evidenced boundary: one generator,
no pending generator requests, a negative-radius sphere, one remaining emission
and a non-expiring generator. Different interrupted instructions or generator
conditions return explicit errors. It is not a general instruction emulator.

## Assembly audit

| Retail instructions | Meaning |
| --- | --- |
| 800E2AEC..800E2B3C | OnLoad sets walljump and copies the attribute block; no item calls or float arithmetic |
| 800E28A4 / AC / B0 | Reset model group, lunge flag, startup flag |
| 80326250 / 5C | `cosf` stack size and saved input argument |
| 8039EC60 / 7C / 84 | Emitter's saved X/Y and pending Z component |
| 8039EC94 | Existing `PSMTXMultVec` kernel |
| 8039ECA0 / B0 / C0 | Separate velocity `fmuls` |
| 8039ED18 / 28 / 38 | Position `fmadds`, retained in PPC operand order |
| 8039ED80 / A8 | Particle allocation/interpretation, then `fsubs` emission count |
| 8039EF3C / 44 | Integer lifetime decrement/store |

Read-only commands:

```sh
python3 harness/asm.py ftCa_Init_OnLoad
python3 harness/asm.py ftCa_Init_OnDeath
python3 harness/asm.py hsd_8039DAD4
python3 harness/asm.py psGenerateParticle0
python3 harness/asm.py cosf
python3 harness/asm.py hsd_8039EE24
```

## Capture notes

The supplied start sequence is correct: 0:322, 6:323, 35:324, 65:29,
78:42, 108:14. The three ledge scripts and aerial-jump script pass without
changes. The idle ledger introduces no new RNG call sites. The interrupted
idle boundary is additional information missing from the task notes.

The red costume name is language-dependent (`PlCaRe.`); this descriptor
resolves the English `.usd` variant. Both captured fighters use costume 0.
The start and idle bone tests compare all 63 Falcon and 73 Fox joints at the
tick boundary. Quaternion W follows the existing Marth contract; unused Euler
W is not meaningful. No rendered matrix words are compared. Start particle
fields were not separately replayed; its scene and ordered ledger are gated.

## Validation

All needed local assets were present; the new oracle tests actually ran.
The initial workspace baseline passed **702 tests**, with no failures and
one pre-existing ignored doctest. No existing test expectations or comparisons
were modified or weakened.

Bone and particle oracle command:

```sh
cargo test -p melee-sim --lib falcon_bones -- --nocapture
```

Final lines:

```text
start_fd_falcon: 63 Captain Falcon bones (0 dynamic), 73 Fox bones, 130 ticks, 159398 SRT words, 0 matrix words (no rendered capture)
idle_fd_falcon: 63 Captain Falcon bones (0 dynamic), 73 Fox bones, 8 ticks, 9860 SRT words, 0 matrix words (no rendered capture)
idle_fd_falcon particles: 600 ticks, 858792 fields, 0 mismatches
```

The exact CLI gates and final lines follow.

| Exact command | Final line |
| --- | --- |
| `cargo run -q -p melee-sim -- gate harness/scenarios/idle_fd_falcon.toml` | 600 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/start_fd_falcon.toml` | 600 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/squat_fd_falcon.toml` | 300 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/turn_fd_falcon.toml` | 300 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/walk_fd_falcon.toml` | 300 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/walkfast_fd_falcon.toml` | 300 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/dash_fd_falcon.toml` | 300 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/turnrun_fd_falcon.toml` | 300 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/jump_fd_falcon.toml` | 300 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/airjumpb_fd_falcon.toml` | 300 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/shield_fd_falcon.toml` | 300 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/spotdodge_fd_falcon.toml` | 300 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/roll_fd_falcon.toml` | 300 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/airdodge_fd_falcon.toml` | 300 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/wavedash_fd_falcon.toml` | 300 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/ledge_fd_falcon.toml` | 420 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/ledgeclimb_fd_falcon.toml` | 420 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/ledgeescape_fd_falcon.toml` | 420 ticks, 49 keys, 0 divergences |

| Additional exact command | Result |
| --- | --- |
| `cargo gate` | 744 passed, 0 failed, 1 pre-existing ignored doctest |
| `cargo test -p melee-sim --test m4_gate` | 141 passed, 0 failed |
| `cargo test -p hsd-particle` | 63 passed, 0 failed |
| `cargo test -p melee-ft` | 89 passed, 0 failed |
| `cargo test -p ft-captain` | 3 passed, 0 failed |
| `cargo test -p melee-sim --lib falcon_bones -- --nocapture` | 3 passed, 0 failed |
| `cargo clippy --workspace --all-targets -- -D warnings` | `Finished dev profile`, exit 0 |
| `cargo fmt --all` | exit 0 |
| `git -c core.fsmonitor=false diff --check` | clean |

The complete gate has 42 added tests: 18 scene gates, 18 ordered ledgers,
2 bone oracles, 1 full particle oracle and 3 attribute/callback tests.
No acceptance work remains blocked. The unsupported gameplay/rendering and
alternate interrupted-save conditions described above remain outside scope.

Logs: `/tmp/falcon-baseline.log`, `/tmp/falcon-final-gate.log`,
`/tmp/falcon-final-{clippy,m4,particle,ft}.log`, `/tmp/falcon-cli.log`,
`/tmp/falcon-bones.log`, `/tmp/falcon-attributes.log`.
Clippy's final line:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.10s
```

## Changed files

- `Cargo.toml`, `Cargo.lock`, `TRACKER.md`
- `crates/ft-captain/Cargo.toml`
- `crates/ft-captain/src/{lib,attributes,init}.rs`
- `crates/ft-captain/tests/attributes.rs`, `tests/support/mod.rs`
- `crates/melee-ft/src/fighter/assets.rs`
- `crates/melee-ft/src/fighter/{README,M4_FALCON}.md`
- `crates/melee-sim/Cargo.toml`
- `crates/melee-sim/src/{scene_fighter,frame}.rs`
- `crates/melee-sim/src/initial_state/{mod,saved_pose,particle_resume}.rs`
- `crates/melee-sim/src/frame/falcon_bones.rs`
- `crates/melee-sim/tests/m4_gate.rs`
- `docs/FALCON_DATA.md`

No harness code changed, so harness Python tests were not run. Traces, ROMs,
scenarios and the decomp submodule were not modified. The worktree's existing
asset symlinks still appear in Git status as described in CLAUDE.md.
No Dolphin invocation, game-data additions, commits or pushes.
