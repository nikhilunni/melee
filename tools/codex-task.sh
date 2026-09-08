#!/usr/bin/env bash
# Delegate a bounded implementation task to Codex (OpenAI) with the repo's
# rules (AGENTS.md -> CLAUDE.md) and mechanical acceptance criteria.
#
# Usage:
#   tools/codex-task.sh <name> <prompt-file> [extra codex exec args...]
#
# Runs `codex exec` non-interactively in workspace-write sandbox from the repo
# root. Codex's final message lands in .codex-runs/<name>.md and the full event
# log in .codex-runs/<name>.jsonl (both gitignored). Review the diff with
# `git status` / `git diff`, run `cargo gate` yourself, then commit.
#
# Model: defaults to gpt-6-astra (also the default in ~/.codex/config.toml);
# override with CODEX_MODEL=... . Effort: CODEX_EFFORT (default high).
# Sandbox: CODEX_SANDBOX (default workspace-write; read-only for reviews).
set -euo pipefail
name="$1"; prompt_file="$2"; shift 2
root="$(cd "$(dirname "$0")/.." && pwd)"
mkdir -p "$root/.codex-runs"
exec codex exec \
  -C "$root" \
  -m "${CODEX_MODEL:-gpt-6-astra}" \
  -c "model_reasoning_effort=\"${CODEX_EFFORT:-high}\"" \
  -s "${CODEX_SANDBOX:-workspace-write}" \
  --json \
  -o "$root/.codex-runs/$name.md" \
  "$@" \
  - < "$prompt_file" > "$root/.codex-runs/$name.jsonl"
