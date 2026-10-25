# Bottom death of a pinned mouth victim

Retail `ftCo_800D3BC8` calls cleanup at0x800D3BE8 before changing motion at
0x800D3C14. Cleanup's0x800D33F8 call to `ftCo_800DD100` parts the pair, resolves
the victim's constrained release and settles its partner. The quake and later
explosion read the released position; the explosion reads current position at
0x800D3CF0..3D04 after the motion change.

The ordinary Update caller previously had only one fighter, so it deferred pair
release until after death entry. That path refuses a release that moves the
victim. The new paired Update borrow supplies a concrete mutable captor, its
assets and the existing map. Only bottom death of a pinned, non-pulling
CaptureKirby victim uses the new cleanup path. Both links are consumed at the
retail cleanup point, with no later scene release. Existing release_thrown FMA,
collision sweep and floor logic are reused. Character death callbacks run first;
velocity/item/effect cleanup and the death motion follow the pair release.
Side/top, pulling, constrained-captor and missing-context guards remain. The
partner-vanish puff and its existing path remain unchanged.

## External constraint cache

An external RObj target refresh is not an invalidation of the constrained joint.
`lb_8000B1CC` tests that joint's own dirty bit before rebuilding it (retail
0x8000B244..0x8000B26C); target resolution is inside the rebuild. The scene
previously sampled the external target and dirtied the owner before every proc,
so the death's Update read a prematurely refreshed XRotN. The new HSD helper
refreshes the snapshot without invalidating an already installed constraint;
install/clear and normal animation/root invalidation retain their old behavior.
The external scene snapshots use it; genuine attachment/escape/throw callbacks
retain the dirty setter. The scene's thrown_request is an own-pose write too,
whereas pre-proc and held-item target observations are snapshots. The sampler
must not traverse the anchor while the constrained owner is clean: that query
can itself rebuild the anchor cache. Orientation is unchanged.

Normal HSD_JObjAnim CheckDepend retains retail's active positional RObj term:
an installed positional dependency dirties its clean owner at that actual
animation point. It does not globally invalidate on every target refresh.
The same-proc attachment boundary is covered by immediate entry graphics:
retail efAsync_Spawn starts at0x800676F0, its immediate branch is0x80067938..68,
and QueueProcessDeferred is0x8006744C. Falcon's Catch entry emits positional
model0x41D on bone61 before attaching its XRotN to a grounded victim. Both
participants now inherit and restore the actual Grab callback's immediate effect
context; entry commands capture their matrix before the later attachment.
The generic positional-model flush uses the saved matrix without eagerly
querying the current joint again. Other live attachment paths remain unchanged.
The provisional post-select geometry refresh was removed.

The regression exercises real0x41D model creation: an issued matrix performs
zero owner queries, and an unresolved request performs one. Entry tests cover
immediate/deferred snapshot ownership, retained issued matrices after a pose
change, and an actually hidden queued command at its next animation step.
A sampler probe demonstrates that refreshing a clean constrained owner does not
rebuild a dirty anchor; after the owner's own CheckDepend it resolves that anchor.

The throw escape's actual release query also receives the borrowed thrower
assets. Its Animation callback can dirty XRotN after the scene's observation;
refreshing immediately before release_thrown supplies the current RObj target
without invalidating a clean owner. Nonperturbing query/backtrace diagnostics
located this endpoint at kirby_throw_f_fd_fox4 tick318. The seven complete560tick
throw/escape witnesses remain mandatory. The diagnostic eager-snapshot rollback
is preserved as diagnosis only, not accepted as retail equivalence. The source
inventory of nine release calls is in target/wave17/pinned-death; this is not a
claim that the pre-proc observer implements a universal lazy resolver.
The new bottom cleanup, mouth escape after its own scale write, and Koopa
set-down after CatchCut likewise refresh at their actual release query with both
borrowed peers and asset sets, with no intervening pose mutation. These are
noninvalidating observations: a clean held-point cache remains untouched.
Existing optional immutable and guarded late-death paths retain their contracts.

The cache regression checks installation invalidation, a clean cached matrix
surviving external refresh, a new target becoming visible after own invalidation,
and clear invalidation. It uses no game data.

## Existing capture and proof boundaries

