# Fox–Marth on Final Destination: interaction matrix (2026-09-27)

Draft of the exit criterion in `MATCHUP_COMPLETENESS.md` ("Exit criteria
before breadth"): reachable motion-state transitions and branches for Fox and
Marth on FD, each linked to the gated retail scenarios that witness it, with
missing coverage visible. It complements `COVERAGE_AUDIT.md` (explicit port
boundaries) by looking at what the retail recordings actually exercise.

**Review status.** Drafted by a read-only research agent from the retail
traces and checked by the coordinating session at the family level (the
table structure, the MISSING list against COVERAGE_AUDIT and the port's
boundaries). Rows marked **(by name)** still rest on scenario names or
comments rather than trace evidence. Regenerate the raw tables with
`cd harness && uv run python interaction_matrix.py <out-dir>`.

## How it was built

- **Transitions come from the retail traces, not from the port.** For every
  FD scenario named in `crates/melee-sim/tests/m5_gate.rs` or `m4_gate.rs`
  with a local `harness/traces/<name>.tick.expected.jsonl`, the script read
  `pN.kind` / `pN.motion_id` at each `frame_end` over the gated window only
  (the scenario's `frames`, which the gate asserts). It kept only fighters of
  kind 1 (Fox) and 18 (Marth). That gave 474 scenarios, 2,666 distinct
  (character, from, to) transitions and 351 (character, state) pairs.
  Script: `harness/interaction_matrix.py`, which writes `transitions.tsv`
  (every pair and every directed witness) and `visits.tsv`.
- **Some transitions cannot show up.** Sampling at `frame_end` merges
  motion changes that happen in the same tick (for example, KneeBend -> JumpF
  -> AttackAirN on one tick shows as KneeBend -> AttackAirN). A branch that
  does not change the motion id (L-cancel, powershield timing, DI/SDI,
  short versus full hop, fast fall, C-stick versus main stick, item catches
  that keep the motion) is invisible here. Those rows are marked **(by
  name)**: the witness is taken from the scenario's name or header comment,
  the gate's comments or `docs/PORT_NOTES`, not checked against the trace.
- Fox-vs-Fox directed scenarios (the `idle_fd_fox` savestate) count as Fox
  witnesses because they exercise the same shared code. They are not
  matchup-specific evidence for contact rows.

### Legend

| Mark | Meaning |
| --- | --- |
| `scenario` | A directed gated scenario whose retail trace contains the transition (the first one or two are listed). |
| `+N` | N further directed witnesses. |
| `corpus:N` | Seen only in N generated-corpus traces (`corpus_v2_*`, `corpus_v3_*`, `corpus_sd_*`, all gated in `m5_gate`). They prove exactness but give no targeted regression. |
| (by name) | Inferred from names or comments; the motion ids cannot distinguish the branch. |
| **MISSING** | No gated trace contains it, and it is reachable or its reachability is unknown. |
| n/a | Not reachable for this character or stage; the reason is given. |

## State coverage summary

Every Fox and Marth motion row the port installs has been entered in at
least one gated retail trace, except the rows below. The list covers common
rows that the port implements (`state/common_table.rs`) and the special
rows (`ft-fox-family`, `ft-mars`).

| Character | Never entered in any gated trace | Reachable? |
| --- | --- | --- |
| Fox | CliffAttackSlow, CliffClimbSlow, CliffEscapeSlow, CliffJumpSlow1/2 | Witnessed at 300%: `sudden_death_ledge{climb,roll,attack,jump}_fd_fox` |
| Fox | DamageHi1, DamageLw1, DamageAir1 | Yes, from weak hits: **MISSING** |
| Fox | DownDamageU | Witnessed: `downdamage_up_fd_marth` |
| Fox | ShieldBreakFall, ShieldBreakDownD, ShieldBreakStandD | Yes, a break in the air or in the other orientation: **MISSING** |
| Fox | Fx.SpecialAirLwHit | Yes, an aerial Reflector reflecting something (a Marth-thrown Bob-omb in Sudden Death, or a returned laser): **MISSING** |
| Fox | LightThrow* other than F and Dash | Yes: **MISSING**. An aerial catch, a held landing and LightThrowF are witnessed (`sudden_death_foxcatchthrow_bomb_fd_marth`); LightGet and LightThrowDash are corpus only |
| Fox | Attack13 | n/a: Fox's third jab is the rapid jab (Attack100*, witnessed) |
| Fox | AttackS3HiS, AttackS3LwS | Investigate: Fox only ever enters AttackS3Hi/S/Lw. Check whether the intermediate angles are authored |
| Both | AppealSL | n/a: `enter_common_taunt` needs `left_taunt_available`, which neither character has |
| Both | DeadUpFallHitCameraFlat | Investigate: what selects the flat camera hit |
| Both | Pass | n/a: FD has no platforms |
| Marth | AttackS3Hi/HiS/Lw/LwS | n/a: Marth's forward tilt is not angled. `ftiltup_fd_marth` and `ftiltdown_fd_marth` enter AttackS3S |
| Marth | Attack13, Attack100* | n/a: Marth has a two-hit jab |
| Marth | CliffClimbSlow, CliffEscapeSlow | Yes, at 100% or more: **MISSING**. CliffAttackSlow is corpus:2 and CliffJumpSlow is witnessed only at 300% in Sudden Death |
| Marth | DamageHi1 | Yes: **MISSING** (DamageLw1 and DamageAir1 are corpus only) |
| Marth | DownDamageU | Yes: **MISSING** |
| Marth | CaptureDamageHi | Yes, a pummel on Marth after an airborne catch: **MISSING** |
| Marth | PassiveWall | Yes, a wall tech against FD's side (Fox has corpus:1): **MISSING** |
| Marth | ShieldBreakDownU, ShieldBreakStandU | Yes, the other break orientation: **MISSING** |
| Marth | Ms.SpecialAirNEnd1 | Yes, a fully charged aerial Shield Breaker: **MISSING** |
| Marth | Ms.SpecialAirS4Hi, Ms.SpecialAirS4Lw | Yes, the fourth aerial Dancing Blade hit up or down: **MISSING** |
| Marth | LightThrowAirB, AirHi, AirLw, AirF4, AirHi4, AirLw4 | Yes, air throws of a Bob-omb: **MISSING** (AirF and AirB4 are witnessed) |

Several states are entered only in the corpus: Marth DamageFlyHi (8),
DamageLw2/3, DownWaitD, DownStand*, DownBack*, Passive/PassiveStand*,
SpecialS2Hi, S3Lw, S4Hi and AirS2Hi/AirS3*/AirS4S; Fox EscapeB, TurnRun
entries other than from Run, MissFoot, FlyReflectWall, PassiveWall,
DownDamageD and CaptureDamageHi. They are exact, but no targeted scenario
guards them.

Motion-row coverage is not enough, so the families below list the
transitions.

---

## 1. Ground movement

Retail: `ftCo_Wait_IASA` (`input::WAIT_PREDICATES`, 21 predicates at 0x8008A4E8..A628),
`ftCo_Walk_IASA`, `ftCo_Turn_IASA`, `ftCo_Dash_IASA`, `ftCo_Run_IASA`,
`ftCo_RunBrake_IASA`, `ftCo_TurnRun_IASA`, `ftCo_Squat*_IASA`, `ftCo_Ottotto_IASA`.

| Transition | Retail | Fox | Marth |
| --- | --- | --- | --- |
| Wait -> Dash | ftCo_Dash_CheckInput | `dash_fd_fox` +61 | `dash_fd_marth` +103 |
| Wait -> Turn | ftCo_Turn_CheckInput | `turn_fd_fox` +9 | `turn_fd_marth` +57 |
| Wait -> WalkSlow | ftCo_Walk_CheckInput | `ftilt_fd_fox` +3 | `sudden_death_walkthrow_bomb_fd_marth` +2 |
| Wait -> WalkMiddle directly | ftCo_Walk_Enter | corpus:1 | corpus:1 |
| Walk Slow -> Middle -> Fast | ftCo_Walk_Anim | `walkfast_fd_fox` +5 | `walkfast_fd_marth` +4 |
| Walk -> Wait | ftCo_Walk_IASA | `walkfast_fd_fox` +8 | `ledgeclimb_fd_marth` +5 |
| Walk -> Turn / KneeBend / Catch / tilts / smashes / specials | ftCo_Walk_IASA | mostly corpus:10-30 | `match2_fd_foxmarth`, `match_fd_foxmarth` for some, the rest corpus |
| Wait -> Squat | ftCo_Squat_CheckInput | `squat_fd_fox` +6 | `squat_fd_marth` +2 |
| Squat -> SquatWait -> SquatRv -> Wait | ftCo_Squat_Anim, ftCo_SquatWait_IASA | `squat_fd_fox`, `cc_ftilt_fd_marth` | `squat_fd_marth`, `jump_fd_marth` |
| SquatWait -> Attack11/KneeBend/Dash/AttackS4S/AttackHi4/shield | ftCo_SquatWait_IASA | corpus only (4-13 each) | corpus only |
| SquatWait/Squat -> Counter | ftCo_SquatWait_IASA -> ftMs_SpecialLw_Enter | n/a | `match_fd_foxmarth`, `match2_fd_foxmarth` |
| Turn -> Dash (dash back) | ftCo_Turn_IASA | `match_fd_foxmarth` +7 | `aircounter_fall_fd_marth` +17 |
| Turn -> Wait / WalkSlow | ftCo_Turn_Anim | `match_fd_foxmarth` +3 | `match2_fd_foxmarth` +53 |
| Turn -> KneeBend / Catch / AttackHi4 / shield / taunt | ftCo_Turn_IASA | Fox corpus only (17-31 each) | `match2_fd_foxmarth` for KneeBend and Catch; the rest corpus |
| Dash -> Run | ftCo_Dash_Anim | `dash_fd_fox` +55 | `dash_fd_marth` +97 |
| Dash -> Turn (dash dance) | ftCo_Dash_IASA | `match2_fd_foxmarth` | `match_fd_foxmarth` +2 |
| Dash -> Wait | ftCo_Dash_Anim | `match2_fd_foxmarth` +5 | +3 |
| Dash -> KneeBend | ftCo_Dash_IASA | corpus:34 | `aircounter_fall_fd_marth` +3 |
| Dash -> CatchDash | ftCo_Dash_IASA | corpus:15 | `match2_fd_foxmarth` +1 |
| Dash -> Squat | ftCo_Dash_IASA | corpus:2 | corpus:1 |
| Dash -> Fall (off the edge) | ftCo_Dash_Coll | `match2_fd_foxmarth` +1 | `rebirth_timeout_fd_marth` +5 |
| Run -> RunBrake -> Wait | ftCo_Run_IASA, ftCo_RunBrake_Anim | `dash_fd_fox` +31 | `bair_fd_marth` +80 |
| Run -> TurnRun -> Run | ftCo_Run_IASA, ftCo_TurnRun_Anim | `turnrun_fd_fox` | `turnrun_fd_marth`, `sudden_death_turnrunhold_bomb_fd_marth` |
| TurnRun -> KneeBend (running jump, fn_800CAF78) | ftCo_TurnRun_IASA | corpus:13 | `match2_fd_foxmarth` +1 |
| Run -> KneeBend | ftCo_Run_IASA | corpus:18 | `capture_edge_fox_air_up_release_candidate` +3 |
| Run -> Fall (off the edge) | ftCo_Run_Coll | `rebirth_timeout_fd_fox` +3 | `human_smoke_fd_marth` |
| Run -> Ottotto (edge stop) | ftCo_Run_Coll -> ftCo_Ottotto | `capture_edge_fox_*` (2) | **MISSING** |
| RunBrake -> KneeBend / Squat / TurnRun / Turn | ftCo_RunBrake_IASA | KneeBend `bair_fd_fox` +9; Squat, TurnRun: corpus only | all witnessed (`bair_fd_marth`, `damage_fly_roll_dtilt_t132_fd_fox_candidate`, `match2_fd_foxmarth`) |
| RunBrake -> Ottotto | ftCo_RunBrake_Coll | corpus:4 | **MISSING** |
| Wait/Walk -> Ottotto -> OttottoWait -> Wait | ftCo_Walk_CheckInput_Ottotto, ftCo_Ottotto_IASA | `match_fd_marth_scripted` | `match_fd_marth_scripted` |
| OttottoWait -> Catch / attack | ftCo_OttottoWait_IASA | `capture_edge_fox_*` | **MISSING** |
| Running into a wall -> StopWall | ftCo_StopWall (port: `unimplemented!` in collision.rs) | n/a on FD (side walls are below the ledge; `COVERAGE_AUDIT` lists it as investigate) | same |

## 2. Jumps, aerials, landings

Retail: `ftCo_KneeBend_IASA`, `ftCo_Jump_IASA`, `ftCo_JumpAerial_IASA`,
`ftCo_Fall_IASA`, `ftCo_AttackAir*_IASA`, `ftCo_LandingAir_EnterWithLag`,
`ftCo_Landing_IASA`, `ftCo_EscapeAir_Coll`.

| Transition / branch | Retail | Fox | Marth |
| --- | --- | --- | --- |
| KneeBend -> JumpF / JumpB | ftCo_KneeBend_Anim | `jump_fd_fox` +33 / `ledge_fd_fox` +13 | `jump_fd_marth` +31 / +11 |
| Short hop versus full hop, tap versus X/Y | ftCo_KneeBend_Check_ShortHop, ftCo_Jump_GetInput | (by name) `nairlc_*`, `jump_fd_*` | (by name) |
| Relaxed / C-stick jump entry | ftCo_Jump.c:69-98 (port `unimplemented!`) | Investigate (audit) | same |
| KneeBend -> up special (jump-squat up-B) | ftCo_KneeBend_IASA | `jumpcancel_upb_fd_fox` +2 | `jumpcancel_upb_fd_marth` +2 |
| KneeBend -> Catch (jump-cancel grab) | ftCo_KneeBend_IASA | corpus:15 | `match_fd_foxmarth` |
| KneeBend -> AttackHi4 (jump-cancel up smash) | ftCo_KneeBend_IASA | corpus:14 | `match2_fd_foxmarth` +1 |
| KneeBend -> air dodge -> LandingFallSpecial (wavedash) | ftCo_EscapeAir_Coll | `wavedash_fd_fox`, `airdodge_fd_fox` | `wavedash_fd_marth` |
| JumpF/Fall -> each aerial (N/F/B/Hi/Lw) | ftCo_Fall_IASA_Inner | all five directed (`nair_fd_fox`, `fair_fd_fox`, ...) | all five directed (`*_fd_marth`) |
| JumpF/Fall -> JumpAerialF / JumpAerialB | ftCo_JumpAerial_CheckInput | `jump_fd_fox`, `airjumpb_fd_fox` | `jump_fd_marth`, `airjumpb_fd_marth` |
| JumpAerial* -> FallAerial | ftCo_JumpAerial_Anim | `airjumpb_fd_fox` | `airjumpb_fd_marth` +2 |
| Aerial -> LandingAir* (lag) | ftCo_LandingAir_EnterWithLag | `bair_fd_fox`, `fair_fd_fox`, `dair_fd_fox`, `match2_fd_foxmarth` (Hi, N) | `fairlc_fd_marth`, `dair_fd_marth`, `hitstun_exit_nair_fd_marth`, `match2_fd_foxmarth` (B); LandingAirHi corpus:19 |
| Aerial -> Landing (autocancel) | ftCo_AttackAir_Coll | `match2_fd_foxmarth` (Hi) | `bair_fd_marth`, `fair_fd_marth`, `clank_airborne_marth_spaced_fd_foxmarth` |
| L-cancel (halved lag) | ftCo_LandingAir_EnterWithLag | (by name) `nairlc/fairlc/bairlc/uairlc/dairlc_fd_fox` | (by name) `fairlc_fd_marth`, `dairlc_fd_marth` |
| Aerial ends in the air -> Fall | ftCo_AttackAir_Anim | `match2_fd_foxmarth` | `human_smoke_fd_marth` +2 |
| Fast fall | ft_80084DB0 | (by name) `jump_fd_fox`, `corpus_v2_s1_e2a_p2` | (by name) `jump_fd_marth` |
| Fall -> Landing -> Wait | ftCo_Landing_Anim | `airdodge_fd_fox` +44 | +20 |
| Landing -> interrupts (Dash, Turn, KneeBend, Catch, smash, special, shield) | ftCo_Landing_IASA | `match2_fd_foxmarth`, `match_fd_foxmarth` | `match2_fd_foxmarth`, `match_fd_foxmarth`, `jump_fd_marth` |
| Landing -> SquatWait (held down) | landing::iasa -> enter_landing_squat | `jump_fd_fox`, `cc_ftilt_fd_marth` | `jump_fd_marth` +1 |
| EscapeAir -> FallSpecial -> LandingFallSpecial | ftCo_EscapeAir_Anim | `airdodge_fd_fox` +1 | `airdodge_fd_marth` |
| FallSpecial with aerial jumps left | ftCo_FallSpecial.c:96-100 (port `unimplemented!`) | Investigate: likely unreachable because entering spends the jumps | same |
| FallSpecial catch window (ftCo_800D705C) | ftCo_FallSpecial_IASA | n/a (items only) | (by name) `corpus_sd_s1_eea202b0d_p2` keeps an item; the window opening is **MISSING** (audit) |
| JumpAerialF -> PassiveWallJump -> Fall | ftCo_WallJump (ftCo_PassiveWall) | `walljump_right_underside_fd_fox_candidate` | n/a (Marth cannot wall jump) |
| JumpAerialF -> StopCeil -> Fall | ftCo_StopCeil_Coll | `stopceil_latejump266_fd_fox_candidate` | `stopceil_left_latejump270_fd_marth_candidate` |
| Repeated or mirrored wall jumps, ceiling-triggered ledge exits | ftCo_PassiveWall_IASA | **MISSING** (MATCHUP_COMPLETENESS) | n/a |

## 3. Ground attacks and attack interrupts

Retail: `ftCo_Attack1_CheckInput`, `ftCo_Attack11_IASA`, `ftCo_Attack100Loop_IASA`,
`ftCo_AttackS3_CheckInput`, `ftCo_AttackS4_CheckInput`/`_IASA`, `ftCo_AttackHi4_CheckInput`,
`ftCo_AttackLw3_IASA`, `ftCo_AttackDash_IASA`.

| Transition / branch | Retail | Fox | Marth |
| --- | --- | --- | --- |
| Wait -> Attack11 -> Attack12 | ftCo_Attack11_IASA | `jabcombo_fd_fox` +2 | `jabcombo_fd_marth` +1 |
| Attack12 -> Attack100Start -> Loop -> End | ftCo_Attack100Loop_IASA | `jabcombo_fd_fox` +2 | n/a |
| Jab continued from Wait/Walk inside the window | count_down_jab_window | (by name) corpus fix, `sudden_death_jab_bomb_fd_marth` | same |
| Attack11 -> ReboundStop (clank) | ftColl clank path | `clank_jab_s74_f122_fd_foxmarth` | same |
| Walk -> AttackS3S; angled Hi/Lw | ftCo_AttackS3_CheckInput | `ftilt_fd_fox`, `ftiltup_fd_fox`, `ftiltdown_fd_fox` | `ftilt_fd_marth`, `cc_ftilt_fd_marth` +7 (not angled) |
| AttackS3HiS / LwS | ftCo_AttackS3 | Investigate: is it authored? | n/a |
| AttackS4S from Wait; stick and C-stick; diagonal | ftCo_AttackS4_CheckInput | `fsmash_diagonal_fd_fox`, `fsmashcharge_fd_fox` +9 | `fsmash_diagonal_fd_marth` +11 |
| Dash -> AttackS4S (dash fsmash) | ftCo_Dash_IASA | `fsmash_dash_diagonal_fd_fox` | `fsmash_dash_diagonal_fd_marth` +2 |
| Smash charge, and knockback while charging | ftCo_AttackS4_8008C114 | (by name) `fsmashcharge_fd_fox`, `corpus_v2_s0_e12345678_p2` | (by name) `fsmashcharge_fd_marth` |
| AttackS4 IASA -> shield, never a spot dodge | ftCo_AttackS4_IASA | (by name) `corpus_v2_s1_e49_p2` | same |
| AttackHi3 / AttackHi4 / AttackLw4 | ftCo_AttackHi3/Hi4/Lw4_CheckInput | `utilt_fd_fox`, `usmash_fd_fox`, `dsmash_fd_fox` | `utilt_fd_marth`, `usmash_fd_marth`, `dsmash_fd_marth` |
| Squat -> AttackLw3 -> SquatRv / held into SquatWait | ftCo_AttackLw3_IASA | `dtilt_fd_fox` +2 | `dtilt_fd_marth`, `damage_fly_roll_dtilt_t132_fd_fox_candidate` |
| AttackLw3 IASA -> up special | ftCo_AttackLw3_IASA | n/a | (by name) `corpus_v3_s1_e8be4d273_p0` |
| Run -> AttackDash -> Wait | ftCo_AttackDash_CheckInput | `dashattack_fd_fox` +7 | `dashattack_fd_marth` +4 |
| AttackDash -> CatchDash (boost grab) | ftCo_AttackDash_IASA | corpus:6 | corpus:8 |
| AttackDash -> KneeBend | ftCo_AttackDash_IASA | `match_fd_foxmarth` | `match2_fd_foxmarth` |
| AttackDash ends at an edge -> Ottotto | ftCo_AttackDash_Coll | corpus:1 | **MISSING** |
| Taunt: Wait/Dash/Run -> AppealSR; taunt IASA | ftCo_AppealS_IASA, ftCo_800DEAE8 | `taunt_fd_fox`, `dash_taunt_fd_fox`, `run_taunt_fd_fox` | `taunt_fd_marth`, `dash_taunt_fd_marth`, `run_taunt_fd_marth` |
| AppealSR -> action through the taunt's IASA | ftCo_AppealS_IASA | corpus only | corpus only |
| Hit or clank ends the attack interaction on a motion change | fighter.c | (by name) corpus v3 header | same |

## 4. Fox specials

Retail: `ftFx_SpecialN_Enter`, `ftFx_SpecialS_Enter`, `ftFx_SpecialHi_Enter`,
`ftFx_SpecialLw_Enter`, plus their `_GroundToAir`/`_AirToGround` hooks.

| Transition / branch | Retail | Witness |
| --- | --- | --- |
| Blaster: NStart -> NLoop -> NEnd -> Wait | ftFx_SpecialNLoop_IASA | `laser_fd_fox` +9 |
| NEnd -> Attack11 / Reflector / Walk / Turn (IASA) | ftFx_SpecialNEnd_IASA | `match2_fd_foxmarth`, `laser_reflect_return_boundary_fd_marth`, `match_fd_foxmarth` |
| Air blaster: AirNStart -> AirNLoop -> AirNEnd -> Fall | ftFx_SpecialAirN* | `match2_fd_foxmarth`, `laser_reflect_overflow_air_timed_fd_marth` +3 |
| AirNStart/AirNLoop/AirNEnd -> Landing (air to ground) | ftFx_SpecialAirNLoop_Coll | `laser_reflect_overflow_air_timed_fd_marth` +2, `match_fd_foxmarth`, `match2_fd_foxmarth` |
| Ground blaster leaving the ground (NLoop -> air) | ftFx_SpecialNLoop_Coll | **MISSING** (Fox fires lasers at the edge often) |
| Blaster put away on grab, death or loop exit | ftFx_SpecialN_RemoveBlaster | (by name) `corpus_v2_s0_e49_p1`, `corpus_v2_s1_e2a_p1`, `corpus_v2_s1_e1_p2` |
| Illusion: SStart -> S -> SEnd -> Wait | ftFx_SpecialS_Enter | `illusion_fd_fox` +2 |
| Wait -> SStart (standing Illusion) | ftCo_Wait_IASA | corpus:49 |
| S -> AirS (dash leaves the stage) | ftFx_SpecialS_GroundToAir | `match_fd_foxmarth` |
| SStart -> AirSStart (at the edge) | ftFx_SpecialSStart_GroundToAir | corpus:2 (`corpus_v3_s0_e4f8edfa8_p0`, by name) |
| AirSStart -> SStart (landing) | ftFx_SpecialAirSStart_AirToGround | `illusion_start_landing_fd_fox` |
| AirS -> S (landing mid-dash) | ftFx_SpecialAirS_AirToGround | corpus:1 |
| AirSStart -> AirS -> AirSEnd -> FallSpecial / LandingFallSpecial | ftFx_SpecialAirSEnd_Enter | `airillusion_fd_fox` +1, `match_fd_foxmarth` |
| AirSEnd -> CliffCatch | ftCo_CliffCatch via collision | corpus:6 |
| Illusion ghost hitlag against a shield; ghost against Marth's down smash/Counter | ftFx_SpecialS_CheckGhostRemove, ftColl_80077970 | (by name) `corpus_v2_s1_e12345678_p1`, `corpus_v2_s0_e1_p1` |
| Fire Fox: HiHold -> Hi (ground) -> HiLanding | ftFx_SpecialHi_Enter | `firefox_charge_hit_fd_marth` +1 |
| HiHold -> AirHi (launch) -> HiFall -> FallSpecial | ftFx_SpecialHi_IASA | `firefox_charge_landing_fd_fox` +5, `airfirefox_fd_fox` +7 |
| HiHoldAir -> HiHold (charge landing) | ftFx_SpecialHiHoldAir_AirToGround | `firefox_charge_landing_fd_fox` |
| HiHold -> HiHoldAir (charge leaving the ground) | ftFx_SpecialHiHold_GroundToAir | corpus:1 |
| Hi -> AirHi (ground travel leaves the edge) | ftFx_SpecialHi_GroundToAir | `firefox_ground_launch_fd_fox` |
| AirHi -> HiBound (floor rebound) -> FallSpecial | ftFx_SpecialHiBound_Enter | `firefox_floor_rebound_fd_fox` |
| HiFall -> HiLanding | ftFx_SpecialHiFall_AirToGround | `firefox_end_air_landing_fd_fox` |
| AirHi / HiFall / HiHoldAir -> CliffCatch | ftFx_SpecialAirHi_Coll | corpus:4 / corpus:13 / corpus:3 |
| AirHi travel landing without a rebound (AirHi -> Hi) | ftFx_SpecialAirHi_AirToGround | **MISSING** (MATCHUP_COMPLETENESS: remaining travel landing) |
| Fire Fox travel into FD's wall | ftFx_SpecialHi_Coll | **MISSING** (wall/ledge combinations still open) |
| Fire Fox platform skip | ftCo_8009A134 | n/a (FD has no platforms) |
| Reflector: LwStart -> LwLoop -> LwEnd -> Wait | ftFx_SpecialLw_Enter | `reflector_fd_fox` +5 |
| LwLoop -> KneeBend (jump cancel) | ftFx_SpecialLwLoop_IASA | `reflectorjc_fd_fox` |
| LwLoop -> LwTurn -> LwLoop / LwEnd (release in the turn) | ftFx_SpecialLwTurn_Check | `reflectorturn_fd_fox` +1, `reflectorturn_release_fd_fox` |
| LwStart -> LwTurn | ftFx_SpecialLwTurn_Check | corpus:3 |
| LwStart -> LwHit -> LwLoop (reflect on the ground) | ftFx_SpecialLwHit_Check | `laser_reflect_return_boundary_fd_marth` |
| Air: AirLwStart -> AirLwLoop -> AirLwEnd -> Fall | ftFx_SpecialAirLw* | `airreflector_fd_fox` +6, `match_fd_foxmarth` |
| AirLwLoop -> JumpAerialF (button or tap jump cancel) | ftFx_SpecialAirLwLoop_IASA | `airreflectorjc_fd_fox`, `airreflectortapjc_fd_fox` (by name for tap versus button) |
| AirLwLoop <-> AirLwTurn; priority with a jump | ftFx_SpecialAirLwTurn_* | `airreflectorturn_fd_fox`, `airreflectorturn_priority_fd_fox` |
| AirLwTurn -> LwTurn (landing keeps the turn) | ftFx_SpecialAirLwTurn_* | `airreflectorturn_landing_fd_fox` |
| AirLwLoop -> LwEnd / LwLoop (landing) | ftFx_SpecialAirLwLoop_AirToGround | `airreflector_fd_fox` +2 / corpus:3 |
| AirLwEnd -> LwEnd (landing) | ftFx_SpecialAirLwEnd_AirToGround | corpus:6 |
| LwStart -> AirLwStart (started at the edge) | ftFx_SpecialLwStart_GroundToAir | `reflector_runedge_jump_fd_fox` (out of Run, jump-cancelled) / corpus:1 |
| LwLoop / LwEnd / LwTurn / LwHit leaving the ground | ftFx_SpecialLw{Loop,End,Turn,Hit}_GroundToAir | (by name) `corpus_v2_s1_edeadbeef_p0` reports a walk-off, but the trace shows no Lw* -> AirLw* pair: **MISSING** as a transition, recheck |
| AirLwHit (aerial reflect) and AirLwHit -> LwHit landing | ftFx_SpecialAirLwHit_* | **MISSING** |
| Reflector turnFrames inherited from an unmodelled scratch word | special_lw.rs `unimplemented!` | Investigate (audit) |
| Reflector platform drop (CheckPass) | ftFx_SpecialLwStart_CheckPass | n/a (FD) |

## 5. Marth specials

Retail: `ftMs_SpecialN_Enter`, `ftMs_SpecialS_Enter`, `ftMs_SpecialHi_Enter`,
`ftMs_SpecialLw_Enter`, `ftMs_SpecialLw_80139140` (Counter hit processing).

| Transition / branch | Retail | Witness |
| --- | --- | --- |
| Shield Breaker: NStart -> NEnd0 (tap) | ftMs_SpecialN_Enter | `match_fd_foxmarth` +1 |
| NStart -> NLoop -> NEnd0 (partial charge) | ftMs_SpecialNLoop_Anim | `shieldbreaker_fd_marth` +3 |
| NLoop -> NEnd1 (full charge) -> Wait | ftMs_SpecialNLoop_Anim | `match_fd_foxmarth`, `match2_fd_foxmarth` |
| Charge gusts on the cape and tail (lb_800119DC) | ftMs_SpecialN_801365A8 | (by name) `corpus_v2_s0_e2a_p2` |
| Air: AirNStart -> AirNLoop -> AirNEnd0 -> Fall | ftMs_SpecialAirN* | `match2_fd_foxmarth` +1 |
| AirNEnd0 -> NEnd0 (landing) | ftMs_SpecialAirNEnd0 collision | `match2_fd_foxmarth` +1 |
| AirNLoop -> NLoop (landing while charging) | ftMs_SpecialAirNLoop_Coll | corpus:10 |
| NStart / NLoop / NEnd0 -> air (leaving the ground) | ground collision | corpus:3 / corpus:1 / corpus:1 (`corpus_v2_s1_e12345678_p1`, by name) |
| AirNEnd1 (aerial full charge) | ftMs_SpecialAirNLoop_Anim | **MISSING** |
| Dancing Blade: S1 -> S2Lw -> S3Hi -> S4Lw | ftMs_SpecialS_Enter, special_s::input | `dancingblade_fd_marth` |
| S2Lw -> S3S -> S4S | same | `match_fd_foxmarth` |
| S1 -> S2Hi; S2Hi -> S3*; S3Hi -> S4Hi / S4S; S3S -> S4Hi | same | corpus only (17 / 1-2 / 2 / 1 / 1) |
| S3Lw (ground) | same | corpus:2 |
| Dancing Blade ending -> Wait / Walk / KneeBend (IASA) | ftMs_SpecialS*_Anim | `match2_fd_foxmarth` +2 |
| Air: AirS1 -> AirS2Lw -> Fall | ftMs_SpecialAirS_Enter | `match2_fd_foxmarth` |
| AirS1 -> AirS2Hi; AirS2* -> AirS3*; AirS3Hi -> AirS4S | same | corpus only |
| AirS4Hi, AirS4Lw | same | **MISSING** |
| Aerial Dancing Blade landing (AirSn -> Sn) | ftMs_SpecialAirS* collision | corpus only (AirS1 2, AirS2Hi 4, AirS2Lw 10, AirS3* 1-2) |
| Ground Dancing Blade leaving the ground (Sn -> AirSn) | ftMs_SpecialS* collision | **MISSING** (never observed; confirm the ground rows can depart) |
| Dancing Blade mv+4 inherited from an unmodelled scratch word | special_s.rs `unimplemented!` | Investigate (audit) |
| Dolphin Slash: Wait -> SpecialHi -> FallSpecial -> LandingFallSpecial | ftMs_SpecialHi_Enter | `dolphinslash_fd_marth` +5 |
| AirHi -> FallSpecial | ftMs_SpecialAirHi_Anim | `match2_fd_foxmarth` |
| AirHi -> LandingFallSpecial / CliffCatch | ftMs_SpecialAirHi_Coll | corpus:2 / corpus:5 |
| Jump-squat up-B, diagonal and priority | ftCo_KneeBend_IASA | `jumpcancel_upb_{,priority_,diagonal_}fd_marth` |
| Shield-recoil decay when Dolphin Slash leaves the ground | Fighter_procUpdate | (by name) `corpus_v2_s1_e12345678_p0` |
| Shield Breaker leaving the ground with one jump | ground collision | (by name) `corpus_v2_s1_e12345678_p1` |
| Counter: SpecialLw -> Wait (miss) | ftMs_SpecialLw_Anim | `aircounter_fd_marth` +3 |
| SpecialLw -> SpecialLwHit -> Wait (catch) | ftMs_SpecialLw_80139140 | `counter_fd_marth` +1 |
| AirLw -> Lw (landing in the stance) | ftMs_SpecialAirLw_Coll | `aircounter_landing_fd_marth`, `aircounter_fd_marth` |
| AirLw -> Fall (offstage end) | ftMs_SpecialAirLw_Anim | `aircounter_fall_fd_marth` |
| AirLw -> AirLwHit -> LwHit (landing) | ftMs_SpecialAirLwHit_* | `aircounter_hit_fd_marth` |
| AirLwHit ending in the air -> Fall | ftMs_SpecialAirLwHit_Anim | **MISSING** |
| Counter catching an item (laser, Illusion ghost, Fire Fox) | ftColl_80077688 | (by name) `corpus_v3_s0_e6117d326_p2` (laser), `corpus_v2_s0_e1_p1` (ghost), `corpus_v3_s0_e1cda1301_p0` (Fire Fox, stale powershield branch) |
| Counter volume dropped on a motion change (grabbed or hit out of the stance) | fighter.c:1049 | (by name) `corpus_v3_s1_e9943b4ab_p0` |
| Counter catch without the minimum hitlag after landing | shield_unk0 | (by name) `corpus_v3_s0_e5f386e5e_p0` |
| Lw / LwHit leaving the ground | ftMs_SpecialLw_Coll | **MISSING** (the audit calls it "deliberate support loss during both phases") |

## 6. Shield and dodges

Retail: `ftCo_GuardOn_IASA`, `ftCo_Guard_IASA`, `ftCo_GuardReflect_IASA`,
`ftCo_GuardSetOff_IASA`, `ftCo_GuardOff_IASA`, `ftCo_Escape_IASA`, `ftCo_EscapeN_IASA`.
The trace shows a digital press entering GuardReflect (182) and an analog
press entering GuardOn (178).

| Transition / branch | Retail | Fox | Marth |
| --- | --- | --- | --- |
| Wait -> GuardReflect -> Guard -> GuardOff -> Wait | ftCo_GuardReflect_IASA | `shield_fd_fox`, `shieldhit_fd_marth` +10 | `shield_fd_marth`, `laser_shield_fd_marth` +13 |
| Wait -> GuardOn -> Guard (light press) | ftCo_GuardOn_IASA | `lightshield_ftilt_fd_marth` +1 | `laser_lightshield_fd_marth` +2 |
| GuardOn -> GuardReflect (delayed powershield) | ftCo_GuardOn_IASA | `shield_delayed_power_fd_fox` | `shield_delayed_power_fd_marth`, `laser_reflect_delayed_timed_fd_marth` |
| Dash -> shield | ftCo_Dash_IASA | `dash_shield_fd_fox`, `dash_late_shield_fd_fox` +1 | `dash_shield_fd_marth` +3 |
| Run -> shield | ftCo_Run_IASA | `run_shield_fd_fox` +1 | `run_shield_fd_marth` +2 |
| GuardReflect -> Catch / CatchDash (shield grab, run-shield grab) | ftCo_Catch_CheckInput | `shield_grab_fd_fox`, `dash_late_shield_grab_fd_fox`, `run_shield_grab_fd_fox` | the three `_marth` variants |
| GuardReflect -> KneeBend (C-stick jump out of shield) | ftCo_Guard_IASA | `shield_cstick_jump_fd_fox` | `shield_cstick_jump_fd_marth` +1 |
| GuardReflect -> EscapeF / EscapeN | ftCo_Guard_IASA | `roll_fd_fox`, `spotdodge_fd_fox` | `roll_fd_marth`, `spotdodge_fd_marth` |
| GuardReflect -> EscapeB | same | corpus:24 | `sudden_death_rollhold_bomb_fd_marth` |
| **Guard (mature shield) -> EscapeN / EscapeF / EscapeB / KneeBend / Catch** | ftCo_Guard_IASA | corpus only (15-35 each) | corpus only (19-33 each) |
| GuardOn -> Escape* / KneeBend / Catch | ftCo_GuardOn_IASA | corpus only | corpus only |
| GuardOff -> KneeBend / EscapeN | ftCo_GuardOff_IASA | corpus only | `match_fd_foxmarth` (KneeBend); EscapeN corpus |
| Shield hit: Guard -> GuardSetOff -> Guard | ftCo_GuardSetOff_Anim | `shieldhit_fd_marth`, `shieldstun_ftilt_fd_marth`, `shieldtilt_ftilt_fd_marth` +3 | `laser_shield_fd_marth` +3 |
| Powershield against a melee hit (Fox shields Marth / Marth shields Fox) | ftColl_80076CBC | (by name) `powershield_ftilt_fd_marth` | **MISSING** (Marth only powershields lasers) |
| Shield SDI/ASDI during shield hitlag | ftCo_GuardSetOff | (by name) `corpus_v2_s0_e1_p0` | same |
| Shield pushed off the edge -> MissFoot | ftCo_Guard*_Coll | corpus:3 (GuardOn, GuardOff, GuardSetOff) | corpus:1 |
| Guard -> ShieldBreakFly -> DownU -> StandU -> Furafura | ftCo_ShieldBreakFly_Anim | `shieldbreak_fd_marth` | `laser_reflect_overflow_air_timed_fd_marth` (Fall/DownD/StandD path) |
| Break in the other orientation (Fox D, Marth U) | ftCo_ShieldBreakDown_Anim | **MISSING** | **MISSING** |
| Furafura -> Wait (dizzy wears off); Furafura -> damage | ftCo_Furafura_Anim / hit | `furafura_expire_fd_marth`, `furafura_hit_fd_marth` | **MISSING** |
| Shield break at the edge or offstage (fly into the blast zone) | ftCo_ShieldBreakFly_Coll | **MISSING** | **MISSING** |
| Simultaneous shield impacts (two hitboxes in one frame) | ftColl_80076CBC | Ported (the strongest impact wins, getEnvDmg rounding); unwitnessed: every Fox/Marth hitbox is group 0, so only a laser and a melee hit together can do it | same |
| Phantom contact and shield impact in the same frame | fighter.c:2907 | Ported (the phantom branch drops the impact's response); unwitnessed: the phantom band is 0.01 of overlap | same |
| Dash -> EscapeF (dash defense) | ftCo_Dash_IASA | `dash_escape_fd_fox` | `dash_escape_fd_marth` |
| EscapeF/B/N -> Wait / GuardOn; IASA actions | ftCo_Escape_IASA | `roll_fd_fox`, `spotdodge_fd_fox`; the rest corpus | `roll_fd_marth`, `spotdodge_fd_marth`; the rest corpus |
| Roll with an item: A is a smash throw | ftCo_8009563C | Fox **MISSING** | (by name) no directed throw-from-roll: **MISSING** |

## 7. Grabs, throws, capture

Retail: `ftCo_Catch_CheckInput`, `ftCo_Catch_Anim`, `ftCo_CatchWait_IASA`,
`ftCo_Throw*_Anim`, `ftCo_Capture*_IASA`, `ftCo_Thrown*_Anim`, ftCo_8008EC90
(damage on a grab pair).

| Transition / branch | Retail | Fox as captor | Marth as captor |
| --- | --- | --- | --- |
| Catch -> CatchPull -> CatchWait | ftCo_Catch_Anim | `cstick_throw_*_fd_fox` +12 | `grab_fd_marth`, `cstick_throw_*_fd_marth` +25 |
| Catch miss -> Wait | ftCo_Catch_Anim | `dash_late_shield_grab_fd_fox` +3 | `dash_late_shield_grab_fd_marth` +3 |
| CatchDash -> CatchDashPull -> CatchWait | ftCo_CatchDash_Anim | `sudden_death_grabbedhold_bomb_fd_marth` +4 | `match2_fd_foxmarth` +3 |
| CatchWait -> CatchAttack (pummel) -> CatchWait | ftCo_CatchWait_IASA | `match2_fd_foxmarth` | `pummel_fd_marth` +2 |
| CatchWait -> ThrowF/B/Hi/Lw, main stick and C-stick priority | ftCo_CatchWait_IASA | `cstick_throw_{forward,back,up,down,down_pulse,horizontal_priority,main_priority}_fd_fox` | same `_marth`, plus `fthrow/uthrow/dthrow_fd_marth` |
| CatchWait -> CatchCut (mash-out release) | grab_escape | `grab_airborne_fd_foxmarth` +5 | `grabmash_fd_marth` +4 |
| Grab and throw stop at the edge (ft_800841B8) | ft_800827A0 | (by name) corpus v3 header | same |
| Grab release only from CaptureWait's own callback | ftCo_CaptureWaitLw_IASA | (by name) `corpus_v2_s0_e80000000_p2` | same |
| Grabbing puts the Blaster away | ftCommon_8007DB58 | (by name) `corpus_v2_s0_e49_p1` | n/a |

| Victim transition | Retail | Fox victim | Marth victim |
| --- | --- | --- | --- |
| Wait -> CapturePulledLw -> CaptureWaitLw | ftCo_CapturePulledLw_Anim | `capture_jump_up_release_fd_marthfox_candidate` +26 | +14 |
| JumpF -> CapturePulledHi (airborne victim) | ftCo_CapturePulledHi | `grab_airborne_fd_marthfox` | `capture_edge_fox_air_up_release_candidate` +1 |
| CapturePulledHi -> CaptureWaitHi | same | corpus:3 | `capture_edge_fox_*` |
| CaptureWaitLw -> CaptureDamageLw -> CaptureWaitLw (pummel) | ftCo_CaptureDamageLw_Anim | `match_fd_foxmarth` +2 | `match2_fd_foxmarth` |
| CaptureWaitHi -> CaptureDamageHi | ftCo_CaptureDamageHi_Anim | corpus:1 | **MISSING** |
| CaptureWaitLw -> CaptureCut -> Wait | ftCo_CaptureCut_Enter | `grabmash_fd_marth`, `pummel_fd_marth` +1 | `grab_airborne_fd_foxmarth` +1 |
| CaptureWaitHi -> CaptureCut -> Fall (air Cut) | same | **MISSING** | `capture_edge_fox_outward_stop43_jump109_grab107_candidate` |
| CaptureWait -> CaptureJump -> Landing / Fall | ftCo_CaptureJump_Anim | `capture_jump_{up_release,xy_latch}_fd_marthfox_candidate` | `capture_jump_*_fd_foxmarth_candidate`, `capture_edge_fox_air_up_release_candidate` |
| CaptureWaitHi -> Thrown* | ftCo_Thrown*_Anim | corpus:3 | corpus:3 |
| ThrownF/B/Hi/Lw -> DamageAir/DamageFly | ftCo_Thrown*_Anim | `cstick_throw_*_fd_marth` | `cstick_throw_*_fd_fox` |
| Thrown into the wall -> PassiveWall | ftCo_PassiveWall | (by name) `corpus_v3_s1_edb4b01fd_p0`; the trace attributes the PassiveWall to Fox, while the comment says Marth | **MISSING** by trace |
| Meteor cancel of a down throw off the ledge | ftCo_JumpAerial_CheckInput | n/a | (by name) `corpus_v3_s1_e8be4d273_p1` |
| Thrown positioning waits out hitlag | Fighter_CallAcessoryCallbacks_8006C624 | (by name) corpus v3 | same |
| Throw damage deferred to ProcessHit (no roll at release) | x1838 | (by name) `corpus_v3_s0_e8be4d273_p0` | same |
| Dying grabber releases the victim | ftCo_800DD100 | (by name) corpus v3 | same |
| Pair launched by a Bob-omb: both, captor only, victim only, pummel frame | ftCo_8008EC90, ftCo_800DCFD4, ftCo_800DE2F0, ftCo_800DE854 | `sudden_death_grabbomb_fd_marth`, `sudden_death_grabbombcaptor_fd_marth`, `sudden_death_releasecaptor_bomb_fd_marth`, `sudden_death_pummelcaptor_bomb_fd_marth`, `sudden_death_grabbedhold_bomb_fd_marth` | same |
| Light third-party hit on the captured member | ftCo_800DE854 (port `unimplemented!`) | **MISSING**: reachable with a laser fired before the grab or a Bob-omb blast | same |
| Only the victim launched (non-item) / armoured members | ftCo_800DE2F0 (partly `unimplemented!`) | **MISSING** outside Sudden Death | same |
| Captured damage outside low capture or throw | grab_escape.rs `unimplemented!` | **MISSING** (audit: reachable) | same |
| Grab pair losing its floor | ftCo_800DC920 (collision.rs `unimplemented!`) | Investigate | same |
| Fox-article hits during capture | fighter.c capture branch | **MISSING** (MATCHUP_COMPLETENESS lists it as outstanding) | same |

## 8. Damage, tumble, down, tech

Retail: `ftCo_Damage_IASA`, `ftCo_Damage_CheckAirMotion`, `ftCo_DamageFall_IASA`,
`ftCo_DamageFly_Coll`, `ftCo_DamageFlyRoll_*`, `ftCo_Passive_*`, `ftCo_PassiveStand_*`,
`ftCo_DownBound_Anim`, `ftCo_Down_CheckInput`, `ftCo_DownDamage_*`, `ftCo_FlyReflect_Coll`,
`ftCo_MissFoot_*`.

| Transition / branch | Retail | Fox | Marth |
| --- | --- | --- | --- |
| Ground hit -> DamageN1/N2/N3, Hi2/Hi3, Lw2/Lw3 -> Wait / Landing | ftCo_Damage_Anim | N1 `jab_fd_fox`; N2 `ftilt_fd_fox` +10; N3 `illusion_fd_fox` +5; Hi2 `nair_fd_fox` +5; Hi3 `fsmashcharge_fd_fox` +3; Lw2 `dair_fd_fox` +4; Lw3 `dtilt_fd_fox` +2 | N1 `sudden_death_releasecaptor_bomb_fd_marth` +1; N2 `firefox_charge_hit_fd_marth` +1; N3 `match_fd_foxmarth`; Hi2 `match_fd_foxmarth`; Hi3 `match2_fd_foxmarth`; Lw2/Lw3 corpus only |
| DamageHi1 / DamageLw1 / DamageAir1 | same, weak-hit selection | **MISSING** (all three) | Hi1 **MISSING**; Lw1 corpus:1; Air1 corpus:1 |
| Crouch cancel (SquatWait -> DamageHi2) | ftCo_Damage.c:124-127 | `cc_ftilt_fd_marth` | **MISSING** |
| Air hit -> DamageAir2/3 -> Fall / Landing | ftCo_Damage_CheckAirMotion | `clank_airborne_fox_spaced_fd_foxmarth`, `cstick_throw_forward_fd_marth` +1 | `firefox_charge_hit_fd_marth` +1, `cstick_throw_forward_fd_fox` +1 |
| Hitstun exit: DamageAir3 -> aerial / EscapeAir; unbuffered attack | ftCo_Damage_IASA | (by name) `hitstun_exit_{nair,fair}_fd_fox`, `hitstun_shield_priority_fd_fox` | `hitstun_exit_nair_fd_marth`, `hitstun_exit_fair_fd_marth`, `hitstun_shield_priority_fd_marth`, `hitstun_unbuffered_attack_fd_marth` |
| DamageFlyTop -> Landing on the hitstun-expiry tick | ftCo_DamageFly_Coll | `hitstun_exit_fair_fd_fox` +2 | corpus:2 |
| DamageFall / DamageFly -> JumpAerialF / special / air dodge after hitstun | ftCo_DamageFall_IASA | `match_fd_foxmarth` (DamageFlyN -> JumpAerialF); DamageFall -> anything: corpus only | DamageFlyTop -> Ms.SpecialAirNStart `match_fd_foxmarth`; the rest corpus |
| Tumble exit by a stick flick -> Fall | ftCo_DamageFall_IASA | corpus:1 | `match2_fd_foxmarth` |
| Airborne transition in ftCo_Damage_IASA outside {Attack, Jump, Escape, AirSpecial} | damage.rs `unimplemented!` | Investigate: air item throw or catch from hitstun in Sudden Death | same |
| DamageFly* -> DamageFall | ftCo_DamageFly_Anim | `di_upaway_fsmash_fd_marth` +4 | `cstick_throw_back_fd_fox` +3 |
| DamageFlyN / Hi / Lw / Top | knockback-angle selection | all four directed | Hi corpus:8; N, Lw, Top directed |
| DamageFlyRoll -> DeadRight / DamageFall / DownBoundU | ftCo_DamageFlyRoll_Coll | `damage_fly_roll_t125/dtilt_t132/crouch_fd_fox_candidate` | Sudden Death only (300%): `sudden_death_airdodgehold_bomb_fd_marth`, `sudden_death_dashhold_bomb_fd_marth`; ordinary-percent roll **MISSING** (audit: "Marth witness") |
| DamageFlyTop ends keeping fast fall | ftCo_80090780 | (by name) `corpus_v3_s1_e46703f61_p0` | same |
| Tumble -> DownBoundU/D (missed tech) | ftCo_DownBound_Anim | `di_downin_fsmash_fd_marth`, `cstick_throw_down_fd_marth` +10 | `cstick_throw_up_fd_fox` +1 |
| DownBound -> DownWait -> getup (stand, roll F/B, attack) | ftCo_Down_CheckInput | face up: `getupstand_fd_fox`, `getuproll_fd_fox`, `getupattack_fd_fox`; face down: `match2_fd_foxmarth`, `match_fd_foxmarth` (attack, forward, back, stand) | DownWaitU -> getups corpus only; DownBound -> DownAttack/DownFoward directed (`match2_fd_foxmarth`, `match_fd_foxmarth`); DownStand*/DownBack* corpus only |
| DownWait -> DownBack (face up) | same | `match_fd_foxmarth` | corpus:2 |
| DownBound -> Fall (bounce off the edge) | ftCo_DownBound_Coll | `match2_fd_foxmarth` +1 | corpus:1 |
| DownWait hit -> DownDamageU/D | ftCo_DownDamage_* | U `downdamage_up_fd_marth`; D corpus:1 (by name `corpus_v2_s0_effffffff_p2`, `corpus_v3_s0_e0fe4dd03_p1`) | U **MISSING**; D corpus:4 |
| DownDamage keeping facing / hitstun air physics | ftCo_8008DCE0 | (by name) `corpus_v2_s0_e49_p2`, `corpus_v3_s0_e0fe4dd03_p1` | same |
| DownReflect wall bounce, DownDamage wall tech/bounce | ftCo_800C7CA0, ftCo_800C1D38 (port `unimplemented!`) | Investigate (audit) | same |
| DownSpot | ftCo_DownSpot_Enter | Investigate: never seen and unported | same |
| Tech in place (Passive) | ftCo_Passive_* | `match2_fd_foxmarth` | corpus only (7) |
| Tech roll (PassiveStandB / F) | ftCo_PassiveStand_* | B `tech_fd_marth`; F corpus:6 | corpus only |
| Wall tech (PassiveWall) | ftCo_PassiveWall_* | corpus:1 | **MISSING** |
| Tumble into a wall -> FlyReflectWall | ftCo_FlyReflect_Coll | corpus:1 (`corpus_v3_s0_e0211286e_p1`, by name) | `sudden_death_aircatchdash_bomb_fd_marth`, `sudden_death_dashhold_bomb_fd_marth` |
| FlyReflectCeil, ceiling tech (PassiveCeil) | ftCo_800C1718, ftCo_800C23A0 (port `unimplemented!`) | Audit: unreachable at Sudden Death timing; at ordinary percents under FD's underside, investigate | same |
| Damage -> MissFoot -> DamageFall / CliffCatch | ftCo_MissFoot_* | corpus:3 | `match2_fd_foxmarth` (DamageHi3 -> MissFoot) |
| DI / ASDI / SDI during hitlag | ftCo_Damage_OnEveryHitlag | n/a | (by name, Fox victim) `di_upaway_fsmash_fd_marth`, `di_downin_fsmash_fd_marth`, `sdi_fsmash_fd_marth`, `tumbledi_dolphinslash_fd_marth`; Marth as the victim **MISSING** |
| Tumble-launch camera quakes, stacked quakes | cm quake | (by name) `corpus_v3_s0_e0d368f02_p0`, `corpus_v3_s0_e46028c49_p0` | same |

## 9. Ledge

Retail: `ftCo_CliffCatch_Anim`, `ftCo_CliffWait_IASA`, `ftCo_CliffClimb/Attack/Escape/Jump*`,
`ftCliffCommon_80081370`.

| Transition | Retail | Fox | Marth |
| --- | --- | --- | --- |
| Fall / JumpB / FallAerial -> CliffCatch -> CliffWait | ftCliffCommon_80081370 | `ledge_fd_fox` +19 | `ledge_fd_marth` +12 |
| FallSpecial -> CliffCatch | same | corpus:1 | `match2_fd_foxmarth` |
| DamageFly/Fall -> CliffCatch | same | `dsmash_fd_marth` (DamageFlyLw) | corpus |
| CliffWait -> ClimbQuick / AttackQuick / EscapeQuick / JumpQuick | ftCo_CliffWait_IASA | `ledgeclimb_fd_fox`, `ledgeattack_fd_fox`, `ledgeroll_fd_fox`, `ledgejump_fd_fox` +more | `ledgeclimb_fd_marth`, `ledge_cstick_attack_fd_marth`, `ledgeescape_fd_marth`, `ledge_fd_marth` |
| C-stick ledge options and priority | ftCo_CliffWait_IASA | (by name) `ledge_cstick_{attack,escape,drop,priority}_fd_fox` | (by name) same `_marth` |
| CliffWait -> Fall (drop) | same | `ledge_cstick_drop_fd_fox` +2 | `ledge_cstick_drop_fd_marth` +2 |
| CliffWait -> DamageFall (hang timeout) | ftCo_CliffWait_Anim | `ledge_timeout_fd_fox` | `ledge_timeout_fd_marth` |
| CliffWait -> slow options (100% or more): ClimbSlow, AttackSlow, EscapeSlow, JumpSlow1/2 | ftCo_CliffWait_IASA | **MISSING** (all four) | JumpSlow `sudden_death_ledgejump_bomb_fd_marth` (300%); AttackSlow corpus:2; ClimbSlow, EscapeSlow **MISSING** |
| CliffCatch -> option before CliffWait (buffered) | ftCo_CliffCatch_IASA | corpus:1 | `match2_fd_foxmarth` (JumpQuick1) |
| Occupied ledge cannot be caught | ftCliffCommon_80081370 | (by name) `corpus_v3_s0_e6d8e8b19_p0` | same |
| Ledge intangibility against a hit | detect_eligible_hit | (by name) `sudden_death_ledgehold_bomb_fd_marth` | same |
| Revival platform -> ledge interactions | ftCo_RebirthWait | (by name) `REVIVAL_CAPTURE_LEDGE.md` witnesses | same |
| Ledge hog / trump (catching an occupied ledge knocks off) | n/a in Melee (no trump) | n/a | n/a |

## 10. Items: Sudden Death Bob-omb and held-item states

Retail: ftpickupitem_80094790 (LightGet), ftCo_80095A30 (ground throw aim),
ftCo_80095328 (air throws), ftCo_80095744 (air Z drop), ftCo_8009515C (A in
shield), ftCo_800D8A38 (dash throw), ftCo_800D7100 (aerial catch),
Fighter_8006CDA4 (drop on launch), it_8027429C (explodes in hand),
it_802703E8/it_80270E30 (fighter hits an item), it_802706D0 (item hits an
item), it_80276FC4 (wall bounce), Ground_801C0C2C (rain). `HELD_ITEM_STATES`
fails closed for anything else.

| Branch | Marth (all directed Sudden Death scenarios use Marth) | Fox |
| --- | --- | --- |
| Rain, fall, landing blast, KO | `sudden_death_bombs/idle/start_fd_marth` | shared |
| Pickup (LightGet) and hold (Wait1_1) | `sudden_death_pickup_bomb_fd_marth` +45 | corpus:2 (by trace: Fox Wait -> LightGet, GuardOff -> LightGet) |
| Ground throws F, B, Hi, Lw, F4, Hi4 | `sudden_death_throw{,b,hi,lw,f4,hi4}_bomb_fd_marth` | **MISSING** |
| C-stick smash throws B4, Hi4, Lw4 | `sudden_death_cstick{b4,hi4,lw4}_bomb_fd_marth` | **MISSING** |
| Walk, turn, turn-run, run, dash, crouch, roll, land, shield, ledge and taunt holding it | `sudden_death_{walkthrow,turnhold,turnrunhold,runhold,dashhold,crouchhold,rollhold,landhold,shieldhold,ledgehold,taunthold}_bomb_fd_marth` | **MISSING** (shared code, but Fox's hold joint and hand pose differ) |
| Special while holding | `sudden_death_specialhold_bomb_fd_marth` (Marth) | Fox special with an item: **MISSING** (`specials_keep_held_item` audited only by source) |
| Dash throw, run-shield throw, throw out of Turn | `sudden_death_dashthrow/runshieldthrow/turnthrow_bomb_fd_marth` | Dash throw corpus:1 |
| Z on the ground (forward throw); Z in the air (drop) | `sudden_death_zthrow_bomb_fd_marth`, `sudden_death_airdrop_bomb_fd_marth` | **MISSING** |
| Air throws: AirF, AirB4 | `sudden_death_airthrow_bomb_fd_marth`, `sudden_death_wallbomb*_fd_marth` | **MISSING** |
| Air throws AirB, AirHi, AirLw, AirF4, AirHi4, AirLw4 | **MISSING** | **MISSING** |
| Aerial catch LR+A (from a dash jump, from a shield jump) | (by name) `sudden_death_aircatch{dash,shield}_bomb_fd_marth` | **MISSING** |
| Catching a thrown (not falling) Bob-omb | **MISSING** | **MISSING** |
| Hit while holding: knocked loose, flies holding it, dies holding it | `sudden_death_knockloose/launchhold/kohold_bomb_fd_marth`, `sudden_death_hitholding_bomb_fd_marth` | n/a as the holder in any witness: **MISSING** |
| Grabbed or thrown while holding | `sudden_death_grabbedhold_bomb_fd_marth`, `sudden_death_thrown{b,f,hi,lw}hold_bomb_fd_marth` | **MISSING** |
| Fighter hitbox detonates a Bob-omb | `sudden_death_smash_bomb_fd_marth` (Marth smash); Fox dash attack `sudden_death_foxdash_fd_marth` hits Marth, not the bomb | Fox hitting a Bob-omb: **MISSING** |
| Item hits item (chain), walk after a soft landing, wall bounce | `sudden_death_bombchain/walkbomb/wallbomb{,slope}_fd_marth`, `sudden_death_turnrunhold_bomb_fd_marth` | shared |
| Held item leaves the hand mid-animation / with no motion | (by name) `corpus_sd_*` (9 cases) | same |
| Laser hits a Bob-omb; Reflector reflects a thrown Bob-omb; shield against a thrown Bob-omb; Counter against a Bob-omb's blast; Illusion hits a Bob-omb | `sudden_death_laserbomb_fd_marth`, `sudden_death_reflectbomb_fd_marth` (it_80273030, fixed), `sudden_death_shieldbomb_fd_marth`; Illusion **MISSING** | `sudden_death_counterbomb_fd_marth` |
| Down states while holding; tilts with an item; HeavyGet; unlit Bob-omb; a walking Bob-omb leaving the ground | Audit: fail closed, classified unreachable or needing investigation | same |
| Item hitbox against an item hitbox (it_8026FE68) | Fails closed (audit) | same |

## 11. Lasers, reflection and contact

Retail: Fox laser (`it-foxlaser`), shield/reflect paths in `melee-coll` and
`damage.rs`, ftColl_80077688 (Counter/shield volume), clank ftColl_8007925C.

| Branch | Witness |
| --- | --- |
| Laser hits a fighter; the item despawns | `laser_fd_fox` |
| Laser against a mature shield, a light shield, an aerial-drift shield | `laser_shield_fd_marth`, `laser_lightshield_fd_marth`, `laser_shield_air_fd_marth` |
| Laser grazes and is deflected by a shield | `laser_shield_deflect_fd_marth` |
| Powershield reflection: fresh, delayed, stale, return boundary | `laser_reflect_{fresh,delayed_timed,stale,return_boundary}_fd_marth` |
| Reflector returns a laser (Fx.SpecialLwHit) | `laser_reflect_return_boundary_fd_marth` |
| Reflection overflow -> shield break in the air | `laser_reflect_overflow_air_timed_fd_marth` |
| Laser against revival invincibility | `revival_laser_fd_marth_candidate` |
| Counter catches a laser at the fixed counter angle | (by name) `corpus_v3_s0_e6117d326_p2` |
| Item hit damage counts, overlay replacement | (by name) `corpus_v2_s0_e12345678_p0` |
| Aerial Reflector reflecting (AirLwHit) | **MISSING** |
| Laser hitting a captured fighter | **MISSING** |
| Laser plus melee hit on one shield in the same frame | **MISSING** (port `unimplemented!`) |
| Inert hitbox touching an item | Investigate (clank.rs `unimplemented!`) |
| Mutual clank, both priority winners, no rebound, airborne controls | `clank_jab_s74_f122_fd_foxmarth`, `clank_priority_{fox,marth}_spaced_fd_foxmarth`, `clank_smash_norebound_spaced_fd_foxmarth`, `clank_airborne_{fox,marth}_spaced_fd_foxmarth` |
| ReboundStop -> Rebound -> Wait | `clank_jab_s74_f122_fd_foxmarth` (both characters) |
| Phantom contacts beside a real hit; simultaneous hit logs | (by name) `corpus_v2_s0_e2a_p1` |
| A per-bone hurt state reaches only the bone's first capsule | (by name) `corpus_v2_s1_e49_p2` |
| Fighter hitbox against a script-invincible hurtbox (not revival) | **MISSING** (damage.rs:1662 `unimplemented!` "invincible contact"). Investigate which Fox or Marth scripts set Invincible rather than Intangible |
| Fire Fox charge and travel hits | `firefox_charge_hit_fd_marth`, `firefox_travel_hit_fd_marth` |
| Illusion hits and passes through | `illusion_fd_fox` |

## 12. Death, revival and match flow

Retail: `ftCo_DeadDown/Left/Right/UpStar/UpFall*_Anim`, `ftCo_Rebirth_Anim`,
`ftCo_RebirthWait_IASA`, `ftCo_Entry_Anim`, gm timer and Sudden Death scene,
gm_80167320 (final stock).

| Transition / branch | Fox | Marth |
| --- | --- | --- |
| Entry -> EntryStart -> EntryEnd -> Fall (match start) | `start_fd_fox`, `match_fd_foxmarth` +8 | `start_fd_marth`, `match2_fd_foxmarth` +4 |
| Fall / DamageFall / FallSpecial -> DeadDown (bottom) | `fthrow_fd_marth` +13 | `aircounter_fall_fd_marth` +10 |
| DamageFly* -> DeadLeft / DeadRight (side) | DeadRight `damage_fly_roll_t125_fd_fox_candidate` +10; DeadLeft `sudden_death_releasecaptor_bomb_fd_marth` | DeadLeft `sudden_death_throw_bomb_fd_marth` +23; DeadRight `sudden_death_dashintofox_bomb_fd_marth` |
| DamageFlyTop -> DeadUpStar (star KO) | `topko_usmash_fd_fox`, `topko_usmash_long_fd_fox` | `sudden_death_launchhold_bomb_fd_marth` +15 |
| DeadUpFall -> DeadUpFallHitCamera (screen KO) | `sudden_death_bombchain_fd_marth` | `sudden_death_pummelcaptor_bomb_fd_marth` +1; ordinary-percent screen KO (by name) `corpus_v2_s0_e80000000_p1_screenko` |
| DeadUpFallHitCameraFlat | **MISSING** (investigate the selector) | **MISSING** |
| Dead* -> Rebirth -> RebirthWait -> Fall (stick tap or timeout) | `ko_fd_marth`, `rebirth_timeout_fd_fox` +2 | `rebirth_timeout_fd_marth`, `ledge_timeout_fd_marth` +2 |
| Rebirth -> Fall directly (no wait) | `match_fd_foxmarth` | `human_smoke_fd_marth` +1 |
| RebirthWait -> EscapeAir / aerial / JumpAerialF | `rebirth_shield_a_fd_fox`, `rebirth_analog_shield_a_fd_fox`, `rebirth_held_shield_a_fd_fox`, `match_fd_foxmarth` | JumpAerialF `match2_fd_foxmarth`; aerials, air dodge and specials corpus only |
| Revival-platform floor contact | collision.rs `unimplemented!` (ftCoD5A30) | Investigate |
| Revival invincibility flash ownership | (by name) `corpus_v2_s0_e80000000_p1_screenko` | same |
| Blaster put away on death | (by name) `corpus_v2_s1_e2a_p1` | n/a |
| Stock loss, last-stock pause, GAME | `match_fd_marth_scripted`, `ko_fd_marth` (by name) | same |
| Timeout: TIME!, standings, tie -> Sudden Death, Bob-omb rain, bomb KO | `timeout_tie_fd_marth`, `sudden_death_start_fd_marth`, `melee-lib` `match_endings.rs` (cold) | same |
| Timeout with unequal stocks or percents (a decisive timeout) | **MISSING** (only the tie is recorded) | same |
| Simultaneous KO on the last stocks | `sudden_death_ledgejump_bomb_fd_marth`: both fighters die on tick 1522 (DeadDown, DeadRight) and the trace stays exact to 1545; the results screen after it is not gated | same |
| Final stock / elimination (gm_80167320) | Audit: unreachable because the scene freezes first | same |
| Swapped ports (Fox P1) | `start_fd_fox4` start boundary (not in m5_gate); directed `*_foxmarth` and `*_marthfox` capture scenarios; most other directed scenes use Marth P1 | **Thin**: swapped-port gameplay is almost entirely corpus |
| Pause during a match | Investigate: reachable by a human in versus mode; not classified in the audit | same |

---

## MISSING reachable transitions, ranked by how likely they are in play

These are the rows above with no gated retail witness, whose reachability
in the Fox–Marth FD scope is shown or plausible. The ranking weighs how
often a real match reaches the branch against how cheaply a directed
Dolphin scenario could witness it.

1. ~~Furafura exits~~: witnessed for Fox (`furafura_expire_fd_marth`,
   `furafura_hit_fd_marth`, 2026-09-27); Marth as the dizzy fighter remains.
2. **Fighter hitbox against a script-invincible hurtbox.** It is a port
   `unimplemented!` (damage.rs:1662). If any Fox or Marth startup (Dolphin
   Slash, the ledge getups, the getup attacks) is authored Invincible rather
   than Intangible, a routine out-of-shield up-B trade reaches it. Resolve the
   reachability first.
3. **Slow ledge options (100% or more): CliffClimbSlow, CliffEscapeSlow,
   CliffAttackSlow, CliffJumpSlow.** High-percent ledge play is common. Fox
   has none of the four; Marth has only the 300% ledge jump and a corpus
   AttackSlow.
4. **Weak damage states: DamageHi1 (both), DamageLw1/DamageAir1 (Fox).** Fox's
   laser and weak hits reach them every match. DamageN1 is the only
   weak-state witness.
5. **Fox holding Bob-ombs in Sudden Death.** No directed witness for Fox's
   pickup, any Fox throw, air drop, aerial catch or being hit while holding.
   Fox's hold joint and hand pose are character-specific, so Marth's
   witnesses do not transfer. Any Sudden Death Fox plays reaches this.
6. **DownDamageU (hit while lying face up), and DownDamageD in general.** It
   is common on FD: a jab or laser on a prone fighter. Face-up is missing
   for both; face-down is corpus only.
7. **Shield break in the other orientation and at the edge.** Fox's
   ShieldBreakFall/DownD/StandD, Marth's DownU/StandU, and a break that flies
   offstage.
8. **Ground blaster leaving the ground (NLoop at the edge), and Reflector
   ground rows walking off (LwLoop/LwEnd/LwTurn/LwHit -> air).** Laser and
   shine at the edge are staple Fox play. The trace shows only LwStart ->
   AirLwStart (corpus), not the corpus_v2 walk-off its comment describes.
9. **Light third-party hit on a captured fighter, and Fox-article hits
   during capture.** Judged unreachable (COVERAGE_AUDIT): a laser flies away
   from Fox faster than either fighter closes, so it meets Marth before any
   grab and cannot come back (Marth deflects, never reflects), and it cannot
   hit its owner; Bob-omb hits are not light. The other grab-pair launch
   branches are ported and witnessed (`sudden_death_releasecaptor_bomb_fd_marth`,
   `sudden_death_pummelcaptor_bomb_fd_marth`).
10. **Simultaneous shield impacts, and a phantom plus a shield impact in one
    frame.** Ported from the retail branch order (2026-09-27); unwitnessed,
    see the shield rows above.
11. **Marth as the DI/SDI victim, and Marth crouch cancel.** Every DI, SDI
    and crouch-cancel witness has Fox as the victim.
12. **Marth tech options and wall tech.** Passive and PassiveStand are
    corpus only; PassiveWall is missing. Fox's forward tech roll is corpus
    only.
13. **Ground Dancing Blade leaving the ground; the fourth aerial hit Up/Down
    (AirS4Hi/Lw); aerial full-charge Shield Breaker (AirNEnd1).**
14. **Counter phases losing support (Lw/LwHit -> air; AirLwHit ending in the
    air).**
15. **Fire Fox travel landing without a rebound (AirHi -> Hi), and travel
    into FD's side wall.**
16. **Marth CaptureDamageHi, and Fox's CaptureWaitHi -> CaptureCut in the
    air.**
17. **Item cross-interactions in Sudden Death:** a laser on a Bob-omb,
    Reflector or Counter against a thrown Bob-omb, a shield against a thrown
    Bob-omb, catching a thrown Bob-omb, the remaining six air-throw
    directions.
18. **A decisive timeout (unequal stocks or percent) and a simultaneous
    last-stock KO (draw).** They are rare, but they end matches, and only
    the tie path is recorded.
19. **Dash -> Squat; Run/RunBrake/AttackDash -> Ottotto; OttottoWait ->
    attacks (Marth).**
20. **Investigate before ranking:** the fighter-hitbox invincible contact
    (item 2), DeadUpFallHitCameraFlat, AttackS3HiS/LwS for Fox, DownReflect
    and DownSpot, a grab pair losing its floor, revival-platform floor
    contact, relaxed or C-stick jump entry, pause.

### High-frequency branches witnessed only by the corpus

These are exact but have no directed regression. A corpus change or reseed
would silently drop them, so each deserves a short directed scenario:

- Exits from a mature shield (Guard -> EscapeN/F/B, KneeBend, Catch;
  GuardOn -> the same; GuardOff -> KneeBend/EscapeN), for both characters.
  Directed witnesses exist only from the GuardReflect startup.
- Fox's back roll (EscapeB). Marth's is witnessed only while holding a
  Bob-omb.
- Fox's jump-cancel grab and jump-cancel up smash; Dash -> KneeBend,
  Dash -> CatchDash, Run -> KneeBend, Turn -> KneeBend/Catch/smash.
- Standing Illusion (Wait -> SpecialSStart), and Illusion landing mid-dash
  (AirS -> S).
- Boost grab (AttackDash -> CatchDash), for both characters.
- SquatWait interrupts (jab, jump, dash, forward smash, up smash, shield).
- DamageFall -> double jump, special or air dodge after tumble hitstun.
- Marth's getup options from DownWait, and face-down stand and back rolls.
- Dancing Blade's up branch (S1 -> S2Hi) and every aerial-to-ground
  Dancing Blade landing; Shield Breaker charge landing (AirNLoop -> NLoop).
- Captured-high throws (CaptureWaitHi -> Thrown*).
- Fire Fox and Illusion ledge catches (AirHi, HiFall, AirSEnd -> CliffCatch).

## Next steps for review

1. Settle each "Investigate" row with the decomp and retail asm. Record
   whether it is reachable, then move it into MISSING or n/a.
2. Confirm the "(by name)" rows with a field-level trace query: L-cancel
   flags, powershield frame, DI angle, C-stick source, item catch.
3. Re-run `harness/interaction_matrix.py` after each recording packet, so this matrix
   is regenerated rather than hand-maintained. Consider promoting it into
   `harness/` as a coverage report that the gate prints.
