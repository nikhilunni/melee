# CLAUDE.md

Rules and map for any session in this repo. Read this, then `TRACKER.md`
(current focus and backlog), then `docs/PLAN.md` (milestones).

## What this project is

A Rust port of Super Smash Bros. Melee (NTSC-U 1.02), built function by
function from the doldecomp `melee` decompilation and verified **bit-exact**
against the retail game running in Dolphin. Goal: a headless, deterministic
simulator for AI research and a modifiable game. The port is verified, not
trusted: nothing merges with a known divergence.

## Repository map

| Path | What |
|---|---|
| `TRACKER.md`, `docs/PLAN.md` | Current focus, backlog, milestones. Update the tracker when work lands. |
| `crates/gekko-math` | Bit-exact PowerPC float semantics, MSL libm, matrix kernels. All arithmetic goes through here. |
| `crates/hsd-types`, `melee-types` | Leaf type crates: vectors, enums, ids. No logic. |
| `crates/hsd-archive` | `.dat` parsing. The only crate that knows on-disc layout. |
| `crates/hsd-gobj`, `hsd-anim`, `hsd-particle` | HSD engine: scheduler, scene graph and animation, particles. |
| `crates/melee-{lb,mp,gr,ft,it,ef,cm,cpu,if,cmd,coll}` | Melee subsystems, one crate each, mirroring decomp directories (`cmd`: subaction decoding; `coll`: hit/hurt capsules and damage math; `cpu`: CPU behaviour, so far Nana's follow AI). |
| `crates/ft-<name>`, `ft-*-family` | One crate per character (20 registered); kinds that share retail code share a family crate (`fox`: Fox/Falco, `mars`: Marth/Roy, `mario`: Mario/Dr. Mario, `pikachu`: Pikachu/Pichu, `captain`: Falcon/Ganondorf, `link`: Link/Young Link). |
| `crates/it-<kind>` | Item and article kinds (projectiles, thrown items, stage items such as Shy Guys). |
| `crates/melee-lib` | Match composition and the curated create/step/inspect/clone API. |
| `crates/melee-sim` | Oracle tooling binary: `gate`, `triage`, `dry-run`, `search`, `particle-sites`, `particles-diff`, `bones-diff`, `replay`. |
| `crates/melee-replay` | Recording format and the corpus explorer (`--example explore`). |
| `crates/melee-platform` | Native graphical consumer (wgpu, macOS app). |
| `crates/slp`, `melee-diff`, `melee-trace-io` | Slippi parsing; trace diffing; `.jsonl`/`.jsonl.zst` trace reading. |
| `harness/` | Python oracle tooling (Dolphin scripts, recorder, decoder, bridges). Run with `cd harness && uv run ...`. |
| `docs/` | `ORACLE.md` (verification design), `DOLPHIN_RUN.md`, `ASM.md`, `PERF.md`, `INTERACTION_MATRIX.md`, `COVERAGE_AUDIT.md`, per-character `*_DATA.md`. `docs/PORT_NOTES/` holds per-task reports: look up, do not preload. |
| `third_party/melee-decomp` | The decomp as a pinned submodule. **Read-only reference.** |

Local, not in git: `harness/roms/` (disc image `GALE01.iso`, extracted
`files/`, `*.sav` savestates), `harness/traces/` (recordings, zstd), and the
headless Dolphin at `~/Projects/dolphin-scripting/build/Binaries/DolphinHeadless.app`
(`tools/build-headless-dolphin.sh`). The windowed `Dolphin.app` is only for
live human play and menu driving (`dolphin/drive.py`).

## Commands

```sh
cargo gate                         # alias: cargo test --workspace (debug); also run --release at boundaries
cargo clippy --workspace --all-targets -- -D warnings
tools/perf-gate.sh                 # release size/time/duplicate-label regressions (docs/PERF.md)
cd harness && uv run python -m pytest -q
cargo test --release -p melee-sim --test m5_gate corpus     # focused combat/corpus oracle

# Oracle loop (all melee-sim commands: cargo run -q --release -p melee-sim -- ...)
melee-sim gate harness/scenarios/<name>.toml          # compare every key against the retail trace
melee-sim triage harness/scenarios/<name>.toml        # first divergence: keys, motions, items, RNG, particles, bones
melee-sim dry-run <new.toml> --state-from <recorded.toml> --out o.jsonl   # run the port alone
melee-sim search <base.toml> --state-from <recorded.toml> --spec <spec.toml> [--out new.toml --name n]
cd harness && uv run python record_many.py scenarios/a.toml ... [--jobs 8]   # record in parallel
cd harness && uv run python record.py scenarios/<name>.toml [--bones N]      # one scene, or human play
cd harness && uv run python explore_batch.py <out> <n> <skip> [--boundary NAME ...] [--sudden-death] [--samples K]
cd harness && uv run python make_boundary.py --stage <Stage> --players <P1> <P2>   # new retail start boundary
tools/agent-merge/pick.sh <commit>                    # merge a worktree agent's commit (tools/agent-merge/README.md)
cd harness && uv run python replay_to_scenario.py <recording.json> --name <name>
cd harness && uv run python asm.py <symbol> --fused                          # retail asm (docs/ASM.md)
```

Oracle tests fail on missing local data with the path and recovery command;
contributors without the disc may set `MELEE_ALLOW_MISSING_DATA=1`
(`tools/merge-check.sh` rejects it).

## Working protocol

- Start: read `TRACKER.md`; the gate must be green before new work.
- While working, run the focused checks a change touches (its scenario gate,
  the crate's tests, the m5 corpus test). Run the full debug and release gates,
  clippy and harness tests at work boundaries, not after every change.
- End: gates and clippy clean, `TRACKER.md` current, commit. One logical change
  per commit; cite retail addresses when porting; never commit a divergence.
- Add files to git by explicit path only. Write shell loops for `bash`, not zsh.

## Verification workflow

**Finding bugs.** The explorer (random inputs over full matches from retail
boundaries) finds real divergences fastest. `explore_batch.py` runs it from
any registered boundaries (`harness/boundaries.toml`; `make_boundary.py` adds
a stage or character layout in one command), bridges the shortest cases of
each distinct fault to retail scenarios, records, gates and writes a triage
report per divergence. Fixed faults' scenarios join `CORPUS_V3_MATCHES` in
`crates/melee-sim/tests/m5_gate.rs` with a one-line reason.

**Directed witnesses.** For a branch the explorer never reaches: write a base
scenario, describe the input variations and goal in a `melee-sim search` spec
(format in `crates/melee-sim/src/search.rs`), record the result with
`record_many.py`, gate it, and register it (a named `m5_gate` test or the
`MATRIX_WITNESSES` list). The scenario's header says what retail actually does.

**Diagnosing a divergence.** Run `melee-sim triage` first. Most bugs are about
*when*, not arithmetic: effect-queue order and flush point, which proc runs a
transition, first-tick state of a new object, part-animation ownership,
staleness captured at the wrong moment. Read retail state rather than theorize:
the tick trace carries raw Item struct bytes; `record.py --bones N` adds a bone
dump for `bones-diff`; `particles-diff`/`particle-sites` show generator lists
and RNG call sites. Confirm the order against the retail asm.

**Reachability.** A branch is out of the gate only with written evidence:
the retail call graph (asm or decomp), stage geometry, or a `melee-sim search`
count ("N candidates, none reached"). Record it in `docs/INTERACTION_MATRIX.md`
or `docs/COVERAGE_AUDIT.md`. Unported branches stay `unimplemented!` with the
C line, so the explorer names them instead of diverging silently.

**Recording.** Always through `record.py`/`record_many.py` (headless Dolphin,
Null video, no audio; byte-identical to the windowed app). `record_many` gives
each Dolphin a private user folder; never start two bare `record.py` runs at
once. Human scenes record one at a time in the windowed app. Traces compress
to `.jsonl.zst` after verification. `vi_frame` in the tick trace depends on
host timing and is not game state.

## Agents

Subagents are allowed and always run on **Opus 5.5** (user, 2026-09-28). No
Codex. Use them for bounded work, as many in parallel as the work allows:
- search-only agents (dry runs and scratch files, no repo edits);
- worktree agents for one bug each, which commit on their own branch and sweep
  every recorded scenario before and after the change. The coordinator reviews,
  cherry-picks, gates and commits.

A worktree never symlinks or copies `harness/roms` or `harness/traces`: an
agent reads the main checkout's data in place (`MELEE_DATA_ROOT` for changed
harness scripts). Brief every worktree agent with `docs/AGENT_BRIEF.md` plus
its task; agents rebase onto `main` before reporting. Merge with
`tools/agent-merge/pick.sh` (it refuses game data and decomp paths), gate the
agent's scenarios and a cross-section after each merge, `m5_gate` every few
merges, and a full checkpoint to close a wave. Remove finished agents'
worktrees (`git worktree remove`) when a wave is merged: each keeps a full
build directory.

## Sources of truth, in order

1. **The retail assembly** (`harness/asm.py`; the DOL is
   `third_party/melee-decomp/orig/GALE01/sys/main.dol`, re-extract from the
   disc if missing). The only record of FMA contraction and operation order.
2. **The decomp C** (`third_party/melee-decomp/src/`): intent, not fusion.
3. **The symbol map** `config/GALE01/symbols.txt` and struct headers.

## Exactness rules

- All float arithmetic goes through `gekko-math`. `std`/`libm` math functions
  are banned outside `gekko-math` and tests.
- Check every multiply-add against the retail asm (`asm.py <symbol> --fused`).
  `fmadds`/`fmsubs`/`fnmsubs`/`ps_madd*` become `gekko_math::fma::*` with
  operands in PowerPC order (a, c, b); separate `fmuls`/`fadds` stay separate.
  Cite it: `// retail 0x80022A3C: fmadds`. Unaudited float code is wrong code.
- Preserve double promotion (`10.0 * HSD_Randf()` computes in `f64`).
- Transcribe MSL and Melee math literally; never simplify or "fix" it.
- Endianness, struct size and field offsets appear only in `hsd-archive` and
  `Snapshot` implementations. Internal state need not match retail layout;
  `harness/schema/` and `Snapshot` handle comparison.

## Code style: write for humans

The Rust must be bit-exact but must not mirror the decomp's machine-recovered
shape (user, 2026-09-08).

- Descriptive names; the decomp symbol and address live in the doc comment:
  `/// ftCo_800B63D8 (ftcpuattack.c)`.
- Enums for C ints that are enums (`c_enum!`), bitflags for flag words, named
  constants with a one-line comment for magic numbers (`// TODO(meaning)` if
  unknown).
- Split by meaning: one module per concept, files well under 2k lines,
  functions that fit on a screen; a five-part decomp function becomes five
  helpers and a caller that keeps the order.
- Exactness constraints show in types and helpers (`fmadds`, `f64` sites,
  `fctiwz`); comment where order matters. Exactness beats a cleaner abstraction.
- Tests are named after the behaviour; fixtures in `tests/data/`; tables of
  cases over walls of asserts.

## Architecture rules (shared across characters, stages, items)

- **Character differences go through the static `CharacterTable`, never a
  `match kind` in shared code.** Each retail `fp->kind` branch in ftCommon
  becomes a `CharacterCallbacks` hook with a default in `melee-ft`, overridden
  in `ft-<char>`; numeric differences stay in attribute data. A branch no
  scenario takes may stay `unimplemented!` with the C line.
- **Motion states are tables.** A state is a `MotionRow` (anim id, metadata,
  five phase callbacks); `state::COMMON` lives in `melee-ft`, characters add
  `SPECIAL_ROWS`. Entry looks up a row; never `match` on state to pick a callback.
- **Family crates** for kinds that share retail code (Fox/Falco, Marth/Roy,
  Mario/Dr. Mario, Pikachu/Pichu, Link/Young Link): shared states generic over a
  small family trait; character crates never depend on each other.
- **Typed per-move scratch** (`fp->mv` becomes a named struct in the character
  payload), accessed through checked `get::<C>()`/`get_mut::<C>()`.
- **Concrete core and shell:** hook-free math takes `FighterCore`; shared
  behaviour takes concrete `Fighter` so it compiles once. A new character adds
  one `static TABLE: CharacterTable = CharacterTable::new::<C>()`, its data and
  special rows.
- Items use static `ItemStateRow`/`ItemLogic` tables; effects go through
  `melee-ef` request queues. Shared engine behaviour (effect flush order,
  item creation, held-item visibility) lives in the shared crates, never in a
  character or item crate.

## Build and zero-cost rules

- Layer 0 (`gekko-math`, `hsd-types`, `melee-types`) holds types and pure
  functions only; no proc-macro or heavy dependencies in layers 0-1.
- One crate per subsystem, none over ~30k lines; subsystems depend directly on
  what they use. `melee-lib` is the application boundary; `melee-sim` may reach
  subsystems for oracle tooling.
- Tick path: static dispatch only (generics, consts, fn-pointer tables), no
  `dyn`/`Box`/`Rc<RefCell>`, no heap allocation after initialization (fixed
  pools), tables are `static` and rows `Copy`.
- No fast-math or implicit FMA; fused ops stay explicit. LTO, codegen-unit or
  CPU-target changes must pass the release oracles (`tools/check-release-math.sh`).
- `tools/perf-gate.sh` enforces time, size and duplicate-label budgets
  (`docs/PERF.md`); raising a baseline is the user's call, recorded there.

## Hard boundaries

- **Never delete, move or overwrite game data or recordings** without the
  user's explicit request naming them: `harness/roms/`, `~/melee-data/`,
  `harness/traces/`. No `rm -rf`, `git clean -x`, `git checkout`/`reset` or
  symlink change that could reach them; never `git add harness`, `.` or `-A`.
  Re-recording a scenario's own traces is fine; freeing disk means asking
  first, except for `target/` and scratch files. Keep `~/melee-data/roms` (disc
  image and savestates) refreshed when a savestate is added.
- Never modify `third_party/melee-decomp`.
- Never commit game data (ISO, DOL, `.dat`, savestates, extracted files).
- Never help obtain the game from ROM sites (`docs/ISO.md`).
- Dolphin's `frsqrte`/`fres` source tables are GPLv2; the committed tables were
  captured empirically by user decision.

## Things that look like bugs but are not

- `sqrtf` returns negative inputs unchanged.
- `cosf`'s near-quadrant linear branch omits a pi/4 factor that `sinf` has
  (off by up to ~7e-5). Retail does this.
- `sinf(-0.0)` is `+0.0`; `fmodf(a, 0)` is `a`; `logf` of a subnormal is `-inf`.
- `HSD_Randi` multiplies before dividing in 32-bit signed arithmetic.
- Fighter `facing_dir` is a float that is always `1.0` or `-1.0`.
- `FighterKind`: Mario is 0, Fox is 1, Captain Falcon is 2.
- Two decomp headers carry stale offset comments (`x675` says `fp+674`; Item
  `jumped_on` shares `@at{D28}` with `entered_hitlag`). Do not "fix" locally.
