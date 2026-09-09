"""Validate decoded ticks: animations, bounded RNG draws, and optional items.

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

from decode import FOX_LASER_KIND


def item_float(state: dict, key: str) -> float:
    return struct.unpack(">f", struct.pack(">I", state[key]["v"]["bits"]))[0]


class ItemValidator:
    """Track spawn generations, independent of list index or recycled addresses.

    End snapshots cannot prove arbitrary callbacks did not move an item. The
    velocity rule is therefore audited for Fox lasers only, with unchanged
    state/owner/velocity and no recorded collision, hitlag or extra movement.
    Other transitions still get identity checks and are counted as skipped.
    """

    def __init__(self):
        self.previous = {}
        self.events = []
        self.checked = 0
        self.skipped = 0
        self.present = False

    def update(self, record: dict, ordinal: int, violations: list[str]) -> None:
        if self.present and "items" not in record:
            violations.append(f"ordinal {ordinal}: items field disappeared")
        self.present |= "items" in record
        current, gobjs, bases = {}, set(), set()
        for item in record.get("items", []):
            state = item["state"]
            spawn_id = state["spawn_id"]["v"]
            gobj, base = int(item["gobj"], 16), int(item["base"], 16)
            label = f"ordinal {ordinal}: item {spawn_id}"
            if spawn_id in current or gobj in gobjs or base in bases:
                violations.append(f"{label}: duplicate item identity/address")
            if not (0x80000000 <= gobj <= 0x81800000 - 0x38 and gobj % 4 == 0
                    and 0x80000000 <= base <= 0x81800000 - 0xFCC and base % 4 == 0):
                violations.append(f"{label}: invalid GObj/Item address")
            if state["entity"]["v"] != gobj or state["kind"]["v"] != item["kind"]:
                violations.append(f"{label}: GObj/kind disagrees with Item bytes")
            owner = item["owner"]
            if owner is not None and (type(owner) is not int or not 0 <= owner < 6):
                violations.append(f"{label}: owner is not a player slot or null")
            for prefix in ("pos", "vel"):
                for axis in "xyz":
                    if not math.isfinite(item_float(state, f"{prefix}.{axis}")):
                        violations.append(f"{label}: {prefix}.{axis} is not finite")
            current[spawn_id] = item
            gobjs.add(gobj)
            bases.add(base)
            before = self.previous.get(spawn_id)
            if before is not None:
                if (int(before["gobj"], 16), int(before["base"], 16), before["kind"]) != (gobj, base, item["kind"]):
                    violations.append(f"{label}: live item changed GObj/Item address or kind")
                elif self.free_laser(before["state"], state):
                    self.checked += 1
                    for axis in "xyz":
                        old = item_float(before["state"], f"pos.{axis}")
                        new = item_float(state, f"pos.{axis}")
                        velocity = item_float(state, f"vel.{axis}")
                        # f32 rounding diagnostic, not a bit-exact physics gate.
                        if not math.isclose(new, old + velocity, rel_tol=1e-6, abs_tol=1e-5):
                            violations.append(f"{label}: pos.{axis} {old:g} -> {new:g} "
                                              f"does not match velocity {velocity:g}")
                        saved = item_float(state, f"laser.prev_pos.{axis}")
                        if not math.isclose(saved, old, rel_tol=1e-6, abs_tol=1e-5):
                            violations.append(f"{label}: laser.prev_pos.{axis} is not previous tick position")
                else:
                    self.skipped += 1
        # Preserve retail order within each event class, never sort by address.
        for event, rows, other in (("despawn", self.previous, current),
                                   ("spawn", current, self.previous)):
            for spawn_id, item in rows.items():
                if spawn_id not in other:
                    self.events.append({"event": event, "ordinal": ordinal,
                                        "tick": record.get("tick", record.get("frame", ordinal)),
                                        "spawn_id": spawn_id, "gobj": item["gobj"],
                                        "kind": item["kind"], "kind_name": item.get("kind_name", "?"),
                                        "initial": ordinal == 0})
        self.previous = current

    @staticmethod
    def free_laser(before: dict, after: dict) -> bool:
        if before["kind"]["v"] != FOX_LASER_KIND or after["kind"]["v"] != FOX_LASER_KIND:
            return False
        stable = ["motion_id", "owner", "facing_dir", *(f"vel.{axis}" for axis in "xyz")]

        def value(state, key):
            field = state[key]
            return field["v"]["bits"] if field["t"] == "f32" else field["v"]

        if any(value(before, key) != value(after, key) for key in stable):
            return False
        # flag32 xN is the Nth MSB (it/types.h:34-64). The common physics proc
        # suppresses movement in hitlag (x9) and while held (x13).
        affected = sum(1 << (31 - bit) for bit in [*range(3, 12), 0x13])
        for state in (before, after):
            if (state["flags"]["v"] & affected or state["ground_or_air"]["v"] != 1
                    or state["env_flags"]["v"]
                    or item_float(state, "hitlag_frames") != 0
                    or state["reflect_gobj"]["v"] or state["atk_victim"]["v"]
                    or item_float(state, "life_timer") <= 1):
                return False
            # The known laser callbacks (symbols.txt); a replaced callback is
            # outside this diagnostic's integration contract.
            if (state["physics_callback"]["v"] != 0x8029C9CC
                    or state["collision_callback"]["v"] != 0x8029C9EC):
                return False
            if any(item_float(state, f"{prefix}.{axis}") != 0
                   for prefix in ("external_vel", "ground_vel", "nudge") for axis in "xyz"):
                return False
        return True


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
    items = ItemValidator()
    count = 0
    for ordinal, record in enumerate(records):
        count += 1
        items.update(record, ordinal, violations)
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
    result = {"records": count, "histogram": dict(sorted(histogram.items())),
              "violations": violations}
    if items.present:
        result.update(item_events=items.events, item_motion_checked=items.checked,
                      item_motion_skipped=items.skipped)
    return result


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("trace", type=Path)
    parser.add_argument("--max-draws", type=int, default=64, metavar="K")
    parser.add_argument("--scripted", action="store_true",
                        help="inputs drive the fighters: skip the unit animation-rate rule")
    parser.add_argument("--items", action="store_true",
                        help="show item spawn/despawn events and motion-check coverage")
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
    if args.items:
        if "item_events" not in result:
            print("items: not recorded (legacy trace)")
        else:
            for event in result["item_events"]:
                initial = " (present at capture start)" if event["initial"] else ""
                print(f"item {event['event']} tick {event['tick']} ordinal {event['ordinal']}: "
                      f"{event['gobj']} id={event['spawn_id']} {event['kind_name']} "
                      f"kind={event['kind']}{initial}")
            events = result["item_events"]
            print(f"items: {sum(e['event'] == 'spawn' for e in events)} spawns; "
                  f"{sum(e['event'] == 'despawn' for e in events)} despawns; "
                  f"motion checked={result['item_motion_checked']}, skipped={result['item_motion_skipped']}")
    for violation in result["violations"]:
        print(violation)
    print(f"{'FAIL' if result['violations'] else 'PASS'}: {len(result['violations'])} violations")
    return int(bool(result["violations"]))


if __name__ == "__main__":
    raise SystemExit(main())
