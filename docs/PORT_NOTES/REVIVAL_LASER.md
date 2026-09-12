# Revival invincibility and laser contact

A 600-tick Fox/Marth Final Destination recording follows Marth offstage, through
revival timeout, and into a Fox laser. At tick 512 the laser disappears while
Marth has 92 revival-protection frames, zero percent and zero hitlag. Full state,
RNG, particle and local-bone recordings are backed up outside the checkout.
Expected retail values remain unchanged.

The item hit path returns a typed contact outcome separately from whether it
logged damage. Revival invincibility still records victim history, emits the
invincible-contact spark and runs the item's contact response; it does not queue
a damage reaction or update the owner's stale-move history. This mirrors
ftColl_80077C60's contact branch and its conditional damage-log/stale call.

Five directed checks cover 600 ticks of pose, ordered particle fields, raw
fighter scratch, fighter/item/RNG state and zero simulation allocations. The raw
test also asserts neutral-special staling stays at 1 throughout this single-shot
recording. Capsule-specific invincibility, phantom contacts, simultaneous damage
logs and invincible-contact sound remain separate work.

Validation: full debug and release workspace gates each pass 1,443 tests, with
zero failures and one existing ignore. Workspace all-target Clippy, formatting,
225 harness tests and the native macOS build pass. The five directed checks are
included in both full gates.

The version 1 deterministic robustness corpus was rerun with this fix plus the
committed wall/ceiling and capture-release packets. Of 48 cases, 20 finish a
match, 17 reach the 6,000-tick limit and 11 reproduce faults. Initially 46 cases
faulted and two finished. Remaining faults group into fighter/item phantoms,
DamageFlyRoll, captured damage, ScreenKO's camera dependency and Reflector
ground departure. This is robustness evidence, not a retail exactness oracle.
Reports and replay inputs remain in /tmp/melee-corpus-v1-after-recovery-capture.

Work pauses at this commit at the user's request. DamageFlyRoll recordings and
source drafts are prepared but its implementation remains unapplied. No breadth
work has started.

Serial performance measurement: 3,995,872 stripped bytes and 3,653,632 text
bytes, unchanged from the capture-release commit. Duplicate-label counts remain
20 in melee-ft and 100 across crates. Load is 185.969 ms; 600 ticks take
26.359 ms. The existing size and tick-time limits still fail; thresholds and
baselines are unchanged. Full measurement is recorded in `../PERF.md`.
