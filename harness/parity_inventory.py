"""Function-level parity inventory: every retail code function, classified by
how far the Rust port has it (docs/PARITY.md says what each status means).

    cd harness && uv run python parity_inventory.py [--out ../target/parity] [--markdown]

Inputs: the decomp's symbol table and splits (config/GALE01), its C sources
(to map a fail-closed site's `file.c:line` to a function), and every `.rs`
file under `crates/`. Outputs, keyed by retail address:
`inventory.tsv` (one row per retail function) and `fail_closed.tsv` (one row
per `unimplemented!`/`todo!`/fail-closed error in the port). `--markdown`
prints the summary tables docs/PARITY.md is built from.

The classification is a set of text heuristics over the port's citation
convention (a doc comment naming the decomp symbol and/or address). Each
heuristic is a named function below; docs/PARITY.md lists them and the
measured error rate.
"""
from __future__ import annotations

import argparse
import bisect
import csv
import re
import sys
from collections import Counter, defaultdict
from dataclasses import dataclass, field
from pathlib import Path

from data_root import CODE_ROOT, DECOMP

CODE_SECTIONS = {"init", "text"}

# ---------------------------------------------------------------- retail side

_SYMBOL = re.compile(r"^(\S+)\s*=\s*\.(\w+):0x([0-9A-Fa-f]+);\s*//\s*(.*)$")


@dataclass
class Symbol:
    name: str
    section: str
    addr: int
    size: int
    kind: str  # "function", "object", ...


def parse_symbols(text: str) -> list[Symbol]:
    out = []
    for raw in text.splitlines():
        m = _SYMBOL.match(raw.strip())
        if not m:
            continue
        name, section, addr, attrs = m.groups()
        meta = dict(kv.split(":", 1) for kv in attrs.split() if ":" in kv)
        out.append(Symbol(name, section, int(addr, 16),
                          int(meta.get("size", "0"), 16), meta.get("type", "")))
    return out


_SPLIT_FILE = re.compile(r"^(\S.*):\s*$")
_SPLIT_RANGE = re.compile(r"^\s+\.?(\w+)\s+start:0x([0-9A-Fa-f]+)\s+end:0x([0-9A-Fa-f]+)")


def parse_splits(text: str) -> list[tuple[int, int, str]]:
    """(start, end, source file) for every code section range in splits.txt."""
    out = []
    current = None
    for raw in text.splitlines():
        m = _SPLIT_FILE.match(raw)
        if m and m.group(1) != "Sections":
            current = m.group(1)
            continue
        m = _SPLIT_RANGE.match(raw)
        if m and current and m.group(1) in CODE_SECTIONS:
            out.append((int(m.group(2), 16), int(m.group(3), 16), current))
    out.sort()
    return out


def source_file(addr: int, splits: list[tuple[int, int, str]]) -> str:
    i = bisect.bisect_right(splits, (addr, 1 << 40, "")) - 1
    if i >= 0 and splits[i][0] <= addr < splits[i][1]:
        return splits[i][2]
    return "(unsplit)"


def subsystem(src: str) -> str:
    """The directory a function's source lives in, as the decomp groups it:
    `melee/ft/kinds/ftKirby/ftkirby.c` -> `ft/kinds/ftKirby`."""
    if src == "(unsplit)":
        return src
    parts = src.split("/")
    if parts[0] == "melee":
        parts = parts[1:]
    return "/".join(parts[:-1]) or parts[0]


def area(src: str) -> str:
    """A coarse grouping for the headline table."""
    sub = subsystem(src)
    if sub.startswith("ft/kinds/ftCommon"):
        return "ft common states"
    if sub.startswith("ft/kinds/"):
        return "ft characters"
    if sub == "ft":
        return "ft engine"
    if sub.startswith("it"):
        return "it items" if "kinds" in sub else "it engine"
    if sub.startswith("sysdolphin"):
        return "HSD (sysdolphin)"
    if sub.startswith(("dolphin", "MetroTRK", "Runtime")):
        return "SDK/runtime"
    if sub.startswith("MSL"):
        return "MSL"
    return sub.split("/")[0]


