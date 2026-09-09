# M5 A1: Marth's first jab into idle Fox

The production `jab_fd_marth` gate matches **300 ticks × 49 keys, zero
divergences**, including both fighters' percent, velocities, motion states
and the shared RNG seed. The simulation uses the saved initial state, archives
and pad samples; later fighter/particle states and RNG ledger results are
assertions, never gameplay inputs.

## Port and scheduler

| Retail source | Address | Rust owner / behavior |
|---|---|---|
| `ftCommon_8007DD7C`, `ftCommon_8007E0E4` | 8007DD7C, 8007E0E4 | `overlap.rs`: ordered grounded separation after each Anim proc; same/neighbor floor eligibility, horizontal push, depth spread/recentering and hitlag suppression |
| `decideAttack11`, `checkAttack11` | 8008AB84, 8008ABC0 | `attack.rs`: shared jab entry; character exceptions use `CharacterCallbacks::jab_variant` |
| Attack11 Anim / IASA / Phys / Coll | 8008AC9C, 8008ACD8, 8008ADF0, 8008AE10 | enum callbacks, typed combo window/latch, grounded friction/root-motion physics and existing collision callback |
| `ftAction_8007121C`, clear commands | 8007121C, 80071784, 800717D8 | `assets.rs`, `commands.rs`, `hitbox.rs`: descriptor decoding, four owned capsules, timing, clear-one/all, group victim inheritance |
| `ftColl_8007AD18` | 8007AD18 | `hitbox.rs`: initial point and subsequent swept endpoints from the animated bone |
| `Fighter_8006CB94`, `ftColl_80078C70` | 8006CB94, 80078C70 | scene visits receivers then other fighters in entity-list order; `damage.rs` scans hitbox IDs then ftData hurtboxes |
| `lbColl_80006E58`, `lbColl_80005EBC` | 80006E58, 80005EBC | `melee-lb/collision.rs`: broadphase, segment projection, inverse bone scaling, contact point and overlap |
| `ftColl_80079AB0`, `ftCo_8008DCE0` | 80079AB0, 8008DCE0 | archive-derived weight/percent/growth/base knockback, reaction thresholds, damage velocity and hitstun |
| `ftCommon_CalcHitlag`, `Fighter_8006A1BC` | 8007DA74, 8006A1BC | attacker/victim countdown; freeze animation/physics, buffer input edges and resume in scheduler order |
| `ftCo_8008F744`, `ftCo_Damage_Anim` | 8008F744, 8008F7F0 | grounded DamageN2 countdown and animation completion, direct return to Wait |
| `ftColl_8007A06C`, `efAsync_Dispatch` | 8007A06C, 80063930 | contact-driven slash effect 1004 → model effect 8 → particle 267, random Z rotation; no fighter ownership |
| `ifStatus_802F5B48`, `ifStatus_802F4EDC`, `ifStatus_802F4B84` | 802F5B48, 802F4EDC, 802F4B84 | new `melee-if`: percent-increase shake, four digit offsets, nine ticks of eight RNG draws; interface s_link 17/p_link 15, after particle s_link 15 |
| `hsd_8039930C`, opcode B3 | 8039930C | `hsd-particle/particle.rs`: signed 16.16 alpha-comparison rebasing, timer and target bytes |

Hurtbox bones, offsets, radii and reaction heights come from each fighter's
ftData table. Shared combat and overlap constants come from PlCo. Hit elements
are a shared enum in `melee-types`; character-specific behavior stays behind
hooks. Sword-trail and action-sound subcommands retain renderer/audio requests.
No rendering or audio engine was added.

`Interaction::{Attack,Damage,Hitlag}` are supported with typed backing state.
Bare flags without their state still fail the existing invariant test.
The independent scheduler expectation now includes the two interface procs.

## Observed first hit

Overlap first changes the baseline at tick 97. The port reproduces the push
through RunBrake/Wait, then Attack11 at tick 121. At tick 124 Marth's first
three 4-damage capsules share a victim group; Fox is hit only once. The fourth
capsule has 6 damage and does not replace the first contact.

