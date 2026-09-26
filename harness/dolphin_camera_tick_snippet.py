"""Tick-aligned gameplay camera dump: the oracle for the camera port (cm/).

Sampled at the scheduler tick boundary (tick_trace.TickTracer), one record per
tick with raw big-endian bytes, decoded by the Rust camera tests:

- `camera`: game_camera (struct Camera, cm/types.h, 0x39C bytes)
- `cobj`: the main camera's HSD_CObj (cobj.h:37, 0x8C bytes), plus the
  positions of its eye (`eye`) and interest (`interest`) WObjs (wobj.h:12,
  Vec3 at +0xC)
- `subjects`: every CmSubject from cm_804D6468 along `prev` (0x6C bytes each)
- `magnify`: ifMagnify_804A1DE0 (if/types.h:123, 0xF0 bytes)

Run through record.py (`--camera`) or directly in Dolphin with
MELEE_CAMERA_SAVESTATE, MELEE_CAMERA_SCENARIO, MELEE_CAMERA_OUT and
MELEE_CAMERA_TICKS, like dolphin_bones_tick_snippet.py.
"""
from __future__ import annotations

import json
import os
import sys
import traceback
from pathlib import Path

HERE = Path(globals().get("__file__") or sys._getframe().f_code.co_filename).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE / "dolphin"))
import symbols  # noqa: E402
import walk  # noqa: E402
from tick_trace import TickTracer  # noqa: E402
from trace_common import event, run  # noqa: E402

CAMERA_ADDR = symbols.addr("game_camera")
CAMERA_BYTES = 0x39C
SUBJECTS_HEAD = symbols.addr("cm_804D6468")
SUBJECT_BYTES = 0x6C
MAGNIFY_ADDR = symbols.addr("ifMagnify_804A1DE0")
MAGNIFY_BYTES = 0xF0
COBJ_BYTES = 0x8C
GOBJ_HSD_OBJ = 0x28  # gobj.h:43
COBJ_EYE, COBJ_INTEREST = 0x24, 0x28  # cobj.h:42-43
WOBJ_POS, VEC3_BYTES = 0xC, 12  # wobj.h:15
MAX_SUBJECTS = 64


def camera_state(mem) -> dict:
    state = {"camera": walk.read_bytes(mem, CAMERA_ADDR, CAMERA_BYTES).hex(),
             "magnify": walk.read_bytes(mem, MAGNIFY_ADDR, MAGNIFY_BYTES).hex()}
    gobj = mem.read_u32(CAMERA_ADDR)  # Camera.gobj, +0
    if gobj:
        cobj = mem.read_u32(gobj + GOBJ_HSD_OBJ)
        state["cobj"] = walk.read_bytes(mem, cobj, COBJ_BYTES).hex()
        for key, offset in (("eye", COBJ_EYE), ("interest", COBJ_INTEREST)):
            wobj = mem.read_u32(cobj + offset)
            if wobj:
                state[key] = walk.read_bytes(mem, wobj + WOBJ_POS, VEC3_BYTES).hex()
    subjects = []
    subject = mem.read_u32(SUBJECTS_HEAD)
    while subject and len(subjects) < MAX_SUBJECTS:
        subjects.append(walk.read_bytes(mem, subject, SUBJECT_BYTES).hex())
        subject = mem.read_u32(subject + 4)  # CmSubject.prev
    state["subjects"] = subjects
    return state


class CameraTracer(TickTracer):
    def __init__(self, *args, camera_out, **kwargs):
        super().__init__(*args, **kwargs)
        self.camera_out = camera_out

    def record(self, phase: str, mem=None) -> dict:
        mem = self.mem if mem is None else mem
        diagnostic = super().record(phase, mem)
        line = {"frame": self.frame, "phase": "camera", "state": camera_state(mem)}
        self.camera_out.write(json.dumps(line) + "\n")
        self.camera_out.flush()
        return diagnostic

    def finish(self) -> None:
        self.camera_out.close()
        super().finish()

    def fail(self, text: str) -> None:
        try:
            self.camera_out.close()
        finally:
            super().fail(text)


def main() -> None:
    import tomllib
    out = Path(os.environ["MELEE_CAMERA_OUT"]).resolve()
    ticks = int(os.environ["MELEE_CAMERA_TICKS"])
    saved = Path(os.environ["MELEE_CAMERA_SAVESTATE"]).resolve()
    scripted = tomllib.loads(Path(os.environ["MELEE_CAMERA_SCENARIO"]).read_text())

    def table(values: dict) -> str:
        return "{ " + ", ".join(f"{k} = {str(v).lower() if isinstance(v, bool) else v}"
                                for k, v in values.items()) + " }"
    steps = [f'  {{ frame = {int(st["frame"])}, port = {int(st.get("port", 0))}, '
             f'buttons = {table(st.get("buttons", {}))}'
             + (f', raw = {table(st["raw"])}' if "raw" in st else "") + " }"
             for st in scripted.get("inputs", [])]
    clock = f'input_clock = "{scripted["input_clock"]}"\n' if "input_clock" in scripted else ""
    scenario = out.with_suffix(".scenario.toml")
    scenario.write_text(f'name = "{out.stem}"\n{clock}savestate = "{saved}"\nframes = {ticks}\n'
                        "inputs = [\n" + ",\n".join(steps) + "\n]\n")
    os.environ["MELEE_SCENARIO"] = str(scenario)
    os.environ["MELEE_RAW_OUT"] = str(out.with_suffix(".raw.jsonl"))
    camera_out = out.open("w")
    run(lambda *a, **k: CameraTracer(*a, camera_out=camera_out, **k))


if event is not None and __name__ == "__main__":
    try:
        main()
    except Exception:
        Path(os.environ.get("MELEE_CAMERA_OUT", "camera.jsonl") + ".err").write_text(traceback.format_exc())
