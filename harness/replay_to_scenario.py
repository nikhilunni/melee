"""Turn a port input recording into a Dolphin scenario, and check the result.

    cd harness && uv run python replay_to_scenario.py <recording.json> --name <scenario> [--ticks N]
    cd harness && uv run python replay_to_scenario.py <recording.json> --name <scenario> --verify

A `melee-replay` recording (the corpus explorer, the native app's export) holds
the match configuration and the controller state the port consumed on every
tick. When its configuration matches a boundary in `boundaries.toml`, the same
match exists in retail: load that savestate and feed Dolphin the same pads.
The scenario it writes is an ordinary scripted scenario, so record.py captures
it and the usual gates compare the port against it.

`Match::new` completes the oracle's tick 0 (the pre-music boundary, whose pads
the savestate already renewed), so the recording's sample k is oracle tick k+1:
the scenario is one tick longer than the recording.

Every analog value must be one retail can produce: sticks are post-clamp
multiples of 1/80 with radius <= 80, triggers multiples of 1/140. The Dolphin
float for each value comes from `dolphin/pad_calibration.json`
(calibrate_pads.py). `--verify` compares the pads the tick trace recorded with
the recording, tick for tick, so a wrong table or VI alignment cannot pass.
"""
from __future__ import annotations

import argparse
import json
import struct
import sys
import tomllib
from pathlib import Path

HERE = Path(__file__).resolve().parent
BOUNDARIES = HERE / "boundaries.toml"
CALIBRATION = HERE / "dolphin" / "pad_calibration.json"

# HSD_Pad bits (melee-ft input/pad.rs) -> dolphin.controller key names.
BUTTONS = {1 << 0: "Left", 1 << 1: "Right", 1 << 2: "Down", 1 << 3: "Up", 1 << 4: "Z",
           1 << 5: "R", 1 << 6: "L", 1 << 8: "A", 1 << 9: "B", 1 << 10: "X", 1 << 11: "Y",
           1 << 12: "Start"}
STICK_MAX, TRIGGER_MAX = 80, 140


class Unreachable(ValueError):
    """A recorded value retail's pad path cannot produce."""


def f32(bits: int) -> float:
    return struct.unpack("<f", struct.pack("<I", bits))[0]


def boundary_for(config: dict) -> dict:
    players = [p[1] for p in sorted(config["players"])]
    layout = [b for b in tomllib.loads(BOUNDARIES.read_text())["boundary"]
              if (b["stage"], b["players"], b["stocks"]) == (config["stage"], players, config["stocks"])]
    if not layout:
        raise SystemExit(f"no boundary in {BOUNDARIES.name} for {config['stage']} {players} "
                         f"{config['stocks']} stocks")
    if any(p[2] != 0 for p in config["players"]) or not config["all_characters_unlocked"]:
        raise SystemExit("boundaries assume default costumes and the unlocked roster")
    for b in layout:
        if b["seed"] == config["seed"]:
            return b
    raise SystemExit(f"recording seed {config['seed']} is no boundary seed for this layout; "
                     f"use one of {[b['seed'] for b in layout]}")


def as_f32(value: float) -> float:
    return struct.unpack("<f", struct.pack("<f", value))[0]


def integral(value: float, scale: int, lo: int, hi: int, what: str) -> int:
    """HSD stores raw / scale as an f32 quotient; recover raw and require an exact match."""
    raw = round(value * scale)
    if as_f32(raw / scale) != value or not lo <= raw <= hi:
        raise Unreachable(f"{what} {value!r} is not k/{scale} with {lo} <= k <= {hi}")
    return raw


def stick_raw(x: float, y: float, what: str) -> tuple[int, int]:
    rx = integral(x, STICK_MAX, -STICK_MAX, STICK_MAX, what)
    ry = integral(y, STICK_MAX, -STICK_MAX, STICK_MAX, what)
    if rx * rx + ry * ry > STICK_MAX * STICK_MAX:
        raise Unreachable(f"{what} ({x}, {y}) lies outside HSD's clamp radius")
    return rx, ry


def dolphin_pad(sample: list[int], table: dict) -> tuple[dict, dict]:
    """One port's recorded sample -> (set_gc_buttons keys, raw PADStatus values),
    neutral entries omitted from both."""
    buttons, sx, sy, cx, cy, lt, rt = sample
    out: dict = {}
    raw_pad: dict = {}
    for bit, name in BUTTONS.items():
        if buttons & bit:
            out[name] = True
    if buttons & ~sum(BUTTONS):
        raise Unreachable(f"button bits {buttons & ~sum(BUTTONS):#x} are not controller buttons")
    stick = stick_raw(f32(sx), f32(sy), "stick")
    cstick = stick_raw(f32(cx), f32(cy), "C-stick")
    if buttons:
        raw_pad["button"] = buttons
    for key, raw_key, raw in (("StickX", "stickX", stick[0]), ("StickY", "stickY", stick[1]),
                              ("CStickX", "substickX", cstick[0]),
                              ("CStickY", "substickY", cstick[1])):
        if raw:
            out[key] = table["stick"][str(raw)]
            raw_pad[raw_key] = raw
    for key, raw_key, bits in (("TriggerLeft", "triggerL", lt), ("TriggerRight", "triggerR", rt)):
        raw = integral(f32(bits), TRIGGER_MAX, 0, TRIGGER_MAX, key)
        if raw:
            out[key] = table["trigger"][str(raw)]
            raw_pad[raw_key] = raw
    return out, raw_pad


