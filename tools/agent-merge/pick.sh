#!/bin/bash
# pick.sh <commit>: cherry-pick an agent commit after clearing identical untracked copies.
set -e
c=$1
if git show --stat --format= $c | grep -E "roms/|traces/|melee-decomp|\.sav"; then echo "REFUSE: data paths"; exit 1; fi
for f in $(git show --name-only --diff-filter=A --format= $c); do
  if [ -f "$f" ] && ! git ls-files --error-unmatch "$f" >/dev/null 2>&1; then
    mkdir -p target/agent-merge/displaced; if git show $c:"$f" | cmp -s - "$f"; then mv "$f" target/agent-merge/displaced/; else echo "untracked $f differs; moving to target/agent-merge/"; mv "$f" target/agent-merge/; fi
  fi
done
if ! git cherry-pick $c; then
  for f in $(git diff --name-only --diff-filter=U); do
    case "$f" in
      crates/melee-sim/tests/m5_gate.rs) python3 tools/agent-merge/resolve_lists.py "$f" && git add "$f";;
      harness/boundaries.toml) python3 tools/agent-merge/resolve_boundaries.py "$f" && git add "$f" || { echo "MANUAL CONFLICT: $f"; exit 2; };;
      crates/melee-lib/src/config.rs|crates/melee-lib/src/scene_fighter.rs|crates/melee-replay/src/config.rs) python3 tools/agent-merge/resolve_union.py "$f" && git add "$f" || { echo "MANUAL CONFLICT: $f"; exit 2; };;
      *) echo "MANUAL CONFLICT: $f"; exit 2;;
    esac
  done
  GIT_EDITOR=true git cherry-pick --continue
fi
python3 -c "import tomllib;tomllib.load(open('harness/boundaries.toml','rb'))" || { echo "BOUNDARIES.TOML BROKEN after $c"; exit 3; }
