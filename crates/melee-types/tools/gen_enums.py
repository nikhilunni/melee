#!/usr/bin/env python3
"""Extract C enum values from the melee decomp headers and emit Rust modules.

Every value is computed the way a C compiler would (previous + 1, or the
explicit initializer, which may name an earlier enumerator), then checked
against the hex annotations the header authors left in comments.
"""
import re
import sys
from pathlib import Path

# This script lives at crates/melee-types/tools/gen_enums.py; resolve the
# decomp submodule and the output directory relative to it so it works from
# any checkout location.
REPO = Path(__file__).resolve().parents[3]
DECOMP = REPO / "third_party" / "melee-decomp"
OUT = REPO / "crates" / "melee-types" / "src"

ENUM_RE = re.compile(r"^\s*([A-Za-z_][A-Za-z0-9_]*)\s*(?:=\s*([^,/]+?))?\s*,?\s*(?://.*|///<.*)?$")


def parse_enum(path: str, start: int, end: int):
    """Return [(c_name, value, hex_comment_or_None, trailing_comment)] for lines start..end (1-based, inclusive)."""
    lines = Path(DECOMP, path).read_text().splitlines()[start - 1:end]
    out = []
    names = {}
    prev = -1
    pending = None  # for initializers split across lines
    for raw in lines:
        line = raw.strip()
        if not line or line.startswith("typedef") or line.startswith("}") or line.startswith("#") or line.startswith("//") or line.startswith("///"):
            continue
        hex_comment = None
        m = re.match(r"^/\*\s*(0x)?([0-9A-Fa-f]+)\s*\*/\s*(.*)$", line)
        if m:
            hex_comment = int(m.group(2), 16)
            line = m.group(3)
        if pending is not None:
            line = pending + " " + line
            pending = None
        # Strip trailing comments.
        code, _, comment = line.partition("//")
        code = code.strip()
        comment = comment.lstrip("/<").strip()
        if not code:
            continue
        if code.endswith("=") or (code.count("=") == 1 and code.split("=")[1].strip() == ""):
            pending = code
            continue
        code = code.rstrip(",").strip()
        if "=" in code:
            name, expr = [s.strip() for s in code.split("=", 1)]
            expr = expr.rstrip(",").strip()
            if re.fullmatch(r"-?0x[0-9A-Fa-f]+|-?\d+", expr):
                value = int(expr, 0)
            elif expr in names:
                value = names[expr]
            else:
                raise SystemExit(f"{path}: cannot evaluate {name} = {expr!r}")
        else:
            name = code
            value = prev + 1
        assert re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", name), (path, name)
        if hex_comment is not None and hex_comment != value:
            # The decomp's own comment disagrees with the computed value; report it, do not trust it.
            print(f"NOTE {path}: {name} computed {value} (0x{value:X}) but comment says 0x{hex_comment:X}", file=sys.stderr)
        names[name] = value
        prev = value
        out.append((name, value, hex_comment, comment))
    return out


def pascal(s: str) -> str:
    return "".join(p[:1].upper() + p[1:] for p in s.split("_") if p)


def emit_enum(rust_name, repr_ty, doc, header, variants, consts, fmt_hex=True, extra=""):
    """variants: [(RustName, value, c_name, comment)], consts: [(CONST_NAME, value, c_name, comment)]."""
    def lit(v):
        if fmt_hex and v >= 0:
            return f"0x{v:02X}"
        return str(v)

    lines = [
        f"//! Generated from `{header}` by",
        "//! `crates/melee-types/tools/gen_enums.py`. Re-run the script and diff",
        "//! rather than editing by hand.",
        "",
    ]
    # `c_enum!` is a textually-scoped macro_rules! defined in lib.rs before
    # these modules are declared, so no `use` is needed (clippy flags one).
    lines.append("c_enum! {")
    for d in doc:
        lines.append(f"    /// {d}".rstrip())
    lines.append(f"    ///")
    lines.append(f"    /// Source: `{header}`.")
    lines.append(f"    pub enum {rust_name}: {repr_ty} {{")
    for rn, v, cn, comment in variants:
        text = f"`{cn}`"
        if comment:
            text += f" — {comment}"
        lines.append(f"        /// {text}")
        lines.append(f"        {rn} = {lit(v)},")
    lines.append("    }")
    lines.append("}")
    if consts:
        lines.append("")
        lines.append(f"impl {rust_name} {{")
        for i, (cn_rust, v, cn, comment) in enumerate(consts):
            if i:
                lines.append("")
            text = f"`{cn}`"
            if comment:
                text += f" — {comment}"
            lines.append(f"    /// {text}")
            lines.append(f"    pub const {cn_rust}: {repr_ty} = {lit(v)};")
        lines.append("}")
    if extra:
        lines.append("")
        lines.append(extra.rstrip())
    return "\n".join(lines) + "\n"


