# Fox static fighter data (T4)

Verified on the owned NTSC-U 1.02 disc, 2026-09-08. This is the static-data
prerequisite for `idle_fd_fox`: two idle Foxes on Final Destination. The older
Yoshi's Story wording in M3_PLAN.md is superseded by the task and TRACKER.md.
Offsets below are relative to archive data sections or the named C struct;
header paths are under `third_party/melee-decomp/src/melee/`.

## Verified layout

`PlFx.dat` exports `ftDataFox` at **0x98F4**. The definition lives in
`ft/types.h:612-661`; `ftdata.h:9,21-26` includes it and declares the loaders.
The animation reader has moved unchanged from `desc.rs` to `desc/animation.rs`
and its public API remains available through `melee_ft::desc`.

| ftData slot | Disc target | Meaning | Header |
|---|---|---|---|
| +00 | 0x36F4 | Per-character common attributes | ft/types.h:613 |
| +04 | 0x3878 | Fox special attributes, ext_attr | ft/types.h:614 |
| +08 | **0** (relocated) | Model visibility descriptor and five bone bytes | ft/types.h:615-623 |
| +0C | 0x771C | Existing animation table, 327 rows | ft/types.h:624; ftdata.c:255 |
| +10 | 0x7060 | Animation blend-byte pairs | ft/types.h:625 |
| +14 | 0x95C4 | Demo animation table | ft/types.h:626 |
| +1C | 0x2848 | Five animation bone-set pointers | ft/types.h:628-633,1301-1308 |
| +24 | 0x730C | Wait choices; existing M3 plan records (2,70),(3,30),(-1,-1) | ft/types.h:635; ftwaitanim.c:47-59 |
| +2C | 0x297C | Dynamic bone sets and collision bones | ft/types.h:637,1842-1850 |
| +30 | 0x2990 | Hurtbox count and pointer | ft/types.h:638-641 |
| +34 | 0x752C | Scaled joint and scale | ft/types.h:642-645 |
| +38 | 0x7534 | Attachment joint, position, scalar | ft/types.h:646-650 |
| +44 | 0x75A4 | Six ECB joints, center and ledge dimensions | ft/types.h:653,584-595 |
| +58 | 0x75DC | Grounded leg-IK bones and lengths | ft/types.h:659,1910-1923 |

`ftCo_DatAttrs` is **0x184 bytes**, not about 0x5C0. A native C sizeof probe
compiled the actual declarations from `ft/types.h:679-769`, with only `u8`,
`Vec3` and the forward typedef supplied. It printed:

```text
sizeof(ftCo_DatAttrs)=0x184 mask offset=0x180
ReflectDesc=0x24 Fox=0xd4 reflector=0xb0 behavior=0x20
```

The second probe compiled the actual `ftFox_DatAttrs` and `ReflectDesc`
declarations (`ft/kinds/ftFox/types.h:76-149`, `lb/types.h:115-126`) with
fixed-width scalar/enum typedefs. These structures contain no pointers, so
host pointer width does not affect the result. Both probes used `cc`.

The common block contains **91 float components, five i32 values, one u8
mask, and three padding bytes**: 97 scalar values, rather than 97 floats.
The extension starts exactly 0x184 bytes after the common block on disc.
The reader has 17 concept groups: walking, ground, running, jumping, air,
combat, size, shield, ledge, items, specials, Yoshi egg, Kirby throw,
landing, wall, ice and camera. Vector components remain named `Vec3` fields.
Every field's Rust documentation cites its header line and byte offset.
Unknown meanings remain explicitly marked TODO; no guessed float arithmetic,
model scaling, or normalization occurs while reading.

## Pinned common Fox attributes

These words were printed from the disc before being pinned in
`crates/melee-ft/tests/real_fox_data.rs::fox_spawn_attributes`.
Decimals are display values; acceptance compares the hexadecimal bits.

