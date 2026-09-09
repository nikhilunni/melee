# Fighter states and scheduler

Fox on Final Destination now supports the M4-T1 grounded movement slice:
Squat/SquatWait/SquatRv, standing Turn, and WalkSlow/Middle/Fast. The three
recorded 300-tick movement gates match all 49 keys, including the scene RNG.
Match-start Entry/Fall/Landing and the dynamic-bone solver are also implemented;
the older unimplemented-dynamics claims in START_FOX.md describe its original
callback-only milestone, not the current implementation.

Final M4-T1 validation: `cargo gate` **550 passed, zero failures, one pre-existing ignored doctest**; clippy clean. [Exact commands, file list and limits](M4_FOX.md).

## M5-A1 First hit

Marth's jab into an idle Fox on FD matches **300 ticks × 49 keys**, including
fighter overlap, damage, knockback, recovery and the complete shared RNG stream.
The independent particle replay matches 467,132 fields and 9,373 ordered draws.
Raw fighter scratch additionally checks hitbox endpoints, damage/radii and both
hitlag/hitstun countdowns. [Port report, audit and limits](M5_HIT.md).

| Retail functions | Addresses | Rust owner |
|---|---|---|
| Grounded fighter push / nudge | 8007DD7C / 8007E0E4 | `overlap.rs`, scene entity ordering and map neighbors |
| Attack11 entry / Anim / IASA / Phys / Coll | 8008ABC0 / 8008AC9C / 8008ACD8 / 8008ADF0 / 8008AE10 | `attack.rs`, typed Jab state, `CharacterCallbacks::jab_variant`, enum dispatch |
| Hitbox spawn / clear-one / clear-all / position | 8007121C / 80071784 / 800717D8 / 8007AD18 | `hitbox.rs`, `commands.rs`, archive descriptors and group victims |
| Hurt contact / capsule geometry | 80078C70 / 80006E58 / 80005EBC | `damage.rs`, `caches.rs`, `melee-lb/collision.rs` |
| Knockback / grounded DamageN2 / hitlag / recovery | 80079AB0 / 8008DCE0 / 8007DA74 / 8008F7F0 | `damage.rs`, typed combat scratch and proc gating |
| Slash spark / percent shake | 8007A06C / 80063930 / 802F4B84 | `melee-sim/effects.rs`, new `melee-if`, interface s_link 17 after particles |

The scene takes DamageN2 directly back to Wait, with no DamageFall or dash
attack. Other reactions, stale hits, DI/SDI, shield hits, clanks and jab follow-ups
remain explicit unsupported branches. No general moveset completeness is claimed.

## Lane C: Yoshi

All eighteen Yoshi idle/start/movement scenes and ordered particle ledgers pass.
The mixed Yoshi/Fox bone oracle compares 177804 SRT words across 70 Yoshi joints
(no dynamic chains) and 73 Fox joints. Egg shield entry/Anim/IASA/exit, egg rolls,
and the armored, animation-driven double jump use `ft-yoshi` hook overrides.
Saved part-owned AObj/FObj streams, nullable Wait/guard data, counted command
loops and static egg-shell effects are now supported. See
[M4_YOSHI.md](M4_YOSHI.md) and [YOSHI_DATA.md](../../../../docs/YOSHI_DATA.md).

## Lane C: Peach

All eighteen Peach idle/start/movement scenes and ordered particle ledgers pass.
The start/idle bone oracles compare 233632 SRT words across 114 Peach joints
(45 dynamic) and the 73-joint Fox opponent. Her aerial jump uses animation-driven
vertical velocity. Per-motion dynamic boundaries, opcode 50 subtree restoration,
static-stage collision stamps and the idle HUD resume boundary are now supported.
See [M4_PEACH.md](M4_PEACH.md) and [PEACH_DATA.md](../../../../docs/PEACH_DATA.md).

## Lane C: Captain Falcon

All eighteen Falcon idle/start/movement scenes and their ordered particle RNG
ledgers pass. The mixed Falcon/Fox bone oracle compares 169258 SRT words;
the full idle particle replay compares 858792 fields. The new `ft-captain`
crate uses existing character hooks. Shared asset loading now follows every
Wait/SquatWait choice, and the importer completes the idle save's interrupted
spherical emission from saved CPU/stack operands. See [M4_FALCON.md](M4_FALCON.md)
and [FALCON_DATA.md](../../../../docs/FALCON_DATA.md).

## M5-A2 Launches and shield contact (partial lane)

`jab_fd_fox`, `utilt_fd_marth`, and `shieldhit_fd_marth` each pass 300 ticks,
49 keys, zero divergences, plus full particle replays and raw fighter scratch.
The capture's jab is Fox versus Fox. **Grab startup passes ticks 0–126; linked
capture stops at tick 127.** Full gameplay/particle grab tests are explicitly
ignored with reasons; separate startup tests pass. The workspace gate is green
with this visible gap; the full grab acceptance remains incomplete.
[Port report, capture corrections, audit and remaining work](M5_COMBAT2.md).

