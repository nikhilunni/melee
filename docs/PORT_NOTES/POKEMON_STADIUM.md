# Pokemon Stadium: Fox/Marth boundary, screen and transformations

Stage bring-up for `start_ps_fox_marth4` (2026-09-27), gr/grpstadium.c.

## Boundary

`make_boundary.py --stage PokemonStadium --players Fox Marth --stocks 4`
made `start_ps_fox_marth4` (seed 866544100). Its saved and cold scenarios
gate exactly over 600 ticks and it is registered in `harness/boundaries.toml`.

## Setup

- `grStadium_OnInit` (0x801D101C) creates maps 0, 1 (screen), 2
  (controller); map 2's init creates map 5 (the base arena). The only setup
  draw is the first base-form duration (`range(3600, 3800)`), so the cold
  setup interval is one stage draw, like Battlefield's.
- Collision, in C order: map 5's `Ground_801C2ED0` binds, updates and
  snapshots joint 4; the pit lines 0x55 and 0x6F are disabled; the form
  joints 1, 2, 3, 5, 7, 0 are disabled; `mpLib_800581DC(6, 4)`. The saved
  path applies the same edits before restoring fighters and validates that
  the boundary is in the base form.
- GrPs.dat binds no marker 0x94, so `Ground_801C39C0` falls back to the
  default "dummy CamRange" (-170, 170, 120, -60, offset 0). Retail's
  `stage_info.cam_info` at the boundary confirms it.
- Unused map_head entries hold an unrelocated -1 joint pointer; they read as
  absent models (`ModelDesc::present`). The forms' models live in
  GrPs1-4.dat (fire, grass, water, rock), loaded with the stage assets and
  looked up in `grDatFiles_801C6330` order.

## Screen (map 1)

`grStadium_801D1390`: the audience flash (`grStadium_801D1E20`, one
`Randi(200)` per tick, bank-30 particle 0x7530 through `grLib_801C96F8`'s
AppSRT), then the mode machine (`grStadium_801D2344/2528/2A60`). The
Versus scene drives it: countdown start (fn_8016B7B4, mode 10 or 13 in
Sudden Death), countdown end (Ground_801C0FB8's deferred callbacks, then
mode 11), GO's end (mode 1). The close-up mode's framing
(`grStadium_801D32D0`) projects the fighter's camera bone with the rendered
camera. The screen's camera subject (fn_801D11E4) is the newest in the
camera's list and is active during announcements. Not hooked: mode 9
(gm_8016B8D4, a player out of stocks) and mode 12 (match end); both draw
nothing and the Ground procs freeze at the match's end.

## Transformation (map 2)

`grStadium_801D4548`: waiting, loading, announcing, delay (x10 = 300),
sinking (fnmsubs 0x801D48C8; sunk hold x18 = 60), rising (x14 = 120),
settling. The announcement tick zeroes the timer through the display
union (`u.display.xD8` is `u.stadium.xD8`). Forms are Ground maps created
and destroyed mid-match with their init and callback3 (fire flames under
joint 0x12, rock dust, water spray, wheels 7/8, the windmill's collision
joint 0 and `JOBJ_CLASSICAL_SCALE`), `Ground_801C3214` relisting after a
destruction released a map, and `Ground_801C2FE0` per proc.

## The DVD read

`lbFile_80016580` reads the form archive asynchronously; the controller
polls `grStadium_801D42B8` until the callback (fn_801D4220) clears map 2's
xC4_b1. The port cannot derive the emulated disc's latency, so
`LOAD_POLLS` holds the polls measured from retail recordings (the choice's
`Randi(4)` at 0x801D4640, then the first sparkle 303 ticks after the
announcement):

| Archive | Form | Polls | Witness |
|---|---|---|---|
| GrPs1.dat | fire | 23 | stage_ps_fire_fox_marth4 |
| GrPs2.dat | grass | 18 | stage_ps_grass_fox_marth4 |
| GrPs3.dat | water | 23 | stage_ps_water_fox_marth4 |
| GrPs4.dat | rock | 21 | stage_ps_idle_fox_marth4 |

All four were measured at the first transformation (tick 3781). The
latency is the emulator's disc timing, which could differ later in a match
(disc head position, music streaming); a divergence of a few ticks at an
announcement is the first suspect.

## Witnesses

`stage_ps_{idle,fire,water,grass}_fox_marth4` (7200-7500 ticks): each form
rises under idle fighters and the base returns; particles exact. A short
walk at tick 130 steers the first choice (found with `melee-sim dry-run`).

## Not ported

- Presentation of the forms (the renderer skips absent models).
- Saved boundaries mid-transformation or in a transformed form.
