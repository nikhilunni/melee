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
  (character, from, to) transitions and 351 (character, state) pairs
  (530, 2,810 and 369 on the 2026-09-27 evening re-run; 594, 2,913 and 398
  on the night re-run).
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
| Fox | DamageHi1, DamageLw1, DamageAir1 | Witnessed: `damage_level1_dancingblade_fd_marth` (Hi1, Lw1), `damage_air1_dancingblade_fd_marth` (Air1) |
| Fox | DownDamageU | Witnessed: `downdamage_up_fd_marth` |
| Fox | ShieldBreakFall, ShieldBreakDownD, ShieldBreakStandD | n/a on FD: ShieldBreakFly entry zeroes x velocity with a fixed launch y velocity and FD's only floor is y = 0, so the airtime is fixed; Fox lands at ShieldBreakFly anim frame 30 of 60 (`shieldbreak_fd_marth`), ShieldBreakFall only follows the Fly anim ending airborne, and ftCo_80097570 picks U/D from the HipN matrix at landing, identical each time: always DownU |
| Fox | Fx.SpecialAirLwHit | Witnessed: `sudden_death_reflecthit_walkoff_fd_marth` (LwHit sliding off the edge) and `sudden_death_airreflect_bomb_fd_fox` (AirLwLoop -> AirLwHit -> LwHit, a reflect started in the air) |
| Fox | LightThrow*, LightGet | Witnessed: every ground, C-stick, dash and air throw (`sudden_death_{zthrow,throwb,throwhi,throwlw,throwf4,cstickb4,cstickhi4,csticklw4,dashthrow}_bomb_fd_fox`, `sudden_death_airthrow{,b,hi,lw,f4,b4,hi4,lw4}_bomb_fd_fox`) and LightGet (`sudden_death_airdrop_bomb_fd_fox` +33). A in shield is n/a: GuardOff has no ftCo_8009515C throw (`sudden_death_throwhi4_bomb_fd_fox` jumps) |
| Fox | Attack13 | n/a: Fox's third jab is the rapid jab (Attack100*, witnessed) |
| Fox | AttackS3HiS, AttackS3LwS | Witnessed: `ftilt_angled_fd_fox` |
| Both | AppealSL | n/a: `enter_common_taunt` needs `left_taunt_available`, which neither character has |
| Both | DeadUpFallHitCameraFlat | n/a on FD (status, 2026-09-27) |
| Both | Pass | n/a: FD has no platforms |
| Marth | AttackS3Hi/HiS/Lw/LwS | n/a: Marth's forward tilt is not angled. `ftiltup_fd_marth` and `ftiltdown_fd_marth` enter AttackS3S |
| Marth | Attack13, Attack100* | n/a: Marth has a two-hit jab |
| Marth | CliffClimbSlow, CliffEscapeSlow | Witnessed at 300%: `sudden_death_ledge{climb,roll}_fd_marth`. CliffAttackSlow is corpus:2 and CliffJumpSlow is `sudden_death_ledgejump_bomb_fd_marth` |
| Marth | DamageHi1 | Witnessed: `damage_level1_victim_fd_marth` (Hi1 and Lw1); DamageAir1 is corpus only |
| Marth | DownDamageU | Witnessed: `downdamage_up_victim_fd_marth` (DownWaitU -> DownDamageU -> DownStandU) |
| Marth | CaptureDamageHi | Witnessed: `capture_hi_edge_pummel_victim_fd_marth` |
| Marth | PassiveWall | Witnessed: `passivewall_victim_fd_marth` (DamageFlyN -> PassiveWall -> Fall); Fox's is corpus:1 |
| Marth | ShieldBreakDownU, ShieldBreakStandU | Witnessed: `shieldbreak_hold_fd_marth`, `furafura_{expire,hit}_victim_fd_marth` (Fall -> DownU) |
| Marth | Ms.SpecialAirNEnd1 | Witnessed: `marth_airn_end1_fd_marth` |
| Marth | Ms.SpecialAirS4Hi, Ms.SpecialAirS4Lw | Witnessed: `marth_air_dancingblade4_fd_marth` (both, and their landings into S4Hi/S4Lw) |
| Marth | LightThrowAirB, AirHi, AirLw, AirF4, AirHi4, AirLw4 | Witnessed: `sudden_death_airthrow{b,hi,lw,f4,hi4,lw4}_bomb_fd_marth` (AirF and AirB4 were already) |

Several states are entered only in the corpus: Marth DamageLw3, DownWaitD,
DownStandD, DownBackD, SpecialS3Lw and AirS2Hi/AirS3Hi/AirS3Lw/AirS4S, PassiveCeil and
FlyReflectCeil; Fox MissFoot, FlyReflectWall, PassiveWall and DownDamageD.
They are exact, but no targeted scenario guards them. (Night re-run: Marth
DamageFlyHi, DamageLw2, DownStandU, DownBackU, the techs, SpecialS2Hi, S4Hi
and AirS3S, and Fox EscapeB and CaptureDamageHi now have directed witnesses.)

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
| Run -> Ottotto (edge stop) | ftCo_Run_Coll -> ftCo_Ottotto | `capture_edge_fox_*` (2) | `ottotto_marth_run_fd_marth` |
| RunBrake -> KneeBend / Squat / TurnRun / Turn | ftCo_RunBrake_IASA | KneeBend `bair_fd_fox` +9; Squat, TurnRun: corpus only | all witnessed (`bair_fd_marth`, `damage_fly_roll_dtilt_t132_fd_fox_candidate`, `match2_fd_foxmarth`) |
| RunBrake -> Ottotto | ftCo_RunBrake_Coll | corpus:4 | `ottotto_marth_runbrake_fd_marth` +2 |
| Wait/Walk -> Ottotto -> OttottoWait -> Wait | ftCo_Walk_CheckInput_Ottotto, ftCo_Ottotto_IASA | `match_fd_marth_scripted` | `match_fd_marth_scripted` |
| OttottoWait -> Catch / attack | ftCo_OttottoWait_IASA | `capture_edge_fox_*` | Catch `ottotto_marth_runbrake_fd_marth` +2; Attack11 `ottotto_marth_run_fd_marth` (Turn and KneeBend are corpus:1) |
| Running into a wall -> StopWall | ftCo_StopWall (port: `unimplemented!` in collision.rs) | n/a on FD (COVERAGE_AUDIT): the wall flag comes from the ECB side-point sweep, a grounded ECB's side points sit above y = 0, and every FD wall is at or below it | same |

## 2. Jumps, aerials, landings

Retail: `ftCo_KneeBend_IASA`, `ftCo_Jump_IASA`, `ftCo_JumpAerial_IASA`,
`ftCo_Fall_IASA`, `ftCo_AttackAir*_IASA`, `ftCo_LandingAir_EnterWithLag`,
`ftCo_Landing_IASA`, `ftCo_EscapeAir_Coll`.