| Retail functions | Addresses | Rust owner |
|---|---|---|
| AttackHi3 entry / callbacks | 8008BA38 / 8008BA98 / 8008BAD4 / 8008BB04 / 8008BB24 | `attack.rs`, `state.rs`, archive motion 58 and shared commands |
| Catch entry / Anim / Phys / Coll | 800D8C54 / 800D8CC8 / 800D8D88 / 800D8E08 | `grab.rs`, submotion 242, character hook; active pair query still unsupported |
| Throw-hitbox records / seek skip | 80071E04 / 80071F0C | `hitbox.rs`, `assets.rs`, `commands.rs`; raw startup oracle |
| First-hit staling of later hitbox commands | 80089118 / 80089228 | `commands.rs`, archive first stale weight; same-instance hitbox respawns |
| Launch angle / air knockback decay / Damage collision | 8008D7F0 / 8006B82C / 8008FB64 | `damage.rs`, `procs.rs`; DamageN1/Hi3, Landing, residual knockback in Fall |
| Shield contact / GuardSetOff / attacker pushback | 80076CBC / 80092F2C / 8006D1EC | `damage.rs`, `shield.rs`; group contact, damage, stun, hitlag and both pushbacks |
| Small normal spark / attached transform lifetime | 80063930 / 8005D174 / 8039D3AC | scene effects and `hsd-particle`; scale, owner aliases and descriptor camera-facing flag |
| FD static collision binding | 8021AAB0 / 804D4968 | scene composition applies map 3 root through existing collision transform updates |

These completed paths supersede A1's airborne-damage and ordinary-shield-hit
boundaries. Tumble, DI/SDI, powershield impacts, grab/throw, missed tech and
Attack12 remain unsupported. No character-specific shared-code branch was added.

## Lane C: Falco

All eighteen Falco idle/start/movement scenes pass at 49 keys per tick, with
ordered particle RNG ledgers. The mixed Falco/Fox bone oracle compares 174160
SRT words. Falco shares the typed Fox special-attribute reader in `melee-ft`
and uses the existing character hooks. His jump flash exposed the shared
motion-change effect queue flush; that now retains the outgoing transform and
retail request order. See [M4_FALCO.md](M4_FALCO.md) and
[FALCO_DATA.md](../../../../docs/FALCO_DATA.md).

## M4-T9 Falls and Marth shield entry

All sixteen Marth movement scenes and Fox's backward aerial jump now pass the
49-key scene gates, with independent ordered particle RNG checks. Shared
Fall/FallAerial/FallSpecial blend secondary directional animations without
changing their primary action-state IDs. Marth's hook selects the shielded
sword model and queues its sound; shield geometry uses the existing PlMs.dat
pose/joint data. See [M4_FALLS.md](M4_FALLS.md) for the sixteen-scene table,
raw scratch verification, assembly audit, corrections and remaining boundaries.

| Retail functions | Addresses | Rust owner |
|---|---|---|
| Fall blend selection / secondary evaluation | 800CCBE0 / 800CC988 | `fall.rs`; typed family, pose and weight; fused smoothing |
| Secondary animation attachment / blend / copy | 8006EDD0 / 8006FE9C / 8006FF74 | `anim/playback.rs`; main animation and command clocks retained |
| FallAerial entry / Anim / IASA / Phys / Coll | 800CCDA8 / 800CCDFC / 800CCE50 / 800CCE74 / 800CCE94 | `jump.rs`, `spawn.rs`, `fall.rs`, enum callbacks; existing air physics/collision |
| FallSpecial entry / Anim / IASA / Phys / Coll / landing callback | 80096900 / 80096AA0 / 80096AF4 / 80096B44 / 80096C98 / 80096D28 | `fall.rs`, `air_dodge.rs`, `procs.rs`, `landing.rs`; typed mobility and landing policy |
| Marth GuardOn / GuardReflect model hook | 800923B4 / 800939B4 | `ft-mars/init.rs`; model group 1 variant 1 and sound 190115 |
| Color-animation command | 80072A5C | `commands.rs`, `assets.rs`; typed renderer request |
| Reverse brake dust 0x400 | efAsync_Dispatch 80063930, efasync.c:270-281 | `effects.rs`, `melee-sim/effects/dust.rs`; existing generator 0x5A with negated facing |

This supersedes earlier reports' unimplemented FallAerial/FallSpecial and
Marth shield statements. Unused direct directional state entries, Yoshi's egg,
alternate special-fall gravity, combat and item branches remain unsupported.

## M4-T8 Marth and mixed-character scenes

Marth vs Fox on FD passes both idle and match-start gates: **600 ticks,
49 keys, zero divergences** each. The SRT oracle matches all 90 Marth bones,
including twelve dynamic joints, plus the 73-bone Fox opponent over 130 start
and eight idle ticks (**202,672 SRT words**). No rendered Marth matrix capture
exists, so the matrix half is not run.

