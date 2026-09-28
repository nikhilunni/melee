"""Where the harness finds machine-local data: the disc and savestates
(harness/roms), recordings (harness/traces) and the decomp submodule, and
where the bridge tools write the scenarios those recordings belong to.

By default that is this checkout. `MELEE_DATA_ROOT` names another checkout
whose data is read and written in place: a git worktree runs its own
harness scripts (a changed tracer, say) against the main checkout's roms,
traces and decomp, without symlinking or copying any of them.
"""
from __future__ import annotations

import os
from pathlib import Path

CODE_ROOT = Path(__file__).resolve().parents[1]
ROOT = Path(os.environ.get("MELEE_DATA_ROOT") or CODE_ROOT).resolve()
ROMS = ROOT / "harness" / "roms"
TRACES = ROOT / "harness" / "traces"
SCENARIOS = ROOT / "harness" / "scenarios"
DECOMP = ROOT / "third_party" / "melee-decomp"
