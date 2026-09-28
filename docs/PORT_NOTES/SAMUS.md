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
rope is a fixed 48-link array in the character payload (boxed, allocated
with the fighter). Links run hand (index 0) to tip (last).

## Not ported (fail closed)

- A tip that meets a wall in the air: `fn_802B805C`'s AirCatch arm
  (`ftCo_800C3CC0`, `it_802BAB40`), beam states 6..8 (`fn_802B895C`,
  `fn_802B8B54`, `fn_802B8D38`), `ftSs_MS_AirCatchHit` (358). The wall and
  ledge tether needs the rope's hang and climb and `ftCliffCommon_80081370`.
- The button code's longer beam (`u.ss.x2240 >= 4`).
- `itSamusbomb_UnkMotion2` (a sliding bomb), bomb shield bounce.

## Known divergence, not registered

`samus_grab_throwu_fd_fox4`: the up throw's zero-damage hitbox on part 58
lands on the thrown Fox in the port (a two-frame freeze at frame 13 of the
throw); retail does not hit him. Not diagnosed.