| Transition / branch | Retail | Fox | Marth |
| --- | --- | --- | --- |
| KneeBend -> JumpF / JumpB | ftCo_KneeBend_Anim | `jump_fd_fox` +33 / `ledge_fd_fox` +13 | `jump_fd_marth` +31 / +11 |
| Short hop versus full hop, tap versus X/Y | ftCo_KneeBend_Check_ShortHop, ftCo_Jump_GetInput | (by name) `nairlc_*`, `jump_fd_*` | (by name) |
| Relaxed / C-stick jump entry | ftCo_Jump.c:69-98 (port `unimplemented!`) | n/a (COVERAGE_AUDIT, resolved): NTSC 1.02 stores only rumble per port (no tap-jump or jump-on-C option), and every `enter_knee_bend` caller first checks `human::jump_input` (ftCo_Jump_GetInput, 0x800CAE80) | same |
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
| FallSpecial with aerial jumps left | ftCo_FallSpecial.c:96-100 (port `unimplemented!`) | n/a: every Fox and Marth entry (air dodge, blaster, Illusion, Fire Fox, Dolphin Slash, ItemThrow, AirCatch) goes through ftCo_80096900/800968C8, which calls ftCommon_UseAllJumps in the air and ftCommon_8007D60C (jumps used = max) on the ground; ftCo_800969D8, the entry that keeps jumps, has no Fox or Marth caller | same |
| FallSpecial catch window (ftCo_800D705C) | ftCo_FallSpecial_IASA | n/a (items only) | (by name) `sudden_death_fallspecialcatch_bomb_fd_marth`: empty-handed in FallSpecial, L+A opens the window and takes a dropped Bob-omb; `corpus_sd_s1_eea202b0d_p2` keeps an item |
| JumpF/JumpAerialF/JumpAerialB -> PassiveWallJump -> Fall | ftCo_WallJump (ftCo_PassiveWall) | right: `walljump_right_underside_fd_fox_candidate`; left: `walljump_left_underside_fd_fox` (JumpAerialB), `walljump_left_vertical_fd_fox` (JumpF, no double jump) | n/a (Marth cannot wall jump) |
| JumpAerialF -> StopCeil -> Fall | ftCo_StopCeil_Coll | `stopceil_latejump266_fd_fox_candidate` | `stopceil_left_latejump270_fd_marth_candidate` |
| Repeated or mirrored wall jumps, ceiling-triggered ledge exits | ftCo_PassiveWall_IASA | Mirrored: `walljump_left_{underside,vertical}_fd_fox`; PassiveWallJump -> aerial is `walljump_aerial_fd_fox`. Repeated: not reachable in practice on FD (search evidence: none in ~4,000 dry-runs; after a wall jump Fox moves away ~1.36/tick for ~45 ticks with no drift control, JumpAerial gives only 0.83/tick, and FD's walls span only y 0 to -54.3). StopCeil -> CliffCatch: n/a, FD's only ceilings are the underside lines at y <= -54.26, ~54 below the ledges; StopCeil -> PassiveWallJump: n/a, the ceiling ends at x = 53.77 and StopCeil never coincides with wall contact | n/a |

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
| AttackS3HiS / LwS | ftCo_AttackS3 | `ftilt_angled_fd_fox` | n/a |
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
| AttackDash ends at an edge -> Ottotto | ftCo_AttackDash_Coll | corpus:1 | `ottotto_marth_attackdash_fd_marth` (Fox's push tips him over) |
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
| Ground blaster leaving the ground (NLoop -> air) | ftFx_SpecialNLoop_Coll | `laser_loop_pushoff_fd_fox`: Marth's braking body pushes the teetering Fox off the edge, NLoop -> Fall (ft_80083F88) |
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
| AirHi travel landing without a rebound (AirHi -> Hi) | ftFx_SpecialAirHi_AirToGround | Shallow floor contact without a rebound or landing (the rotation branch, every snap): `firefox_shallow_floor_fd_fox`. AirHi -> Hi itself is n/a: in the retail asm the only ChangeMotionState(SpecialHi) is ftFx_SpecialAirHi_AirToGround (800E7AF4), called only from SpecialHiHold_Anim/SpecialHiHoldAir_Anim (800E7398/800E73F8) when the charge ends grounded; travel floor contact only rebounds or rotates |
| Fire Fox travel into FD's wall | ftFx_SpecialHi_Coll | `firefox_wall_ledge_fd_fox` (pinned on the wall under the ledge, HiFall -> CliffCatch), `firefox_wall_notch_fd_fox` (pinned on the notch wall, HiFall -> FallSpecial) |
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
| LwLoop / LwEnd / LwTurn / LwHit leaving the ground | ftFx_SpecialLw{Loop,End,Turn,Hit}_GroundToAir | `reflector_loop_walkoff_fd_fox`, `reflector_end_walkoff_fd_fox`, `reflector_turn_walkoff_fd_fox`, `sudden_death_reflecthit_walkoff_fd_marth` |
| AirLwHit (aerial reflect) and AirLwHit -> LwHit landing | ftFx_SpecialAirLwHit_* | AirLwHit -> AirLwLoop after a walk-off: `sudden_death_reflecthit_walkoff_fd_marth`; a reflect started in the air and its landing: `sudden_death_airreflect_bomb_fd_fox` (a Bob-omb) |
| Reflector turnFrames inherited from an unmodelled scratch word | special_lw.rs `unimplemented!` | Ported for every predecessor whose mv+4 the port models (COVERAGE_AUDIT: Walk, Dash, Run, RunBrake, Turn, Squat, KneeBend, WallJump, the aerial and jump states); fails closed only when the Reflector starts from an unmodelled one (TurnRun's retained word, an unmodelled smash/tilt word) and a later state reads it. TurnRun itself exits only to the running jump (ftCo_TurnRun_IASA -> fn_800CAF78). No gated trace reaches it; `melee-sim search` over every grounded action (jab, tilts, smashes, grab, taunt, walk, dash, crouch, jump, shield; then a move, then the special 1-60 ticks later) entered the special 27,897 times (Reflector) and 78,477 times (Marth) with no fault: not reached in practice |
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
| AirNLoop -> NLoop (landing while charging) | ftMs_SpecialAirNLoop_Coll | `marth_special_branches_fd_marth`, `marth_airn_end1_fd_marth` |
| NStart / NLoop / NEnd0 -> air (leaving the ground) | ground collision | corpus:3 / `marth_airn_end1_fd_marth` / corpus:1 (`corpus_v2_s1_e12345678_p1`, by name) |
| AirNEnd1 (aerial full charge) | ftMs_SpecialAirNLoop_Anim | `marth_airn_end1_fd_marth` (the charge lands into NLoop, slides off into AirNLoop and completes offstage) |
| Dancing Blade: S1 -> S2Lw -> S3Hi -> S4Lw | ftMs_SpecialS_Enter, special_s::input | `dancingblade_fd_marth` |
| S2Lw -> S3S -> S4S | same | `match_fd_foxmarth` |
| S1 -> S2Hi; S2Hi -> S3*; S3Hi -> S4Hi / S4S; S3S -> S4Hi | same | corpus only (17 / 1-2 / 2 / 1 / 1) |
| S3Lw (ground) | same | corpus:2 |
| Dancing Blade ending -> Wait / Walk / KneeBend (IASA) | ftMs_SpecialS*_Anim | `match2_fd_foxmarth` +2 |
| Air: AirS1 -> AirS2Lw -> Fall | ftMs_SpecialAirS_Enter | `match2_fd_foxmarth` |
| AirS1 -> AirS2Hi; AirS2* -> AirS3*; AirS3Hi -> AirS4S | same | corpus only |
| AirS4Hi, AirS4Lw | same | `marth_air_dancingblade4_fd_marth` |
| Aerial Dancing Blade landing (AirSn -> Sn) | ftMs_SpecialAirS* collision | AirS4Hi -> S4Hi, AirS4Lw -> S4Lw `marth_air_dancingblade4_fd_marth`; the rest corpus only (AirS1 2, AirS2Hi 4, AirS2Lw 10, AirS3* 1-2) |
| Ground Dancing Blade leaving the ground (Sn -> AirSn) | ftMs_SpecialS* collision | n/a on FD: the ground rows call ft_800827A0 -> mpColl_8004B2DC with flags = 2, and mpColl_8004A45C_Floor snaps to the floor end unless a wall crosses a probe starting at the edge's y+1; FD's walls are all at y <= 0 and it has no platforms |
| Dancing Blade mv+4 inherited from an unmodelled scratch word | special_s.rs `unimplemented!` | Same guard as the Reflector row: ported for the modelled predecessors, fail-closed otherwise; no gated trace reaches it; `melee-sim search` over every grounded action (jab, tilts, smashes, grab, taunt, walk, dash, crouch, jump, shield; then a move, then the special 1-60 ticks later) entered the special 27,897 times (Reflector) and 78,477 times (Marth) with no fault: not reached in practice |
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
| AirLwHit ending in the air -> Fall | ftMs_SpecialAirLwHit_Anim | `counter_air_fall_fd_marth` |
| Counter catching an item (laser, Illusion ghost, Fire Fox) | ftColl_80077688 | (by name) `corpus_v3_s0_e6117d326_p2` (laser), `corpus_v2_s0_e1_p1` (ghost), `corpus_v3_s0_e1cda1301_p0` (Fire Fox, stale powershield branch) |
| Counter volume dropped on a motion change (grabbed or hit out of the stance) | fighter.c:1049 | (by name) `corpus_v3_s1_e9943b4ab_p0` |
| Counter catch without the minimum hitlag after landing | shield_unk0 | (by name) `corpus_v3_s0_e5f386e5e_p0` |
| Lw / LwHit leaving the ground | ftMs_SpecialLw_Coll | n/a: a grounded Counter stops at the ledge (status, 2026-09-27) |

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
| Powershield against a melee hit (Fox shields Marth / Marth shields Fox) | ftColl_80076CBC | (by name) `powershield_ftilt_fd_marth` | (by name) `powershield_ftilt_victim_fd_marth` (GuardReflect -> GuardSetOff on Fox's forward tilt) |
| Shield SDI/ASDI during shield hitlag | ftCo_GuardSetOff | (by name) `corpus_v2_s0_e1_p0` | same |
| Shield pushed off the edge -> MissFoot | ftCo_Guard*_Coll | corpus:3 (GuardOn, GuardOff, GuardSetOff) | corpus:1 |
| Guard -> ShieldBreakFly -> DownU -> StandU -> Furafura | ftCo_ShieldBreakFly_Anim | `shieldbreak_fd_marth` | `laser_reflect_overflow_air_timed_fd_marth` (Fall/DownD/StandD path) |
| Break in the other orientation (Fox D, Marth U) | ftCo_ShieldBreakDown_Anim | n/a on FD: the fixed vertical flight always lands Fox into DownU (see the state table) | `shieldbreak_hold_fd_marth` +2 (Fall -> DownU -> StandU) |
| Furafura -> Wait (dizzy wears off); Furafura -> damage | ftCo_Furafura_Anim / hit | `furafura_expire_fd_marth`, `furafura_hit_fd_marth` | `furafura_expire_victim_fd_marth`, `furafura_hit_victim_fd_marth` |
| Shield break at the edge or offstage (fly into the blast zone) | ftCo_ShieldBreakFly_Coll | n/a on FD: the flight is vertical (x velocity 0, knockback 0, no drift in `shield_break.rs` physics) and fighter push applies only on the ground, so the fighter lands where it left, on stage | same |
| Simultaneous shield impacts (two hitboxes in one frame) | ftColl_80076CBC | Ported (the strongest impact wins, getEnvDmg rounding); unwitnessed on Fox's shield: every Fox/Marth hitbox is group 0 and Marth has no projectile, so it needs a Bob-omb blast and Marth's hit together | `sudden_death_shieldlaserbomb_fd_marth`: a laser and a Bob-omb blast on tick 1232, the blast's pushback wins. Laser plus melee: n/a (see section 11) |
| Phantom contact and shield impact in the same frame | fighter.c:2907 | Ported (the phantom branch drops the impact's response); unwitnessed: the phantom band is 0.01 of overlap | same |
| Dash -> EscapeF (dash defense) | ftCo_Dash_IASA | `dash_escape_fd_fox` | `dash_escape_fd_marth` |
| EscapeF/B/N -> Wait / GuardOn; IASA actions | ftCo_Escape_IASA | `roll_fd_fox`, `spotdodge_fd_fox`; the rest corpus | `roll_fd_marth`, `spotdodge_fd_marth`; the rest corpus |
| Roll with an item: A is a smash throw | ftCo_8009563C | `sudden_death_rollthrow_bomb_fd_fox` (EscapeF -> LightThrowF4) | `sudden_death_rollthrow_bomb_fd_marth` (EscapeB -> LightThrowB4; the stick at A never changes it) |

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
| CapturePulledHi -> CaptureWaitHi | same | `capture_hi_edge_cut_fall_fd_fox` +2 | `capture_edge_fox_*` |
| CaptureWaitLw -> CaptureDamageLw -> CaptureWaitLw (pummel) | ftCo_CaptureDamageLw_Anim | `match_fd_foxmarth` +2 | `match2_fd_foxmarth` |
| CaptureWaitHi -> CaptureDamageHi | ftCo_CaptureDamageHi_Anim | `capture_hi_edge_pummel_fd_marth` | `capture_hi_edge_pummel_victim_fd_marth` |
| CaptureWaitLw -> CaptureCut -> Wait | ftCo_CaptureCut_Enter | `grabmash_fd_marth`, `pummel_fd_marth` +1 | `grab_airborne_fd_foxmarth` +1 |
| CaptureWaitHi -> CaptureCut -> Fall (air Cut) | same | `capture_hi_edge_cut_fall_fd_fox` (Cut -> Fall); `capture_hi_edge_pummel_fd_marth` (the air Cut lands into Wait) | `capture_edge_fox_outward_stop43_jump109_grab107_candidate`, `capture_hi_edge_pummel_victim_fd_marth` |
| CaptureWait -> CaptureJump -> Landing / Fall | ftCo_CaptureJump_Anim | `capture_jump_{up_release,xy_latch}_fd_marthfox_candidate` | `capture_jump_*_fd_foxmarth_candidate`, `capture_edge_fox_air_up_release_candidate` |
| CaptureWaitHi -> Thrown* | ftCo_Thrown*_Anim | ThrownB `capture_hi_edge_throwb_fd_marth`; the rest corpus:3 | corpus:3 |
| ThrownF/B/Hi/Lw -> DamageAir/DamageFly | ftCo_Thrown*_Anim | `cstick_throw_*_fd_marth` | `cstick_throw_*_fd_fox` |
| Thrown into the wall -> PassiveWall | ftCo_PassiveWall | (by name) `corpus_v3_s1_edb4b01fd_p0`; the trace attributes the PassiveWall to Fox, while the comment says Marth | `passivewall_thrownhi_victim_fd_marth` (ThrownHi -> DamageFlyTop -> PassiveWall); also out of an aerial hit, `passivewall_victim_fd_marth`. Fox's column is right by trace (`corpus_v3_s1_edb4b01fd_p0`: p1 ThrownLw -> PassiveWall) |
| Meteor cancel of a down throw off the ledge | ftCo_JumpAerial_CheckInput | n/a | (by name) `corpus_v3_s1_e8be4d273_p1` |
| Thrown positioning waits out hitlag | Fighter_CallAcessoryCallbacks_8006C624 | (by name) corpus v3 | same |
| Throw damage deferred to ProcessHit (no roll at release) | x1838 | (by name) `corpus_v3_s0_e8be4d273_p0` | same |
| Dying grabber releases the victim | ftCo_800DD100 | (by name) corpus v3 | same |
| Pair launched by a Bob-omb: both, captor only, victim only, pummel frame | ftCo_8008EC90, ftCo_800DCFD4, ftCo_800DE2F0, ftCo_800DE854 | `sudden_death_grabbomb_fd_marth`, `sudden_death_grabbombcaptor_fd_marth`, `sudden_death_releasecaptor_bomb_fd_marth`, `sudden_death_pummelcaptor_bomb_fd_marth`, `sudden_death_grabbedhold_bomb_fd_marth` | same |
| Light third-party hit on the captured member | ftCo_8008EC90 (grab_damage.rs `unimplemented!`) | n/a (COVERAGE_AUDIT): the only third-party hitboxes are items; a Bob-omb hits for 25, not under PlCo +3C0's 6, and a captor's laser flies away from its victim | same |
| Only the victim launched (non-item) / armoured members | ftCo_800DE2F0 | Victim only: `sudden_death_releasecaptor_bomb_fd_marth`; non-item and armoured: n/a (COVERAGE_AUDIT: no third-party fighter hitboxes in a 1v1; armoured members fail closed) | same |
| Captured damage outside low capture or throw | grab_escape.rs `unimplemented!` | n/a (COVERAGE_AUDIT): it needs a captor hit outside the ported pummel/throw states or a light third-party hit | same |
| Grab pair losing its floor | ftCo_800DC920 (collision.rs `unimplemented!`) | n/a on FD (status, 2026-09-27) | same |
| Fox-article hits during capture | fighter.c capture branch (damage.rs `unimplemented!`) | n/a (COVERAGE_AUDIT): a laser flies away from the victim Fox holds and meets Marth before any grab | same |

## 8. Damage, tumble, down, tech

Retail: `ftCo_Damage_IASA`, `ftCo_Damage_CheckAirMotion`, `ftCo_DamageFall_IASA`,
`ftCo_DamageFly_Coll`, `ftCo_DamageFlyRoll_*`, `ftCo_Passive_*`, `ftCo_PassiveStand_*`,
`ftCo_DownBound_Anim`, `ftCo_Down_CheckInput`, `ftCo_DownDamage_*`, `ftCo_FlyReflect_Coll`,
`ftCo_MissFoot_*`.

| Transition / branch | Retail | Fox | Marth |
| --- | --- | --- | --- |
| Ground hit -> DamageN1/N2/N3, Hi2/Hi3, Lw2/Lw3 -> Wait / Landing | ftCo_Damage_Anim | N1 `jab_fd_fox`; N2 `ftilt_fd_fox` +10; N3 `illusion_fd_fox` +5; Hi2 `nair_fd_fox` +5; Hi3 `fsmashcharge_fd_fox` +3; Lw2 `dair_fd_fox` +4; Lw3 `dtilt_fd_fox` +2 | N1 `sudden_death_releasecaptor_bomb_fd_marth` +1; N2 `firefox_charge_hit_fd_marth` +1; N3 `match_fd_foxmarth`; Hi2 `match_fd_foxmarth`; Hi3 `match2_fd_foxmarth`; Lw2/Lw3 corpus only |
| DamageHi1 / DamageLw1 / DamageAir1 | same, weak-hit selection | `damage_level1_dancingblade_fd_marth` (Hi1, Lw1), `damage_air1_dancingblade_fd_marth` (Air1) | Hi1, Lw1 `damage_level1_victim_fd_marth`; Air1 corpus:1 |
| Crouch cancel (SquatWait -> DamageHi2) | ftCo_Damage.c:124-127 | `cc_ftilt_fd_marth` | `crouchcancel_victim_fd_marth` (SquatWait -> DamageN1) |
| Air hit -> DamageAir2/3 -> Fall / Landing | ftCo_Damage_CheckAirMotion | `clank_airborne_fox_spaced_fd_foxmarth`, `cstick_throw_forward_fd_marth` +1 | `firefox_charge_hit_fd_marth` +1, `cstick_throw_forward_fd_fox` +1 |
| Hitstun exit: DamageAir3 -> aerial / EscapeAir; unbuffered attack | ftCo_Damage_IASA | (by name) `hitstun_exit_{nair,fair}_fd_fox`, `hitstun_shield_priority_fd_fox` | `hitstun_exit_nair_fd_marth`, `hitstun_exit_fair_fd_marth`, `hitstun_shield_priority_fd_marth`, `hitstun_unbuffered_attack_fd_marth` |
| DamageFlyTop -> Landing on the hitstun-expiry tick | ftCo_DamageFly_Coll | `hitstun_exit_fair_fd_fox` +2 | corpus:2 |
| DamageFall / DamageFly -> JumpAerialF / special / air dodge after hitstun | ftCo_DamageFall_IASA | `match_fd_foxmarth` (DamageFlyN -> JumpAerialF); DamageFall -> anything: corpus only | DamageFlyTop -> Ms.SpecialAirNStart `match_fd_foxmarth`; the rest corpus |
| Tumble exit by a stick flick -> Fall | ftCo_DamageFall_IASA | corpus:1 | `match2_fd_foxmarth` |
| Airborne transition in ftCo_Damage_IASA outside {Attack, Jump, Escape, AirSpecial} | damage.rs `unimplemented!` | n/a: `fall::iasa` returns only those four or None, so the arm is dead; the Sudden Death air item throw and catch (ftCo_80095328, ftCo_800D7100) now run before it (44a3e56), unwitnessed from hitstun | same |
| DamageFly* -> DamageFall | ftCo_DamageFly_Anim | `di_upaway_fsmash_fd_marth` +4 | `cstick_throw_back_fd_fox` +3 |
| DamageFlyN / Hi / Lw / Top | knockback-angle selection | all four directed | Hi corpus:8; N, Lw, Top directed |
| DamageFlyRoll -> DeadRight / DamageFall / DownBoundU | ftCo_DamageFlyRoll_Coll | `damage_fly_roll_t125/dtilt_t132/crouch_fd_fox_candidate` | Sudden Death (300%): `sudden_death_airdodgehold_bomb_fd_marth`, `sudden_death_dashhold_bomb_fd_marth`; ordinary percent (100%): `damage_fly_roll_victim_fd_marth` (Wait -> DamageFlyRoll) |
| DamageFlyTop ends keeping fast fall | ftCo_80090780 | (by name) `corpus_v3_s1_e46703f61_p0` | same |
| Tumble -> DownBoundU/D (missed tech) | ftCo_DownBound_Anim | `di_downin_fsmash_fd_marth`, `cstick_throw_down_fd_marth` +10 | `cstick_throw_up_fd_fox` +1 |
| DownBound -> DownWait -> getup (stand, roll F/B, attack) | ftCo_Down_CheckInput | face up: `getupstand_fd_fox`, `getuproll_fd_fox`, `getupattack_fd_fox`; face down: `match2_fd_foxmarth`, `match_fd_foxmarth` (attack, forward, back, stand) | DownWaitU -> getups corpus only; DownBound -> DownAttack/DownFoward directed (`match2_fd_foxmarth`, `match_fd_foxmarth`); DownStand*/DownBack* corpus only |
| DownWait -> DownBack (face up) | same | `match_fd_foxmarth` | corpus:2 |
| DownBound -> Fall (bounce off the edge) | ftCo_DownBound_Coll | `match2_fd_foxmarth` +1 | corpus:1 |
| DownWait hit -> DownDamageU/D | ftCo_DownDamage_* | U `downdamage_up_fd_marth`; D corpus:1 (by name `corpus_v2_s0_effffffff_p2`, `corpus_v3_s0_e0fe4dd03_p1`) | U `downdamage_up_victim_fd_marth`; D corpus:4 |
| DownDamage keeping facing / hitstun air physics | ftCo_8008DCE0 | (by name) `corpus_v2_s0_e49_p2`, `corpus_v3_s0_e0fe4dd03_p1` | same |
| DownReflect wall bounce, DownDamage wall tech/bounce | ftCo_800C7CA0, ftCo_800C1D38 (port `unimplemented!` for DownReflect) | DownReflect: n/a on FD (COVERAGE_AUDIT: the wall-hug flag comes only from the ECB side-point sweep, a grounded ECB's side points sit above y = 0 and every FD wall is at or below it). Airborne DownDamage into a wall: tech `passivewall_downdamage_victim_fd_marth`. The bounce (ftCo_800C17CC) is not reachable in practice on FD: it needs |kb.x| > PlCo +1B0 (1.0) toward a hugged wall, but DownDamage's sub-7% hits at 155% still gave |kb.x| <= 0.71, and FD's walls run down and inward from the ledge, so a hit from the stage side pushes the victim away from them. `melee-sim search` found no bounce in 652k candidates (Fox aerials, jabs and tilts on a prone Marth at the ledge, with DI); the bounce helper itself is witnessed from tumbling (FlyReflectWall) | same |
| DownSpot | ftCo_DownSpot_Enter | n/a on FD (status, 2026-09-27) | same |
| Tech in place (Passive) | ftCo_Passive_* | `match2_fd_foxmarth` | `tech_inplace_victim_fd_marth` |
| Tech roll (PassiveStandB / F) | ftCo_PassiveStand_* | B `tech_fd_marth`; F corpus:6 | `tech_rollb_victim_fd_marth`, `tech_rollf_victim_fd_marth` |
| Wall tech (PassiveWall) | ftCo_PassiveWall_* | corpus:1 | `passivewall_victim_fd_marth` (L at 334; L at 330 bounces instead) |
| Tumble into a wall -> FlyReflectWall | ftCo_FlyReflect_Coll | corpus:1 (`corpus_v3_s0_e0211286e_p1`, by name) | `sudden_death_aircatchdash_bomb_fd_marth`, `sudden_death_dashhold_bomb_fd_marth` |
| FlyReflectCeil, ceiling tech (PassiveCeil) | ftCo_800C1718, ftCo_800C23A0 | Ported (COVERAGE_AUDIT); Fox unwitnessed | corpus:1 each (`corpus_v3_s0_ef4efb740_p1`, `corpus_v3_s0_ef4efb740_p1_ceiltech`, by name) |
| Damage -> MissFoot -> DamageFall / CliffCatch | ftCo_MissFoot_* | corpus:3 | `match2_fd_foxmarth` (DamageHi3 -> MissFoot) |
| DI / ASDI / SDI during hitlag | ftCo_Damage_OnEveryHitlag | n/a | (by name, Fox victim) `di_upaway_fsmash_fd_marth`, `di_downin_fsmash_fd_marth`, `sdi_fsmash_fd_marth`, `tumbledi_dolphinslash_fd_marth`; Marth as the victim (by name) `di_sdi_upin_fsmash_victim_fd_marth` (SDI up and up-in in hitlag, then up-in DI) |
| Tumble-launch camera quakes, stacked quakes | cm quake | (by name) `corpus_v3_s0_e0d368f02_p0`, `corpus_v3_s0_e46028c49_p0` | same |

## 9. Ledge

Retail: `ftCo_CliffCatch_Anim`, `ftCo_CliffWait_IASA`, `ftCo_CliffClimb/Attack/Escape/Jump*`,
`ftCliffCommon_80081370`.

| Transition | Retail | Fox | Marth |
| --- | --- | --- | --- |
| Fall / JumpB / FallAerial -> CliffCatch -> CliffWait | ftCliffCommon_80081370 | `ledge_fd_fox` +19 | `ledge_fd_marth` +12 |
| FallSpecial -> CliffCatch | same | corpus:1 | `match2_fd_foxmarth` |
| DamageFly/Fall -> CliffCatch | same | `dsmash_fd_marth` (DamageFlyLw) | `di_sdi_upin_fsmash_victim_fd_marth` (DamageFall); DamageFlyLw corpus |
| CliffWait -> ClimbQuick / AttackQuick / EscapeQuick / JumpQuick | ftCo_CliffWait_IASA | `ledgeclimb_fd_fox`, `ledgeattack_fd_fox`, `ledgeroll_fd_fox`, `ledgejump_fd_fox` +more | `ledgeclimb_fd_marth`, `ledge_cstick_attack_fd_marth`, `ledgeescape_fd_marth`, `ledge_fd_marth` |
| C-stick ledge options and priority | ftCo_CliffWait_IASA | (by name) `ledge_cstick_{attack,escape,drop,priority}_fd_fox` | (by name) same `_marth` |
| CliffWait -> Fall (drop) | same | `ledge_cstick_drop_fd_fox` +2 | `ledge_cstick_drop_fd_marth` +2 |
| CliffWait -> DamageFall (hang timeout) | ftCo_CliffWait_Anim | `ledge_timeout_fd_fox` | `ledge_timeout_fd_marth` |
| CliffWait -> slow options (100% or more): ClimbSlow, AttackSlow, EscapeSlow, JumpSlow1/2 | ftCo_CliffWait_IASA | all four at 300%: `sudden_death_ledge{climb,attack,roll,jump}_fd_fox` | JumpSlow `sudden_death_ledgejump_bomb_fd_marth`, ClimbSlow `sudden_death_ledgeclimb_fd_marth`, EscapeSlow `sudden_death_ledgeroll_fd_marth` (all 300%); AttackSlow corpus:2 |
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

| Branch | Marth (every directed Sudden Death scenario starts from Marth's savestate) | Fox |
| --- | --- | --- |
| Rain, fall, landing blast, KO | `sudden_death_bombs/idle/start_fd_marth` | shared |
| Pickup (LightGet) and hold (Wait1_1) | `sudden_death_pickup_bomb_fd_marth` +45 | `sudden_death_airdrop_bomb_fd_fox` +33 (Wait -> LightGet); GuardOff/Walk -> LightGet corpus |
| Ground throws F, B, Hi, Lw, F4, Hi4 | `sudden_death_throw{,b,hi,lw,f4,hi4}_bomb_fd_marth` | F `sudden_death_zthrow_bomb_fd_fox`, `sudden_death_foxcatchthrow_bomb_fd_marth` (out of a landing); B, Hi, Lw `sudden_death_throw{b,hi,lw}_bomb_fd_fox` (tilt throws), Lw from a crouch `sudden_death_squatthrow_bomb_fd_fox`; F4 `sudden_death_throwf4_bomb_fd_fox`, `sudden_death_rollthrow_bomb_fd_fox`; Hi4 `sudden_death_cstickhi4_bomb_fd_fox` (A+up in GuardOff is a jump: `sudden_death_throwhi4_bomb_fd_fox`) |
| C-stick smash throws B4, Hi4, Lw4 | `sudden_death_cstick{b4,hi4,lw4}_bomb_fd_marth` | `sudden_death_cstick{b4,hi4,lw4}_bomb_fd_fox` |
| Walk, turn, turn-run, run, dash, crouch, roll, land, shield, ledge and taunt holding it | `sudden_death_{walkthrow,turnhold,turnrunhold,runhold,dashhold,crouchhold,rollhold,landhold,shieldhold,ledgehold,taunthold}_bomb_fd_marth` | Walk (every `*_bomb_fd_fox` pickup), turn (`sudden_death_csticklw4_bomb_fd_fox`), turn-run (`sudden_death_turnrunhold_bomb_fd_fox`), run and Ottotto (`sudden_death_ottottohold_bomb_fd_fox`), dash (`sudden_death_dashthrow_bomb_fd_fox`), crouch (`sudden_death_squatthrow_bomb_fd_fox`), roll (`sudden_death_rollthrow_bomb_fd_fox`), land (`sudden_death_landhold_bomb_fd_fox`), shield (`sudden_death_throwhi4_bomb_fd_fox`), taunt (`sudden_death_taunthold_bomb_fd_fox`). Ledge: not reachable in practice (the held Bob-omb's fuse ends 58 ticks after Fox's pickup; running off the edge faces away from the ledge and a turned backward jump cannot come back down in time: `melee-sim search`, 5,118 candidates, none caught the ledge); the shared ledge states hold the item for Marth (`sudden_death_ledgehold_bomb_fd_marth`) |
| Special while holding | `sudden_death_specialhold_bomb_fd_marth` (Marth) | Reflector: `sudden_death_reflectorhold_bomb_fd_fox`. Blaster: `sudden_death_specialhold_bomb_fd_fox` (the script's held-item hide releases the grip, opcode 35). Illusion: `sudden_death_illusionhold_bomb_fd_fox`; Fire Fox: `sudden_death_firefoxhold_bomb_fd_fox` |
| Dash throw, run-shield throw, throw out of Turn | `sudden_death_dashthrow/runshieldthrow/turnthrow_bomb_fd_marth` | Dash throw `sudden_death_dashthrow_bomb_fd_fox`; run-shield `sudden_death_runshieldthrow_bomb_fd_fox`, Turn `sudden_death_turnthrow_bomb_fd_fox` |
| Z on the ground (forward throw); Z in the air (drop) | `sudden_death_zthrow_bomb_fd_marth`, `sudden_death_airdrop_bomb_fd_marth` | `sudden_death_zthrow_bomb_fd_fox`, `sudden_death_airdrop_bomb_fd_fox` |
| Air throws: AirF, AirB4 | `sudden_death_airthrow_bomb_fd_marth`, `sudden_death_wallbomb*_fd_marth` | `sudden_death_airthrow_bomb_fd_fox`, `sudden_death_airthrowb4_bomb_fd_fox` |
| Air throws AirB, AirHi, AirLw, AirF4, AirHi4, AirLw4 | `sudden_death_airthrow{b,hi,lw,f4,hi4,lw4}_bomb_fd_marth` | `sudden_death_airthrow{b,hi,lw,f4,hi4,lw4}_bomb_fd_fox` (AirF/B/Hi/F4/B4/Hi4 also land into the ground throw) |
| Aerial catch LR+A (from a dash jump, from a shield jump) | (by name) `sudden_death_aircatch{dash,shield}_bomb_fd_marth` | (by name) `sudden_death_foxcatchthrow_bomb_fd_marth` (a falling Bob-omb) |
| Catching a thrown (not falling) Bob-omb | `sudden_death_catchthrown_bomb_fd_marth` (A at 1253-1255 as Fox's forward smash throw arrives) | `sudden_death_catchthrown_bomb_fd_fox` (A as Marth's forward smash throw arrives: LightGet) |
| Hit while holding: knocked loose, flies holding it, dies holding it | `sudden_death_knockloose/launchhold/kohold_bomb_fd_marth`, `sudden_death_hitholding_bomb_fd_marth` | Knocked loose `sudden_death_knockloose_bomb_fd_fox`; launched holding it (DownBoundD) `sudden_death_hitholding_bomb_fd_fox`; dying holding it `sudden_death_kohold_bomb_fd_fox` (star KO at 1289; the death destroys the held item) |
| Grabbed or thrown while holding | `sudden_death_grabbedhold_bomb_fd_marth`, `sudden_death_thrown{b,f,hi,lw}hold_bomb_fd_marth` | `sudden_death_grabbedhold_bomb_fd_fox`, `sudden_death_thrown{b,f,hi,lw}hold_bomb_fd_fox` |
| Fighter hitbox detonates a Bob-omb | `sudden_death_smash_bomb_fd_marth` (Marth smash); Fox dash attack `sudden_death_foxdash_fd_marth` hits Marth, not the bomb | (by name) `sudden_death_smashbomb_fd_fox` (Fox forward-smashes a thrown Bob-omb) |
| Item hits item (chain), walk after a soft landing, wall bounce | `sudden_death_bombchain/walkbomb/wallbomb{,slope}_fd_marth`, `sudden_death_turnrunhold_bomb_fd_marth` | shared |
| Held item leaves the hand mid-animation / with no motion | (by name) `corpus_sd_*` (9 cases) | same |
| Laser hits a Bob-omb; Reflector reflects a thrown Bob-omb; shield against a thrown Bob-omb; Counter against a Bob-omb's blast; Illusion hits a Bob-omb | `sudden_death_laserbomb_fd_marth`, `sudden_death_reflectbomb_fd_marth` (it_80273030, fixed), `sudden_death_shieldbomb_fd_marth`, `sudden_death_illusionbomb_fd_marth` | `sudden_death_counterbomb_fd_marth` |
| Down states while holding; tilts with an item; HeavyGet; unlit Bob-omb; a walking Bob-omb leaving the ground | HeavyGet, an unlit Bob-omb and a walking Bob-omb leaving the ground: audit, fail closed (unreachable or unwitnessed) | Down states: `sudden_death_hitholding_bomb_fd_fox` (4f7beb2); a tilt with a throwable item throws it (3a78890): `sudden_death_throw{b,hi,lw}_bomb_fd_fox`; the rest as Marth |
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
| Aerial Reflector reflecting (AirLwHit) | `sudden_death_airreflect_bomb_fd_fox` (a reflect started in the air, a thrown Bob-omb, landing into LwHit); AirLwHit after a grounded reflect walks off: `sudden_death_reflecthit_walkoff_fd_marth`. A laser (returned by Marth's powershield) reflected in the air is not directed |
| Laser hitting a captured fighter | n/a (COVERAGE_AUDIT): Fox's laser cannot hit its owner and flies away from the victim he holds; it meets Marth before any grab |
| Laser plus melee hit on one shield in the same frame | n/a: a laser travels ~6.6 units/tick and Fox has no hitbox for more than 15 ticks after any laser spawn; two lasers cannot coincide either. Two impacts on one shield are witnessed with a laser plus a Bob-omb blast: `sudden_death_shieldlaserbomb_fd_marth` (tick 1232, the blast's pushback wins) |
| Inert hitbox touching an item | n/a (COVERAGE_AUDIT): no element 11 hitbox in any Fox or Marth script, article or the Bob-omb |
| Mutual clank, both priority winners, no rebound, airborne controls | `clank_jab_s74_f122_fd_foxmarth`, `clank_priority_{fox,marth}_spaced_fd_foxmarth`, `clank_smash_norebound_spaced_fd_foxmarth`, `clank_airborne_{fox,marth}_spaced_fd_foxmarth` |
| ReboundStop -> Rebound -> Wait | `clank_jab_s74_f122_fd_foxmarth` (both characters) |
| Phantom contacts beside a real hit; simultaneous hit logs | (by name) `corpus_v2_s0_e2a_p1` |
| A per-bone hurt state reaches only the bone's first capsule | (by name) `corpus_v2_s1_e49_p2` |
| Fighter hitbox against a script-invincible hurtbox (not revival) | n/a (status, 2026-09-27): Fox and Marth author only Intangible; the reachable case, the thrower's 8 frames at throw start, is ported. damage.rs `unimplemented!` "invincible contact" stays fail-closed |
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
| DeadUpFallHitCameraFlat | n/a on FD (status, 2026-09-27) | same |
| Dead* -> Rebirth -> RebirthWait -> Fall (stick tap or timeout) | `ko_fd_marth`, `rebirth_timeout_fd_fox` +2 | `rebirth_timeout_fd_marth`, `ledge_timeout_fd_marth` +2 |
| Rebirth -> Fall directly (no wait) | `match_fd_foxmarth` | `human_smoke_fd_marth` +1 |
| RebirthWait -> EscapeAir / aerial / JumpAerialF | `rebirth_shield_a_fd_fox`, `rebirth_analog_shield_a_fd_fox`, `rebirth_held_shield_a_fd_fox`, `match_fd_foxmarth` | JumpAerialF `match2_fd_foxmarth`; aerials, air dodge and specials corpus only |
| Revival-platform floor contact | collision.rs `unimplemented!` (ftCoD5A30) | n/a on FD (status, 2026-09-27) |
| Revival invincibility flash ownership | (by name) `corpus_v2_s0_e80000000_p1_screenko` | same |
| Blaster put away on death | (by name) `corpus_v2_s1_e2a_p1` | n/a |
| Stock loss, last-stock pause, GAME | `match_fd_marth_scripted`, `ko_fd_marth` (by name) | same |
| Timeout: TIME!, standings, tie -> Sudden Death, Bob-omb rain, bomb KO | `timeout_tie_fd_marth`, `sudden_death_start_fd_marth`, `melee-lib` `match_endings.rs` (cold) | same |
| Timeout with unequal stocks or percents (a decisive timeout) | `timeout_decisive_fd_marth` (unequal stocks) | same |
| Simultaneous KO on the last stocks | `sudden_death_ledgejump_bomb_fd_marth`: both fighters die on tick 1522 (DeadDown, DeadRight) and the trace stays exact to 1545; the results screen after it is not gated | same |
| Final stock / elimination (gm_80167320) | Audit: unreachable because the scene freezes first | same |
| Swapped ports (Fox P1) | `start_fd_fox4` start boundary (not in m5_gate); directed `*_foxmarth` and `*_marthfox` capture scenarios; most other directed scenes use Marth P1 | **Thin**: swapped-port gameplay is almost entirely corpus |
| Pause during a match | Out of scope: the port does not model gm_DoPauseChecksAndRoutine (8016CA68) / gm_DefaultVSGetPauser (8016BC74); a Start press in a dry run leaves every key byte-identical. Tick-clock scenarios never pause; porting pause is a separate task (TRACKER) | same |

---

## MISSING reachable transitions, ranked by how likely they are in play

**Status, 2026-09-27 (evening).** Witnessed since the draft: Fox's four slow
ledge options at 300% (`sudden_death_ledge{climb,roll,attack,jump}_fd_fox`),
Furafura's exits, DownDamageU (`downdamage_up_fd_marth`), the Reflector's
loop walking off (`reflector_loop_walkoff_fd_fox`), Marth crouch-cancelling
(`crouchcancel_victim_fd_marth`), Marth's techs (`tech_{inplace,rollb,rollf}_victim_fd_marth`)
and getups (`getup_{stand,rollf,rollb,attack}_fd_marth`), Dancing Blade's up
branch, an aerial Dancing Blade and Shield Breaker charge landing
(`marth_special_branches_fd_marth`), the Bob-omb interactions (laser,
Reflector, shield, Counter, forward smash, Illusion, Fox's aerial catch and
throw), mature-shield exits, jump-cancelled grab and up smash, dash grab,
boost grab and crouch interrupts (`shield_exits_fd_marth`,
`tech_interrupts_fd_marth`, `squat_interrupts_fd_marth`), and a same-tick
final-stock KO pair. Resolved as unreachable on FD: script Invincible
hurtboxes (Fox and Marth author only Intangible; the reachable case, the
thrower's 8 frames at throw start, is now ported), DeadUpFallHitCameraFlat,
DownSpot, a grab pair losing its floor, revival-platform floor contact, and
a grounded Counter losing support (it stops at the ledge). Later the
same day: Fox's level-1 reactions (`damage_level1_dancingblade_fd_marth`,
`damage_air1_dancingblade_fd_marth`: N1, Hi1, Lw1, Air1), Fox's angled
forward tilts (`ftilt_angled_fd_fox`), DamageFall exits
(`damagefall_{jump,upb}_fd_fox`), captured-high throw and pummel/CaptureCut
(`capture_hi_edge_{throwb,pummel}_fd_marth`), the Reflector's Turn, End and
Hit rows leaving the ground (`reflector_{turn,end}_walkoff_fd_fox`,
`sudden_death_reflecthit_walkoff_fd_marth`), a decisive timeout
(`timeout_decisive_fd_marth`) and a shield broken by decay inside the Guard
proc (`shieldbreak_hold_fd_marth`; the burst queues behind ShieldBreakFly's
script graphics). Marth as the victim is witnessed too: dizzy
until it wears off and hit while dizzy (`furafura_{expire,hit}_victim_fd_marth`),
and DamageHi1/Lw1 from Fox's jab 1 on his head capsule while crouched and his
legs while taunting (`damage_level1_victim_fd_marth`). Still open from that batch: only Fox's
other shield-break orientation (research: it needs a different landing
height; not reachable on FD). The rest of the list below is still open.

**Night, 2026-09-27.** 64 more directed scenarios (commits `3a3a432`,
`ab68079`; 594 gated scenarios on the re-run) witness: Marth's Ottotto
entries and teeter options (`ottotto_marth_{run,runbrake,attackdash}_fd_marth`),
slow ledge climb and roll (`sudden_death_ledge{climb,roll}_fd_marth`),
AirNEnd1, AirS4Hi/Lw and AirLwHit -> Fall (`marth_airn_end1_fd_marth`,
`marth_air_dancingblade4_fd_marth`, `counter_air_fall_fd_marth`), Marth as the
victim of DownDamageU, CaptureDamageHi, a powershielded tilt, an
ordinary-percent fly roll, DI/SDI and a wall tech
(`downdamage_up_victim_fd_marth`, `capture_hi_edge_pummel_victim_fd_marth`,
`powershield_ftilt_victim_fd_marth`, `damage_fly_roll_victim_fd_marth`,
`di_sdi_upin_fsmash_victim_fd_marth`, `passivewall_victim_fd_marth`), Fox's
air Cut -> Fall (`capture_hi_edge_cut_fall_fd_fox`), left-side wall jumps,
Fire Fox into both walls and along the floor, the ground blaster pushed off
the edge, an aerial Reflector reflect, two impacts on one shield, the
FallSpecial catch window, all six remaining air throws for both characters,
and almost every Fox held-item branch. Newly n/a on FD (research pass): Fox's
other shield-break orientation and any break at the edge, a laser plus a melee
hit on one shield, a ground Dancing Blade leaving the ground, repeated wall
jumps (search evidence), ceiling-triggered ledge and wall-jump exits,
relaxed jump entry and FallSpecial with jumps left. Still open: Fox's
blaster, Illusion and Fire Fox while holding a Bob-omb (the blaster is being
fixed), Fox holding one on the ledge, Fox's run-shield and Turn throws and
dying while holding, Marth catching a thrown Bob-omb and throwing from a
roll, Fire Fox AirHi -> Hi, a thrown Marth teching the wall, an airborne
DownDamage into a wall, Fox's shield taking two impacts, and pause.

These are the rows above with no gated retail witness, whose reachability
in the Fox–Marth FD scope is shown or plausible. The ranking weighs how
often a real match reaches the branch against how cheaply a directed
Dolphin scenario could witness it.

1. ~~Furafura exits~~: witnessed for Fox (`furafura_expire_fd_marth`,
   `furafura_hit_fd_marth`) and Marth (`furafura_{expire,hit}_victim_fd_marth`),
   2026-09-27.
2. ~~Fighter hitbox against a script-invincible hurtbox~~: n/a, Fox and
   Marth author only Intangible (status above). It is a port
   `unimplemented!` (damage.rs:1662). If any Fox or Marth startup (Dolphin
   Slash, the ledge getups, the getup attacks) is authored Invincible rather
   than Intangible, a routine out-of-shield up-B trade reaches it. Resolve the
   reachability first.
3. ~~Slow ledge options (100% or more)~~: Fox has all four at 300%
   (`sudden_death_ledge{climb,roll,attack,jump}_fd_fox`); Marth has ClimbSlow,
   EscapeSlow and JumpSlow at 300% (`sudden_death_ledge{climb,roll}_fd_marth`,
   `sudden_death_ledgejump_bomb_fd_marth`) and a corpus AttackSlow.
4. ~~Weak damage states: DamageHi1 (both), DamageLw1/DamageAir1 (Fox)~~:
   witnessed (`damage_level1_dancingblade_fd_marth`,
   `damage_air1_dancingblade_fd_marth`, `damage_level1_victim_fd_marth`);
   Marth's DamageAir1 is corpus only.
5. **Fox holding Bob-ombs in Sudden Death.** Now directed: pickup, every
   throw (ground, tilt, C-stick, dash, roll, Z, all eight air directions),
   the hold states, catching a thrown Bob-omb, the Reflector held, hit,
   knocked loose, grabbed and thrown while holding (section 10). Still open:
   the blaster while holding (`sudden_death_specialhold_bomb_fd_fox`, being
   fixed: the held-item hand pose during SpecialN), Illusion and Fire Fox
   with an item, holding on the ledge (not reached before the fuse ends),
   run-shield and Turn throws, and dying while holding.
6. **DownDamageD in general.** ~~DownDamageU~~: Fox `downdamage_up_fd_marth`,
   Marth `downdamage_up_victim_fd_marth`. Face-down is corpus only.
7. ~~Shield break in the other orientation and at the edge~~: n/a on FD
   (the flight is vertical with a fixed airtime; see section 6). Marth's
   DownU/StandU are witnessed (`shieldbreak_hold_fd_marth`).
8. ~~Ground blaster leaving the ground~~: `laser_loop_pushoff_fd_fox`.
   ~~Reflector ground rows walking off~~: all four witnessed.
9. ~~Light third-party hit on a captured fighter, and Fox-article hits
   during capture~~: n/a. Judged unreachable (COVERAGE_AUDIT): a laser flies away
   from Fox faster than either fighter closes, so it meets Marth before any
   grab and cannot come back (Marth deflects, never reflects), and it cannot
   hit its owner; Bob-omb hits are not light. The other grab-pair launch
   branches are ported and witnessed (`sudden_death_releasecaptor_bomb_fd_marth`,
   `sudden_death_pummelcaptor_bomb_fd_marth`).
10. **Simultaneous shield impacts on Fox's shield, and a phantom plus a
    shield impact in one frame.** Two impacts on Marth's shield are witnessed
    (`sudden_death_shieldlaserbomb_fd_marth`: a laser and a Bob-omb blast);
    a laser plus a melee hit is n/a (section 11).
11. ~~Marth as the DI/SDI victim~~: (by name) `di_sdi_upin_fsmash_victim_fd_marth`.
12. ~~Marth wall tech~~: `passivewall_victim_fd_marth`. Fox's PassiveWall
    and forward tech roll are corpus only.
13. ~~The fourth aerial Dancing Blade hit Up/Down; aerial full-charge Shield
    Breaker~~: `marth_air_dancingblade4_fd_marth`, `marth_airn_end1_fd_marth`.
    Ground Dancing Blade leaving the ground: n/a on FD (section 5).
14. ~~Counter's AirLwHit ending in the air~~: `counter_air_fall_fd_marth`.
15. **Fire Fox travel landing without a rebound (AirHi -> Hi).** ~~Travel
    into FD's side wall~~: `firefox_wall_{ledge,notch}_fd_fox`; shallow floor
    contact: `firefox_shallow_floor_fd_fox`.
16. ~~Marth CaptureDamageHi; Fox's air Cut -> Fall~~:
    `capture_hi_edge_pummel_victim_fd_marth`, `capture_hi_edge_cut_fall_fd_fox`.
17. **Item cross-interactions in Sudden Death:** Marth catching a thrown
    Bob-omb. ~~Fox catching one, the six remaining air throws, an aerial
    Reflector reflecting one~~: `sudden_death_catchthrown_bomb_fd_fox`,
    `sudden_death_airthrow*_bomb_fd_{fox,marth}`, `sudden_death_airreflect_bomb_fd_fox`.
18. ~~A decisive timeout and a simultaneous last-stock KO~~:
    `timeout_decisive_fd_marth`, `sudden_death_ledgejump_bomb_fd_marth`.
19. **Dash -> Squat (corpus only).** ~~Run/RunBrake/AttackDash -> Ottotto;
    OttottoWait -> attacks (Marth)~~: `ottotto_marth_{run,runbrake,attackdash}_fd_marth`.
20. ~~Open, reachable in principle~~ (2026-09-28): the DownDamage wall tech
    and a thrown Marth's wall tech are witnessed; the DownDamage wall bounce
    and Fox holding a Bob-omb on the ledge are not reachable in practice
    (`melee-sim search`, 652k and 5k candidates); pause is out of scope (not
    modelled); the Reflector and Dancing Blade scratch-word guards were never
    reached in 259k searched predecessor sequences. (Resolved as n/a: relaxed or C-stick jump entry, FallSpecial with jumps
    left, grounded DownReflect, StopWall, the airborne ftCo_Damage_IASA
    catch-all, inert hitboxes, the invincible contact,
    DeadUpFallHitCameraFlat, DownSpot, a grab pair losing its floor and
    revival-platform floor contact.)

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
