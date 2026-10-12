# Kirby data (NTSC-U 1.02)

`ft-kirby` owns Kirby's callbacks and descriptor; shared gameplay has no
Kirby kind branch. The local archives were inspected on 2026-10-11. Kirby is
internal kind 4, `FTKIND_KIRBY` / `FighterKind::Kirby`.

## Archives and model

| Resource | Bytes | Metadata |
| --- | ---: | --- |
| `PlKb.dat` | 198866 | `ftDataKirby` at data offset `0xE250` |
| `PlKbNr.dat` | 283994 | `PlyKirby5K_Share_joint`, `PlyKirby5K_Share_matanim_joint` |
| `PlKbAJ.dat` | 1721600 | 479 animation-table rows (`ftData_Table_Unk0[4]`) |
| `EfKbData.dat` | 65702 | `effKirbyDataTable`: efAsync bank 5, nine model rows (`0x1388..0x1390`) |

Costumes, in `ftKb_Init_CostumeStrings` order: Nr, Ye, Bu, Re, Gr, Wh; a
coloured skeleton symbol inserts the colour after `PlyKirby5K`.

Copy abilities load more archives, none of them read yet:
`PlKbCp<Xx>.dat` (`ftDataKirbyCopy<Name>`, one per copied kind,
`ftKb_Init_803CA9D0`), whole replacement models for the four abilities that
change Kirby's body (`PlKb<Colour>Cp{Dk,Fc,Mt,Pr}.dat`, and `PlKbNrCpGw.dat`),
and per-ability effect files `EfKb{Ca,Dk,Fe,Fx,Gn,Ic,Kp,Lg,Mr,Ms,Pc,Pk,Ss,Zd}.dat`.

## Parts: 59 joints, 13 of them empty

PlCo's part table for kind 4 has **59 joints** and 54 semantic parts, but the
costume's own tree has **46**. `Fighter_804D6540[4]` lists thirteen
conditional parts the costume leaves empty for the hats:

| Part | Parent | Insertion type | Source joint |
| ---: | ---: | ---: | ---: |
| 7 | 6 | 1 | 7 |
| 8, 9, 10 | 7, 8, 9 | 0 | 8, 9, 10 |
| 15, 16 | 14, 15 | 3 | 11, 12 |
| 28 | 12 | 2 | 22 |
| 29..34 | 28..33 | 0 | 23..28 |

`ftParts_SetupParts` numbers the costume's joints in pre-order and skips
those parts (`parts[part].joint = NULL`), so the joints after them keep the
indices the part table, `ftData`'s bone tables and the animations use
(`part_to_joint`: the legs are 45..55, parts 52 and 53 are joints 57 and 58).
The port numbers a skeleton's joints by pre-order alone, so each empty part
takes a stand-in leaf joint at its pre-order index
(`melee_ft::desc::fill_empty_conditional_parts`): no display object, the
identity pose, and no other joint's matrix depends on it. A copy ability
grafts the hat's joints into these parts (`ftParts_800753D4`), which is
stage-3 work.

`ftData.x1C` holds **three** part-animation groups (roots 39, 44 and 12).

## Common attributes

| Attribute | Value | Offset within common attributes (`0x4CC8`) |
| --- | --- | --- |
| Maximum total jumps | 6 | +58, integer |
| Gravity | `3DA3D70A` (about 0.08) | +5C |
| Terminal speed | `3FCCCCCD` (about 1.6) | +60 |
| Weight | `428C0000` (70) | +88 |
| Model scale | `3F6B851F` (about 0.92) | +8C |
| Landing interruption lag | `40800000` (4) | +E4 |

## Special attributes

`ftData.ext_attr` points to **0x4E6C**, length **0x424** (`ftKb_DatAttrs`,
`ftKirby/types.h`). The block is too large for the character payload's
inline 1024 bytes, so `Kirby` shares it behind an `Arc`.

| Group | Offsets | Read so far |
| --- | --- | --- |
| Shared multijump (`Fighter_x2D0_t`) | +00..30 | yes |
| Inhale, swallow, spit, the ability star | +38..C8 | no |
| Hammer | +CC..D0 | no |
| Final Cutter | +D4..E8 | yes (+E8 has no reader) |
| Stone | +EC..114 | no |
| One group per copied kind | +118..3E8 | no |
| Peach's `AbsorbDesc`, Zelda's `ReflectDesc` | +3EC, +400 | no |

