# Generated matches as retail oracles: phantom hits and twelve fixes

2026-09-26. The version 2 robustness corpus (`melee-replay` example
`explore`) now starts every case from a registered retail boundary
(`harness/boundaries.toml`), and `harness/replay_to_scenario.py` replays any
case in Dolphin with exact per-tick pads (see `docs/DOLPHIN_RUN.md`, "Corpus
bridge"). The seven cases that had faulted on phantom hits were recorded in
retail at full length and compared tick for tick. Each divergence was traced to
retail source and assembly and fixed in shared code. All seven now match retail
from match start to GAME: 25,115 ticks, 62 keys including items, and ordered
particle RNG sites (`m5_gate::corpus_v2_matches_through_game`).

## Damage logs and phantom hits (shared, `melee-coll::damage_log`, `fighter/hit_log.rs`)

Retail detects every contact against one victim before resolving any of them
(Fighter_8006CB94). Contacts go to `dmg_log0` (hits) or `dmg_log1` (phantom
"tip" contacts, `coll_distance < PlCo +7A8`), then:

- ftColl_8007AB48 computes each logged hit's knockback with the damage of
  *every* hit logged this frame (`dmg.x1838_percentTemp`), spawns every
  entry's hit effect in log order, and keeps the strictly strongest entry.
  Percent rises by the frame total; hitlag uses the largest integer damage
  (`x183C_applied`).
- A real hit discards the frame's phantom log (`dmg_log1_idx = 0`); a phantom
  is only logged with no hit yet this frame and no phantom lockout.
- ftColl_8007AB80 keeps the strongest phantom. ProcessHit gives the victim
  hitlag from the halved damage (`x1840`) and records that hitlag as the
  phantom lockout (`x189C`). When the lockout runs out without a hit,
  ftColl_8007BE3C applies the halved damage, spawns its effect and credits
  the source fighter's stale moves and combo (the scene applies the credit
  after the victim's ProcessHit).
- Phantom victims live in a second 12-slot list per hitbox (`victims_2`),
  inherited by group members like the ordinary list.

Item entries count knockback damage as the truncated staled item damage
(`size_of_xC = (size_t) it_8026B1D4`), and report the pre-capture damage to
their effect.

## Other retail behaviour the generated matches exposed

| Divergence | Retail | Fix |
| --- | --- | --- |
| Attacker SDI-shifted after a special | Only the damage reaction installs `hitlag_cb`/`post_hitlag_cb`; every motion change clears them | `HitlagCallbacks` enum instead of testing leftover damage scratch |
| Charged Shield Breaker unstaled | ftColl_8007ABD0 stales the charged damage (ft_80089228) | `CommandState::stale_damage`, shared by commands and characters |
| Burn effects after a slash hit | Every damaging hit installs its color animation (element or plain flash 4); all are priority 100 in the primary slot | `color_overlay.rs`: one primary slot with retail priorities, fed in request order |
| Charge sparkle drawn while flashing | The secondary slot runs through ft_800BFF70 (no effects or sounds) while the primary is busy | Charge program stepped after the primary, suppressed when it is busy |
| Charge sparkle suppressed after respawn | The invincibility flash (9) is cleared when x1990/x1994 run out, and reinstalled on ledge grabs | Expiry clear and ledge-grab install |
| Marth keeps landing velocity | A grounded mid-animation root-motion entry sets `self_vel.x = gr_vel = offset.z * facing` unless Ft_MF_SkipAnimVel | `MotionChange::skip_animation_velocity`; TurnRun keeps velocity |
| Ledge attacks unstaled | CliffAttack, DownAttack and landing aerials carry move ids | Stale table rows and `StaleMove` variants |
| TurnRun runs past the edge | ftCo_TurnRun_Coll tests Collide_LeftEdge / Collide_RightEdge | Correct flags (teeter keeps Collide_Edge) |
| Marth spends both jumps walking off | ftCommon_GroundToAirStateChange spends one (ftCommon_8007D5D4) | Shield Breaker and Dancing Blade use `leave_ground` |
| Knockback frozen in aerial Fire Fox | Fighter_procUpdate decays airborne knockback after every physics callback | `FighterCore::finish_air_update` for airborne tails |
| Down tilt interrupted into Squat | ftCo_AttackLw3_IASA calls the pure check (800D5F58): down held keeps the tilt | `WaitPredicate::SquatHeld` / `WaitTransition::Hold` |
| Hit while charging a smash launched too weakly | ftCo_Damage_CalcKnockback multiplies by PlCo +7C4 while charging (also +718 frozen, Y scale, armor, +104 floor) | `FighterCore::modified_knockback`, the whole chain |
| Reflector walking off the edge | ftFx_SpecialLw*_GroundToAir: one jump spent, same frame, bubble/reflect state reinstalled | `ground_collision` for all five ground rows |
| Reflector air drift too slow to decay | ftCommon_8007CF58 uses PlCo +1FC above the air drift maximum | Shared `air_drift_friction_acceleration` (Reflector, Fire Fox rebound) |
| Double jump out of the Reflector | Reflector Start never writes turnFrames (+2344), the mv word JumpAerial/Landing inherit | `CharacterCallbacks::retained_scratch_word`; the inherited word is carried |
| No shield SDI/ASDI | ftCo_80092F2C installs ftCo_80093240 / ftCo_800932DC (PlCo +4C0 scale, fmadds) | Guard hitlag callbacks |

## Validation

Debug and release workspace gates, workspace clippy, formatting and the harness
suite pass (numbers in TRACKER.md). The seven generated matches pass fighter,
item and ordered-particle comparisons; two stay allocation-free through GAME.
Retail expectations were never edited.

## Remaining in this area

- Phantom contacts combined with a shield impact in the same frame stop with
  an explicit unimplemented message; so do secondary-slot color programs that
  emit effects outside the smash charge.
- Retail's other overlay fallbacks (0x7A, 8, 0x6B) need states the port does
  not reach.
- Item hitboxes record victims without the `x41_b5` rehit timer, as before.
