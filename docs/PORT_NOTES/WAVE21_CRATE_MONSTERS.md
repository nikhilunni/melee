# Crate-monster constructor boundary

This admits only the existing crate `chk=false` births: Kuriboh (kind43) in
both witnessed facings, and left-facing ReDead (kind44). Their next ordinary
Animation, Physics and Collision callbacks explicitly refuse. Pickup, drop,
throw, damage, destruction and Kuriboh's installed jumped-on callback remain
unavailable. Other monster kinds, ordinary spawns without required constructor
context, and positive-facing ReDead remain guarded. No later lifecycle is
inferred from a terminal birth.

## Retail construction and scheduling

`it_8026F6BC` checks the live hold-kind6 limit, draws odds and the monster
weighted table, then calls `it_8026F5C8`. Its first facing query uses the
stage-plane previous position. The constructor makes a distinct live query
using the collision-resolved current position. A tie can draw at each actual
call; no answer is cached or query eagerly evaluated for ordinary kinds.

The static ItemLogic row owns a required before-query/after-query pair. A thin
static wrapper shares the concrete pool initializer and common finish, refuses
missing context before allocation, and preserves the contextual retail order:
`it_80275E98` collision, root translation, initial spin, `it_8027B730` shared
scratch/push-flag writes, live query, kind setup, then `foobar` common spawn
intangibility. Ordinary kinds keep their existing initializer/callback order.
The common scene postspawn tail is compiled once and retains normal scheduler
registration. The contextual path does not repeat initial collision afterward.

Kuriboh follows `802D73F0 -> 802D747C -> 802D848C`, entering motion0 without
an article animation and installing `802D8098`. ReDead follows `802E8BCC ->
802E9308`, entering motion10/article1 through normal `change_motion_with` and
its one constructor pose step. Its actual article1 script is NULL in both
available regional ItCo archives. Unsupported row scripts are not activated.
ReDead's owned hurt-offset/radius and ECB snapshots are retained; unwritten
union words and stale pointer/storage bytes are not invented as semantic state.

The typed asset boundary reads the public+8 bank (slots0/1), independently of
the public+4 common-item bank. ReDead attr+1C is signed16, despite neighboring
float and packed fields. The raw regression caught and fixed the prototype's
incorrect float-word interpretation. `it_8027C56C` uses double multiplication
then frsp at 8027C668/66C: DOL @210 at804DC808 contains 3ff921fb54442d18,
exactly the Rust PI/2 constant. ReDead X scaling is separate fmuls802E9338;
launch is fsubs8026F760 then separate fmuls76C/770, without FMA.

The damage14 parent appends ordinary child procs after equal priority. New
child damage14 and generic16 execute this frame; earlier links await the next
tick. The raw regression inspects actual scheduler visit tags after the normal
tick, not fabricated tags or table presence alone. The child has no pending
event; its parts and dynamic-bone counts are zero. No newborn skip, timer,
filename/seed admission rule or scheduler exception is introduced.

## Complete existing evidence and limits

| Original scenario | Rows | Terminal birth |
|---|---:|---|
| corpus_v3_fd_items_sword_e266e1150_p1 |1874| Box -> left Kuriboh at1873 |
| corpus_v3_fd_items_sword_e4abe72ec_p2 |297| Barrel -> left ReDead at296 |
| corpus_v3_fd_items_sword_e5ca12c3b_p0 |595| Barrel -> right Kuriboh at594 |
| corpus_v3_fd_items_taru_eae02b66b_p2 |285| Barrel -> right Kuriboh at284 |

All five original streams are complete, totaling3051 rows each. All child
observations are the final row; there are no later recorded rows. Scenario
headers now describe those actual retail facts; parsed TOML is unchanged.
The raw regression consumes the raw stream's actual display cadence, compares
all-row fighter motion/position and live item identity/state/position/velocity,
and checks constructor-owned scratch, flags, callbacks, timer and NULL owner.
This is selected semantic raw-state coverage, not whole Item byte-layout or
bone-matrix equality. No monster bone recording exists in this slice.

The strict group retains all original fields, items and particles. Separate
ledger-clock runs compare every ordered particle call site and every port
end seed. Independently recorded initial, metadata and boundary seeds cover
the genuine row0 setup draw; all127256 raw global LCG edges and result-store
PCs are checked. There are125405 ordered particle draws across the four
ledgers. This does not globally instrument every nonparticle port RNG call.

The actual public API test measures step, observe and presentation capture,
including the terminal birth, with zero allocations after initialization. A
clone is constructed before birth outside the counter, then both matches enter
independently. The next actual API step refuses the unported Animation. No
later retail continuation, whole model/bone oracle or allocation-free clone
construction is claimed.

## Presentation ownership

The two new kinds register Arc ownership of the existing ItCo archive at
initialization and use the existing free-item `ModelSource::Item`/`ItemPose`
pipeline. It preloads authored rest/state trees; Kuriboh's negative animation
uses rest and ReDead uses article1. No fighter-owned `MODEL_COPIES`, new pose
math, cache algorithm or tick allocation is introduced. The public API test
requires a newly populated instance group at the actual born position, with
visible nonempty vertex/index geometry, and equal cloned instance matrices.
This proves public draw submission, not pixel or retail bone equality.

Independent archive inspection found ReDead article1 has33 joint nodes and77
rotation/translation FObjs, no DPtcl/event/path/RObj animation, and NULL material,
shape and script. Its constructor hurt/ECB snapshots precede the state change
and do not demand a posed joint. Lazy gameplay article-state/step bookkeeping
therefore misses no observed constructor event; those counters alone are not
claims of eagerly evaluated JObj matrices. Generic presentation evaluates the
authored pose through its existing pipeline. No monster bone recording exists.

## Validation provenance

Completed headless-only proof remains historical under
`target/wave21/crate-monsters/`. Final amended visual proof is recorded separately
under `target/wave21/crate-monsters/visual/`, against post-Sword baseline
`b029dc45d2d3b20a2abed12eb7bf2289c59008cb`. The coordinator supplied a fresh
full sweep and explicitly documented same-tree worker replay aliases. The final
normal CLI was built after all test/clippy compilations and copied separately
(SHA256 `5995205fe0043d46d16bba4bd649ab061770e9ec0af4f1b12a2aa8db02485891`).
The3490-scenario sweep reports3332EXACT,115NOTCMP,42existingFAIL and1existing
particleFAULT, zero regressions and exactly the four planned fixes in117s.
Both replay corpora preserve every path/category/matched/frames result:
public-v3.7 completes108/108 with964782 matched frames; public-v3.7-b completes
486/502 with5102652 matched frames. All610 results are unchanged. Source and
scenario manifests and the fixed binary hash were rechecked after completion.
The focused item/raw/phase/strict/API and shared Heavy5/random37/PartyBall/
Barrel-owner/Sword23/item-contact/human3 cross-sections pass. Post-rebase Sword
raw2, its seven complete new captures, existing aerial catch and live-clone API
also pass. Workspace release all-target clippy passes.
Prototype failures remain preserved; retail expectations were unchanged.

The final touched melee-lib test binary ran without optout:204passed and9failed
on legacy direct worktree paths for protected disc/trace data. Each failure
names the unavailable worktree path; focused tests read MAIN data in place.
The coordinator must cover those9 tests from MAIN during its combined checkpoint.
No worker claim of full workspace, WASM, web or performance acceptance is made.
