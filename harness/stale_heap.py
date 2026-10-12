"""Which bytes of a Fighter does retail leave as stale heap at a match's start?

    cd harness && uv run python stale_heap.py runs            # stale byte runs of the whole struct
    cd harness && uv run python stale_heap.py characters      # per kind: stale character variables and their readers
    cd harness && uv run python stale_heap.py compare <boundary A> <boundary B> [fighter]

HSD_ObjAlloc (0x8037ABC8) returns the Fighter block uncleared and
Fighter_Create (0x80068E98) writes only part of it (docs/PORT_NOTES/
STALE_HEAP.md). Every `start_*` recording's first record is each Fighter as
created, so the recordings themselves show which bytes nothing wrote:

- The first fighter's block address depends on the stage alone, and the
  boot-to-match path make_boundary.py drives is the same every time. A byte
  that creation never writes therefore holds one value for every character
  at one address, and (usually) another at another stage's address. `runs`
  lists the bytes where at least 80% of the characters at each address agree
  and the addresses disagree.
- `characters` does the same for the character variables (+222C..+233F) of
  one kind at a time (a kind's own bytes differ between addresses, or equal
  what the kinds that leave the byte show there), then lists the functions of
  that character's units in the split retail asm (docs/ASM.md) that load from
  or store to each stale word. A word with a reader and no store on the way
  from creation is read before it is written; that last step is read from
  the asm by hand.
- `compare` lists the words that differ between two boundaries' first
  records (pointers into the block itself compared relative to its address).

A scan by displacement cannot see an access through a computed address
(`addi rN, rFp, off`), so arrays and embedded structs (hit and hurt capsules)
are judged by their guard fields, not by this script.
"""
from __future__ import annotations

import collections
import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import data_root  # noqa: E402
import trace_io  # noqa: E402

FIGHTER_SIZE = 0x23EC
CHARACTER_VARS = range(0x222C, 0x2340)
#: FighterKind order (melee/ft/forward.h) and each kind's asm unit directories.
KINDS = ["Mario", "Fox", "Captain", "Donkey", "Kirby", "Koopa", "Link", "Seak", "Ness", "Peach", "Popo", "Nana",
         "Pikachu", "Samus", "Yoshi", "Purin", "Mewtwo", "Luigi", "Mars", "Zelda", "CLink", "DrMario", "Falco",
         "Pichu", "GameWatch", "Ganon", "Emblem"]
SHARED_UNITS = {"DrMario": ["Mario"], "Luigi": ["Mario"], "Falco": ["Fox"], "Ganon": ["Captain"],
                "CLink": ["Link"], "Nana": ["Popo"], "Pichu": ["Pikachu"], "Emblem": ["Mars"]}
SIZES = {"lwz": 4, "lfs": 4, "stw": 4, "stfs": 4, "lhz": 2, "lha": 2, "sth": 2, "lbz": 1, "stb": 1, "lfd": 8,
         "stfd": 8}
LOADS = {"lwz", "lbz", "lhz", "lha", "lfs", "lfd"}
STORES = {"stw", "stb", "sth", "stfs", "stfd"}


def first_fighters(prefix: str = "start_") -> dict[str, list[tuple[int, bytes]]]:
    """Each matching recording's first-record fighters as (address, bytes)."""
    out = {}
    for path in sorted(data_root.TRACES.glob(f"{prefix}*.tick.raw.jsonl*")):
        name = path.name.split(".tick.raw.jsonl")[0]
        if name in out or path.name.endswith(".done"):
            continue
        with trace_io.open_text(data_root.TRACES / f"{name}.tick.raw.jsonl") as f:
            record = json.loads(f.readline())
        out[name] = [(int(row["base"], 16), bytes.fromhex(row["bytes"])) for row in record["fighters"]]
    return out


def kind_of(raw: bytes) -> int:
    return int.from_bytes(raw[4:8], "big")


def by_address(boundaries) -> dict[int, list[bytes]]:
    blocks = collections.defaultdict(list)
    for fighters in boundaries.values():
        address, raw = fighters[0]
        blocks[address].append(raw)
    return blocks


def stale_runs(boundaries) -> None:
    blocks = by_address(boundaries)
    addresses = sorted(blocks)
    stale = []
    for offset in range(FIGHTER_SIZE):
        shared = []
        for address in addresses:
            value, count = collections.Counter(raw[offset] for raw in blocks[address]).most_common(1)[0]
            if count < 0.8 * len(blocks[address]):
                break
            shared.append(value)
        else:
            if len(set(shared)) > 1:
                stale.append((offset, shared))
    print("first-fighter addresses:", ", ".join(f"{a:08X} x{len(blocks[a])}" for a in addresses))
    print(f"{len(stale)} of {FIGHTER_SIZE} bytes hold what the block held before")
    run: list[tuple[int, list[int]]] = []
    for offset, shared in stale + [(-2, [])]:
        if run and offset != run[-1][0] + 1:
            values = " ".join("".join(f"{row[1][i]:02x}" for row in run)[:16] for i in range(len(addresses)))
            print(f"+{run[0][0]:04X}..+{run[-1][0]:04X} ({len(run):4d}) {values}")
            run = []
        run.append((offset, shared))


