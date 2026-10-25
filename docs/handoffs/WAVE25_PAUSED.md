# Wave 25 pause checkpoint — 2026-10-25

The user requested a pause and stocktake. All three agents are interrupted;
no Dolphin capture, Cargo build/test/benchmark, sweep or replay batch was
running at the pause check. Do not resume development without user instruction.
Only checkpoint documentation was updated after the pause.

## Integrated source and verification

MAIN before this documentation checkpoint is `7c06907b3deabff2054d5998506a8f83838889ac`.
Runtime Rust remains the wave-24 source `f570da10` (tree
`a21a201bfc9734964d51fced2885d4bbb49de923`). Later commits contain documentation
and the explicitly approved performance policy update, not new gameplay.

Last full coordinator checkpoint: `target/wave24/final-main/`, canonical
`target/sweeps/f570da10b93561b33a35501bc0e6c9ac01d81562.*`:

- 3,490 scenarios: 3,354 EXACT, 115 NOTCMP, 21 existing local failures,
  zero particle faults; zero changed rows against wave 23 and 287 fixes
  against startup `0a99db31`.
- 2,020 release workspace tests passed, zero failed, one existing ignore;
  m4 345 / m5 314; clean all-target clippy; 407 harness tests; schema checks;
  five expected WASI guard traps; 6,003 native/WASM hash lines; web check.
- Replay tuples unchanged: 108/108 complete with 964,782 matched frames;
  486/502 with 5,102,652 frames. Setup refusals remain distinct from stops.
- Inventory: `target/parity/wave24-combined/`; all-retail 29.4% / 38.1%,
  historical in-game 45.2% / 57.2%; 297 guards (278 unimplemented, 19 bail).
  The last two invariant proofs admitted no gameplay or covered-byte gain.

**No new wave-25 full sweep or union baseline has run.** The two newly generated
Flat Zone helper SCNs bring the local main scenario directory to 3,492; they
are unregistered, unsupported local work, not accepted new passes. Do not
mislabel the old 3,490-scenario checkpoint as covering new capture cases.

Fresh quiet performance at source `7c06907b` passed:
`target/wave25/performance-baseline/fresh/perf.log`, appended to `docs/PERF.md`.
Measurements: 7,675,616 stripped bytes, 6,897,664 text bytes, 54.833 ms load,
19.084 ms / 600 ticks; shared duplicate labels 20 / 94. User approved exactly
7,605,760 / 6,848,512 size baselines and Kirby allowance one. The +5% size
tolerance, timing and all other duplication limits remain unchanged. Existing
13 performance guard tests passed. Historical failed reports are retained.

## Fresh capture evidence — not implementation acceptance

The user authorized new captures and necessary boundaries on 2026-10-25.
Authority is retained in `target/wave25/recording-authorization.json`; it does
not override the current pause. Eight successful chains total **9,725 ticks**.
Existing data, including failed attempts, remains intact; no game data is in
Git. Worktree agents must read MAIN data in place, never copy or symlink it.

| Prefix | Full ticks | Actual evidence and current limit |
|---|---:|---|
| `w25_ness_yoyo_overlap_v02` | 4775 | All five streams, genuine initial seed 512840711, 200210 LCG draws; original full raw/expected/ledger and 3340 particle/meta prefix reproduce except one host-VI-only row. Binding replacement 877, selected removal 924, older cleanup 925. Frozen port diverges at 924 by removing both. |
| `w25_heavy_lift_f_600_v01` | 600 | Five streams and full raw/decoded bones; 25675 LCG draws. First pickup interrupted at 301, later attachment 388 and LiftWait 423; no carry or throw in this case. |
| `w25_heavy_lift_f_800_v02` | 800 | Five streams and full bones; 32125 LCG draws. Same genuine initial seed 2394518020; LiftWalk 175 at 450, LiftWait at 494, HeavyThrowF 104 at 520, Wait at 559. Original 295-tick prefix retained. |
| `w25_sword_lw_land_750_v01` | 750 | Five streams and full bones; 32949 LCG draws. Held common down air 69 at 441, LandingAirLw 74 at 479, CliffCatch 245 at 481, CliffWait 246 at 489. No unlocked airborne common-69 row. The conditional throw-at-470 draft was NOT recorded and is not a valid witness. |
| `w25_redead_fall_700_v01` | 700 | Five streams and full bones; 27884 LCG draws. Item motions 10/11/2/4/5/6; coupled fighter CaptureLeadead 333 at 517, CaptureCut 229 at 638, Wait 668. Item remains live at 699; no full lifetime claim. |
| `w25_kirby_sheik_ground_v01` | 900 | Five streams and full bones; genuine initial seed 3825146497 / 32929 LCG draws. Hat acquired 346; ground start/loop/end, six-needle charge/volley and second short release; no articles remain 690–899. No air, LR-cancel, copy-loss or projectile-contact proof. |
| `w25_start_fz_fox_marth4_v01` | 600 | Helper capture, new savestate and warm/cold SCNs. Five-stream structural completion; expected cold gate explicitly refuses unported Flat Zone. Legacy helper cadence is not a per-tick rendering proof. New save/sidecar backed up exclusively to `~/melee-data/roms`. Not registered. |
| `w25_fz_start600_v01` | 600 | Per-tick rendering; five streams/full bones and camera, genuine initial seed 2265280010 / 4363 LCG draws. Camera raw pass equals full tick raw; all 600 camera records present. No simulator stage/camera/bone parity or later hazard proof. |

