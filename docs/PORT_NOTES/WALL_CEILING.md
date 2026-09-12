# Wall jump and ceiling recovery

Four independent 450-tick recordings cover Fox's underside wall jump, Fox and
Marth ceiling impacts, and a Marth wall-contact control on Final Destination.
The control demonstrates wall contact by a character without wall-jump support;
its stick timing is not an eligible wall-jump input witness.

The port now owns PassiveWallJump203 and StopCeil250, their authored motions,
persistent wall-contact counters, freeze/thaw collision shapes, buffered jump,
vertical decay, effects, and landing/wall/ledge priorities. Ceiling entry retains
horizontal velocity and performs its immediate unlocked collision pass before
clearing vertical velocity. Its empty callback still receives the common physics
integration tail.

The recordings also exposed three shared ownership defects. Revival must
reinitialize collision state in place, retaining the flag bits retail does not
clear. Tornado generators must restore the dump's `aux.tornado_velocity` key.
Every nonzero motion entry evaluates its preceding pose, regardless of whether
it is a ground/air transition. The missing evaluation left Marth's walking blend
one sample behind at tick331 even though its animation clock matched retail.
The corrected entry samples start-rate and start, then the ordinary walk entry
advances once more. Primary and secondary root histories reset only when their
authored extraction flags are active.

Named checks use the `walljump_stopceil` filter: four full SRT trajectories,
one ordered particle comparison across all recordings, one raw ownership check,
four fighter/item/RNG gates and four allocation gates. Retail expected values
were not modified. Captures remain ignored and backed up outside the checkout.

Further wall-jump repetitions and mirrored input witnesses, ceiling-triggered
ledge exits and nested ceiling landing remain separately unproven branches.
The next packet is recorded CaptureJump and prolonged airborne grab release.
Breadth remains paused.

## Verification

Debug and release workspace gates each pass 1,418 tests, zero failures and one
existing ignore. A final callback-sharing cleanup passes all 14 affected checks
in each profile, followed by all-target Clippy and the native macOS rebuild.
Formatting and 225 harness tests pass. No new graphical smoke is claimed.

The first performance run found two newly duplicated local closure labels in the
wall-jump and ceiling collision callbacks. Sharing the pose closure within each
callback and naming the floor predicate removes those repeated closure definitions
without changing geometry or predicate behavior. The original and final
measurements remain in `docs/PERF.md`; fixed ceilings are not relaxed.

Final census: melee-ft20 duplicate labels and cross-crate100, both within the
reviewed limits. Stripped size3,995,440 bytes (+17,408 versus6d05b47), text3,653,632;
load182.453ms and600ticks26.928ms. The callback cleanup changes the IR census but
not stripped size. Overall performance remains REGRESSION against the existing
size and tick-time ceilings. This packet does not claim to resolve binary debt.
