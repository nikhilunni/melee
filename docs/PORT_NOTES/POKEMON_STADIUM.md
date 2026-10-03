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
(`grStadium_801D32D0`) projects the fighter's camera bone with the main
CObj as the last display pass left it (`InitialState::rendered_camera`, set
in `render_cameras`): a tick without a display pass (the recorded `ps_frame`
did not move) sees the previous tick's camera, quake translation included.
Fighter graphics 0x513-0x515 (ftCo_09F7.c:263-274, efAsync kind 8) request
Small/Medium/Large quakes that shake that CObj; the scene forwards them to
Camera_RequestQuake after each fighter proc. The screen's camera subject
(fn_801D11E4) is the newest in the camera's list and is active during
announcements. Not hooked: mode 9 (gm_8016B8D4, a player out of stocks) and
mode 12 (match end); both draw nothing and the Ground procs freeze at the
match's end.

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
xC4_b1. No game state predicts when (it is the disc's, in Dolphin the
emulated DVD timing), so the port takes the poll's outcome as an external
event (docs/ORACLE.md, "External events"): the tick tracer records map 2's
read state (`stage_io`), `decode.py` marks the tick whose poll succeeded
(`events.stage_read_completed`), and the gate replays it; a trace without
it fails closed at the first poll. Standalone runs (explorer, native app,
dry runs) use the default policy `DEFAULT_READ_POLLS`, the polls measured for
each file's first read of a match (the choice's `Randi(4)` at 0x801D4640,
then the first sparkle 303 ticks after the announcement), for every read.
It is a plausible sample of retail's timing, not a prediction; a fixed table
keeps it free of hidden state (a seeded draw from the measured range would
need its own stream in every clone and recording):

| Archive | Form | Polls | Witness |
|---|---|---|---|
| GrPs1.dat | fire | 23 | stage_ps_fire_fox_marth4 |
| GrPs2.dat | grass | 18 | stage_ps_grass_fox_marth4 |
| GrPs3.dat | water | 23 | stage_ps_water_fox_marth4 |
| GrPs4.dat | rock | 21 | stage_ps_idle_fox_marth4 |

All four were measured at the first transformation (tick 3781), where the
reads are 10-12 ticks slower than their size alone suggests (a long seek
from wherever match setup left the head). Recordings vary even there: five
Fox/Marth explorer matches read GrPs4.dat (rock) in 22 or 23 polls and
GrPs3.dat (water) in 22. A later read is faster: `stage_ps_second_fox_marth4`
(11000 ticks, idle) chooses fire at tick 10102 after the rock form, and
GrPs1.dat arrives after 12 polls. With the recorded completion all of them
gate exactly and are registered (m5_gate). Modelling the timing instead would
need Dolphin's disc emulation (seek from the last read's end, transfer by
size and position, the streamed music).

## Witnesses

`stage_ps_{idle,fire,water,grass}_fox_marth4` (7200-7500 ticks): each form
rises under idle fighters and the base returns; particles exact. A short
walk at tick 130 steers the first choice (found with `melee-sim dry-run`).
`stage_ps_second_fox_marth4` (11000 ticks) adds a second read; particles
exact. The five explorer matches
`corpus_v3_ps_fox_marth4_{e9943b4ab_p0,ef89b3e70_p2,edb2b114a_p1,e89a89d0e_p2,e75fb4a9a_p1}`
gate exactly over their full length (CORPUS_V3_MATCHES) after three fixes
they exposed: a grounded fighter in hitlag still rides a moving floor
(Fighter_procUpdate's mpGetSpeed, fighter.c:2380-2390; the water form's
windmill), the water form's material-10 splashes (mpLib_803BE118 swaps in
mpLib_803BD850: footstep 30026, landing 30009), and ftCo_8009A134 reading the
floor line's flags from the map (a dynamic platform's bit is not in the
flags cached at contact; Fire Fox on the rock form).

## Slippi codes (2026-10-03)

Slippi recordings run the transformation preload code, and some the Frozen
Stadium code; both live in the controller as `Transformation::preload` and
`Transformation::frozen`, off in retail (docs/SLIPPI.md, "Pokémon Stadium
codes"). With preload the form is drawn on the first waiting tick and
announced the tick the wait ends, so no read is polled and no external
event is consumed. Witnesses: `slippi_ps_preload_fox_marth4`,
`slippi_ps_frozen_fox_marth4`. A saved boundary run with the preload code
reads the code's two fields from map 2 (+0xF0, +0xEC).

## Not ported

- A model of the read latency: standalone runs use the default policy.
- Presentation of the forms (the renderer skips absent models).
- Saved boundaries mid-transformation or in a transformed form.
- Screen modes 9 (a player out of stocks, gm_8016B8D4) and 12 (match end).

## Next boundaries (explorer)

With the recorded read completion, a 10-seed batch (seeds after 20, 30
cases) from `start_ps_fox_marth4` finishes or plays out every case without a
fault, and its three bridged samples
(`corpus_v3_ps_fox_marth4_{ee133b82f_p0,ecdf8887e_p1,e2b9e1400_p2}`) gate
exactly through the first transformation. `particles-diff` still finds
particle-position differences (no RNG or generator difference) in several
Pokémon Stadium corpus matches, as it does in registered Final Destination
ones; the gates do not compare particles.
