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
cargo gate --release                 # same bit-exact oracles under optimization
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

## The gates, in one command each

Missing local oracle files fail tests with the first missing path and its recovery command; contributors without the disc may explicitly opt out with `MELEE_ALLOW_MISSING_DATA=1`.
`tools/merge-check.sh` rejects that variable whenever it is set, so a green merge chain requires the data oracles to run.

```sh
cargo gate                                                   # every unit/oracle test
tools/perf-gate.sh                                            # separate release performance/size/instantiation regressions
cargo gate --release                                         # every unit/oracle test, optimized
tools/check-release-math.sh                                  # fused semantics at every opt level
cargo run -q -p melee-sim -- gate harness/scenarios/idle_fd_fox.toml   # M3: 600 ticks x 49 keys vs Dolphin
cargo run -q -p melee-sim -- gate harness/scenarios/start_fd_fox.toml  # match start: entry, fall, landing, idle
cargo test -p melee-ft --test start_fox_bones_130                      # 73 bones incl. tail dynamics vs Dolphin
cargo test -p hsd-particle --test live_fd                    # particle system vs 600 dumped frames
cargo test -p melee-sim --test m2_gate                       # M2: Fox bones vs Dolphin
```

Milestones 1-3 and the match-start scenario passed on 2026-09-09. Their traces are machine-local
(`harness/traces/`, gitignored); `docs/DOLPHIN_RUN.md`, `docs/M2_GATE.md`,
`docs/PARTICLES_DUMP.md` say how to re-record each one.

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
- **Character differences go through the `CharacterCallbacks` trait, never
  a `match kind` in shared code.** Retail's ftCommon states are shared by
  every character and branch on `fp->kind` in ~170 places (Yoshi's shield,
  Ness/Peach/Mewtwo/Yoshi double jumps, Samus/Yoshi rolls, walljumpers).
  Each such branch becomes a trait method with a default implementation in
  `melee-ft` (e.g. `aerial_jump_variant()`, `shield_shape()`), overridden in
  the character's `ft-<char>` crate. Numeric differences stay in the attribute
  data. Special moves are per-character modules reached through the trait.
  A branch the current scenarios never take may stay an explicit
  `unimplemented!` with the C line, but when it is ported it becomes a hook.
- **Motion states are tables, not matches.** A state is a `MotionRow<C>`
  (anim id, flags, move id, five `fn(&mut Fighter<C>)` callbacks), exactly
  retail's `MotionState`. The common table lives in `melee-ft`; a character
  adds rows through `CharacterCallbacks::special_rows()` and enters them
  from the `enter_special` hook. Dispatch is a table lookup; never a
  `match` on the state or the kind to pick a callback.
- **Kinds that share retail code share a family crate.** Fox and Falco
  (retail `ftFx_`), Marth and Roy, Mario and Dr. Mario, Pikachu and Pichu,
  Link and Young Link: the shared states live in `ft-<family>` as functions
  generic over a small family trait (item kinds, sound ids, attribute
  accessor, typed per-move scratch); each character crate implements the
  trait and instantiates the table. Character crates never depend on each
  other. Retail's `switch (kind)` inside the family code becomes a trait
  constant or method.
- **Per-move scratch is typed.** Retail's `fp->mv` union becomes a named
  struct in the character payload (`Fox { laser: SpecialNeutral, .. }`),
  visible only to that character's callbacks.
- **Concrete core, thin generic shell.** Code that never calls a character
  hook (physics, environment collision, animation player, subaction
  interpreter, hitboxes) takes the non-generic fighter core, not
  `Fighter<C>`. Only hook-calling code is generic. Monomorphization is the
  one real cost of the trait design; keep the generic surface thin and
  measure it (`cargo llvm-lines`, budget in `docs/PERF.md`).

## Build-speed rules

- Layer 0 (`gekko-math`, `hsd-types`, `melee-types`): types and pure
  functions only. Change rarely; everything depends on them.
- One crate per subsystem. No crate over roughly 30k lines; split along
  the decomp's directory structure when one grows past that.
- One crate per character under `crates/ft-<name>`. They depend on
  `melee-ft`, `melee-types` and (when retail shares the code) their
  `ft-<family>` crate, never on each other. Copy `ft-fox`.
- No cross-layer `pub use` facades. Depend on the crate you use.
- Only `melee-sim` and `melee-platform` may depend on everything.
- No proc-macro or heavy dependencies in layers 0 and 1. Dependencies build
  at opt-level 2 once; our crates at opt-level 0 incrementally.

