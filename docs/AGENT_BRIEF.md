# Brief for a worktree agent

The coordinator points each worktree agent at this file, then states the
agent's task (a fault tag, its scenarios, what to port). Follow `CLAUDE.md`
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

## Task

Make the named scenarios gate exact by porting what retail does, from the asm
and decomp, into the owning crate (architecture rules in `CLAUDE.md`). Keep
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
2. No regressions: sweep every scenario with a retail trace before your
   change (pristine build) and after; nothing that passed may fail.
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