The multijump prefix on the disc: 12 turn frames, reverse threshold
`3E99999A` (0.3), horizontal impulse 0.5, drift acceleration and speed
multipliers 0.8, vertical impulses 2.0, 2.0, 1.73, 1.56, 1.33, five states,
first actions **341** (bare) and **346** (helmeted). As with Jigglypuff, the
header declares +00 and +28..30 unsigned and the impulses floats;
`ftCo_800D74A4` reads them so (`lwz` and `lfs`).

Final Cutter on the disc: the rise's upward speed times `3F666666` (0.9,
+D4), PlCo's aerial drift acceleration times 8 (+D8), the beam 3.5 ahead
and 0 above TopN (+DC, +E0; the code then lowers it by 1), a stick past
`3E4CCCCD` (0.2, +E4) turns Kirby round at the start, and +E8 is 0.

`ftData.x48_items[0]` (article at 0xE17C) is the Cutter beam,
`It_Kind_Kirby_CBeam` (0x32); its special attributes at 0x7B2C
(`itKirbyCutterBeamAttributes`): speed 4.0, an unread word 0, lifetime 25
frames, deceleration 0.1 per frame. Items [1]..[3] (the hammer and two
more) are not read.

Stone words pinned for the later port: at most 150 and at least 18 ticks in
the rock, 38 HP (+108).

## Motion table

`ftKb_Init_MotionStateTable` (0x803C8368) has **203 rows**, 341..543:

| Rows | States |
| --- | --- |
| 341..345, 346..350 | JumpAerialF1..F5 and the helmeted JumpAerialF1Met..F5Met (submotions 295..304) |
| 351, 352 | AttackDash, AttackDashAir (both play `ftCo_SM_AttackDash`, 52) |
| 353..382 | Inhale: start, loop, end, capture, the mouthful's walk, jumps and turn, swallow, spit; ground then air |
| 383, 384 | Hammer |
| 385..392 | Final Cutter |
| 393..398 | Stone |
| 399..543 | The copied neutral specials, one group per kind |

Move ids (the row's third word): the multijumps are `FtMoveId_Default`,
the dash attack 6, Inhale 0x12, Hammer 0x13, Final Cutter 0x14, Stone 0x15,
and each copied special its own id from 0x16 to 0x2F, which the stale-move
queue will need as `StaleMove` variants.

## Effects

| Request | What | Port |
| --- | --- | --- |
| efAlt 0x495 | the dash attack's flame: `efLib_Create_Attach_Scale(0x138B)` turned to the facing, from the script (efAsync kind 3) | `melee-ef` `kirby.rs` |
| efAlt 0x49E | the bump star: `efLib_CreateGenerator(0x206, pos)` (efAsync kind 2 at TopN plus an ECB offset) | `EffectRequest::Graphics` with id 0x206 |
| efAlt 0x49D | a rapid jab fist: `efLib_Create_Attach_Pos(0x1390)` at the script's offset, scaled as the fighter root, turned to the facing and tilted by the command's X range in 256ths of a degree; `efLib_Cb_SetJObjOffsetZ` moves its child 2 forward per update (efAsync kind 6) | `melee-ef` `kirby.rs` |
| efAlt 0x494 | Final Cutter's blade: `efLib_Create_AttachChild(0x1388)` and `efLib_Create_Attach(0x1389)` on `fp->parts[44]`, `efLib_Create_Attach(0x138A)` on `fp->parts[1]`, each with a lifetime of 0x41 and `efLib_Cb_SetRotYAndTransition` (the last two turn to the fighter's facing; all loop frames 60..65 and play out from 65 once `efLib_SetFlags(gobj, 1)` marks them); efSync from both entries, so sync-loaded | `melee-ef` `kirby.rs`; AttachChild's orientation constraint is not applied (drawn pose only) |
| efAlt 0x49A | the Inhale's wind: `efLib_CreateGenerator_AppSRT_SetFacingDir(0x138B)` | `melee-ef` `kirby.rs` |
| efAlt 0x496..0x499, 0x49B, 0x49C | Stone, hammer and swallow models and generators | not ported |
