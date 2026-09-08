# Milestone 2 bone comparison

This gate compares the cached retail Fox JObj world matrices and SRT against
the neutral-costume skeleton evaluating unfiltered Wait1 in `hsd-anim`.
It is ready to capture once a playable `harness/roms/idle_<stage>_fox.sav`
exists. A matching oracle trace has **not** been established by the tooling
tests: the real-disc tests check loading, frame advancement, and trace format.

## Contract and root assumption

Each JSONL line is a `melee_diff::Record`, with one scalar in `state`:

```json
{"frame":0,"phase":"bones","state":{"p0.bone[17].mtx[5]":{"t":"f32","v":{"bits":1065353216,"approx":1.0}}}}
```

`frame` is the integer **capture ordinal**, starting at zero even when
`--frame` is fractional or nonzero. Animation time is a separate f32.
Within each frame, visit bones in numeric preorder (root, children, siblings).
Within each bone emit `mtx[0..11]`, `rotate[0..3]`, `scale[0..2]`, then
`translate[0..2]`. Matrix index `4 * row + column` is row-major. Rotation is
the four stored quaternion words x/y/z/w, even in Euler mode where w is
unused. There are **73 × (12 + 4 + 3 + 3) = 1,606 records per frame**.
`p0` identifies the selected fighter, not necessarily controller port zero.

Both emitters preserve f32 bits; `approx` is only a display hint. Non-finite
words retain their bits with `approx: 0.0`, because JSON NaN/Infinity or null
cannot round-trip through the canonical f64 `approx` field. Addresses, flags,
and AObj times are diagnostics in the oracle `.meta.jsonl`, not comparison
keys. No `decode.py` step is needed for bones.

**The Rust gate assumes an identity world/root boundary:** it loads the
archive SRT, attaches Wait1, requests the selected frame once, then calls
`anim_all` and `setup_matrix` for every bone in preorder. The root has no
parent; no artificial identity-matrix concatenation is added. Its local
archive/animation SRT is preserved. Later samples advance through AObj's
normal rate-1 playback rather than repeatedly requesting frames.

