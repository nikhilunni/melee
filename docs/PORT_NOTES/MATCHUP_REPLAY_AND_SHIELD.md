# Matchup coverage: replay capture and projectile shields

First implementation packet of `docs/MATCHUP_COMPLETENESS.md` (2026-09-11).
The larger matchup-completeness milestone remains open.

## Reproducing playtest failures

The native app records the exact normalized inputs consumed by each simulation
tick, including the input whose step failed. Simulation faults stop the match,
preserve the original diagnostic and one-based attempted tick, and automatically
save a JSON recording under the OS temporary directory's `melee-replays` folder.
The error dialog includes its absolute path. Save important recordings elsewhere
before OS temporary-file cleanup. Cmd-S / **Save Replay…** also exports the current
match; a user-selected existing destination is replaced atomically. Automatic
fault saves use exclusive creation so previous fault evidence is not overwritten.

```sh
cargo run -p melee-replay -- harness/roms/files /absolute/path/to/replay.json
```

The command loads the same configuration and seed through `melee-lib`, verifies
the loaded asset fingerprint, and replays without a renderer. A simulation failure
reports its attempted tick and exits nonzero. An old failing recording succeeds
after the gameplay bug is fixed; the original fault is metadata, not an expected
failure that must be preserved.

`melee-replay` owns versioned serialization outside the core library. Controller
values are serialized as integer float bit patterns, preserving signed zeros and
all normalized analog values. The fingerprint uses `DefaultHasher::new()` over
named file bytes as actually loaded (including common and character article
assets). The fixed constructor is deterministic, unlike randomly seeded maps.
It detects accidental asset drift; it is not a cryptographic authenticity check.
Rust toolchain or loader changes can change it, so compare recordings with the
same toolchain and loader. This is not a cross-version state format. Buffered
hashing avoids the measured load regression of the initial byte-at-a-time FNV
implementation.

Recording reserves 108,000 ticks (30 minutes at 60 Hz, roughly 12 MiB) up front.
At capacity the session stops and exports rather than discarding its cold-start
prefix. Healthy stepping/recording performs no I/O or allocation. Serialization
and fault handling can allocate. Pause/focus changes add no artificial ticks;
reset clears the recording along with the match.

## Laser shield behavior

The original code panicked whenever an eligible laser existed while a shield was
active, before checking overlap. The port now checks the shield capsule first,
records the shield's health damage and strongest impact, and dispatches the
item's shield-hit/deflection callback at item process link 14. The attacker does
not receive the melee-hit shield recoil. A shield miss continues into the normal
hurtbox checks. Item shieldability and hurtbox eligibility are independent.

Retail references: `ftColl_80077688` (0x80077688), `lbColl_800077A0`
(0x800077A0), `lbColl_80007DD8` (0x80007DD8), `lbVector_AngleXY`
(0x8000D790), `Item_80269DC8` (0x80269DC8), `Item_8026A294`
(0x8026A294). Float sites were read with `harness/asm.py`; the geometry uses
explicit FMA ordering, three/four-step MSL square roots where retail does, and
the shared SDK vector normalization. Source bytes and the decomp are unchanged.

Four newly recorded scenarios cover mature shield, analog lightshield, an
airborne laser and grazing deflection against Marth. The grazing case retains
the laser at tick 66 and turns its velocity upward; the other three cases do
not exercise that callback. Data stays in ignored `harness/traces` and has been
copied to the existing external mirror. Scenario TOMLs and tests are source files.

Projectile powershield reflection remains an explicit unsupported contact.
The rest of the audited Fox/Marth gaps are listed in `MATCHUP_COMPLETENESS.md`;
this packet does not claim complete projectile, shield, or matchup coverage.

## Mechanical checks

- `melee-replay/tests/recording.rs`: controller bit roundtrip, exact direct/replay
  continuation, asset/version mismatch rejection, retained failing input, bounded
  capacity/reset, and allocation-free recording plus stepping.
- `melee-platform/tests/replay.rs`: UI latching/pause/focus/reset roundtrip and
  automatic first-fault save, headless reproduction, and stop-until-reset behavior.
- `melee-sim/tests/m5_gate.rs`: `laser_shield_fd_marth`,
  `laser_lightshield_fd_marth`, `laser_shield_air_fd_marth`,
  `laser_shield_deflect_fd_marth`: 300 ticks each,
  fighter/item bit comparisons and ordered particle RNG; original laser gate retained.
- `melee-lib`'s `laser_shield_matches_retail_health_stun_and_hitlag`: raw shield
  health, lightshield amount, pushback, animation rate, hitlag and hitbox checks.
- `laser_shield_fd_marth_allocation_budget`: zero simulate-only allocations.
- Strict `cargo gate`: 1,192 passed, zero failed, three existing ignores. The
  subsequently added grazing scenario and overwrite regression also passed in
  focused debug runs. After the fingerprint optimization, all six replay tests
  and both platform replay tests passed again.
- Final `cargo gate --release`: 1,194 passed, zero failed, three existing ignores.
  The ignores are two pre-existing throw scratch comparisons and a schema doc
  example; this packet does not change them.
- Workspace all-target clippy with `-D warnings`, all six release-math optimization
  levels, harness pytest (220 tests), formatting and diff checks pass.
- Rebuilt native smoke passes: 233 ticks, keyboard movement, resize/focus/pause,
  replay export. Its exported recording replays through the headless CLI. The
  first smoke attempt during other verification failed the timing threshold;
  both subsequent isolated runs passed without changing that threshold.
- Performance: see the final serial result in `docs/PERF.md`. An unchanged HEAD
  source snapshot measured 3,910,040 stripped bytes, 178.86 ms load and 26.11 ms
  per 600 ticks. The first packet version measured 3,910,032 bytes, 195.61 ms
  load and 25.90 ms per 600 ticks. Binary-size debt predates this work; the new
  loading cost prompted the buffered-hash correction above. Baseline source was
  kept outside the checkout; no commit or expected-value changes were made.
  The final serial measurement is **REGRESSION**: 3,926,592 stripped bytes,
  184.015 ms load, 26.036 ms per 600 ticks. Relative to the unchanged snapshot,
  size is +16,552 bytes (0.42%), load +5.16 ms (2.88%), and tick time slightly
  lower. Duplicate-definition counts and the relative timing gates pass, but
  the fixed size/load/tick ceilings remain red (3,747,632 bytes, 182.6 ms,
  25.947 ms). No baseline or ceiling was changed. This remains an open acceptance
  item; do not describe the overall performance gate as passing.

## Changed files

- Workspace manifests and lockfile; new `crates/melee-replay/{Cargo.toml,src/config.rs,src/lib.rs,src/main.rs,tests/recording.rs}`.
- `crates/melee-lib/src/{assets.rs,diagnostics.rs,scene_items.rs,frame/combat.rs}`.
- `crates/melee-platform/{Cargo.toml,src/session.rs,src/ffi.rs,include/melee_platform.h,tests/replay.rs}` and `apps/macos/main.swift`.
- `crates/melee-ft/src/fighter/{damage.rs,shield.rs}`, `crates/melee-it/src/{desc.rs,engine.rs}`, `crates/melee-lb/src/{lib.rs,shield.rs}`.
- `crates/melee-sim/tests/{m5_gate.rs,alloc_gate.rs}` and the four named `harness/scenarios/*.toml` files.
- `TRACKER.md`, `docs/MATCHUP_COMPLETENESS.md`, generated `docs/PERF.md` evidence,
  and this report.
