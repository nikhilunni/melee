# M5 A2: jab, launch and shield hit; grab unfinished

Three scenes pass **300 ticks, 49 keys, 0 divergences**:
`jab_fd_fox`, `utilt_fd_marth`, and `shieldhit_fd_marth`. Each also has a
full-field particle replay through `dust_replay.rs` and ordered particle RNG
assertions in `m5_gate.rs`. The original `jab_fd_marth` gate remains intact.

**The lane is incomplete.** Catch startup now matches ticks 0 through 126
(127 records, all 49 keys), the raw throw-hitbox fields and the ordered particle
ledger. The full grab gate stops during tick 127 at the active-capsule pair
query. Linked capture/throw and missed-tech states remain unported. Following
the continuation request, the full gameplay and particle acceptance tests carry
explicit `#[ignore = "..."]` reasons; separate startup tests stay enabled.
No expected values were changed. The workspace gate is green with this visible
gap; the full 300-tick grab acceptance is **not** satisfied.

## Port table

| Retail function / data | Address | Implementation |
|---|---|---|
| AttackHi3 entry / Anim / IASA | 8008BA38 / 8008BA98 / 8008BAD4 | `attack.rs`: shared attack priority, up-tilt entry, Wait interrupts after script unlock |
| AttackHi3 Phys / Coll | 8008BB04 / 8008BB24 | `state.rs`, `procs.rs`: existing grounded friction and edge-clamped collision |
| Damage angle / velocity | 8008D7F0 / 8008DCE0 | `damage.rs`: ordinary degrees, grounded 361-degree interpolation, air launch, strength/height reaction |
| Damage Phys / Coll / Anim | 8008FB04 / 8008FB64 / 8008F7F0 | `damage.rs`: airborne gravity, knockback decay, hitstun recovery, low-speed landing versus Landing entry |
| First-hit stale scaling of later hitbox commands | 80089118 / 80089228 / 8007891C | `assets.rs`, `commands.rs`, `damage.rs`: first stale weight from PlCo; separate subtract then multiply, retained for this attack instance |
| Air knockback decay | Fighter_procUpdate 8006B82C | shared decay in Damage and existing airborne states, including the residual velocity after Landing slides off FD |
| Shield contact | 80076CBC / lbColl_80007BCC 80007BCC | `damage.rs`, `shield.rs`: scaled shield capsule, group hit suppression, health damage and impact |
| GuardSetOff entry / attacker pushback | 80092F2C / Fighter_ProcessHit 8006D1EC | `shield.rs`, `damage.rs`: stun animation rate, defender ground velocity, attacker shield knockback and hitlag |
| Small normal hit spark | 80078538 / efAsync_Dispatch 80063930 | `effects.rs`: damage scale, effect 9/10 random choice, particles 2/306/307; slash path remains data-selected |
| Attached particle transform | efLib_SpawnParticleEffect 8005D174 / hsd_8039D3AC 8039D3AC | root scale inheritance, attached owner lifetime while child aliases survive; descriptor camera-facing flag survives caller overrides |
| Catch entry / Anim / Phys / Coll | 800D8C54 / 800D8CC8 / 800D8D88 / 800D8E08 | `grab.rs`, callback tables and submotion 242; startup only, pair query remains explicit |
| Throw-hitbox script records | 80071E04 / 80071F0C | `hitbox.rs`, `assets.rs`, `commands.rs`: owned damage records, seek skip |
| FD collision binding | grLast_8021AAB0 / grLast_804D4968 | `melee-sim/frame.rs`, `initial_state/stage.rs`: load map 3's root and apply the stage's static `{0,3,0}` binding through existing collision-update code |

The hitbox data and damage reaction heights come from each character's archive.
Fox Attack11 and Marth AttackHi3 use the shared command interpreter and capsule
ordering, including the up-tilt sourspot/tipper descriptors. No fighter-kind
branch was introduced. Existing `CharacterCallbacks::jab_variant` and
`guard_variant` hooks remain the character boundaries. `melee-if` already
implements the four-digit shake; the 9-percent increase needs no HUD code change.

