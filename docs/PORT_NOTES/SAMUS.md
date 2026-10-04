# Samus

Samus (PlSs.dat) vs Fox on Final Destination, boundary `start_fd_samus_fox4`.
Crates: `ft-samus` (the fighter), `it-samus` (bomb, charge shot, missile,
grapple beam).

## Ported

- Screw Attack, the morph-ball roll, the jump thruster.
- Missiles: homing and super (`it_802B62D0`), lock-on targets, trail on the
  model's grandchild, the two-newer-missiles rule.
- Charge Shot: charge kept across states, full-charge glow through
  `ftData_UnkMotionStates4` (colour 53), launch, recoil, reflect, shield
  bounce.
- Bomb and bomb jump: rows 341/342 and 355/356, the bomb's bounce to rest
  (`it_8026E248`), the blast offered to its owner through the accessory
  (`ItemLogic::accessory` returns an `OwnerBlast`), the launch conditions
  (`ftColl_8007B868`, the motion's x2071 class from the x4_flags column, the
  ball model's late start).
- Grapple beam: grab and dash grab timelines (`fn_800D9558`, `fn_800D9930`)
  with their RNG sparks, the rope (`grapple/chain.rs`: the itsamusgrapple.c
  link walks with every fmadds), the tip as `fp->parts[0x8B]`
  (`FighterCore::grafted_part`) for the catch capsule and the capture hold
  point, the reel-in pull (`it_802BAA94`), throws (the throw beam model is
  drawing only).
- Aerial grapple: `ftCo_800C3B10` at every aerial IASA (`try_air_tether`,
  `used_tether`, the same API as the Link branch), `ftSs_MS_AirCatch` (357),
  landing without interrupt.

## The rope

The beam article only mirrors its state; Samus's accessory2 (`it_802BAC80`)
runs the rope from her own proc, accessory3 (`it_802BACC4`) in hitlag. The
rope is a fixed 60-link array in the character payload (boxed, allocated
with the fighter). Links run hand (index 0) to tip (last).

## The button code

While a beam hangs from the hand or flies (`fn_802B7E34`, `fn_802B805C`),
d-pad up, down, up held in turn and then A pressed complete `u.ss.x2240`
(`samus_grapple_state_sync`). Every beam made after that, until Samus dies,
carries xDD4 x16 (`grapple::Extended`):

- made on the ground (1): twice the links (60); her hitboxes, the catch
  capsule with them, are cleared every frame of the thrown, bounced and
  sagging rope; while L is held the tip flies at `ftCo_800A4A40`'s fighter
  (the scene offers it as `FighterCore::nearest_fighter`); A gives the tip a
  catch capsule once (`it_802B7160`, words `it_803B8660`) and the beam
  becomes kind 2.
- made in the air (2): the usual rope; hanging from a wall, L held stops the
  countdown.

`it_802B7160` copies HitCapsule +0x134 bit 3 (hit the grabbed victim only)
from a stack byte nothing wrote. `samus_grab_code_strike_fd_fox4` shows it
clear on `it_802B9328`'s path (the beam catches Fox); the bounced and sagging
rope's callers (`it_802B99A0`, `it_802B9CE8`) have no witness.

## Not ported (fail closed)

- A tip that meets a wall in the air: `fn_802B805C`'s AirCatch arm
  (`ftCo_800C3CC0`, `it_802BAB40`), beam states 6..8 (`fn_802B895C`,
  `fn_802B8B54`, `fn_802B8D38`), `ftSs_MS_AirCatchHit` (358). The wall and
  ledge tether needs the rope's hang and climb and `ftCliffCommon_80081370`.
- `itSamusbomb_UnkMotion2` (a sliding bomb).

## Known divergence, not registered

`samus_grab_throwu_fd_fox4`: the up throw's zero-damage hitbox on part 58
lands on the thrown Fox in the port (a two-frame freeze at frame 13 of the
throw); retail does not hit him. Not diagnosed.

## A Bomb bouncing on Randall (fixed 2026-10-03)

`slp_ys_samus_falco_t6900` (`SAMUS/17_27_54 Samus + Falco (YS)` played back
on retail from `start_ys_slippi8_p24_samus3_falco0_ucf073`, ports 2 and 4,
UCF 0.73; a `SLIPPI_REPLAY_WITNESSES` entry, exact for 6900 ticks) differed
at tick 1731 on a falling Bomb's position below the stage's right side:
retail 93.7959, the port 94.1508. The Bomb had bounced on Randall's cloud at
1729 (`it_8026E248`): `it_8027781C` stores the line's speed in x64, and the
next position update adds it once and clears it (`Item_802697D4`:
`it_8027346C` at 0x80269954, as `it_80273484` at 0x802698F0 clears x58). The
port never cleared it, so the Bomb went on moving with the cloud (0.3548 a
tick) after leaving it.
