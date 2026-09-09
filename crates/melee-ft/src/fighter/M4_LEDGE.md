# M4-T5: Fox air dodge, wavedash and ledges on Final Destination

Implemented and verified for the supplied scenes; no commit. Air dodge and
wavedash each match **300 ticks, 49 keys, zero divergences**. Ledge matches
**420 ticks, 49 keys, zero divergences**. Callback replays also match raw
movement scratch, command clocks, hurt status and timers; the full scene
produces the matching RNG stream and ordered particle call sites.

## Port

- EscapeAir stick deadzones, retail atan2/sin/cos direction, preserved pre-dodge
  momentum and item-throw timer, script-controlled decay/ordinary-air-physics
  switch, and existing typed subaction hurt status. Digital shoulders enter
  from the shared Fall/Jump/JumpAerial input path. EscapeAir collision uses
  ordinary airborne collision, without ledge grabs.
- LandingFallSpecial uses common Landing callbacks with interruption disabled,
  ground momentum conversion and the normal Landing animation length to derive
  its rate. The rate is installed before frame-zero animation and commands.
  Installing it afterwards matched the 49-key trace but failed the raw command
  timer at the landing tick; the expanded callback test caught that ordering.
- JumpB and the shared basic JumpAerialB selection use the retail facing-relative
  threshold. Choosing a backward animation does not change facing.
- CliffCatch checks down input and the disabled-grab flag after the airborne
  map query; cooldown selects the map path without ledge detection. Catch
  chooses the ledge ID/facing, clears momentum, locks the ECB, immediately
  advances the animation and snaps to the map endpoint plus animated TransN.
- CliffWait retains the ledge ID, sets its damage-dependent wait timer, latches
  neutral input and installs timed intangibility. Attack, escape, jump, stick
  option and timeout checks retain their ordering. Untaken option bodies and
  the timeout's DamageFall entry fail explicitly with C locations.
- CliffJump1 stays attached through TransN. CliffJump2 retains the inactive
  wait word, uses the fused horizontal launch addition and skips gravity on
  its first physics callback. Quick and Slow use their own archive descriptors
  with shared callbacks; only Quick is exercised by the supplied recording.
- CliffCatch/Wait have a distinct camera callback: update the camera box, mark
  airborne ledge occupancy and notify the supporting stage joint at link 18.
  Jump1/2 use the ordinary camera callback, as in the retail table.
- `CharacterCallbacks::on_landing` owns character resource-reset differences
  from ftCo_Landing.c:51-83. `air_dodge_tether` owns Link/Young Link/Samus's
  ftCo_AirCatch.c:54-79 branch. Fox uses ordinary defaults; unsupported character
  bodies remain explicit boundaries inside the hooks.

Resources are owned archive data. Gameplay reads neither traces nor tick
numbers; test fixtures supply only the initial boundary and recorded pads.
The callback-only oracle additionally supplies external pre-draw seeds to
isolate fighter behavior; the full-scene gate independently produces RNG.
No expected values, field exclusions or tolerances were changed.

## Corrections to the scenario notes

1. **262/263 are CliffJumpQuick1/Quick2**, selected at zero damage. Slow1/Slow2
   are 260/261. The recorded transition ticks agree with the task; the names
   differ. This follows ftmotionstates.c:2995-3037 and ftCo_CliffJump.c:35-43.
2. The ledge ledger's randomized dust calls are at **246 and 283**. Tick 246 is
   the CliffJumpQuick2 launch, not a landing. The ordinary Landing state starts
   at 283. The initial-emission-count draw `hsd_8039F05C+0x1F4` is also at 246.
3. EscapeAir has a **single PlCo decay multiplier**, not a velocity-decay table.
   Command variable 0 switches from repeated X/Y multiplication to ordinary
   air physics. Neither air-dodge scene reaches FallSpecial (35) before landing.
4. LandingFallSpecial entry lives in **ftCo_Landing.c:103-113**. It uses the
   normal Landing animation length (`Fighter.x2EC`, populated by fighter.c:836),
   not the LandingAir helper's current-motion length.

## Effects and RNG

| Request | Source / dispatch |
|---|---|
| CliffCatch 0x41C | ftcliffcommon.c:110; async kind 2 with no bone and an absolute map endpoint; efasync.c:521-523 -> positional particle 93 (0x5D) |
| LandingFallSpecial 0x407 | existing landing command request and three randomized offset draws; efasync.c:305-307 -> positional particle 60 (0x3C), after the root-relative offset transform |
| Ordinary landing 0x404 and jump/launch animation graphics | existing effect dispatch and particle paths |