Fox receives 4 percent, knockback `29.959999084472656`, horizontal velocity
`0.8987998962402344`, eleven hitstun frames and four hitlag frames. Both
animation clocks freeze through tick 127, resume at 128, and Fox's hitstun
reaches zero at 138. Fox returns to Wait at 150; Marth returns at 151.
The independent particle replay also verifies the contact-derived spark's
position and animation inputs.

Two qualifications to the task notes:

- Fox enters **DamageN2 (79)** and returns directly to Wait. Neither DamageFall,
  AttackS3 nor dash attack is taken; those branches remain explicit unsupported
  paths. The movement/action observations in the task otherwise agree.
- The push-away implementation is in **ftcommon.c**, called by fighter.c's
  animation proc. The ledger also contains **one spark-rotation draw at
  0x80063B70**, beyond the additional sites listed in the task.

## RNG and particle replay

Every ledger draw is classified. The production M5 test compares all 9,373
particle callsites in order, and the 49-key gate compares every final seed.
HUD tests independently verify 72 draws, no draws on the reset tick or a
percent decrease, and restarting the shake on a later increase.

| Callsite(s), instruction address (LR minus 4) | Draws |
|---|---:|
| 8039EF00 | 443 |
| 8039EB74 / 8039EBCC | 1463 / 1465 |
| 8039B088 / 8039B0F4 / 8039B160 / 8039B1CC | 1463 each |
| 8039E1E4 / 8039E3D4 | 38 each |
| 8039F250 | 1 |
| 8039B5E0 / 8039A810 | 22 / 18 |
| 8039C4F8 / 8039C58C | 4 each |
| 8039C870 / 8039E088 | 12 / 6 |
| 8039EB04 / 8039EB5C / 8039EBF0 | 2 each |
| 8039C8D4 | 1 |
| 8009FCDC / 8009FD00 / 8009FD24 (fighter GFX) | 8 each |
| 8008A8BC (Wait choice) | 4 |
| 80063B70 (spark rotation) | 1 |
| 802F4D44 / 802F4D54 (HUD shake) | 36 each |
| 8021AEC8 (FD) | 1 |

`live_fd_jab_marth` runs through `dust_replay.rs`: **467,132 simulation fields,
300 ticks, 9,373 ordered particle draws, zero mismatches**, final seed
`0x3066C858`. The existing 1,792 render-only AppSRT cache exclusions are
unchanged. The new fixture contains nine external spawns and 41 total input
events logged from production; it contains no particle outputs. See
`hsd-particle/tests/data/README.md` for reproduction.

The helper keeps pre-particle external draws and post-particle HUD draws in
separate segments. Every HUD suffix must consist of alternating X/Y callsites
in complete groups of eight; any other external interleaving still fails.
Old scene names now pass their full trace stem to the same helper. B3 moved
from unsupported-opcode classification to supported behavior and explicit
truncated-operand assertions; no oracle expectations were changed.

## Fusion audit

Ran `python3 harness/asm.py <symbol> --fused` against the existing retail DOL
for every table entry with arithmetic. Source comments cite the corresponding
instructions. In particular:

- Overlap centers fuse at 8007DE5C/8007DE64; depth integration is unfused.
- Hitbox command offsets use the literal **0.003906f**, not 1/256.
- Capsule geometry preserves the unfused hit-length sum, fused other dot
  products, 80007174/80007420/80007424 fmsubs, double midpoint fmadd/frsp,
  audited MSL sqrt and paired-single matrix operations.
- Knockback uses 80079C34/80079C40/80079C44; the fixed-weight formula's
  corresponding sites are 80079B48/80079B50/80079B54. Hitlag uses 8007DAA8.
- Grounded jab root-motion subtraction uses 8008505C fmsubs. Damage velocity
  products stay separate. Spark rotation multiplies in double then rounds.
- HUD shake has no fused instructions. B3 uses integer arithmetic only.

## Boundaries and validation