The FD binding matters even though the floor does not move. Updating its
collision epoch selects the remapped floor sweep. Without that update, Fox
incorrectly lands again at tick 147 beyond the right endpoint. With it, the
existing Fall and ledge code reproduces the capture. This is composition wiring;
no `melee-mp` geometry or other lane's stage crate was edited.

## Capture observations and corrections

| Scene | Observed path |
|---|---|
| `jab_fd_fox` | Attack11 at 106; defender DamageN1 (78), 4 percent at 108; a small airborne hop, grounded again at 115, Wait at 122; attacker Wait at 126 |
| `utilt_fd_marth` | AttackHi3 at 121; Fox DamageHi3 (77), 9 percent at 126; Landing 144, Fall 146, CliffCatch 155, CliffWait 162; attacker Wait 165 |
| `shieldhit_fd_marth` | Fox GuardReflect (182) at 101, Guard 109; ordinary shield contact and GuardSetOff (181) at 124, Guard 131; GuardOff 201, Wait 216 |
| `grab_fd_marth` (retail only) | Catch (212) 121, CatchPull (213) 127, CatchWait (216) 129; ThrowB (220) 141; Fox CapturePulledLw (226), CaptureWaitLw (227), ThrownB (240), DamageFlyN (88), DownBoundU (191), DownWaitU (192) |

The task's attacker-swap prose says Fox hits Marth, but both fighter kinds in
`jab_fd_fox` are Fox (1), agreeing with its table and scenario. It does not
exercise Marth as a defender or Attack12. State 77 is DamageHi3, not a tumble
state. The shield capture uses GuardReflect rather than GuardOn (178), and the
jab lands after its powershield window. State 213 is CatchPull rather than
CatchWait; states 226/227 are the Lw capture states, not Ms.

## Grab stopping point

`ftCo_800D8C54` now enters Catch through `CharacterCallbacks::catch_variant`,
clears animation velocity and installs submotion 242 without an immediate
animation step. The Catch Anim/Phys/Coll callbacks use archive friction and
existing grounded collision. `ftAction_80071E04` decodes the three-word
throw/pummel descriptors into owned `ThrowHitbox` records; script seeking skips
them, as `ftAction_80071F0C` does. No fighter-kind branch was added.

The exact verified prefix is ticks **0 through 126**: Catch begins at tick 121,
animation frame zero, and reaches frame five at tick 126. Both the canonical
49-key trace and ordered particle ledger agree. The raw scratch test also
compares throw damage, angle, growth, fixed/base knockback, element and sound
fields against retail, in addition to existing hitbox/velocity/rate checks.

Processing tick **127**, s_link 12, now stops explicitly with:

```
not implemented: ftColl_80078A2C: active catch capsule needs pair query and linked CatchPull/CapturePulled
```

The active capsule cannot use the ordinary damage path. Still needed are
pair-grab selection and attacker/victim links, CatchPull's retained animation
frame (`fn_800D9CE8`), CapturePulled's hold-bone/XRotN alignment (`fn_800DA8E4`,
`fn_800DAC78`), CatchWait and CaptureWait, then ThrowB's constrained victim
animation (`ftCo_800DD398`, `ftCo_800DE3FC`). Retail changes to 213/226 at 127,
216/227 at 129, 220/240 at 141 (animation rate `1.3333333730697632`), releases
Fox into DamageFlyN (88) at 146 with 4 percent, and enters DownBoundU (191) at
165 and DownWaitU (192) at 191. Marth returns to Wait at 174. None of those
linked/post-release states is claimed as ported.

`m5_gate.rs` retains the full 300-tick gate, ignored with this reason, and adds
a separate 127-tick startup gate. `live_grab_fd_marth.rs` likewise retains an
explicitly ignored full replay and verifies the startup via `dust_replay.rs`.
The helper still checks that all three oracle files contain 300 records; its
existing full replays still compare every tick. The startup fixture records
only production external particle calls before the boundary, not later retail
outputs. Completing the full replay requires capturing the remaining
production calls after the linked gameplay port is finished.