# Historical reporting groups, retained for comparable measurements. The
# objective includes every family below (user, 2026-10-24); these tags do
# not exclude work. Rendering ports in melee-platform count when cited.
_OUT_OF_SCOPE: list[tuple[str, str]] = [
    (r"^melee/mn/", "menus"),
    (r"^melee/gm/(gmmenu|gmtitle|gmhanyu|gmmenumode|gmtitlemode|gmdebugmode|gm_1A3F|gm_1A36|gm_1A45)", "menus"),
    (r"^melee/gm/(gmresult|gmregclear|gmregcommon|gmregenddisp|gmregtyfall|gm_17)", "results"),
    (r"^melee/gm/gmpause|^melee/gm/gmcamera", "pause"),
    (r"^melee/gm/(gmclassic|gmadventure|gmallstar|gmmultiman|gmhomerun|gmevent|gmtrainingmode|gmtou|gmtoy|gmgover|gmstaffroll|gmopening|gmhowto|gmmovieend|gmprogressive|gmapproach|gmomake|gmsupersudden|gmtiny|gmgiant|gmstamina|gmfixedcamera|gmsinglebutton|gminvisible|gmslomo|gmlightning|gmcameramode|gm_18|gm_19|gm_1A7A|gm_1A9B|gm_1ADD|gm_1AED|gm_1B03)", "single-player and modes"),
    (r"^melee/ft/kinds/(ftMasterHand|ftCrazyHand|ftZakoBoy|ftZakoGirl|ftGigaKoopa|ftSandbag)/", "single-player fighters"),
    (r"^melee/gr/(grt[a-z]+|grhomerun|grfigure\w*|grheal|\w+route)\.c", "single-player and modes"),
    (r"^melee/ty/", "trophies"),
    (r"^melee/pl/(plbonus|pltrick)", "results"),
    (r"^melee/vi/", "cutscenes"),
    (r"^melee/db/|^sysdolphin/baselib/(debug|debugconsole|leak|perf|devcom)", "debug"),
    (r"^melee/lb/(lbcard|lbsnap)|^dolphin/card/|^melee/mn/mndatadel", "memory card"),
    (r"^melee/lb/lbaudio|^melee/sfx/|^dolphin/(ax|axfx|dsp|ai|ar|ARQ)|^sysdolphin/baselib/(synth|axdriver)|^melee/lb/lbarq", "audio"),
    (r"^dolphin/(gx|vi|gd)|^sysdolphin/baselib/(tev|tobj|mobj|pobj|dobj|lobj|displayfunc|video|fog|texp|texpdag|psdisp|psdisptev|sislib|sobjlib|shadow|state|gobjgxlink|hsd_3915|hsd_3A64|hsd_3A76)|^melee/lb/(lbgx|lbshadow|lbrefract|lbbgflash|lbspdisplay|lbmthp)|^melee/if/(text|soundtest)", "rendering"),
    (r"^dolphin/|^MetroTRK|^Runtime|^melee/lb/(lbdvd|lbheap|lbfile|lbarchive|lbmemory)|^sysdolphin/baselib/(memory|initialize|objalloc|archive)", "OS, hardware, loading"),
    (r"^MSL/(?!math|trigf|float)", "OS, hardware, loading"),
]


def scope(src: str) -> str:
    for pattern, family in _OUT_OF_SCOPE:
        if re.search(pattern, src):
            return family
    return "in-game"


# ------------------------------------------------------------- C source side

_C_DEF_START = re.compile(r"^[A-Za-z_][\w\s\*]*?\b(\w+)\s*\(")


def c_functions(text: str) -> list[tuple[str, int, int]]:
    """(name, first line, last line) of each top-level function definition,
    1-based. A definition starts at column 0 with `type name(`; its body
    opens with a `{` (at column 0, or ending the signature) within ten
    lines and closes at the next `}` at column 0."""
    lines = text.splitlines()
    out = []
    i = 0
    while i < len(lines):
        m = _C_DEF_START.match(lines[i])
        if not m or lines[i].startswith(("typedef", "struct", "#", "extern")):
            i += 1
            continue
        body = None
        for j in range(i, min(i + 10, len(lines))):
            line = lines[j].rstrip()
            if line.startswith("{") or (line.endswith("{") and ")" in line):
                body = j
                break
            if line.endswith(";"):
                break
        if body is None:
            i += 1
            continue
        k = body + 1
        while k < len(lines) and not lines[k].startswith("}"):
            k += 1
        out.append((m.group(1), i + 1, k + 1))
        i = k + 1
    return out


_MOTION_TABLE = re.compile(r"^MotionState (ftData_MotionStateList|\w+_Init_MotionStateTable)\[[^\]]*\]\s*=\s*\{", re.MULTILINE)
_ITEM_TABLE = re.compile(r"^(?:static\s+)?ItemStateTable \w+\[[^\]]*\]\s*=\s*\{", re.MULTILINE)


def motion_tables(text: str, pattern: re.Pattern = _MOTION_TABLE) -> list[list[list[str]]]:
    """The rows of each retail motion-state table in a C file: per row, the
    identifiers it names (its callbacks among them), in table order. With
    `_ITEM_TABLE`, the same for item state tables."""
    out = []
    for m in pattern.finditer(text):
        rows: list[list[str]] = []
        depth = 1
        i = m.end()
        start = None
        while i < len(text) and depth > 0:
            c = text[i]
            if text.startswith("//", i):
                i = text.find("\n", i)
                continue
            if c == "{":
                depth += 1
                if depth == 2:
                    start = i
            elif c == "}":
                if depth == 2 and start is not None:
                    rows.append(_TOKEN.findall(text[start:i]))
                depth -= 1
            i += 1
        out.append(rows)
    return out


def motion_table_kind(path: Path) -> str | None:
    """The port's table name for a decomp file: `common` for ftmotionstates.c,
    else the FighterKind's Debug name (`ftFox/ftfox.c` -> `Fox`)."""
    if path.name == "ftmotionstates.c":
        return "common"
    if path.parent.name.startswith("ft") and path.parent.parent.name == "kinds":
        return path.parent.name[2:]
    return None


def implemented_rows(tsv: str) -> set[tuple[str, int]]:
    """(table, action) of every row `motion_rows` reports implemented."""
    out = set()
    for line in tsv.splitlines()[1:]:
        table, action, implemented = line.split("\t")
        if implemented == "true":
            out.add((table, int(action)))
    return out


