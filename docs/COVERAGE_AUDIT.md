# Fox–Marth on Final Destination: coverage audit (2026-09-26)

Scope as in `MATCHUP_COMPLETENESS.md`: two humans, Fox and Marth, Final
Destination, stock rules with an optional timer, random items off, character
articles and Sudden Death Bob-ombs in scope. This audit classifies every
explicit port boundary (`unimplemented!`, `todo!`, fail-closed `bail!`) in
the crates that run this matchup, and the silent gaps found by reading the
retail callers of paths the port skips. A boundary is **reachable** when
legal inputs in scope can reach it, **out of scope** when only another
character, stage, mode or CPU can, and **investigate** when neither is shown.

Evidence of exactness for what is implemented: 48 generated full matches
(corpus v2), the nine corpus v3 matches that faulted the port (240 explored)
and the directed scenarios in `m5_gate`, the timeout and Sudden
Death path (`tests/match_endings.rs`), cold starts, bones, particles and
allocation gates.

## Silent gaps (no boundary today): highest priority

Sudden Death makes common items reachable, and the port's item support was
built for character articles, none of which can be picked up or hit.

| Retail path | Port | Reachable by |
| --- | --- | --- |
| `ftpickupitem_80094790` before jab, jab follow-ups, rapid jab, side and down tilt (ftCo_Attack1.c, ftCo_AttackS3.c, ftCo_AttackLw3.c, ftCo_Attack100.c) | Skipped: the attack starts | A near a lit Bob-omb (grabbable while `xDC8 x15` is set) |
| `ftCo_800D7100` / `ftCo_800D71D8` aerial catch | Asserts the scene proved no grabbable item; fails whenever a Bob-omb exists | Shield + A in the air during Sudden Death |
| Fighter hitboxes versus item hurtboxes (Bob-omb `DmgReceived` detonates) | Not modelled; items have no hurtboxes | Attacking a Bob-omb |
| Item hitboxes versus item hurtboxes (one explosion detonating another) | Not modelled | Two Bob-ombs within an explosion |
| Holding, throwing and dropping items (LightGet, ItemThrow*, Bob-omb states 7–10) | Absent | Any pickup above |

Next packet: port the pickup search geometry, item hurtboxes and both item
hurt tests, failing closed at the first unsupported response, then record
directed Sudden Death witnesses (jab a falling bomb, stand under two bombs,
pick one up and throw it) and implement the responses they show.

## Explicit boundaries

### Reachable

| Site | Boundary | Note |
| --- | --- | --- |
| `melee-ft/fighter/damage.rs` ftCo_8008EC90 | Third-party hit on a captured fighter | A Bob-omb explosion during a grab |
| `melee-ft/fighter/grab_escape.rs` ftCo_8008EC90 | Captured damage outside low capture or throw | Same interaction family |
| `melee-ft/fighter/damage.rs` fighter.c:2907 | Phantom contact and shield impact together | Corpus fault family; needs a witness |
| `melee-ft/fighter/damage.rs` ftColl_80076CBC | Simultaneous shield impacts | Two hitboxes on one shield in a frame |
| `melee-ft/fighter/clank.rs` ftColl_8007925C | An inert hitbox touching an item | Investigate whether Fox/Marth own inert hitboxes |
| `melee-ft/fighter/down.rs` | DownReflect wall bounce, DownDamage wall tech/bounce | Needs FD's walls under the stage; investigate |
| `melee-ft/fighter/state/callbacks/collision.rs` ftCo_StopWall | Running into a wall | FD's side walls are below the ledge; investigate |
| `melee-it/map.rs` it_80276FC4 | Item wall/ceiling bounce | A Bob-omb dropped over the ledge edge |
| `it-bombhei` unported rows | Walking, turning, held and thrown states | Follows from pickup; a soft landing never happens in the rain (terminal speed exceeds the explosion thresholds) |
| `ft-fox-family/special_hi.rs` | Fire Fox platform skip | FD has no platforms: out of scope |

### Out of scope (other characters, stages, CPU or modes)

Character hooks without a Fox/Marth override (`fighter/mod.rs`: forward-smash
entry, throw callback, jab entry, tether, landing reset, morph-ball roll,
multi-jump table; `character.rs` taunt entry; `attack.rs` Link's second
forward smash; `jump.rs` non-basic double jumps; `damage.rs` double-jump
armor, DamageIce, model-scaled victims; `spawn.rs` scaled attributes;
`shield.rs` cape shield), CPU input (`input/human.rs`), platform drops
(`shield.rs` Pass, Reflector check-pass, grounded special fall), coin mode
(`fighter/mod.rs` attachment collision), other stages' backgrounds and lights
(`melee-gr`), the zoomed single-player camera (`melee-cm`), and item kinds
other than the Fox articles and the Bob-omb (`melee-it`).

### Investigate

| Site | Question |
| --- | --- |
| `jump.rs` ftCo_Jump.c:69 | Which inputs reach KneeBend without tap or X/Y (C-stick up with jump-on-C is a controller option retail stores per port) |
| `landing.rs` scratch inheritance | Landing from a state whose second scratch word the port does not track |
| `procs.rs` ft_081B.c terrain footsteps | FD's floor has flags 0; unreachable on FD |
| `procs.rs` ft_0899.c pose paths | Which pose paths the corpus has not exercised |
| `commands.rs` sound behaviors | Script sound behaviors outside the Fox/Marth scripts |
| `color_overlay.rs` | Secondary-slot programs with effects outside the smash charge |
| `special_lw.rs` Reflector turnFrames | The unmodelled scratch word's reachable sources |
| `input.rs` special fall with jumps left | Entry spends the jumps; likely unreachable |
| `life.rs` gm_80167320 final stock | The scene freezes before the timer can expire; unreachable |
| `state/special.rs` buffered special | Entry without a supported special buffer |

Catch boxes against items (`ftColl_8007BC90`) are not a gap: the item side
requires `xDD0 b4`, which only stage enemies set. A grabber dying with its
victim (ftCo_800D331C, ftCo_800DD100) is ported and gated by corpus v3.

## Exit status

Not complete: the silent Sudden Death item interactions above are reachable
and unported, as are the listed reachable boundaries. The timer, timeout and
Sudden Death flow, the screen KO and the idle Bob-omb rain are complete and
gated.
