# State schema

One schema describes what both sides of the oracle comparison emit.

- `harness/decode.py` reads raw memory dumps from Dolphin and emits canonical
  JSONL using these files.
- The Rust side implements `Snapshot` per type and must emit the **same paths
  and value types**. A test in `melee-sim` will load these files and assert
  the Rust emitter covers every path.

Offsets are copied from the decomp headers, which carry them as comments
(`/* fp+B0 */ Vec3 cur_pos;`). When you add a field, cite the header line.
Symbol names are resolved to addresses at runtime from the decomp's
`config/GALE01/symbols.txt` by `harness/symbols.py`; never hardcode a global's
address in a schema.

Types: `u8 u16 u32 s8 s16 s32 f32 f64 ptr vec3`. All big-endian.

## Bones phase

`harness/jobjdump.py` and `melee-sim bones` emit phase `bones`, with one
singleton-state Record per scalar. For each preorder bone: `p0.bone[i].mtx[j]`
(12 row-major words), `.rotate[j]` (4), `.scale[j]` (3), `.translate[j]` (3).
All values use the canonical `{"t":"f32","v":{"bits":u32,"approx":number}}`
encoding. Frame numbers are capture ordinals; AObj animation times, runtime
pointers, and flags live in a separate oracle metadata file. See
[M2_GATE.md](../../docs/M2_GATE.md) for offsets, the identity-root assumption,
capture commands, and the requirement to inspect cached-matrix dirty flags.
