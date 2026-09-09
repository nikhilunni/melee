# Lane A3: capture, throw, tech and stock loss

2026-09-09. **Acceptance incomplete; stopped under the repository's exact-test
rule.** All three requested canonical scenarios and their independent particle
replays have passed. Expanded raw-fighter coverage exposed an incorrect offset
in the existing comparator; it has not been corrected or bypassed. No commit.

## Blocking evidence

`cargo test -q -p melee-sim frame::combat -- --nocapture` reports six passes and
two failures: the new full `capture_back_throw_and_missed_tech_match_retail_scratch`
and `back_throw_and_tech_match_retail_scratch` cases fail with `throw field 0x34`,
actual 0, expected 3840. The existing comparator in `frame/combat.rs` reads
`sound_severity` at HitCapsule +0x34 and `sound_kind` at +0x38.

Retail `ftAction_80071E04` stores severity at +0x38 (`80071ED0: stw r0,0x38(r31)`)
and kind at +0x3C (`80071EE0: stw r0,0x3c(r31)`). `lb/types.h` agrees: +0x34 is
`x34`, not sound severity. The original 127-tick grab test never reached the
throw records populated at ThrowB. This is a comparator defect, not permission
to change the oracle: the task explicitly requires stopping in this situation.
The failing assertions and full tests remain enabled and unchanged.

Clippy also stopped on two cleanup issues in the new `melee-if` code:
`items_after_test_module` and `unnecessary_unwrap`. These were not fixed after
the exactness-rule stop. A final full workspace gate was not rerun. This tree
must not be described as gate/clippy clean or ready to merge.

## Verified results

| Command / evidence | Result |
|---|---|
| `cargo run -q -p melee-sim -- gate harness/scenarios/grab_fd_marth.toml` | 300 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/tech_fd_marth.toml` | 300 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/ko_fd_marth.toml` | 480 ticks, 49 keys, 0 divergences; repeated after revival platform addition |
| Full grab particle replay | 502342 fields, 9833 ordered draws, zero mismatches; seed `cb806b71` |
| Full tech particle replay | 496564 fields, 9894 ordered draws, zero mismatches; seed `de422e0b` |
| `cargo test -q -p hsd-particle --test live_ko_fd_marth -- --nocapture` | 872466 fields, 16596 ordered boundary/particle draws, zero mismatches; seed `77fd175a` |
| Expanded raw fighter tests | 6 passed, 2 blocked by the comparator offsets above; KO raw scratch passed |
| `cargo fmt --all` | Completed |
| `cargo clippy --workspace --all-targets -- -D warnings` | Failed on the two HUD cleanup issues above |

Full grab gameplay/particle tests are unignored; tech and KO tests were added to
`m5_gate.rs`. Existing expected values and scenario files were not changed.
The original particle display-cache exclusions remain: 2324 grab, 2576 tech,
3248 KO. No new exclusions were added. The KO draw count includes the external
slash-orientation boundary event; this event is checked in order, not discarded.

## Behavior and ownership

- Shared grab pair selection resolves the catch capsule against grabbable
  hurtboxes, selects by distance, and links typed captor/victim spawn identities.
  CatchPull and CatchWait maintain the held geometry and constrain the thrown
  fighter through the shared HSD position-constraint mechanism.
- Back throw borrows Marth's victim animation/script for Fox with common-bone
  remapping, weight-scaled playback, HipN displacement and release geometry.
  Thrown velocity, DamageFlyN, DamageFall and the missed-tech DownBoundD/DownWaitD
  path use enum callbacks and typed scratch.
- Tech eligibility uses the digital shield edge window and prior-press lockout.
  The captured path is PassiveStandB; its script drives intangibility, movement
  and completion at Wait.
- Forward smash uses archive hitbox data and a typed charge phase. The one-frame
  charge release scales damage with the retail fused arithmetic; knockback
  preserves integer conversion of the damage operand while retaining the
  fractional percentage. Fox receives exactly 14.08565616607666 percent.
