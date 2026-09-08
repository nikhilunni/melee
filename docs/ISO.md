# Which disc image

**Super Smash Bros. Melee, NTSC-U, revision 1.02. Game ID `GALE01`.**

This is the version every competitive tool targets, the version the decomp
matches, and the only version the symbol map in
`third_party/melee-decomp/config/GALE01/symbols.txt` is valid for. Do not
use 1.00, 1.01, the PAL release, or the Japanese release: every address in
the harness would be wrong.

## Verifying

Dump your own disc to a full, unscrubbed `.iso` (about 1.35 GB) or `.rvz`.
In Dolphin, right-click the game, Properties, Verify tab, and compare:

| Hash | Value |
|---|---|
| MD5 | `0e63d4223b01d9aba596259dc155a174` |
| SHA-1 | `d4e70c064cc714ba8400a849cf299dbd1aa326fc` |

These are the community-published hashes for a clean NTSC 1.02 dump. If your
dump has been scrubbed or modified they will not match.

The authoritative check inside this project is the executable itself.
Extract `sys/main.dol` from the disc and confirm:

```
sha1: 08e0bf20134dfcb260699671004527b2d6bb1a45   main.dol
```

That value comes from `third_party/melee-decomp/config/GALE01/build.sha1` and
is what the decomp reproduces byte for byte.

## Where it goes

Put the image under `harness/roms/`, which is gitignored, or point the
harness at it with `MELEE_ISO`. For the Rust simulator, extract the disc
(Dolphin: right-click, Properties, Filesystem, Extract Entire Disc) and pass
the extracted `files/` directory to `melee-sim --assets`.

## Dolphin

A scripting-capable Dolphin is needed to drive scenarios. The harness targets
the Python scripting fork ("dolphin scripting" by Felk). Mainline Dolphin's
GDB stub is the fallback and needs a different driver. Slippi's Dolphin is
useful for replay playback but its Gecko codes assume retail addresses, so
never run the oracle against a modified DOL.
