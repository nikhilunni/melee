# Yoshi data (NTSC-U 1.02)

Lane C loads kind 14 through `ft-yoshi::init::DESCRIPTOR`. The values below
come from the owned local disc and the read-only decomp, checked 2026-09-09.
The scene registry adds one Yoshi entry; shared fighter behavior uses data
and `CharacterCallbacks` overrides. No game data is part of this change.

## Archives and skeleton

| Resource | Bytes | Data bytes | Relocations | Public root |
| --- | ---: | --- | ---: | --- |
| `PlYs.dat` | 287,136 | `0x42380` | 3,963 | `ftDataYoshi`, `0x9318` |
| `PlYsNr.dat` | 294,834 | `0x46420` | 1,739 | `PlyYoshi5K_Share_joint` |
| `PlYsAJ.dat` | 1,493,088 | packed animation archives | — | per-motion figatrees |

`ftYs_Init_*` strings in `ft/kinds/ftYoshi/ftyoshi.c` select costumes
Nr, Re, Bu, Ye, Pi, Aq. Colored joint symbols insert the suffix after
`PlyYoshi5K`. `ftData_Table_Unk0[14]` supplies **314 motion rows**, of which
259 have animations, referencing 197 distinct AJ subarchives. Missing rows
are valid data: in particular, Yoshi has no ordinary Guard hold motion 38.

PlCo's `ftPartsTable[14]` descriptor is at `0xD6B0`, with reverse map
`0xD630` and forward map `0xD678`. It maps **70 joints** to **54 semantic
parts**. The five part-animation group roots are `[28,58,38,38,38]`.
The dynamic-bone descriptor at `ftData.x2C`, `0x25C8`, contains four zero
words: **zero dynamic chains and zero collision spheres**.

| Shared data | Value / words | Source |
| --- | --- | --- |
| Common attributes | `0x333C`, length `0x184` | `ftData.x0` |
| Gravity | `3DBE76C9` | common attributes +5C |
| Terminal velocity | `3FF70A3D` | +60 |
| Weight | `42D80000` (108) | +88 |
| Model scale | `3F866666` (approximately 1.05) | +8C |
| Normal landing interruption lag | `40800000` (4) | +E4 |
| ECB joints | `[38,56,26,14,7,19]` | `ftData.x44` |
| ECB center / ledge X / Y / height | `0,13,10,11` | `ftData.x44+C..18` |
| Translation / shield / held item / feet | `[65,68,43,9,16]` | `ftData.x8+10..14` |
| Wait-choice / squat-choice table | both null | `ftData.x24/x28` |
| Ordinary shield-pose table | null | `ftData.x20->x0` |

A null Wait-choice table means no alternate-idle selection and no RNG draw
at that choice site. It is different from a malformed mandatory pointer.
An absent shield-pose table is also valid because the egg owns its poses.

## Typed special attributes

`YoshiAttributes` reads all **0x138 bytes** through `ftData.ext_attr` at
`0x34C0`. Both overlapping structs in `ftYoshi/types.h` describe this block;
`ftYs_DatAttrs` names Egg Throw fields hidden by padding in
`ftYoshiAttributes`. Each Rust field has its decomp offset in its doc comment.

| Group | Offsets | Contents |
| --- | --- | --- |
| Double jump | +00..08 | signed turn duration, reversal threshold, armor |
| Egg shield | +0C | material animation end frame, populated by OnLoad |
| Egg Lay / captured egg | +10..44 | initial speeds, capture/wobble parameters, duration, mash reduction/animation, exit intangibility and velocity, damage reduction |
| Egg Roll | +48..E8 | durations, acceleration, ground/air speed, steering, rotation, collision/bounce parameters, effect cadence, ending speeds and landing lag |
| Egg Throw | +EC..110 | angle response, charge-dependent speed/spin, spawn offsets |
| Ground Pound | +114..11C | falling speed and star offset |
| Captured hurtbox | +120 | capture hurtbox scale |
| Unnamed tail | +124..137 | two parameters and twelve opaque bytes, retained without invented semantics |