def accesses(units: list[str]) -> dict[int, dict[str, set[str]]]:
    """Displacement loads and stores of the units' functions, by Fighter byte."""
    asm = data_root.DECOMP / "build/GALE01/asm/melee/ft/kinds"
    line = re.compile(r"^/\* [0-9A-F]{8} [0-9A-F]{8}  (?:[0-9A-F]{2} ){4}\*/\t(\S+) .*?(0x[0-9a-fA-F]+)\(r(\d+)\)")
    out: dict[int, dict[str, set[str]]] = collections.defaultdict(lambda: {"r": set(), "w": set()})
    for unit in units:
        for path in sorted((asm / f"ft{unit}").glob("*.s")):
            function = None
            for text in path.read_text(errors="replace").splitlines():
                if text.startswith(".fn "):
                    function = text.split()[1].rstrip(",")
                m = line.match(text)
                if not m or m.group(3) in ("1", "2", "13") or m.group(1) not in LOADS | STORES:
                    continue
                kind = "r" if m.group(1) in LOADS else "w"
                start = int(m.group(2), 16)
                for offset in range(start, start + SIZES[m.group(1)]):
                    out[offset][kind].add(function)
    return out


def characters(boundaries) -> None:
    rows = collections.defaultdict(lambda: collections.defaultdict(list))
    everyone = collections.defaultdict(list)
    for fighters in boundaries.values():
        address, raw = fighters[0]
        rows[kind_of(raw)][address].append(raw)
        everyone[address].append(raw)
    for kind in sorted(rows):
        name = KINDS[kind]
        stale = set()
        for offset in CHARACTER_VARS:
            own = {address: {raw[offset] for raw in blocks} for address, blocks in rows[kind].items()}
            left_as_found = nonzero = 0
            for address, values in own.items():
                others = collections.Counter(raw[offset] for raw in everyone[address] if kind_of(raw) != kind)
                if values <= {value for value, _ in others.most_common(2)}:
                    left_as_found += 1
                    nonzero += any(values)
            if len(set().union(*own.values())) > 1 or (left_as_found == len(own) and nonzero):
                stale.add(offset)
        words = sorted({offset & ~3 for offset in stale})
        samples = sum(len(blocks) for blocks in rows[kind].values())
        print(f"{name}: {samples} first fighters at {len(rows[kind])} addresses; "
              f"{len(words)} stale words in +222C..+233F")
        touched = accesses([name] + SHARED_UNITS.get(name, []))
        for word in words:
            readers = set().union(*(touched[word + i]["r"] for i in range(4)))
            writers = set().union(*(touched[word + i]["w"] for i in range(4)))
            if not readers:
                continue
            held = "; ".join(f"{address:08X}: {blocks[0][word:word + 4].hex()}"
                             for address, blocks in sorted(rows[kind].items()))
            print(f"   +{word:04X} [{held}]")
            print(f"      loads:  {', '.join(sorted(readers))}")
            print(f"      stores: {', '.join(sorted(writers)) or '(none)'}")


def compare(boundaries_a, a: str, boundaries_b, b: str, fighter: int) -> None:
    (base_a, raw_a), (base_b, raw_b) = boundaries_a[a][fighter], boundaries_b[b][fighter]
    print(f"{a} at {base_a:08X} (kind {kind_of(raw_a)}), {b} at {base_b:08X} (kind {kind_of(raw_b)})")
    differing = 0
    for offset in range(0, FIGHTER_SIZE - 3, 4):
        x = int.from_bytes(raw_a[offset:offset + 4], "big")
        y = int.from_bytes(raw_b[offset:offset + 4], "big")
        own_pointer = 0x80000000 <= x < 0x81800000 and x - base_a == y - base_b
        if x != y and not own_pointer:
            differing += 1
            print(f"+{offset:04X} {x:08x} {y:08x}")
    print(f"{differing} of {FIGHTER_SIZE // 4} words differ")


def main(argv: list[str]) -> None:
    if argv[:1] == ["runs"]:
        stale_runs(first_fighters())
    elif argv[:1] == ["characters"]:
        characters(first_fighters())
    elif argv[:1] == ["compare"] and len(argv) in (3, 4):
        a, b = argv[1], argv[2]
        compare(first_fighters(a), a, first_fighters(b), b, int(argv[3]) if len(argv) == 4 else 0)
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main(sys.argv[1:])