## Zero-cost rules (the tick path)

The abstractions above must cost nothing at run time; the checks below
keep it that way. Design notes and code sketches: `docs/STEEL_THREAD.md`.

- Static dispatch only in the tick path: generics, associated consts and
  fn-pointer tables. No `dyn`, no `Box`, no `Rc<RefCell<..>>` per tick.
  Heterogeneous runtime kinds (items, scene fighters) are an enum built by
  one macro (`scene_characters!` is the model), not trait objects.
- No heap allocation after initialization. Retail used fixed pools for
  items, effects and particles; so do we (`[T; N]`, fixed-capacity vectors).
  An allocation-count test asserts zero allocations per tick on a 600-tick
  scene once it exists (C5).
- Tables are `static`/`const` data (retail's `.data` tables), rows are
  `Copy`, indices are enums so bounds checks vanish.
- Rust does not enable fast-math or implicit FMA contraction. LLVM can
  still fold negations around an explicit FMA incorrectly for signed zero
  (C14); use the audited `gekko-math` helpers and gate both profiles.
  Fused ops stay explicit (`fmadds`), never inferred. Changes to LTO,
  codegen units or CPU targeting must also pass the optimized oracles.
- Performance, binary size and instantiation counts are regression gates
  (`tools/perf-gate.sh`, `docs/PERF.md`), not bit-exact ones; they fail on
  regression against the recorded baseline.

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
5. **One Codex task at a time.** Two concurrent runs each doing cargo builds
   got the OS to kill them for memory (2026-09-08; the user's IDE holds ~14 GB).
   Codex sessions survive. To continue one, from the repo root:
   `codex exec resume <thread_id> -c 'sandbox_mode="workspace-write"' --json
   -o .codex-runs/<name>.md "<what happened, what to finish>"` (the thread id is
   on the first line of `.codex-runs/<name>.jsonl`; `resume` takes no `-C`/`-s`).

### Parallel lanes (worktrees)

Codex tasks that touch disjoint crates run concurrently, one per git
worktree under `../melee-lanes/<lane>` (branches `lane/<lane>`), created
from `main` with `harness/roms`, `harness/traces` and
`third_party/melee-decomp` symlinked to the main checkout (so recordings and
the retail asm split are shared; those three show as untracked/typechange in
the lane, which is expected). Launch with the lane's own
`tools/codex-task.sh`; its `.codex-runs/` is per lane. Recording (Dolphin)
happens only in the main checkout. Claude merges: review in the lane, rebase
the lane branch onto `main`, run the full gates on the merged tree, commit
on `main`. Lanes never commit on their own and never touch each other's
crates; if two lanes drift into one file, Claude resolves it.
**Never `git add harness`, `git add .` or `git add -A` in a lane.** The
lane's `harness/roms` and `harness/traces` are symlinks to the main
checkout; on 2026-09-09 a `git add harness` committed them, the
fast-forward into `main` replaced the real directories with
self-referential symlinks, and git deleted the ignored contents: the disc
image, every savestate and every recorded trace. Add files by explicit
path, run `git show --stat` on the lane commit and refuse to merge if it
lists `harness/roms`, `harness/traces`, `third_party/melee-decomp` or any
path you did not expect; `git ls-files harness/roms harness/traces` must
print nothing on `main` before any fast-forward. Keep a copy of
`harness/roms` and `harness/traces` outside the repo (`~/melee-data/`) and
refresh it after every recording session.
Before fast-forwarding `main`, run `tools/merge-check.sh <lane-branch>`
in the lane worktree. It refuses tracked or touched game-data and decomp
paths and an empty traces directory, checks ancestry and the checked-out
revision, then runs the strict build, gate, M4, M5, particle, clippy and
format chain, rejecting failure markers even when a command exits
successfully. A lane that passed in isolation can be broken by a sibling's
struct change; the merged tree must pass this script. The script never
rebases or commits.

## Hard boundaries

- Never modify `third_party/melee-decomp` from this repo.
- Never commit game data: no ISO, DOL, `.dat`, savestates, or extracted
  files. `harness/roms/` and `harness/traces/` are gitignored for this.
  The disc lives at `harness/roms/GALE01.iso`; extracted files go under
  `harness/roms/files/`. Rust tests that need them follow the strict missing-data policy in
  "The gates" above.
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