# ---------------------------------------------------------------------------
# FighterKind
# ---------------------------------------------------------------------------
FK_HDR = "src/melee/ft/forward.h"
fk = parse_enum(FK_HDR, 89, 125)
FK_NAMES = {
    "FTKIND_MARIO": ("Mario", "Mario"),
    "FTKIND_FOX": ("Fox", "Fox"),
    "FTKIND_CAPTAIN": ("Captain", "Captain Falcon"),
    "FTKIND_DONKEY": ("Donkey", "Donkey Kong"),
    "FTKIND_KIRBY": ("Kirby", "Kirby"),
    "FTKIND_KOOPA": ("Koopa", "Bowser"),
    "FTKIND_LINK": ("Link", "Link"),
    "FTKIND_SEAK": ("Seak", "Sheik"),
    "FTKIND_NESS": ("Ness", "Ness"),
    "FTKIND_PEACH": ("Peach", "Peach"),
    "FTKIND_POPO": ("Popo", "Ice Climbers leader (Popo)"),
    "FTKIND_NANA": ("Nana", "Ice Climbers partner (Nana); internal id, not selectable"),
    "FTKIND_PIKACHU": ("Pikachu", "Pikachu"),
    "FTKIND_SAMUS": ("Samus", "Samus"),
    "FTKIND_YOSHI": ("Yoshi", "Yoshi"),
    "FTKIND_PURIN": ("Purin", "Jigglypuff"),
    "FTKIND_MEWTWO": ("Mewtwo", "Mewtwo"),
    "FTKIND_LUIGI": ("Luigi", "Luigi"),
    "FTKIND_MARS": ("Mars", "Marth"),
    "FTKIND_ZELDA": ("Zelda", "Zelda"),
    "FTKIND_CLINK": ("CLink", "Young Link"),
    "FTKIND_DRMARIO": ("DrMario", "Dr. Mario"),
    "FTKIND_FALCO": ("Falco", "Falco"),
    "FTKIND_PICHU": ("Pichu", "Pichu"),
    "FTKIND_GAMEWATCH": ("GameWatch", "Mr. Game & Watch"),
    "FTKIND_GANON": ("Ganon", "Ganondorf"),
    "FTKIND_EMBLEM": ("Emblem", "Roy"),
    "FTKIND_MASTERH": ("MasterH", "Master Hand; internal id"),
    "FTKIND_CREZYH": ("CrezyH", "Crazy Hand; internal id (header spelling)"),
    "FTKIND_BOY": ("Boy", "Male Wireframe; internal id"),
    "FTKIND_GIRL": ("Girl", "Female Wireframe; internal id"),
    "FTKIND_GKOOPS": ("GKoops", "Giga Bowser; internal id"),
    "FTKIND_SANDBAG": ("Sandbag", "Sandbag; internal id"),
    "FTKIND_NONE": ("None", "No fighter"),
}
fk_variants, fk_consts = [], []
for cn, v, _, comment in fk:
    if cn == "FTKIND_MAX":
        fk_consts.append(("MAX", v, cn, "alias of `FTKIND_NONE`; one past the last real fighter kind"))
        continue
    rn, desc = FK_NAMES[cn]
    fk_variants.append((rn, v, cn, desc))
assert len(fk_variants) == 34, len(fk_variants)
assert dict((c, v) for c, v, *_ in fk)["FTKIND_FOX"] == 1

# ---------------------------------------------------------------------------
# GroundOrAir
# ---------------------------------------------------------------------------
ga = parse_enum(FK_HDR, 442, 445)
assert [(n, v) for n, v, *_ in ga] == [("GA_Ground", 0), ("GA_Air", 1)], ga

# ---------------------------------------------------------------------------
# CommonMotionState
# ---------------------------------------------------------------------------
MS_HDR = "src/melee/ft/kinds/ftCommon/forward.h"
ms = parse_enum(MS_HDR, 287, 631)
ms_variants, ms_consts = [], []
for cn, v, _, comment in ms:
    short = cn[len("ftCo_MS_"):]
    if short == "Count":
        ms_consts.append(("COUNT", v, cn, "number of common motion states; one past `ftCo_MS_Barrel`"))
    else:
        ms_variants.append((short, v, cn, comment))
assert ms_variants[0] == ("None", -1, "ftCo_MS_None", ""), ms_variants[0]
assert ms_variants[1][1] == 0
ms_count = ms_consts[0][1]