Disc pins: double-jump turn **12 frames**, reversal threshold `3E99999A`,
armor `42F00000` (**120**), Egg Roll duration **360**, Ground Pound fall
speed `C0A00000` (**-5**). Synthetic tests preserve signed values, negative
zero, NaN payloads and a relocated-zero ext_attr pointer, and reject null or
truncated attributes.

## OnLoad and OnDeath

`ftYs_Init_OnLoad` (**8012B99C**) pushes the attributes, sets Fighter
+2226 bit 1 (grounded DownBound capability), and registers Yoshi Egg Throw,
Star and Egg Lay items. All four specials have callbacks; walljump is absent.
Item registration is retained metadata; special bodies and item interactions
remain explicit unsupported paths outside these scenes.

OnLoad also calls `ftYs_Init_8012B6E8` (**8012B6E8**). It selects the first
variant of the first two costume visibility groups, with costume-zero
fallbacks, and freezes those MObj AObjs. Both DObj indices are **[35,34]**
for each of the six costumes. Their material end frames are all **100**;
the archive attribute +0C starts at zero and becomes 100. The test loads
all six costume archives independently. Frozen material indices and requested
shield material frame are retained renderer outputs, like existing TObj/model
selection requests; this does not add mesh rendering.

`ftYs_Init_OnDeath` (**8012B960**) selects model group zero and clears the
Egg Throw handle at Fighter +2238. It preserves the saved Egg Roll model
scale at +222C/+2230/+2234. Reset and saved-boundary tests pin this distinction.

## Shield and double jump

The task's shield labels need correction. The actual `ftYs_MS_*` table is:

| Retail action | Semantic callback family |
| ---: | --- |
| 341 | GuardOn_0: ordinary egg startup |
| 342 | GuardHold |
| 343 | GuardOff |
| 344 | GuardDamage, corresponding to common GuardSetOff |
| 345 | GuardOn_1: Reflect/powershield startup |

Thus **341 is not GuardSetOff**. The recordings use 345 on initial shield
press; spot dodge later enters 341, while the recorded roll returns directly
to 342. Egg hold uses SM_None, frame -1, rest pose, hidden articles, fixed
shield size, intangible body and an egg capsule. Hold release is checked by
Anim; Reflect startup reuses the common Reflect IASA. Escape hooks retain
guard scratch across rolls and spawn twelve shell fragments on egg exit.

`ftYs_JumpAerial_Enter` (**800CBE98**) always enters forward aerial-jump
animation **27**, including the backward-drift scene. Vertical movement comes
from the animation root-motion delta; initial self velocity Y is zero.
Armor is 120 during the jump and clears on state change. A stick reversal
below the strict negative threshold starts twelve turning frames and reverses
facing halfway through. This is `AerialJumpStyle::Yoshi` plus character-owned
entry/Anim hooks, with no shared fighter-kind branch.

## Evidence and limits

All eighteen scenes and their ordered particle RNG ledgers pass. Start and
idle bone tests compare **177,804 local SRT words** across 70 Yoshi and
73 Fox joints: 167,508 over 130 start ticks and 10,296 over eight idle ticks.
Unused Euler W follows the existing oracle exclusion; quaternion W is exact.
There is no rendered matrix capture, so zero matrix words are claimed.
Five additional 300-tick raw-ledger tests check shield health, guard timers
and double-jump armor, beyond the 49-key scene schema.

Egg shield damage/break, hits against the egg capsule, armor damage resolution,
delayed egg powershield, grabs, specials and item interactions remain explicit
unsupported branches. Yoshi particle dumps are not newly compared field by
field; ordered particle draws and scene seeds are gated, and existing
`hsd-particle` field oracles still run. Commands, changed files, arithmetic
audits and final results are in
[`M4_YOSHI.md`](../crates/melee-ft/src/fighter/M4_YOSHI.md).