Retail also sets the root's translation from `fp->cur_pos`
([fighter.c:519](../third_party/melee-decomp/src/melee/ft/fighter.c#L519)),
sets model scale ([fighter.c:213-229](../third_party/melee-decomp/src/melee/ft/fighter.c#L213)),
and sets bone 0 Y rotation to `M_PI_2 * facing_dir`
([fighter.c:1173-1175](../third_party/melee-decomp/src/melee/ft/fighter.c#L1173)).
Those fighter-layer overrides are **not applied** by this gate. Thus even a
fighter standing at the origin facing right is not automatically an
identity-root oracle pose. An ordinary match may immediately diverge at the
root. Inspect the captured root SRT before interpreting that as an HSD bug.
To establish a passing identity-root gate, the oracle must provide that same
root convention; extending the simulator with the audited fighter root
overrides is a separate alternative. Do not normalize matrices after capture:
inverse/concatenation introduces extra rounding and hides the original bits.

## Capture in Dolphin

Use the scripting build and configuration in [DOLPHIN.md](DOLPHIN.md) and
[DOLPHIN_BUILD.md](DOLPHIN_BUILD.md). Run from the repository root, changing
the savestate's stage name as needed. Start with **one sample** to establish
timing and root conventions before extending the capture.

```sh
mkdir -p harness/traces
MELEE_BONES_SAVESTATE="$PWD/harness/roms/idle_fd_fox.sav" \
MELEE_BONES_OUT="$PWD/harness/traces/fox.bones.expected.jsonl" \
MELEE_BONES_FRAMES=1 MELEE_BONES_FIGHTER_INDEX=0 \
  "$HOME/Projects/dolphin-scripting/build/Binaries/Dolphin.app/Contents/MacOS/Dolphin" \
  -e "$PWD/harness/roms/GALE01.iso" --script "$PWD/harness/dolphin_bones_snippet.py"
```

Use a fresh output basename on each run. Wait for `<output>.done`; an
`<output>.err` means capture failed, and the partial trace is not a gate.
The selected index is in `walk.fighter_gobjs` list order. The snippet checks
Fox kind=1, Wait1 animation-table row=2, and 73 joints. It loads the state
inside the first CPU-thread `frameadvance` callback, following the current
`trace_scenario.py` workflow, then records that boundary as ordinal 0. It
reissues neutral inputs and stops after N callbacks. Savestate boundary
synchronization still needs checking against the saved state on a live run.

For integration into `trace_scenario.py`, import `dump_frame` from
`dolphin_bones_snippet`, open separate bones and metadata streams, and add
this call after savestate loading and before incrementing `self.frame`:

```python
dump_frame(self.mem, selected_gobj, self.frame, bones_out, bones_metadata)
```

Select `selected_gobj` with the existing `walk.fighter_gobjs` and close both
streams on completion/failure. Importing the snippet does not register an
event listener; Dolphin permits only one listener per event. This task does
not modify anything under `harness/dolphin/`.

The `.meta.jsonl` includes raw `Fighter.cur_anim_frame`, each present AObj's
`curr_frame`, all SRT/matrix words, and `dirty_bones`. **Inspect dirty flags**:
scalar reads return cached `mtx`; they cannot invoke `HSD_JObjGetMtxPtr`, which
first performs setup ([jobj.h:697-701](../third_party/melee-decomp/src/sysdolphin/baselib/jobj.h#L697)).
Dirty matrices can be stale, especially with Null video or hidden bones.
A capture containing dirty bones needs a capture point after game-side
matrix setup before it can certify the gate. The snippet preserves those
values and flags instead of silently discarding joints or rebuilding them.

## Evaluate and compare

Inspect the first metadata line and derive the starting time from the AObjs.
This example refuses mixed times and dirty matrices rather than guessing an
off-by-one correction. The integer ordinal and Fighter time are not substitutes
for the time that produced the bone pose.

```sh
ANIM_FRAME=$(python3 - <<'PY'
import json, struct
p = "harness/traces/fox.bones.expected.jsonl.meta.jsonl"
with open(p) as f:
    m = json.loads(next(f))
assert not m["dirty_bones"], f"stale matrices: {m['dirty_bones']}"
times = {j["aobj_curr_frame"] for j in m["joints"] if j["aobj_curr_frame"] is not None}
assert len(times) == 1, f"AObj times disagree: {times}"
bits = times.pop()
print(struct.unpack(">f", struct.pack(">I", bits))[0])
PY
) && cargo run -q -p melee-sim -- bones --fighter fox --anim Wait1 \
  --frame "$ANIM_FRAME" --frames 1 > harness/traces/fox.bones.actual.jsonl

cargo run -q -p melee-diff -- \
  harness/traces/fox.bones.expected.jsonl harness/traces/fox.bones.actual.jsonl
```

`--assets /absolute/path/to/files` overrides the compiled repository-root
default `harness/roms/files/`. The three required files are `PlFxNr.dat`,
`PlFx.dat`, and `PlFxAJ.dat`; the CLI reports missing assets, while real-disc
tests skip cleanly if any are absent. See [DISC.md](DISC.md) for extraction
and the Wait1 sub-archive layout. The library entry point is
`melee_sim::bones::write_fox_wait1_bones(assets, frame, frames, writer)`.

For N samples, use the same N on both sides and verify metadata AObj times
follow rate-1 playback throughout, with no hitlag, state transition, blending,
or motion change. Fractional first frames are supported. Match record counts
(`wc -l` should show N × 1,606 for both): `first_divergence` intentionally
allows extra actual records and fields, so a successful diff alone does not
prove equal trace lengths.

Likely causes of divergence:

- **Wrong frame/capture point:** FIRST_PLAY, frame-begin callback timing,
  hitlag or animation rate. Compare AObj and Fighter times in metadata.
- **Root transform:** root position, facing rotation, or model scale differs;
  an early root mismatch propagates to most descendants.
- **Fused operation:** matching SRT but a few matrix ULPs differ. Check retail
  assembly for FMA/paired-single ordering; do not loosen expected bits.
- **Envelope matrices:** `JObj.mtx` is a world matrix, while `envelopemtx`
  is an inverse bind matrix. Skinning palettes include additional products.
  This gate compares the former, never a palette or `envelopemtx`.
- **Dirty matrices or attachment differences:** cached matrices are stale,
  or fighter part masks/blending differ from the unfiltered Wait1 attachment.

## Runtime layout derivation

All offsets are hexadecimal, all pointers/f32/u32 are four bytes on Gekko,
and memory is big-endian. These are runtime structures, not on-disc Joint
descriptors. Header comments agree with the following width calculations.

| Structure / field | Offset / width | Header evidence and calculation |
|---|---|---|
| HSD_GObj classifier | 00 / 2 | [gobj.h:28-35](../third_party/melee-decomp/src/sysdolphin/baselib/gobj.h#L28); u16 plus six u8 = 8 bytes; fighter classifier=4 at line 12 |
| HSD_GObj hsd_obj, user_data | 28, 2C / 4 each | [gobj.h:36-44](../third_party/melee-decomp/src/sysdolphin/baselib/gobj.h#L36); six pointers after byte 8 reach 20, u64 ends at 28 |
| Fighter gobj, kind, anim_id | 00, 04, 14 / 4 each | [ft/types.h:1126-1132](../third_party/melee-decomp/src/melee/ft/types.h#L1126) |
| Fighter facing_dir, cur_pos | 2C / 4, B0 / 12 | [ft/types.h:1138-1148](../third_party/melee-decomp/src/melee/ft/types.h#L1138) |
| Fighter cur_anim_frame | 894 / 4 | [ft/types.h:1294](../third_party/melee-decomp/src/melee/ft/types.h#L1294), also `fighter.generated.yaml` |
| HSD_JObj object | 00 / 8 | [class.h:14-16](../third_party/melee-decomp/src/sysdolphin/baselib/class.h#L14) and [object.h:60-64](../third_party/melee-decomp/src/sysdolphin/baselib/object.h#L60): class pointer + two u16 = 8 |
| next, parent, child, flags, u | 08, 0C, 10, 14, 18 / 4 each | [jobj.h:104-114](../third_party/melee-decomp/src/sysdolphin/baselib/jobj.h#L104); five words after HSD_Obj |
| rotate, scale, translate | 1C / 16, 2C / 12, 38 / 12 | [jobj.h:115-117](../third_party/melee-decomp/src/sysdolphin/baselib/jobj.h#L115); Quaternion is four f32, Vec3 three ([mtx.h:14-16,46-50](../third_party/melee-decomp/extern/dolphin/include/dolphin/mtx.h#L46)) |
| mtx | 44 / 30 | [jobj.h:118](../third_party/melee-decomp/src/sysdolphin/baselib/jobj.h#L118); Mtx[3][4] = 12 × 4 bytes |
| scl, envelopemtx, aobj | 74, 78, 7C / 4 each | [jobj.h:119-125](../third_party/melee-decomp/src/sysdolphin/baselib/jobj.h#L119); followed by robj at 80 and id at 84, total 88 |
| HSD_AObj curr_frame | 04 / 4 | [aobj.h:40-48](../third_party/melee-decomp/src/sysdolphin/baselib/aobj.h#L40); u32 flags precedes it |

`fighter_root_jobj` distinguishes the classifier from a MEM1 pointer and
checks the Fighter→GObj→user_data backlink. It never reads Fighter.+28 as a
JObj. The tree reader skips `JOBJ_INSTANCE` children and root siblings, as
[HSD_JObjAddAnimAll](../third_party/melee-decomp/src/sysdolphin/baselib/jobj.c#L323)
does. This equals `JObjTree::depth_first` for Fox's one-root skeleton;
that Rust iterator can also traverse a forest, while animation attachment
is scoped to a subtree. Corrupt pointers/cycles fail rather than truncate.
