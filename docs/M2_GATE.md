# Milestone 2 bone comparison

The M2 gate compares non-dirty retail Fox world-matrix words against complete
simulated fighter poses on Yoshi's Story. Capture ordinals are aligned to
scheduler ticks by exact animation-frame and position bits, using the same
chronological aligner as the Final Destination post-render fighter oracle.

## Aligned VI gate (2026-09-09)

The legacy two-frame, standalone Wait1 comparison was replaced because a
re-created savestate does not reproduce the old capture's animation/matrix
boundary. The old test reported 378 differing words; a diagnostic using the
full fighter simulation matched all 480 non-dirty matrix words in those same
two captured frames (VI 0/1 aligned to ticks 0/1). A dirty flag excludes only
that bone's cached matrix; neither capture ordinal nor `cur_anim_frame` alone
specifies the complete standalone animation evaluator state.

The gate now imports `idle_ys_fox`, completes its pending scheduler work, and
records 600 candidate poses. Matrix setup runs on cloned skeletons, including
the fighter's actual animation cursors, part state and dynamics; it does not
change live simulation state. `melee-test-support::rendered_pose::PoseTimeline`
selects the earliest chronological exact frame/position match, including
repeated VI observations of one tick. Matrix values never select a match.
Every non-dirty matrix word of every aligned frame is compared bit-for-bit.

Record **130 VI frames**, fighter index **0**. The gate requires **at least
100 distinct aligned scheduler ticks with non-dirty matrix coverage**; repeated
observations and all-dirty frames cannot satisfy that minimum. It also checks
all 130 capture ordinals and the complete 73 × 22-word input layout. Unmatched
frames are counted and reported. SRT words are input-format evidence here;
this oracle compares the 12 world-matrix words per non-dirty bone.

Claude records from the main checkout; do not run this in a lane:

```sh
MELEE_BONES_SAVESTATE="$PWD/harness/roms/idle_ys_fox.sav" \
MELEE_BONES_OUT="$PWD/harness/traces/fox_ys.bones.expected.jsonl" \
MELEE_BONES_FRAMES=130 MELEE_BONES_FIGHTER_INDEX=0 MELEE_BONES_ANY_ANIM=1 \
"$HOME/Projects/dolphin-scripting/build/Binaries/Dolphin.app/Contents/MacOS/Dolphin" \
-v OGL -C Dolphin.Core.SIDevice0=6 -C Dolphin.Core.SIDevice1=6 \
-e "$PWD/harness/roms/GALE01.iso" --script "$PWD/harness/dolphin_bones_snippet.py"
```

Wait for the capture's `.done` marker; an `.err` marker is a failed capture.
`MELEE_BONES_ANY_ANIM=1` permits the idle animation transitions in the longer
recording. Then run `cargo test -p melee-sim --test m2_gate -- --nocapture`.
The shared missing-data policy and the short-capture/coverage failures print
this same recording command. Missing files remain hard failures unless
`MELEE_ALLOW_MISSING_DATA=1` explicitly opts out; a short or mismatching capture
is never accepted through that switch.

## Capture word layout

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

## Historical standalone evaluator (not the current M2 gate)

The following CLI instructions document the original isolated evaluator and
remain useful for diagnosis; they do not certify the aligned VI gate above.

**The standalone evaluator assumes an identity world/root boundary:** it loads the
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
Those fighter-layer overrides are **not applied** by the historical standalone evaluator. Thus even a
fighter standing at the origin facing right is not automatically an
identity-root oracle pose. An ordinary match may immediately diverge at the
root. Inspect the captured root SRT before interpreting that as an HSD bug.
To establish a passing identity-root gate, the oracle must provide that same
root convention; extending the simulator with the audited fighter root
overrides is a separate alternative. Do not normalize matrices after capture:
inverse/concatenation introduces extra rounding and hides the original bits.

## Historical single-frame capture in Dolphin

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
matrix setup before it can certify the historical all-bone standalone comparison. The aligned
VI gate above instead compares non-dirty bones with explicit coverage requirements.
The snippet preserves those
values and flags instead of silently discarding joints or rebuilding them.

## Historical standalone evaluation and comparison

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
tests follow the explicit missing-data policy if any are absent. See [DISC.md](DISC.md) for extraction
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

## Result: passed 2026-09-08

First run against the real game, savestate `harness/roms/idle_ys_fox.sav`
(two idle Foxes, Yoshi's Story; P1 at (-42, 23.450098, 0) facing right,
Wait1 at animation frame 6.0, dirty bones 67/71/72):

```sh
# oracle (4 s wall)
OUT="$PWD/harness/traces/fox_ys.bones.expected.jsonl"
MELEE_BONES_SAVESTATE="$PWD/harness/roms/idle_ys_fox.sav" MELEE_BONES_OUT="$OUT" \
MELEE_BONES_FRAMES=2 MELEE_BONES_FIGHTER_INDEX=0 \
  ~/Projects/dolphin-scripting/build/Binaries/Dolphin.app/Contents/MacOS/Dolphin \
  -v OGL -C Dolphin.Core.SIDevice1=6 -e "$PWD/harness/roms/GALE01.iso" \
  --script "$PWD/harness/dolphin_bones_snippet.py"
# ours, with the fighter-layer root overrides
cargo run -q -p melee-sim -- bones --fighter fox --anim Wait1 --frame 6.0 --frames 2 \
  --pos=-42,23.450098,0 --facing 1 --model-scale 0.96 --bone-scale 67=1.0416667
```

Outcome: 3,212 records per side; **0 mismatches on the 70 bones the game had
recomputed** (2 frames x 70 bones x 22 words = 3,080 bit-exact values). The 58
differing words are all on the three dirty bones whose cached matrices retail
had not rebuilt. Without the root overrides the same run diverges at
`p0.bone[0].mtx[0]`, exactly as predicted above. Every non-root joint's local
SRT matched with no overrides at all, so the keyframe evaluator, quaternion
and matrix code, and the fused-multiply-add audit are all confirmed against
hardware-produced data for that historical capture. The current
`crates/melee-sim/tests/m2_gate.rs` uses the aligned model described above.

Open item: bone 67 carries scale 1/0.96 in retail; the fighter.c site that
writes it has not been located (`TODO(meaning)` in `FighterPose`).
