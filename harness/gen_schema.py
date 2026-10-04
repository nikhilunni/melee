"""Generate harness/schema/*.generated.yaml from the decomp's struct headers.

The decomp annotates struct members with offset comments:

    /* fp+B0 */ Vec3 cur_pos;        absolute offset from the struct base
    /*  +10 */ int level;            offset relative to the enclosing struct
    /* 0x1A88 */ ...                 same, hex-prefixed
    /* fp+2218:0 */ u8 flag : 1;     byte:bit for bitfields
    /// @at{2C} @sz{4}               doxygen form (it/types.h uses this)

This script does *not* parse C. It scans a struct body for those comments,
pairs each with the declaration that follows, and recurses into nested
struct/union bodies (inline `struct dmg { ... } dmg;` as well as
`struct CpuFighter cpu;` when `struct CpuFighter` is defined in the same
header). Nested `+XX` offsets are relative to the enclosing member's offset;
`fp+XX` offsets are always absolute.

C types are mapped onto the schema scalars (u8 u16 u32 s8 s16 s32 f32 f64 ptr
vec3). Enums, `int`, `bool` and `enum_t` become s32; anything with a `*` or a
function-pointer typedef becomes ptr. Everything else (arrays, bitfields,
nested aggregates, opaque typedefs) is emitted as `unknown` with the C type
string so nothing is silently dropped.

Alongside each `*.generated.yaml` the script writes a `*.generated.json` with
identical content (struct, size, source, ordered fields) for consumers without
a YAML parser: the Rust schema-coverage test in `crates/melee-sim` loads it via
`include_str!`. The hand-written `fighter.yaml` is likewise mirrored to
`fighter.hand.generated.json` so the same test can check an emitter against
the subset the decoder currently reads.

Usage (from harness/):
    uv run python gen_schema.py            # writes schema/*.generated.{yaml,json}, prints report
    uv run python gen_schema.py --check    # exit 1 on layout violations
"""
from __future__ import annotations

import json
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

import yaml

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import data_root  # noqa: E402

ROOT = HERE.parent
# The decomp is machine-local data: a worktree reads the main checkout's.
DECOMP = data_root.DECOMP
SRC = DECOMP / "src"
SCHEMA_DIR = HERE / "schema"

SCHEMA_SCALARS = {"u8", "u16", "u32", "s8", "s16", "s32", "f32", "f64", "ptr", "vec3"}

# Primitive C spellings -> schema types.
PRIMITIVES = {
    "u8": "u8", "s8": "s8", "u16": "u16", "s16": "s16", "u32": "u32", "s32": "s32",
    "f32": "f32", "f64": "f64",
    "float": "f32", "double": "f64",
    "int": "s32", "signed": "s32", "signed int": "s32", "long": "s32", "signed long": "s32",
    "unsigned": "u32", "unsigned int": "u32", "unsigned long": "u32",
    "short": "s16", "signed short": "s16", "short int": "s16", "unsigned short": "u16",
    "char": "s8", "signed char": "s8", "unsigned char": "u8",
    "bool": "s32",      # MSL/stdbool.h: typedef int bool;
    "BOOL": "s32",      # dolphin/types.h: typedef int BOOL;
    "enum_t": "s32",    # Runtime/platform.h: typedef int enum_t;
    "size_t": "u32", "ssize_t": "s32",
    "Vec": "vec3", "Vec3": "vec3",   # dolphin/mtx.h:16  } Vec, Vec3, ...
    "UNK_T": "ptr",     # placeholder.h: #define UNK_T void*
}


# --------------------------------------------------------------------------- #
# Offset comments
# --------------------------------------------------------------------------- #

# `fp+B0`, `fp+x1A88` (a typo the header carries), `+10`, `0x1A88`, each with
# an optional `:bit` suffix.
_OFFSET_RE = re.compile(
    r"^\s*(?P<form>fp\s*\+|\+|0[xX])\s*x?(?P<hex>[0-9A-Fa-f]+)(?::(?P<bit>\d+))?\s*$"
)
_AT_RE = re.compile(r"///?<?[^\n]*?@at\{([0-9A-Fa-f]+)\}[^\n]*")


@dataclass
class Off:
    value: int          # byte offset, absolute (fp+) or relative (+/0x) per `absolute`
    absolute: bool
    bit: int | None = None

    def resolve(self, base: int | None) -> int | None:
        if self.absolute:
            return self.value
        if base is None:
            return None
        return base + self.value


