# Cold start: Lane B3

Follow-up: [lane B4](SLIPPI.md) adds replay pads, gapped ports/spawn markers,
stock/time rule carriage, and the normal countdown's input release. The
historical B3 measurements and boundary limitations below remain unchanged.

2026-09-09. The four cold scenarios construct their match boundary using only
scenario parameters and owned DAT assets. Each gates against its existing
trace for **600 ticks, 49 keys, 0 divergences**. Construction reads no savestate,
sidecar, fighter capture, particle capture, expected trace or RNG ledger.

## Constructor and scenario contract

`InitialState::from_parameters` in `melee-sim/src/initial_state/cold.rs`
composes Ground setup, Player slots and shared `Fighter::spawn_for_match`.
Stage behavior stays in `melee-gr`, fighter creation in `melee-ft`, character
assets behind the existing composition enum, and particle creation in
`hsd-particle`. `savestate` omitted selects cold setup; `expected` names the
independent comparison trace. `run` needs DAT assets only; `gate` additionally
reads the expected trace for comparison.

The supported contract is Final Destination or Battlefield, two ordered human
ports 0/1, registered Fox/Marth/Falco characters and valid costumes, one stock,
items off, normal scale/damage, normal Entry and neutral inputs. The seed must
be explicit and located at the pre-music boundary described below. The music
unlock flag `all_characters_unlocked` must also be explicit: it cannot be
inferred from stage and character parameters. Forced music is outside this
constructor's contract.

## CPU initialization correction

The two old spawn-test seed assertions were self-authored from incomplete C
analysis, not oracle-derived. The user authorized correcting exactly those
assertions from `0xB3E97B5B` to `0x5B3F58B2` for starting seed `0x12345678`.
Retail initializes human and CPU slots with two draws:

1. `ftCo_800A101C` calls `HSD_Randf` at **0x800A123C** for the reaction timer.
2. Its unconditional call at **0x800A1744** reaches `ftCo_800B9704`
   (`ftcpuattack.c:2098-2106`), which calls `HSD_Randf` at **0x800B9718**.

`CpuState.attack_delay` represents Fighter+0x1ABC. The port computes
`(10 - level) * (rand * 15 + 15) + 10` using retail's two `fmadds`
(**0x800B9734 / 0x800B974C**) and `fctiwz` (**0x800B9750**), then halves
the integer toward zero in mode 7. The saved-boundary adapter reads this
field independently for the comparison test. New CPU tests cover level and
mode variation and truncation before halving. No other existing expectations
were changed.

## Seed boundary and setup order

The scenario seed requested for these cold cases is the **saved boundary
seed**, after stage and fighter creation but before pending music selection.
It is not a measured stage-select seed. `slp::scenario::to_scenario_named`
already distinguishes `seed` (first-frame seed) from `game_start_seed`
(Game Start event seed); the two phases must not be conflated.

The retail path is `gm_Scene_Vs_OnEnter` -> `fn_8016E730`
(`gm/gm_16AE.c:1970-2037`): initialize services and Player slots, load the
stage, run `Stage_8022524C` to create Ground objects, create fighters through
`fn_8016E2BC` / `Player_80031AD0`, then finish setup and select music through
`Stage_80225074`. `Player` has no direct RNG calls in this path.

The following sequence is supported by the initial captures and C/assembly:

| Order | Consumer | Draws |
|---|---|---:|
| Stage init, FD | `grLast_8021AC30`: pitch magnitude, pitch sign, yaw magnitude, yaw sign | 4 |
| Stage init, Battlefield | `grBattle_BG_Callback0`: `HSD_Randi(1200) + 2400` | 1 |
| P0 fighter creation | `ftCo_800A101C` reaction timer, then `ftCo_800B9704` attack delay | 2 |
| P1 fighter creation | Same two calls, in the same order | 2 |
| Saved boundary | Both fighters in Entry; scheduler current proc null, link 24 | 0 |
| Pending music | `Ground_801C24F8`: conditional `HSD_Randi(100)` | 0 or 1 |
| Observation zero | Match tick-counter reset; no fighter or particle scheduler pass | 0 |

