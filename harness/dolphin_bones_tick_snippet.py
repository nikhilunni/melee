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
from tick_trace import TickTracer  # noqa: E402
from trace_common import ENTITIES_ADDR, event, run  # noqa: E402


class BonesTracer(TickTracer):
    def __init__(self, *args, bones_out, **kwargs):
        super().__init__(*args, **kwargs)
        self.bones_out = bones_out

    def record(self, phase: str, mem=None) -> dict:
        mem = self.mem if mem is None else mem
        diagnostic = super().record(phase, mem)
        state = {}
        for index, gobj in enumerate(walk.fighter_gobjs(mem, ENTITIES_ADDR)):
            root = jobjdump.fighter_root_jobj(mem, gobj)
            for rec in jobjdump.records(self.frame, jobjdump.jobj_tree(mem, root)):
                for key, value in rec["state"].items():
                    state[f"p{index}." + key.split(".", 1)[1]] = value
        line = {"frame": self.frame, "phase": "bones", "state": state}
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
    scenario = HERE / "traces" / (out.stem + ".bones_scenario.toml")
    scenario.write_text(f'name = "{out.stem}"\nsavestate = "{saved}"\nframes = {ticks}\ninputs = []\n')
    os.environ["MELEE_SCENARIO"] = str(scenario)
    os.environ["MELEE_RAW_OUT"] = str(out.with_suffix(".raw.jsonl"))
    bones_out = out.open("w")
    run(lambda *a, **k: BonesTracer(*a, bones_out=bones_out, **k))


if event is not None and __name__ == "__main__":
    try:
        main()
    except Exception:
        Path(os.environ.get("MELEE_BONES_OUT", "bones.jsonl") + ".err").write_text(traceback.format_exc())
