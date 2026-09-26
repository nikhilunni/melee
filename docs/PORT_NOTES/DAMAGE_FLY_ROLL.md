# DamageFlyRoll

Common motion state 91 now loads animation/script 181 and uses the shared damage
callbacks for every fighter. The retail DOL's `ftData_MotionStateList[91]` at
0x803C3360 supplies that animation ID. Bone lookup uses the fighter's part data;
knockback enter/exit still use `CharacterTable`. No character-specific branches
were added.

Retail `ftCo_Damage.c` callbacks are Anim 0x800901D0, IASA 0x80090324, Phys
0x8009035C and Coll 0x800904C0. Address 0x8009027C is inside Anim, not its entry.
Anim retains the damage trail and hitstun/character-exit work, then enters
DamageFall through `ftCo_80090780` immediately when `x221C_b6` clears. It does
not wait for animation end or consume the DamageFly buffered jump.

The roll rotation is facing (`Fighter+0x2C`, not an attribute) times
`atan2f(self_vel.x + kb_vel.x, self_vel.y + kb_vel.y)`. Retail uses separate
`fadds` at 0x800903D8/DC and 0x80090470/74, and `fmuls` at 0x800903F0 and
0x80090488; there are no fused sites. Both writes are preserved around the
thrown-hitbox cleanup and precede global knockback decay/integration.
`ftPartSetRotX` (0x8007592C) redirects quaternion primary joints to their Euler
secondary joints.

The first bone replay exposed the missing initial rotation during hitlag:
Fox XRotN was zero at tick 135 (t125/crouch) or 138 (dtilt). Retail
`ftCo_8008DCE0` initializes it after frame-zero animation and before the trail
countdown at 0x8008E2F4–0x8008E334. This write uses the same helper; its `fadds`
are 0x8008E308/30C and its `fmuls` is 0x8008E320. Expected traces are unchanged.

The three fixture stems are `damage_fly_roll_t125`,
`damage_fly_roll_dtilt_t132`, and `damage_fly_roll_crouch`; each uses the scenario
suffix `_fd_fox_candidate`. Tests for each stem are:

- `m5_gate`: the stem itself, 450 ticks of combat/RNG/ordered particle events.
- `frame::recovery`: `_bones` and `_particles`, full local SRT and ordered
  particle simulation fields.
- `frame::combat`: `_scratch`, hitstun ownership/countdown, trail timer, buffered
  jump and the existing raw combat fields, plus XRotN rotation.
- `alloc_gate`: `_allocation_budget`, zero allocations during simulation.

Retail P2 motion transitions confirm the witnesses: t125 enters 91 at tick 135
and DeadRight (2) at 177; dtilt enters 91 at 138 and DamageFall (38) at 181;
crouch enters 91 at 135, DownBoundU (183) at 139 and Fall (29) at 143. Thus dtilt
covers roll hitstun expiry and crouch covers the floor-contact sequence.

The actual bone captures and their `.bones_scenario.toml` metadata contain
450 ticks, not 150. The new tests verify all 450. Raw fighter allocations do
not contain the pointed-to XRotN JObj, so the scratch tests also stream the
companion bone recording to assert its rotation. The initial test-length
assertion was corrected to match the captured metadata; no oracle was edited.

Collision reuses DamageFly's existing floor tech/down-bound handling. Its
pre-existing wall/ceiling gap remains: the shared damage collision path does
not yet implement `ftCo_800C1D38`, `ftCo_800C23A0` or `ftCo_800C17CC` (wall tech,
ceiling tech and FlyReflect states). These three FD recordings do not verify
those branches. This packet does not claim complete wall/ceiling damage
behavior or retail replay coverage of every character.

Validation: `cargo gate` and `cargo gate --release` each pass 1,458 tests,
with zero failures and one existing ignored documentation test. All 15 new
checks pass in both profiles; simulation allocates zero times in all three
fixtures (449 measured ticks after warmup). `cargo clippy --workspace
--all-targets -- -D warnings`, `cargo fmt --check` and `git diff --check` pass.
No traces were changed or re-recorded. No commit was made; TRACKER.md and
Cargo.toml members were not edited.