| Attribute | Offset | Approximate value | f32 bits | Header line (ft/types.h) |
|---|---|---|---|---|
| walking.walk_accel_mul | +000 | 0.2 | `0x3E4CCCCD` | 687 |
| walking.walk_accel_base | +004 | 0.1 | `0x3DCCCCCD` | 688 |
| walking.walk_max_vel | +008 | 1.6 | `0x3FCCCCCD` | 689 |
| ground.ground_friction | +018 | 0.079999998 | `0x3DA3D70A` | 693 |
| running.dash_initial_velocity | +01C | 1.9 | `0x3FF33333` | 694 |
| running.dash_max_velocity | +028 | 2.2 | `0x400CCCCD` | 697 |
| ground.ground_max_horizontal_velocity | +034 | 3 | `0x40400000` | 700 |
| jumping.jump_startup_time | +038 | 3 | `0x40400000` | 701 |
| jumping.jump_v_initial_velocity | +040 | 3.6800001 | `0x406B851F` | 703 |
| jumping.hop_v_initial_velocity | +04C | 2.0999999 | `0x40066666` | 706 |
| air.gravity | +05C | 0.23 | `0x3E6B851F` | 710 |
| air.terminal_velocity | +060 | 2.8 | `0x40333333` | 711 |
| air.air_drift_max | +06C | 0.82999998 | `0x3F547AE1` | 714 |
| air.fast_fall_velocity | +074 | 3.4000001 | `0x4059999A` | 716 |
| size.weight | +088 | 75 | `0x42960000` | 721 |
| size.model_scaling | +08C | 0.95999998 | `0x3F75C28F` | 722 |
| shield.initial_shield_size | +090 | 14.375 | `0x41660000` | 723 |
| landing.normal_landing_lag | +0E4 | 4 | `0x40800000` | 737 |

Also pinned: `max_jumps=2` (+058, line 709), `rapid_jab_window=4` (+098,
line 725), and `weight_independent_throws_mask=0x09` (+180, line 768).
The mask is a byte; reading the full word would incorrectly produce 0x09000000.
Model scale is 0.96 (`0x3F75C28F`), kept separate from player scale.

## Part mapping and bone lists

The semantic part map is **not in PlFx's ftData**. `Fighter_LoadCommonData`
installs `ftPartsTable` from `ftLoadCommonData[4]` (`fighter.c:182-192`).
PlCo's public is 0xECD8; slot +10 points to 0xEB38. Fox is kind 1, whose
entry points to 0xCFC8. `FighterPartsTable` (`ft/types.h:46-50`) is two
32-bit pointers and a u32 joint count:

| Field | Relative offset | Fox disc value |
|---|---|---|
| joint_to_part | +0 (line 47) | 0xCF44, 73 bytes |
| part_to_joint | +4 (line 48) | 0xCF90, 54 bytes |
| parts_num | +8 (line 49) | 73 |

`parts_num` counts skeleton joints, not semantic parts. The forward-table
length is not stored in the header; the caller supplies it explicitly.
`FOX_PART_COUNT=54` records this disc's table. Reverse-map part 53 is used
by `ftparts.c:692`; the forward bytes occupy 0xCF90..0xCFC6, followed by
alignment padding and the descriptor. We do not infer lengths from padding.
The 73-joint count is independently checked against `PlFxNr.dat`'s skeleton.
`0xFF` is `FTPART_INVALID` (`ft/types.h:43`) and becomes `None`.

