# Timeout, Sudden Death and the Bob-omb rain

Witnesses (recorded 2026-09-12 and 2026-09-26, machine-local traces):

- `timeout_tie_fd_marth` (3839 ticks) from `start_fd_marth4_timer60.sav`: an
  idle one-minute match through TIME! and the frozen hold to the scene exit;
  `timeout_tie_fd_marth_cold` rebuilds it from parameters.
- `sudden_death_start_fd_marth` (1697 ticks) from
  `sudden_death_start_fd_marth4.sav`, captured at the Sudden Death scene's
  first frame (after TIME!, A pressed twice on the tie screen);
  `sudden_death_start_fd_marth_cold` rebuilds that scene from parameters.
- `melee-lib/tests/match_endings.rs`: `Match::new` with a 60-second limit,
  stepped to `Finished(SuddenDeath)`, then `Match::sudden_death()` to the
  Bob-omb KO, compared with both retail traces every tick.

- `sudden_death_bombs_fd_marth` (1300 ticks) and `sudden_death_idle_fd_marth`
  (1576 ticks, through the Bob-omb KO and GAME), both from
  `sudden_death_fd_marth4_timer60.sav`, taken while the GO banner plays after
  a one-minute Marth/Fox timeout tie. Gated by
  `m5_gate::sudden_death_bomb_rain_matches_through_game` (state, items,
  particle RNG order); `particles-diff` also matches every tick.
- `timer_natural_tie_*_flow_fd_marth`: VI-frame flow captures of the whole
  path (timeout at frame_count 3600, the intermediate scene, the Sudden
  Death scene, GAME and results). Not yet gated.

## Match clock and banners

`lbl_8046B6A0` holds the clock: `hud_enabled` (+0x05) is set by
`fn_8016B784` when the GO banner (ifStatus element 4) finishes, and
`frame_count` (+0x24) then advances once per frame in the scene's OnFrame
(`fn_8016CD98`) while no outcome is decided. The countdown banner (element 3
for Versus, 1 for Sudden Death) releases input when it ends and starts GO.
`melee-lib::banner` models all three; GO is preloaded so starting it never
allocates. A savestate boundary resumes the running banner from MEM1 with its
saved AObj frame and flags (a fresh request would set `AOBJ_FIRST_PLAY` and
finish one tick late).

## The rain

`Ground_801C0C2C` (every stage, s_link 10) drops a Bob-omb when the
Sudden Death rule (`StartMeleeRules.x6`) is set, the clock is past 1200 and
more than 30 frames have passed since the last drop (`stage_info.x9C`).
`Ground_801C0A70` picks half the time a random player slot (x ± 50 at the
blast top − 5), otherwise `Stage_80224FDC` draws item markers 0x7F..0x93 from a
shrinking range, then spawn markers. `melee-gr::bomb_rain` holds the logic;
the scene resolves the markers once at load.

## The Bob-omb (`it-bombhei`)

`it_8027D670` faces the item by `ftLib_800864A8` (a tie draws `Randi(2)`),
creates it airborne with an initial collision pass, and lights it at once
(state 5). Leaving the ground switches to state 6 without restarting the
script (`Item_80268E5C` flags 0x1). The fuse (`it_8027D820`) pulses the model
scale and detonates after 90 frames; a landing faster than (0.8, 0.7)
detonates at once. `it_80280B60` hides the model, starts the 80-frame explosion
lifetime, spawns efSync 0x410 (generator 0x22A), a radial gust and state 11,
whose script owns the explosion hitbox and calls a subroutine that requests a
Medium camera quake (efAsync kind 8, flushed at the item's link 9) after three
`it_80278800` offset draws.

Shared item engine additions (`melee-it`): common-item Articles from
`itPublicData->x4[kind]` with each motion state's article row; scripts with
subroutine calls; persistent map collision (`it_80275E98`, `it_8026D62C`,
`it_8026E414`); airborne spin (`it_80274658`, `it_80274A64`), which also
turns the fixed ECB; the common item lifetime; and the item hitbox radius
stored at `1/scl` (`it_80275594`), which contacts scale back.

## Timeout and the transition

`MatchRules::time_limit_seconds` enables the countdown timer
(`timer_seconds = limit`, `unk_2C = 59`, fn_8016E730). fn_8016CD98 advances
it with the frame count; gm_GetMatchOutcome reports OUTCOME_TIMEOUT at 0
seconds and 59 frames, one frame after frame_count reaches the limit. The
results screen ranks a timed-out stock match by stocks; a tie reports
`MatchOutcome::SuddenDeath`.

The Versus scene then holds (fn_8016D634: `unk_30++ <= rules.xD`, 110 in
Versus), sets unk_0 = 3 and leaves on the next frame: 114 frozen ticks from
the outcome tick. The tie screen draws no RNG, so the Sudden Death scene's
boundary seed is the exit seed after its 8 setup draws (verified against the
flow capture: 392813310 → 4258399878). `Match::sudden_death` runs those ticks
on a clone and builds the scene with `MatchRules::sudden_death` (one stock,
300%, countdown element 1, the rain, Sudden Death music without the unlock
draw).

The GO banner's GObj is created during the countdown's s_link 0 proc and
steps in the same pass; FD's `Ground_801C0C2C` and `fn_801CADBC` GObjs are
created when the countdown releases the stage (Ground_801C0FB8), so the
scheduler registers them then.

## Remaining

- Bob-omb walking, turning, pickup and throwing, wall/ceiling bounces
  (`it_80276FC4`) and item-versus-item explosions.
