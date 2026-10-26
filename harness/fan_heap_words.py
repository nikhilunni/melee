"""Write a scenario's `heap_ammo_words` from its retail recording.

    cd harness && uv run python fan_heap_words.py scenarios/<name>.toml ...

A new Fan (It_Kind_Harisen) reads xD4C, which nothing writes for it: the
word is whatever the heap held where HSD_ObjAlloc placed the Item, so no game
state decides it (itHarisen_UnkMotion8_Coll lands a dropped Fan on nonzero
and removes it on zero). The raw tick trace carries each Item struct; this
lists the xD4C of each Fan in the order the Fans appear and writes it into
the scenario (`heap_ammo_words`, melee-sim's Scenario), replacing an earlier
list. A scenario whose recording has no Fan is left alone.
"""
from __future__ import annotations

import json
import re
import sys
import tomllib
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import data_root  # noqa: E402
import trace_io  # noqa: E402

HARISEN = 24
KIND = 0x10
XD4C = 0xD4C


def fan_words(raw: Path) -> list[int]:
    """xD4C of each Fan, in the order the Fans first appear."""
    words: list[int] = []
    live: dict[str, int] = {}
    with trace_io.open_text(raw) as f:
        for line in f:
            record = json.loads(line)
            if record.get("phase") != "frame_end":
                continue
            now = {}
            for item in record.get("items") or []:
                data = bytes.fromhex(item["bytes"])
                kind = int.from_bytes(data[KIND:KIND + 4], "big")
                now[item["base"]] = kind
                if kind == HARISEN and live.get(item["base"]) != kind:
                    words.append(int.from_bytes(data[XD4C:XD4C + 4], "big"))
            live = now
    return words


def write(scenario: Path, words: list[int]) -> None:
    text = scenario.read_text()
    text = re.sub(r"^heap_ammo_words = \[[^\]]*\]\n", "", text, flags=re.M)
    listed = ", ".join(f"0x{w:08X}" for w in words)
    entry = ("# The xD4C each new Fan read in the recording (stale heap; fan_heap_words.py).\n"
             f"heap_ammo_words = [{listed}]\n")
    # Top-level keys go before the first table.
    table = re.search(r"^\[\[?\w", text, flags=re.M)
    at = table.start() if table else len(text)
    text = text[:at] + entry + ("\n" if table else "") + text[at:]
    text = text.replace("\n\n\n", "\n\n")
    scenario.write_text(text)


def main(argv: list[str]) -> None:
    for arg in argv:
        scenario = Path(arg)
        if not scenario.is_absolute() and not scenario.exists():
            scenario = data_root.SCENARIOS / scenario.name
        spec = tomllib.loads(scenario.read_text())
        trace = spec.get("expected") or spec["name"]
        raw = data_root.TRACES / f"{trace}.tick.raw.jsonl"
        if not trace_io.exists(raw):
            print(f"{scenario.name}: no raw trace {raw.name}")
            continue
        words = fan_words(raw)
        if words:
            write(scenario, words)
        print(f"{scenario.name}: {len(words)} Fans")


if __name__ == "__main__":
    main(sys.argv[1:])
