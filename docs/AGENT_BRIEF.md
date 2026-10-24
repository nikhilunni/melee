# Brief for a worktree agent

The coordinator points each worktree agent at this file, then states the
agent's task (a fault tag, its scenarios, what to port). Follow `AGENTS.md`
and `AGENTS.md` in full as well.

## Data

You work in a git worktree of the main checkout
(`/Users/nikhilunni/Projects/melee`). Game data lives only there:
`harness/roms` (disc, savestates), `harness/traces` (retail recordings) and
`~/melee-data`. Never symlink, copy, move or delete any of it.

- Run the oracle from your worktree's code against the main checkout's
  scenario path; the data root follows the path:
  `cargo run -q --release -p melee-sim -- gate /Users/nikhilunni/Projects/melee/harness/scenarios/<name>.toml`
  (also `triage`, `particle-sites`, `particles-diff`, `bones-diff`, `dry-run`,
  `search`).
- To run a harness script you changed against the main checkout's data, set
  `MELEE_DATA_ROOT=/Users/nikhilunni/Projects/melee` (`harness/data_root.py`).
- Record new retail scenarios in the main checkout with
  `record_many.py` (a private Dolphin folder per run; never bare `record.py`).
  `make_boundary.py` is safe to run alongside other agents; pass
  `--no-register` and add the entry to your worktree's `boundaries.toml`.
- Crate tests that read disc data cannot find it from a worktree; run them
  with `MELEE_ALLOW_MISSING_DATA=1` there and rely on the gate and sweep. The
  `m5_gate` test binary likewise runs only in the main checkout.
- Keep scratch files under your worktree's `target/`, not `/tmp`.

- A scenario file you wrote in your worktree gates against the main
  checkout's data with `MELEE_DATA_ROOT=/Users/nikhilunni/Projects/melee`
  (also needed for `melee-sim replay`). Recording with Slippi Gecko codes
  needs `MELEE_GECKO_DIR=~/melee-data/gecko`.
- Sweeps (`melee-sim sweep`, columns: scenario, PASS/FAIL, particle
  EXACT/FAULT/NOTCMP/-, detail). The "before" is done for you: the
  coordinator's sweep of the main commit you start from is
  `/Users/nikhilunni/Projects/melee/target/sweeps/<commit>.tsv`. While
  working, sweep only what you touch, against it:
  `melee-sim sweep --scenarios /Users/nikhilunni/Projects/melee/harness/scenarios --filter <character> --baseline <that tsv>`.
  Once, at the end, sweep everything:
  `melee-sim sweep --scenarios /Users/nikhilunni/Projects/melee/harness/scenarios --out target/after.tsv --baseline <that tsv> --lock /Users/nikhilunni/Projects/melee/target/sweep.lock`.
  The lock queues agents' sweeps so each runs on every core (about 2
  minutes); wait for it in the background rather than polling.

## Slippi replay tasks

The oracle is `melee-sim replay "<game.slp>" --all-characters-unlocked true`
(it chooses the UCF version itself and prints it); the
corpus is `~/melee-data/replays/public-v3.7`, read in place. A `pN.input_seed`
stop is RNG drift on the tick before: find what drew differently on that
tick. A replay has no retail memory, so play the window on retail with the
bridge in `docs/SLIPPI.md` ("A replay played back on retail") and triage the
port against that trace; or build a smaller directed witness. Check the whole
corpus before and after with `replay-batch --jsonl`: no replay may match
fewer frames. `slippi_to_scenario.py` refuses an existing scenario name and
writes `render_every_tick = true` (a console's display cadence; a "retail
in Dolphin equals the port, the console differs" result from an older
bridge is not evidence). On Final Destination and Fountain of Dreams the stage's
creation draws precede the boundary seed, so the bridge needs a boundary
made for that replay: `make_boundary.py --game-start-seed <the header's
game_start_seed> --name <new name> --no-register`.

## Task

Make the named scenarios gate exact by porting what retail does, from the asm
and decomp, into the owning crate (architecture rules in `AGENTS.md`). Keep
going while the next divergence is the same feature; if a scenario then stops
at an unrelated unported boundary, stop and report it. Leave unported
branches as `unimplemented!` with the C line.

Character bring-up (see the Pikachu, Mario or Samus commits): crate,
descriptor, attributes, rows; register it in `melee-lib` (`Character`,
`scene_characters!`), `melee-replay` `CHARACTERS`, `CHARACTER_EFFECT_FILES`
(bank order) and `make_boundary.py`; a start boundary vs Fox on FD; explore
from your worktree build; bridge faults with `replay_to_scenario.py`; port
common-state gaps, then the specials; directed witnesses for main branches.
Kinds that share retail code share a family crate; the original's scenarios
must stay exact.

## Verify before committing

1. Your scenarios: `melee-sim gate` exact over their full `frames`, and
   `particle-sites` 0 differing ticks on what you register.
2. No regressions: the full sweep against the baseline prints no
   REGRESSION line and exits 0 (base PASS stays PASS, particle EXACT stays
   EXACT).
3. `cargo clippy -p <crates> --all-targets -- -D warnings`; the crates' tests.
4. Register exact scenarios (`CORPUS_V3_MATCHES` or `MATRIX_WITNESSES` in
   `crates/melee-sim/tests/m5_gate.rs`, one-line reason) and copy their TOMLs
   into your worktree.

## Commit and report

Commit on your branch (explicit paths, never game data, retail addresses in
the message, the trailer lines the coordinator gives). Before the final
report, `git rebase main` and reconcile: shared name tables and lists keep
everyone's entries, `boundaries.toml` must parse with every entry under its
own `[[boundary]]`, and a shared mechanism another agent already merged is
used rather than duplicated. Rebuild and re-gate.

Report briefly: branch and commit hashes; files changed; gate results; sweep
before/after; what is still unported (function or C line and the scenario that
reaches it); what you could not do. Never weaken a test or replace retail
expectations with port output. Do not edit `TRACKER.md`.