_AT_INLINE_RE = re.compile(r"@at\{([0-9A-Fa-f]+)\}")


def parse_offset_comment(text: str) -> Off | None:
    m = _OFFSET_RE.match(text)
    if not m:
        # `/** @at{D38} @sz{4} ... */` block doc comments (it/types.h).
        at = _AT_INLINE_RE.search(text)
        if at:
            return Off(value=int(at.group(1), 16), absolute=False)
        return None
    return Off(
        value=int(m.group("hex"), 16),
        absolute=m.group("form").startswith("fp"),
        bit=int(m.group("bit")) if m.group("bit") is not None else None,
    )


def preprocess(text: str) -> str:
    """Rewrite `/// @at{XX}` into `/* +XX */` and blank `//` comments.

    Length-preserving so that positions map back to source lines.
    """
    def at_repl(m: re.Match) -> str:
        rep = f"/* +{m.group(1)} */"
        return rep.ljust(len(m.group(0))) if len(rep) <= len(m.group(0)) else rep

    text = _AT_RE.sub(at_repl, text)
    out = []
    i, n = 0, len(text)
    while i < n:
        if text.startswith("/*", i):
            j = text.find("*/", i + 2)
            j = n if j < 0 else j + 2
            out.append(text[i:j])
            i = j
        elif text.startswith("//", i):
            j = text.find("\n", i)
            j = n if j < 0 else j
            out.append(" " * (j - i))
            i = j
        else:
            out.append(text[i])
            i += 1
    return "".join(out)


# --------------------------------------------------------------------------- #
# Typedef table (enum / pointer / alias), built by scanning every header
# --------------------------------------------------------------------------- #

@dataclass
class TypedefTable:
    kinds: dict[str, str] = field(default_factory=dict)   # name -> enum|ptr|struct|union|array|alias
    alias: dict[str, str] = field(default_factory=dict)   # name -> aliased spelling
    enum_tags: set[str] = field(default_factory=set)

    def _scan(self, text: str) -> None:
        text = preprocess(text)
        for m in re.finditer(r"\benum\s+(\w+)\s*\{", text):
            self.enum_tags.add(m.group(1))
        for m in re.finditer(r"\btypedef\b", text):
            start = m.end()
            depth = 0
            i = start
            while i < len(text):
                c = text[i]
                if c == "{":
                    depth += 1
                elif c == "}":
                    depth -= 1
                elif c == ";" and depth == 0:
                    break
                i += 1
            decl = text[start:i]
            self._record(decl)

    def _record(self, decl: str) -> None:
        decl = re.sub(r"/\*.*?\*/", " ", decl, flags=re.S).strip()
        if not decl:
            return
        if "{" in decl:
            head = decl[: decl.index("{")].split()
            kind = head[0] if head and head[0] in ("struct", "union", "enum") else None
            names = decl[decl.rindex("}") + 1:]
            for name in names.split(","):
                name = name.strip()
                if not name:
                    continue
                is_ptr = "*" in name
                name = name.strip("* ").split("[")[0].strip()
                if not name:
                    continue
                if is_ptr:
                    self.kinds[name] = "ptr"
                elif kind:
                    self.kinds[name] = kind
            return
        fp = re.match(r"^.*\(\s*\*\s*(\w+)\s*\)\s*\(.*$", decl, flags=re.S)
        if fp:
            self.kinds[fp.group(1)] = "ptr"
            return
        arr = re.match(r"^(.*?)\b(\w+)\s*((?:\[[^\]]*\])+)$", decl, flags=re.S)
        if arr:
            self.kinds[arr.group(2)] = "array"
            return
        m = re.match(r"^(.*?)\s*\b(\w+)\s*$", decl, flags=re.S)
        if not m:
            return
        base, name = m.group(1).strip(), m.group(2)
        if "*" in base:
            self.kinds[name] = "ptr"
        elif base.startswith("enum "):
            self.kinds[name] = "enum"
        elif base.startswith(("struct ", "union ")):
            self.kinds[name] = base.split()[0]
        else:
            self.alias[name] = base

    @classmethod
    def build(cls, roots: list[Path]) -> "TypedefTable":
        t = cls()
        for root in roots:
            for h in sorted(root.rglob("*.h")):
                try:
                    t._scan(h.read_text(errors="replace"))
                except OSError:
                    pass
        return t

    def classify(self, ctype: str) -> str:
        """Map a C type spelling (no declarator) to a schema type or 'unknown'."""
        c = re.sub(r"\b(const|volatile|static)\b", " ", ctype)
        c = " ".join(c.split())
        if "*" in c:
            return "ptr"
        if c.startswith("enum "):
            return "s32"
        if c.startswith(("struct ", "union ")):
            return "unknown"
        seen: set[str] = set()
        while c not in seen:
            seen.add(c)
            if c in PRIMITIVES:
                return PRIMITIVES[c]
            k = self.kinds.get(c)
            if k == "enum":
                return "s32"
            if k == "ptr":
                return "ptr"
            if k in ("struct", "union", "array"):
                return "unknown"
            if c in self.alias:
                c = " ".join(self.alias[c].split())
                if "*" in c:
                    return "ptr"
                if c.startswith("enum "):
                    return "s32"
                if c.startswith(("struct ", "union ")):
                    return "unknown"
                continue
            break
        if c in self.enum_tags:
            return "s32"
        return "unknown"


