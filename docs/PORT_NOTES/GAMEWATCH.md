# Mr. Game & Watch

Mr. Game & Watch (PlGw.dat, scenario name `GameAndWatch`) vs Fox on Final
Destination, boundary `start_fd_gameandwatch_fox4`. Crates: `ft-gamewatch`
(the fighter), `it-gamewatch` (his ten articles). He has no character effect
file (no `EfGwData.dat`).

## What is different about him

- **Flat model.** `ftGw_Init_OnLoad` sets `fp->x34_scale.z` to the width
  attribute (0.01). `Fighter_UpdateModelScale` (80067BB4) puts it in the
  root's x scale (`Capabilities::model_width`), so every bone's world z is
  flattened; the articles' recorded z positions depend on it. Hit tests
  against him go through `x44_mtx`
  (`Fighter_UnkApplyTransformation_8006C0F0`, `CombatState::flat_matrix`):
  hurt, shield, reflect and absorb volumes take `x44_mtx * bone` as their
  matrix and the fighter's depth as their z (`lbColl_80007ECC`,
  `lbColl_8000805C`, `lbColl_80007BCC`).
- **Normals are character rows.** Jab and rapid jab (341..344), down tilt
  (345), forward smash (346), neutral, back and up aerials (347..349) and
  their landings (350..352) reuse the common callbacks behind his own
  entries (`ENTER_JAB`, `ENTER_RAPID_JAB`, `ENTER_DOWN_TILT`,
  `FORWARD_SMASH`, `ENTER_AERIAL`). The down tilt takes no new attack
  instance and has no repeat latch; the three aerials land at the full
  attribute lag whatever L does (the up aerial reads the back aerial's lag).
- **Articles.** Each of those moves creates an article from accessory4 in
  one of his parts (`Item_AttachGameWatchArticle`): Greenhouse sprayer,
  Manhole, Fire torch, Parachute, Turtle, Spitball Sparky breath. Each
  lives while its owner stays in the move's rows; damage, death and leaving
  the ground in a grounded attack remove them all (`ftGw_Init_OnDamage`,
  `ftGw_Init_8014A538`). The hitboxes are the fighter's own script's.
- **Stepped animation** is in the animation data; nothing in the code
  treats it specially. `x2223_b1`, which OnLoad sets, has no reader in the
  retail fighter code.

## Specials

- **Chef** (353/354): accessory4 throws a food on the script's flag;
  `HSD_Randi(3)` over the five foods minus the last two thrown. Held B
  loops on `cmd_vars[2]`, a B press on `cmd_vars[1]`, up to five foods.
  The food (`it-gamewatch::chef`) flies on its own attribute entry, turns
  back off walls and lies spent after a floor, a hit or its lifetime.
- **Judgment** (355..372): the entry draws the face before the motion
  change (`ftGw_SpecialS_GetRandomInt`: the enabled faces minus the last
  two), so it is deferred to `Fighter::finish_input`
  (`INPUT_RNG_ENTRY`). Each face is its own row and script; the sign is an
  article. Face 7's food (`it_8028FAF4`) needs items switched on
  (`it_8026D324`), so with items off retail makes none; the port does not
  model the items-on branch. One hop per airtime (`x2234`, cleared by
  landing).
- **Fire** (373/374): both entries leave the ground with jumps spent; the
  trampoline is a free article below TopN (8014DF4C: fnmsubs); the rise is
  the animation's TransN turned by the one-time lean (`ft_80085154`); the
  end is the special fall.
- **Oil Panic** (375..380): the absorbing bubble follows the script's
  `cmd_vars[0]`; absorbed hitboxes fill the bucket through
  `ftData_OnAbsorb` (the Catch rows) and add their truncated damage; at
  three levels the next use spills (the Shoot rows), whose hitboxes take
  `absorbed * x78 + x74` (two `__cvt_fp2unsigned`).

## Shared-engine work that came with him

- Absorbing bubbles (`melee-ft::fighter::absorb`): `ftColl_CreateAbsorbHit`,
  the absorb step of `ftColl_8007925C`, `ON_ABSORB`, and the item side
  (`ItemCore::pending_absorb`, the kind's `absorbed` callback).
- `ItemHitFlags::absorbable` is x42_b0 (word bit 19); bit 17 is x42_b2,
  "misses a fighter facing the same way", now tested where retail tests it.
- `ItemEvent::BoneGust` (the Manhole's `lb_800119DC` from its cover bone),
  `ItemLogic::PICKUP_READS_OWNER`, `ARTICLE_HITLAG_BEGIN`.
- efAsync 0x3FB from a landing command (the down aerial's landing).

## Not ported (fail closed)

- A held item stowed under the Manhole (`ftGw_AttackLw3_ItemManholeSetup`,
  ftgamewatchattacklw3.c:36-41): needs items on.
- The rapid jab's accessory over an item pickup
  (`ftGw_Attack100Start_Enter`): needs items on.
- `ftGw_SpecialS_GetRandomInt` with no face enabled (retail reads an
  uninitialised result).