Other explicit boundaries remain: subsequent/staled hits, simultaneous impact
selection, DI/SDI/ASDI, initially airborne victims, sloped-floor launches,
tumble landings, powershields and shield break, high-strength/additional normal
sparks, jab follow-ups, and unported attack families. `fsmash_fd_marth` was
left for death/respawn as requested.

## Particle evidence

| Scene | Fields compared | Ordered particle draws | Final seed | External spawns / fixture events |
|---|---:|---:|---|---:|
| `jab_fd_fox` | 474,513 | 9,642 | `0x384c3fb0` | 8 / 13 |
| `utilt_fd_marth` | 510,260 | 9,900 | `0xb17d642c` | 12 / 16 |
| `shieldhit_fd_marth` | 468,078 | 9,441 | `0xe33e4a8a` | 10 / 22 |

The grab **startup only** compares 127 frames, 215,230 fields and 4,188
ordered particle draws, final seed `0x71afbd66`, with zero mismatches and 1,960
existing display-cache exclusions. Its fixture has 9 external spawns / 10
events. This is not a full grab particle replay.

All three completed scenes compare 300 frames with zero mismatches. The pre-existing AppSRT
display-cache exclusions are unchanged (1,848 / 1,792 / 2,016 fields respectively).
The new external RNG site is `80063990`, the small normal spark's `HSD_Randi(8)`.
The jab and up tilt each produce 72 HUD draws; shield contact produces none.

The fixtures contain only production external SpawnRequest/joint/flag calls,
with floats encoded as bits. No child-generator requests, particle outputs,
future fighter states or expected RNG seeds are simulation inputs. See
`hsd-particle/tests/data/README.md` for regeneration. The replay adapter now
recognizes explicitly attached AppSRT owners as well as descriptor-created
owners; it still compares the generator reference and camera-facing byte.
The same ownership fix is applied to the production particle snapshot adapter.

`attached_transform_owner_outlives_its_direct_particles` uses a synthetic bank
to verify continued attachment updates through a child generator and eventual
cleanup. The raw fighter oracle now checks hitbox endpoints, descriptor fields,
hitlag and hitstun in all three completed scenes as well as A1. It also checks
shield health/lightshield amount, ground velocity, attacker shield knockback
and animation rate, which are not all present in the 49-key gate.

This exposed later up-tilt hitboxes being created with fresh damage after the
first contact. Retail registers the move in the stale table immediately;
`ft_80089118` subtracts the first archive weight and `ft_80089228` scales
subsequent hitbox commands (10 becomes `9.09999942779541`). The port now
retains that first-contact penalty for the current attack instance. It rejects
a later attack instance after a recorded hit until general stale history is
implemented. The raw test expectations were not changed.

## Assembly audit and validation

Retail assembly was inspected with `python3 harness/asm.py <symbol> --fused`
and full instruction listings for the functions above. Key sites:

- Angle interpolation: `8008D8AC` fmadds, then separately rounded degree conversion.
- Launch velocity: separate products; knockback length at `8006B938` fmadds,
  decay at `8006B9C8` / `8006B9E4` fnmsubs.
- Damage landing length: separate products and sum at `8008FBD4..E0`.
- Shield stun: `80093038` / `8009305C` fmadds; shield health at
  `8006D2AC` / `8006D2CC`; attacker pushback at `8006D8D8`.
- Normal spark scale: `80063A10` fmadds. Root scale and collision binding use
  the existing audited scene-graph and collision-transform operations.
- First-hit stale scaling: `ft_80089118 --fused` has no fused sites; the
  separately rounded product is `8008927C` fmuls.
- Catch entry/physics/collision and throw descriptor loading have no fused
  sites (`ftCo_800D8C54`, `ftCo_Catch_Phys`, `ftCo_Catch_Coll`,
  `ftAction_80071E04`, `ftColl_8007ABD0`).