Character crates own archive/costume descriptors and callbacks. Shared loaders
accept explicit animation, part and part-animation counts. The simulator uses
an enum of boxed concrete fighters and a generic per-fighter proc dispatcher;
Snapshot's 24 fighter keys and scheduler order are unchanged. Marth's landing
reset is a character hook. The existing effect 0x24 warp and dynamic solver
work directly from Marth data. The imported boundary now handles both partial
idle ticks and completed idle savestates, plus unlock-dependent FD music RNG.
See [M4_MARTH.md](M4_MARTH.md) for the exactness evidence, complete list of Fox
assumptions, capture corrections and remaining boundaries.

## M4-T7 Running reversal and ledge options

`turnrun_fd_fox` and `walkfast_fd_fox` match **300 ticks, 49 keys, zero
divergences**; `ledgeclimb_fd_fox` and `ledgeescape_fd_fox` match **420 ticks,
49 keys, zero divergences**. All four have raw fighter replays and ordered
particle RNG checks. WalkFast needed no behavior correction. See
[M4_TURNRUN.md](M4_TURNRUN.md) for the audit, commands and remaining boundaries.

| Retail functions | Addresses | Rust owner |
|---|---|---|
| TurnRun predicates / entry | 800C9CEC / 800C9D40 / 800C9D94 | `turn_run.rs`, Run/RunBrake input in `run.rs`; typed facing and pause latch |
| TurnRun Anim / IASA / Phys / Coll | 800C9E10 / 800C9ED8 / 800C9EFC / 800CA024 | `turn_run.rs`, enum dispatch in `procs.rs`; fused friction, pause/reversal, edge stop |
| Post-turn Run predicate | 800CA644 | `turn_run.rs`; archive-derived interrupt delay from PlCo +430 |
| CliffClimbQuick / CliffEscapeQuick entry | 8009AB9C / 8009B040 | `ledge.rs`; shared retained `CliffState`, grab exclusions and nudge suppression |
| CliffClimb Anim / Phys / Coll; CliffEscape wrappers | 8009AC68 / 8009ACA8 / 8009ADA4; 8009B10C / 8009B130 / 8009B150 | `ledge.rs`; map endpoint + TransN, grounded root motion and edge collision |
| Ground conversion / grounded root-motion physics | 8007D6A4 / 80084FA8 / 80085030 | `landing.rs`, `ledge.rs`; retained Y velocity and audited horizontal extraction |
| Grab-category exclusions | 8007E2F4 | typed `GrabExclusions`; catch/wait use 0x1FF, options use 0x20 |

Climb becomes grounded at tick 254 and enters WalkSlow at 266; escape becomes
grounded at 252 and enters Wait at 281. Subaction intangibility ends at 262
and 266 respectively. Quick/Slow selection uses **damage percentage**, not
hang duration; Slow climb/escape remain explicit unsupported entries. Existing
`CharacterCallbacks` defaults remain the character boundary; these new retail
state bodies have no character-kind branches. Effect routing and particle
math needed no changes, and no new particle field dumps were supplied.

## M4-T5 Air dodge, wavedash and ledges

`airdodge_fd_fox` and `wavedash_fd_fox` match **300 ticks, 49 keys, zero
divergences**; `ledge_fd_fox` matches **420 ticks, 49 keys, zero divergences**.
All three also have callback replays and ordered particle RNG checks.
See [M4_LEDGE.md](M4_LEDGE.md) for validation, the retail audit and limitations.

| Retail functions | Addresses | Rust owner |
|---|---|---|
| EscapeAir input / entry / Anim / IASA / Phys / Coll | 80099A58 / 80099A9C / 80099BD0 / 80099C24 / 80099CEC / 80099D48 | `air_dodge.rs`, shared aerial input in `procs.rs`; typed momentum and timer, command-controlled hurt status |
| LandingFallSpecial entry; Landing common callbacks | 800D5CB0; 800D5D3C / 800D5D78 / 800D5F18 / 800D5F38 | `landing.rs`, caller-supplied rate installed before frame-zero commands |
| Jump / JumpAerial backward selection | 800CB250 / 800CBBC0 | `jump.rs`; facing is retained for both backward animations |
| Ledge grab predicate / catch entry / snap | 80081298 / 80081370 / 80081544 | `ledge.rs`; map-derived ledge ID, animated TransN, cleared momentum |
| CliffCatch Anim / Coll; Cliff camera | 80081504 / 800815E4; 80081644 | `ledge.rs`, `procs.rs`; ECB mode 0xA, stage notification and camera flag |
| CliffWait entry / Anim / IASA / timeout check | 8009A804 / 8009A8D8 / 8009A8FC / 8009A9AC | `ledge.rs`; wait duration, neutral latch, timed intangibility, ordered option predicates |
| CliffJump entry / phase transition / launch Phys | 8009B1B8 / 8009B2F8 / 8009B464 | `ledge.rs`; quick/slow descriptors, retained timer, first-tick gravity skip |
| Airborne collision / ledge-cooldown branch | 80083090 / 800835B0 | `collision/air.rs`, `procs.rs`; platform pass with or without ledge detection |
| EscapeAir ground contact | 80082C74 / 80081D0C | `collision/air.rs`; ordinary airborne collision without ledge grabs |
| Ledge effect 0x41C; special landing 0x407 | efasync.c:521-523; 305-307 | `melee-sim::effects`; positional particle 93 / root-relative particle 60 |

