# Fire Fox charge and travel contact

Two 300-tick retail recordings cover repeated charge contact and a travel hit
against Marth. This packet closes those directed contacts, not all recovery
collisions or simultaneous contact combinations.

## Corrections

- Fire hit sparks dispatch effect 0x3EA and positional generator 0x14.
  Fire damage uses authored color programs 11 through 14, including attached
  body effects 0x413/0x414 and their rotating bone selection. The programs own
  particle and sound timing; they are not replaced with synthesized bursts.
- Fighter_8006D9AC skips dynamic-bone integration during hitlag. Retaining the
  springs and local transforms fixes the first damaged Marth cape pose.
- Display publishes dynamic-chain joint matrices even while the solver owns
  their local transforms. Fox can leave SpecialHiLanding, enter Wait and then
  Walk in one tick; releasing and reclaiming a chain reads the prior display
  cache. Publishing only animation-owned joints lost that cache.
- Fire charge/launch/rebound install the retail effect-hitlag callbacks, and
  motion entry clears them. efLib_Update gives eligible models one final update
  on entering pause, then suspends them until resume. ASYNC models bypass this
  pause state; attached body particles retain their separate lifecycle.
- The particle snapshot adapter now names rectangle dimensions and diagonal
  axes according to the active oracle union. It checks imported axes/flags
  against the supported authored representation instead of silently dropping
  those fields. Rotated rectangle import remains explicitly unsupported.

Source anchors: efAsync_Dispatch (0x80063AEC), Fighter_8006D9AC,
Fighter_SetEffectHitlagCallbacks, efLib_Update (0x8005BC50),
ftFx_SpecialHi callbacks and hsd_8039F05C. Arithmetic and callback ordering were
checked against retail assembly. No captured expected values were changed.

## Verification

Named directed tests: fire_contact_firefox_charge_hit_fd_marth_300_ticks_and_ordered_particles,
fire_contact_firefox_travel_hit_fd_marth_300_ticks_and_ordered_particles,
both fire_contact_bones tests, fire_contact_matches_retail_damage_and_recovery_scratch,
fire_contact_particle_simulation_fields_match_retail and fire_contact_allocation_budget.
Both recordings pass fighter state, RNG, ordered particles, all local bone
transforms, additional damage/recovery scratch and zero simulation allocations.

The particle-field check compares simulation state including AppSRT local
transforms and lifetimes. Camera/display caches written by psDispSubAppSRT are
outside that test; separate renderer oracles cover display calculations.

Full workspace debug and release each pass: 1,352 passed, zero failed, one
existing ignore. All-target clippy with warnings denied, formatting, 220 harness
tests and schema checks pass. Native macOS build passes. Stripped simulator is
3,960,712 bytes (+8 from 91a6f02); full performance census not rerun.
The OGL backend and native graphics smoke encounter the host IOSurface client
limit. A separate Null-backend probe matches all 300 canonical tick states of
the existing powershield recording; full particle/bone capture equivalence is
still being checked. Existing ignored captures are backed up outside Git.
