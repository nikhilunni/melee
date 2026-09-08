# CLAUDE.md

Orientation for any Claude session working in this repo. Read this file,
then `TRACKER.md`, before doing anything. `TRACKER.md` holds current
status and the task list; this file holds the rules and the map.

## What this project is

A Rust port of Super Smash Bros. Melee (NTSC-U 1.02, GameCube), built
function by function from the doldecomp `melee` decompilation and
verified **bit-exact** against the retail game running in Dolphin. The
long-term goal is a headless, deterministic simulator for AI research and
a modifiable game. The near-term goal is matching the retail game's state
to the bit over recorded scenarios.

The port is verified, not trusted. Nothing merges with a known divergence.

## Repository map

| Path | What |
|---|---|
| `TRACKER.md` | Milestones, tasks, blockers, decisions, session log. **Update it every session.** |
| `crates/` | Rust workspace, layered for build speed (rules below). |
| `crates/gekko-math` | Bit-exact PowerPC float semantics and MSL libm. All arithmetic goes through here. |
| `crates/hsd-types`, `crates/melee-types` | Leaf type crates: vectors, enums, ids. No logic. |
| `crates/hsd-archive` | `.dat` archive parsing. The only crate that knows on-disc layout. |
| `crates/hsd-gobj`, `crates/hsd-anim` | HSD engine: scheduler, scene graph, animation. |
| `crates/melee-lb/mp/gr/it/ft/cpu` | Melee subsystems, one crate each, mirroring decomp directories. |
| `crates/ft-<name>` | One crate per playable character. |
| `crates/melee-sim` | Headless simulator binary. Emits canonical traces. |
| `crates/melee-diff` | Compares two traces, reports first bit-level divergence. |
| `crates/slp` | Slippi replay parser. Produces scenarios and expected traces. |
| `harness/` | Python oracle tooling: Dolphin script, symbol resolver, schema generator, decoder. Run with `cd harness && uv run ...`. |
| `harness/schema/` | Field offset schemas shared by the decoder and the Rust `Snapshot` emitters. |
| `docs/` | `PLAN.md` (milestone definitions), `ORACLE.md` (verification design), `DOLPHIN.md` (emulator options, verified API), `ISO.md` (disc requirements). |
| `third_party/melee-decomp` | The decomp as a pinned submodule. **Read-only reference.** |

## Local assets (not in git)

| Path | What |
|---|---|
| `harness/roms/GALE01.iso` | Verified NTSC 1.02 disc (hashes in `docs/ISO.md`). |
| `harness/roms/sys/main.dol` | Retail executable, matches decomp `build.sha1`. |
| `harness/roms/files/` | Extracted disc filesystem, once `extract_fst.py` exists. |
| `harness/roms/*.sav` | Dolphin savestates for scenarios. |
| `harness/traces/` | Captured oracle traces and probe data. |
| `~/Projects/dolphin-scripting/build/Binaries/Dolphin.app` | Dolphin scripting fork, arm64. |

## Commands

```sh
git submodule update --init          # once
cargo gate                           # alias: cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cd harness && uv sync && uv run python -m pytest -q
cd harness && uv run python gen_schema.py --check
cd harness && uv run python symbols.py <symbol...>      # retail addresses
cargo run -p melee-diff -- expected.jsonl actual.jsonl
cargo run -p slp --bin slp-dump -- replay.slp --trace out.jsonl
# Dolphin oracle (see docs/DOLPHIN_BUILD.md for the config flags)
~/Projects/dolphin-scripting/build/Binaries/Dolphin.app/Contents/MacOS/Dolphin \
  -e harness/roms/GALE01.iso --script $PWD/harness/dolphin/trace_scenario.py
```

## Session protocol

At the start:
1. Read `TRACKER.md`. Find the current milestone and any `[!]` blockers.
2. Run `cargo gate`. If it is red, fixing it is the first task.
3. Pick a task marked `[ ]` whose dependencies are `[x]`. Mark it `[~]`
   with the date.

At the end:
1. `cargo gate` and clippy clean. Harness tests pass if you touched harness.
2. Update `TRACKER.md`: task status, any new blockers or decisions, and a
   line in the session log saying what landed and what is next.