LCG reversal uses `previous = (seed - 2531011) * 0xB9B33155`, modulo
2^32; `0xB9B33155` is the multiplicative inverse of 214013. During the audit, a read-only diagnostic compared imported boundaries with
stage initialization from candidate predecessor seeds. All three
FD background structs match their cold initializer exactly eight draws
before the boundary. Battlefield's nearest matching timer is five draws
before the boundary. Its coarser integer timer also matches predecessors
433 and 519 in the searched 0..999 interval; the timer alone is not a
unique seed identification. The source call order and the two CPU reaction
timers provide the additional evidence.

| Scene | Saved seed | Seed immediately before stage init | P0/P1 reaction timer | Music draw before observation 0 |
|---|---|---|---|---:|
| `start_fd_fox` | `0xCC51A0A5` | `0xBC0716BD` (8 draws back) | 3 / 6 | 0 |
| `start_fd_marth` | `0x64080096` | `0x90383B0E` (8 draws back) | 5 / 1 | 1 |
| `start_fd_falco` | `0x32577A78` | `0xFF86FDB0` (8 draws back) | 9 / 4 | 1 |
| `start_bf_fox` | `0x8AAA75D2` | `0x8034B407` (5 draws back) | 8 / 4 | 1 |

Replaying stage-init draws followed by two CPU draws per player reproduces
these boundary seeds and reaction timers. This identifies the previously
unexplained two advances; it does not establish the earlier stage-select
seed or every draw before Ground creation. No frame-counter dependency was
found in these identified initialization draws. Boot's `gmmain.c:156`
seeds from `OSGetTick`, but an explicitly supplied seed already accounts
for that earlier source of nondeterminism.

## Fighters, effects and particles

`fn_8016D8AC` increments the Entry delay by five per player: slots receive
5 and 10. Spawn positions come from the archive's map-0 marker bindings and
map scale: FD (-60,10,0)/(60,10,0), Battlefield (0,8,0)/(0,62.4,0).
`fn_8016DEEC` chooses facing toward the other player; equal-X markers resolve
in player order, giving +1/-1 on Battlefield.

`Fighter::spawn_for_match` shares the ordinary creation/reset/support probe,
then enters Entry directly, as `Fighter_Create` does when Player's entry flag
is set. It does not insert a Wait/Fall motion entry. Slot parameters, CPU
initialization, model state and collision state come from that shared path.
The ordinary `spawn` API continues to select Wait/Fall.

FD begins with empty particle lists and family counter 256. Its map-4
animation requests the first generator at scheduler tick 1. Battlefield
immediately evaluates frame zero for maps 0,1,6 during Ground creation; DAT
animation events and bank descriptors create four generators, no particles,
and family counter 260. Initial JObj attachment matrices remain cached
identities: `grLib_801C99C0` attaches pointers without requesting matrices.
The first scheduled Ground update refreshes them before the particle pass.
No initial population is copied from `.initial` captures. Neither scene has
entry-warp particles yet; the existing Entry timers produce those later.

The constructor sets `resume_s_link = 24`, the boundary past the scheduler,
and leaves music pending. Observation zero performs that pending setup and
tick reset; it does not run a full scheduler pass. Subsequent ticks use the
existing production scene scheduler.

### Inactive heap storage in the imported owners

Snapshot equality is not raw heap equality:
the importer retains Entry union words that retail has not initialized yet.
`Fighter_Create` obtains an uncleared `HSD_ObjAlloc` block at 0x80068EE8.
`ftCo_800C61B0` writes the timer, origin, model scales and collision box,
but does not write Fighter+0x2360/+0x2364/+0x2368. Their first EntryStart
writes occur at **0x800C65E4 / 0x800C6490 / 0x800C65E0**, respectively.

These are the captured bit patterns, in that offset order:

| Scene | P0 words | P1 words |
|---|---|---|
| `start_fd_fox` | `001E0000 0500000F 00001E00` | `00000000 00000000 00000000` |
| `start_fd_marth` | `001E0000 0500000F 00001E00` | `CF924F4F 5959597D 9C4A5151` |
| `start_fd_falco` | `001E0000 0500000F 00001E00` | `80CAE900 80C6E97D 80C6E978` |
| `start_bf_fox` | `DF083309 BB03DF08 3409BC03` | `80405570 00000000 00000000` |