The recorded **262/263 are CliffJumpQuick1/Quick2**, not Slow1/Slow2 (260/261).
The ledge ledger's dust calls at 246 and 283 are the launch and the landing.
No supplied scene enters FallSpecial (35), so that animation-completion branch
remains explicit. CliffAttack and Slow climb/escape, timeout into DamageFall, occupied
ledge arbitration, items/tethers and ceiling interactions remain unsupported.
`CharacterCallbacks::on_landing` and `air_dodge_tether` own character branches.
No new RNG site or particle arithmetic was needed; particle field dumps for
these three scenes have not been supplied.

## M4-T4 Shield, spot dodge and roll

`shield_fd_fox`, `spotdodge_fd_fox` and `roll_fd_fox` each match **300 ticks,
49 keys, 0 divergences**, including RNG. Raw callback replays also check shield
health, lightshield, tilt/timer scratch, startup flags and dodge hurt status;
ordered particle RNG checks cover all three scenes. See [M4_SHIELD.md](M4_SHIELD.md)
for the commands, fusion audit, character hooks and remaining boundaries.

| Retail functions | Addresses | Rust owner |
|---|---|---|
| Guard input / GuardOn entry / GuardReflect entry | 80091A4C / 800924C0 / 80093A50 | `shield.rs`, typed `GuardState` and `ShieldState` |
| GuardOn / Guard / GuardOff / GuardSetOff / GuardReflect Anim | 800926DC / 80092A24 / 80092CAC / 80093354 / 80093CD0 | `shield.rs`; enum callback tables in `state.rs` |
| Guard hold / GuardOff entry / startup-window expiration | 80092908 / 80092C54 / 80093BC0 | `shield.rs`, scratch retention and collision reset |
| Shield tilt / size / pose / health drain | 80091BC4 / 80091D58 / 80091E78 / 800925A4 | `shield.rs`, `anim/playback.rs`; owned descriptor pose and joint scale |
| Shield / reflect collision descriptors | 80092450 / 8009370C | `shield.rs`, typed volumes and hit callbacks; M5 owns hit response |
| Per-frame shield proc | 8006D1EC | `procs.rs`, `shield.rs`; active drain and inactive regeneration |
| Roll predicate / entry / Anim; spot-dodge predicate / entry / Anim | 8009917C / 80099314 / 800994D8; 8009980C / 800998EC / 800999D8 | `escape.rs`, typed timer, facing and hurt status |
| Roll physics / TransN acceleration; escape collision | 80085004 / 80085030; 80084104 | `escape.rs`, `collision/ground.rs`; stage-edge clamping |
| Stick angle | 8000D008 | `melee-lb::trigf::stick_angle`, including neutral-vector handling |
| Shield effects / attached AppSRT refresh | 8005BC50 / 8005D174 / 8039D214 | `melee-sim::effects`, `hsd-particle::system` |

Retail IDs are **178 GuardOn, 179 Guard, 180 GuardOff, 181 GuardSetOff,
182 GuardReflect**. These captures start with GuardReflect; the post-dodge
return to 178 is GuardOn. GuardSetOff callbacks exist, but its damage/stun entry
is explicitly M5. `CharacterCallbacks::guard_variant` and `escape_variant`
keep Yoshi/Marth and Samus/Yoshi branches in character hooks; Fox uses defaults.

## M4-T3 KneeBend, JumpF, JumpAerialF and fast fall

`jump_fd_fox` matches **300 ticks, 49 keys, 0 divergences**. The raw callback
replay checks the hop decision, first-physics flag, command variables, fast-fall
flag, vertical input age and landing/crouch scratch. Button/stick hold and
release/repress tests distinguish full hop from the latched short hop.
See [M4_JUMP.md](M4_JUMP.md) for exact commands, effect routing and limits.