3. Commit. One logical change per commit. Include retail addresses in the
   message when porting functions. Never commit a known divergence.

Parallel work: independent tasks (different crates, different decomp
directories) can be given to subagents at once. Each subagent touches only
its assigned paths and does not commit; the coordinating session verifies
and commits each result. Shared files (`Cargo.toml` members, `TRACKER.md`)
are edited only by the coordinator.

## Sources of truth, in order

1. **The retail assembly.** It is the only record of where the compiler
   fused multiply-adds and how it ordered float operations. The original
   `main.dol` is at `third_party/melee-decomp/orig/GALE01/sys/main.dol`
   (gitignored; re-extract from `harness/roms/GALE01.iso` if missing). Use
   the decomp's `dtk`/`objdiff` tooling, or `harness/asm.py` once written
   (see TRACKER.md).
2. **The decomp C** in `third_party/melee-decomp/src/`. Readable intent, but
   it does not show FMA contraction. Never port float math from the C alone
   without marking the sites `// FUSION AUDIT PENDING`.
3. **The symbol map** `third_party/melee-decomp/config/GALE01/symbols.txt`
   and the struct headers with their offset comments and `ASSERT_SIZE`.

## Exactness rules

- All float arithmetic goes through `gekko-math`. `std`/`libm` math
  functions (`sqrt`, `sin`, `cos`, `atan2`, `powf`, `floor`, ...) are
  banned outside `gekko-math` and tests.
- Every multiply-add is checked against the retail asm before it is
  written: `cd harness && uv run python asm.py <symbol> --fused`
  (`docs/ASM.md`). Where the asm shows `fmadds`/`fmsubs`/`fnmsubs` (or the
  paired-single `ps_madd*`), use `gekko_math::fma::*` with operands in
  PowerPC order (a, c, b). Where it shows separate `fmuls`/`fadds`, write
  separate `*` and `+`. Either way cite it: `// retail 0x80022A3C: fmadds`.
  The old `// FUSION AUDIT PENDING` marker is retired; the 2026-09-08 audit
  found MWCC fused most eligible sites (sinf, cosf, sqrtf, slerp, Hermite,
  collision), changing 15-25% of results, so unaudited code is wrong code.
- Preserve double promotion. `10.0 * HSD_Randf()` computes in `f64` and
  rounds once.
- Transcribe MSL and Melee math routines literally. Do not simplify,
  reassociate, or "fix" apparent bugs (see the list at the bottom).
- Endianness, struct size, field offset, and pointer width appear only in
  `hsd-archive` and in `Snapshot` implementations. Everything else works on
  owned Rust types.
- Internal state does not have to match retail memory layout. Use enums
  for the per-character union. The `Snapshot` trait and `harness/schema/`
  handle comparison.

## Code style: write for humans

The decomp C is machine-recovered and often ugly: `x1A88`-style field
names, `ftCo_800AA320` function names, 400-line switch statements, unions
of every character's state, magic numbers, and gotos. The Rust must be
bit-exact in behaviour but must **not** mirror that shape. The user's
explicit request (2026-09-08): idiomatic, readable Rust with good
abstractions and clean organization.

- Descriptive names everywhere. `cpu.reaction_delay_frames`, not `x7C`.
  The decomp symbol and address live in the doc comment, not the identifier.
- Enums with named variants for every C int-that-is-really-an-enum
  (`melee-types` has the `c_enum!` macro). Bitflags types for flag words.
- Split by meaning, not by decomp file. One Rust module per concept
  (`ledge.rs`, `shield.rs`), files well under 2k lines, functions that fit
  on a screen. A decomp function that does five things becomes five
  named helpers plus one caller that preserves the original order.
- Named constants with a one-line comment for every magic number whose
  meaning is known (`const LEDGE_GRAB_COOLDOWN_FRAMES: u32 = 30;`). Unknown
  ones get a `// TODO(meaning)` comment, not a bare literal.
- Per-character state is an enum over per-character structs, never a
  union or a bag of `f32`s.
- Exactness constraints are expressed in types and helpers
  (`fmadds`, `f64` promotion sites, `fctiwz`) so the reader sees *why* an
  odd operation order exists. Add a short comment when the order matters.
