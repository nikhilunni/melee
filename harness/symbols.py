"""Resolve retail symbol names to addresses using the decomp's symbols.txt.

Lines look like:
    seed = .sdata:0x804D5F90; // type:object size:0x4 scope:global
"""
from __future__ import annotations

import re
from functools import lru_cache
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SYMBOLS = ROOT / "third_party" / "melee-decomp" / "config" / "GALE01" / "symbols.txt"
_LINE = re.compile(r"^(\w+)\s*=\s*\.(\w+):0x([0-9A-Fa-f]+);\s*//\s*(.*)$")


@lru_cache(maxsize=1)
def load() -> dict[str, dict]:
    out: dict[str, dict] = {}
    for raw in SYMBOLS.read_text().splitlines():
        m = _LINE.match(raw.strip())
        if not m:
            continue
        name, section, addr, attrs = m.groups()
        meta = dict(kv.split(":", 1) for kv in attrs.split() if ":" in kv)
        out[name] = {"addr": int(addr, 16), "section": section, **meta}
    return out


def addr(name: str) -> int:
    try:
        return load()[name]["addr"]
    except KeyError as e:
        raise KeyError(f"symbol {name!r} not in {SYMBOLS}") from e


if __name__ == "__main__":
    import sys
    for n in sys.argv[1:] or ["seed", "HSD_GObj_Entities", "ftCo_800B3900"]:
        s = load()[n]
        print(f"{n:32s} 0x{s['addr']:08X} {s['section']:8s} size={s.get('size','?')}")