# ---------------------------------------------------------------- Rust side

@dataclass
class LineInfo:
    code: str
    comment: str
    strings: str
    depth: int  # brace depth at the start of the line
    doc: bool  # the comment is `///`
    plain: bool  # the comment is `//` (not `//!`, not `///`)


def lex_rust(src: str) -> list[LineInfo]:
    """Split each line into code, comment and string-literal text, with the
    brace depth at the line's start. Handles nested block comments, raw
    strings, char literals versus lifetimes."""
    lines: list[LineInfo] = []
    code, comment, strings = [], [], []
    depth = 0
    line_depth = 0
    doc = plain = False
    i, n = 0, len(src)
    block = 0
    in_str = False
    raw_hashes = -1

    def flush():
        nonlocal code, comment, strings, line_depth, doc, plain
        lines.append(LineInfo("".join(code), "".join(comment), "".join(strings), line_depth, doc, plain))
        code, comment, strings = [], [], []
        line_depth = depth
        doc = plain = False

    while i < n:
        c = src[i]
        if c == "\n":
            if in_str:
                strings.append(" ")
            flush()
            i += 1
            continue
        if block:
            if src.startswith("/*", i):
                block += 1
                i += 2
            elif src.startswith("*/", i):
                block -= 1
                i += 2
            else:
                comment.append(c)
                i += 1
            continue
        if in_str:
            if raw_hashes >= 0:
                if c == '"' and src.startswith("#" * raw_hashes, i + 1):
                    in_str = False
                    i += 1 + raw_hashes
                    code.append('"')
                    continue
            elif c == "\\":
                strings.append(src[i:i + 2])
                i += 2
                continue
            elif c == '"':
                in_str = False
                code.append('"')
                i += 1
                continue
            strings.append(c)
            i += 1
            continue
        if src.startswith("//", i):
            j = src.find("\n", i)
            j = n if j < 0 else j
            text = src[i:j]
            doc = text.startswith("///") and not text.startswith("////")
            plain = not doc and not text.startswith("//!")
            comment.append(text.lstrip("/!"))
            i = j
            continue
        if src.startswith("/*", i):
            block = 1
            i += 2
            continue
        m = re.match(r'(b?r)(#*)"', src[i:i + 40]) if c in "rb" and (i == 0 or not (src[i - 1].isalnum() or src[i - 1] == "_")) else None
        if m:
            in_str = True
            raw_hashes = len(m.group(2))
            code.append('"')
            i += m.end()
            continue
        if c == '"':
            in_str = True
            raw_hashes = -1
            code.append('"')
            i += 1
            continue
        if c == "'":
            if src.startswith("\\", i + 1):
                j = src.find("'", i + 2)
                code.append("' '")
                i = j + 1
                continue
            if i + 2 < n and src[i + 2] == "'":
                code.append("' '")
                i += 3
                continue
            code.append(c)  # lifetime
            i += 1
            continue
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
        code.append(c)
        i += 1
    flush()
    return lines


_ITEM = re.compile(
    r"^(?:pub(?:\([^)]*\))?\s+)?(?:(?:const|async|unsafe|extern\s+\"C\")\s+)*"
    r"(fn|const|static|impl|trait|struct|enum|mod|type|macro_rules!)\b\s*(\w*)")
IMPLEMENTING_ITEMS = {"fn", "const", "static", "impl", "trait"}


@dataclass
class Item:
    kind: str
    name: str
    line: int  # 0-based
    end: int  # last line of the item (body), 0-based
    header: str  # the first line of the attached doc/comment block
    header_line: int
    doc: str = ""  # the whole attached comment block


def rust_items(lines: list[LineInfo]) -> list[Item]:
    """Items with the comment block attached directly above them. The block
    is the run of comment-only and attribute lines above the item."""
    items = []
    for idx, info in enumerate(lines):
        code = info.code.strip()
        m = _ITEM.match(code)
        if not m or code.startswith("fn(") or (m.group(1) == "fn" and not m.group(2)):
            continue
        kind, name = m.group(1), m.group(2)
        # Attached comment block.
        j = idx - 1
        first = None
        while j >= 0:
            prev = lines[j]
            pc = prev.code.strip()
            if not pc and (prev.doc or prev.plain):
                first = j
            elif pc.startswith(("#[", "#![")):
                pass
            else:
                break
            j -= 1
        header = lines[first].comment.strip() if first is not None else ""
        doc = " ".join(lines[k].comment.strip() for k in range(first, idx)) if first is not None else ""
        # Body extent: from the item's line to the line where depth returns.
        end = idx
        base = info.depth
        opened = False
        k = idx
        while k < len(lines):
            seg = lines[k].code
            if "{" in seg:
                opened = True
            if not opened and ";" in seg and kind in ("fn", "const", "static", "type", "mod"):
                end = k
                if kind != "const" and kind != "static":
                    break
            nxt = lines[k + 1].depth if k + 1 < len(lines) else base
            if opened and nxt <= base:
                end = k
                break
            if not opened and kind in ("const", "static") and ";" in seg:
                end = k
                break
            k += 1
        items.append(Item(kind, name, idx, end, header, first if first is not None else -1, doc))
    return items


