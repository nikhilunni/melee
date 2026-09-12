# C-stick throws and Fox throw articles

This packet closes directional C-stick throw selection for Fox and Marth on
Final Destination and implements Fox's authored throw blaster callbacks.

## Retail behavior

- `ftCo_800DD1E4`, `ftCo_800DF7F4`, `ftCo_800DF844`, and `ftCo_800DF878` select
  main-stick horizontal, C-stick horizontal, up, then down. Retail C-stick down
  requires both current and previous samples below the threshold; a one-tick
  pulse does not throw. Main-stick down instead uses a threshold crossing.
- `ftAction_800718A4` at `800718B0` decodes all lower 26 operand bits. Shifting
  the operand incorrectly decoded the authored reversal word as release.
  `ftCo_800DD724` consumes the reversal before linked throw release.
- `ftFx_Throw_Anim` (`800E6CDC`) controls the held blaster and throw lasers.
  The two subtractions at `800E6EFC/800E6F00` precede `atan2f` without fusion.
  Common forward throw does not call this character hook in retail.
- `ftColl_80077C60` (`80077DE4`) applies the captured-item damage multiplier
  after owner staling. `ftCo_8008EC90` retains the current thrown motion for
  low-damage contacts: Fox's forward-throw collateral hit causes linked
  hitlag without replacing the victim's throw animation.
- The release-owner callback (`fn_800DE798`) runs inside motion entry before
  initial commands (`80069BC4..BE0`, `8006A0BC`). Restoring it afterward skipped
  collateral damage-animation hitboxes. `ftCo_DamageFly_Phys` (`80090030..C8`)
  clears those hitboxes below PlCo+1C8 before shared knockback decay. Its
  separate squared-speed products/sums and three-step square root are retained.
  Both previously ignored throw scratch tests now pass without expected changes.
- Retail laser creation (`it_8029C504`) selects its initial motion once.
  Removing the port's extra motion-0 initialization fixes a prematurely
  advanced hitbox phase in throw lasers. Same-group hitbox preservation and
  item scheduler ordering are retained.

## Acceptance scenarios

The fourteen `cstick_throw_{direction}_fd_{character}` scenarios cover each
character with `forward`, `back`, `up`, `down`, `down_pulse`, `main_priority`,
and `horizontal_priority`. Each records 300 ticks, ordered RNG and particle
state. The M5 tests additionally compare item kind, owner, position, velocity,
facing, motion, lifetime, and hitbox phase. Internal scratch assertions cover
throw flags, command variables, linked hitlag, and Fox's blaster lifetime.
All oracle files remain ignored and are backed up outside the repository.

These cases use Marth in port 0 and Fox in port 1, with the initial inward
facings. Swapped-port and combination coverage remain part of the matchup
completion milestone. The generic item motion-change hitbox-preservation
flags need a separate audit before broader item kinds are added.

## Validation

Both `cargo gate` and `cargo gate --release`: 1,249 passed, 0 failed,
1 existing ignore. Workspace all-target clippy, formatting, 220 harness tests,
and schema generation check pass. All fourteen throw trajectories allocate
zero heap memory during simulation. The two formerly ignored throw scratch
regressions are re-enabled with unchanged expected bytes.

The rebuilt native app passes resize/focus/pause/keyboard/export smoke at
232 ticks; the exported recording also replays headlessly for 232 ticks.
The stripped simulator is 3,926,840 bytes, up 56 bytes from 3,926,784.
This is a size measurement, not a new passing claim for the separately
tracked performance ceilings.
