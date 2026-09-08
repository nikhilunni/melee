"""M2 frameadvance capture, usable standalone or from trace_scenario.py.

Drop-in call inside the existing callback, after savestate load and before
incrementing its frame: dump_frame(mem, selected_gobj, frame, bones, metadata).
It does not register another listener when imported. See docs/M2_GATE.md.
"""
from __future__ import annotations

from dataclasses import asdict
import json
import os
from pathlib import Path
import sys
import traceback

# Dolphin --script has no __file__; the code object still has its filename.
HERE = Path(globals().get("__file__") or sys._getframe().f_code.co_filename).resolve().parent
sys.path.insert(0, str(HERE))

import jobjdump as bones  # noqa: E402


def dump_frame(mem, gobj: int, frame: int, out, metadata) -> None:
    """Capture one selected Fox as p0. Keep addresses/timing out of the diff.

    Capture dirty matrices too, marking them in metadata: scalar reads cannot
    call HSD_JObjGetMtxPtr and must never pretend they performed matrix setup.
    """
    root = bones.fighter_root_jobj(mem, gobj)
    if not root:
        raise ValueError("selected fighter has no root JObj")
    fp = mem.read_u32(gobj + bones.GOBJ_USER_DATA)
    if not fp or mem.read_u32(fp + bones.FIGHTER_KIND) != 1:
        raise ValueError("selected GObj must be Fox (FighterKind=1)")
    if mem.read_u32(fp + bones.FIGHTER_ANIM_ID) != 2:
        raise ValueError("selected Fox is not playing Wait1 (animation-table row 2)")
    joints = bones.jobj_tree(mem, root)
    if len(joints) != 73:
        raise ValueError(f"expected 73 Fox joints, got {len(joints)}")
    diagnostic = {
        "frame": frame,
        "gobj": gobj,
        "fighter": fp,
        "cur_anim_frame": bones.float_bits(mem, fp + bones.FIGHTER_CUR_ANIM_FRAME, 1)[0],
        "facing_dir": bones.float_bits(mem, fp + bones.FIGHTER_FACING, 1)[0],
        "cur_pos": bones.float_bits(mem, fp + bones.FIGHTER_POSITION, 3),
        "dirty_bones": [i for i, j in enumerate(joints) if j.flags & bones.JOBJ_MTX_DIRTY],
        "joints": [asdict(j) for j in joints],
    }
    metadata.write(json.dumps(diagnostic, allow_nan=False) + "\n")
    for record in bones.records(frame, joints):
        out.write(json.dumps(record, allow_nan=False) + "\n")
    metadata.flush()
    out.flush()


def main() -> None:
    from dolphin import controller, event, memory, savestate

    sys.path.insert(0, str(HERE / "dolphin"))
    import walk
    import symbols

    saved = Path(os.environ["MELEE_BONES_SAVESTATE"]).resolve()
    if not saved.is_file():
        raise FileNotFoundError(saved)
    output = Path(os.environ["MELEE_BONES_OUT"]).resolve()
    count = int(os.environ.get("MELEE_BONES_FRAMES", "1"))
    selected = int(os.environ.get("MELEE_BONES_FIGHTER_INDEX", "0"))
    if count <= 0 or selected < 0:
        raise ValueError("frames must be positive and fighter index nonnegative")
    output.parent.mkdir(parents=True, exist_ok=True)
    entities = symbols.addr("HSD_GObj_Entities")
    out = output.open("w")
    metadata = Path(str(output) + ".meta.jsonl").open("w")
    frame = 0

    def finish() -> None:
        out.close()
        metadata.close()
        event.on_frameadvance(lambda: None)

    def on_frame() -> None:
        nonlocal frame
        try:
            if frame == 0:
                # Like trace_scenario.py: load inside the CPU-thread callback.
                savestate.load_from_file(str(saved))
            for port in range(4):
                controller.set_gc_buttons(port, {})
            gobjs = walk.fighter_gobjs(memory, entities)
            if selected >= len(gobjs):
                raise ValueError(f"fighter index {selected} absent; found {len(gobjs)} GObjs")
            dump_frame(memory, gobjs[selected], frame, out, metadata)
            frame += 1
            if frame == count:
                finish()
                Path(str(output) + ".done").write_text(json.dumps({"frames": frame}))
        except Exception:
            Path(str(output) + ".err").write_text(traceback.format_exc())
            finish()

    event.on_frameadvance(on_frame)


if __name__ == "__main__":
    main()
