"""Read-only trace adapter: schema offsets/types -> compact binary for Rust.

No PyYAML dependency. Parse the scalar inline mapping format actually checked
into the harness, and fail on unrecognized input/CPU fields. The generated
schema labels Vec2/float/button arrays unknown; expand their C array types.
"""
import json
import pathlib
import re
import struct
import sys

root = pathlib.Path(sys.argv[1])
def schema(name):
    text = (root / 'harness/schema' / name).read_text()
    return {name: (int(offset, 16), kind) for name, offset, kind in re.findall(
        r'^\s+([\w.]+):\s*\{ offset: (0x[0-9A-Fa-f]+), type: (\w+)', text, re.M)}
generated = schema('fighter.generated.yaml')
canonical = schema('fighter.yaml')
expected = [json.loads(s) for s in (root / 'harness/traces/idle_fd_fox.tick.expected.jsonl').read_text().splitlines()]
raw = [json.loads(s) for s in (root / 'harness/traces/idle_fd_fox.tick.raw.jsonl').read_text().splitlines()]
assert len(expected) == len(raw) == 600
for key, offset in [('input.lstick', 0x620), ('input.cstick', 0x638), ('input.triggers', 0x650),
                    ('input.held_buttons', 0x65c), ('input.pressed_buttons', 0x668), ('input.released_buttons', 0x66c)]:
    assert generated[key][0] == offset
# Generated x675 has the header's stale +674 comment. Check the discrepancy;
# do not change the schema or let it silently alias x674 in the replay.
assert generated['x675'][0] == 0x674
cpu_fields = ['cpu.buttons', 'cpu.lstick_x', 'cpu.lstick_y', 'cpu.type', 'cpu.level', 'cpu.behavior', 'cpu.timer']
assert {k for k in canonical if k.startswith('cpu.')} == set(cpu_fields)
assert not any(k.startswith('input.') for k in canonical), 'extend comparison for new canonical inputs'
formats = {'s8': 'b', 'u8': 'B', 's32': 'i', 'u32': 'I'}
for raw_record, record in zip(raw, expected):
    assert raw_record['frame'] == record['frame']
    assert len(raw_record['fighters']) == 2
    for p, fighter in enumerate(raw_record['fighters']):
        b = bytes.fromhex(fighter['bytes'])
        sys.stdout.buffer.write(b[0x620:0x68c])
        for key in cpu_fields:
            offset, kind = canonical[key]
            value = struct.unpack_from('>' + formats[kind], b, offset)[0]
            assert record['state'][f'p{p}.{key}']['v'] == value
            sys.stdout.buffer.write(struct.pack('>q', value))
        # Relevant external gates and directions, so the caller's idle assumptions
        # are checked against each raw Fighter, not merely defaulted to false.
        offsets = [0x2219, 0x221d, 0x221f, 0x2224, 0x2228, 0x2229]
        sys.stdout.buffer.write(bytes(b[o] for o in offsets))
        for offset in [0x1980, 0x2114, 0x1974, 0x196c]:
            sys.stdout.buffer.write(b[offset:offset+4])