# ---------------------------------------------------------------------------
# ItemKind
# ---------------------------------------------------------------------------
IT_HDR = "src/melee/it/forward.h"
it = parse_enum(IT_HDR, 109, 392)
it_by_name = {n: v for n, v, *_ in it}
# Anchors written as comments in the header.
assert it_by_name["It_Kind_M_Ball"] == 0x22
assert it_by_name["It_PKind_Random"] == 0xA0, hex(it_by_name["It_PKind_Random"])
assert it_by_name["It_PKind_Start"] == 0xA1
assert it_by_name["It_PKind_Tosakinto"] == 0xA1
assert it_by_name["It_PKind_Terminate"] == 0xBF
assert it_by_name["It_Kind_Chicorita_Leaf"] == 0xBF
assert it_by_name["It_Kind_Pokemon_Unk"] == 0xCF
assert it_by_name["It_Kind_None"] == -999
it_variants, it_consts = [], []
for cn, v, _, comment in it:
    if cn == "It_PKind_Start":
        it_consts.append(("POKEMON_START", v, cn, "first Poke Ball Pokemon; alias of `It_PKind_Tosakinto`"))
        continue
    if cn == "It_PKind_Terminate":
        it_consts.append(("POKEMON_TERMINATE", v, cn, "one past the last Poke Ball Pokemon; alias of `It_Kind_Chicorita_Leaf`"))
        continue
    if cn.startswith("It_PKind_"):
        short = cn[len("It_PKind_"):]
        rn = "PokemonRandom" if short == "Random" else pascal(short)
    else:
        rn = pascal(cn[len("It_Kind_"):])
    it_variants.append((rn, v, cn, comment))
rust_names = [r for r, *_ in it_variants]
dupes = {r for r in rust_names if rust_names.count(r) > 1}
assert not dupes, dupes

# ---------------------------------------------------------------------------
# GrKind
# ---------------------------------------------------------------------------
GR_HDR = "src/melee/gr/forward.h"
gr = parse_enum(GR_HDR, 54, 127)
gr_variants, gr_consts = [], []
for cn, v, _, comment in gr:
    short = cn[len("Gr_Kind_"):]
    if short == "Count":
        gr_consts.append(("COUNT", v, cn, "explicit sentinel in the header (221), not one past `Gr_Kind_Figure3`"))
    else:
        gr_variants.append((short, v, cn, comment))
assert gr_variants[-1] == ("Figure3", 0x46, "Gr_Kind_Figure3", "")
assert gr_consts[0][1] == 221

# ---------------------------------------------------------------------------
# Gm_PKind
# ---------------------------------------------------------------------------
PL_HDR = "src/melee/pl/forward.h"
pl = parse_enum(PL_HDR, 11, 17)
assert [(n, v) for n, v, *_ in pl] == [
    ("Gm_PKind_Human", 0), ("Gm_PKind_Cpu", 1), ("Gm_PKind_Demo", 2), ("Gm_PKind_NA", 3), ("Gm_PKind_Boss", 4)], pl

# ---------------------------------------------------------------------------
# CpuCmd
# ---------------------------------------------------------------------------
CPU_HDR = "src/melee/ft/ftcmdscript.h"
cpu = parse_enum(CPU_HDR, 6, 77)
cpu_by_name = {n: v for n, v, *_ in cpu}
assert cpu_by_name["CpuCmd_PressA"] == 1
assert cpu_by_name["CpuCmd_ReleaseAll"] == 25
assert cpu_by_name["CpuCmd_Done"] == 0x7F
assert cpu_by_name["CpuCmd_ZeroArgEnd"] == 0x7F
assert cpu_by_name["CpuCmd_SetLstickX"] == 0x80
assert cpu_by_name["CpuCmd_Unk0x93"] == 0x93, hex(cpu_by_name["CpuCmd_Unk0x93"])
assert cpu_by_name["CpuCmd_OneArgEnd"] == 0xBF
assert cpu_by_name["CpuCmd_Count"] <= 0xFF
cpu_variants, cpu_consts = [], []
for cn, v, _, comment in cpu:
    short = cn[len("CpuCmd_"):]
    if short == "ZeroArgEnd":
        cpu_consts.append(("ZERO_ARG_END", v, cn, "alias of `CpuCmd_Done`; commands `<= ZERO_ARG_END` take zero arguments"))
    elif short == "Count":
        cpu_consts.append(("COUNT", v, cn, "one past the last command; the header asserts this fits in a `u8`"))
    else:
        cpu_variants.append((short, v, cn, comment))
        if short == "OneArgEnd":
            # Also exposed as a const so range checks can pair it with ZERO_ARG_END.
            cpu_consts.append(("ONE_ARG_END", v, cn, "same value as the `OneArgEnd` variant; commands in `(ZERO_ARG_END, ONE_ARG_END]` take one argument"))

