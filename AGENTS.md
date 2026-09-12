# AGENTS.md (read by Codex)

This repository's rules live in `CLAUDE.md`. Read it in full before doing
anything, then read `TRACKER.md`. Every section of `CLAUDE.md` applies to you:
exactness rules, code style ("write for humans"), build-speed rules, porting
workflow, and the hard boundaries (never modify `third_party/melee-decomp`,
never commit game data, never commit at all unless your task says so).

When you are handed a task, the acceptance criteria are mechanical and are
stated in the task: named tests that must pass under `cargo gate`, plus
`cargo clippy --workspace --all-targets -- -D warnings`. Do not weaken, skip,
or loosen a test merely to make it pass. If a bit-exact test fails, immediately
trace the delta against retail assembly and the captured oracle, fix the faulty
implementation or test, and continue without requesting confirmation. A test
correction must preserve valid coverage and document the evidence, including
field ownership and initialization boundaries. Never replace captured expected
values with simulator output or change expectations without independent retail
evidence. Rerun the affected checks and required gates before committing.

Finish with a short report: what you changed (file list), what tests you ran
and their results, and anything you could not do.
