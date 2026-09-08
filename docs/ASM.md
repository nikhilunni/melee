# Retail assembly lookup

The retail assembly is the first source of truth (`CLAUDE.md`): it is the
only record of which multiply-adds MWCC fused and how it ordered float
operations. This document says how to get per-function disassembly of the
retail `main.dol` and how to read it for a port.

## What is needed

- The decomp submodule checked out (`git submodule update --init`).
- The retail executable at
  `third_party/melee-decomp/orig/GALE01/sys/main.dol` (gitignored in the
  submodule; copy it from `harness/roms/sys/main.dol`, which must match
  `third_party/melee-decomp/config/GALE01/build.sha1`).
- `python3` and `ninja` on `PATH` (`brew install ninja`). No compiler, no
  wine: the split step never compiles anything.

## Producing the split (once, about five seconds)

```sh
cd third_party/melee-decomp
python3 configure.py                    # writes build.ninja; 0.3 s
ninja build/GALE01/config.json          # downloads dtk, splits the DOL; 4.5 s
```

`configure.py` needs no arguments and no compiler; it only writes
`build.ninja` and `objdiff.json`, both gitignored. The single ninja target
`build/GALE01/config.json` runs two rules:

1. `tools/download_tool.py dtk` fetches decomp-toolkit `v1.8.3` (the version
   pinned in `configure.py`) into `build/tools/dtk`.
2. `dtk dol split config/GALE01/config.yml build/GALE01` analyses the DOL
   (19,827 functions) and writes 1130 objects.

Measured 2026-09-08 on an M-series Mac: 0.26 s for configure, 4.5 s for the
download and split together, of which the split itself is 2.2 s. Do **not**
run a bare `ninja`; that is the full matching build and needs the Metrowerks
compiler under wine.

Output, all gitignored (`build/` is in the submodule's `.gitignore`, so
`git -C third_party/melee-decomp status` stays clean):

| Path | What |
|---|---|
| `build/GALE01/asm/<unit>.s` | One file per translation unit, e.g. `MSL/trigf.s`, `melee/lb/lbtrigf.s`. 59 MB total. |
| `build/GALE01/config.json` | The unit list dtk produced (name, object path, sizes). |
| `build/GALE01/include/`, `obj/`, `ldscript.lcf` | Inputs for the matching build; not needed here. |
| `build/tools/dtk` | The downloaded toolkit binary. |

If `dtk` cannot be downloaded, `cargo install decomp-toolkit` and run the
`dtk dol split` command above by hand, or pass `--dtk <path>` to
`configure.py`.

## The `.s` format

```
.fn sinf, global
/* 803263D4 00323188  7C 08 02 A6 */	mflr r0
...
.L_80326424:
/* 80326424 003231D4  C0 02 E7 D4 */	lfs f0, "@39"@sda21(r0)
...
.endfn sinf
```

Each instruction line carries the virtual address, the DOL file offset, the
four instruction bytes, and the text. Local branch targets are `.L_<addr>`.
Small-data constants appear as `"@39"@sda21(r0)`; their values are in the
same file's `.sdata2` section at the bottom (`.obj "@39"` ... `.float 0.5`).
Table symbols and callees are named from `config/GALE01/symbols.txt`.

## `harness/asm.py`

```sh
cd harness
uv run python asm.py sinf                # whole function
uv run python asm.py 0x80326470          # any address inside a function
uv run python asm.py sinf --fused        # only fmadd-family instructions
uv run python asm.py atan2f --calls      # bl targets in program order
uv run python asm.py asinf lb_sqrtf      # several at once, with headers
```

The tool resolves a symbol through `symbols.py` (or an address through the
`type:function size:` entries in `symbols.txt`), finds the owning unit by
the `.text` ranges in `config/GALE01/splits.txt`, and parses
`build/GALE01/asm/<unit>.s`. Output is `address  bytes  text`:

```
80326470  EC0201BA  fmadds f0, f2, f6, f0
80326474  EC0301BA  fmadds f0, f3, f6, f0
```

`--fused` lists `fmadd`, `fmsub`, `fnmadd`, `fnmsub` and their `s` forms.
If the split has not been run it says so and exits 1.

Tests: `uv run python -m pytest -q tests/test_asm.py`. They use an inline
`.s` fixture and need neither the disc nor the split (the two `resolve`
assertions skip when the submodule is absent).

## Reading the asm for a port

- PowerPC fused forms are `frD = frA * frC (+|-) frB`, so
  `fmadds f0, f2, f6, f0` is `f2 * f6 + f0`. `gekko_math::fma::*` takes
  operands in that order: `fmadds(f2, f6, f0)`. Note which register holds
  the accumulator; MWCC alternates between `coef * x + acc` and
  `x * acc + coef` within one polynomial.
