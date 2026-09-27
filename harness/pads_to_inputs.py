"""Turn a human-port pad log into the scripted schedule the replay passes use.

    uv run python pads_to_inputs.py scenarios/match_fd_foxmarth.toml traces/match_fd_foxmarth.tick.raw.jsonl.pads.jsonl \
        --frames 28800 --out traces/match_fd_foxmarth.replay.toml [--name match_fd_foxmarth_replaycheck]

The tick tracer logs `{"vi_frame": f, "pads": {"0": {...}}}` for every human port
and VI frame (tick_trace.py `log_human_pads`). A step `{frame = f, port, buttons}`
re-issued by the tracer at VI frame f is what Dolphin polls during that frame, so
each change of a port's pad becomes one step at its VI frame. The replay scenario
marks those fighters `scripted` so the tracer overrides them, and keeps any
scripted steps of the other ports.
"""
from __future__ import annotations

import argparse
import json
import tomllib
from pathlib import Path

import trace_io


def steps_from_pads(lines) -> list[dict]:
    steps: list[dict] = []
    previous: dict[int, dict] = {}
    for line in lines:
        if not line.strip():
            continue
        record = json.loads(line)
        frame = int(record["vi_frame"])
        for port, pads in record["pads"].items():
            port = int(port)
            if previous.get(port) != pads:
                steps.append({"frame": frame, "port": port, "buttons": pads})
                previous[port] = pads
    return steps


def _toml_value(value) -> str:
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, float):
        text = repr(value)
        return text if ("." in text or "e" in text or "n" in text) else text + ".0"
    if isinstance(value, int):
        return str(value)
    return json.dumps(value)


def _step(step: dict) -> str:
    buttons = ", ".join(f"{k} = {_toml_value(v)}" for k, v in step["buttons"].items())
    return f'  {{ frame = {int(step["frame"])}, port = {int(step.get("port", 0))}, buttons = {{ {buttons} }} }}'


def write_replay_toml(scenario: dict, steps: list[dict], frames: int, name: str, out: Path) -> None:
    merged = sorted([*scenario.get("inputs", []), *steps], key=lambda s: (int(s["frame"]), int(s.get("port", 0))))
    lines = [f"# Replay schedule derived from the human-port pad log; not a hand-written scenario.",
             f'name = "{name}"', f'savestate = "{scenario["savestate"]}"', f"frames = {frames}"]
    for key in ("seed", "stage", "all_characters_unlocked"):
        if key in scenario:
            lines.append(f"{key} = {_toml_value(scenario[key])}")
    lines.append("inputs = [")
    lines.extend(_step(s) + "," for s in merged)
    lines.append("]")
    for fighter in scenario.get("fighters", []):
        lines.append("")
        lines.append("[[fighters]]")
        for key, value in fighter.items():
            if key == "controller" and value == "human":
                value = "scripted"
            lines.append(f"{key} = {_toml_value(value)}")
    out.write_text("\n".join(lines) + "\n")


def main(argv: list[str] | None = None) -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("scenario", type=Path)
    ap.add_argument("pads", type=Path)
    ap.add_argument("--frames", type=int, required=True, help="ticks actually recorded")
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--name", default=None)
    a = ap.parse_args(argv)
    scenario = tomllib.loads(a.scenario.read_text())
    with trace_io.open_text(a.pads) as pads:
        steps = steps_from_pads(pads)
    write_replay_toml(scenario, steps, a.frames, a.name or f'{scenario["name"]}_replay', a.out)
    print(f"{len(steps)} human steps -> {a.out}")


if __name__ == "__main__":
    main()
