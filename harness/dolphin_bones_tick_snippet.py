"""Tick-aligned bone dump for every fighter: one record per fighter per tick.

Like dolphin_bones_snippet.py but sampled at the scheduler tick boundary
(tick_trace.TickTracer) instead of the VI frame, and for all fighters in
list order (keys `p0.bone[i].*`, `p1.bone[i].*`). Run in Dolphin with
MELEE_BONES_SAVESTATE, MELEE_BONES_OUT, MELEE_BONES_TICKS, e.g.

    MELEE_BONES_SAVESTATE=$PWD/harness/roms/start_fd_fox.sav \\
    MELEE_BONES_OUT=$PWD/harness/traces/start_fd_fox.bones.jsonl MELEE_BONES_TICKS=130 \\
    Dolphin -v OGL -C Dolphin.Core.SIDevice0=6 -C Dolphin.Core.SIDevice1=6 \\
      -e $PWD/harness/roms/GALE01.iso --script $PWD/harness/dolphin_bones_tick_snippet.py
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
import jobjdump  # noqa: E402
import walk  # noqa: E402
from tick_trace import TickTracer, playback_toml  # noqa: E402
from trace_common import ENTITIES_ADDR, event, run  # noqa: E402


# MELEE_BONES_DYNAMICS=1: also dump each fighter's lb_8001044C chain nodes
# (DynamicsData, lb/types.h lb_00F9_UnkDesc0) beside the row: per node its
# joint index and the solver's own state, which no joint SRT carries:
# +2C position, +38 velocity axis, +44 angular velocity (raw f32 bits).
DYNAMICS = bool(os.environ.get("MELEE_BONES_DYNAMICS"))
GOBJ_USER_DATA = 0x2C
FIGHTER_DYNAMIC_SETS = 0x2F0  # BoneDynamicsDesc[10], 0x18 bytes each
FIGHTER_DYNAMICS_NUM = 0x3E0
DYNAMICS_NEXT = 0x90


def dynamic_nodes(mem, gobj: int, joints) -> list:
    index_of = {j.pointer: i for i, j in enumerate(joints)}
    fp = mem.read_u32(gobj + GOBJ_USER_DATA)
    sets = []
    for s in range(min(mem.read_u32(fp + FIGHTER_DYNAMICS_NUM), 10)):
        node = mem.read_u32(fp + FIGHTER_DYNAMIC_SETS + 0x18 * s + 4)
        nodes = []
        while node and len(nodes) < 32:
            nodes.append({"bone": index_of.get(mem.read_u32(node), -1),
                          "words": [f"{w:08X}" for w in jobjdump.float_bits(mem, node + 0x2C, 7)]})
            node = mem.read_u32(node + DYNAMICS_NEXT)
        sets.append(nodes)
    return sets


class BonesTracer(TickTracer):
    def __init__(self, *args, bones_out, first_tick=0, **kwargs):
        super().__init__(*args, **kwargs)
        self.bones_out = bones_out
        self.first_tick = first_tick

    def record(self, phase: str, mem=None) -> dict:
        mem = self.mem if mem is None else mem
        diagnostic = super().record(phase, mem)
        if self.frame < self.first_tick:
            return diagnostic
        state = {}
        # Beside the state, as dolphin_bones_snippet.py's metadata does: the
        # joints whose cached mtx is stale (JOBJ_MTX_DIRTY), built by no one
        # since their SRT changed. Outside `state`, so word counts stay.
        dirty = {}
        dynamics = {}
        for index, gobj in enumerate(walk.fighter_gobjs(mem, ENTITIES_ADDR)):
            root = jobjdump.fighter_root_jobj(mem, gobj)
            joints = jobjdump.jobj_tree(mem, root)
            for rec in jobjdump.records(self.frame, joints):
                for key, value in rec["state"].items():
                    state[f"p{index}." + key.split(".", 1)[1]] = value
            dirty[f"p{index}"] = [i for i, j in enumerate(joints) if j.flags & jobjdump.JOBJ_MTX_DIRTY]
            if DYNAMICS:
                dynamics[f"p{index}"] = dynamic_nodes(mem, gobj, joints)
        line = {"frame": self.frame, "phase": "bones", "state": state, "dirty_bones": dirty}
        if DYNAMICS:
            line["dynamics"] = dynamics
        self.bones_out.write(json.dumps(line, allow_nan=False) + "\n")
        self.bones_out.flush()
        return diagnostic

    def finish(self) -> None:
        self.bones_out.close()
        super().finish()

    def fail(self, text: str) -> None:
        try:
            self.bones_out.close()
        finally:
            super().fail(text)


def main() -> None:
    import tomllib
    out = Path(os.environ["MELEE_BONES_OUT"]).resolve()
    ticks = int(os.environ.get("MELEE_BONES_TICKS", "2"))
    saved = Path(os.environ["MELEE_BONES_SAVESTATE"]).resolve()
    # Reuse trace_common.run by synthesising the scenario it expects.
    # Beside the dump: the traces directory of the data root (a worktree has
    # none of its own, harness/data_root.py).
    scenario = out.parent / (out.stem + ".bones_scenario.toml")
    inputs = "inputs = []\n"
    clock = ""

    def table(values: dict) -> str:
        return "{ " + ", ".join(f"{k} = {str(v).lower() if isinstance(v, bool) else v}"
                                for k, v in values.items()) + " }"
    if "MELEE_BONES_SCENARIO" in os.environ:
        # Replay a scripted scenario's inputs (tick_trace.py contract) so the
        # bone dump covers the same motion as its tick trace, including the
        # tick input clock and its raw pads.
        scripted = tomllib.loads(Path(os.environ["MELEE_BONES_SCENARIO"]).read_text())
        steps = [f'  {{ frame = {int(st["frame"])}, port = {int(st.get("port", 0))}, '
                 f'buttons = {table(st.get("buttons", {}))}'
                 + (f', raw = {table(st["raw"])}' if "raw" in st else "") + " }"
                 for st in scripted.get("inputs", [])]
        inputs = "inputs = [\n" + ",\n".join(steps) + "\n]\n"
        # The tick clock, the seed and the display cadence (tick_trace.py).
        clock = playback_toml(scripted)
    scenario.write_text(f'name = "{out.stem}"\n{clock}savestate = "{saved}"\nframes = {ticks}\n{inputs}')
    os.environ["MELEE_SCENARIO"] = str(scenario)
    os.environ["MELEE_RAW_OUT"] = str(out.with_suffix(".raw.jsonl"))
    bones_out = out.open("w")
    first_tick = int(os.environ.get("MELEE_BONES_FROM", "0"))
    run(lambda *a, **k: BonesTracer(*a, bones_out=bones_out, first_tick=first_tick, **k))


if event is not None and __name__ == "__main__":
    try:
        main()
    except Exception:
        Path(os.environ.get("MELEE_BONES_OUT", "bones.jsonl") + ".err").write_text(traceback.format_exc())
