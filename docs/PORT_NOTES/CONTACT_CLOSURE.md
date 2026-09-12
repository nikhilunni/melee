# Directed clank and reflector-overflow contacts

This packet stays within Fox/Marth on Final Destination. Six 300-tick contact
recordings and one 600-tick overflow recording exercise branches that ordinary
attack and reflection replays had not distinguished. Captures stay ignored and
are mirrored outside the checkout; expected values were not edited.

| Scenario | Retail witness |
| --- | --- |
| `clank_jab_s74_f122_fd_foxmarth` | Both fighters enter ReboundStop237 at124, then Rebound238; two generator93 sparks, no percent damage. |
| `clank_priority_fox_spaced_fd_foxmarth` | At132 a clank spark accompanies15 damage to Marth; damage response supersedes recoil. |
| `clank_priority_marth_spaced_fd_foxmarth` | At130 a clank spark accompanies14 damage to Fox. |
| `clank_smash_norebound_spaced_fd_foxmarth` | At130 both sparks occur; Marth rebounds while Fox's authored smash disallows recoil. |
| `clank_airborne_fox_spaced_fd_foxmarth` | Airborne control, no ReboundStop; retained damage pose checked. |
| `clank_airborne_marth_spaced_fd_foxmarth` | Opposite airborne control, no ReboundStop. |
| `laser_reflect_overflow_air_timed_fd_marth` | Marth enters205 at233 with positive shield health1.34008777 and reflector maximum1; the laser disappears. Then206 at292,208 at294,210 at320 and211 at350. |

The airborne controls are exact trajectories; they are not independently
instrumented proofs of an overlapping attack-pair exclusion branch.

## Implementation and evidence

`lbColl_80006094` has a separate swept attack-pair geometry path, without the
hurt-capsule broadphase inflation. Retail instruction order governs degenerate,
parallel, endpoint-clamping and tie branches. Four exact rational tests cover
those branches and inclusive contact; the directed recordings supply numerical
integration evidence. A standalone guest numerical geometry sweep is not claimed.

`ftColl_8007699C` records ordered clank candidates and group victim histories.
Received knockback and shield impact precede clank; clank precedes ordinary
attack damage and reflection. `ftCo_80099D9C` enters animation-free237 and then
submotion45 for238, retaining the facing word and consuming pending recoil.
Spark93 and sound106 follow the contact order. Animation-free entry releases
scripts and AObjs while retaining the outgoing pose and dynamic ownership.

The overflow replay exposed two independent defects. `Fighter_ProcessHit`
updates shield health before a response can clear Guard. `efAsync_Spawn`
dispatches the shield-break burst immediately at link14. Health previously
regenerated after Guard was cleared, and the burst waited until the next frame.
Both are corrected at their owning phase. `ftCo_80098D90`, `80098E3C` and
`80098F3C` now cover falling and both down/stand orientations, preserving the
requested hit status and material ownership before frame-zero commands.

Fox's damage SRT delta was a missing `ftFx_Init_OnKnockbackEnter` callback:
part groups3/4 select variant3, then variant2 when hitstun expires. Bone42's
expected rotation3E860800 is the authored signed16/2^15 value0x2182. Static
character callbacks and explicit callback asset dependencies install those
sources; ordinary part playback owns their visible application. Hitstun and
texture-override latches are imported and reset at retail ownership boundaries.
Fox/Falco share family callbacks. Arbitrary costume TObj playback remains outside
this modeled texture-request representation; exact rendered pixels are not an
acceptance claim.

## Verification

Debug and release workspace gates each pass 1,404 tests, zero failures, one
existing ignore. After replacing generic callback defaults with concrete function
pointers, all 56 affected contact/allocation/throw/taunt/hitstun checks pass in
each profile. The final no-inline default also passes all 14 release throw checks.
All-target Clippy, formatting, 225 harness tests, schema validation and the native
macOS rebuild pass. Expected retail values were not changed.

The performance census caught four additional cross-crate default callback labels.
Associated function-pointer constants plus one explicitly non-inlined concrete
no-op restore the reviewed count of100, without changing callback dispatch behavior.
The final simulator is3,978,032 stripped bytes (+488 versus f0dd3b8); load183.259ms,
600 ticks26.137ms. Full perf remains REGRESSION against existing fixed size and
latency ceilings; those limits were not raised. The three diagnostic runs and
final census are retained in `docs/PERF.md`. Larger binary debt predates this packet;
the separate size audit also identified duplicated concrete cleanup code and cold
loader specializations as candidates, with no unmeasured savings claimed.

No new graphical smoke result is claimed; the last GUI smoke was host-limited by
IOSurface exhaustion. Headless oracle captures and the native build are verified.
No breadth expansion has started. Next packet: witnessed Fox WallJump, Fox/Marth
StopCeil, then recorded CaptureJump and remaining airborne release branches.