`kirby_inhale_randall_pit_ys_fox4` has1500 sequential retail rows in all four
streams. Fox is pulled at1347, held at1353, dies1358 at
(-63.28199005126953,-92.9803237915039,0), and Kirby falls1358 then dies1359.
They revive1418/1419 and wait1478/1479. The named m5 test covers all1500 fields,
items and particles, ordered particle RNG sites and ledger-cadence seeds. The
raw global RNG values are separately checked as an ordered LCG chain; that is
not a port-vs-retail comparison of every nonparticle call-site identity.

The two complete4097/3501tick Falcon Dive captures also check ordered raw LCG
values and every ledger-cadence port seed. Their tick0 draw is not an empty setup
observation: the independently recorded start_fd_kirby_captainfalcon4 particle
initial state supplies seed4055080798 (validated against metadata, sav.json and
saved RAM), and its first draw is3212490921. This proves the first edge without
inverting the LCG or deriving an expectation from the port. Both SCNs must reuse
that boundary without a replacement/online RNG override. The old generic
no-setup-draw audit remains unchanged for its original fixtures.

The raw-state regression compares both fighters' actions, positions, actual
root-motion pinned bit and grab-link presence on every row through both
revivals. A probe using the actual live captured pose verifies cleanup uses
XRotN rather than a pre-release selection position, consumes the links once,
and queues ArticlesFreed/DestroyOwned before CameraQuake/Death at the released
position. The actual public API test steps/captures the complete scene without
allocating and clones a live pinned victim, checking independent continuation
through death and revival.

The separate bone recording is only15 rows1345..1359, with its own1360row raw
clock. It supports the clean constrained-joint cache investigation; it is not a
full1500tick bone proof. The dedicated loops explicitly apply their own raw
clock's display cadence; the generic TestSimulation wrapper alone does not. The broad bones diagnostic compares live retail DFS indices against port part
indices, including empty conditional slots. PlCo's table and ftParts_SetupParts
prove the resulting Kirby numbering mismatch: raw DFS10/14/27/36/42 correspond
to port parts14/20/40/49/55. Direct mapped assertions compare the nine newly
reported rotation/scale words over all15 rows; all match, including six actual
scale words3F666600 at1358/1359. Existing CLI reports are preserved, not treated
as exact. Certified be2 baseline and final candidate have identical reported mismatch
words on all13 pre1358 rows. Their aggregate matrix comparison counts differ
at1353..1357 (156 versus216 words;107 versus102 port-dirty joints), so the
complete reports are not byte-identical. An identical temporary read-only export on the source-certified be2 baseline
and final candidate identifies exactly Fox parts2,3,4,21,22, with no Kirby index
changes. Fox has no conditional virtual slots. Over1353..1357 all five retail
joints and candidate joints are clean; all300 world-matrix words match retail
exactly. Baseline cached words equal the candidate but baseline flags are dirty.
The temporary example only reads cached fields and was removed; production and
test hashes and the preserved oracle executable remain unchanged. This does not
establish whole Kirby bone exactness.
Final oracle,
cross-section, sweep/replay logs and provenance live under the coordinator's
`target/wave17/pinned-death/`.


## Final validation

The final candidate is based on12dc5ce5. Its preserved normal release executable
is target/wave17/pinned-death/final12dc-melee-sim, SHA256
ee51a7aac6bc00549978ae59867284f85508243bcf15c4fdb072833ee9d1cf9d.
It was built after the tests and release all-target clippy completed. The3490
MAIN scenario name/content union was used in place. The final sweep has zero
regressions and one fixed capture versus12dc:3314EXACT,115NOTCMP,60existingFAIL
and one existingFAULT. The same result versus77c3ef37 has zero regressions and
five fixed existing captures. Both replay JSONL outputs are identical per path
and every result field to the12dc baselines:108/108 complete with964782 matched
frames, and486/502 complete with5102652 matched frames.

Final focused checks pass: whole1500 strict/raw, partial15row bones, both whole
Falcon captures, complete Kirby and Bowser groups, six air-capture releases,
moving-platform capture/throw, real0x41D effect and entry timing, HSD cache
contract, public API pinned-live-clone step/capture allocation, and the newly
merged item-program API allocation regression. Eight complete CLI death/partner/
edge captures also pass. The broader touched-crate attempt before the final
endpoint additions had267 passes and33 missing-worktree-data failures across12
legacy targets; no missing-data optout was used. Coordinator MAIN checkpoint
coverage is required for those hardcoded paths and the full workspace.