TOOLING_CRATES = {"melee-sim", "melee-replay", "melee-diff", "melee-trace-io", "slp",
                  "melee-test-support", "gc-disc", "melee-web"}


def crate_of(path: Path, crates_root: Path) -> str:
    return path.relative_to(crates_root).parts[0]


def is_test_path(path: Path) -> bool:
    parts = path.parts
    return any(p in ("tests", "benches", "examples") for p in parts)


def test_line_ranges(lines: list[LineInfo]) -> list[tuple[int, int]]:
    """Line ranges of `#[cfg(test)]` modules."""
    out = []
    for idx, info in enumerate(lines):
        if info.code.strip().startswith("#[cfg(test)]"):
            k = idx + 1
            while k < len(lines) and not lines[k].code.strip():
                k += 1
            if k < len(lines) and re.match(r"^(pub\s+)?mod\b", lines[k].code.strip()):
                base = lines[k].depth
                e = k
                while e + 1 < len(lines) and (lines[e + 1].depth > base or e == k):
                    e += 1
                out.append((idx, e + 1))
    return out


# ----------------------------------------------------------- symbol resolver

_TOKEN = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
_HEX_TOKEN = re.compile(r"(?<![\w.])(?:0x)?(8[01][0-9A-Fa-f]{6})(?![\w])")
_ADDR_SUFFIX = re.compile(r"_([0-9A-Fa-f]{8})$")
# Decomp-style names: a subsystem prefix and an underscore.
_DECOMP_LIKE = re.compile(
    r"^(?:HSD|hsd|ft[A-Z]\w*|it[A-Z]\w*|it|ft|gr[A-Z]\w*|gr|mp[A-Z]\w*|lb[A-Z]?\w*|ef[A-Z]\w*|cm\w*|if[A-Z]\w*|gm|mn\w*|un\w*|pl\w*|"
    r"Ground|Item|Fighter|Camera|Player|Stage|fn)_\w+$")
# Lowercase symbol names that are also ordinary words or Rust identifiers.
_NAME_STOPLIST = {"callback", "cb", "exit", "main", "reset", "rand", "srand", "printf",
                  "memchr", "memcmp", "memcpy", "memmove", "memset", "memzero", "strlen",
                  "strchr", "strcmp", "strcpy", "strncmp", "strncpy", "tolower", "toupper",
                  "fwrite", "fwide", "sprintf", "vprintf", "vsnprintf", "bitrev", "roll2upvec",
                  "dropcallback", "powi", "frexp"}


class Resolver:
    def __init__(self, symbols: list[Symbol]):
        self.funcs = sorted((s for s in symbols if s.kind == "function" and s.section in CODE_SECTIONS),
                            key=lambda s: s.addr)
        self.starts = [f.addr for f in self.funcs]
        self.by_name = {f.name: f for f in self.funcs}
        self.data_names = {s.name for s in symbols if s.kind != "function"}
        objs = sorted((s for s in symbols if s.kind != "function"), key=lambda s: s.addr)
        self.obj_starts = [s.addr for s in objs]
        self.objs = objs
        self.inlines: set[str] = set()
        self.idents: set[str] = set()

    def func_at(self, addr: int) -> Symbol | None:
        i = bisect.bisect_right(self.starts, addr) - 1
        if i >= 0:
            f = self.funcs[i]
            if f.addr <= addr < f.addr + max(f.size, 4):
                return f
        return None

    def data_at(self, addr: int) -> bool:
        i = bisect.bisect_right(self.obj_starts, addr) - 1
        if i >= 0:
            o = self.objs[i]
            return o.addr <= addr < o.addr + max(o.size, 1)
        return False


_C_INLINE = re.compile(r"^static\s+inline\s+[\w\s\*]+?\b(\w+)\s*\(", re.MULTILINE)


def decomp_identifiers(src_root: Path) -> tuple[set[str], set[str]]:
    """(static inline function names, every identifier) in the decomp's C
    sources and headers. A decomp-style name the symbol table lacks is an
    inline (no retail function of its own), another decomp identifier (a
    type, field, enum or macro), or a stale name (renamed since)."""
    inlines: set[str] = set()
    idents: set[str] = set()
    for p in src_root.rglob("*"):
        if p.suffix not in (".c", ".h"):
            continue
        text = p.read_text(errors="replace")
        inlines.update(_C_INLINE.findall(text))
        idents.update(_TOKEN.findall(text))
    return inlines, idents