| Retail functions | Addresses | Rust owner |
|---|---|---|
| Jump_GetInput; KneeBend Enter / Anim / Check_ShortHop / IASA | 800CAE80; 800CB4E0 / 800CB528 / 800CB59C / 800CB5FC | `jump.rs`, typed input source and hop decision |
| Jump entry velocities / Enter / Anim / IASA / Phys_Inner | 800CB110 / 800CB250 / 800CB2F8 / 800CB334 / 800CB438 | `jump.rs`; first Phys skips gravity, later Phys uses air drift |
| JumpAerial_Enter_Basic / shared entry / Anim / IASA / Phys | 800CBBC0 / 800CBAC4 / 800CC388 / 800CC4F8 / 800CC634 | `jump.rs`, shared aerial input dispatch in `procs.rs` |
| Fall_IASA_Inner / CheckFallFast / FallFast / air physics | 800CCAAC / 8007D528 / 8007D4E4 / 80084DB0 | `fall.rs`, `jump.rs`, `physics/airborne.rs` |
| Landing_Enter / Landing_IASA / direct SquatWait | 800D5AEC / 800D5D78 / 800D62C4 | `landing.rs`; preserves scratch and shows nametag |
| KneeBend Coll / Jump Coll / JumpAerial Coll | 800CB6CC / 800CB4B0 / 800CC700 | shared ground/air collision, explicit StopCeil boundary |
| ftCo_8009F834 kind 0; efAsync_Dispatch 402/403 | 8009F834; efasync.c:282-287 | `effects.rs`: live fighter-bone attachment, no offset RNG |

Fox uses the ordinary aerial jump, not the multijump `JumpAerialF1` family.
Both JumpF launches in this capture are short hops. The first lands before its
animation ends; it never enters Fall. Fast fall starts at tick 136, landing
clears it at 152, and landing lag admits SquatWait at 156. Direct SquatWait
preserves Landing's scratch; it does not run Squat's platform-drop reset.
The otherwise inactive retained timer contains the prior jump physics flag's
bits, matching the raw-state replay without changing expected values.

Jump entry arithmetic was checked in retail assembly: all products and sums
are separately rounded; no fused instruction appears in the audited jump,
KneeBend, or fast-fall helpers. Grounded jump physics skips only its first
callback; aerial jump physics applies gravity immediately.

The full particle replay matches **489,588 fields and 9,544 ordered draws**
over 300 ticks, with **zero display-cache exclusions**. Its five external
spawn requests and attachment matrices come from the port's own run. The
shared replay retains the original dash AppSRT display-cache exclusion helper
unchanged. HSD's existing particle paths cover all three dust descriptors.

FallAerial animation completion, character-specific/multijump
entries, combat/item transitions, and ceiling impacts remain explicit
unsupported paths. Full-hop release behavior has a callback test; the supplied
300-tick retail jump scenario validates the short-hop/double-jump trajectory.

## M4-T2 Dash/Run/RunBrake

`dash_fd_fox` matches **300 ticks, 49 keys, 0 divergences**. The state replay
also compares command variables, both effect flags, ground velocity, animation
rate/command clocks, and Dash/Run/RunBrake scratch fields. `m4_gate` verifies
the complete particle call-site sequence against the 300-tick RNG ledger.
See [M4_DASH.md](M4_DASH.md) for validation, affected files and limitations.

| Retail functions | Addresses | Rust owner |
|---|---|---|
| Dash CheckInput / Enter / Anim / IASA / Phys / Coll | 800CA094 / 800CA120 / 800CA1F4 / 800CA230 / 800CA53C / 800CA5D0 | `dash.rs`, `run.rs`, shared ground collision |
| Run check / Enter / Enter_Full / Anim / IASA / Phys / Coll | 800CA5F0 / 800CA6F4 / 800CA71C / 800CA77C / 800CA830 / 800CA95C / 800CAA2C | `run.rs`; ordinary zero-phase entry |
| RunBrake CheckInput / Enter / Anim / IASA / Phys / Coll | 800CABC4 / 800CAC18 / 800CAC9C / 800CADB0 / 800CAE18 / 800CAE60 | `run.rs`, shared Wait collision |
| ftCommon_800804A0 / ftCommon_8007C98C / ftCommon_ApplyGroundMovement | 800804A0 / 8007C98C / 8007CB74 | initial secondary acceleration, target clamp and projection |
| ft_8008A2BC / ft_800844EC / ftCo_8009EDA4 | 8008A2BC / 800844EC / 8009EDA4 | Wait return, ordinary ground support and explicit StopWall boundary |
| ftAction_80071028 / ftAction_80071820 | 80071028 / 80071820 | five-word GFX decode and command variables |
| ftCo_8009F834 / ftCommon_8007DB24 | 8009F834 / 8007DB24 | randomized offsets, typed rotating bone cursor and effect destruction flag |
| ft_80089B08 | 80089B08 | RunBrake body tilt on long flat floors |

Dash first-frame acceleration uses `xE8_ground_accel_2`; later ticks use the
shared ground acceleration/clamp. Run tapers acceleration and scales animation
rate by ground velocity. RunBrake retains its two command-variable controls,
maximum duration and pause/release logic. Turn can now enter a dash with its
initial attack/escape window disabled. Combat/item/shield/jump-cancel,
short/sloping body tilt and StopWall entries remain explicit boundaries. No
new `ftCo_0A01.c` helper is called by these item-free human paths.

Dash needs the existing TransN extraction code wired into fighter construction;
motion entry clears extracted frame-zero velocity. GFX commands retain their
archive-derived bone, flags, parameter, signed offsets and unsigned ranges.
The literal scale is **0.003906f**, not exactly 1/256. The renderer-facing
invisibility flag suppresses the command before RNG or bone-cursor changes.

