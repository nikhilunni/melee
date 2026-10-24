"""Tick trace plus retail execution coverage: every branch the game executed.

Runs inside the scripting Dolphin like tick_trace.py (same env vars, the same
raw records) and additionally records Dolphin's Branch Watch over exactly the
recorded ticks: started right after the savestate load (the tracer's install),
stopped inside the callback that takes the last tick's record, and written to
MELEE_BRANCH_WATCH_OUT before the `.done` marker.

Branch Watch counts every executed branch by (origin, destination,
instruction), taken and not taken, so a function executed iff a branch inside
it ran (every function leaves through one), and a function's entries are the
taken branches that land on its first instruction. retail_coverage.py maps the
dump onto symbols.txt.

Needs the fork's `emulation.branch_watch_*` functions
(docs/patches/0004-scripting-branch-watch.patch) and debug mode
(`-C Dolphin.Interface.DebugModeEnabled=True`): the JITs emit the recording
code only then. Debug mode does not change what the game computes; each
coverage run checks that against the recording made without it.
"""
from __future__ import annotations

import os
import sys
from pathlib import Path

HERE = Path(globals().get("__file__") or sys._getframe().f_code.co_filename).resolve().parent
sys.path.insert(0, str(HERE))
from tick_trace import TickTracer
from trace_common import event, run

try:
    from dolphin import emulation  # type: ignore
except ImportError:  # importable outside Dolphin for tests
    emulation = None

OUT_ENV = "MELEE_BRANCH_WATCH_OUT"
#: "0": write no tick records (a run with no recording to compare against;
#: a four-fighter match with items writes gigabytes of them).
TRACE_ENV = "MELEE_COVERAGE_TRACE"


class Discard:
    """A raw-trace file that keeps nothing."""

    def __init__(self, name: str) -> None:
        self.name = name

    def write(self, text: str) -> int:
        return len(text)

    def flush(self) -> None:
        pass

    def close(self) -> None:
        pass


class CoverageTracer(TickTracer):
    def __init__(self, *args, emu=None, branch_out: str | None = None, **kwargs):
        super().__init__(*args, **kwargs)
        if os.environ.get(TRACE_ENV) == "0":
            self.out.close()
            self.out = Discard(self.out.name)
        self.emu = emulation if emu is None else emu
        self.branch_out = branch_out or os.environ[OUT_ENV]
        self.watching = False
        self.ticks_watched = 0

    def install(self) -> None:
        super().install()
        self.emu.branch_watch_clear()
        self.emu.branch_watch_start()
        self.watching = True

    def stop_watch(self) -> None:
        if self.watching:
            self.emu.branch_watch_stop()
            self.watching = False
            self.ticks_watched = self.frame

    def on_memory(self, is_write: bool, addr: int, value: int) -> None:
        super().on_memory(is_write, addr, value)
        # The last record was just taken (or the match ended): nothing after
        # this tick belongs to the coverage.
        if self.pending_finish or self.done:
            self.stop_watch()

    def finish(self) -> None:
        self.stop_watch()
        self.emu.branch_watch_save(self.branch_out)
        super().finish()

    def summary(self) -> dict:
        return {**super().summary(), "branch_watch": self.branch_out,
                "branch_watch_ticks": self.ticks_watched}


if event is not None and __name__ == "__main__":
    run(CoverageTracer)
