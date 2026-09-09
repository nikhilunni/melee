"""Tick-boundary particle capture; importing this module starts no listeners.

Run in Dolphin with MELEE_PARTICLES_SAVESTATE, MELEE_PARTICLES_OUT and
MELEE_PARTICLES_TICKS. See docs/PARTICLES_DUMP.md for the exact command.
"""
from __future__ import annotations

import json
import os
from pathlib import Path
import sys
import traceback

HERE = Path(globals().get("__file__") or sys._getframe().f_code.co_filename).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE / "dolphin"))

import particle_dump as particles  # noqa: E402
from tick_trace import TickTracer  # noqa: E402
from trace_common import read_sidecar  # noqa: E402


class ParticleTracer(TickTracer):
    """Reuse TickTracer's pre-store checks, duplicate guard and deferred cleanup.

    self.out holds the existing tick diagnostics; particle_out holds portable
    melee-diff records. The initial snapshot is taken before the first guest
    tick resumes, allowing restoration of the state that produces tick zero.
    """
    def __init__(self, *args, particle_out, initial_path: Path, **kwargs):
        super().__init__(*args, **kwargs)
        self.particle_out = particle_out
        self.initial_path = initial_path

    def install(self) -> None:
        super().install()
        captured = particles.snapshot(self.mem)
        self.initial_path.write_text(json.dumps(particles.record(0, captured), allow_nan=False) + "\n")
        Path(str(self.initial_path) + ".meta.json").write_text(
            json.dumps({"sampling": "savestate_loaded_before_first_tick",
                        "tick": self.initial_tick, "particles": captured}, allow_nan=False) + "\n")

    def record(self, phase: str, mem=None) -> dict:
        mem = self.mem if mem is None else mem
        diagnostic = super().record(phase, mem)
        captured = particles.snapshot(mem)
        diagnostic["particles"] = captured
        self.particle_out.write(json.dumps(particles.record(self.frame, captured), allow_nan=False) + "\n")
        self.particle_out.flush()
        return diagnostic

    def finish(self) -> None:
        self.particle_out.close()
        super().finish()

    def fail(self, text: str) -> None:
        self.particle_out.close()
        super().fail(text)

    def summary(self) -> dict:
        return {**super().summary(), "phase": "particles", "initial_snapshot": str(self.initial_path)}


def main() -> None:
    from dolphin import controller, event, memory, savestate

    output = Path(os.environ["MELEE_PARTICLES_OUT"]).resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    done, error = Path(str(output) + ".done"), Path(str(output) + ".err")
    initial = Path(str(output) + ".initial.jsonl")
    for marker in (done, error, initial, Path(str(initial) + ".meta.json")):
        marker.unlink(missing_ok=True)
    out, metadata = output.open("w"), Path(str(output) + ".meta.jsonl").open("w")
    try:
        saved = Path(os.environ["MELEE_PARTICLES_SAVESTATE"]).resolve()
        if not saved.is_file():
            raise FileNotFoundError(saved)
        count = int(os.environ.get("MELEE_PARTICLES_TICKS", "3"))
        scenario = {"frames": count}
        if "MELEE_PARTICLES_SCENARIO" in os.environ:
            # Scripted inputs (tick_trace.py contract); the savestate and tick
            # count still come from the MELEE_PARTICLES_* variables.
            import tomllib
            scripted = tomllib.loads(Path(os.environ["MELEE_PARTICLES_SCENARIO"]).read_text())
            scenario["inputs"] = scripted.get("inputs", [])
            scenario["fighters"] = scripted.get("fighters", [])
        tracer = ParticleTracer(
            scenario, metadata, saved, read_sidecar(saved), done,
            memory, controller, savestate, event, particle_out=out, initial_path=initial,
        )
        event.on_frameadvance(tracer.on_frame)
    except Exception:
        out.close()
        metadata.close()
        error.write_text(traceback.format_exc())


if __name__ == "__main__":
    main()