They depend on prior heap contents, not the supplied scene parameters.
The existing 24-key fighter `Snapshot` does not expose them; therefore this
is **not** evidence that the 49-key cold trace gate is impossible. It does
mean that equality of those public snapshots would not prove equality of
every field retained by the importer. The new comparison uses the existing canonical snapshot and additionally
compares initialized Entry fields, CPU attack delay, stage fields and particle
attachment metadata. It does not compare these three inactive words. The cold
Entry constructor leaves them zero until their first write. A complete-owner
comparison would need prior heap history as another input; another seed alone
cannot reproduce those bytes. Ground snapshots likewise retain their existing
contract for inactive fade storage. This is a limit on acceptance item 2 if
"field for field" means every field retained by the importer, including
inactive storage, rather than the defined initial snapshot.

## Parameters a Slippi replay still lacks

The four new TOMLs use the real sidecar boundary seeds, with the sidecars
serving only as authoring oracles. Runtime setup reverses the fixed audited
stage/fighter draw interval, then executes those draws forward. It performs
no candidate search and has no scene-name lookup of seeds or state.

`slp::scenario::to_scenario_named` distinguishes first-frame `seed` from
`game_start_seed`. Neither phase is yet proven equivalent to this constructor's
pre-music boundary. The writer emits additional metadata and a per-frame
input schema that `melee-sim::Scenario` does not yet ingest. Replay input
integration, gaps in ports, version-specific rules and seed/frame alignment
remain corpus work. Cold scenarios currently require neutral inputs.

Slippi's parsed fields also lack the saved unlock mask and scene-entry L/R
state. `Ground_801C24F8` uses the former; `fn_8016E5C0` derives forced music
from a rules override or L/R held by all humans. Here the explicit unlock
flag is false for FD Fox and true for the other three scenes, with normal
music selection. No frame-counter dependency was found in the audited draw
interval. Boot's earlier `OSGetTick` seeding is outside it.

No further capture was needed for these four boundaries. To verify an earlier
Slippi Game Start/stage-select seed, record an RNG ledger beginning before
**fn_8016E730 (0x8016E730)**, through **Stage_8022524C**, both
**Fighter_Create** calls, **Stage_80225074**, and the first scheduler
observation. Record the initial seed, every seed write's PC/LR, StartMeleeData
rules/player slots, unlock mask and copied L/R state, and mark those function
boundaries and tick reset. The current ledgers begin too late for that proof.
Inactive heap-word equality would additionally require allocation history,
which is outside parameter-only construction.

## Mechanical verification

Each `initial_state::cold_tests::start_*_cold_600` compares the independently
constructed and imported pre-tick states: 107 scene fields for FD or 99 for
Battlefield, plus the complete existing particle snapshot and attachment IDs
and matrix bits. It then runs the 600-tick gate. These tests do not normalize
oracle values. The inactive-storage limitation above remains explicit.

`cold_run_reads_only_dat_assets` runs all four 600-tick simulations in a
temporary root containing only a link to owned DAT files. There is no traces
directory or savestate in that root. This verifies the construction/input
path independently of comparison captures.

```sh
cargo run -q -p melee-sim -- gate harness/scenarios/start_fd_fox_cold.toml
cargo run -q -p melee-sim -- gate harness/scenarios/start_fd_marth_cold.toml
cargo run -q -p melee-sim -- gate harness/scenarios/start_bf_fox_cold.toml
cargo run -q -p melee-sim -- gate harness/scenarios/start_fd_falco_cold.toml
cargo gate
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

Verified results:

- Four CLI gates: each `600 ticks, 49 keys, 0 divergences`.
- `cargo gate`: 715 passed, 0 failed, 1 pre-existing ignored.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo fmt --all -- --check` and `git diff --check`: passed.
- Harness pytest: 186 passed using the main checkout's existing Python
  environment. Offline `uv run` in this lane could not install an uncached
  Pygments package; the existing environment supplied the test dependencies.

No commits, Dolphin runs, or changes to existing scenarios, traces, ROMs or
decomp sources were made.