# --------------------------------------------------------------------------- #
# Struct body scanner
# --------------------------------------------------------------------------- #

@dataclass
class Node:
    kind: str                     # 'field' | 'struct' | 'union'
    name: str                     # dotted path for fields; container name or '' if anonymous
    offset: int | None            # absolute byte offset
    bit: int | None = None
    width: int | None = None
    schema_type: str = ""
    ctype: str = ""
    line: int = 0
    children: list["Node"] = field(default_factory=list)

    def first_offset(self) -> tuple[int, int] | None:
        if self.kind == "field":
            return None if self.offset is None else (self.offset, self.bit or 0)
        for ch in self.children:
            fo = ch.first_offset()
            if fo is not None:
                return fo
        return None

    def leaves(self):
        if self.kind == "field":
            yield self
        for ch in self.children:
            yield from ch.leaves()


class Header:
    def __init__(self, path: Path, typedefs: TypedefTable) -> None:
        self.path = path
        self.rel = str(path.relative_to(data_root.ROOT))
        self.raw = path.read_text()
        self.text = preprocess(self.raw)
        self.typedefs = typedefs
        self.notes: list[str] = []
        self.warnings: list[str] = []

    def line_of(self, pos: int) -> int:
        return self.text.count("\n", 0, pos) + 1

    # -- locating a struct -------------------------------------------------- #

    def find_struct(self, name: str) -> tuple[int, int, int] | None:
        """Return (body_start, body_end, decl_line) for `struct NAME {`."""
        m = re.search(rf"\b(?:struct|union)\s+{re.escape(name)}\s*\{{", self.text)
        if not m:
            return None
        start = m.end()
        end = self._match_brace(start - 1)
        return start, end, self.line_of(m.start())

    def assert_size(self, name: str) -> int | None:
        m = re.search(rf"ASSERT_SIZE\(\s*(?:struct|union)\s+{re.escape(name)}\s*,\s*(0[xX][0-9A-Fa-f]+|\d+)\s*\)", self.text)
        return int(m.group(1), 0) if m else None

    def _match_brace(self, open_pos: int) -> int:
        depth = 0
        i = open_pos
        n = len(self.text)
        while i < n:
            if self.text.startswith("/*", i):
                j = self.text.find("*/", i + 2)
                i = n if j < 0 else j + 2
                continue
            c = self.text[i]
            if c == "{":
                depth += 1
            elif c == "}":
                depth -= 1
                if depth == 0:
                    return i
            i += 1
        raise ValueError("unbalanced braces")

    # -- scanning a body ---------------------------------------------------- #

    def scan(self, name: str) -> Node | None:
        loc = self.find_struct(name)
        if loc is None:
            return None
        start, end, line = loc
        root = Node("struct", name, 0, line=line)
        self._scan_body(start, end, base=0, prefix="", into=root)
        return root

    def _scan_body(self, start: int, end: int, base: int | None, prefix: str, into: Node) -> None:
        text = self.text
        i = start
        pending: Off | None = None
        while i < end:
            c = text[i]
            if c.isspace():
                i += 1
                continue
            if text.startswith("/*", i):
                j = text.find("*/", i + 2)
                j = end if j < 0 else j + 2
                off = parse_offset_comment(text[i + 2:j - 2])
                if off is not None:
                    pending = off
                i = j
                continue
            # declaration: read to ';' at depth 0
            j = i
            depth = 0
            while j < end:
                if text.startswith("/*", j):
                    k = text.find("*/", j + 2)
                    j = end if k < 0 else k + 2
                    continue
                ch = text[j]
                if ch == "{":
                    depth += 1
                elif ch == "}":
                    depth -= 1
                elif ch == ";" and depth == 0:
                    break
                j += 1
            decl_pos = i
            self._handle_decl(decl_pos, j, pending, base, prefix, into)
            pending = None
            i = j + 1

    def _handle_decl(self, dstart: int, dend: int, off: Off | None, base: int | None,
                     prefix: str, into: Node) -> None:
        text = self.text
        decl = text[dstart:dend]
        line = self.line_of(dstart)
        abs_off = off.resolve(base) if off is not None else None
        if off is not None and abs_off is None:
            self.warnings.append(f"{self.rel}:{line}: relative offset +{off.value:X} with unknown base; dropped")

        brace = decl.find("{")
        if brace >= 0 and re.match(r"^\s*(struct|union)\b", decl):
            close = self._match_brace(dstart + brace)
            head = decl[:brace].split()
            kind = head[0]
            tag = head[1] if len(head) > 1 else ""
            tail = text[close + 1:dend].strip()
            inner_start, inner_end = dstart + brace + 1, close
            if not tail:
                node = Node(kind, "", abs_off, line=line, ctype=f"{kind} {tag}".strip())
                self._scan_body(inner_start, inner_end, abs_off if abs_off is not None else base, prefix, node)
                if not any(node.leaves()) and abs_off is not None:
                    into.children.append(Node("field", f"{prefix}_anon_{abs_off:X}", abs_off,
                                              schema_type="unknown", ctype=node.ctype, line=line))
                else:
                    into.children.append(node)
                return
            for d in self._split_top(tail):
                is_ptr = "*" in d
                arr = re.search(r"\[.*\]", d)
                dname = re.sub(r"[\*\[\]\s]|\[.*", "", d.split("[")[0]).strip()
                ctype = f"{kind} {tag}".strip() + ("*" if is_ptr else "") + (arr.group(0) if arr else "")
                if is_ptr:
                    into.children.append(Node("field", prefix + dname, abs_off, schema_type="ptr", ctype=ctype, line=line))
                elif arr:
                    into.children.append(Node("field", prefix + dname, abs_off, schema_type="unknown", ctype=ctype, line=line))
                else:
                    node = Node(kind, prefix + dname, abs_off, line=line, ctype=ctype)
                    self._scan_body(inner_start, inner_end, abs_off if abs_off is not None else base, prefix + dname + ".", node)
                    if not any(node.leaves()) and abs_off is not None:
                        into.children.append(Node("field", prefix + dname, abs_off, schema_type="unknown", ctype=ctype, line=line))
                    else:
                        into.children.append(node)
            return

        # simple declaration(s): TYPE decl1, decl2 ...
        flat = " ".join(re.sub(r"/\*.*?\*/", " ", decl, flags=re.S).split())
        if not flat:
            return
        parts = self._split_top(flat)
        first = parts[0]
        ctype, dname, extra = self._split_declarator(first)
        if dname is None:
            self.warnings.append(f"{self.rel}:{line}: could not parse declaration {flat!r}")
            return
        decls = [(ctype, dname, extra)]
        for p in parts[1:]:
            _, n2, e2 = self._split_declarator("int " + p.strip())
            if n2:
                decls.append((ctype, n2, e2))
        for ctype, dname, extra in decls:
            self._emit_simple(into, prefix, dname, ctype, extra, abs_off, off, base, line)

    def _emit_simple(self, into: Node, prefix: str, dname: str, ctype: str, extra: dict,
                     abs_off: int | None, off: Off | None, base: int | None, line: int) -> None:
        if off is None:
            return  # no offset comment: not part of the schema
        if abs_off is None:
            return
        # Nested struct by reference to a struct defined in this header
        # (e.g. `struct CpuFighter cpu;`): recurse with the member's offset as base.
        m = re.match(r"^(struct|union)\s+(\w+)$", ctype)
        if m and not extra.get("ptr") and not extra.get("array") and self.find_struct(m.group(2)):
            node = Node(m.group(1), prefix + dname, abs_off, line=line, ctype=ctype)
            s, e, _ = self.find_struct(m.group(2))
            self._scan_body(s, e, abs_off, prefix + dname + ".", node)
            if node.children:
                into.children.append(node)
                return
        if extra.get("bits") is not None:
            base_t = self.typedefs.classify(ctype)
            into.children.append(Node("field", prefix + dname, abs_off, bit=off.bit if off.bit is not None else 0,
                                      width=extra["bits"], schema_type="unknown",
                                      ctype=f"{ctype}:{extra['bits']}", line=line))
            return
        if extra.get("ptr"):
            into.children.append(Node("field", prefix + dname, abs_off, schema_type="ptr",
                                      ctype=ctype + "*" * extra["ptr"], line=line))
            return
        if extra.get("array"):
            into.children.append(Node("field", prefix + dname, abs_off, schema_type="unknown",
                                      ctype=ctype + extra["array"], line=line))
            return
        st = self.typedefs.classify(ctype)
        into.children.append(Node("field", prefix + dname, abs_off, schema_type=st, ctype=ctype, line=line))

    @staticmethod
    def _split_top(s: str) -> list[str]:
        out, depth, cur = [], 0, []
        for ch in s:
            if ch in "([{":
                depth += 1
            elif ch in ")]}":
                depth -= 1
            if ch == "," and depth == 0:
                out.append("".join(cur))
                cur = []
            else:
                cur.append(ch)
        if "".join(cur).strip():
            out.append("".join(cur))
        return [p.strip() for p in out]

    @staticmethod
    def _split_declarator(s: str) -> tuple[str, str | None, dict]:
        """'HSD_GObj* next' -> ('HSD_GObj', 'next', {'ptr': 1}); handles arrays, bitfields, (*name)[N], (*name)(args)."""
        s = s.strip()
        extra: dict = {}
        m = re.match(r"^(.*?)\(\s*\*+\s*(\w+)\s*\)\s*(\[.*\]|\(.*\))?\s*$", s)
        if m:
            extra["ptr"] = 1
            return m.group(1).strip(), m.group(2), extra
        m = re.match(r"^(.*?)\b(\w+)\s*:\s*(\d+)\s*$", s)
        if m:
            extra["bits"] = int(m.group(3))
            s = m.group(1) + " " + m.group(2)
        m = re.match(r"^(.*?)\b(\w+)\s*((?:\[[^\]]*\]\s*)+)$", s)
        if m:
            extra["array"] = "".join(m.group(3).split())
            s = m.group(1) + " " + m.group(2)
        m = re.match(r"^(.*?)(\**)\s*(\w+)\s*$", s.strip())
        if not m:
            return s, None, extra
        ctype = m.group(1).strip()
        stars = m.group(2).count("*") + ctype.count("*")
        ctype = ctype.replace("*", "").strip()
        if stars:
            extra["ptr"] = stars
        return ctype, m.group(3), extra