def cite_tokens(text: str, resolver: Resolver) -> list[tuple[Symbol | None, str, str]]:
    """Every symbol-like token in a piece of comment or string text."""
    out = []
    seen_spans = []
    for m in _TOKEN.finditer(text):
        tok = m.group(0)
        after = text[m.end():m.end() + 2]
        if after in (".c", ".h") and not text[m.end():m.end() + 3].startswith(".c:") and after == ".h":
            continue
        if after == ".c" and not _ADDR_SUFFIX.search(tok):
            continue  # a file name such as ftCo_Landing.c
        if tok in resolver.by_name and tok not in _NAME_STOPLIST:
            out.append((resolver.by_name[tok], "name", tok))
            seen_spans.append(m.span())
            continue
        sm = _ADDR_SUFFIX.search(tok)
        if sm and len(tok) > 9:
            addr = int(sm.group(1), 16)
            f = resolver.func_at(addr)
            seen_spans.append(m.span())
            if f is not None:
                out.append((f, "name+addr" if f.addr == addr else "addr-interior", tok))
            elif tok in resolver.data_names or resolver.data_at(addr):
                out.append((None, "data", tok))
            else:
                out.append((None, "unresolved-addr", tok))
            continue
        if _DECOMP_LIKE.match(tok) and tok not in resolver.data_names:
            if tok.endswith("_") or re.fullmatch(r"(ft|it)_[a-z_]+", tok):
                continue  # a name prefix (`ftCo_MS_`) or a Rust crate (`ft_fox_family`)
            if tok in resolver.inlines:
                out.append((None, "decomp-inline", tok))
            elif tok in resolver.idents:
                out.append((None, "decomp-identifier", tok))
            else:
                out.append((None, "unresolved-name", tok))
    for m in _HEX_TOKEN.finditer(text):
        if any(a <= m.start() < b for a, b in seen_spans):
            continue
        addr = int(m.group(1), 16)
        f = resolver.func_at(addr)
        if f is not None:
            out.append((f, "addr" if f.addr == addr else "addr-interior", m.group(0)))
        elif resolver.data_at(addr):
            out.append((None, "data", m.group(0)))
        elif 0x80003100 <= addr < 0x803B7240:
            out.append((None, "unresolved-addr", m.group(0)))
        else:
            out.append((None, "data", m.group(0)))
    return out


def header_tokens(header: str) -> str:
    """The part of an item's first comment line that names what it ports:
    the text before the first `: ` (the `sym (addr): description` form), or,
    without a colon, the line only when it starts with the symbol."""
    head = header.strip().strip("`")
    cut = re.search(r":(\s|$)", head)
    if cut:
        return head[:cut.start()]
    first = re.match(r"[`(]*([A-Za-z_]\w*|0x[0-9A-Fa-f]+|[0-9A-F]{8})", head)
    if first:
        # The leading run of symbols: `a / b`, `a (addr)`, `a, b`.
        m = re.match(r"^[`(]*[\w]+[`)]*(?:\s*(?:\(|/|,|and)\s*[`(]*[\w]+[`)]*)*", head)
        return m.group(0) if m else first.group(0)
    return ""


# ------------------------------------------------------------- fail-closed

_FAIL = re.compile(r"\b(unimplemented!|todo!|bail!|Error::UnsupportedFeature\()")
_C_LINE = re.compile(r"\b([A-Za-z_][\w]*\.c):(\d+)(?:-(\d+))?")
# bail!/errors outside these crates are input validation, not fail-closed sites.
_BAIL_CRATES = {"melee-lib", "melee-ef", "melee-ft", "melee-it", "melee-gr", "melee-mp", "melee-cm",
                "melee-lb", "melee-if", "melee-cpu", "melee-coll", "melee-cmd", "hsd-anim",
                "hsd-gobj", "hsd-particle"}


@dataclass
class Site:
    path: str
    line: int
    macro: str
    message: str
    context: str
    crate: str
    func: Symbol | None = None
    via: str = ""
    c_ref: str = ""
    enclosing: str = ""


def macro_text(lines: list[LineInfo], idx: int) -> str:
    """The macro call's string text and comments, up to its closing `)`."""
    parts = []
    depth = 0
    for k in range(idx, min(idx + 12, len(lines))):
        parts.append(lines[k].strings + " " + lines[k].comment)
        code = lines[k].code
        if k == idx:
            m = _FAIL.search(code)
            code = code[m.start():] if m else code
        depth += code.count("(") - code.count(")")
        if depth <= 0 and k >= idx:
            break
    return " ".join(" ".join(parts).split())


def preceding_comment(lines: list[LineInfo], idx: int) -> str:
    out = []
    k = idx - 1
    while k >= 0 and k >= idx - 6 and not lines[k].code.strip() and lines[k].comment.strip():
        out.insert(0, lines[k].comment.strip())
        k -= 1
    if lines[idx].comment.strip():
        out.append(lines[idx].comment.strip())
    return " ".join(out)


# ------------------------------------------------------------------ driver

@dataclass
class Inventory:
    rows: list[dict] = field(default_factory=list)
    sites: list[Site] = field(default_factory=list)
    form_counts: Counter = field(default_factory=Counter)
    unresolved: Counter = field(default_factory=Counter)


