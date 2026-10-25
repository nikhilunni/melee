# Wave 14 integration checkpoint

2026-10-24. All six incoming wave-13 branches are integrated. The final
runtime source is `bf98d61d227f916fd01ab5b74a8f94ad20dde183`; later
documentation commits do not change its behavior. The full task remains
100% retail parity, with breadth before individual replay stops.

## Accepted behavior and boundaries

- Grab/common edges: four retail witnesses, 21,580 ticks. Incomplete
  particle dumps remain explicitly not comparable; ordered RNG is checked.
- Kirby: Nayru's Love, Fire Breath and Ice Shot complete 24 of 25 copied
  neutrals. Sheik start boundaries do not prove Needle Storm. Giga Bowser's
  copied fire breath remains guarded.
- Match flow: witnessed FFA/teams and doubles are integrated. Stock sharing
  and timed-team ranking remain guarded. Replay setup refusal does not
  count as a completed game or a diagnosis of its original divergence.
- Items: two-player FD/Battlefield random-item setups and witnessed
  Capsule/Box/Barrel paths. Empty blasts, Party Ball, heavy fighter rows and
  other unwitnessed branches remain guarded at this checkpoint.
- Kongo Jungle/Jungle Japes: witnessed stage construction and interactions;
  unwitnessed barrel/log/hazard paths remain guarded. Partial RNG ledgers
  retain explicit prefix assertions rather than a full-ledger claim.
- CPU: 95 strict witnesses, static character-owned profiles and persistent
  rapid-jab counting. Wider recovery/item/stage branches remain guarded.
  Slippi lacks CPU levels, so CPU replay admission remains unsupported.

## Checkpoint corrections

One primary Ice Climbers/Fox CPU boundary was saved before fighter creation
finished, with Entry script PC `0x5ACB5ACB`. Its warm import is rejected
before script lookup. The invalid warm boundary was removed from automatic
explorer selection; both scenario files and all recordings remain intact.
The strict cold witness is still registered. All 165 registered boundaries
pass the cold-construction test.

The v3.13 replay fixture still has a zero-frame admission floor because its
items/damage setup is unsupported. Its expected diagnostic was updated to
the current setup predicate; no admission rule or frame floor was loosened.

WASI aborts on panic, so its four guarded spline operations are verified in
separate subprocesses. Each must execute its named test, fail, and print the
specific missing-witness diagnostic. Native tests use `should_panic` with
the same expected messages. Production spline behavior is unchanged.

The performance cleanup shares the default CPU callback and common empty
callbacks across character tables. Sound fallback remains lazy. The emitted
census is back to 20 duplicate labels in `melee-ft`, 94 across crates. Timing
passes, but size and Kirby's existing one-label allowance still exceed the
unchanged user-owned budgets. Measurements are appended to `docs/PERF.md`.

## Evidence

The post-cleanup full sweep and both replay corpora are published under
`target/sweeps/bf98d61d227f916fd01ab5b74a8f94ad20dde183.*`, with a provenance
file distinguishing the tested source from concurrent documentation edits.
There are zero regressions against the integration baseline; versus wave-14
startup there are 193 newly passing scenarios and two newly complete replay
games. Existing unregistered local failures are not accepted divergences.

Full checkpoint logs are `target/wave14-checkpoint-*`; agent acceptance and
search reports are under `target/wave14/`. `TRACKER.md` records final test
counts, inventory measurements and the next bounded breadth tasks. Incoming
`docs/handoffs/` notes describe historical branch state; verify their open
faults against current source before assigning them.