# ---------------------------------------------------------------------------
# Emit
# ---------------------------------------------------------------------------
files = {
    "fighter_kind.rs": emit_enum(
        "FighterKind", "i32",
        ["Internal fighter kind (`FighterKind`, the `FTKIND_*` ids).",
         "",
         "This is the *internal* numbering used by `Fighter::kind`, not the",
         "character-select `CharacterKind`. It includes ids that are never",
         "selectable: Nana, Master Hand, Crazy Hand, the Wireframes, Giga",
         "Bowser and Sandbag."],
        FK_HDR, fk_variants, fk_consts),
    "ground_or_air.rs": emit_enum(
        "GroundOrAir", "i32",
        ["Whether a fighter or item is standing on ground or airborne (`GroundOrAir`)."],
        FK_HDR,
        [("Ground", 0, "GA_Ground", ""), ("Air", 1, "GA_Air", "")], [], fmt_hex=False),
    "motion_state.rs": emit_enum(
        "CommonMotionState", "i32",
        ["Motion state ids shared by every fighter (`ftCommon_MotionState`, the `ftCo_MS_*` ids).",
         "",
         "Character-specific states start at [`CommonMotionState::COUNT`]."],
        MS_HDR, ms_variants, ms_consts, fmt_hex=False),
    "item_kind.rs": emit_enum(
        "ItemKind", "i32",
        ["Item kind (`ItemKind`, the `It_Kind_*` and `It_PKind_*` ids).",
         "",
         "Poke Ball Pokemon occupy `[POKEMON_START, POKEMON_TERMINATE)`."],
        IT_HDR, it_variants, it_consts),
    "gr_kind.rs": emit_enum(
        "GrKind", "i32",
        ["Internal stage kind (`GrKind`, the `Gr_Kind_*` ids).",
         "",
         "This is the internal ground numbering, not the stage-select `StKind`."],
        GR_HDR, gr_variants, gr_consts),
    "player_kind.rs": emit_enum(
        "PlayerKind", "i32",
        ["Who controls a player slot (`Gm_PKind`)."],
        PL_HDR,
        [("Human", 0, "Gm_PKind_Human", ""), ("Cpu", 1, "Gm_PKind_Cpu", ""), ("Demo", 2, "Gm_PKind_Demo", ""),
         ("Na", 3, "Gm_PKind_NA", "slot not in use"), ("Boss", 4, "Gm_PKind_Boss", "")], [], fmt_hex=False),
    "cpu_cmd.rs": emit_enum(
        "CpuCmd", "i32",
        ["CPU command-script opcodes (`CPUCommand`, the `CpuCmd_*` ids).",
         "",
         "Opcodes `<= ZERO_ARG_END` take no argument, `(ZERO_ARG_END, ONE_ARG_END]`",
         "take one, and the rest take more. Scripts store opcodes as bytes;",
         "see the `u8` conversions below."],
        CPU_HDR, cpu_variants, cpu_consts,
        extra="""impl From<CpuCmd> for u8 {
    fn from(v: CpuCmd) -> u8 {
        // Every opcode is < COUNT <= 0xFF (STATIC_ASSERT in the header).
        v as i32 as u8
    }
}

impl TryFrom<u8> for CpuCmd {
    type Error = crate::InvalidDiscriminant;

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        CpuCmd::try_from(i32::from(v))
    }
}
"""),
}
for name, text in files.items():
    Path(OUT, name).write_text(text)
    print(f"wrote {OUT / name}")

print()
print(f"FighterKind: {len(fk_variants)} variants + {len(fk_consts)} consts; MAX = {fk_consts[0][1]}")
print(f"GroundOrAir: 2 variants")
print(f"CommonMotionState: {len(ms_variants)} variants (incl. None=-1) + COUNT = {ms_count}; last real = {ms_variants[-1]}")
print(f"ItemKind: {len(it_variants)} variants + {len(it_consts)} consts; Coin = 0x{it_by_name['It_Kind_Coin']:X}, Kyasarin_Egg = 0x{it_by_name['It_Kind_Kyasarin_Egg']:X}")
print(f"GrKind: {len(gr_variants)} variants + COUNT = {gr_consts[0][1]}")
print(f"PlayerKind: 5 variants")
print(f"CpuCmd: {len(cpu_variants)} variants + {len(cpu_consts)} consts; COUNT = 0x{cpu_by_name['CpuCmd_Count']:X} ({cpu_by_name['CpuCmd_Count']})")