def build(decomp: Path, crates_root: Path, motion_rows: str | None = None) -> Inventory:
    cfg = decomp / "config" / "GALE01"
    symbols = parse_symbols((cfg / "symbols.txt").read_text())
    splits = parse_splits((cfg / "splits.txt").read_text())
    resolver = Resolver(symbols)
    resolver.inlines, resolver.idents = decomp_identifiers(decomp / "src")

    # C functions by source basename, for `file.c:line` references.
    src_root = decomp / "src"
    c_index: dict[str, list[tuple[str, int, int]]] = {}
    c_paths: dict[str, Path] = {}
    for p in src_root.rglob("*.c"):
        c_paths.setdefault(p.name.lower(), p)

    def c_lookup(fname: str, line: int) -> Symbol | None:
        key = fname.lower()
        if key not in c_index:
            p = c_paths.get(key)
            c_index[key] = c_functions(p.read_text(errors="replace")) if p else []
        for name, a, b in c_index[key]:
            if a <= line <= b:
                f = resolver.by_name.get(name)
                if f is None:
                    sm = _ADDR_SUFFIX.search(name)
                    f = resolver.func_at(int(sm.group(1), 16)) if sm else None
                return f
        return None

    ported: dict[int, list[str]] = defaultdict(list)
    partial: dict[int, list[str]] = defaultdict(list)
    referenced: dict[int, list[str]] = defaultdict(list)
    body: dict[int, list[str]] = defaultdict(list)
    inv = Inventory()

    for path in sorted(crates_root.rglob("*.rs")):
        if "target" in path.parts:
            continue
        rel = path.relative_to(crates_root.parent).as_posix()
        crate = crate_of(path, crates_root)
        lines = lex_rust(path.read_text(errors="replace"))
        test_mods = test_line_ranges(lines)
        whole_file_test = is_test_path(path)

        def in_test(i: int, test_mods=test_mods, whole_file_test=whole_file_test) -> bool:
            return whole_file_test or any(a <= i < b for a, b in test_mods)

        tooling = crate in TOOLING_CRATES
        items = rust_items(lines)

        # Header citations: the functions each implementing item ports.
        header_cites: dict[int, list[Symbol]] = {}
        for it in items:
            cites: list[Symbol] = []
            if it.kind in IMPLEMENTING_ITEMS and it.header and not tooling and not in_test(it.line):
                head = header_tokens(it.header)
                for f, form, tok in cite_tokens(head, resolver):
                    if f is not None and form in ("name", "name+addr", "addr") and not re.search(re.escape(tok) + r"'s\b", it.header):
                        cites.append(f)
            if it.kind == "fn" and it.name in resolver.by_name and it.name not in _NAME_STOPLIST and not tooling and not in_test(it.line):
                cites.append(resolver.by_name[it.name])
            if cites:
                header_cites[it.line] = cites
                for f in cites:
                    ported[f.addr].append(f"{rel}:{it.line + 1}")
        header_addrs: dict[int, set[int]] = defaultdict(set)
        # A module whose `//!` doc opens with a symbol ports that function.
        if lines and lines[0].comment.strip() and not lines[0].code.strip() \
                and not lines[0].doc and not lines[0].plain and not tooling and not in_test(0):
            for f, form, tok in cite_tokens(header_tokens(lines[0].comment), resolver):
                if f is not None and form in ("name", "name+addr", "addr"):
                    ported[f.addr].append(f"{rel}:1")
                    header_addrs[0].add(f.addr)
        for it in items:
            for f in header_cites.get(it.line, []):
                header_addrs[it.header_line].add(f.addr)
        # Lines inside a fn/const/static item: its comment block and body.
        inside = [False] * len(lines)
        if not tooling:
            for it in items:
                if it.kind in ("fn", "const", "static"):
                    for k in range(it.header_line if it.header_line >= 0 else it.line, it.end + 1):
                        inside[k] = True

        # A fail-closed macro's message names what is NOT ported: never `body`.
        for idx, info in enumerate(lines):
            if _FAIL.search(info.code):
                depth = 0
                for k in range(idx, min(idx + 12, len(lines))):
                    inside[k] = False
                    depth += lines[k].code.count("(") - lines[k].code.count(")")
                    if depth <= 0:
                        break

        # Every citation, for the resolution rate, `body` and `referenced`.
        for idx, info in enumerate(lines):
            for text in (info.comment, info.strings):
                for f, form, tok in cite_tokens(text, resolver):
                    inv.form_counts[form] += 1
                    if f is None:
                        if form.startswith("unresolved"):
                            inv.unresolved[tok] += 1
                        continue
                    if f.addr in header_addrs.get(idx, ()):
                        continue
                    where = f"{rel}:{idx + 1}"
                    if inside[idx] and not in_test(idx):
                        body[f.addr].append(where)
                    else:
                        referenced[f.addr].append(where)

        # Fail-closed sites.
        for idx, info in enumerate(lines):
            m = _FAIL.search(info.code)
            if not m or in_test(idx) or tooling:
                continue
            macro = m.group(1).rstrip("(!")
            if macro == "bail" and crate not in _BAIL_CRATES:
                continue
            if info.comment.strip().startswith("/") or "`" + m.group(1) in info.code:
                continue
            msg = macro_text(lines, idx)
            if re.fullmatch(r"\{\w+(:#?\??)?\}", msg.strip()):
                continue  # re-raising another error, not a site of its own
            ctx = preceding_comment(lines, idx)
            enclosing = [it for it in items if it.kind == "fn" and it.line <= idx <= it.end]
            enclosing.sort(key=lambda it: it.line, reverse=True)
            site = Site(rel, idx + 1, macro, msg, ctx, crate,
                        enclosing=enclosing[0].name if enclosing else "")
            for source, text in (("message", msg), ("comment", ctx)):
                if site.func:
                    break
                for f, form, tok in cite_tokens(text, resolver):
                    if f is not None and form != "data":
                        site.func, site.via = f, source
                        break
                if site.func:
                    break
                cm = _C_LINE.search(text)
                if cm:
                    site.c_ref = cm.group(0)
                    f = c_lookup(cm.group(1), int(cm.group(2)))
                    if f is not None:
                        site.func, site.via = f, source + " C line"
            if not site.c_ref:
                cm = _C_LINE.search(msg + " " + ctx)
                site.c_ref = cm.group(0) if cm else ""
            for it in enclosing:
                if site.func:
                    break
                if it.line in header_cites:
                    site.func, site.via = header_cites[it.line][0], f"enclosing fn {it.name}"
            for it in enclosing:
                if site.func:
                    break
                for f, form, _ in cite_tokens(it.doc, resolver):
                    if f is not None and form != "data":
                        site.func, site.via = f, f"enclosing fn doc {it.name}"
                        break
            for it in enclosing:
                for f in header_cites.get(it.line, []):
                    partial[f.addr].append(f"{rel}:{idx + 1}")
            if site.func is not None:
                partial[site.func.addr].append(f"{rel}:{idx + 1}")
            inv.sites.append(site)

    # Motion-state callbacks of the rows the port implements.
    rows_at: dict[int, list[str]] = defaultdict(list)
    if motion_rows is not None:
        done = implemented_rows(motion_rows)
        for p in sorted((src_root / "melee" / "ft").rglob("*.c")):
            kind = motion_table_kind(p)
            if kind is None:
                continue
            for table_rows in motion_tables(p.read_text(errors="replace")):
                base = 0 if kind == "common" else 341
                for index, idents in enumerate(table_rows):
                    action = base + index
                    if (kind, action) not in done:
                        continue
                    for tok in idents:
                        f = resolver.by_name.get(tok)
                        if f is not None:
                            rows_at[f.addr].append(f"{kind}:{action}")

    # Item state rows: a row with one callback ported is a ported state, and
    # its other callbacks (often empty or a one-call wrapper) come with it.
    for p in sorted((src_root / "melee" / "it").rglob("*.c")):
        for table_rows in motion_tables(p.read_text(errors="replace"), _ITEM_TABLE):
            for index, idents in enumerate(table_rows):
                funcs = [resolver.by_name[t] for t in idents if t in resolver.by_name]
                if any(f.addr in ported or f.addr in body for f in funcs):
                    for f in funcs:
                        rows_at[f.addr].append(f"{p.stem}:{index}")

    for f in resolver.funcs:
        src = source_file(f.addr, splits)
        if f.addr in partial and (f.addr in ported or f.addr in body or f.addr in rows_at):
            status = "partial"
        elif f.addr in ported:
            status = "ported"
        elif f.addr in body:
            status = "body"
        elif f.addr in rows_at:
            status = "row"
        elif f.addr in referenced or f.addr in partial:
            status = "referenced"
        else:
            status = "not started"
        inv.rows.append({
            "address": f"{f.addr:08X}", "size": f.size, "name": f.name, "source": src,
            "subsystem": subsystem(src), "area": area(src), "scope": scope(src), "status": status,
            "ported_at": " ".join(dict.fromkeys(ported.get(f.addr, []))),
            "fail_closed_at": " ".join(dict.fromkeys(partial.get(f.addr, []))),
            "body_at": " ".join(list(dict.fromkeys(body.get(f.addr, [])))[:5]),
            "motion_rows": " ".join(list(dict.fromkeys(rows_at.get(f.addr, [])))[:5]),
            "referenced_at": " ".join(list(dict.fromkeys(referenced.get(f.addr, [])))[:5]),
        })
    for s in inv.sites:
        s_src = source_file(s.func.addr, splits) if s.func else ""
        s.__dict__["subsystem"] = subsystem(s_src) if s_src else ""
    return inv


