"""Tick trace reduced to a particle-family ledger: who creates each effect
family, per tick.

Runs inside the scripting Dolphin like gobj_ledger.py (same env vars). It
watches the particle system's family counter (lbl_804D6368, the id
hsd_8039F05C and hsd_8039EFAC give a new generator's family): each store
is one family. A record is

    {"frame": N, "families": [{"pc", "lr", "id", "cur", "stack": [...]}]}

`stack` is the chain of saved link registers from the store outwards (the
PowerPC back chain: each frame's first word is its caller's frame, whose
+4 holds the return address), so the creating effect call and the proc it
ran in are both named. `MELEE_FAMILY_FROM` limits the ledger to records
from that tick on (earlier records carry an empty list).
"""
from __future__ import annotations

import os
import sys
from pathlib import Path

HERE = Path(globals().get("__file__") or sys._getframe().f_code.co_filename).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent))  # harness/ -> symbols
from tick_trace import TickTracer  # noqa: E402
from trace_common import event, run  # noqa: E402
import symbols  # noqa: E402
import walk  # noqa: E402

try:
    from dolphin import registers  # type: ignore
except ImportError:  # importable outside Dolphin for tests
    registers = None

FAMILY_COUNTER = symbols.addr("lbl_804D6368")
CURRENT_GOBJ = symbols.addr("HSD_GObj_804D781C")
STACK_DEPTH = 24


class FamilyLedgerTracer(TickTracer):
    def __init__(self, *args, regs=None, **kwargs):
        super().__init__(*args, **kwargs)
        self.regs = registers if regs is None else regs
        self.families: list[dict] = []
        self.first = int(os.environ.get("MELEE_FAMILY_FROM", "0"))
        self.records = 0

    def install(self) -> None:
        super().install()
        self.mem.add_memcheck(FAMILY_COUNTER)

    def stack(self) -> list[int]:
        frame, out = self.regs.read_gpr(1), []
        for _ in range(STACK_DEPTH):
            if not walk.MEM1_LO <= frame < walk.MEM1_HI or frame & 3:
                break
            caller = self.mem.read_u32(frame)
            if not walk.MEM1_LO <= caller < walk.MEM1_HI or caller & 3:
                break
            out.append(self.mem.read_u32(caller + 4))
            frame = caller
        return out

    def on_memory(self, is_write: bool, addr: int, value: int) -> None:
        if addr == FAMILY_COUNTER:
            if is_write and self.installed and not self.done and self.records + 1 >= self.first:
                self.families.append({"pc": self.regs.read_pc(), "lr": self.regs.read_lr(), "id": value,
                                      "cur": self.mem.read_u32(CURRENT_GOBJ), "stack": self.stack()})
            return
        super().on_memory(is_write, addr, value)

    def record(self, phase: str, mem=None) -> dict:
        rec = super().record(phase, mem)
        self.records += 1
        families, self.families = self.families, []
        return {"frame": rec["frame"], "phase": phase, "fighters": [len(rec["fighters"])], "families": families}

    def unregister(self) -> None:
        if self.installed and not self.in_callback:
            self.mem.remove_memcheck(FAMILY_COUNTER)
        super().unregister()

    def summary(self) -> dict:
        return {**super().summary(), "ledger": "families per record: family counter stores with their call chains"}


if event is not None and __name__ == "__main__":
    run(FamilyLedgerTracer)
