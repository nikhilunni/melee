# Wave 15 integrated breadth

2026-10-24. The three bounded changes are integrated: owner transfer
`8c594ca5`, held Sword ground inputs `aba6e931`, and container blasts
`af5f2587`. Production source is
`af5f258722204a731d0f809fe8f6365af2a6a02b`; subsequent commits through
`58118743` add combined regression coverage. This is progress toward 100%
retail parity, with breadth still taking precedence over replay depth.

## Accepted behavior and evidence limits

- **Barrel owner transfer:** an initially unowned REST barrel can take the
  primary fighter owner of the article that struck it. The two complete
  captures end at contact: they prove owner/team and attack ID1/instance0,
  not every later transferred-barrel damage-attribution path. Previously
  owned recipients, nonresting transfers, follower ownership and articles
  without a primary fighter owner keep their guards. See
  `BARREL_OWNER_TRANSFER.md`.
- **Ground Sword inputs:** 23 complete captures prove independent C-stick
  throw detection, main-stick direction priority, ordinary down-tilt/up-smash
  entry and existing throws. Only actual common rows AttackLw3 (57) and
  AttackHi4 (63) gain held-Sword admission. These captures end at entry;
  private rows, aerials, LandingAir, Hi3 and Lw4 remain guarded. The private
  Game & Watch row345/identity57 does not inherit the common-row admission.
  See `HELD_SWORD_GROUND.md`.
- **Empty Box/Barrel blasts:** complete lifecycles use the normal script
  and hitbox path with the issuing callback's context retained across
  deferred Resume. Early/late hitbox refresh and immediate efAsync context
  are restored after the continuation. The eight complete captures cover
  34,220 ticks, including the primary 4,711-tick Box and 4,669-tick Barrel
  recordings. Unrelated thrown/dropped/sliding/floating, carry and ownership
  guards were not widened. See `RANDOM_ITEMS.md`.

After the combined merge, two recordings that previously reached a later
held-item AttackHi4 guard also pass completely. They join the item family,
which now has **37 strict witnesses**; the full field/particle/ordered-RNG
group passes in 19.59 seconds. Expanded actual API allocation/capture checks
also pass. Root compiled and checked focused item/character and full-human
cases after each pick. The subsequent combined release checkpoint also passes, as detailed below.

## Combined current results

Root evidence lives under `target/wave15/main/`:

- `sweep.tsv` / `sweep.log`: all 3,485 scenarios, zero regressions and
  **35 fixes** against `bf98d61d`; 3,290 particle-exact, 115 not comparable,
  79 existing local failures and one existing particle fault.
- `sweep-vs-startup.log`: zero regressions and **228 fixes** against
  `0a99db31`. Local failures and incomplete recordings remain distinct from
  accepted exact witnesses.
- Both replay JSONLs and comparison logs retain every path's category and
  matched count: **108/108**, 964,782 frames; **486/502**, 5,102,652 frames.
- Fresh inventory: `target/parity/wave15-integrated/summary.md`, 4,202
  ported, 201 partial, 1,361 body, 1,378 row, 221 referenced and 12,464 not
  started; 252 fail-closed sites (233 unimplemented, 19 bail). Rounded
  all-retail 29.2% / 37.9% and historical in-game 44.9% / 56.9% are unchanged.
  Some classification shifts reflect comment and guard attribution changes,
  rather than changed runtime behavior. This is a citation/byte proxy.

The combined release workspace passes **1,940 tests, zero failures and one
existing ignore**: m4 345, m5 297. Workspace clippy is clean; harness 407,
schema layout checks, WASM pure suites and four expected spline traps pass.
Native/WASM hashes match over 6,003 lines and the release web check passes.
The full runtime is unchanged after `af5f2587`; the later regression commits
and the 37-case focused item group retain the two interaction captures.
The quiet final performance measurement at `58118743` passes timing:
55.729 ms load and 18.368 ms per 600 ticks. Shared-code duplication passes
at 20 melee-ft labels and 94 across crates. The gate still fails unchanged
size ceilings (7,605,840 stripped bytes; 6,848,512 text bytes) and Kirby's
existing one label against baseline zero. The exact report is appended to
`docs/PERF.md`; no budget, timing limit or other allowance was changed.

## Next bounded work

Party Ball FALL/REST uses the existing left-facing 600-tick capture; opening
and contents remain separate guarded branches. Moving Kongo barrel work
must restore the saved live AObj clock independently of the controller's
future rate, then prove actual held positions and launch behavior. Its plan
is `target/wave14/kongo-search/moving-barrel-implementation-plan.md`.

Monster births, heavy entries and held aerial entries do not prove their
later callbacks. Keep those guards while pursuing complete existing
witnesses. No new recording or protected-data write is authorized in this
task. No performance baseline increase or push is authorized.
