# gekko_probe: empirical capture of Gekko `frsqrte` / `fres`

Runs the real `frsqrte` and `fres` instructions inside Dolphin (Felk's
scripting fork, see `docs/DOLPHIN_BUILD.md`) with no game disc, records
input/output bit pairs, and infers the table structure behind the estimates
from those pairs alone. Nothing here reads or copies emulator source; the
tables printed by `analyze.py` are fitted from captured data.

## Files

| File | Purpose |
|---|---|
| `layout.py` | Guest memory layout shared by the assembler and the host script. |
| `mkdol.py` | Hand-assembles the guest program (a few dozen PowerPC words) into a bootable `.dol`. `--hex` prints the words for cross-checking with `llvm-mc --disassemble -triple=powerpc`. |
| `probe.py` | Dolphin `--script`. Writes input sweeps into MEM1, starts the guest, polls for completion, dumps `*.jsonl`. |
| `run.sh` | Builds the `.dol`, launches Dolphin headless-ish (`-v Null`, no panic dialogs, file logging), waits, prints result paths. `run.sh 0` = interpreter (default), `run.sh 4` = ARM64 JIT. |
| `analyze.py` | Infers the estimate structure from the pairs and verifies the model reproduces every pair. `--table` prints the fitted entries. |

Outputs go to `harness/traces/` (gitignored): `frsqrte_probe.jsonl`,
`fres_probe.jsonl` (f32 in via `lfs`, f32 out via `stfs`),
`fres64_probe.jsonl` (f64 in/out via `lfd`/`stfd`), each line
`{"input_bits": "0x..", "output_bits": "0x.."}`.

## How it works

1. **Guest program** (`mkdol.py`): sets `MSR[FP]`, writes an ALIVE marker,
   spins until the host writes a START magic word, then runs three loops
   (`lfd/frsqrte/stfd`, `lfs/fres/stfs`, `lfd/fres/stfd`) over host-supplied
   arrays with host-supplied counts, writes a DONE magic word, and spins.
   Loaded at `0x80003100`; data regions at `0x80400000+`. Dolphin boots a
   raw `.dol` with `-e file.dol` (no disc needed) and the VI still generates
   fields, so `event.on_frameadvance` fires.
2. **Host script** (`probe.py`): on the first frame confirms ALIVE, writes
   the sweeps with `memory.write_u64/u32` (bit patterns, never Python
   floats, so NaN payloads survive), poisons the output regions, sets the
   counts and START. On later frames polls DONE, then reads the outputs and
   writes JSONL. `__file__` is undefined in Dolphin's embedded Python, so
   paths come from `GEKKO_PROBE_DIR` / `GEKKO_PROBE_OUT` (set by `run.sh`).
   It exits with `os._exit` since the scripting API has no stop call.
3. **Sweeps**: for each instruction, all 2048 patterns of the top 11
   mantissa bits at several exponents of each parity (low bits zero); 2048
   inputs with fixed top bits and random low bits (to find which bits
   matter); 4096 random normals over the full exponent range; specials
   (signed zero, inf, qNaN, sNaN, NaN payloads, negatives, denormals);
   and dense coverage of the exponents where `fres` leaves the single
   precision range. About 23k-26k pairs per instruction, one run, ~2 s.

## Findings (both Dolphin cores agree on every numeric pair)

`analyze.py` derives, and then verifies bit for bit against 23317 frsqrte,
25045 fres64 and 25557 fres32 pairs (100% reproduced):

**frsqrte** (double in, double out)
- Result mantissa depends on the exponent's parity and the top 15 mantissa
  bits only. Table of 32 entries indexed by `(exp & 1, top 4 mantissa bits)`,
  each `(base, slope)`; linear correction from the next 11 bits:
  `mant_out = (base - slope * next11) << 26` (26 significant mantissa bits).
- `exp_out = (0xBFC - exp_in) >> 1`.
- Denormal inputs are fully normalised first (implicit exponent goes
  negative) and then use the same table.
- `+0 -> +inf`, `-0 -> -inf`, `+inf -> +0`, `-inf` and any negative
  (including negative denormals) `-> 0x7FF8000000000000`.
- NaN: interpreter passes the NaN through unchanged (sNaN stays signalling);
  ARM64 JIT passes it through with the quiet bit set. That is the only
  interpreter/JIT difference observed.

**fres** (f64 in/out)
- Result mantissa depends on the top 15 mantissa bits only, independent of
  the exponent. Table of 32 entries indexed by the top 5 mantissa bits;
  linear correction from the next 10 bits with one extra bit of slope
  precision: `mant_out = ((base - slope * next10) >> 1) << 29`
  (23 significant bits, i.e. a single-precision mantissa). 14 entries have
  an even slope, so their base LSB is unobservable; either value gives the
  same outputs.
- `exp_out = 0x7FD - exp_in`, clamped to the single range: if `exp_in < 0x37F`
  the result is `+/-0x47EFFFFFE0000000` (FLT_MAX as a double, sign kept);
  if `exp_in > 0x47C` (result would be a single denormal) it is `+/-0`.
  Denormal inputs fall under the first rule. Sign is preserved for all
  finite inputs.
- `+/-0 -> +/-inf`, `+/-inf -> +/-0`; NaN as for frsqrte.

**fres via f32** (`lfs` / `fres` / `stfs`) equals: widen to f64 (single
denormals normalise to a normal double), apply the f64 model, narrow to f32.
The narrowing is exact for every captured pair (the result already has a
23-bit mantissa and lies in single range), so `fres` on a single is
`1/x` with the table above and an 8-bit exponent `0xFD - exp_in`.

## Reproducing

```sh
harness/gekko_probe/run.sh          # interpreter -> harness/traces/*.jsonl
GEKKO_PROBE_OUT=harness/traces/jit harness/gekko_probe/run.sh 4   # ARM64 JIT
python3 harness/gekko_probe/analyze.py [harness/traces] [--table]
```

`run.sh` writes a private Dolphin user dir under the output dir
(`gekko_probe_user/`; its `Logs/dolphin.log` has the `Script stdout:` lines
and any boot errors) and `gekko_probe.status`, a plain-text progress log.

## Licensing note

The fitted tables are derived from observed behaviour, but whether they may
be committed under `crates/` is the licensing decision tracked in
`TRACKER.md`. Nothing in this directory is imported by any crate.