Detailed evidence, scripts and hashes:

- `target/wave25/ness-capture/{capture-accepted-v02.md,complete-validation-v02.json,old-prefix-comparison-v02.json,triage-v02.log,completed-output-manifest-v02.json}`.
- `target/wave25/breadth-capture-validation/`: report, three full validation
  JSONs, output manifest and frozen-CLI triages; `heavy-followup/` contains
  the 800-tick draft, manifest, validator and completed validation JSON.
- `target/wave25/copied-sheik-validation/{report.md,stream-audit.json,raw-timeline.json,item-instance-episodes.json,triage.log}`.
- `target/wave25/flat-zone-validation/`: full validation JSON, camera
  validation JSON, scripts and logs. An initial camera validator assumed
  every field was a string; corrected to the source-defined subject list,
  preserving the failure log and unchanged recorder data.
- `target/wave25/copy-stage-capture-plan/`: queue scripts, launch logs,
  successful action JSONs and source plan. Helper backup was verified;
  missing existing Sheik backup metadata was added exclusively after
  matching the original savestate hashes.

The first Ness v01 attempt hit macOS LaunchServices restrictions and wrote
only failure logs; preserve them. v02 used fresh names with required access.
UV cache permissions were resolved using task-local `target/uv-cache` and
`UV_NO_SYNC=1`. Repeat fresh-prefix checks before any future recording;
recorders can truncate same-named outputs. Never blindly rerun used prefixes.

## Preserved worktrees

All start at `f570da10`; rebase onto current MAIN before final verification.
All are paused. No wave-25 runtime commit has landed.

| Worktree under MAIN `target/worktrees/` | Branch | Actual pause state |
|---|---|---|
| `wave25-ness-current-instance` | `codex/wave25-ness-current-instance` | Nine modified tracked Rust files, uncommitted, unbuilt and untested. |
| `wave25-redead-lifecycle` | `codex/wave25-redead-lifecycle` | Clean; source investigation and capture analysis only. |
| `wave25-kirby-sheik` | `codex/wave25-kirby-sheik` | Clean; source investigation and capture analysis only. |

**Ness:** draft exact-instance request/control, static creation/retirement and
current-target hooks, plus preallocated per-instance string storage. Actual
retail clears the current binding unconditionally when an article ends; do
not invent an identity-comparison suppression. Same-batch pending creation
routing was drafted but has not compiled or passed its proposed regression.
Raw pointer-field/schema assertion work and API allocation/clone tests are
not finished. `target/wave25/ness-implementation-prep/plan.md` and the actual
worktree diff are the recovery sources. Review before building or accepting.

**ReDead:** whole 700 requires coupled common CaptureLeadead, not an item-only
patch. Audit literal entry/timer/mash/collision, attachment/release, callbacks,
ECB, matrix and effect ordering. Root owns independent review; no other
agent was assigned it. Preserve unsupported contexts and original window.

**Copied Sheik:** use copy-owned ground callbacks in ft-kirby and shared
it-seak item logic, without a character-crate dependency or speculative full
family rewrite. Retail charge color/bone/emission differ. Held kind153 changes
in place to thrown152; a recycled GObj address later represents a distinct
presence episode. Preserve these identities in tests, rather than treating
an ItemID as a fighter index. Ground proof alone must not admit air/cancel.

## Resume checklist — only after user instruction

1. Inspect all worktrees and current MAIN first; preserve unfinished edits
   and every original/new capture. Do not resume agents merely to summarize.
2. Establish the current scenario manifest and explicit baseline provenance;
   the production f570 baseline covers 3490 original cases, not the fresh
   captures. Keep immutable old artifacts unchanged. Any new source-keyed
   or union sweep must be named separately and include fresh SCN hashes.
3. Continue only the three bounded ports, release-only with incremental off
   and modest job counts. No new capture is required for their currently
   witnessed paths. Additional fresh captures remain authorized if needed,
   but wait for the user to end this pause.
4. Require whole original windows, strict particles, genuine RNG/ordered
   sites, raw identities/callbacks, meaningful API allocation/clone and
   negative controls; never truncate, alter expectations or weaken guards.
5. Rebase successfully, run relevant checks, then build the normal CLI and
   freeze source/scenarios/owned notes/executable before authoritative full
   sweep (absolute shared lock) and both replay tuple comparisons. Root
   reviews, picks and gates accepted commits; no push is authorized.
6. Heavy carry/throw and Sword continuation now have evidence for a later
   bounded port. Flat Zone remains unported; its later hazards and the other
   18 unsupported stages remain in scope. Current inventory percentages do
   not increase merely because captures or untested worktree edits exist.
