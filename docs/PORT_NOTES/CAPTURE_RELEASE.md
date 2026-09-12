# CaptureJump and airborne grab release

Six independent450-tick Fox/Marth recordings on Final Destination distinguish
held-Up and buffered-XY CaptureJump with both captors, prolonged airborne
CapturePulledHi/CaptureWaitHi followed by CaptureCut, and the same edge hold
followed by CaptureJump. Inputs drive the simulation; retail state is comparison
only. No expected values were modified.

CaptureWait latches XY only inside its authored initial interval. Timer expiry
returns before playback-rate maintenance and chooses Jump if that latch or held
Up requests it. Captor separation is represented separately and always selects
ordinary Cut. Release clears the paired links and supplies the authored ground
or air velocities. CaptureJump owns motion258, its float interrupt counter,
non-fastfall gravity/drift and AirCatchHit collision. CaptureCut can land while
retaining its motion; its airborne map uses ECB5, while CatchCut uses ECB6 and
ordinary landing. Wall jump precedes ledge in both cut paths.

The recordings exposed two additional owners. CaptureJump modifies only scratch
x0; Landing inherits CaptureWait's retained x4 playback timer. Grab startup uses
ft_800827A0's stop-at-edge collision, which keeps Fox correctly positioned before
his catch capsule activates. The fix changes Catch/CatchDash rows and preserves
the distinct collision paths for unrelated down/throw states.

The four grounded releases cover both input choices and both captors. The edge
Cut recording has Marth in223at115,224at116..190,229at191 andFall29at221. The edge
Jump recording enters230at185 andFall29at235. These are actual sustained air
holds, unlike the earlier same-tick high-to-low capture counterparts.

Twenty directed checks cover six full local-bone trajectories, all particle
fields, owned raw capture/collision fields, six fighter/item/RNG replays and six
zero-allocation replays. The raw comparison includes the capture timer, jump
latch, CaptureJump x0/x4, velocities, animation clocks and collision histories.

Forced-separation-Up, airborne captor CatchCut and CaptureCut wall-jump remain
source-derived paths without independent directed witnesses in this packet.
Arbitrary active-capture savestate restoration and thrown-constraint release
are not claimed complete. This packet does not expand characters or stages.

## Verification

Both full workspace profiles pass1,438 tests,0 failures and1 existing ignore.
All-target Clippy,225 harness tests,formatting and the native macOS rebuild pass.
No new graphical smoke is claimed. The six recordings and backups remain ignored.

Perf census stays20 local/100 cross-crate duplicate labels. Stripped size is
3,995,872 bytes (+432 versus987fe7f),text3,653,632; load182.804ms and600ticks26.004ms.
The existing size/time ceilings remain red and unchanged; `docs/PERF.md` retains
the complete measurement. This packet does not claim to resolve binary debt.