The recorded dust takes the later `ftCo_8009F834` switch: fused additions at
**8009FCF8, 8009FD1C, 8009FD44**, following draws at FCDC/FD00/FD24. The early
`gfx_id < 0x250 || gfx_id/1000 == 30` branch uses **8009F94C, 8009F970,
8009F9A4** and async kind 2. Both are implemented. The scene resolves queued
command requests at each fighter proc boundary and flushes the async queue in
reverse insertion order at s_link 9. Runtime code reads no RNG ledger.

## M4-T1 ports

Addresses below were resolved against the retail symbol map. State callbacks
are enums in `state.rs`; scratch data uses typed `MotionData` variants.

| Retail function(s) | Address(es) | Rust owner |
|---|---|---|
| `ftCo_Squat_Enter`, `ftCo_Squat_Anim`, `ftCo_Squat_IASA` | 800D600C, 800D607C, 800D60B8 | `squat.rs` |
| `ftCo_800D638C`, `ftCo_SquatWait_Anim`, `ftCo_SquatWait_IASA` | 800D638C, 800D6448, 800D6474 | `squat.rs`, shared animation restart |
| `ftCo_SquatRv_CheckInput`, `ftCo_SquatRv_Enter`, `ftCo_SquatRv_Anim`, `ftCo_SquatRv_IASA` | 800D65D8, 800D6620, 800D6658, 800D6694 | `squat.rs` |
| Squat / SquatWait / SquatRv Phys | 800D623C / 800D6584 / 800D6784 | shared `physics/grounded.rs`; only hold invalidates shield position |
| Squat / SquatWait / SquatRv Coll | 800D625C / 800D65B8 / 800D67A4 | `collision/ground.rs::map_ground_action` |
| `ftCo_Turn_Enter`, `ftCo_Turn_Enter_Basic`, `ftCo_Turn_Enter_Smash` | 800C9840, 800C98AC, 800C9C74 | `turn.rs` |
| `ftCo_Turn_Anim_Inner`, Anim, IASA, Phys, Coll | 800C9924, 800C9970, 800C99F8, 800C9BEC, 800C9C0C | `turn.rs`, shared ground helpers |
| `fn_800C9C2C` | 800C9C2C | Turn dash-direction buffering |
| `ftCo_Walk_CheckInput`, Enter, Anim, IASA, Phys, Coll | 800C9468, 800C9528, 800C95F4, 800C9614, 800C9768, 800C9788 | `walk.rs`, shared ground helpers |
| `ftWalkCommon_GetWalkType_800DFBF8`, `800DFCA4`, `800DFDDC`, `800DFEC8` | 800DFBF8, 800DFCA4, 800DFDDC, 800DFEC8 | walk tier, entry, rate and phase conversion |
| `ftWalkCommon_800E0060`, `ftCommon_8007C98C`, `ftCommon_ApplyGroundMovement` | 800E0060, 8007C98C, 8007CB74 | `physics/grounded.rs` acceleration, target clamp and projection |
| `ft_80084F3C`, `ftColl_8007AEE0` | 80084F3C, 8007AEE0 | shared friction, explicit shield-cache invalidation |
| `ft_80083F88`, `ft_80082708`, `ft_80084280` | 80083F88, 80082708, 80084280 | ordinary Squat/Turn collision versus Walk/Wait teeter collision |
| `ftCo_Wait_IASA`, `ft_8008A244`, `ft_8008A348` | 8008A4D4, 8008A244, 8008A348 | ordered movement entries and return to Wait |
| `Command_07`, `Command_08`, `ftAction_80073354` | 80005AE4, 80005B00, 80073354 | script goto, animation-loop wait and nonzero-phase seek |
| `ftAction_80072CD8`, `ftAction_800728F8` | 80072CD8, 800728F8 | FD footstep sound and controller rumble requests |

WalkFast shares the same selection, rate and physics implementation. Its tier
threshold boundaries are unit-tested, and M4-T7's `walkfast_fd_fox` now verifies
all three tiers against retail. Walk phase conversion uses retail `800E0010 fnmsubs`, followed
by separate divide/multiply and `fctiwz`. Acceleration uses separate `fmuls`
at 800E008C/0090/009C or 00AC, then `800E00B4 fadds`. No FMA is introduced there.
Every imported resource comes from the owned archives; gameplay reads no trace
or tick number.

`movement_fox_states` uses the common replay helper with trace-derived length
and an explicit `ledger` suffix. It supplies only recorded pad inputs and the
external pre-draw seeds needed to isolate fighter RNG. It compares 48 snapshot
keys plus raw submotion, rate, remainder, ground velocity, command clocks,
nametag timer and relevant Turn/Walk/Squat scratch fields. `m4_gate` separately
runs the complete scene and checks all 49 keys with produced RNG. Local-data
absence skips both test families cleanly.