- `fnmadds` is `-(a*c + b)`: MWCC uses it to fold a leading unary minus into
  the last Horner step (`cosf`, `0x80326368`).
- Single vs double: `fmadds`/`fmuls`/`lfs` are single, `fmadd`/`fmul`/`lfd`
  double. MSL's inline `sqrtf` iterates in double (`fnmsub`); Melee's
  `lb_sqrtf` in single (`fnmsubs`).
- Int to float is the `0x4330` idiom: `xoris r0, r0, 0x8000`, two `stw`, an
  `lfd`, then `fsubs`/`fsub` against the magic double `4503601774854144`.
  Watch what happens next: in `sinf` the resulting double feeds `fsubs`
  directly (never rounded to single on its own), which differs from C
  semantics for large arguments.
- Inline functions (`sqrtf`, `fmodf` from MSL's headers) have no symbol of
  their own except where the compiler emitted an out-of-line copy
  (`sqrtf__Ff`, `0x8000D5BC`, weak). Look at a caller: `grep -rn 'bl
  __cvt_sll_flt' build/GALE01/asm` finds every inlined `fmodf`.
- Record the finding at the site as `// retail 0x........: fmadds` or
  `// retail 0x........: fmuls + fadds, not fused`, and cite the address
  range in the function's doc comment.

## Fusion audit results, 2026-09-08

`gekko-math` and `melee-lb` were audited with this tool; every
`FUSION AUDIT PENDING` marker in those crates is resolved. MWCC fused every
`a * b + c` shape in `sinf`, `cosf`, `logf`, `sqrtf`, `fmodf`, `atanf`,
`asinf`, `acosf` and `lb_sqrtf`; `expf` and `powf` have no fusable shape and
none appears. The native-C oracle tests now compile retail-faithful copies
of the C (`crates/*/tests/ref/*/retail/`) and report, per function, how
many sweep inputs change result between the fused and unfused builds
(`cargo test -p gekko-math -p melee-lb -- --nocapture`):

| Function | Fused instructions (retail) | Sweep inputs whose result changed |
|---|---|---|
| `sqrtf` (MSL inline; `sqrtf__Ff` 0x8000D5BC) | 3 x `fnmsub` (double) | 0 of 102,083 |
| `sinf` 0x803263D4 | 4 `fmadds` reduction, 1 `fmadds` small branch, 4 + 4 `fmadds` polynomials | 18,624 of 102,083 |
| `cosf` 0x80326240 | 4 `fmadds` reduction, 1 `fnmsubs` small branch, 4 `fmadds` + 3 `fmadds` and 1 `fnmadds` polynomials | 17,798 of 102,083 |
| `tanf` 0x803261BC | via `sinf`/`cosf` | 23,982 of 102,083 |
| `logf` 0x803265A8 | 3 `fmadds` | 16,056 of 102,083 |
| `fmodf` (inline; aobj.c 0x80364380) | 1 `fnmsubs` | 33,410 of 60,016 |
| `atanf` 0x80022E68 | 6 `fmadds` (+2 explicit `fnmsubs` already in the C) | 176 of 103,997 |
| `asinf` 0x80022DBC | 1 `fnmsubs` (+ `lb_sqrtf`) | 3,137 of 103,997 |
| `acosf` 0x80022D1C | 4 `fnmsubs` | 1,785 of 103,997 |
| `lb_sqrtf` 0x80022DF8 | 3 `fnmsubs` | 3,568 of 103,997 |
| `atan2f` 0x80022C30 | none of its own (via `atanf`) | 87 of 111,729 |
| `expf` 0x8000CE50 | none | 0 of 100,326 |
| `powf` 0x8000CEE0 | none | 0 of 105,126 |

The `sinf`/`cosf`/`tanf` counts include the int-to-float finding above
(`fmodf`'s count also includes `__cvt_sll_flt`'s round-through-double). The
`sqrtf` count is 0 because the Newton steps run in double and the final
`frsp` hides a last-bit difference in the intermediate; the fused form is
still what the disc executes.

## hsd-anim

The 31 original marker lines resolve to 20 fused arithmetic groups, six
unfused scalar-length groups, and five overview notes. Determinants start with
the second triple product; quaternion xyz products keep separate final adds;
Hermite fuses only its three weighted accumulations. Existing paired-single
lane transcriptions remain correct. Retail/verbatim C builds differ on 29,028
of 129,706 matrix/quaternion inputs, 18,070 of 60,000 Hermite inputs, and 1,998
of 4,011 FObj streams. See the [complete site table and oracle limitations](../crates/hsd-anim/tests/ref/FUSION_AUDIT.md).
