# The disc, extracted

What `harness/extract_fst.py` found on the verified NTSC 1.02 image
(`docs/ISO.md`), and what `crates/hsd-archive/tests/real_dat.rs` measured
when the archive parser first met real data (2026-09-08). Every number here
is asserted by one of those tests; if a number changes, the code changed.

```sh
cd harness && uv run python extract_fst.py roms/GALE01.iso            # -> roms/sys, roms/files
cd harness && uv run python extract_fst.py roms/GALE01.iso --list     # tree only
cargo test -p hsd-archive --test real_dat                             # skips if roms/files is absent
```

Everything under `harness/roms/` is gitignored. Never commit it.

## Disc layout

GameCube image, 1,459,978,240 bytes (`0x5705_8000`). All fields big-endian.

| Disc offset | Length | What | Value on this disc |
|---|---|---|---|
| `0x0000` | `0x440` | `boot.bin`, disc header | game id `GALE01`, title `Super Smash Bros Melee` |
| `0x0420` | u32 | offset of `main.dol` | `0x1E800` (124,928) |
| `0x0424` | u32 | offset of the FST | `0x456E00` (4,550,144) |
| `0x0428` | u32 | FST size | `0x7529` (29,993) |
| `0x042C` | u32 | max FST size | same as FST size (single disc) |
| `0x0440` | `0x2000` | `bi2.bin` | |
| `0x2440` | 115,444 | `apploader.img` (0x20 header + code + trailer) | |
| `0x1E800` | 4,425,184 | `main.dol`; length is the furthest section end from its own header | sha1 `08e0bf20134dfcb260699671004527b2d6bb1a45`, byte-identical to `third_party/melee-decomp/orig/GALE01/sys/main.dol` |
| `0x456E00` | 29,993 | `fst.bin` | 1,212 entries |
| `0x460000` | ... | first file data | runs to `0x5705_0341` |

`sys/` is five files, 4,579,901 bytes total.

### FST

12-byte entries followed by a NUL-separated string table. Entry 0 is the
root; its "next sibling" field is the entry count. A directory's
descendants are the entries between it and its next-sibling index.

1,212 entries: root, 2 directories, **1,209 files, 1,426,086,598 bytes**.
The only directories are `audio/` and `audio/us/`; the other 999 files
sit at the top level.

| Extension | Files | Meaning |
|---|---|---|
| `.dat` | 838 | HSD archives (fighters, stages, menus, trophies, effects) |
| `.ssm` | 110 | sound sample banks (`audio/`) |
| `.hps` | 98 | music streams (`audio/`) |
| `.thp` | 75 | movies |
| `.usd` | 56 | English-text variants of `.dat` menu files |
| `.mth` | 28 | video |
| `.sem`, `.bnr`, `.ini` | 4 | sound event map, banner, an empty `usa.ini` |

Top-level prefixes: `Ty` (trophies, 357), `Pl` (fighters, 274), `Gm`
(game modes, 133), `Gr` (stages, 76), `Ef` (effects, 36), `Mv`, `Sd`,
`If`, `Ir`, `Vi`, `Mn`, `Lb`, `Nt`, `It`, `Db`.

Fighter file naming, from `ftfox.c` (`ftFx_Init_*` strings) and
`ftdata.c`:

| File | Public symbol(s) | Role |
|---|---|---|
| `PlFx.dat` | `ftDataFox` | `struct ftData`: attributes, animation table, subaction scripts, hurtboxes |
| `PlFxNr.dat`, `PlFxOr.dat`, `PlFxLa.dat`, `PlFxGr.dat` | `PlyFox5K_Share_joint`, `PlyFox5K_Share_matanim_joint` | one costume each (neutral, orange, lavender, green); same symbol names in every costume file |
| `PlFxAJ.dat` | one `..._figatree` per sub-archive | motion file, see below |
| `PlFxDViWaitAJ.dat` | figatrees | demo/result-screen motions, indexed by `ftData.x14` |
| `PlCo.dat` | `ftLoadCommonData` | shared fighter data |

## What the archive parser saw

No change to `hsd-archive` was needed: all four stand-alone files and all
221 `PlFxAJ.dat` sub-archives parse with `Archive::parse`, every
relocation slot holds an offset inside the data section, and every public
name resolves. Observations worth keeping:

- **`version` bytes differ.** `PlFx.dat`, `PlCo.dat`, `GrNLa.dat` carry
  `"001B"`; `PlFxNr.dat` and every sub-archive inside `PlFxAJ.dat` carry
  four zero bytes. Retail never reads them, and neither do we.
