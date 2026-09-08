# Standing instructions for agents working in this repo

This is a Rust port of Melee verified against the retail game running in
Dolphin. Correctness means **bit-exact** agreement with the oracle. Read
`docs/ORACLE.md` and `docs/PLAN.md` before touching code.

## Sources of truth, in order

1. The retail assembly. It is the only record of where the compiler fused
   multiply-adds and how it ordered float operations. Available via the
   decomp submodule's tooling (`objdiff`, `dtk`), or the `.s` files it can
   emit.
2. The decomp C in `third_party/melee-decomp/src/`. Readable intent, but it
   does **not** show FMA contraction. Never port float math from the C alone.
3. The symbol map `third_party/melee-decomp/config/GALE01/symbols.txt` and the
   struct headers with their offset comments and `ASSERT_SIZE` checks.

The submodule is read-only reference. Never modify it from this repo.

## Exactness rules

- All float arithmetic goes through `gekko-math`. No `std` or `libm` math
  functions in `hsd-*`, `melee-*`, or `ft-*` crates. `sqrt`, `sin`, `cos`,
  `atan2`, `powf`, `floor` and friends are all banned outside `gekko-math`.
- Where the retail asm shows `fmadds`/`fmsubs`/`fnmsubs`, use the matching
  `gekko_math::fma::*` function with operands in PowerPC order (a, c, b).
  Where it shows separate `fmuls`/`fadds`, write separate `*` and `+`.
- Preserve double promotion. `10.0 * HSD_Randf()` computes in `f64` and
  rounds once; write it as `(10.0f64 * rng.randf() as f64) as f32`.
- Transcribe MSL routines literally. Do not simplify or reassociate.
- Endianness, struct size, field offset, and pointer width may only appear in
  `hsd-archive`. Everything above it works on owned Rust types.
- Internal state does **not** have to match retail memory layout. Use enums
  for the per-character union, real types for item state. The `Snapshot`
  trait handles comparison against the schema.

## Build-speed rules

The workspace is layered so an edit rebuilds as little as possible.
`cargo build` after touching one fighter crate should take seconds.

- Layer 0 leaf crates (`gekko-math`, `hsd-types`, `melee-types`) hold types
  and pure functions only. Change them rarely and deliberately; everything
  depends on them.
- One crate per subsystem (`melee-lb`, `melee-mp`, `melee-gr`, `melee-it`,
  `melee-ft`, `melee-cpu`). No crate may exceed roughly 30k lines; split it
  along the decomp's directory structure when it does.
- One crate per character under `crates/ft-<name>`. Character crates depend
  on `melee-ft` and `melee-types`, never on each other. Copy `ft-fox` as the
  template.
- Cross-layer `pub use` facades are banned. Depend on the crate you use.
- Only `melee-sim` and `melee-platform` may depend on everything.
- Do not add proc-macro or heavy dependencies to layer 0 or 1.

## Workflow for porting a function

1. Pick the next function from the dependency order (`dep_graph.py` in the
   submodule). Prefer leaves: functions whose callees are already ported.
2. Read its C, its retail asm, and its callers.
3. Write the Rust in the crate matching its decomp directory. Cite the decomp
   path and retail address in a doc comment: `/// ftCo_800B63D8 (ftcpuattack.c)`.
4. If golden fixtures exist for it under `harness/goldens/`, add a test that
   replays them.
5. `cargo gate` must pass. If a scenario diverges, `melee-diff` prints the
   first frame, phase, and field. Fix it before moving on; never commit a
   known divergence.
6. Commit one function or one tight group per commit, with the retail
   address in the message.

## Things that look like bugs but are not

- `sqrtf` returns negative inputs unchanged. MSL does that.
- `HSD_Randi` multiplies before dividing in 32-bit signed arithmetic.
- Fighter `facing_dir` is a float that is always `1.0` or `-1.0`.
- Many decomp functions have address names (`ftCo_800A101C`). Keep them in
  doc comments so the retail asm can be found; give the Rust function a
  descriptive name.
