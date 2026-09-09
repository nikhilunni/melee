# Captain Falcon data (NTSC-U 1.02)

`ft-captain` owns Captain Falcon's attributes and `CharacterCallbacks`.
`DESCRIPTOR` selects `ftDataCaptain`, kind 2 (`FTKIND_CAPTAIN`), and the
character archives. All values below were read from the owned local disc on
2026-09-09. Shared movement uses the common attributes and existing hooks.

## Archives and tables

| Resource | Bytes | Data bytes | Relocations | Public root |
| --- | ---: | --- | ---: | --- |
| `PlCa.dat` | 161726 | `0x24D60` | 2698 | `ftDataCaptain`, `0x9A04` |
| `PlCaNr.dat` | 525805 | `0x7EC40` | 1627 | `PlyCaptain5K_Share_joint`, `0x1F5E8` |
| `PlCaAJ.dat` | 1633440 | packed animation archives | — | per-motion figatrees |

`ftData_Table_Unk0[2]` (`ftdata.c:255`) supplies **318 animation rows**;
275 have a non-null animation-name pointer. `ftCa_Init_CostumeStrings` lists
Nr, Gy, Re, Wh, Gr, Bu in that order. The root is `PlyCaptain5K_Share_joint`
for Nr; other colors insert their suffix immediately after `5K`.

The red costume's retail filename is **`PlCaRe.`**, including the final dot.
`lbFileGetFullName` (`lbfile.c:74-81`) resolves it to `PlCaRe.usd` for the US
language setting and `PlCaRe.dat` otherwise. The descriptor uses `.usd` for
NTSC-U English. These scenarios use neutral costume 0; general language
selection is outside this port.

PlCo `ftPartsTable[2]` is at `0xD04C`; its reverse/forward maps are at
`0xCFD4` / `0xD014`. It contains **63 skeleton joints** and **54 semantic
parts**. `ftparts.c:400,451,697` distinguishes the skeleton count from the
forward map. The forward map includes the unnamed part 53.

The `ftData.x1C` table at `0x29DC` contains **three** part-animation groups,
rooted at joints 25, 47, 39. Five is the shared runtime slot capacity.
`ftData.x2C` points to `0x29EC`, containing zero chains and zero colliders;
Captain Falcon has **no dynamic bones**. Both facts use the existing readers.

| Shared fighter data | Value | Source |
| --- | --- | --- |
| Common attributes | `0x3754`, length `0x184` | `ftData.x0` |
| Gravity | `3E051EB8` (about 0.13) | common +5C |
| Terminal velocity | `4039999A` (about 2.9) | common +60 |
| Weight | `42D00000` (104) | common +88 |
| Model scale | `3F7851EC` (about 0.97) | common +8C |
| Normal landing interruption lag | `40800000` (4) | common +E4 |
| Trophy scale | `3F8CCCCD` (about 1.1) | common +110 |
| ECB joints | `[39,47,25,14,8,4]` | `ftData.x44`, `0x7910` |
| ECB center / ledge X / Y / height | `0,9,17,11` | `ftData.x44+C..18` |
| Translation / shield / held item / foot joints | `[57,61,39,10,16]` | `ftData.x8+10..14` |
| Wait choices | `(2,70), (3,20), (4,10), (-1,-1)` | `ftData.x24` |

The final Wait pair is the sentinel. Motion **4 (Wait3)** exposed the loader's
fixed Wait1/Wait2 list. The loader now includes every non-sentinel motion in
the character's Wait and optional SquatWait tables, together with its script.
This adds no runtime character check or change to random-choice arithmetic.

## Special attributes and callbacks

`ftCaptain_DatAttrs` is **0x8C bytes**, reached through `ftData.ext_attr` at
`0x38D8`. `CaptainAttributes` preserves its float payloads and signed/unsigned
integers, grouped by move and documented with each decomp offset.

| Group | Offsets | Fields |
| --- | --- | --- |
| Falcon Punch | +00..10 | stick thresholds, maximum angle, aerial speed, momentum multiplier |
| Raptor Boost | +14..3C | ground-hit speed multiplier, gravity, terminal speed, six unused floats, miss/hit landing lag |
| Falcon Dive | +40..64 | air acceleration/speed multipliers, freefall mobility, landing lag, two unused floats, reversal threshold, initial command value, catch gravity, signed initial air counter |
| Falcon Kick | +68..88 | unknown float, unused unsigned word, flame angle, hit slowdown multiplier and signed counter limit, ending animation rates and traction multipliers |

The +68 field is `float` in the header even though its disc bits are
`00000002`; it remains that exact subnormal payload. +64/+78 are signed
integers, +6C unsigned. Unknown fields retain explicit names and comments;
no invented special-move behavior uses them. The reader rejects null links,
missing symbols and truncated blocks, and accepts a relocated pointer to zero.

`ftCa_Init_OnLoad` (**800E2AEC**) copies the block and enables walljumping.
It registers **no items**. All four ground/air special entries are present in
`ftdata.c`. `ftCa_Init_LoadSpecialAttrs` (**800E2B40**) copies without scaling;
`OnLoadForGanon` (**800E2AAC**) shares the layout, not gameplay support.
`ftCa_Init_OnDeath` (**800E2888**) resets model group 0, then clears the Raptor
Boost lunge/start GFX flags at Fighter +2230/+222C. The saved-state hook imports
those two flags independently. Basic double jump, ordinary shield and ordinary
escape behavior use the existing `CharacterCallbacks` defaults. No new hook
or shared kind check was needed.

## Capture findings and verification

The supplied start sequence is confirmed: ticks 0:322, 6:323, 35:324,
65:29, 78:42, 108:14. The sixteen movement scripts pass unchanged, including
the supplied ledge drift-back and aerial-jump timing.

One additional boundary condition was absent from the task notes: the idle
save is paused at PC **80326290** inside `cosf`, called from **8039EC80**
in the particle sphere emitter, under scheduler link 15. The current particle
has not been allocated yet. The importer resumes it from saved stack operands;
tick zero's remaining four draws are the existing primary-color sites.
See [M4_FALCON.md](PORT_NOTES/M4_FALCON.md) for the
instruction audit, exact commands and limits.

Start and idle bone oracles compare local SRT at every tick boundary: **159398**
words over 130 start ticks and **9860** over eight idle ticks, across 63 Falcon
and 73 Fox bones. Quaternion W is compared when enabled; unused Euler W follows
the existing Marth contract. No rendered matrix capture is claimed.
The full idle particle oracle compares **858792 fields over 600 ticks**, with
no exclusions. All eighteen ordered particle RNG ledgers and 49-key scene
gates pass. Start particle fields were not separately replayed.

Special moves, walljump transitions, items and combat branches not exercised
by these scenes remain explicit unsupported paths. No protected data changed,
no Dolphin was run, and no commits were made.