# --------------------------------------------------------------------------- #
# Validation
# --------------------------------------------------------------------------- #

def validate(root: Node, size: int | None, hdr: Header) -> tuple[list[str], list[str]]:
    violations: list[str] = []
    notes: list[str] = []

    def cite(n: Node) -> str:
        return f"{hdr.rel}:{n.line}"

    def walk(node: Node) -> None:
        if node.kind == "struct":
            prev: tuple[int, int] | None = None
            prev_node: Node | None = None
            for ch in node.children:
                fo = ch.first_offset()
                if fo is not None:
                    if prev is not None and fo < prev:
                        violations.append(
                            f"{cite(ch)}: {ch.name or ch.ctype} at 0x{fo[0]:X}:{fo[1]} follows "
                            f"{prev_node.name or prev_node.ctype} at 0x{prev[0]:X}:{prev[1]} (offsets decrease in {node.name or 'anonymous struct'})"
                        )
                    elif (prev is not None and fo == prev and ch.kind == "field" and prev_node.kind == "field"
                          and ch.bit is None and prev_node.bit is None
                          and not re.match(r"^(.*\.)?(filler|pad|_)", ch.name)
                          and not re.match(r"^(.*\.)?(filler|pad|_)", prev_node.name)):
                        notes.append(f"{cite(ch)}: {ch.name} shares offset 0x{fo[0]:X} with {prev_node.name} (header typo?)")
                    prev, prev_node = fo, ch
                walk(ch)
        elif node.kind == "union":
            for ch in node.children:
                walk(ch)
        else:
            if size is not None and node.offset is not None and node.offset >= size:
                violations.append(f"{cite(node)}: {node.name} at 0x{node.offset:X} >= size 0x{size:X}")

    walk(root)
    return violations, notes


