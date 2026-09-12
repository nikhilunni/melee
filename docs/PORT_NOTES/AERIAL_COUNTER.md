# Aerial Counter and counterpart transitions

Marth's aerial down-special previously stopped at `ftMs_SpecialAirLw_Enter`.
The port now installs both aerial rows, divides entry momentum, applies the
stance's authored gravity/friction, selects the aerial counterattack on contact,
and returns to Fall when the move finishes offstage. Landing preserves animation,
commands, active defense and hitboxes; counterattack transitions also keep effects.
Ground support-loss handlers now use Counter's own counterpart transitions,
including its ordinary remaining-jump policy.

Retail sources: `ftmarsspeciallw.c`, `ftMs_SpecialAirLw_Enter` (80138A30),
stance physics (80138C5C), hit physics (80138FE8), counterpart transitions
(80138D38/80138DD0/80139080/801390E0), hit response (80139140).
The entry divide was checked at 80138A60; physics uses the audited shared
gravity/friction primitives. No decomp or expected-value modifications.

Four 300-tick Dolphin gates pass: `aircounter_fd_marth`,
`aircounter_landing_fd_marth`, `aircounter_hit_fd_marth`, and
`aircounter_fall_fd_marth`. They cover stance entry/landing, aerial retaliation,
landing during retaliation, offstage completion and KO/respawn, with exact
fighter state and ordered particle RNG. Raw scratch additionally checks damage,
Counter window/volume, jump count, ECB lock, hitboxes, hitlag and animation speed.
The three allocation scenarios all allocate zero during simulation. Captures
are ignored local data, mirrored externally. The hit scenario uses the existing
four-stock boundary because its KO otherwise ends the game within 300 ticks.

Release workspace gate and all-target clippy pass. Debug focused gates/scratch
and allocation tests pass; baseline workspace runtime tests pass. Its later
rustdoc process saw stale dependency artifacts after the focused builds, so
workspace doc tests were rerun separately. Three pre-existing ignored tests
are unchanged. The earlier UI replay
fault scenario now succeeds through aerial Counter and agrees with headless replay.

Remaining coverage: deliberate support loss during each Counter phase and
projectile contacts with Counter need dedicated retail scenarios. This packet
does not claim those combinations complete. The user has explicitly kept
gameplay work moving while the documented binary-size/performance debt remains.

Changed files: `crates/ft-mars/src/{special_lw.rs,lib.rs,init.rs}`,
`crates/melee-lib/src/frame/combat.rs`, `crates/melee-sim/tests/{m5_gate.rs,alloc_gate.rs}`,
four `harness/scenarios/aircounter*.toml` files, `TRACKER.md`,
`docs/MATCHUP_COMPLETENESS.md`, and this note.