STATUSES = ["ported", "partial", "body", "row", "referenced", "not started"]


def write_tsv(inv: Inventory, out: Path) -> None:
    out.mkdir(parents=True, exist_ok=True)
    with open(out / "inventory.tsv", "w", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=list(inv.rows[0]), delimiter="\t", lineterminator="\n")
        w.writeheader()
        w.writerows(inv.rows)
    with open(out / "fail_closed.tsv", "w", newline="") as fh:
        w = csv.writer(fh, delimiter="\t", lineterminator="\n")
        w.writerow(["address", "function", "subsystem", "rust_location", "macro", "c_ref",
                    "resolved_via", "enclosing_rust_fn", "message", "comment"])
        for s in inv.sites:
            w.writerow([f"{s.func.addr:08X}" if s.func else "", s.func.name if s.func else "",
                        s.__dict__.get("subsystem", ""), f"{s.path}:{s.line}", s.macro, s.c_ref,
                        s.via, s.enclosing, s.message[:300], s.context[:300]])


def table(rows: list[dict], key: str, in_game_only: bool = False) -> list[str]:
    groups: dict[str, Counter] = defaultdict(Counter)
    for r in rows:
        if in_game_only and r["scope"] != "in-game":
            continue
        g = r[key]
        groups[g][r["status"]] += 1
        groups[g][r["status"] + "_b"] += r["size"]
        groups[g]["n"] += 1
        groups[g]["b"] += r["size"]
    out = [f"| {key} | functions | ported | partial | body | row | referenced | not started | KiB | bytes ported+partial | with body+row |",
           "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|"]
    total = Counter()
    for g in sorted(groups, key=lambda g: -groups[g]["b"]):
        c = groups[g]
        total.update(c)
        out.append(_row(g, c))
    out.append(_row("**total**", total))
    return out


