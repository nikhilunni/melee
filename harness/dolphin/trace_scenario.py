"""Legacy VI begin-field oracle; use tick_trace.py for complete game ticks.

MELEE_SCENARIO and MELEE_RAW_OUT select the scenario and raw JSONL output.
Loading on the first frameadvance and the raw record format are shared with
that sampler in trace_common.py. Legacy frame 0 remains the loaded state.
"""
from __future__ import annotations

import sys
from pathlib import Path

# Dolphin's --script has no __file__, but its code object has the script path.
HERE = Path(globals().get("__file__") or sys._getframe().f_code.co_filename).resolve().parent
sys.path.insert(0, str(HERE))
from trace_common import (  # noqa: E402,F401
    ENTITIES_ADDR, FIGHTER_SIZE, GOBJ_USER_DATA_OFF, REPO, SEED_ADDR,
    Tracer, event, fighter_bases, memory, read_sidecar, resolve_savestate,
    run, script_dir,
)


def main() -> None:
    run()


if event is not None:
    main()