# --------------------------------------------------------------------------- #
# Emission
# --------------------------------------------------------------------------- #

def unique_names(leaves: list[Node]) -> list[tuple[str, Node]]:
    seen: dict[str, int] = {}
    out = []
    for n in leaves:
        name = n.name
        if name in seen:
            seen[name] += 1
            name = f"{name}__dup{seen[name]}"
        else:
            seen[name] = 0
        out.append((name, n))
    return out


def emit_yaml(struct: str, size: int | None, hdr: Header, root: Node, out_path: Path) -> list[tuple[str, Node]]:
    leaves = [n for n in root.leaves() if n.offset is not None]
    named = unique_names(leaves)
    width = max((len(n) for n, _ in named), default=8)
    lines = [
        f"# GENERATED by harness/gen_schema.py from {hdr.rel} -- do not edit by hand.",
        f"# `struct {struct}` declared at {hdr.rel}:{root.line}; ASSERT_SIZE 0x{size:X}." if size is not None
        else f"# `struct {struct}` declared at {hdr.rel}:{root.line}; no ASSERT_SIZE found.",
        "# Fields are those carrying an offset comment. `unknown` keeps the C type so nothing is dropped;",
        "# bitfields carry bit/width using the header's own numbering (`fp+XXXX:N`).",
        f"struct: {struct}",
        f"size: 0x{size:X}" if size is not None else "size: null",
        f"source: {hdr.rel}:{root.line}",
        "fields:",
    ]
    for name, n in named:
        attrs = [f"offset: 0x{n.offset:04X}", f"type: {n.schema_type}"]
        if n.schema_type == "unknown" or n.schema_type not in SCHEMA_SCALARS:
            attrs.append(f'ctype: "{n.ctype}"')
        if n.bit is not None:
            attrs.append(f"bit: {n.bit}")
            attrs.append(f"width: {n.width}")
        lines.append(f"  {name + ':':<{width + 1}} {{ {', '.join(attrs)} }}  # {hdr.rel.split('/')[-1]}:{n.line}")
    out_path.write_text("\n".join(lines) + "\n")
    yaml.safe_load(out_path.read_text())  # must round-trip
    return named


