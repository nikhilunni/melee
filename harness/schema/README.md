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