def toml_value(value) -> str:
    return "true" if value is True else repr(value)


def write_scenario(recording: dict, name: str, ticks: int | None) -> Path:
    boundary = boundary_for(recording["config"])
    table = json.loads(CALIBRATION.read_text())
    samples = recording["samples"][:ticks] if ticks else recording["samples"]
    steps, previous = [], {}
    # Sample k drives oracle tick k+1; tick 0 is the boundary's own.
    for k, sample in enumerate(samples, start=1):
        for port in range(2):
            pad, raw = dolphin_pad(sample[port], table)
            if previous.get(port) != raw:
                keys = ", ".join(f"{key} = {toml_value(v)}" for key, v in pad.items())
                raws = ", ".join(f"{key} = {v}" for key, v in raw.items())
                steps.append(f"  {{ frame = {k}, port = {port}, buttons = {{ {keys} }}, raw = {{ {raws} }} }},")
                previous[port] = raw
    fault = recording.get("fault")
    note = f"# Port fault at attempt {fault['attempt']}: {fault['message']}\n" if fault else ""
    fighters = "\n".join(f'[[fighters]]\nslot = {i}\nkind = "{kind}"\ncontroller = "scripted"\n'
                         for i, kind in enumerate(boundary["players"]))
    path = HERE / "scenarios" / f"{name}.toml"
    path.write_text(f'''# Generated by replay_to_scenario.py from a melee-replay recording.
# Boundary {boundary["name"]}; {len(samples)} recorded ticks. Step frames count
# tick records (input_clock = "tick", dolphin/tick_trace.py).
{note}name = "{name}"
input_clock = "tick"
savestate = "{boundary["savestate"]}"
frames = {len(samples) + 1}
seed = 1
stage = "{boundary["stage"]}"
inputs = [
{chr(10).join(steps)}
]

{fighters}''')
    return path


def fit_to_trace(name: str) -> None:
    """A retail match that reached GAME before the recording's end stopped early
    (tick_trace.py `ended_early`); shorten the scenario to the recorded ticks."""
    done = json.loads((HERE / "traces" / f"{name}.tick.raw.jsonl.done").read_text())
    if not done.get("ended_early"):
        return
    path = HERE / "scenarios" / f"{name}.toml"
    text = path.read_text()
    frames = tomllib.loads(text)["frames"]
    if frames != done["ticks"]:
        path.write_text(text.replace(f"\nframes = {frames}\n", f"\nframes = {done['ticks']}\n", 1))
        print(f"{name}: retail reached GAME after {done['ticks']} ticks (recording {frames}); "
              "scenario shortened")


def verify(recording: dict, name: str) -> int:
    """Compare recorded game pads with the recording; return the first mismatching tick or -1."""
    fit_to_trace(name)
    trace = HERE / "traces" / f"{name}.tick.expected.jsonl"
    lines = trace.open()
    next(lines)  # tick 0 is the boundary's; sample k drove tick k+1
    for k, (line, sample) in enumerate(zip(lines, recording["samples"]), start=1):
        pads = json.loads(line)["inputs"]
        for port in range(2):
            p = pads[f"p{port}"]
            want = sample[port]
            got = [p["button"]["v"] & 0xFFFF, p["nml_stickX"]["v"]["bits"], p["nml_stickY"]["v"]["bits"],
                   p["nml_subStickX"]["v"]["bits"], p["nml_subStickY"]["v"]["bits"],
                   p["nml_analogL"]["v"]["bits"], p["nml_analogR"]["v"]["bits"]]
            if got != [want[0] & 0xFFFF, *want[1:]]:
                print(f"tick record {k} port {port}: trace {got} != recording {want}")
                return k
    return -1


def main(argv: list[str] | None = None) -> None:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("recording", type=Path)
    ap.add_argument("--name", required=True, help="scenario name (harness/scenarios/<name>.toml)")
    ap.add_argument("--ticks", type=int, help="bridge only the first N ticks")
    ap.add_argument("--verify", action="store_true", help="check the recorded trace's pads instead")
    a = ap.parse_args(argv)
    recording = json.loads(a.recording.read_text())
    if a.verify:
        first = verify(recording, a.name)
        sys.exit(0 if first < 0 else 1)
    print(write_scenario(recording, a.name, a.ticks))


if __name__ == "__main__":
    main()