def _field_json(name: str, n: Node) -> dict:
    d: dict = {"name": name, "offset": n.offset, "type": n.schema_type}
    if n.schema_type == "unknown" or n.schema_type not in SCHEMA_SCALARS:
        d["ctype"] = n.ctype
    if n.bit is not None:
        d["bit"] = n.bit
        d["width"] = n.width
    return d


def write_schema_json(struct: str, size: int | None, source: str | None,
                      fields: list[dict], out_path: Path) -> None:
    """Write the JSON twin of a schema: same struct/size/fields, fields as an ordered list."""
    doc = {"struct": struct, "size": size, "source": source, "fields": fields}
    out_path.write_text(json.dumps(doc, indent=2) + "\n")


def emit_json(struct: str, size: int | None, hdr: Header, root: Node,
              named: list[tuple[str, Node]], out_path: Path) -> None:
    write_schema_json(struct, size, f"{hdr.rel}:{root.line}",
                      [_field_json(name, n) for name, n in named], out_path)


def hand_to_json(hand_path: Path, out_path: Path) -> int:
    """Mirror a hand-written schema YAML (fields as a mapping) into the JSON list form."""
    hand = yaml.safe_load(hand_path.read_text())
    fields = []
    for name, f in hand["fields"].items():
        d = {"name": name, "offset": f["offset"], "type": f["type"]}
        for k in ("ctype", "bit", "width"):
            if k in f:
                d[k] = f[k]
        fields.append(d)
    write_schema_json(hand["struct"], hand.get("size"), str(hand_path.relative_to(ROOT)), fields, out_path)
    return len(fields)


