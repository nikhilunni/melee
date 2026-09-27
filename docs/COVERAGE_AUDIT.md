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

## Sudden Death items: highest priority

Sudden Death makes common items reachable. Ported and gated
(`sudden_death_pickup_bomb_fd_marth`): the pickup search
(ftpickupitem_800942A0) before jab, jab follow-ups, rapid jab, side and down
tilt; LightGet; the held Bob-omb, the hand pose and item idle (Wait1_1); the
lit fuse in hand and the blast releasing it from the hand (it_8027429C), which
hits its holder (xDCD b5). Fighter hitboxes land on item hurt capsules
(it_802703E8), with the item's hit spark, knockback and hitlag
(it_80270E30, OnTakeDamageThink), so a smash detonates a falling Bob-omb
(`sudden_death_smash_bomb_fd_marth`). A neutral A throws a held item
forward (LightThrowF, ftCo_80095EFC, Item_8026AD20; the Bob-omb's thrown
state 10) (`sudden_death_throw_bomb_fd_marth`).

While a fighter holds an item only LightGet and Wait are audited
(`item_pickup::HELD_ITEM_STATES`); entering any other state fails closed.
The remaining reachable gaps:

| Retail path | Port | Reachable by |
| --- | --- | --- |
| Held-item states beyond LightGet, Wait and LightThrowF: directed and smash throws, air and dash throws, drops (LR+A, Z, shield, `fighter.c:2678` random drop on a hit), walking, jumping, damage and death while holding | Fail closed at the motion change or input | Any other input or a hit while holding a Bob-omb |
| A thrown Bob-omb's soft landing (state 2) | Fails closed | A short toss onto the stage |
| `fn_800D6F58` aerial catch (ftCo_800D7100) | Fails closed when LR + A finds a light item in reach | Shield + A in the air beside a Bob-omb |
| HeavyGet | Fails closed | No heavy item appears in scope |
| Item hitboxes versus item hurtboxes (it_802706D0) | Detection ported; a landing contact fails closed | A thrown or dropped Bob-omb, or a kindred-striking hitbox, reaching another |
| Unlit Bob-omb pickup, walking and turning | Fail closed | Only after a soft landing, which the rain's speed prevents |

Next packet: the remaining throws and drops, a soft landing, and damage or
death while holding, each from a recorded witness.

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

Not complete: the Sudden Death item interactions above are reachable and
unported (they fail closed), as are the listed reachable boundaries. The timer, timeout and
Sudden Death flow, the screen KO and the idle Bob-omb rain are complete and
gated.
