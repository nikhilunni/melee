"""Tick trace reduced to a GObj allocation ledger: every HSD_ObjAlloc and
HSD_ObjFree on `gobj_alloc_data`, in order, per tick.

Runs inside the scripting Dolphin like rng_ledger.py (same env vars). It
watches the allocator's free-list head (`gobj_alloc_data.freehead`,
objalloc.h): HSD_ObjAlloc stores the popped object's next link there and
HSD_ObjFree the freed object, so each store is one event. A record is

    {"tick": N, "items": [{"gobj", "kind"}], "gobj": [event, ...]}

and an event {"pc", "head", "old", "cur", "cur_word", "word"}: the store's
pc, the new and previous head, the GObj whose proc is running
(HSD_GObj_804D781C) with its first word (classifier, p_link, gx_link), and
for an allocation the new object's first word, read at the next event or
at the tick's end (CreateGObj fills it right after the allocation).
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
import walk  # noqa: E402

try:
    from dolphin import registers  # type: ignore
except ImportError:  # importable outside Dolphin for tests
    registers = None

# HSD_ObjAllocData.freehead (+4, after the flag word).
FREEHEAD_ADDR = symbols.addr("gobj_alloc_data") + 4
CURRENT_GOBJ = symbols.addr("HSD_GObj_804D781C")


class GObjLedgerTracer(TickTracer):
    def __init__(self, *args, regs=None, **kwargs):
        super().__init__(*args, **kwargs)
        self.regs = registers if regs is None else regs
        self.gobj_events: list[dict] = []
        self.head = 0
        self.unclassified: dict | None = None

    def install(self) -> None:
        super().install()
        self.head = self.mem.read_u32(FREEHEAD_ADDR)
        self.mem.add_memcheck(FREEHEAD_ADDR)

    def word(self, addr: int) -> int | None:
        if not walk.MEM1_LO <= addr < walk.MEM1_HI or addr & 3:
            return None
        return self.mem.read_u32(addr)

    def classify_pending(self) -> None:
        if self.unclassified is not None:
            self.unclassified["word"] = self.word(self.unclassified["old"])
            self.unclassified = None

    def on_memory(self, is_write: bool, addr: int, value: int) -> None:
        if addr == FREEHEAD_ADDR:
            if is_write and self.installed and not self.done:
                self.classify_pending()
                cur = self.mem.read_u32(CURRENT_GOBJ)
                ev = {"pc": self.regs.read_pc(), "head": value, "old": self.head,
                      "cur": cur, "cur_word": self.word(cur)}
                self.head = value
                self.gobj_events.append(ev)
                self.unclassified = ev
            return
        super().on_memory(is_write, addr, value)

    def record(self, phase: str, mem=None) -> dict:
        rec = super().record(phase, mem)
        self.classify_pending()
        items = [{"gobj": item["gobj"], "kind": item["kind"]} for item in rec.get("items", [])]
        events, self.gobj_events = self.gobj_events, []
        return {"frame": rec["frame"], "phase": phase, "fighters": [len(rec["fighters"])],
                "items": items, "gobj": events}

    def unregister(self) -> None:
        if self.installed and not self.in_callback:
            self.mem.remove_memcheck(FREEHEAD_ADDR)
        super().unregister()

    def summary(self) -> dict:
        return {**super().summary(), "ledger": "gobj events per record: freehead stores"}


if event is not None and __name__ == "__main__":
    run(GObjLedgerTracer)
