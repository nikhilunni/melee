# Merging worktree agents' commits

The coordinator merges each agent's branch onto `main` one commit at a time:

    tools/agent-merge/pick.sh <commit>

`pick.sh` refuses commits that touch game data or the decomp, moves aside any
untracked copy of a file the commit adds (agents record scenarios in the main
checkout; identical copies go to `target/agent-merge/displaced/`), cherry-picks,
and resolves the conflicts every parallel wave produces:

| File | Resolver |
|---|---|
| `crates/melee-sim/tests/m5_gate.rs` | `resolve_lists.py`: keep both sides' entries, recount each `[(&str, usize); N]` |
| `harness/boundaries.toml` | `resolve_boundaries.py`: keep both entries, each with its own `[[boundary]]`; must parse |
| name tables (`melee-lib` `config.rs`, `scene_fighter.rs`, `melee-replay` `config.rs`) | `resolve_union.py`: keep both sides (fix array lengths by hand if the build complains) |

Any other conflict stops with `MANUAL CONFLICT`. Then either resolve it with a
three-way merge of that file (`git merge-file -p --diff3 ours base theirs`,
base = `<commit>^`), never `git checkout --ours` on the whole file (that drops
the commit's other hunks in it), or abort and ask the agent to
`git rebase main` and reconcile: an agent knows its own change best, and
shared pieces (hooks, effect tables, item engine) must exist once.

After each merge: build `melee-sim`, gate the agent's registered scenarios
plus a cross-section (both human Fox-Marth matches, one scenario per touched
shared area), and run `m5_gate` after every few merges. A checkpoint (clippy,
`cargo test --workspace --release --no-fail-fast`, harness pytest,
`gen_schema.py --check`) closes a wave.
