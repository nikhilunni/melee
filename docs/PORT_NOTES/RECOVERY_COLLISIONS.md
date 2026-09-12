# Fox recovery collision transitions

The five retail scenarios exercise actual ground/air counterparts and the floor
rebound, rather than only completing each special move from a fixed starting
state. Each has 300 ticks of fighter, particle and RNG evidence, plus 150 ticks of
local bone transforms for both players.

| Scenario | Observed transition |
| --- | --- |
| `illusion_start_landing_fd_fox` | Air startup 350→ground startup 347 at 57 |
| `firefox_charge_landing_fd_fox` | Air charge 354→ground charge 353 at 86 |
| `firefox_ground_launch_fd_fox` | Ground launch 355 at 73→air launch 356 at 79 |
| `firefox_floor_rebound_fd_fox` | Air launch 356→bound 359 at 94 |
| `firefox_end_air_landing_fd_fox` | Air ending 358→ground ending 357 at 121 |

The callbacks preserve the retail motion frame, command cursor, owned graphics,
velocity, jumps, and hitboxes according to the particular transition flags.
Fire Fox bound has its own row and motion 312, TransN vertical velocity, and
immediate model 4 rebound effect. The ending landing resumes frame 13 through
UpdateCmd with the 0x5000 preservation policy and the explicit animation step.
Primary references are ftfoxspecialhi.c 800E71AC..800E83E0 and
ftfoxspecials.c 800E9DF8..800EAD68; fusion sites are cited at their arithmetic.

## Two shared state-lifetime fixes

The all-bone gate first failed on Fox tail bone 18 during KneeBend, before any
special. Fox's Wait relinquishes dynamic ownership; retail drawing updates the
animation-owned tail joint matrices. ftCo_8009CB40 reads those cached matrices
when the next motion re-enables springs. The headless loop had stale endpoint
matrices for bones 19 and 20. Between observations, the library now demands only
animation-owned dynamic-joint caches of a visible active fighter. It never forces
matrix dirtiness and does not add a display pass to an imported partial tick.
The diagnostic presentation path still evaluates a clone.

The longer ground-launch case loses a stock and reaches Rebirth at 218. Retail
Fighter_UnkInitReset 80067C98 retains cmd_vars 2200..220C. The library now retains
those four words across reset, with subsequent commands owning initialization.
The raw scratch assertion remains unchanged; no expected oracle values were
edited.

## Verification and remaining coverage

Full debug and release workspace gates each pass 1,257 tests, with zero failures
and one existing ignore. All five local-SRT replays, expanded move counters,
items and zero-allocation checks pass. All-target clippy, formatting, 220 harness
tests and schema validation pass. Native resize/focus/pause/keyboard smoke and
its exported headless replay pass at 234 ticks. The stripped simulator is
3,943,568 bytes, up 16,728 bytes from the throw packet; no new full performance
or duplication census is claimed. Local tick, particle, ledger and bone captures are
ignored and mirrored in the user's melee-data archive.

These five witnesses do not close every recovery edge: Illusion travel landing,
Fire Fox charge/ending departure, and wall/ledge combinations still need actual
transition witnesses. Early sweep scenarios that stayed airborne or simply
finished on the floor are not counted as those witnesses. General platforms are
outside this Final Destination packet. Broader matchup completion remains open.
