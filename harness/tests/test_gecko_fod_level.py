"""The constructed level-Fountain codes, built from a made-up spawn block."""
import pytest

import gecko_fod_level as level

NOP = 0x60000000
BLR = 0x4E800020


def block(body: list[int]) -> list[str]:
    words = [0xC216E510, len(body) // 2, *body]
    return [f"{words[i]:08X} {words[i + 1]:08X}" for i in range(0, len(words), 2)]


def body_of(lines: list[str]) -> list[int]:
    return [int(part, 16) for line in lines[1:] for part in line.split()]


# Not the real code: a forward `beq` and `bl` over the facing store, a
# backward `b` before it, the store, an epilogue, the Fountain rows, an exit.
BEFORE = [0x41820024,  # beq +0x24: word 9, past the epilogue's first word
          0x48000021,  # bl +0x20: word 9 too
          NOP,
          0x4BFFFFFC]  # b -4: stays before the store
AFTER = [BLR, NOP, *level.FOUNTAIN_ROWS, NOP, 0x00000000]
BODY = [*BEFORE, *level.FACING_STORE, *AFTER]


def test_entry_code_goes_after_the_facing_store_and_branches_across_it_move():
    out = body_of(level.spawn_code(block(BODY)))
    at = len(BEFORE) + len(level.FACING_STORE) - 1
    assert out[at:at + len(level.ENTRY_FRAMES)] == level.ENTRY_FRAMES
    assert out[:len(BEFORE)] == [0x4182003C, 0x48000039, NOP, 0x4BFFFFFC]
    assert len(out) == len(BODY) + len(level.ENTRY_FRAMES)


def test_only_the_first_two_fountain_rows_become_level():
    out = body_of(level.spawn_code(block(BODY)))
    row = out.index(0x00000002)
    assert out[row:row + 5] == [0x00000002, 0xC2250000, level.LEVEL, 0x42250000, level.LEVEL]


def test_a_block_that_is_not_the_spawn_code_is_refused():
    with pytest.raises(SystemExit):
        level.spawn_code(block([NOP, 0x00000000]))


@pytest.mark.parametrize("word, expected", [
    (0x48000009, 8),           # bl .+8
    (0x4BFFFFFC, -4),          # b .-4
    (0x4182FFF0, -16),         # beq .-16
    (0x48000002, None),        # ba: absolute
    (0x7C0802A6, None),        # mflr r0
])
def test_relative_branch_displacements(word, expected):
    assert level.displacement(word) == expected


def test_the_entry_code_alone_keeps_every_row():
    plain = body_of(level.spawn_code(block(BODY), level_fountain=False))
    leveled = body_of(level.spawn_code(block(BODY)))
    # Only the two Fountain of Dreams heights differ from the level code.
    assert [(a, b) for a, b in zip(plain, leveled) if a != b] \
        == [(0x41A80000, level.LEVEL), (0x41D80000, level.LEVEL)]