| Part id | FtPart | Joint |
|---|---|---|
| 0 | TopN | 0 |
| 1 | TransN | 1 |
| 2 | XRotN | 2 |
| 3 | YRotN | 3 |
| 4 | HipN | 4 |
| 5 | WaistN | absent |
| 6 | LLegJA | 5 |
| 7 | LLegJ | 6 |
| 8 | LKneeJ | 7 |
| 9 | LFootJA | 8 |
| 10 | LFootJ | 9 |
| 11 | RLegJA | 11 |
| 12 | RLegJ | 12 |
| 13 | RKneeJ | 13 |
| 14 | RFootJA | 14 |
| 15 | RFootJ | 15 |
| 16 | BustN | 21 |
| 17 | LShoulderN | 22 |
| 18 | LShoulderJA | 23 |
| 19 | LShoulderJ | 24 |
| 20 | LArmJ | 25 |
| 21 | LHandN | 26 |
| 22 | L1stNa | 27 |
| 23 | L1stNb | 28 |
| 24 | L2ndNa | 29 |
| 25 | L2ndNb | 30 |
| 26 | L3rdNa | 31 |
| 27 | L3rdNb | 32 |
| 28 | L4thNa | 33 |
| 29 | L4thNb | 34 |
| 30 | LThumbNa | 35 |
| 31 | LThumbNb | absent |
| 32 | LHandNb | 37 |
| 33 | NeckN | 38 |
| 34 | HeadN | 40 |
| 35 | RShoulderN | 41 |
| 36 | RShoulderJA | 53 |
| 37 | RShoulderJ | 54 |
| 38 | RArmJ | 55 |
| 39 | RHandN | 56 |
| 40 | R1stNa | 57 |
| 41 | R1stNb | 58 |
| 42 | R2ndNa | 59 |
| 43 | R2ndNb | 60 |
| 44 | R3rdNa | 61 |
| 45 | R3rdNb | 62 |
| 46 | R4thNa | 63 |
| 47 | R4thNb | 64 |
| 48 | RThumbNa | 65 |
| 49 | RThumbNb | 67 |
| 50 | RHandNb | 68 |
| 51 | ThrowN | 69 |
| 52 | TransN2 | 71 |
| 53 | unnamed in header | 72 |

`FtPart` transcribes `ft/forward.h:255-311`, including named numeric entries
56 and 109. Those two are outside Fox's table and return `None`; they must
not cause reads into the following descriptor. `WaistN` and `LThumbNb` are
explicitly absent inside the table. TopN/TransN/XRotN/YRotN map to 0/1/2/3.

Other bone references (all pinned in `fox_ecb_bones`):

| List | Values | Layout/consumer |
|---|---|---|
| ECB joints, in stored order | **41,55,25,13,7,4** | six s16, ft/types.h:585-590; direct joint lookup ft_081B.c:53-57 |
| ECB center Y; ledge X/Y/height | 0,11,13,9 (bits 00000000,41300000,41500000,41100000) | floats +C,+10,+14,+18; ft/types.h:591-594 |
| Animation translation / shield / held item / two foot-effect joints | 67,71,41,9,15 | five u8 at model descriptor +10..+14; ft/types.h:618-622 |
| Animation set 0 | root 27; 27..38 | root u16 +0, count u16 +2, byte-list pointer +4; ft/types.h:629-632 |
| Animation set 1 | root 57; 57..69 | same |
| Animation set 2 | root 41; [51] | same |
| Animation set 3 | root 41; [42,43,44,45] | same |
| Animation set 4 | root 41; [46,47,48,49] | same |
| Dynamic roots | [17] | bone +0, stride 0x18; lb/types.h:490-499; ft/types.h:1845-1846 |
| Dynamic collision bones | [41] | bone +0, stride 0x14; ft/types.h:646-650,1848-1849 |
| Hurtbox joints | [4,22,41,41,55,25,56,26,12,6,13,7,18] | bone +0, stride 0x28; ft/kinds/ftCommon/types.h:23-30 |
| Scaled/attachment joint | 4 / 4 | ft/types.h:643,647 |
| Ground IK right leg / left leg | [12,13,15] / [6,7,9] | ft/types.h:1911-1920; ft_0899.c:101-105,136-140 |
| IK upper / lower / foot extension | bits 404AE148 / 4028F5C3 / 3FB47AE1 | +4,+C,+18; ft/types.h:1914,1918,1922 |