- Death checks run after Update. DeadDown clears velocity owners, decrements the
  saved player-slot stock count, hides the fighter and creates the KO effect.
  The 60-tick death delay rebuilds the fighter at the player's FD revival marker,
  consumes the two CPU reset draws, resets percent and allocates a spawn number.
- Revival runs its own collision wrappers, idle animation loop, countdown and
  velocity. The collision pass is necessary even above the stage: omitting it
  changes tick 269 Y by one bit. The platform is the model/animation referenced
  by PlCo `ftLoadCommonData[8]`, retained as a typed accessory and updated in the
  accessory proc. Respawn currently sets intangible hurt status.
- `melee-if` owns the departing percent digits and five stock-icon animations.
  Percent death draws eight values once. Losing the icon spawns generator 247
  on HUD link 1 after the particle passes; resetting percent adds no draws.
  Saved HUD positions and animation counters are imported only at the boundary.
- Death particles require tornado emission/physics, rectangular volume emission
  and A9 random-cone velocity rotation. These use the existing HSD particle
  engine, actual programs and shared RNG. No future trace state drives runtime.

## Capture corrections

State 213 is CatchPull; 216 is CatchWait. Fox states 226/227 are
CapturePulledLw/CaptureWaitLw. States 191/192 are DownBoundD/DownWaitD;
PassiveStandB is 201. These supersede the informal names in the task/A2 notes.

The KO trace does **not** show Fox dropping from the platform after tick 329.
It remains RebirthWait at (-50,45) through tick 479, with 90 platform ticks left.
No post-platform landing or timeout completion is claimed.

The stock-2 save has scene-named initial particle metadata. Boundary lookup now
uses the shared save-stem file when present, otherwise the scene-named file;
seed consistency with the owned savestate remains checked.

## Assembly audit

The audit uses `harness/asm.py --fused` and the owning retail assembly. In the
last portion of this run the script was run directly with Python 3: `uv` could
not write its default cache, and a temporary cache attempted unavailable network
downloads. The script itself has no external dependency.

| Path | Audit evidence |
|---|---|
| Grab selection / hold alignment | 80078A2C, 800DAC78: no fused sites |
| Thrown HipN displacement | 8007E4C8/DC/F0: fmadds |
| Thrown accessory / release positions | 800DE558/56C; 800DE084/098: fmadds |
| Local XRotN retention | 800DB500: no fused sites |
| Smash damage scaling | 800DEEDC: fmadds; 800DEF38/800DF0D0 have none |
| Down landing / tech entry | 8009794C, 80097E8C, 800986B0, 80098928, 800989D4: no fused sites |
| Ground knockback projection | 8007CCE8: no fused sites |
| DeadDown / revival physics | 800D3BC8 no sites; 800D53BC/800D5954 fused moving-marker offsets; static interpolation uses separate subtract/divide/multiply |
| HUD departing digits | 802F49A0 fmadds; horizontal product/add remain separate |
| Tornado generator orientation | 8039DFEC fmadds |
| Tornado particle physics | 8039CAFC; 8039CB38/40/54/68 fmadds; 8039CB60 fmsubs |
| Rectangle emission | 8039E978/7C, E994/98, E9B0/B4 fmadds, including zero matrix terms |
| A9 direction rotation | 80399048/B8/BC, 80399178/80/90/94 fmadds; 8039918C fmsubs |
| Camera bounds | Ground_801C39C0 and Stage_GetCamBoundsTopOffset: no fused sites |

`doEnter` is an ambiguous local symbol in asm.py; the AttackS4 owning unit was
inspected directly rather than trusting another unit's same-named function.
Existing audited HSD matrix and inverse-concat kernels implement the capture
constraint; no alternate math implementation was introduced.

## Limits and follow-up

