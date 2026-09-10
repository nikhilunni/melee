# S10: match flow: the scripted four-stock match

2026-09-10, combat lane, ported directly. `match_fd_marth_scripted` (1,600
ticks from the stock-4 `idle_fd_marth4.sav`) is exact: **1600 ticks, 62 keys,
0 divergences**, zero simulate-only allocations. Marth KOs Fox four times with
charged forward smashes; after each respawn Fox drops from the platform on a
stick tap, walks to the left edge, teeters, and is smashed off once his
respawn invincibility expires; the GAME banner covers the last 72 ticks.

## What was ported

- `ftCo_RebirthWait_IASA` (`life.rs::revival_input`): B -> buffered special;
  LR+A -> explicit stop (item pickup / catch timer, PlCo `+1C` not loaded);
  digital shoulders -> air dodge; aerial jump; otherwise a held shield bit,
  D-pad up, the squat / turn / walk stick tests drop into Fall. Either way the
  fighter receives the revival invincibility (`ftColl_8007B7A4`, PlCo `+5D8`)
  and colour animation 9.
- Ottotto 245 / OttottoWait 246 (`teeter.rs`, PlCo `+474/+478/+47C`,
  `x4C_sfx->x18`): entered from the Wait ground test with velocities cleared;
  the animation ends into OttottoWait; the input callback is Wait's IASA list
  minus Escape, FoxTaunt and Walk (`OTTOTTO_PREDICATES`) plus the teeter walk
  threshold; the collision callback is `ft_800827A0` (`map_stop_at_edge`):
  when the fighter is farther than `edge_distance + edge_margin` from the
  floor endpoint it returns to Wait, when unsupported it falls.
- Retail `Fighter_procUpdate` integrates velocity and the fighter-overlap
  nudge unconditionally after the state's physics callback; the port
  integrates inside per-state callbacks, so a row with an empty retail
  physics callback still needs `finish_ground_update`. The nudge at the edge
  pushes the teetering fighter 0.3 past the endpoint and the stop-at-edge
  collision snaps it to the vertex (y exactly 0.0); missing that tail showed
  up as 0.0001 vs 0.0 in the third stock.
- The victim-state whitelist in `damage.rs` is gone; only the DamageIce
  victim remains an explicit stop (`ftcoll.c:199`).
- The scenario frame cap is 4,000.

## Retail facts from the recording

- A stock-1 savestate ends the match on the first death; the scene reset
  (tick counter -> 0) comes about 113 frames after the last KO.
- Respawn invincibility lasts about 120 frames after the platform drop; a
  smash 69 frames after landing whiffed.
- A tipper forward smash at 0% flies about 25 units before touching down.
- Pressing L from Wait enters GuardReflect 182 then Guard 179; an analog
  half-press enters GuardOn 178.