The bone reader retains indices, not evaluated world-space positions.
Skeleton/animation evaluation and runtime ECB construction belong to later
M3 tasks. Visibility tables, animation resources and shape/dynamics parameters
are not duplicated by this reader; it extracts their bone references only.

## Fox special block

`ftFox_DatAttrs` is 0xD4 bytes, including the full ReflectDesc. The reader
uses four concept groups and typed `ItemKind` values. The pinned subset is:

| Attribute | Offset | Approximate value | Bits | Header |
|---|---|---|---|---|
| Blaster velocity | +14 | 7 | 40E00000 | ftFox/types.h:86 |
| Illusion startup gravity delay | +24 | 15 | 41700000 | ftFox/types.h:93 |
| Illusion ending gravity delay | +44 | 5 | 40A00000 | ftFox/types.h:102 |
| Illusion ending fall acceleration | +48 | 0.08 | 3DA3D70A | ftFox/types.h:103 |
| Fire Fox travel duration | +68 | 30 | 41F00000 | ftFox/types.h:117 |
| Fire Fox speed | +74 | 3.8 | 40733333 | ftFox/types.h:120 |
| Reflector release lag | +98 | 18 | 41900000 | ftFox/types.h:141 |
| Reflector fall acceleration | +AC | 0.026666667 | 3CDA740E | ftFox/types.h:147 |
| Reflection radius | +C4 | 8.5 | 41080000 | ftFox/types.h:148 + lb/types.h:119 |
| Reflection damage multiplier | +C8 | 1.5 | 3FC00000 | ftFox/types.h:148 + lb/types.h:120 |

Here `ftFox/types.h` abbreviates `ft/kinds/ftFox/types.h`. Integers pinned:
laser item kind 0x36 (+1C), gun item kind 0x4A (+20), Fire Fox bounce
parameter 15 (+6C), reflector gravity delay 4 (+A4), reflection joint 1
(+B0), maximum reflected damage 50 (+B4), ownership behavior byte 0 (+D0).

## PlCo common data selected for M3

`ftLoadCommonData[0]` points to 0x9FC0 (`fighter.c:188`). The reader exposes
18 fields. It does not deserialize the entire unfinished ftCommonData.
All are pinned in `common_idle_attributes_match_disc_bits`.

| Field | Offset | Bits / integer | Header line (ft/types.h) |
|---|---|---|---|
| Horizontal / vertical stick deadzone | +0 / +4 | 3E8F5C29 / 3E8F5C29 | 54-55 |
| Horizontal / vertical smash deadzone | +8 / +C | 3E800000 / 3E800000 | 56-57 |
| Analog shoulder deadzone | +10 | 3E99999A | 58 |
| Z-press analog value | +14 | 3EB33333 | 59 |
| Shield threshold | +18 | 3E800000 | 60 |
| Walk threshold | +24 | 3E3851EC | 63 |
| Turn threshold | +34 | BE800000 | 67 |
| Dash threshold / window | +3C / +40 | 3F4CCCCD / integer 2 | 69-70 |
| Friction above walking speed | +6C | 40000000 (2) | 81 |
| Tap jump threshold / window | +70 / +74 | 3F29999A / integer 4 | 82-83 |
| Squat threshold | +90 | 3F300000 | 90 |
| Ground knockback speed cap | +164 | 4104CCCD (8.3) | 143 |
| Ledge snap height multiplier | +1CC | 3F19999A (0.6) | 169 |
| Ground pose maximum angle, degrees | +804 | 41A00000 (20) | 552 |

Wait's movement predicates use ftCo_Turn.c:32, ftCo_Dash.c:35-37,
ftCo_Jump.c:33-34, ftCo_Squat.c:38 and ftwalkcommon.c:61-63. The common
friction multiplier is read by Wait Phys -> ft_084E.c:42-53. Ground
knockback clamping reads +164 (ftcommon.c:197-201). ECB creation itself
reads ftData.x44; ledge checks also use +1CC (ft_081B.c:142-147), and
grounded pose uses +804 (ft_0899.c:225). `ftColl_8007AEE0`, called by Wait
Phys, only clears a shield-position flag (ftcoll.c:3082-3085); it reads no
common float. Wait animation choice uses ftData.x24, not PlCo.
Attack/special-state physics and full input handling remain later tasks.