Remaining explicit movement boundaries: platform-drop entry (ftCo_Squat.c:79-84), ledge
Fall/Ottotto entries, attack/special/jump/shield transition bodies, metal
or scaled-player modifiers, and non-default terrain footstep effects. The new
IASAs retain their own predicate ordering and reject unsupported transition
bodies. Sound/rumble are queued output requests; audio/controller playback is
outside the headless simulation. FD's default footstep mapping has no graphics
request or RNG. There were no new RNG sites or unexpected recorded transitions.

`Fighter<C>` owns `FighterPhysics` (including position and facing),
`FighterAnimation`, `FighterInput`, `EnvironmentCollision`, attributes, bone
descriptors, the main `JObjTree`, installed `MotionState` callbacks, command
state, CPU state, character hooks, and collision/camera caches. Source names
and Fighter offsets are documented on the fields. `FighterAssets` owns the
archive-derived resources; neither archive addresses nor trace ticks appear in
the gameplay state machine.

`Snapshot` emits exactly the 24 keys from `harness/schema/fighter.yaml`, including
all seven CPU fields for human slots. The scene wraps it in `PrefixSink` with
`p0` or `p1` and owns the separate global RNG field.

## Spawn and motion entry

The port composes these ordinary Fox paths, with exceptional interactions gated:

| C entry | Rust owner / behavior |
| --- | --- |
| `Fighter_Create`, `0x80068E98` | `Fighter::spawn`; caller supplies a costume skeleton, player slot and stage marker |
| `Fighter_UnkInitLoad_80068914`, `0x80068914` | `prepare`; typed data, parts, skeleton, input and character load hook |
| `Fighter_UnkInitReset_80067C98`, `0x80067C98` | spawn position/facing, velocities, damage, counters, shield and inactive interactions |
| `Fighter_NewSpawn_80068E40`, `0x80068E40` | shared `SpawnCounter`, increment with zero skipped on wrap |
| `Fighter_UnkProcessDeath_80068354`, `0x80068354` | initial support probe, model scale, character reset, CPU init and thrown capsule initialization |
| `ft_80082A68`, `0x80082A68` | vertical support probe through T5/MP |
| `ftCommon_8007D5D4`, `0x8007D5D4` | failed probe: airborne, one jump used, ten-frame ECB lock |
| `ftCo_800A101C`, `0x800A101C` | initializes CPU reaction timers, then calls `ftCo_800B9704` to initialize typed attack delay; two RNG draws even for a human slot |
| `Fighter_ChangeMotionState`, `0x800693AC` | `change_motion_state`; reset motion-owned state, install callbacks, animation, command entry and dynamic-bone gates |
| `ft_8008A348`, `0x8008A348` | grounded Wait entry and nametag duration |
| `ftCo_Fall_Enter` | neutral airborne Fall entry and callbacks, including Landing |
| `ftLib_800867E8`, `0x800867E8` | clear/freeze input at the end of cold creation |
| `ftFx_Init_OnLoad`, `0x800E57AC` | `ft_fox::init::Fox`: owned special attributes, walljump/special capabilities and three item resource registrations |
| `ftFx_Init_OnDeath`, `0x800E5554` | clear blaster state and select default model group |

The reset assertions cover all 24 snapshot fields, previous position, spawn
number, input freeze, disabled/interaction state, hit/smash timer sentinels,
shield health, ledge cooldown, sword trail, walljump, Fox item registrations and
blaster state, thrown capsule state, dynamic-bone selection, callback table,
ground-pose bits, ECB lock, floor index, six ECB bones and model scales. This is
an owned idle representation, not a byte-layout port of every Fighter member.

Root scale is `player.scale * co_attrs.model_scaling` (`ftCommon_GetModelScale`,
retail `0x8007F69C`, separate `fmuls`), giving Fox `0.96` (`0x3F75C28F`). Bone 67
gets the reciprocal (`0x3F855556`) in T7's `reset_pose`, corresponding to
`ftCommon_8007F6A4`. Spawn X uses the retail `fmadds` at `0x80067CE8`. CPU init
uses double multiply/truncation at `0x800A124C`/`0x800A1258`. Other newly ported
arithmetic sites cite their checked instruction sequence beside the expression.

FD's stage-position bindings 0 and 1 are `(-60,10,0)` and `(60,10,0)`.
The retail probe does not find support there, so cold creation enters Fall.
The idle savestate is already grounded at approximately y=0.0001 and must be
imported separately. The original self-authored cold expectation assumed a
ground-level marker; it was corrected against the stage data and C path.
No oracle expectations were changed.

## Scheduler interface

`interleaved_order(2)` visits P0 then P1 within each row, preserving scheduler
link order. The scene supplies pad samples, wind, collision map, camera zoom
and shared RNG, and inserts its own stage/particle callbacks between phases.