- **Relocated zero is a real link.** Each file has slots that the
  relocation table names and that hold `0`: `PlFx.dat` 1, `PlFxNr.dat` 3,
  `PlCo.dat` 4, `GrNLa.dat` 1. The one in `PlFx.dat` is `ftDataFox.x8`,
  the `FtPartsDesc`, which lives at data offset 0. This is exactly the case
  the crate's pointer policy was designed for: `Archive::link` (relocation
  table) says `Some(0)` where `Reader::offset` (`0 == null` heuristic)
  says `None`. Nothing above `hsd-archive` should use the heuristic on a
  pointer field.
- **No externs anywhere.** `nb_extern == 0` in all five files.
- **Relocation tables are already sorted** on disc; the parser's sort is a
  no-op for retail data.

| File | Bytes | `data_size` | Relocs | Publics |
|---|---|---|---|---|
| `PlFxNr.dat` | 362,978 | `0x56D80` | 1,792 | 2 |
| `PlFx.dat` | 259,850 | `0x3BCE0` | 3,710 | 1 |
| `PlCo.dat` | 149,101 | `0x239A0` | 805 | 1 |
| `GrNLa.dat` | 611,125 | `0x8D340` | 7,999 | 27 |
| `PlFxAJ.dat` | 1,525,984 | not one archive | | 221 sub-archives |

### `PlFxNr.dat`: Fox's skeleton

Publics: `PlyFox5K_Share_joint` (data offset `0x1C668`) and
`PlyFox5K_Share_matanim_joint` (`0x52A08`). The root joint name is the
costume table's `joint_name` (`ftFx_Init_CostumeStrings[0]`), so the
`_Share_joint` guess was right.

`read_public_jobj` on the root gives **73 joints**, root at depth 0 and
the deepest joint at **depth 13** (14 levels). The root has no `next`
sibling. 65 joints carry an envelope matrix (`HSD_Joint.mtx`), 4 carry
geometry (`DObjDesc` chains), none has a class name, an `RObjDesc`, or
the `JOBJ_INSTANCE` flag. Distinct `flags` words: `0x8`, `0x9`,
`0x40089`, `0x10000009`, `0x10050089`, `0x1005018E`. The material
animation tree has 73 nodes, one per joint.

Wait1's `FigaTree.nodes` also has 73 entries, so the motion format
addresses the skeleton in `JObjLoad` allocation order.

### `PlFx.dat` and `PlFxAJ.dat`: the animation table and motion file

`ftDataFox` is at `0x98F4`. Its `+0x0C` pointer (`ftData.xC`) is the
animation table at `0x771C`: an array of `struct Fighter_WaitAnimData`
(`ft/types.h:885`, 0x18 bytes) with **327** rows, the count coming from
code (`ftData_Table_Unk0[FTKIND_FOX]`, `ftdata.c:255`), not the file.
The row index is the motion-state id.

| Offset | Field | Meaning |
|---|---|---|
| `+0x00` | `char* x0` | public symbol of the `FigaTree` in the sub-archive, e.g. `PlyFox5K_Share_ACTION_Wait1_figatree` |
| `+0x04` | `s32 x4` | byte offset of the sub-archive inside `PlFxAJ.dat` |
| `+0x08` | `s32 x8` | byte length of the sub-archive; `0` means no animation (then `x0` is null too) |
| `+0x0C` | `CmdUnion* xC` | subaction script |
| `+0x10` | `s32 x10_animCurrFlags` | |
| `+0x14` | `u32 x14` | 0 on disc; runtime pointer to the sub-archive in RAM |

How retail uses it (`ftData_80085A14`, `ftData_80085E50`, ftdata.c):
`lbFile_800168A0` loads the whole AJ file flat; each row's `x14` becomes
`base + x4`. To play motion state `n`, `x8` bytes at `x14` are copied into
the fighter's 0x8000-byte buffer (`Fighter_x59C_t`; `x8 > 0x8000` is an
assert), `HSD_ArchiveParse` runs on the copy, and `x0` is looked up with
`HSD_ArchiveGetPublicAddress`. So **each animation is a complete
stand-alone `.dat`** with one public and no externs, and the AJ file is
nothing but those archives packed on 32-byte boundaries
(`OSRoundUp32B`).

Measured:

- 278 of the 327 rows have an animation; the other 49 are all-zero.
- 220 distinct sub-archives are referenced. Rows share them: motion states
  2 (`Wait1`) and 6 both point at offset 0, size 5,077, with different
  scripts.
- Scanning the file header by header finds **221** sub-archives that tile
  it exactly (last one ends at `0x1748DC`, file is `0x1748E0`). Table
  order is not file order.
- The one unreferenced sub-archive is
  `PlyFox5K_Share_ACTION_WalkBrake_figatree` at `0x5DC0`, 3,601 bytes.
- Largest sub-archive: `0x7CC8` bytes, under the `0x8000` ceiling.
- `Wait1` (offset 0, 5,077 bytes: header `file_size=5077 data_size=4508
  nb_reloc=123 nb_public=1`) is a `FigaTree` of 120 frames with 73 nodes.
  Every referenced figatree reads with `read_public_figatree`, has
  `sum(nodes)` tracks, and every track's byte stream has its declared
  length.