## Header/disc discrepancies and limits

- The task's approximate common block size and +10/+14 bone guesses do not
  match this revision's headers or disc. The verified locations are above.
- The part enum omits disc part 53 but includes 56 and 109, absent on Fox.
  The reader requires the explicit forward count and preserves unnamed IDs.
- Illusion +44 is named FALL_ACCEL in the header, but ftfoxspecials.c:575
  assigns it to gravityDelay. +48 is named TERMINAL_VELOCITY, but line 532
  passes it as fall acceleration with the common terminal velocity. The
  Rust names follow these consumers and preserve the disc words 5 and 0.08.
  Likewise +28 divides startup momentum (lines 100,123), +2C is startup air
  friction (193), +30 startup fall acceleration (191), and +40 ending air
  friction (535); names follow those uses.
- `ftData_x58_t` marks +12..+17 as padding (ft/types.h:1921), but Fox has
  nonzero word **40600000 (3.5)** at +14. Its meaning is unverified; the
  reader follows the declared fields and does not invent another length.
- No floating-point arithmetic was added, hence there are no multiply-add
  sites requiring an asm fusion audit. Nothing under third_party or harness
  was changed, and no game-data file was copied into the repository.

## Validation

- `cargo gate` (workspace tests).
- `cargo clippy --workspace --all-targets -- -D warnings`.
- `cargo test -p melee-ft --test real_fox_data -- --nocapture` prints all
  97 named attribute values and runs the real-disc pins.
- `cargo test -p ft-fox --test attributes real_fox_special_attributes -- --nocapture`
  prints the typed Fox extension and checks its pinned words.
- Synthetic archives exercise every common/Fox scalar, signed values, NaN
  payloads, negative zero, byte flags versus padding, relocated-zero links,
  null/unrelocated pointers, truncated extents and malformed bone counts.
- Real tests skip when `harness/roms/files` is absent; missing/corrupt files
  inside an existing extracted directory fail, rather than silently skip.

Final session results: the baseline and first completed workspace `cargo gate`
passed. A later gate, after concurrent melee-gr files appeared, failed in
`crates/melee-gr/tests/real_fd.rs:28` (`real_fd_archive_models_parameters_and_collision`,
actual 13, expected 4). Workspace clippy failed in concurrently edited
`crates/melee-gr/src/last/init.rs:77` with `suspicious_assignment_formatting`.
Those files are outside T4's allowed paths and were left untouched.
All 14 new T4 tests passed, as did the two existing animation descriptor tests.
`cargo clippy -p melee-ft -p ft-fox -p melee-types --all-targets -- -D warnings`
passed after the final T4 changes. Full-workspace acceptance remains blocked
on the concurrent melee-gr work. No expected values were loosened.

## Files changed by T4

- `crates/melee-ft/src/desc.rs` moved unchanged to `desc/animation.rs`.
- `crates/melee-ft/src/desc/{mod,read,attributes,bones,common}.rs` added.
- `crates/melee-ft/tests/{fighter_attributes,bone_desc,common_desc,real_fox_data}.rs`
  and `tests/support/mod.rs` added.
- `crates/ft-fox/src/attributes.rs`, `tests/attributes.rs` and
  `tests/support/mod.rs` added; `src/lib.rs` exports the new module.
- `crates/melee-types/src/ft_part.rs` added and exported from `src/lib.rs`.
- `crates/{melee-ft,ft-fox}/Cargo.toml` and corresponding `Cargo.lock`
  dependency entries updated; no new external dependency.
- `docs/FOX_DATA.md` added. TRACKER.md, CLAUDE.md, root Cargo.toml,
  third_party and harness were not edited. No commit was created.