| s_link | Method | C symbol |
| --- | --- | --- |
| 0 | `proc_status` | `Fighter_8006A1BC` |
| 1 | `proc_anim` | `Fighter_8006A360` |
| 2 | `proc_cpu_gate` | `Fighter_8006ABA0` |
| 3 | `proc_input` | `Fighter_Spaghetti_8006AD10` |
| 4 | `proc_update` | `Fighter_procUpdate` (`0x8006B82C`) |
| 6 | `proc_map` | `Fighter_procMap` (`0x8006C27C`) |
| 7 | `proc_pose` | `Fighter_8006C5F4` |
| 8 | `proc_accessories` | `Fighter_CallAcessoryCallbacks_8006C624` |
| 9 | `proc_hitbox_positions` | `Fighter_8006C80C` |
| 12 | `proc_grab` | `Fighter_UnkProcessGrab_8006CA5C` |
| 13 | `proc_hit_detection` | `Fighter_8006CB94` |
| 14 | `proc_process_hit` | `Fighter_ProcessHit_8006D1EC` |
| 16 | `proc_dynamics` | `Fighter_8006D9AC` |
| 18 | `proc_camera` | `Fighter_UnkCallCameraCallback_8006D9EC` |
| 22 | `proc_player_mirror` | `Fighter_8006DA4C` |

Animation, input, physics and map dispatch consult the installed callback set.
The ordinary attacks are disabled during Wait, but the thrown capsule still
advances its current/previous bone position at link 9 (`ft_8007C224`). Hurtbox
caches are invalidated at link 4 and feed CPU extents at link 14. Dynamic
collider positions update at link 16. Wait flags select disabled dynamic-bone
solving: `ftCo_8009CB40(..., false, NULL)` sets the first bone to `0x100`, and
`lb_8001044C` returns on that sentinel. This is checked against every ledger row.

## Explicit boundaries

`Status::interaction` guards unsupported systems. Attack, damage and hitlag now
have typed owners for the M5-A1 slice above; the scene scans fighter pairs in
entity-list order and handles grounded overlap after each animation callback.
Shield without a hit is supported by the typed Guard state machine. Items,
grabs, death, status effects, accessories, stage hazards and coin-match rules
retain C-located `unimplemented!` boundaries. Empty ordinary-Wait branches remain
checked against the raw ledger's item, accessory, async and catch fields.

IASA bodies outside the movement slice above, CPU AI, unsupported motion entry,
teeter/fall from a ledge, sloped leg correction/body tilt, shield hit response,
and scaled-player attribute modifiers fail explicitly. Neutral Fall through
Landing/Wait and the Fox tail solver are implemented and covered by match-start
gates.

The reachable Wait command subset executes timing, call/return, part animation
and ground-pose commands. Texture-frame commands retain requests for a renderer;
TObj/material/GX execution remains outside this simulation slice. Other opcodes
are rejected while loading. This is not a general T8 command interpreter.

## Original idle verification and initial-state import

`idle_fox_600` compares all 48 fighter values through `Snapshot` and `melee-diff`:
P0 600/600, P1 600/600, first mismatch none. The fixture contains frame 0 and 599
transitions. All nine Wait RNG draws match. Particle draws are represented by
feeding each fighter draw's preceding seed from the ledger, as in T7; this does
not certify the standalone scene RNG stream.

The test imports frame-zero physics, input histories, command state, CPU fields
and complete CollData history. A test-only LZ4 reader also imports the main/blend
joint transforms, flags and matrix caches from the locally owned savestate.
It validates that its fighter and animation scalars match the ledger boundary.
An inferred Entry pose initially failed P1's capsule check at tick 1; importing
the actual saved pose fixed it. Both capsule positions, command timers/frames,
nametag countdown and dynamic-bone sentinel are additionally checked every tick.
No subsequent expected row drives gameplay, except the explicitly scoped RNG
seed input. The test skips if local disc files, traces or savestate are absent.

`fox_spawn_native` loads fresh costume skeletons and actual FD spawn markers.
Its reset expectations come from C stores/data and the cited retail arithmetic;
it does not claim a complete native-C clone of Fighter_Create. Other tests cover
CPU initialization modes, counter wrap, callback dispatch, unsupported-path
panics, the Wait callback table and scheduler interleaving.

Changed files (crate paths are relative to `crates/`):

- `melee-ft/src/fighter/{mod,assets,caches,commands,procs,snapshot,spawn,state}.rs`
- `melee-ft/src/fighter/README.md`
- `melee-ft/src/lib.rs` and `melee-ft/Cargo.toml`
- `ft-fox/src/{lib,init}.rs`
- `melee-ft/tests/{idle_fox_600,fox_spawn_native}.rs`
- `melee-ft/tests/fighter_support/{mod,collision,saved_pose}.rs`
- Root `Cargo.lock`: two local dev-dependency edges.

Original T10 checks: `cargo gate` passed (514 passed, zero failures, one pre-existing
ignored doctest); `cargo clippy --workspace --all-targets -- -D warnings` passed.
The four cold-spawn/guard tests and the 600-record replay ran with local data.
No protected files, game data or commits were added.