- Tests are readable too: name them after the behaviour, keep fixtures in
  `tests/data/`, and prefer a table of cases to a wall of asserts.
- When a clean abstraction would change floating-point operation order or
  width, exactness wins; say so in a comment.

## Build-speed rules

- Layer 0 (`gekko-math`, `hsd-types`, `melee-types`): types and pure
  functions only. Change rarely; everything depends on them.
- One crate per subsystem. No crate over roughly 30k lines; split along
  the decomp's directory structure when one grows past that.
- One crate per character under `crates/ft-<name>`. They depend on
  `melee-ft` and `melee-types`, never on each other. Copy `ft-fox`.
- No cross-layer `pub use` facades. Depend on the crate you use.
- Only `melee-sim` and `melee-platform` may depend on everything.
- No proc-macro or heavy dependencies in layers 0 and 1. Dependencies build
  at opt-level 2 once; our crates at opt-level 0 incrementally.

## Porting a function

1. Pick the next function from the dependency order. The decomp's
   `tools/dep_graph.py` computes it; prefer leaves whose callees are ported.
2. Read its C, its callers, and its retail asm if available.
3. Write the Rust in the crate matching its decomp directory. Doc comment
   cites decomp path and retail address: `/// ftCo_800B63D8 (ftcpuattack.c)`.
   Give the Rust function a descriptive name; keep the address name in the
   doc so the asm can be found.
4. If golden fixtures exist under `harness/goldens/`, add a replay test.
5. `cargo gate`. If a scenario diverges, `melee-diff` names the frame,
   phase, and field. Fix before moving on.

## Delegating work

Fable subagents are expensive; the user asked (2026-09-08) to keep them to at
most one or two, only for judgment-heavy work. Bulk porting goes to **Codex**
(OpenAI, model `gpt-6-astra`) through `tools/codex-task.sh`:

1. Write the task as a prompt file in the scratchpad: the decomp functions to
   port, the crate and module to put them in, the naming and abstraction
   expectations, and the **mechanical acceptance criteria**: named tests that
   must pass (native-C reference oracle, Dolphin trace comparison, existing
   suites) plus clippy. Codex reads `AGENTS.md`, which points at this file.
2. `tools/codex-task.sh <name> <prompt-file>` runs it in the background from
   the repo root in a workspace-write sandbox. Its final report is in
   `.codex-runs/<name>.md`.
3. Review the diff yourself: `cargo gate`, clippy, read the code for style.
   Codex must never commit; you commit after verification.
4. Bit-exact tests are the guardrail. If Codex touched expected values or
   loosened a test, reject the change.

## Hard boundaries

- Never modify `third_party/melee-decomp` from this repo.
- Never commit game data: no ISO, DOL, `.dat`, savestates, or extracted
  files. `harness/roms/` and `harness/traces/` are gitignored for this.
  The disc lives at `harness/roms/GALE01.iso`; extracted files go under
  `harness/roms/files/`. Rust tests that need them must skip cleanly when
  they are absent so the gate stays green on any machine.
- Never help obtain the game from ROM sites. The disc must be owned and
  dumped by the user (`docs/ISO.md`).
- Dolphin's `frsqrte`/`fres` tables are GPLv2. Do not copy them into this
  MIT repo without an explicit decision recorded in `TRACKER.md`.

## Things that look like bugs but are not

- `sqrtf` returns negative inputs unchanged.
- `cosf`'s near-quadrant linear branch omits a pi/4 factor that `sinf` has.
  It is off by up to ~7e-5 there. Retail does this.
- `sinf(-0.0)` is `+0.0`. `fmodf(a, 0)` is `a`. `logf` of a subnormal is
  `-inf`.
- `HSD_Randi` multiplies before dividing in 32-bit signed arithmetic.
- Fighter `facing_dir` is a float that is always `1.0` or `-1.0`.
- `FighterKind`: Mario is 0, Fox is 1, Captain Falcon is 2.
- Two decomp headers carry stale offset comments (`x675` says `fp+674`;
  Item `jumped_on` shares `@at{D28}` with `entered_hitlag`). Report
  upstream; do not "fix" locally.
