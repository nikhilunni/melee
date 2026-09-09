# Falco data (NTSC-U 1.02)

Falco is kind 22 (`FTKIND_FALCO`). `ft-falco::init::DESCRIPTOR` owns the
archive names, root symbol, costumes and table counts. Values below were read
from the local owned disc on 2026-09-09.

## Archives and shared data

| Resource | Bytes | Data bytes | Relocations | Public root |
| --- | ---: | ---: | ---: | --- |
| `PlFc.dat` | 237784 | `0x36780` | 3657 | `ftDataFalco`, `0x99D8` |
| `PlFcNr.dat` | 243994 | `0x3A0A0` | 1541 | `PlyFalco5K_Share_joint`, `0x16EA8` |
| `PlFcAJ.dat` | 1495328 | packed animation archives | — | per-motion figatrees |

The neutral costume also exports `PlyFalco5K_Share_matanim_joint` at
`0x35D78`. `ftfalco.c:412-438` supplies the Nr, Re, Bu and Gr costume strings.
`ftData_Table_Unk0[22]` (`ftdata.c:255-261`) supplies 327 animation rows.
`ftparts.c:393-451,695-708` consumes PlCo's part mapping: Falco's descriptor
at `0xDB48` has 67 joints, reverse table `0xDACC`, and 54 semantic forward
entries at `0xDB10`. Joint count and semantic part count are distinct.
The five part-animation groups at `ftData.x1C` have roots 26, 52, 39, 39, 39.

| Data | Falco value | Source |
| --- | --- | --- |
| Common attributes | `0x37C4`, length `0x184` | `ftData.x0` |
| Gravity | `3E2E147B` (about 0.17) | common +5C |
| Terminal velocity | `40466666` (about 3.1) | common +60 |
| Weight | `42A00000` (80) | common +88 |
| Model scale | `3F8CCCCD` (about 1.1) | common +8C |
| Landing interruption lag | `40800000` (4) | common +E4 |
| ECB joints | `[39,49,23,13,7,4]` | `ftData.x44`, `0x76D0` |
| ECB center / ledge X / Y / height | `0,11,13,9` | x44 +C..18 |
| Translation / shield / held item / feet | `[61,65,39,9,15]` | x8 +10..14 |
| Dynamic chains and collision spheres | both zero, both pointers null | x2C, `0x2A50` |

Falco has **no dynamic-bone set**. The shared descriptor loader supplies an
empty set; OnLoad does not install a special one. The bone oracle checks all
67 joints, plus all 73 Fox opponent joints.

## Special attributes and callbacks

The task's `ftFalco/types.h` does not exist in the pinned decomp. Instead,
`ftfalco.c:17` includes `ftFox/types.h`, and `ftFc_Init_OnLoad` calls
`ftFx_Init_OnLoadForFalco` (800E576C), which copies `ftFox_DatAttrs`.
The complete **0xD4-byte** block is at `ftData.ext_attr = 0x3948`.

The existing typed reader was moved from `ft-fox` into
`melee-ft::desc::fox_attributes`; both character crates select their own
`ftData` root and call that reader. There is no character-to-character
dependency or copied special reader. Every field retains its decomp offset
comment, signed integer type, float bits and byte flags.

| Concept | Offsets | Falco pins |
| --- | --- | --- |
| Blaster | +00..20 | speed `40A00000` (5), laser kind `0x37`, gun kind `0x4B` |
| Illusion / Phantasm | +24..50 | startup gravity delay `41700000` (15) |
| Fire Fox / Fire Bird | +54..94 | travel speed `40866666` |
| Reflector | +98..AC | signed gravity delay 4 |
| ReflectDesc | +B0..D0, padding to D4 | joint 1, max damage 50, radius `41080000` |

`ftFc_Init_OnLoad` (80149CC4) enables walljump and registers item resources
0 and 1 with the archive's laser/gun IDs, and resource **3** as Falco Phantasm
(`0x39`). Fox instead registers resource 2 as Fox Illusion. These registrations
do not spawn items. All four special capabilities are present in `ftdata.c`.
`ftFc_Init_OnDeath` (80149ACC) clears `u.fx.x222C_blasterGObj` and resets
model group 0 to selection 0. `ftFc_Init_LoadSpecialAttrs` delegates to the
same unscaled Fox-layout copy.

Falco takes the existing Basic aerial-jump default and ordinary guard/escape
hooks; no character-specific override is required by these scenes. Special
move bodies, combat, item interactions and unused character variants retain
explicit unsupported boundaries.

## Oracle contract

Both idle/start gates cover 600 ticks and all 49 keys. Sixteen movement gates
cover 300 ticks each, except the three ledge scenes at 420 ticks. Every scene
also compares ordered particle RNG call sites with its supplied ledger.

The SRT oracle runs the actual mixed-character scheduler, with produced RNG:
164080 words across 130 start ticks and 10080 across eight idle ticks, for
**174160 words** total. Only unused Euler rotation W is excluded, exactly as
in the existing Marth/Fox tests; quaternion W is compared. No rendered matrix
oracle is claimed. Particle dumps for Falco idle/start were not separately
replayed field-by-field; the ordered ledgers and all existing particle dump
regressions are tested.

Start P1 is 322 at tick 0, 323 at 6, 324 at 35, Fall 29 at 65, Landing 42
at 77, Wait 14 at 107, matching the notes. In this mixed capture the P2 Fox
lands at **80**, after its staggered entry (11/40/70); the notes' Fox tick 75
is not this opponent's landing tick. The re-recorded `airjumpb_fd_falco`
trace is present. Idle has only the existing idle RNG sites.

See [M4_FALCO.md](PORT_NOTES/M4_FALCO.md) for the shared
queue-flush correction, exact commands, results and changed files.
