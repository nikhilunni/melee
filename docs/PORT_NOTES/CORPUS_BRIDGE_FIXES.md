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
| Shield pushback off the edge enters Fall | Every Guard state collides through ft_800845B4: sliding off the back edge enters MissFoot | `collision::guard` for all five Guard rows |
| Grab released a tick early after a pummel | Only CaptureWait's own Anim tests the expired timer; entering it from CaptureDamage does not release | `CaptureState::release_requested`, consumed by the scene |
| Light hits launch a prone fighter | ftCo_8009F0F0: damage below PlCo +428 keeps a Down* fighter down in DownDamage (U only from DownWaitU) | DownDamageU/D rows, `down_damage_state` |
| No boost grab | The dash attack's IASA (ftCo_800D8AE0) cancels into CatchDash while shield is held within PlCo +68 frames | `MotionData::DashAttack { grab_window }` |
| Dust generators drew in the wrong order | Fighter_ChangeMotionState flushes efAsync before the new script's graphics draw their offsets | `EffectTiming::Sealed` flush before resolving graphics |
| Illusion ghost never paused | Items have hitlag (xCBC, `it_8026B424` fmadds), set from shield hits; the Illusion's DmgDealt clears it for body hits | `ItemCore::hitlag_*`, link-0 countdown, paused animation/physics/accessory |
| Blaster survives a grab or a death | ftCommon_8007DB58 (capture) and death2_cb (ftCo_800D331C) put it away | `Fighter::interrupt_actions`, `CharacterCallbacks::DEATH` |
| Particle lists sorted on every tick | particleSort runs once per display pass; a recording that ran behind executes several ticks per VI frame and renders once | Tracer records psFrameNum (`ps_frame`); replays sort only when it moved |
| New generators inserted mid-list after an effect destruction | hsd_8039D4DC / hsd_8039D688 leave the insertion cursor at the list's tail | `park_cursor_at_tail` |
| Landing dust dispatched before script graphics | Both are script commands queued on the fighter's efAsync stack and flushed newest first | Landing effects resolve with graphics, at their script position |
| No fast fall in the air Blaster or a platform drop | ft_80084DB0 (their physics) checks CheckFallFast and never skips a first frame | `physics_pass`: fast-fall check, gravity, drift |
| Burn flames spawned twice after a shield hit | Only Fighter_ChangeMotionState steps color programs at GuardSetOff entry | Extra step removed from `take_shield_hit` |
| Spot dodge straight out of a forward smash | ftCo_AttackS4_IASA (8008C55C) has no ftCo_80099794 check; shield comes first | `FORWARD_SMASH_PREDICATES` |
| Fox's second head capsule intangible in up-smash | ftColl_8007B128 sets only the first capsule on the bone | `hurt_status` first-on-bone rule |
| Tail springs seeded from a stale pose | Dynamics reclaim a chain from joint matrices that only rendering refreshes | Display caches refreshed only on display passes |
| Fire Fox fall skipped its first physics frame | Only ftCo_Jump_Phys skips a frame; stale Jump scratch must not | Skip limited to JumpF/JumpB |
| Phantom laser from an Illusion | The Blaster's accessory4 callback is cleared by any motion change | Accessory fires only in the firing loop |
| No shield SDI/ASDI | ftCo_80092F2C installs ftCo_80093240 / ftCo_800932DC (PlCo +4C0 scale, fmadds) | Guard hitlag callbacks |

## Validation

Debug and release workspace gates, workspace clippy, formatting and the harness
suite pass (numbers in TRACKER.md). The seven generated matches pass fighter,
item and ordered-particle comparisons; two stay allocation-free through GAME.
Retail expectations were never edited.

## Corpus status (2026-09-26, end of session)

All 48 corpus v2 cases are bridged to retail. 37 (including three committed
prefixes restored above) pass fighter keys, items and ordered particle draws
in `m5_gate::corpus_v2_matches_through_game`. The rest stop at:

- the gameplay camera (8 cases): off-screen magnifier damage
  (fighter.c:1595, PlCo +7AC/+7B0/+7B4) and the screen-KO approach both read
  the camera's projection, computed at render time;
- Fox's tail during GuardSetOff in one case (s0_e2a_p2), probably the same
  display-pass seeding with a recording that predates `ps_frame` checks;
- s1_e12345678_p0 (x at 2044), s0_e49_p2 (facing at 5177) and
  s1_effffffff_p1 (RNG at 3971), not yet diagnosed.

## Remaining in this area

- Phantom contacts combined with a shield impact in the same frame stop with
  an explicit unimplemented message; so do secondary-slot color programs that
  emit effects outside the smash charge.
- Retail's other overlay fallbacks (0x7A, 8, 0x6B) need states the port does
  not reach.
- Item hitboxes record victims without the `x41_b5` rehit timer, as before.
