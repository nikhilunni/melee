"""Validate a decoded idle trace: unit-rate animations and bounded retail RNG draws.

Ordinals are zero-based nonblank record indices, independent of `frame`.
This is an idle/unit-animation-rate diagnostic, not a rule for hitlag, paused
fighters, or actions whose animation rate differs from one.
"""
from __future__ import annotations

import argparse
from collections import Counter
import json
import math
from pathlib import Path
import struct


def lcg(seed: int) -> int:
    return (214013 * seed + 2531011) & 0xFFFFFFFF


def draw_count(before: int, after: int, maximum: int) -> int | None:
    for count in range(maximum + 1):
        if before == after:
            return count
        before = lcg(before)
    return None


def animations(state: dict) -> dict[str, float]:
    # Trust the canonical bits, never the human-readable approximation.
    return {key: struct.unpack(">f", struct.pack(">I", field["v"]["bits"]))[0]
            for key, field in state.items() if key.endswith(".cur_anim_frame")}


def validate(records, max_draws: int = 64, scripted: bool = False) -> dict:
    """`scripted`: inputs drive the fighters, so animation rates are not unit;
    only finiteness, RNG reachability and the tick metadata are checked."""
    if max_draws < 0:
        raise ValueError("max_draws must be nonnegative")
    histogram = Counter()
    violations = []
    previous = None
    previous_anim = {}
    count = 0
    for ordinal, record in enumerate(records):
        count += 1
        state = record["state"]
        current_anim = animations(state)
        seed = state["rng.seed"]["v"]
        if type(seed) is not int or not 0 <= seed <= 0xFFFFFFFF:
            raise ValueError(f"ordinal {ordinal}: rng.seed is not a u32")
        if not current_anim:
            violations.append(f"ordinal {ordinal}: no fighter animation fields")
        for key, current in sorted(current_anim.items()):
            if not math.isfinite(current):
                violations.append(f"ordinal {ordinal}: {key} is not finite")
            elif key in previous_anim:
                before = previous_anim[key]
                # Legitimate per-tick changes: +1 (rate 1), a restart, or a
                # hold (frame speed 0, e.g. EntryEnd waiting for "GO!", or -1
                # before the first animation). Anything else, such as +2, is a
                # sampling straddle; the tick-counter metadata check below is
                # the exact detector when the raw record carries it.
                if not scripted and not (current - before == 1 or current <= before):
                    violations.append(f"ordinal {ordinal}: {key} {before:g} -> {current:g} "
                                      "(expected +1, hold, or restart)")
        if previous is not None:
            if current_anim.keys() != previous_anim.keys():
                violations.append(f"ordinal {ordinal}: fighter animation field set changed")
            before_seed = previous["state"]["rng.seed"]["v"]
            draws = draw_count(before_seed, seed, max_draws)
            if draws is None:
                violations.append(f"ordinal {ordinal}: rng.seed {before_seed} -> {seed} "
                                  f"not reachable in 0..{max_draws} LCG draws")
            else:
                histogram[draws] += 1
        # Optional sampler diagnostics survive decode without becoming state keys.
        metadata = {"tick", "vi_frame", "watch_address", "watch_value"}
        if metadata.intersection(record):
            if not metadata.issubset(record):
                violations.append(f"ordinal {ordinal}: incomplete tick metadata")
            else:
                scene_reset = ordinal == 0 and record["tick"] == 0
                if record["tick"] != (record["watch_value"] + 1) & 0xFFFFFFFF and not scene_reset:
                    violations.append(f"ordinal {ordinal}: watch value is not pre-increment tick")
                if previous is not None and metadata.issubset(previous):
                    if record["tick"] != (previous["tick"] + 1) & 0xFFFFFFFF:
                        violations.append(f"ordinal {ordinal}: nonconsecutive game tick")
                    if record["vi_frame"] < previous["vi_frame"]:
                        violations.append(f"ordinal {ordinal}: VI count moved backwards")
                    if record["watch_address"] != previous["watch_address"]:
                        violations.append(f"ordinal {ordinal}: watched address changed")
        elif previous is not None and metadata.intersection(previous):
            violations.append(f"ordinal {ordinal}: tick metadata disappeared")
        previous, previous_anim = record, current_anim
    if count < 2:
        violations.append("trace needs at least two records to validate transitions")
    return {"records": count, "histogram": dict(sorted(histogram.items())),
            "violations": violations}


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("trace", type=Path)
    parser.add_argument("--max-draws", type=int, default=64, metavar="K")
    parser.add_argument("--scripted", action="store_true",
                        help="inputs drive the fighters: skip the unit animation-rate rule")
    args = parser.parse_args(argv)
    if args.max_draws < 0:
        parser.error("--max-draws must be nonnegative")
    try:
        with args.trace.open() as stream:
            result = validate((json.loads(line) for line in stream if line.strip()),
                              args.max_draws, args.scripted)
    except (OSError, ValueError, KeyError, TypeError, struct.error, OverflowError) as exc:
        print(f"invalid trace: {exc}")
        return 1
    print(f"records: {result['records']}; transitions: {max(0, result['records'] - 1)}")
    print(f"LCG draw-count histogram (0..{args.max_draws}): {result['histogram']}")
    for violation in result["violations"]:
        print(violation)
    print(f"{'FAIL' if result['violations'] else 'PASS'}: {len(result['violations'])} violations")
    return int(bool(result["violations"]))


if __name__ == "__main__":
    raise SystemExit(main())