This is the first unstaled jab hit on a neutral grounded victim. Subsequent
hit history/staling, other damage reactions, airborne knockback/DamageFall,
DI/SDI/ASDI, shield hits, clanks, phantom/invincible contacts, simultaneous
hit selection, jab follow-ups, tilts/smashes/dash attacks, items, grabs and
special character jab variants remain explicit boundaries. General geometry
has analytical sphere/parallel/crossing and broadphase-rejection tests; the
retail oracle certifies the path exercised by this scene, not all combat.

- `cargo run -q -p melee-sim -- gate harness/scenarios/jab_fd_marth.toml`: 300 ticks, 49 keys, zero divergences.
- `cargo test -p melee-sim --test m4_gate`: 65 passed.
- `cargo test -p melee-ft`: 89 passed.
- `cargo test -p hsd-particle`: 60 passed.
- `cargo gate`: **652 passed, zero failures, one pre-existing ignored doctest**;
  includes `m5_gate`, the raw combat oracle, the new particle replay and all
  existing particle tests.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean. The two
  localized geometry lint allowances preserve the audited comparison form;
  replacing the range rejection with `!contains` would change NaN behavior.
- `cargo fmt --all` and `git diff --check`: clean.

No Dolphin run, protected-path edits, commits, or game-data additions. The
lane's pre-existing data/decomp symlinks remain unchanged.

## Changed files

Paths below are relative to the repository root. The full list includes the
shared replay helper's existing callers, whose only change is a full scene stem.


- `Cargo.lock`
- `Cargo.toml`
- `TRACKER.md`
- `crates/hsd-particle/src/particle.rs`
- `crates/hsd-particle/tests/data/README.md`
- `crates/hsd-particle/tests/data/jab_fd_marth_spawns.json`
- `crates/hsd-particle/tests/live_fd_airdodge.rs`
- `crates/hsd-particle/tests/live_fd_dash.rs`
- `crates/hsd-particle/tests/live_fd_jab_marth.rs`
- `crates/hsd-particle/tests/live_fd_jump.rs`
- `crates/hsd-particle/tests/live_fd_ledge.rs`
- `crates/hsd-particle/tests/live_fd_roll.rs`
- `crates/hsd-particle/tests/live_fd_shield.rs`
- `crates/hsd-particle/tests/live_fd_spotdodge.rs`
- `crates/hsd-particle/tests/live_fd_wavedash.rs`
- `crates/hsd-particle/tests/opcodes.rs`
- `crates/hsd-particle/tests/support/dust_replay.rs`
- `crates/hsd-particle/tests/support/fixture_spawns.rs`
- `crates/melee-ft/src/fighter/M5_HIT.md`
- `crates/melee-ft/src/fighter/README.md`
- `crates/melee-ft/src/fighter/air_dodge.rs`
- `crates/melee-ft/src/fighter/assets.rs`
- `crates/melee-ft/src/fighter/attack.rs`
- `crates/melee-ft/src/fighter/caches.rs`
- `crates/melee-ft/src/fighter/commands.rs`
- `crates/melee-ft/src/fighter/damage.rs`
- `crates/melee-ft/src/fighter/effects.rs`
- `crates/melee-ft/src/fighter/hitbox.rs`
- `crates/melee-ft/src/fighter/mod.rs`
- `crates/melee-ft/src/fighter/overlap.rs`
- `crates/melee-ft/src/fighter/procs.rs`
- `crates/melee-ft/src/fighter/spawn.rs`
- `crates/melee-ft/src/fighter/state.rs`
- `crates/melee-ft/src/fighter/walk.rs`
- `crates/melee-if/Cargo.toml`
- `crates/melee-if/src/lib.rs`
- `crates/melee-lb/src/collision.rs`
- `crates/melee-lb/src/lib.rs`
- `crates/melee-sim/Cargo.toml`
- `crates/melee-sim/src/effects.rs`
- `crates/melee-sim/src/frame.rs`
- `crates/melee-sim/src/frame/combat.rs`
- `crates/melee-sim/tests/m5_gate.rs`
- `crates/melee-types/src/hit.rs`
- `crates/melee-types/src/lib.rs`
