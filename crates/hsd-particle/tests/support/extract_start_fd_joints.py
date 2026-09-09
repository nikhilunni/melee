"""Print only effect-owned matrix inputs from the local oracle metadata.

Run from the repository root, redirect to a scratch file, then compare it
with start_fd_joints.json. No particle/generator values become replay inputs.
"""
import json
from pathlib import Path

root = Path(__file__).resolve().parents[4]
source = root / "harness/traces/start_fd_fox.particles.jsonl.meta.jsonl"
# Pointers are observation identities, with disjoint windows for allocator reuse.
windows = [
    (0, 0x80C378E0, 1, 600),
    (1, 0x80D417E0, 6, 71),
    (2, 0x80D41CA0, 6, 71),
    (3, 0x80D48FA0, 11, 71),
    (4, 0x80D49420, 11, 71),
    (5, 0x80D49420, 75, 97),
    (6, 0x80D48FA0, 80, 102),
]
last = {}
rows = []
with source.open() as stream:
    for line in stream:
        record = json.loads(line)
        tick = record["frame"]
        joints = {
            joint["pointer"]: joint["fields"]["matrix"]
            for joint in record["particles"]["joints"]
        }
        for identity, pointer, start, end in windows:
            if start <= tick < end and pointer in joints:
                words = joints[pointer]
                if last.get(identity) != words:
                    rows.append([tick, identity, words])
                    last[identity] = words
print("[\n" + ",\n".join(json.dumps(row) for row in rows) + "\n]")