No new particle opcode, emitter arithmetic or RNG site was needed. The existing
`rng_sites.rs` entries retain their gating conditions, including F250's
nonnegative initial rate / kind-0x100-clear condition. All particle sites match
in order on every tick of each new recording, independently of final seed.

| Scene | Ledger records | Total draws | Randomized dust ticks |
|---|---:|---:|---|
| airdodge | 300 | 9,949 | 55, 186 |
| wavedash | 300 | 9,295 | 36, 128 |
| ledge | 420 | 13,430 | 246, 283 |

There are no particle field captures for these three scenes. The proven scope
is the 49-key scene trace, raw fighter replay and ordered RNG, plus regression
of the existing particle-field oracles. **Would you like a field-level particle
check? If so, please record dumps for `airdodge_fd_fox`, `wavedash_fd_fox` and
`ledge_fd_fox`.** No Dolphin run was performed.

## Retail audit

Read-only commands ran from `harness` with `UV_CACHE_DIR=/tmp/melee-uv-cache`.
The complete fused audit is saved at `/tmp/melee-m4-t5-fused.log`; full selected
instructions are at `/tmp/melee-m4-t5-asm.txt`. Symbols were also resolved
through `symbols.py`.

```sh
uv run python asm.py ftCliffCommon_80081298 ftCliffCommon_80081370 ftCo_CliffCatch_Anim ftCo_CliffCatch_Phys ftCo_CliffCatch_Coll ftCo_Cliff_Cam ftCo_8009A804 ftCo_CliffWait_Anim ftCo_CliffWait_IASA ftCo_8009AAFC ftCo_8009B1B8 ftCo_8009B2F8 ftCo_CliffJump2_Phys ftCo_80099A9C ftCo_EscapeAir_Anim ftCo_EscapeAir_IASA ftCo_EscapeAir_Phys ftCo_LandingFallSpecial_Enter ftCo_Jump_Enter ftCo_JumpAerial_Enter_Basic --fused
uv run python asm.py ftCo_80099A9C ftCo_LandingFallSpecial_Enter ftCo_8009B2F8 ftCo_CliffCatch_Phys
uv run python asm.py hsd_8039F05C efLib_CreateGenerator --fused
```

| Arithmetic | Retail instructions |
|---|---|
| Ledge X snap | **800815A8 fmadds**, TransN Z * facing + endpoint X |
| Ledge Y snap | 800815B8 fadds, separate rounding |
| CliffJump2 horizontal launch | **8009B368 fmadds**, facing * attribute + retained X velocity |
| Air-dodge direction components | 80099B4C / 80099B64 fmuls, after retail trig |
| Special landing rate | 800D5D08 fadds, then 800D5D14 fdivs |
| Backward selection, decay, timers and input predicates | no fused multiply-add sites |

Existing root-motion extraction, drift, integration and particle arithmetic
retain their earlier audited implementations. Archive parsing introduces no
new gameplay math or cross-layer dependencies.

## Validation

All checks use the local retail assets. Baseline `cargo gate`: **572 passed,
zero failed, one pre-existing ignored doctest**. Final workspace result:
**582 passed, zero failed, one pre-existing ignored doctest**. The final empty
suite line in the log is the last crate's doctest target.

| Exact command | Result |
|---|---|
| `cargo run -q -p melee-sim -- gate harness/scenarios/airdodge_fd_fox.toml` | 300 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/wavedash_fd_fox.toml` | 300 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/ledge_fd_fox.toml` | 420 ticks, 49 keys, 0 divergences |
| `cargo test -p melee-sim --test m4_gate` | 19 passed |
| `cargo test -p melee-ft --test movement_fox_states` | 13 passed |
| `cargo run -q -p melee-sim -- gate harness/scenarios/idle_fd_fox.toml` | 600 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/start_fd_fox.toml` | 600 ticks, 49 keys, 0 divergences |
| `cargo test -p melee-ft --test start_fox_bones_130` | 4 passed |
| `cargo test -p hsd-particle --test live_fd --test live_fd_dash --test live_fd_jump` | 4 passed across three targets |
| `cargo test -p melee-sim --test m2_gate` | 1 passed |
| `cargo gate` | 582 passed, 0 failed, 1 pre-existing ignored |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo fmt --all` | success |
| `git -c core.fsmonitor=false diff --check` | success |

