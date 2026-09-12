# Post-hitstun aerial input

The post-hitstun handler selected aerial attacks but had no transition arm,
so fresh A or C-stick could panic as soon as hitstun expired. Ordinary aerial
damage also shared its input ordering with tumble, incorrectly allowing air
dodge in tumble. A directed ordinary-damage air dodge exposed a separate
missing knockback decay in its physics callback.

Retail references: `ftCo_Damage_IASA` (8008FA44), `ftCo_DamageFly_IASA`
(8008FF48), `ftCo_DamageFlyRoll_IASA` (80090324), `ftCo_DamageFall_IASA`
(80090828), `ftCo_Fall_IASA_Inner` (800CCAAC), and `Fighter_procUpdate`
(8006B82C). The ordinary handler delegates to Fall's input ordering; both
DamageFly handlers delegate to DamageFall. In the item-free matchup,
ordinary damage permits special, air dodge, attack, then jump; tumble omits
air dodge. Attack entry belongs to the existing character table hook.
Knockback decay follows the state-specific physics callback and precedes
position integration; the existing audited helper implements that arithmetic.

Seven 300-tick scenarios were recorded before changing behavior:

- `hitstun_exit_{nair,fair}_fd_marth`: DamageAir3 (86) becomes AttackAirN (65)
  or AttackAirF (66) at tick 87. The neutral-air input includes X, proving
  attack priority over jump. Directional landing states follow at tick 95.
- `hitstun_unbuffered_attack_fd_marth`: A pressed during hitstun and held
  through expiry does not start an attack; a fresh press starts neutral air
  at tick 90.
- `hitstun_shield_priority_fd_marth`: simultaneous L and C-stick in ordinary
  aerial damage starts EscapeAir (236) at tick 87. Its remaining knockback
  must decay each tick before integration.
- `hitstun_exit_{nair,fair}_fd_fox` and `hitstun_shield_priority_fd_fox`:
  DamageFlyTop (90) exits on tick 184 and lands that same tick (Landing 42).
  These exercise aerial entry followed by immediate collision, not a
  persistent airborne attack pose. DamageFall and DamageFlyRoll use the same
  retail input delegation but have no separate directed witness in this packet. Tumble must not select air dodge even
  when L accompanies the C-stick input.

Before the fix, five scenarios panic on airborne Attack. Fox's competing
shield input diverges at tick 184 in animation frame; Marth's ordinary-damage
shield input diverges at tick 87 in X position. The baseline CLI evidence is
`/tmp/melee-hitstun-before.log`.

Acceptance: all seven fighter/ordered-particle gates, zero simulated-tick
allocations, workspace debug/release gates, all-target clippy and harness
pytest. Full debug/release gates each passed 1,213 tests, with 0 failures and
3 existing ignores; clippy, 220 harness tests, native build/smoke and a
233-tick exported headless replay pass. Oracle
files remain ignored and are backed up outside the repository.

Changed files: `melee-ft/src/fighter/{damage.rs,fall.rs,state/callbacks/input.rs,
state/callbacks/physics.rs}`, `melee-sim/tests/{m5_gate.rs,alloc_gate.rs}`,
the seven scenario TOMLs, TRACKER, the matchup inventory and this note.
