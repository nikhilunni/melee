# Laser reflection on Final Destination

Fox lasers now use the fighter reflector volume and deferred item response.
The physical shield contact retains priority. At item link 14, successful
reflection transfers ownership, invokes the laser response, updates working
and integer base damage, and applies the new owner's staling. The original
attack identity survives transfer. Mode-7 victim history expires at link 11.

Source owners include ftColl_80077464, Fighter_ProcessHit, Item_80269F14,
Item_80269B60 and lbColl_80008688. Fox's character table installs the
Reflector contact and hit-response callbacks; shared fighter code does not
branch on character kind. The response installs ReflectorHit and its radial
impulse. Powershield creates effect 1050 and retains synchronous hitlag
ownership.

The powershield effect exposed two lower-level defects. Child generators
with descriptor-owned AppSRTs must retain those transforms rather than
share the parent. PSVECMag/Normalize must round the double-width estimate
multiplier before fmuls. The latter corrects a documented omission in both
the Rust implementation and native-C reference. The C correction uses an
independent mathematical quantization. No captured expectations changed.
See `harness/gekko_probe/fmuls/README.md` for standalone instruction evidence
and the interpreter/JIT subnormal limitation.

Four 300-tick retail recordings exercise fresh and delayed powershields,
a reflected laser returning into Fox's Reflector, and reflection of a stale
laser. Tests cover canonical fighter/item state, RNG, ordered particles,
all captured fighter SRT frames, raw contact state, original item identity,
working/base damage and reflection history, plus zero allocations. Particle
simulation-field checks include AppSRT ownership counts and local transforms;
display-cache fields retain the existing separate presentation boundary.

The Null backend was independently compared with an existing OGL capture:
all 300 canonical/particle frames and 150 bone frames matched. It permits
new deterministic recordings on this host despite the native IOSurface
resource problem; it does not validate native rendering.

Both full workspace profiles pass: 1,371 tests, zero failures, one existing
ignore. After the final Clippy-only argument grouping and source ordering
cleanup, all 18 focused reflection tests also passed in each profile. The
harness has 225 passing tests, including immediate process-exit reporting and
backend/timeout forwarding. Workspace all-target Clippy with warnings denied,
formatting/schema checks, and the native app rebuild pass. Stripped simulator:
3,977,544 bytes (+16,832 versus the Fire-contact boundary). The full performance
and duplicate-definition census was not rerun; native visual smoke remains
limited by host IOSurface exhaustion.

Low-health overflow now has a legal aerial-laser recording: tick 233 enters
ShieldBreakFly with positive shield health and zero percent damage. Its
600-tick follow-through reaches the still-unimplemented ShieldBreakFall206;
that transition and its full regression are the next contact packet.
Reflection statistics metadata at retail D90 and beyond is not yet modeled.
The broader Fox/Marth/FD milestone remains open.
