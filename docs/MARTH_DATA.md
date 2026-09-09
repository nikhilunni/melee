# Marth data (NTSC-U 1.02)

M4-T8 loads Marth from the owned extracted disc, with no character-specific
branch in shared action-state code. `ft-mars::init::DESCRIPTOR` supplies the
archive names, root symbol, animation count, costume symbols and part counts.
Numbers below were read from the local archives on 2026-09-09.

## Archives and tables

| Resource | Bytes | Data bytes | Relocations | Public root |
| --- | ---: | --- | ---: | --- |
| `PlMs.dat` | 175,023 | `0x27880` | 3,263 | `ftDataMars`, `0x9D6C` |
| `PlMsNr.dat` | 594,960 | `0x8F430` | 2,015 | `PlyMars5K_Share_joint`, `0x1F768` |
| `PlMsAJ.dat` | 1,942,752 | packed animation archives | — | per-motion figatrees |

The neutral costume also exports `PlyMars5K_Share_matanim_joint` at `0x65AB0`.
Costume strings in `ft/kinds/ftMars/ftmars.c` select Nr, Re, Gr, Bk, Wh;
the latter four joint symbols insert the color suffix after `PlyMars5K`.
`ftData_Table_Unk0[18]` in `ftdata.c:255` supplies **327 animation rows**;
271 contain animations and reference 212 distinct subarchives.

The skeleton has **90 joints**. PlCo's `ftPartsTable[18]` descriptor is at
`0xD750`: reverse map `0xD6BC`, forward map `0xD718`. The forward map covers
**54 semantic parts**, including unnamed part 53. Joint count and forward
part count are distinct. `ftparts.c` obtains the table from PlCo; there is
no separately declared Marth forward-count constant in that C file.

Marth stores only **three part-animation groups** at `ftData.x1C`; their
roots are 31, 73 and 60. Five is the runtime slot capacity, not the archive
array length. `ftAnim_800707B0` and related functions dereference only active
slots. The descriptor supplies the used prefix; slots 3 and 4 remain absent.

| Shared fighter data | Marth value / words | Source |
| --- | --- | --- |
| Common attributes | `0x3724`, length `0x184` | `ftData.x0` |
| Gravity | `3DAE147B` (approximately 0.085) | common attributes +5C |
| Terminal velocity | `400CCCCD` (approximately 2.2) | +60 |
| Weight | `42AE0000` (87) | +88 |
| Model scale | `3F933333` (approximately 1.15) | +8C |
| Normal landing interruption lag | `40800000` (4) | +E4 |
| Trophy scale | `3F666666` (approximately 0.9) | +110 |
| ECB joints | `[60,70,70,14,7,4]` | `ftData.x44` |
| ECB center / ledge X / Y / height | `0,12,17,11` | `ftData.x44+C..18` |
| Translation / shield / held item / foot joints | `[40,88,60,9,16]` | `ftData.x8+10..14` |
| Dynamic roots / chain lengths | `(55,4), (45,4), (50,4)` | `ftData.x2C` |
| Dynamic collision spheres | zero, null pointer | `ftData.x2C+8/+C` |

The 12 dynamic joints use the existing `ftdynamics` / `lb_8001044C` solver.
No new spring arithmetic or character hook was needed. Initial heap springs,
SRT, matrices, locks and multipliers are imported from the saved boundary.
The oracle checks the entire skeleton, including all three chains.

## Special attributes

`MarsAttributes` is **0x98 bytes**, reached through `ftData.ext_attr` at
`0x38A8`. The Rust reader follows `ft/kinds/ftMars/types.h`, naming groups
from their consumers in `ftmarsspecial{n,s,hi,lw}.c` and `ftafterimage.c`.

| Group | Offsets | Fields |
| --- | --- | --- |
| Shield Breaker | +00..10 | signed maximum charge levels (30 ticks each), base damage, damage per level; momentum divisor and friction |
| Dancing Blade | +14..24 | momentum divisor, air friction, first-air-use vertical speed, fall acceleration and terminal velocity |
| Dolphin Slash | +28..48 | freefall mobility, landing lag, reversal/angle thresholds, maximum angle, startup/ending momentum, gravity and terminal velocity |
| Counter | +4C..60 | momentum divisor, air friction, gravity, terminal velocity, damage and collision multipliers |
| Counter volume | +64..74 | signed joint, Vec3 offset, radius (`AbsorbDesc`) |
| Sword trail | +78..94 | two interpolation parameters, nine appearance bytes, signed joint and two endpoint values |

Sword bytes +89..8B are padding. The appearance bytes retain their raw values;
their complete rendering semantics are not yet named in the decomp.
The loader preserves signed values, negative zero and NaN payloads, distinguishes
a relocated zero from a null pointer, and rejects truncated blocks.

Disc pins in `ft-mars/tests/attributes.rs`: charge parameters `4,7,5`,
Shield Breaker momentum divisor `3FA00000`, Dancing Blade gravity `3D75C28F`,
Dolphin Slash mobility `3ECCCCCD`, Counter momentum divisor `40000000`,
Counter joint `3`, radius `41080000`, and sword joint `75`.

## Character hooks

`ftMs_Init_OnLoad` (801364AC) only pushes attributes: no item registration and
no walljump enable. The four special callbacks are present in `ftdata.c`.
Roy uses the same layout through `ftMs_Init_OnLoadForRoy` (80136474) and shares
Marth's special-state callbacks; Roy gameplay is not enabled by this task.
`ftMs_Init_LoadSpecialAttrs` (801364E8) copies the same block without scaling.

`ftMs_Init_OnDeath` (80136258) resets model groups 0 and 1 and Fighter +222C.
The Rust field `side_special_boost_used` follows `ftmarsspecials.c:56-60`.
`CharacterCallbacks::on_landing` resets that flag, matching
`ftCo_Landing_Enter` (800D5AEC). Marth's unported guard model/offset behavior
is an explicit hook in `ft-mars`; shared code no longer checks Marth's kind.

## Verification and limitations

The two scenario gates match all 600 ticks and 49 keys. The Marth bone oracle
runs in `melee-sim::frame::marth_bones` so it exercises the mixed scene's real
scheduler, effects and produced RNG, with no per-tick external RNG seeds.
It matches 190,936 SRT words over 130 start ticks and 11,736 over eight idle
ticks, across all 90 Marth and 73 Fox bones. Quaternion W is compared when
quaternion mode is active; unused Euler W is excluded under the existing Fox
oracle contract. **Zero rendered matrix words** are compared: Marth has no
VI-aligned rendered capture. Tick-dump matrix caches are not rendered oracles.

Special moves, sword-trail rendering, items, combat and Marth shield behavior
remain outside the two idle/start scenes. No game files, captures or scenarios
were modified, no Dolphin was run, and nothing was committed. See
[`M4_MARTH.md`](PORT_NOTES/M4_MARTH.md) for commands,
scene-boundary corrections and the complete generalization list.
