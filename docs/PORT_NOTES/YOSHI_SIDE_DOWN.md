# Yoshi Egg Roll and Yoshi Bomb

Ports ftyoshispecials.c (actions 356..363) and ftyoshispeciallw.c (366..368)
with the star article (ityoshistar.c, `it-yoshistar`).

## Witnesses (retail-recorded, exact)

| Scenario | What retail does |
|---|---|
| `yoshi_sidedown_bomb_ground` | Grounded Yoshi Bomb beside Fox: hop, descent at frame 20 (8012EA04), the descent hits Fox, landing (8012EAD8) and two stars. |
| `yoshi_sidedown_bomb_air` | Aerial Yoshi Bomb after a jump: descent, landing, a star hits Fox and vanishes (it_80272BA4, 0x411). |
| `yoshi_sidedown_roll_ground` | Hop (360), aerial loop (361), floor bounce (362), ground loop (357), turn (358) with dust (0x3FF) and the sparkle program 59 in the secondary color slot, B break on the ground (359, shell 0x4CF and 0x3F6). |
| `yoshi_sidedown_roll_hit` | Ground roll into Fox (hit, fn_8012EFF4), off the edge (362), B break in the air (363), FallSpecial with the landing lag, bottom KO. |
| `yoshi_sidedown_roll_air` | Aerial entry after a jump, floor bounce, ground roll hitting Fox, bottom KO while rolling (fn_8012EC7C as death2). |

## Ported but not reached by a witness

- Wall bounces (ftYs_SpecialS_SpawnWallBounceEffect): FD has no wall a roll
  can reach from the stage in these layouts.
- Yoshi Bomb ledge catch (ftCliffCommon_80081298 once cmd_vars[0] is set) and
  its floor touch before the script arms the landing (ftCommon_8007D5D4).
- The Egg Roll's take-damage callback (fn_8012EDE8): no witness hits Yoshi
  mid-roll.

## Left `unimplemented!`

- Star shield bounce (it_802B312C -> itColl_BounceOffShield).
- ftColl_8007ABD0 for a model-scaled fighter (ftCo_CalcYScaledKnockback).
- x21F8 (fn_8013295C) needs the hit-turn rotation (ftCo_800C37A0), which the
  port does not model; the callback is therefore never invoked.

## Shared pieces

- `Fighter::change_motion_state_with_flags` / `MotionEntryFlags`: retail
  flag words for character state changes. Ft_MF_SkipColAnim keeps the newly
  modelled secondary color slot (x488, ftCo_800C0134 / ftCo_800C0408).
- Opcode 14 (ftAction_80071708): HitCapsule x42_b5 (hits fighters) and x42_b7
  (hits items). The Egg Roll's script clears x42_b5 on hitbox 1.
- `CharacterCallbacks::DEAL_DAMAGE` (fighter.c:2929) and
  `FighterCore::set_part_rotation` (ftPartSetRotX/Y/Z).
