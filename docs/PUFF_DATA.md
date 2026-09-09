# Jigglypuff data (NTSC-U 1.02)

`ft-purin` owns the Jigglypuff callbacks and descriptor; shared gameplay has
no Jigglypuff kind branch. The local owned archives were inspected on 2026-09-09.

## Archives and model

| Resource | Bytes | Metadata |
| --- | ---: | --- |
| `PlPr.dat` | 114528 | `ftDataPurin` at data offset `0x9094` |
| `PlPrNr.dat` | 244518 | `PlyPurin5K_Share_joint` |
| `PlPrAJ.dat` | 1087072 | 327 animation-table rows |

`ftPr_Init_*` in `ft/kinds/ftPurin/ftpurin.c` supplies costumes Nr, Re, Bu,
Gr, Ye; colored skeleton symbols insert the color after `PlyPurin5K`.
`ftData_Table_Unk0[15]` supplies the animation count. Jigglypuff is internal
kind 15, `FTKIND_PURIN` / `FighterKind::Purin`.

The PlCo part table consumed by `ftparts.c` is at `0xD884`, with reverse map
`0xD818`, forward map `0xD84C`: **50 joints and 54 semantic parts**. These
counts are distinct. `ftData.x1C` has **two** part-animation groups, roots
35 and 13; the runtime capacity of five is not the archive array length.
`ftData.x2C` has one dynamic chain rooted at **joint 7**, with **three**
spring nodes, and no collision spheres. The existing dynamic solver handles it.

| Common attribute | Value | Offset within common attributes (`0x389C`) |
| --- | --- | --- |
| Maximum total jumps | 6 | +58, integer |
| Gravity | `3D83126F` (about 0.064) | +5C |
| Terminal speed | `3FA66666` (about 1.3) | +60 |
| Weight | `42700000` (60) | +88 |
| Model scale | `3F70A3D7` (about 0.94) | +8C |
| Landing interruption lag | `40800000` (4) | +E4 |
| Trophy scale | `3F59999A` (about 0.85) | +110 |

## Special attributes

`ftData.ext_attr` points to **0x3A20**, length **0x100**. The reader follows
that relocated pointer, including a valid relocated offset zero. Signed fields,
negative zero and NaN payloads are preserved; null pointers and truncated blocks
are rejected by the attribute tests.

| Group | Offsets | Interpretation |
| --- | --- | --- |
| Shared multijump | +00..30 | turn duration/threshold, horizontal impulse, drift multipliers, five vertical impulses, state count, ordinary/helmet action bases |
| Rollout | +34..D8 | duration and hit cost; gravity, speed/rotation/charge/damage parameters, rebounds, steering, landing lag |
| Pound | +DC..F4 | stick-angle limits, angle, air launch speed and speed decay |
| Unused words | +E8/+EC | `UNK_T` in the header, no consumers or archive relocations; retained as words |

Unconsumed padding (+48, +60..64, +B0, +F8..FC) is not given invented gameplay
semantics. Sing and Rest use subaction/common data, with no extra attribute
fields consumed by their current C bodies.

**Header correction:** `ftPurin/types.h` calls +00 a float and several +14..24
words integers. `ftPr_Init_OnLoad` aliases this prefix through `Fighter_x2D0_t`
(`ft/types.h:1170`), whose types agree with the retail instructions: `lwz` for
the turn count and action IDs; `lfs` for all five jump impulses. The reader uses
those consumer types rather than converting the misdeclared words.

The disc pins are: 12 turn frames, threshold `3E99999A`, horizontal impulse
`3F000000`, both drift multipliers `3F4CCCCD`, five states starting at 341,
helmet family -1. Vertical impulses F1..F5 are
`3FD33333, 3FCB851F, 3FBC28F6, 3FAE147B, 3FA00000`.
Submotions 295..299 come from `ftPr_Init_MotionStateTable` and `forward.h`.
Jump availability uses the common **max_jumps = 6**, not a fixed two-jump limit.

## Initialization and hooks

`ftPr_Init_OnLoad` (8013C67C) pushes attributes, enables multijumps and aliases
the shared jump prefix. It registers no item kinds and does not enable walljump.
All four special callback tables are present. `ftPr_Init_OnDeath` (8013C318)
resets model group 0 to selection 0.

The neutral costume takes the null-accessory branch of `ftPr_Init_8013C360`.
Colored costumes require a separate hat skeleton, visibility and renderer
callbacks and explicitly stop at that unported hook. Special-move entry remains
explicitly unsupported. Ordinary shields and rolls need no override.

The crate overrides `aerial_jump_style`, `multi_jump_attributes`, and
`multi_jump_animation`; the shared `multi_jump_family` defaults to the ordinary
family and provides the boundary for Kirby's helmet selection. Kirby gameplay
itself is not enabled.

See [M4_PUFF.md](PORT_NOTES/M4_PUFF.md) for exact validation
commands, save-boundary corrections, changed files and limits.