def compare_with_hand(hand_path: Path, named: list[tuple[str, Node]]) -> list[str]:
    hand = yaml.safe_load(hand_path.read_text())
    by_off: dict[int, list[tuple[str, Node]]] = {}
    for name, n in named:
        by_off.setdefault(n.offset, []).append((name, n))
    by_name = {name: n for name, n in named}
    report: list[str] = []
    ok = 0
    for hname, hf in hand["fields"].items():
        hoff, htype = hf["offset"], hf["type"]
        cands = [(nm, n) for nm, n in by_off.get(hoff, []) if n.bit is None]
        if not cands:
            report.append(f"  {hname} @0x{hoff:X} ({htype}): no generated field at this offset")
            continue
        exact = [(nm, n) for nm, n in cands if nm == hname]
        nm, n = exact[0] if exact else cands[0]
        msgs = []
        if nm != hname:
            msgs.append(f"named {nm!r} in header")
        if n.schema_type != htype:
            msgs.append(f"type {htype} vs header {n.schema_type} ({n.ctype})")
        if msgs:
            report.append(f"  {hname} @0x{hoff:X} ({htype}): " + "; ".join(msgs))
        else:
            ok += 1
    if hand.get("size") is not None:
        report.append(f"  hand size 0x{hand['size']:X}")
    report.insert(0, f"  {ok}/{len(hand['fields'])} hand-written fields match exactly (name, offset, type)")
    return report


# --------------------------------------------------------------------------- #

def run(struct: str, header: Path, out_name: str, hand: Path | None, typedefs: TypedefTable) -> tuple[int, int]:
    hdr = Header(header, typedefs)
    root = hdr.scan(struct)
    print(f"== struct {struct} ({hdr.rel}) ==")
    if root is None:
        print(f"  not found in {hdr.rel}")
        return 0, 0
    size = hdr.assert_size(struct)
    named = emit_yaml(struct, size, hdr, root, SCHEMA_DIR / out_name)
    json_name = out_name.removesuffix(".yaml") + ".json"
    emit_json(struct, size, hdr, root, named, SCHEMA_DIR / json_name)
    kinds: dict[str, int] = {}
    for _, n in named:
        kinds[n.schema_type] = kinds.get(n.schema_type, 0) + 1
    unknown = [n for _, n in named if n.schema_type == "unknown"]
    bitfields = sum(1 for n in unknown if n.bit is not None)
    arrays = sum(1 for n in unknown if n.bit is None and "[" in n.ctype)
    aggregates = sum(1 for n in unknown if n.bit is None and "[" not in n.ctype and n.ctype.startswith(("struct", "union")))
    other = len(unknown) - bitfields - arrays - aggregates
    print(f"  declared at line {root.line}; ASSERT_SIZE = {f'0x{size:X}' if size is not None else 'n/a'}")
    print(f"  fields with offsets: {len(named)}")
    print(f"  by type: {', '.join(f'{k}={v}' for k, v in sorted(kinds.items()))}")
    print(f"  unknown breakdown: bitfields={bitfields} arrays={arrays} nested-aggregates={aggregates} opaque-typedefs={other}")
    if other:
        opaque = sorted({n.ctype for n in unknown if n.bit is None and '[' not in n.ctype and not n.ctype.startswith(('struct', 'union'))})
        print(f"  opaque C types: {', '.join(opaque)}")
    violations, notes = validate(root, size, hdr)
    for w in hdr.warnings:
        print(f"  warning: {w}")
    print(f"  layout violations: {len(violations)}")
    for v in violations:
        print(f"    {v}")
    for nt in notes:
        print(f"  note: {nt}")
    print(f"  wrote schema/{out_name} and schema/{json_name}")
    if hand is not None and hand.exists():
        print(f"  vs hand-written schema/{hand.name}:")
        for line in compare_with_hand(hand, named):
            print(line)
        hand_json = hand.name.removesuffix(".yaml") + ".hand.generated.json"
        n = hand_to_json(hand, SCHEMA_DIR / hand_json)
        print(f"  wrote schema/{hand_json} ({n} fields)")
    return len(named), len(violations)


def main(argv: list[str]) -> int:
    typedefs = TypedefTable.build([SRC, DECOMP / "extern"])
    total_viol = 0
    _, v = run("Fighter", SRC / "melee" / "ft" / "types.h", "fighter.generated.yaml",
               SCHEMA_DIR / "fighter.yaml", typedefs)
    total_viol += v
    print()
    _, v = run("Item", SRC / "melee" / "it" / "types.h", "item.generated.yaml", None, typedefs)
    total_viol += v
    if "--check" in argv and total_viol:
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