Resolve the comparator blocker only with authorization consistent with the stop
rule, then fix clippy and run the complete gate. Additional branch completeness
review remains necessary: grab timeout/mashing, capture air transitions,
neutral/forward tech, sustained smash charge, other blast directions, final-stock
elimination, shared-stage revival marker allocation and platform exit are not
complete. Existing explicit unsupported branches remain. Cold HUD initialization
is not yet implemented (`None` at cold match start); saved-scene HUD state is.
Respawn intangibility currently uses hurt status; its independent lifecycle flags
and post-platform invincibility deserve further coverage. Knocked-fighter body
hitbox expiry, cold capture geometry and unused reversal inputs also need review.
This is a verified scenario slice, not a complete combat or match lifecycle.

No decomp, capture, ROM or existing scenario file was changed; Dolphin was not
run. The lane's pre-existing symlinks remain unchanged.

## Changed files

- `TRACKER.md`
- `crates/hsd-anim/src/jobj.rs`
- `crates/hsd-particle/src/generator.rs`
- `crates/hsd-particle/src/particle.rs`
- `crates/hsd-particle/src/system.rs`
- `crates/hsd-particle/tests/data/README.md`
- `crates/hsd-particle/tests/data/grab_fd_marth_spawns.json`
- `crates/hsd-particle/tests/data/ko_fd_marth_spawns.json`
- `crates/hsd-particle/tests/data/tech_fd_marth_spawns.json`
- `crates/hsd-particle/tests/live_grab_fd_marth.rs`
- `crates/hsd-particle/tests/live_ko_fd_marth.rs`
- `crates/hsd-particle/tests/live_tech_fd_marth.rs`
- `crates/hsd-particle/tests/support/dash_fd_spawns.rs`
- `crates/hsd-particle/tests/support/dust_replay.rs`
- `crates/hsd-particle/tests/support/fixture_spawns.rs`
- `crates/hsd-particle/tests/support/jump_fd_spawns.rs`
- `crates/hsd-particle/tests/support/restore.rs`
- `crates/hsd-particle/tests/support/start_fd_spawns.rs`
- `crates/melee-ft/src/fighter/M5_COMBAT3.md`
- `crates/melee-ft/src/fighter/README.md`
- `crates/melee-ft/src/fighter/assets.rs`
- `crates/melee-ft/src/fighter/attack.rs`
- `crates/melee-ft/src/fighter/caches.rs`
- `crates/melee-ft/src/fighter/commands.rs`
- `crates/melee-ft/src/fighter/damage.rs`
- `crates/melee-ft/src/fighter/down.rs`
- `crates/melee-ft/src/fighter/effects.rs`
- `crates/melee-ft/src/fighter/escape.rs`
- `crates/melee-ft/src/fighter/grab.rs`
- `crates/melee-ft/src/fighter/grab_throw.rs`
- `crates/melee-ft/src/fighter/hitbox.rs`
- `crates/melee-ft/src/fighter/life.rs`
- `crates/melee-ft/src/fighter/mod.rs`
- `crates/melee-ft/src/fighter/overlap.rs`
- `crates/melee-ft/src/fighter/procs.rs`
- `crates/melee-ft/src/fighter/smash.rs`
- `crates/melee-ft/src/fighter/spawn.rs`
- `crates/melee-ft/src/fighter/state.rs`
- `crates/melee-if/src/lib.rs`
- `crates/melee-sim/src/assets.rs`
- `crates/melee-sim/src/effects.rs`
- `crates/melee-sim/src/frame.rs`
- `crates/melee-sim/src/frame/combat.rs`
- `crates/melee-sim/src/frame/grab_pairs.rs`
- `crates/melee-sim/src/initial_state/cold.rs`
- `crates/melee-sim/src/initial_state/fighter.rs`
- `crates/melee-sim/src/initial_state/mod.rs`
- `crates/melee-sim/src/scenario.rs`
- `crates/melee-sim/tests/m5_gate.rs`
