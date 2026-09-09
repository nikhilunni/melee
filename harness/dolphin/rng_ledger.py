"""Tick trace plus an RNG ledger: who draws from `seed`, in order, per tick.

Runs inside the scripting Dolphin like tick_trace.py (same env vars and
outputs) and additionally watches the RNG `seed` word. HSD_Rand, HSD_Randi
and HSD_Randf all inline the LCG and store `seed` themselves without saving
LR (see `asm.py HSD_Rand`), so at the store the link register is the direct
caller. Each record gains `rng_draws: [{"pc", "lr"}]` in draw order.

Needs the fork patch that adds `registers.read_pc()` / `read_lr()`
(docs/patches/0002-scripting-read-pc-lr.patch).
"""
from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(globals().get("__file__") or sys._getframe().f_code.co_filename).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent))  # harness/ -> symbols
from tick_trace import TickTracer  # noqa: E402
from trace_common import event, run  # noqa: E402
import symbols  # noqa: E402

try:
    from dolphin import registers  # type: ignore
except ImportError:  # importable outside Dolphin for tests
    registers = None

SEED_ADDR = symbols.addr("seed")


class RngLedgerTracer(TickTracer):
    def __init__(self, *args, regs=None, **kwargs):
        super().__init__(*args, **kwargs)
        self.regs = registers if regs is None else regs
        self.draws: list[dict] = []

    def install(self) -> None:
        super().install()
        self.mem.add_memcheck(SEED_ADDR)

    def on_memory(self, is_write: bool, addr: int, value: int) -> None:
        if addr == SEED_ADDR:
            if is_write and self.installed and not self.done:
                self.draws.append({"pc": self.regs.read_pc(), "lr": self.regs.read_lr(),
                                   "seed": value})
            return
        super().on_memory(is_write, addr, value)

    def record(self, phase: str, mem=None) -> dict:
        rec = super().record(phase, mem)
        rec["rng_draws"] = self.draws
        self.draws = []
        return rec

    def unregister(self) -> None:
        if self.installed and not self.in_callback:
            self.mem.remove_memcheck(SEED_ADDR)
        super().unregister()

    def summary(self) -> dict:
        return {**super().summary(), "ledger": "rng_draws per record: pc/lr at the seed store"}


if event is not None and __name__ == "__main__":
    run(RngLedgerTracer)
