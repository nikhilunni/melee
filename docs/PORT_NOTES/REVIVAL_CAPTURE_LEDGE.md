# Revival, capture and ledge continuation

This packet extends Fox versus Marth on Final Destination. The directed checks pass; it does not close airborne grab release, occupied ledges, wall/ceiling
recovery, slow ledge options or match timer outcomes.

## Ownership and transitions

- Revival timeout grants the remaining invincibility before entering Fall
  (ftCo_800D5738). Shield+A follows the retail digital dodge / analog aerial
  priority after an actual scene item search establishes no pickup candidate.
- A grab can catch a jumping victim in CapturePulledHi, then immediately map
  back to the grounded counterpart. Capture alignment and the captor accessory
  correction follow fn_800DA054; the latter retains small nonzero ground speeds.
- C-stick ledge attack, escape and outward drop use crossing predicates and
  retail IASA order. CliffWait expiration enters DamageFall without inventing
  DamageState in the retained motion union. Slow rows are source-ported but
  still need recorded high-percent witnesses.
- Fighter_procMap publishes the collision-adjusted position after its callback
  (8006C27C). Omitting this left the root joint at the pre-snap ledge position
  even when canonical fighter positions matched.
- Revival explicitly requests no interpolation (800D50E4/800D50E8, anim_blend
  sentinel -1). The authored six-frame blend incorrectly softened the first
  Rebirth pose. The outgoing blend-exit joint predicate still uses the authored
  blend byte, as fighter.c:1299 specifies.
- Revival retains dynamic-bone locks, spring state and secondary joint poses.
  Fighter_Create's chain initialization (80069018) runs only at creation;
  Fighter_UnkProcessDeath does not repeat it or enter Wait/Fall before Rebirth.
- Ledge-timeout provenance (2227.b1) survives death reset. Only grounded motion
  entry clears it; rebuilding all status fields from defaults erased it early.

## Test correction

The new raw ledge test initially compared 2344/2348 during CliffCatch. Retail
CliffCatch initializes only the ledge ID at2340. ftCo_8009A804 initializes the
wait timer and neutral latch on entering CliffWait. The preceding motion's
inactive union words are not values of the typed CliffState placeholders.

The corrected test always checks the ledge ID and begins checking the timer and
latch after CliffCatch. It retains all valid checks through later ledge states
and timeout. No recorded expectations were changed. AGENTS.md and CLAUDE.md
now explicitly require immediate evidence-backed repair of implementation or
test defects without a confirmation pause.

## Recorded witnesses

Seven capture/revival scenarios cover both-character timeout, three Fox
shield+A schedules, and both orders of a transient airborne grab. Ten ledge
scenarios cover four C-stick schedules for each character and both ledge wait
timeouts. Tests compare fighter state, RNG, ordered particles, all captured
local joint transforms, additional raw scratch and allocations. These captures
are local ignored game data backed up outside the repository.

New Dolphin captures and native graphics smoke remain blocked by the host
IOSurface client limit. Existing recordings remain usable for headless checks.

## Verification status

Full debug and release workspaces: each 1,345 passed, zero failed, one
pre-existing ignore. All
17 new scenarios pass their fighter/RNG/particle, local-SRT and allocation
checks; raw capture/revival and ledge checks pass. All-target workspace clippy passes with warnings denied.
Harness: 220 passed; schema and formatting checks pass.

Native macOS build passes. Stripped simulator: 3,960,704 bytes, an increase of
16,856 bytes from e5db5fd. No new full performance-census claim.
