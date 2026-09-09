# Peach data, NTSC-U 1.02

`ft-peach` owns Peach's archive descriptor, typed special attributes and character
callbacks. The scene registry adds one `Peach` row; shared fighter code dispatches
through `CharacterCallbacks`. No game data is checked in.

## Sources and layout

Read-only references: `ft/kinds/ftPeach/{types.h,ftpeach.c,ftpeachfloat.c}`,
`ft/kinds/ftCommon/ftCo_JumpAerial.c`, `ft/ftdata.c`, `ft/ftparts.c`,
`ft/ftdynamics.c`, `ft/ftanim.c`, `gr/ground.c` and `if/ifstock.c` under
`third_party/melee-decomp/src/melee/`. Runtime reads use archive relocations,
including a relocated pointer whose target is offset zero.

| Resource | Value |
| --- | --- |
| Fighter kind | 9, `FTKIND_PEACH` |
| Data / public symbol | `PlPe.dat` / `ftDataPeach` |
| Animation archive / entries | `PlPeAJ.dat` / 318 |
| Costume files | `PlPeNr.dat`, `PlPeYe.dat`, `PlPeWh.dat`, `PlPeBu.dat`, `PlPeGr.dat` |
| Costume roots | `PlyPeach5K_Share_joint`; colored roots insert `Ye`, `Wh`, `Bu`, `Gr` after `5K` |
| Joint count / semantic parts / part animation groups | 114 / 54 / 3 |
| Dynamic chain roots, archive joint indices | 19, 31, 61, 49, 37, 43, 55, 25, 88 |
| Dynamic chain sizes / colliders | Nine chains of five joints / zero |
| ECB joint indices | 87, 97, 71, 13, 7, 4 |

On the local disc `PlPe.dat` is 269820 bytes, `ftDataPeach` is data-relative
`0xB37C`, common attributes are `0x3874`, special attributes `0x3A1C`, animation
table `0x8680`, blend metadata `0x7C98`, part animations `0x2618`, and dynamics
`0x2B00`. These are diagnostic observations, not loader constants.

`ftData.ext_attr` (+4) points to the complete `0xC0`-byte `ftPe_DatAttrs`:

| Range | Owned concept | Representative retail values |
| --- | --- | --- |
| 00–0C | Float animation boundaries, offset, duration | Raw boundaries 0/0; offset 5, duration 150 |
| 10–2C | Vegetable rare-item count, odds, weighted item kinds | 3, 128; `(2, BombHei=6)`, `(3, Dosei=7)`, `(1, Sword=12)` |
| 30–6C | Peach Bomber input, launch, travel and rebound | Input window 3; gravity `0x3DA3D70A`; rebound X −1 |
| 70–90 | Parasol mobility, angle, momentum and duration | Startup multiplier `0x3F2AA64C`; open duration 600 |
| 94–A8 | Toad momentum, gravity and collision parameter | Air friction `0x3B23D70A` |
| AC–BC | Toad counter volume | Bone 3; offset (0, 1, 3.5); radius 6 |

Each field documents its decomp offset. Integer fields remain signed where
specified; item kinds are enums. `types.h` has a stale `+1C` comment on the
item array: its first weight is at **+18**, and first kind at **+1C**. The reader
follows actual member sizes and the retail data. No reference files were edited.

## Load and reset

`ftPe_Init_OnLoad` (`8011B628`) replaces the first two float attributes with the
FigaTree lengths of motions 18 and 19 (`lbAnim_8001E8F8`). The default
`on_resources_loaded` callback exposes decoded resources and costume identity;
Peach performs these writes there after `on_load`. Five definitions register:
Explode `0x62`, Turnip `0x63`, Parasol `0x67`, Toad `0x68`, Spore `0x6F`.
All four special directions are available; Peach has no walljump capability.

`ftPe_Init_OnDeath` (`8011B51C`) restores float availability, resets the smash
choice to −1 and Toad boost usage, and clears both parasol references, Toad and
vegetable references. It preserves the float time field. Model groups 0–6 are
`[0,-1,0,-1,0,0,-1]` for costume 1 and `[0,0,0,-1,0,-1,0]` otherwise.
Landing clears aerial Toad usage; grounded motion changes restore float
availability. Active parasol removal on interruptible landing is explicitly
unimplemented. Saved character state is imported at the snapshot boundary.

## Character hooks and shared corrections

- `aerial_jump_style = Peach`: `ftPe_JumpAerial_Enter` (`800CC0E8`) starts with
  zero Y/Z velocity. Horizontal input multiplication is separate (`800CC130`,
  `800CC15C`). `ftCo_JumpAerial_Phys_Cb` (`800CC6C8`) runs horizontal drift and
  `ft_800851D0` copies the animation's vertical displacement (`800851F8/FC`),
  without ordinary jump gravity or fast fall.
- `check_float_input`: runs the two Peach float predicates before and after the
  double-jump check, preserving Jump's physics-start and JumpAerial's command
  variable gates. Actual float entry (`8011BB6C`) is explicitly unimplemented;
  these recordings exercise the supported double jump without entering Float.
- `dynamics_first_force_bone`: `ftCo_8009DD94` starts forces at bone 3 for every
  Peach chain except the final chain, which starts at zero.
- Motion metadata bit `0x08000000` selects per-chain start indices through
  `ftData.x2C.x10` and the motion blend slot. Those rows are integer indices,
  despite their decomp `FigaTree**` type. Missing rows disable solving with
  sentinel `0x100`. The prior all-or-none selection was insufficient for Peach.
- Subaction opcode 50 (`ftCo_8009E318`) toggles dynamic ownership. Unlocking
  restores the descriptor pose and attaches animation at its current frame;
  locking removes main/blend subtree animation. Pose reset includes following
  sibling descriptors, while attachment and evaluation stop at the subtree.
  This distinction is required for start tick 100, bone 93. Command changes
  finish before independent part blends.
- `Ground_801C1CD0` increments the collision stamp after every ground object's
  animation, including static objects. This selects the unextended floor check
  at Peach ledge-escape tick 250; skipping the increment incorrectly landed her
  one tick early. Existing collision arithmetic is unchanged.
- The idle savestate interrupts stock HUD callback `802F9410` at scheduler link
  17. The importer validates its inactive particle-emission conditions and
  resumes after completed fighter, particle and dynamics phases. It does not
  replay those phases or substitute expected RNG values.

## Verification and limits

The three `ft-peach` tests independently encode archive relocation and truncated
input cases, check float payloads and enum validity, verify load/reset behavior,
and pin disc attributes, parts, animation-derived boundaries and all nine chains.
Local disc-dependent tests skip cleanly if those files are absent.

See [M4_PEACH.md](PORT_NOTES/M4_PEACH.md) for the exact scenario
commands, final output and full-workspace checks. The bone oracle compares SRT at
the tick boundary, including quaternion W when enabled. Unused Euler W is excluded
under the existing Marth/Fox contract. No rendered Peach matrix capture exists.
Float, specials, attacks and active item interactions remain outside this port;
capability flags do not imply a complete Peach moveset. No Dolphin run was made.
