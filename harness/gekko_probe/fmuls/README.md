# Register-width fmuls probe

Run `bash harness/gekko_probe/fmuls/run.sh 0` from the repository root.
Use argument `4` for the ARM64 JIT. Generated DOL and output stay under
`/tmp/melee-fmuls-probe`; the program contains original instructions only.
The script currently uses the documented local Dolphin installation path.

The guest resets FPSCR to zero, loads double-width operands, executes direct
and swapped fmuls, then frsqrte followed by estimate-square and estimate-half.
Cases cover multiplier rounding boundaries, signs, carries, subnormals,
underflow/overflow, and the shield-scale squared input 0x426e26ee.

The 264 interpreter records captured on 2026-09-12 supply
`crates/gekko-math/tests/data/fmuls_probe.txt`. Columns are hexadecimal
A, C, result, swapped result, estimate, estimate square, estimate half.
These are guest outputs, not production-helper outputs. The ARM64 JIT agrees
on all fields except direct results in rows 216..233 (zero-based): it flushes
those artificially scaled subnormal multipliers to signed zero. All actual
estimate products agree. This establishes the concrete PSVEC consumers;
it does not establish hardware exception flags or NaN payload propagation.

The native-C matrix reference independently quantizes with frexp/ldexp and
half-away rounding instead of sharing Rust's integer representation logic.
