# AGENTS.md

Any agent working here follows `CLAUDE.md` in full: read it, then `TRACKER.md`.
It covers the verification workflow, exactness and style rules, and the hard
boundaries (never touch `harness/roms/`, `harness/traces/`, `~/melee-data/` or
`third_party/melee-decomp`; add files to git by explicit path only).

A delegated task states its scope and mechanical acceptance criteria (named
tests or gates). Never weaken, skip or loosen a test to make it pass, and never
replace retail expectations with simulator output. When a bit-exact check
fails, trace the difference against the retail asm and recorded traces
(`melee-sim triage` first) and fix the implementation. Commit only when your
task says to, on your own branch.

Finish with a short report: what changed (files), what you ran and the
results, and what you could not do.