Logs: `/tmp/melee-m4-t5-{baseline,gate,clippy,m4,states}.log` and
`/tmp/melee-m4-t5-final-00.log` through `07.log` for the named focused checks.
The earlier `final-08.log` is the initial clippy finding, superseded by
`clippy.log`. Clippy required simplifying the nametag predicate and removing
two unnecessary references in the boundary adapter; no behavior was relaxed.

Callback checks include all existing movement fields plus EscapeAir saved
momentum/timer, backward-jump scratch, landing interruption/retained word,
ledge ID/wait timer/neutral latch, launch latch, cooldown, hanging flag,
subaction hurt status and timed intangibility. The added approach test follows
the recorded pads from the initial idle boundary and varies only the final
grab gates: one-frame cooldown expires before collision, two frames suppress
the grab, and down-at-threshold / disabled-grab flags both prevent it.

## Limits

FallSpecial (35) is not entered by these scenes and remains an explicit
animation-completion boundary. CliffClimb/Attack/Escape, timeout into DamageFall,
grounded ledge-option collision, ceiling impacts, occupied-ledge arbitration,
held-item throws/pickup, tethers and other character-specific bodies remain
unsupported. Slow ledge jumps and JumpAerialB are wired through the shared
retail paths but have no supplied independent retail replay. Rendering, audio,
rumble playback and combat interactions are outside this slice.

Nothing under `harness/traces`, `harness/roms`, `harness/scenarios` or
`third_party/melee-decomp` was modified. No game data was added, no Dolphin
was run, and nothing was committed. Harness tests were not run because no
harness code changed.

## Files changed

- `TRACKER.md`
- `crates/melee-ft/src/collision/air.rs`
- `crates/melee-ft/src/fighter/air_dodge.rs`, `ledge.rs`
- `crates/melee-ft/src/fighter/{assets,effects,jump,landing,mod,procs,spawn,state}.rs`
- `crates/melee-ft/src/fighter/{README.md,M4_LEDGE.md}`
- `crates/melee-ft/tests/fighter_support/{mod,replay}.rs`
- `crates/melee-ft/tests/movement_fox_states.rs`
- `crates/melee-sim/src/{effects.rs,effects/dust.rs,frame.rs,initial_state/fighter.rs}`
- `crates/melee-sim/tests/m4_gate.rs`

## M4-T5 particle-dump follow-up (2026-09-09)

The supplied air-dodge, wavedash and ledge tick-boundary dumps pass the shared
full-field `hsd-particle/tests/support/dust_replay.rs` runner. The runner now
takes an explicit expected tick count: 300 for movement scenes and 420 for
ledge. Both dump and ledger lengths remain strictly checked.

| Test | Ticks | Compared fields | Ordered particle draws | Display-cache exclusions |
|---|---:|---:|---:|---:|
| `live_fd_airdodge` | 300 | 509,415 | 9,935 | 0 |
| `live_fd_wavedash` | 300 | 472,911 | 9,284 | 0 |
| `live_fd_ledge` | 420 | 669,918 | 13,418 | 0 |

**All fields matched on the first replay**, including every dumped
generator/particle/AppSRT field, ordered draw sites and final seeds. No dust
routing, attachment, particle interpreter or arithmetic changes were needed.
The fixtures contain the production port's external spawn requests and joint
inputs, logged by running each `melee-sim` scenario gate; children and particle
outputs are computed. Capture provenance and reproduction are documented in
`hsd-particle/tests/data/README.md`. The tests print their field counts under
`--nocapture`. The shield-scene snapshot correction is documented in
`M4_SHIELD.md`; it does not alter these three results.

Validation for this follow-up (local assets present, no skipped asset tests):

- Six new integration targets with `--nocapture`: all pass and print the
  counts above; `/tmp/melee-six-final.log`.
- `cargo test -p hsd-particle`: 58 passed, zero failed.
- `cargo test -p melee-sim --test m4_gate`: 19 passed, covering the 11 movement
  scenes plus ordered particle RNG checks. Idle and match-start also pass in
  `cargo gate`, completing all 13 scenes.
- `cargo gate`: 588 passed, zero failed, one pre-existing ignored doctest
  across 108 suite results; `/tmp/melee-final-gate.log`.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo fmt --all` and `git -c core.fsmonitor=false diff --check`: passed.

Temporary capture instrumentation was removed. No protected harness paths or
submodule files were changed, no Dolphin was run, and no commits were made.