- Generator retention and owner-reference bookkeeping are integer operations.

Final continuation validation:

- `cargo run -q -p melee-sim -- gate harness/scenarios/grab_fd_marth.toml`:
  **incomplete**, exits 101 at tick 127's `ftColl_80078A2C` boundary. It does
  not report 300 ticks. A separate emitted run contains 127 exact records,
  frames 0–126; the enabled startup test verifies that prefix and the ledger.
- `cargo test -p melee-sim --test m4_gate --test m5_gate`: M4 **109 passed**;
  M5 **5 passed, 1 ignored** (full grab, explicit reason). A1 and the three
  completed A2 scenes still pass their full 300-tick gates.
- `cargo test -p hsd-particle`: **69 passed, 1 ignored** (full grab replay,
  explicit reason); existing replays and the new startup replay pass.
- `cargo test -p melee-ft`: **89 passed**.
- `cargo test -p melee-sim catch_startup -- --nocapture`: both the raw scratch
  startup test and canonical/ledger startup test pass.
- `cargo gate`: **722 passed, 0 failed, 3 ignored**. The ignores are the two
  full grab acceptance tests and the pre-existing schema doctest. No failing
  test remains enabled; no other acceptance case was disabled or weakened.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo fmt --all` and `git diff --check`: clean.

No Dolphin run,
commits, protected-path edits, game data, dependency additions or changes to
expected traces. The pre-existing decomp/data symlinks remain unchanged.

## Changed files

Paths are relative to the repository root.

- `TRACKER.md`
- `crates/hsd-particle/src/system.rs`
- `crates/hsd-particle/tests/data/README.md`
- `crates/hsd-particle/tests/data/grab_fd_marth_startup_spawns.json`
- `crates/hsd-particle/tests/live_grab_fd_marth.rs`
- `crates/hsd-particle/tests/data/jab_fd_fox_spawns.json`
- `crates/hsd-particle/tests/data/shieldhit_fd_marth_spawns.json`
- `crates/hsd-particle/tests/data/utilt_fd_marth_spawns.json`
- `crates/hsd-particle/tests/live_jab_fd_fox.rs`
- `crates/hsd-particle/tests/live_shieldhit_fd_marth.rs`
- `crates/hsd-particle/tests/live_utilt_fd_marth.rs`
- `crates/hsd-particle/tests/start_paths.rs`
- `crates/hsd-particle/tests/support/dust_replay.rs`
- `crates/hsd-particle/tests/support/fixture_spawns.rs`
- `crates/hsd-particle/tests/support/restore.rs`
- `crates/melee-ft/src/fighter/M5_COMBAT2.md`
- `crates/melee-ft/src/fighter/README.md`
- `crates/melee-ft/src/fighter/assets.rs`
- `crates/melee-ft/src/fighter/attack.rs`
- `crates/melee-ft/src/fighter/commands.rs`
- `crates/melee-ft/src/fighter/grab.rs`
- `crates/melee-ft/src/fighter/hitbox.rs`
- `crates/melee-ft/src/fighter/damage.rs`
- `crates/melee-ft/src/fighter/effects.rs`
- `crates/melee-ft/src/fighter/landing.rs`
- `crates/melee-ft/src/fighter/mod.rs`
- `crates/melee-ft/src/fighter/procs.rs`
- `crates/melee-ft/src/fighter/shield.rs`
- `crates/melee-ft/src/fighter/spawn.rs`
- `crates/melee-ft/src/fighter/state.rs`
- `crates/melee-ft/src/fighter/walk.rs`
- `crates/melee-sim/src/effects.rs`
- `crates/melee-sim/src/frame.rs`
- `crates/melee-sim/src/frame/combat.rs`
- `crates/melee-sim/src/initial_state/particles.rs`
- `crates/melee-sim/src/initial_state/stage.rs`
- `crates/melee-sim/tests/m5_gate.rs`