`ftData.x14` (`0x95C4`) is a second table of the same row type with 14
entries (`ftData_UnkIntPairs[FTKIND_FOX]`, `ftdata.c:1505`): Win1..3 and
their waits, Selected, SelectedWait, Lose, IntroL/R, Ending, Wait1. Its
offsets index the demo files named by `ftFx_Init_DemoMotionFilenames`
(result, intro, ending, and `PlFxDViWaitAJ.dat`), not `PlFxAJ.dat`.

### `PlCo.dat`

One public, `ftLoadCommonData` at `0xECD8`. Not read further yet.

### `GrNLa.dat`: Final Destination

27 publics: `ALDYakuAll`, 17 `GrdLast*_image` textures
(`GrdLastCloud2_I8_image`, `GrdLastGround2_CMPR_image`,
`GrdLastLine1_I4_image`, `GrdLastLine2_I4_image`,
`GrdLastMilkyWay1_I8_image`, `GrdLastMilkyWay2_I8_image`,
`GrdLastRipple1_CMPR_image`, `GrdLastRipple3_I8_image`,
`GrdLastStar1_I4_image`, `GrdLastStar2_I8_image`, `GrdLastStar3_I8_image`,
`GrdLastStar4_I4_image`, `GrdLastStarDust1_I4_image`,
`GrdLastTile2_CMPR_image`, `GrdLastWall1_RGBA8_image`,
`GrdLastYuka0_I4_image`, `GrdLastYuka1_I4_image`), and the nine
`grDatFiles_801C6038` looks up by name: `map_head`, `coll_data`,
`grGroundParam`, `itemdata`, `map_ptcl`, `map_texg`, `yakumono_param`,
`map_plit`, `quake_model_set`.

**`coll_data` is its own public** (`0x4E0F0`); it is not reached through
`map_head`. `ground.c:505` passes `stage_info.coll_data` straight to
`mpLibLoad`. Layout is `struct MapCollData` (`mp/types.h:119`):

| Field | Value |
|---|---|
| `verts` | `0x4DF48`, 16 `Vec2` |
| `lines` | `0x4DFC8`, 16 `MapLine` (16 bytes each) |
| floor / ceiling / right wall / left wall / dynamic `(start, count)` | `(0,3) (3,3) (6,5) (11,5) (0,0)` |
| `joints` | `0x4E0C8`, 1 `MapJoint` (40 bytes) |
| `x2C` | 0 |

The floor is `y = 0` from `x = -85.5657` to `85.5657` in three lines
(`v4->v15`, `v15->v6`, `v6->v5`; the outer two have `LINE_FLAG_LEDGE` in
`lo_flags`, the inner one spans `-75..75`). The joint's bound is
`(-93.5657, -63.3882, 93.5657, 8.0)` and it owns vertices `0..16`.
`grGroundParam->y`, the map scale `Ground_801C0498` returns, is `1.0`.

`map_head` (`0x358`) is `struct UnkStageDat` (`gr/types.h:2055`): six
`(pointer, s32 count)` pairs then a zero word. On this stage:
`(0x54, 1)`, `(0xAC, 10)` model sets of `0x34` bytes (`UnkStageDat_x8_t`:
joint, animjoint, matanimjoint, shapeanimjoint, camera, lights, fog,
`GrJoint` list), `(0x2B4, 2)` splines, `(0x2BC, 32)`, `(0x33C, 3)`
shadow entries, `(0x354, 1)`.

**Building a `melee_mp::CollMap` from this.** Not done yet, and
deliberately not started in the test: `CollMap::load(MapCollData, scale,
GrKind)` exists (`crates/melee-mp/src/map.rs`) but takes the owned
`melee_types::mp::MapCollData`, and nothing reads that type out of an
archive; `CollMapBuilder` (`builder.rs`) is a hand-construction stand-in
"until `hsd-archive` can read a stage's `coll_data` node". What is needed:

1. A reader `MapCollData::read(&Archive, offset)` following the offsets
   above (`Vec2` verts, 16-byte `MapLine`s, 40-byte `MapJoint`s), which
   means a crate that depends on both `hsd-archive` and `melee-types`.
   Per the layering rules that is either `hsd-archive` gaining a
   `melee-types` dependency (it currently has none) or a small
   `desc`-style module in `melee-mp`, which already depends on
   `melee-types` and would only add `hsd-archive`. The second keeps
   `hsd-archive` a leaf; the first keeps "the only crate that knows
   on-disc layout" literally true. Decide in `TRACKER.md`.
2. Then `CollMap::load(coll_data, 1.0, GrKind::Last)` (`Gr_Kind_Last` is
   Final Destination) and a test that the loaded floor/ledge queries agree
   with the synthetic FD used by the existing 42 `melee-mp` tests.