def _row(name: str, c: Counter) -> str:
    done_b = c["ported_b"] + c["partial_b"]
    pct = 100 * done_b / c["b"] if c["b"] else 0
    pct_wide = 100 * (done_b + c["body_b"] + c["row_b"]) / c["b"] if c["b"] else 0
    return (f"| {name} | {c['n']} | {c['ported']} | {c['partial']} | {c['body']} | {c['row']} | {c['referenced']} | "
            f"{c['not started']} | {c['b'] / 1024:.0f} | {pct:.1f}% | {pct_wide:.1f}% |")


def markdown(inv: Inventory) -> str:
    out = ["## By area, every function", ""] + table(inv.rows, "area")
    out += ["", "## By area, in-game logic only", ""] + table(inv.rows, "area", True)
    out += ["", "## By scope", ""] + table(inv.rows, "scope")
    out += ["", "## By subsystem, in-game logic only", ""] + table(inv.rows, "subsystem", True)
    files: dict[str, Counter] = defaultdict(Counter)
    for r in inv.rows:
        if r["scope"] == "in-game":
            files[r["source"]]["b"] += r["size"]
            files[r["source"]][r["status"]] += r["size"]
    out += ["", "## Largest not-started source files, in-game logic", "",
            "| source | KiB | not started | ported+partial | with body+row |", "|---|---:|---:|---:|---:|"]
    for src in sorted(files, key=lambda s: -files[s]["not started"])[:60]:
        c = files[src]
        out.append(f"| {src} | {c['b'] / 1024:.1f} | {100 * c['not started'] / c['b']:.0f}% | "
                   f"{100 * (c['ported'] + c['partial']) / c['b']:.0f}% | "
                   f"{100 * (c['ported'] + c['partial'] + c['body'] + c['row']) / c['b']:.0f}% |")
    total = sum(inv.form_counts.values())
    out += ["", "## Citations", "", "| form | count |", "|---|---:|"]
    for form, n in inv.form_counts.most_common():
        out.append(f"| {form} | {n} ({100 * n / total:.1f}%) |")
    fc = inv.form_counts
    resolved = fc["name"] + fc["name+addr"] + fc["addr"] + fc["addr-interior"]
    candidates = resolved + fc["decomp-inline"] + fc["unresolved-name"] + fc["unresolved-addr"]
    out += ["", (f"Function citations resolved: {resolved} of {candidates} "
                f"({100 * resolved / max(candidates, 1):.1f}%); decomp inlines {fc['decomp-inline']}, "
                f"unresolved names {fc['unresolved-name']}, unresolved addresses {fc['unresolved-addr']}. "
                f"Not function citations: data {fc['data']}, other decomp identifiers {fc['decomp-identifier']}.")]
    out += ["", "Unresolved tokens (top 40): " + ", ".join(f"`{t}` {n}" for t, n in inv.unresolved.most_common(40))]
    by_sub = Counter(s.__dict__.get("subsystem") or "(unresolved)" for s in inv.sites)
    out += ["", f"## Fail-closed sites: {len(inv.sites)}", "",
            "| retail subsystem | sites |", "|---|---:|"]
    out += [f"| {k} | {v} |" for k, v in by_sub.most_common()]
    by_crate = Counter(s.crate for s in inv.sites)
    out += ["", "| port crate | sites |", "|---|---:|"] + [f"| {k} | {v} |" for k, v in by_crate.most_common()]
    out += ["", "Resolved via: " + ", ".join(f"{k or 'unresolved'} {v}" for k, v in
                                             Counter(s.via.split(' ')[0] if s.via else '' for s in inv.sites).most_common())]
    return "\n".join(out)


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--decomp", type=Path, default=DECOMP)
    ap.add_argument("--crates", type=Path, default=CODE_ROOT / "crates")
    ap.add_argument("--out", type=Path, default=CODE_ROOT / "target" / "parity")
    ap.add_argument("--markdown", action="store_true", help="print the summary tables")
    ap.add_argument("--motion-rows", type=Path, help="the TSV `cargo run --release -p melee-lib --example motion_rows` prints")
    args = ap.parse_args(argv)
    if not (args.decomp / "config" / "GALE01" / "symbols.txt").exists():
        print(f"missing {args.decomp}/config/GALE01/symbols.txt: run `git submodule update --init "
              "third_party/melee-decomp`, or set MELEE_DATA_ROOT to a checkout that has it",
              file=sys.stderr)
        return 2
    rows = args.motion_rows.read_text() if args.motion_rows else None
    inv = build(args.decomp, args.crates, rows)
    write_tsv(inv, args.out)
    counts = Counter(r["status"] for r in inv.rows)
    print(f"{len(inv.rows)} functions: " + ", ".join(f"{s} {counts[s]}" for s in STATUSES)
          + f"; {len(inv.sites)} fail-closed sites; wrote {args.out}")
    if args.markdown:
        print(markdown(inv))
    return 0


if __name__ == "__main__":
    sys.exit(main())
