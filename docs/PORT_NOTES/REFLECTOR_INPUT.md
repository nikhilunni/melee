# Reflector turn and aerial jump cancel

Packet scope: grounded and aerial Reflector turns, release during the turn,
turn priority over jumping, turn landing, and aerial jump cancel from button or tap input.
Other phases' ground/air preservation and edge departure remain separate
coverage work; this packet must not imply complete Reflector support.

Retail references: `ftFx_SpecialLw_Turn` (800E8F20), ground/air Turn Anim
(800E8FDC/800E90EC), Turn_Check (800E942C), Hit_Check (800E9564),
`ftPartGetRotZ` (80075F48) and `ftCo_800CB870` (800CB870). Turn actions 364/369
reuse Loop submotions 314/318; they require no new animation assets.

Turn entry preserves effects and the live reflector, initializes its integer
countdown with `fctiwz`, clears command variable 0 and advances the turn once
immediately. The first advance decrements the countdown and flips facing.
Subsequent Turn animation callbacks latch B release, decrement release lag,
advance rotation and select Loop or End when the countdown expires. Turn
IASAs are empty. The loop checks turn before jumping.

`ftPartGetRotZ` is misleadingly named: both branches read the joint's Y
rotation (`lfs` at JObj +20). The turn uses separately rounded `180 / frames`
followed by `fnmsubs` with the authored radians-per-degree constant
(800E8FB8/800E8FBC; inlined copies at 800E90AC/800E90B0 and
800E91BC/800E91C0). Do not substitute a separate multiplication/subtraction.
The effect callback does not respawn the loop effect while it is already owned.

Seven 300-tick Fox–Marth scenarios are captured from `idle_fd_marth.sav`, with
Fox in slot 1:

- `reflectorturn_fd_fox`: Turn 364 at tick 41, Loop 361 at tick 44.
- `airreflectorturn_fd_fox`: Turn 369 at tick 55, Loop 366 at tick 58.
- `reflectorturn_release_fd_fox`: Turn 364 at tick 61; releasing B while
  turning enters End 363 at tick 64, after the minimum hold has expired.
- `airreflectorturn_landing_fd_fox`: AirTurn 369 at tick 80 becomes grounded
  Turn 364 at tick 82, then Loop at tick 83. Its previous collision callback
  panicked at this landing. The dedicated callback uses `ft_80081D0C` and
  preserves common ground/air state plus graphics (0x0C4C5082), as in
  `ftFx_SpecialAirLwTurn_GroundToAir` (800E93A4), then clamps drift.
- `airreflectorturn_priority_fd_fox`: simultaneous backward stick and X
  must turn before considering an aerial jump.
- `airreflectorjc_fd_fox` and `airreflectortapjc_fd_fox`: both enter
  JumpAerialF (27) at tick 55.

Acceptance: all seven fighter/ordered-particle gates, raw Reflector countdown,
release/activation and jump-count comparisons, 80 ticks of exact root-Y bone
rotation for ground/air turns and 110 ticks across turn landing, zero simulated-tick allocations, full workspace
debug/release gates and all-target clippy. Verification status is tracked in
TRACKER.md. Recorded data remains ignored and backed up outside the repository.

Changed files: `crates/ft-fox-family/src/special_lw.rs`,
`crates/melee-ft/src/fighter/jump.rs`, `crates/melee-lib/src/frame/combat.rs`,
`crates/melee-sim/tests/{m5_gate.rs,alloc_gate.rs}`, seven scenario TOMLs,
TRACKER, the matchup inventory and this note.

Verified: full debug and release `cargo gate` each passed 1,223 tests with
zero failures and three existing ignores. All-target clippy, formatting and
220 harness tests passed. Rebuilt native smoke and exported headless replay
passed at 232 ticks. The stripped simulator grew from 3,926,672 to 3,926,784
bytes (+112) against the saved pre-packet binary; existing size debt remains
and this measurement does not constitute a new full performance-gate pass.
