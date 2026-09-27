# Project tracker

Status legend: `[ ]` todo, `[~]` in progress (add date), `[x]` done,
`[!]` blocked (say on what), `[-]` out of scope for now.

Keep this file honest. A task is `[x]` only when its tests pass in
`cargo gate` and it is committed. When you finish a session, update the
task lines you touched and add one line to the session log.

## Library extraction (2026-09-10)

- [x] `melee-lib`: implementation complete and verified; committed at user request. Public create/step/inspect/clone API, shared assets, exact cloning, and both headless examples delivered. Debug/release gates each 1,144 passed, 0 failed; clippy and allocation gates clean. Final perf timing/duplicate limits pass; only binary-size ceilings fail, explicitly deferred by the user. See `docs/PORT_NOTES/MELEE_LIB.md` for files, measurements, and limitations. macOS/wgpu consumer is the next milestone; no app work included.

## Native graphical consumer (2026-09-10)

- [x] First native visual prototype: shared archive mesh/texture decoder, allocation-free presentation capture, wgpu renderer, session/C ABI, and Swift/AppKit window. Focused skinning, presentation-allocation and session tests, clippy, native build and resize/focus/pause/keyboard smoke pass. Full release gate passed before final presentation/focus changes; debug workspace run stopped at user request in favor of focused app checks. Basic Fox/Marth/FD rendering only; visual effects, materials and camera fidelity remain follow-up work. See `docs/PORT_NOTES/NATIVE_APP.md`.

- [x] Rendering fidelity pass (2026-09-10), committed as `a4487fe`: common multi-texture color/alpha operations, authored sampling/transforms and pixel state, background/world compositing, living-fighter camera framing, live stage poses, and instanced original laser meshes. Shared character archives avoid duplicate article data. Focused release checks (16 tests), clippy, native build/smoke and Metal preview pass. Custom TEV/material animation, precise transparency, held weapon poses and combat effects remain; see `docs/PORT_NOTES/NATIVE_APP.md`.

- [x] Custom texture-combiner phase (2026-09-10): decoded HSD texture expressions, shared GPU arithmetic/comparison stages, and ten numeric Metal fixtures. Descriptor, real-asset and presentation checks pass; clippy and native build pass.

- [x] Particle/shield presentation phase (2026-09-10): shared GX pixel decoding for stage/common/Fox/Marth particle banks, immutable atlas resources, allocation-free sprite capture, and procedural shields scaled by the live shield bone. Capture reads particle color/AppSRT state without advancing simulation. Particle suite, focused rendering tests, both complete-match capture/allocation checks, workspace clippy, native build/smoke and Metal previews pass; specialized particle geometry, model effects and precise GX rendering remain.
- [x] Laser/shield gameplay gap discovered during rendering verification: resolved in `4e08fb7` with mature/light/airborne/grazing shield-contact oracles. See `docs/PORT_NOTES/MATCHUP_REPLAY_AND_SHIELD.md`.

- [x] Authored lighting phase (2026-09-10): shared light-table parser, tick-indexed Final Destination directional light paths using the existing AObj and linear-spline evaluators, per-vertex ambient/diffuse/specular shading, and inverse-transpose normals. Capture-frequency/reset, both complete-match presentation checks, static Battlefield lights, 13 numeric Metal fixtures, workspace clippy and native build/smoke pass. Material/texture animation, held weapons, specialized/model effects, shadows and camera/framebuffer fidelity remain.

- [x] Shared material/texture animation and FD fade state (2026-09-10): `24a5810` foundation, `0d5b938` live stage/GPU animation, then shared color-overlay playback and actual scheduler map transitions. All 17 phases complete, including the two request-time overlay evaluations; debug/release cycle checks, 27,000 allocation-free ticks/captures/cloned continuations, both exact full-match oracles, scheduler tests, clippy and native smoke pass. Pixel/camera fidelity remains unverified; held weapons, model/specialized effects, shadows and camera/framebuffer work continue.

- [x] Perspective/framebuffer phase (2026-09-11): shared mesh/particle camera, perspective shield depth and specular view vectors, authored GX face culling, 4x MSAA with capability fallback/resizing, and stage-driven clear color. Six platform tests, eight allocation checks, clippy, 15 Metal fixtures and native smoke pass. Exact retail camera tracking/pixel equivalence remains outside the verified display behavior; continue held items, effects, shadows and mip/LOD rendering.

- [x] Held article phase (2026-09-11): typed item visual descriptors, table-owned hand attachment and pose corrections, authored motion seeking, and blaster opening/recoil sampled in shared item state. Presentation retains no independent clock; hand alignment, sparse/frequent capture and reset checks pass, as do eight allocation tests and 84 release combat/ordered-particle oracles including both full matches. Metal firing preview inspected. Continue model/specialized effects, shadows and mip/LOD rendering.

- [x] Model effects phase (2026-09-11): retain authored effect materials/texture clocks in shared simulation pools, expose borrowed visual models, restore pooled clocks without allocation, and draw independently animated instances through shared geometry and bounded GPU pose/material buffers. Sparse/frequent/reset effect checks, eight allocation tests, 84 exact combat/ordered-particle tests, workspace clippy and Metal firing preview pass. Current effect shape trees contain no deformation tracks; real shape animation is explicitly rejected. Specialized particles, historical afterimages, shadows and mip/LOD remain.

- [x] Authored mip/LOD phase (2026-09-11): shared typed texture LOD, padded GX mip-chain decoding, prepared GPU mip uploads and selected-image dimensions at every level; nearest/bilinear/trilinear minification, live LOD bias and anisotropic taps. Archive suite and explicit padded/truncated-chain tests pass, eight allocation checks pass, and 18 numeric Metal fixtures include mip selection/interpolation; firing preview inspected. Retail bias-clamp/edge-LOD details remain pixel-fidelity work. Continue particle geometry, afterimages and shadows.

- [x] Illusion afterimages (2026-09-11): two authored article meshes follow the existing position/rotation history entries 1 and 3. Shared item state retains secondary creation and trailing pose across clones; primary hides in the end state. Held weapons and afterimages reuse one prepared article-animation path and preload every motion's texture variants. Eight allocation tests, 84 combat/ordered-particle oracles and secondary-lifetime/capture/clone tests pass; both blue afterimages inspected on Metal. Continue particle geometry and shadows.





- [x] Specialized particle geometry (2026-09-11): velocity-oriented sprites, ribbon trails, framebuffer-sized points/lines, authored alpha comparisons and texture-edge depth writes. Tornado trails reconstruct the previous cylindrical point with audited retail FMA ordering. Shared fixed-point color/alpha interpolation avoids duplicate arithmetic; capture remains read-only. Particle suite, eight allocation checks, workspace clippy and Metal Illusion preview pass. Shadows remain.

- [x] Planar shadows and final rendering integration (2026-09-11): fighter/held-article shadows reuse skinned geometry and live floor segments. Shared depth/stencil restricts coverage to stage pixels and prevents repeated darkening; no copied meshes or simulation clocks. Eleven library API tests, six platform tests, eight allocation tests, workspace clippy, native build/resize/focus/pause/keyboard smoke, 18 numeric Metal fixtures, shield/laser/Illusion and tick-5000 background renders pass. Shadow-on/off comparison confirms floor coverage; offstage floor regression passes. All planned first-app rendering features are implemented. Retail pixel equivalence, exact camera tracking, GX rounding/generated-coordinate/LOD quirks and retail soft-shadow filtering remain fidelity work; see `docs/PORT_NOTES/NATIVE_APP.md`. The separately tracked laser-shield gameplay fault remains.

- [x] Screenshot lighting correction (2026-09-11): traced dark fighters to missing spline-JObj light transforms, and the absent central white glow to an ignored billboard flag. Corrected shared presentation transforms, GPU billboard orientation, view-space reflection/highlight coordinates and HSD diffuse/specular/EXT composition order. Eight release allocation checks, debug light regression, six platform tests, 24 numeric Metal fixtures, workspace clippy and rebuilt native smoke pass (tick 226, keyboard movement verified). GPU composition uses compact state after the first full-material version failed native timing; numeric outputs are unchanged.

## Current focus

**Paused by user request after `415a230` (2026-09-12).** The revival-laser fix
is committed; no further implementation or breadth work is underway. Latest
full debug/release gates: **1,443 passed, zero failed, one existing ignore**.
All-target Clippy, formatting, 225 harness tests and native macOS build pass.
Size is unchanged at 3,995,872 stripped bytes; existing size/time limits remain
red. See `docs/PORT_NOTES/REVIVAL_LASER.md` and `docs/PERF.md`.

Latest completed packets: wall/ceiling recovery (`987fe7f`), CaptureJump and
airborne release (`fb53f39`), revival-laser contact (`415a230`). The fixed
48-case corpus now produces 20 match finishes, 17 full 6,000-tick runs and
11 faults (initially 46 faults); this is robustness evidence only.

On resumption, start with DamageFlyRoll: three retail recordings cover flight,
floor collision and hitstun expiry; the source draft is not applied. Remaining
work includes captured-pull damage, Reflector edge departures and Counter
interactions, phantom/simultaneous contacts, ScreenKO camera, timer/tie/Sudden
Death behavior, and final interaction coverage/corpus acceptance. The detailed
scope and unresolved combinations live in `docs/MATCHUP_COMPLETENESS.md`.

- [x] Fire Fox contact (2026-09-12): recorded charge and travel hits on Marth.
  Acceptance: fire_contact_* fighter/RNG/ordered-particle and full SRT gates,
  zero allocations, both workspace profiles and clippy. Source efAsync_Dispatch
  80063AEC maps fire hit spark0x3EA to positional generator0x14.

- [x] Directed ledge input and timeout (2026-09-12): ten recorded C-stick
  attack/escape/drop/priority and 1,200-tick timeout scenarios. Acceptance:
  `ledge_input_*` fighter/RNG/ordered-particle, all captured SRT, raw ledge
  scratch and zero allocations, both workspace gates and clippy. Slow rows
  are source-ported but await successful high-percent captures; wall/ceiling
  and further threshold/priority combinations remain open.

- [x] Directed revival and airborne capture (2026-09-12): seven recorded
  600/900-tick scenarios cover platform timeout, digital/analog shield+A, and
  transient high capture with ground return. Acceptance: named
  `capture_revival_*` fighter/ordered-particle, full local bone and allocation
  gates, raw revival/capture scratch, both workspace profiles and clippy.
  Subsequent prolonged hold and airborne release witnesses are committed in
  `fb53f39`; see `docs/PORT_NOTES/CAPTURE_RELEASE.md`.

- [~] Matchup completion (started 2026-09-11; paused 2026-09-12): user requested
  finishing the remaining Fox/Marth/FD gaps before breadth. Original work order:
  C-stick throws and Fox throw articles, recovery counterparts/rebounds, common
  defense/grab/ledge inputs, lifecycle/timer outcomes, then a reproducible
  combination corpus and final coverage audit. Resume only when requested.
  First acceptance: fourteen `cstick_throw_*_fd_{fox,marth}` 300-tick retail
  fighter/particle gates, zero-allocation checks, both workspace gates and clippy.

- [x] Common-input completion (2026-09-12): twenty-four Fox/Marth taunt,
  dash/run defense and shield-input scenarios pass fighter/particle/RNG, raw
  countdown and both-fighter 150-tick bone gates. Fixed neutral shield pose and
  outgoing blend-joint ownership. Debug/release each 1,307 passed, zero failed,
  one existing ignore; clippy, formatting, zero allocations, 220 harness tests
  and schema checks pass. Native build passes; current native smoke and new
  Dolphin captures are blocked by macOS IOSurface exhaustion (1,020 clients,
  922 attributed to Safari), not a simulation divergence. Stripped simulator
  3,943,848 bytes (+280); full performance census not rerun. Version 1 public-API
  exploration saves 48 reproducible episodes; initial 46 faults are triaged
  within this matchup. See `docs/PORT_NOTES/COMMON_INPUTS.md`.

- [~] Recovery collision transitions (2026-09-11): exact acceptance starts
  with `illusion_start_landing_fd_fox`, `firefox_charge_landing_fd_fox`,
  `firefox_ground_launch_fd_fox`, `firefox_floor_rebound_fd_fox`, and
  `firefox_end_air_landing_fd_fox` (300 ticks,
  ordered particles/RNG and 150 captured bone ticks), plus zero allocations,
  both workspace gates and all-target clippy. Ending-phase and travel-edge
  witnesses are still being refined; do not count an intended transition
  unless the retail trace actually reaches it.

- [x] C-stick throws and Fox throw articles (2026-09-11): fourteen exact
  directional/priority/pulse scenarios compare fighters, items, particles and RNG.
  Fixed reversal operand decoding, blaster/laser callbacks, captured item damage,
  and linked forward-throw hitlag. Restored owner-before-script ordering and
  slow-knockback collateral hitbox clearing; both old ignored throw tests now pass
  with unchanged expected bytes. Debug/release each 1,249 passed, 0 failed,
  1 existing ignore; clippy, formatting, 220 harness tests and schema checks pass.
  Simulation allocation gates and native smoke/exported replay pass (232 ticks).
  Stripped simulator 3,926,840 bytes (+56); no new full performance claim.
  See `docs/PORT_NOTES/CSTICK_THROWS.md`. Next: captured recovery transitions.

- [x] Jump-squat up-special (2026-09-11): explicit Up-slot dispatch before
  grab/up-smash. Six captured Fox/Marth cases cover ordinary, competing A/Z
  and diagonal input. Fighter/particle and zero-allocation gates pass; both
  workspace profiles: 1,230 passed, 0 failed, 3 existing ignores. Clippy,
  220 harness tests and rebuilt native smoke/replay (232 ticks) pass. See
  `docs/PORT_NOTES/JUMP_SQUAT_UP_SPECIAL.md`.

- [x] Reflector input (2026-09-11): turn rows, release and turn/jump priority,
  button/tap aerial jump cancel and preserved turn landing. Acceptance: seven
  `reflector*turn*`/`airreflector*jc`
  300-tick fighter/ordered-particle gates, raw scratch and root-Y bone checks,
  zero allocations, full debug/release gates and clippy. Retail source/ASM and
  captures verified. Both full profiles: 1,223 passed, 0 failed, 3 existing
  ignores; clippy, 220 harness tests and native smoke/replay (232 ticks) pass.
  See `docs/PORT_NOTES/REFLECTOR_INPUT.md`.

- [x] Post-hitstun aerial input (2026-09-11): missing Attack entry, ordinary
  damage versus tumble air-dodge priority, and air-dodge knockback decay.
  Acceptance: seven `hitstun_*` 300-tick fighter/ordered-particle gates,
  `post_hitstun_allocation_budget`, full debug/release workspace gates,
  all-target clippy and harness pytest. All seven pre-fix replays fail as
  documented in `docs/PORT_NOTES/POST_HITSTUN_INPUT.md`. Full debug/release
  gates each pass 1,213 tests with 0 failures and 3 existing ignores; clippy,
  220 harness tests and the native build pass. Native smoke exported 233 ticks
  and headless replay passed. Binary-size audit committed as `e00b442`; the
  existing size debt remains open.

- [x] Aerial Counter packet (2026-09-11, `5c3d72b`): aerial entry/physics/end, ground-air
  preservation and aerial hit response. Acceptance: `aircounter_fd_marth`,
  `aircounter_landing_fd_marth`, `aircounter_hit_fd_marth` 300-tick Dolphin/ordered
  RNG gates, existing grounded Counter/full matches, zero allocations, debug and
  release workspace gates plus clippy. User authorized continuing gameplay gaps
  while binary-size debt remains open, and commits at verified packet boundaries.
  Replay/laser-shield packet committed as `4e08fb7`.
  Four directed scenarios now pass, including offstage completion and KO/respawn;
  raw window/volume/jump/ECB checks and zero-allocation tests pass. Full release
  gate and clippy pass. See `docs/PORT_NOTES/AERIAL_COUNTER.md`; deliberate
  support-loss and projectile-contact coverage remain on the matchup inventory.

- [x] Diagonal forward smash (2026-09-11): authored angle availability and
  main-stick+A versus C-stick selection, including dash entry. Acceptance:
  `fsmash_diagonal_fd_fox`, `fsmash_diagonal_fd_marth`,
  `fsmash_dash_diagonal_fd_fox`, `fsmash_dash_diagonal_fd_marth` 300-tick
  Dolphin/ordered particle gates; existing smash/full-match tests, allocation
  checks, workspace gates and clippy. Keep other characters' unavailable state
  bodies explicit rather than silently forcing every character to straight smash.
  Verified: full debug/release workspace gates each 1,205 passed, 0 failed,
  3 existing ignores; all-target clippy and 220 harness tests pass. Four directed
  scenarios and zero-allocation checks pass. See `docs/PORT_NOTES/DIAGONAL_SMASH.md`.

- [~] Approved next milestone (2026-09-11): Fox–Marth/FD interaction completeness
  before breadth. Source review confirms ordinary input/contact variants still
  panic despite exact recorded full matches. Start with UI-to-headless replay
  capture and a reachable-gap inventory, then laser/shield, special transitions,
  contact edge cases and match endings. Scope, evidence and exit criteria:
  `docs/MATCHUP_COMPLETENESS.md`. Replay capture, reachable-gap audit and ordinary
  laser/shield response implemented; correctness/native checks pass. Serial
  performance gate remains red: existing binary-size debt plus a 0.42% size
  increase and 2.88% load cost versus unchanged HEAD; fixed load/tick ceilings
  narrowly exceeded. Details: `docs/PORT_NOTES/MATCHUP_REPLAY_AND_SHIELD.md`.
  First packet acceptance: bit-preserving replay
  round trip, direct/replayed continuation equality, retained failing input and
  diagnostic, zero-allocation recording, pause/reset/capacity behavior, native
  export integration, `cargo gate` and workspace all-target clippy.

**Steel thread (from 2026-09-09 evening): one complete match, bit-exact.**
Fox vs Marth on Final Destination, four stocks, human inputs on both
ports, from match start through the GAME banner, compared tick for tick
on the gated keys plus the RNG ledger and particle replay. Breadth (more
characters, more stages) is paused until the thread closes, except where
it falls out as a by-product. Two phases, tracked in the **Steel thread**
section below:

1. **Consolidation round (C1..C10):** the architecture that makes going
   wide a matter of adding tables and data: fn-pointer motion state tables
   (common + per-character), special-move entry hooks, family crates for
   kinds that share retail code, a non-generic fighter core under a thin
   generic shell, the `melee-ef` crate, no per-tick heap allocation, and
   size/throughput/instantiation budgets as regression gates. Existing
   gates prove each refactor changed nothing.
2. **Combat table (S1..S11):** every state a pad can reach in the match,
   in cost order (ground attacks, aerials + L-cancel, specials,
   items/projectiles, hit reactions incl. DI/SDI/ASDI/CC, shield, grabs,
   ledge, KO variants, match flow), then the full-match acceptance gate.

Status 2026-09-10 (early morning): C1..C15 + P1 merged; S1 (ground attacks)
and S4 (items, laser, Fox family) merged; every S2..S10 scene recorded.
Running: S2 aerials/L-cancel (harness lane), S3 specials (core lane), S5
hit reactions (perf lane); S6, S7+S8, S9, S10 queued; S11 = the recorded
four-stock scripted match plus the user's Slippi replay. Main: 30 crates, 977/0 strict tests
in both profiles. Originally: consolidation lanes launched 2026-09-09 (core: C1 then C2/C7;
perf: C6, C5 sim-side, C10; harness: C9). Going in: seven characters x 18
scenes on FD (+ Battlefield, Yoshi's Story, Dream Land idle/start/cold),
combat through KO/respawn, cold start, Slippi pipeline (needs an offline
replay), 141 commits, 26 crates, ~950 tests, main green.

<details><summary>Earlier focus text (history, 2026-09-08..09)</summary>

**The disc is present, extracted, and the oracle works.** `harness/roms/idle_ys_fox.sav` + `harness/traces/idle_ys_fox.expected.jsonl` are the first real-game trace (two idle Foxes, Yoshi's Story, 600 frames). Fusion audit complete workspace-wide. Fox's Wait1 animation plays through the port. **Milestones 1-3 gates passed, plus the match-start scenario.** The port reproduces a Fox vs Fox match on Final Destination from its first initialised frame (entry warp, fall, landing, idle) for 600 ticks bit-exactly, RNG included, and the bone oracle matches all 73 bones. **2026-09-09 evening: paused for a stock-take; all lanes idle, main green.** Milestone 4 movement is essentially complete on FD, Battlefield, Yoshi's Story and Dream Land for seven characters; Milestone 5 combat has jab, tilt launch, shield hit, grab/throw, tech, KO/respawn; cold start and the Slippi pipeline exist. Earlier: **Milestone 4 has started (2026-09-09):** the tick tracer injects scripted inputs and records the pad each tick consumed; five movement scenarios (walk, dash, jump, turn, squat; 300 ticks each from `idle_fd_fox.sav`) are recorded with RNG ledgers; the port replays their pads and stops at each first transition with an explicit `unimplemented`. Squat/Turn/Walk (M4-T1), Dash/Run/RunBrake with dust effects (M4-T2) jumps/fast fall/landing (M4-T3), shield/spot dodge/roll (M4-T4) air dodge/wavedash/backward jump/ledge (M4-T5) and TurnRun/WalkFast/ledge climb+escape (M4-T7) are ported and gated: 17 scenario gates; dash and jump need dust effects (new particle draw sites) next. A `melee-ef` crate for the effect layer is still pending. Older: Milestone 3 groundwork (melee-gr Yoshi's Story, melee-ft init and frame order, melee-sim loop).

Older note: `harness/roms/GALE01.iso` (gitignored)
matches both hashes in `docs/ISO.md`; `harness/roms/sys/main.dol` matches
the decomp's `build.sha1`; a copy of `main.dol` is at
`third_party/melee-decomp/orig/GALE01/sys/main.dol` where the decomp's
`dtk`/`objdiff` tooling expects it (that path is gitignored in the
submodule). The Dolphin scripting build is at
`~/Projects/dolphin-scripting/build/Binaries/Dolphin.app` (`docs/DOLPHIN_BUILD.md`).

Nothing disc-dependent has been run yet. The next session should start
with the three items under "Milestone 0: disc arrived" below; they are
independent and can run in parallel. Every one of them is the first time
the port meets the real game, so expect surprises and record them here.

</details>

## Blockers

- (resolved 2026-09-10 ~08:00: the user reset the Codex credits) 03:56: Codex usage limit reached; S9 and S10 were ported directly meanwhile; S6 and S7+S8 WIP lanes rebased onto S9 and resumed on Codex.

- **[~] 2026-09-09: local game data lost and restored.** A `git add harness` in the chars lane committed the `harness/roms`/`harness/traces` symlinks; the fast-forward into `main` deleted the ignored contents (disc image, extracted files, 25 savestates, every recording). Fixed in git (`6722008`, `70126a4`). Recovery the same evening: disc image re-supplied by the user and verified; `extract_fst.py`; all 25 savestates recreated from the documented menu scripts (`drive.py`, RAM-only pokes, `save-when-fighters` + `save-when-wait`), each verified by fighter kinds, Wait state and stage kind; all 162 saved scenes re-recorded with `record.py` (tick trace with items, ledger, particles, bones); both directories mirrored to `~/melee-data/`. Cold seeds refreshed from the new sidecars (`922982c`). **Findings on the new recordings:** Fox idle, nine match-start scenes (six characters, four stages) and all six cold starts match 600x49 with zero divergences (the port is bit-exact from fresh boundaries, not fitted to the old captures). **Open:** (1) nine of the ten new idle savestates and start_fd_puff sit at scheduler positions the importer never supported (anim proc link 1, collision proc link 6, a stage proc at link 4, fighter creation PC 8037F210); ~100 gates depend on them -> **C12** (harness lane, running). (2) Tests that copied values from the old recordings (hsd-particle `live_fd` seed, the `*_spawns.json` fixtures, two melee-ft tests) -> **C13** (perf lane, running). Dolphin also crashed on an IOSurface limit mid-batch: 919 `SafariPlatformSupport.Helper` processes on the machine (killed; they respawn, ~100 in ten minutes, cause unknown).
- ~~Oracle sampling point~~ **fixed 2026-09-08**: `harness/dolphin/tick_trace.py` samples on the memcheck of the scheduler tick counter (`gm_80479D58`, store at retail 0x801A4FB8, right after `HSD_GObj_80390CFC` returns). `run_scenario.py --tick-trace` records, decodes and validates; `harness/validate_ticks.py` checks +1 animation steps and LCG-consistent seeds. `idle_fd_fox.tick.expected.jsonl`: 600 ticks, 0 violations, rerun byte-identical.
- **RNG consumers in an idle match.** User chose FD (unlocked 2026-09-08 by OR-ing bits 6-7 into the save's stage unlock mask via `drive.py poke-or`; Battlefield came with it). But FD is *not* RNG-quiet: `idle_fd_fox` draws 1..61 times per tick (histogram in the run output; pattern 6k+1), from `gr/grlast.c` background lights/flicker (lines 440-515, 652-658), plus the Wait1/Wait2 choice. Matching `rng.seed` on FD means porting grlast.c's per-tick decoration logic (1,035 lines, no particles or items). User decision (2026-09-08): port FD's decorations now so `rng.seed` is part of the M3 gate; stay on FD; keep the other stages' RNG dependencies tracked in Milestone 6.

## Decisions

| Date | Decision | Why |
|---|---|---|
| 2026-09-08 | Layered workspace, one crate per subsystem and per character | Build time. 435k lines of game C cannot be one crate. |
| 2026-09-08 | Internal state need not match retail memory layout; `Snapshot` + schema handles comparison | Lets the port use enums and real types. |
| 2026-09-08 | Floats in traces compare by bit pattern only | JSON round trip of the decimal caused false divergences. |
| 2026-09-08 | Oracle reads retail memory via Dolphin scripting; never a modified DOL | Keeps Slippi replays and community tools valid as test inputs. |
| 2026-09-08 | Target Felk's Dolphin scripting fork; GDB stub as fallback | Only option with savestate, frame hook, memory read, input in one process. |
| 2026-09-08 | Slippi fixtures from hohav/peppi (MIT), not slippi-js (LGPL) | License. |
| 2026-09-08 | Commit the empirically captured `frsqrte`/`fres` tables (user decision) | Captured from executing the instructions in Dolphin with no Dolphin source consulted (`harness/gekko_probe/README.md`). Table values describe hardware behaviour, but Dolphin's emulation is GPLv2 code; user to decide. Model: frsqrte 32 (base, slope) entries by exp parity + top 4 mantissa bits; fres 32 entries by top 5 bits, single-range clamp. |
| 2026-09-08 | Bulk porting delegated to Codex (`gpt-6-astra`) via `tools/codex-task.sh`; Fable subagents kept to one or two | User: Fable fan-out too expensive. Guardrail is the bit-exact test suite; Claude reviews and commits. |
| 2026-09-08 | Melee-specific on-disc structs (`coll_data`, `ftData`, `map_head`) are read by the owning game crate (`melee-mp`, `melee-ft`, `melee-gr`) in a `desc`-style module that depends on `hsd-archive`; `hsd-archive` stays HSD-only and never depends on `melee-types` | Keeps the archive crate a leaf; game crates already know their own types. |
| 2026-09-09 | Scripted scenarios: the TOML `inputs` schedule drives Dolphin at VI frames; the port replays `HSD_PadGameStatus` as recorded at each tick boundary (`inputs.pN` beside the expected record, `melee-sim/src/inputs.rs`) | VI-to-tick alignment is not modelled; pads are inputs, never compared state. Several scenarios from one savestate share its particle capture (`Scenario::boundary_path`). |
| 2026-09-09 | **Steel thread before breadth:** Fox vs Marth on Final Destination, a full match bit-exact, with a consolidation round first | User (2026-09-09): the gold standard is clean, zero-cost abstractions; going wide should be additional trait implementations and tables, not more shared code. |
| 2026-09-09 | Motion states are fn-pointer tables (common table + per-character table from `CharacterCallbacks::special_rows()`), specials enter through a trait hook; kinds that share retail code get a family crate (`ft-fox-family` with a `FoxFamily` trait) that both characters depend on; `Fighter<C>` becomes a non-generic core plus a thin generic shell; no heap allocation in the tick path; perf/size/instantiation budgets are regression gates | Mirrors retail's own dispatch (`ftData_MotionStateList`, `ftFx_Init_MotionStateTable`, `ftData_SpecialN` per-kind tables), so the port's code is the C with the `switch (kind)` deleted. Keeps monomorphization (the one real cost) small. Baseline 2026-09-09: stripped `melee-sim` 3.9 MB, 600-tick gate 0.25 s CPU incl. load. |
| 2026-09-26 | Record with the headless Dolphin (`DolphinHeadless.app`, Null video, host audio muted) by default; windowed only for live human play and menu driving | Byte-identical to windowed recordings on two scenes; no IOSurface exhaustion, no window or sound. `tools/build-headless-dolphin.sh`, `docs/DOLPHIN_RUN.md`. |
| 2026-09-26 | Work single-threaded in the main session: no Codex, subagents or lanes | User request mid-milestone. |
| 2026-09-26 | Robustness corpus v2 starts from registered retail boundaries so every case (and fault) is replayable in Dolphin | Turns generated failures into exact oracles; extends to other layouts by adding boundaries. |
| pending | Retail asm workflow once disc arrives | `dtk` disassembly vs `objdiff`; how agents look up a function's asm. |

## Milestone 0: Infrastructure

- [x] Cargo workspace, layered, dev profile tuned (2026-09-08)
- [x] `melee-diff` canonical trace format and first-divergence report
- [x] Harness: symbol resolver over decomp `symbols.txt`
- [x] Harness: schema-driven memory decoder
- [x] Harness: `gen_schema.py` extracting struct layouts from header comments (692 Fighter fields, 35 Item fields)
- [x] Harness: GObj fighter-list walk with fake-memory tests
- [x] Harness: Dolphin script skeleton (`trace_scenario.py`)
- [x] Docs: ISO, oracle design, Dolphin options, plan
- [x] Decomp pinned as submodule
- [x] Build Dolphin scripting fork on macOS arm64 (`docs/DOLPHIN_BUILD.md`; binary under `~/Projects/dolphin-scripting/build/Binaries/`)
- [x] Verify `frameadvance` fires with Null video backend (yes, even with no disc, via `-e probe.dol`)
- [x] Measure oracle frames/sec with a booted game: 60 fps realtime, 131 fps unlimited (600 frames in 4.6 s; 6.8 s wall incl. boot + savestate load)

### Milestone 0: disc arrived (do these first)

- [x] (2026-09-08) **Extract the disc filesystem.** Write `harness/extract_fst.py` (GameCube FST: offset at 0x424, size at 0x428, 12-byte entries, string table after) to `harness/roms/files/` + `sys/`. Expect ~1200 files incl. `PlFxNr.dat`, `PlFx.dat`, `PlFxAJ.dat`, `PlCo.dat`, `GrNLa.dat` (Final Destination). Then `crates/hsd-archive/tests/real_dat.rs`: parse those five, assert file_size/relocs/publics, read the Fox root `JObjDesc` and count joints; document the `PlFxAJ.dat` animation layout and the `GrNLa.dat` `map_head`/`coll_data` offsets. Tests must skip cleanly when the disc is absent.
- [x] (2026-09-08) **Boot the game in Dolphin scripting and get the first real trace.** Launch `Dolphin -e harness/roms/GALE01.iso --script harness/dolphin/trace_scenario.py` with the config flags from `docs/DOLPHIN_BUILD.md`; confirm `on_frameadvance` fires and `memory.read_u32(seed)` changes. Navigate to a Vs match (Fox, Final Destination, 1 stock, no items, CPU off or a second human slot idle), save a savestate at the first playable frame to `harness/roms/idle_fd_fox.sav`, run the `idle_fd_fox` scenario for 600 frames, `decode.py` it to `harness/traces/idle_fd_fox.expected.jsonl`. Verify the fighter-list walk finds one fighter and `p0.cur_pos` is sane. Record frames/sec. Note: `walk.py` assumes `HSD_GObj_Entities` layout from gobj.h; this is its first real test.
- [~] (2026-09-08) **Retail assembly lookup.** In the decomp submodule run its tooling (`python configure.py` then `ninja` may be heavy; `dtk dol split` or `objdiff` per `docs/getting_started.md`) to get per-function disassembly from `orig/GALE01/sys/main.dol`. Deliver `harness/asm.py <symbol>` that prints a function's retail asm. Then start the **fusion audit**: `grep -rn "FUSION AUDIT PENDING" crates/` lists every site (gekko-math 25, melee-lb 14, hsd-anim 92, melee-mp many); for each, read the asm, replace with `fmadds`/`fmsubs`/`fnmsubs` or confirm unfused, and remove the marker. Start with gekko-math `sqrtf` and `sinf` since everything depends on them, then `lbtrigf`, then mtx.
- [x] (2026-09-08) **Gate Milestone 2 — PASSED.** 0 mismatches over 3,080 bone-matrix/SRT words vs the real game (`docs/M2_GATE.md`, `crates/melee-sim/tests/m2_gate.rs`). Original text: With the trace from the Dolphin item and the skeleton from the extraction item: read Fox's bone matrices from the oracle (JObj `mtx` at each joint via `HSD_JObjGetMtxPtr`; find the Fighter's root JObj through `fp->x28_jobj` or equivalent in `ft/types.h`) and compare against `hsd-anim` evaluating the Wait animation at the same frame. This is the first bit-exact comparison against the real game.
- [x] Verify savestate load is synchronous with next frame (yes; memory is frozen during a callback, load lands on the saved boundary)
- [ ] Small C++ patch or debugger workflow to set code breakpoints for intra-frame phases
- [x] `Snapshot` trait in `melee-types`, `RecordSink` in `melee-diff`, `SchemaCoverage` in `melee-sim` with stale-exclusion detection
- [x] RNG ledger: `harness/dolphin/rng_ledger.py` + `harness/rng_ledger_report.py` (memcheck on `seed`, PC/LR via patched fork) attribute every draw per tick to its `bl` site
- [ ] Golden fixture recorder: break on function entry/exit, dump args and touched memory to `harness/goldens/`
- [ ] Retail asm lookup tool: given a symbol, print its disassembly from `main.dol` (needs disc)
- [ ] CI: `cargo gate`, clippy `-D warnings`, harness pytest, `gen_schema.py --check`

## Milestone 1: Math and RNG (`gekko-math`)

- [x] HSD RNG, bit-exact with tests
- [x] FMA wrappers in PowerPC operand order
- [x] MSL `sinf`, `cosf`, `tanf`, `logf`, `frexp`, `fmodf`, classify, fabs
- [x] Gekko int/float conversion semantics (`fctiwz`, `__cvt_*`)
- [x] Native-C reference oracle test (bit-exact vs decomp C, `-ffp-contract=off`)
- [x] `frsqrte` / `fres` hardware-exact from captured tables (golden fixture 10,600 pairs; full captures 148k pairs, 0 mismatches; `sqrtf` bit-exact vs native C)
- [ ] FMA audit of the 25 marked sites against retail asm (blocked on disc)
- [ ] Int-conversion audit of the 1 marked site (blocked on disc)
- [x] Paired-single matrix routines: transcribed from asm in `hsd-anim::mtx`; fusion audited against retail (`crates/hsd-anim/tests/ref/FUSION_AUDIT.md`)
- [x] (2026-09-12) `fmuls` double-width estimate multiplier rounding: PSVECMag/Normalize use 25 significant bits; 264 standalone interpreter cases and independent C matrix oracle pass. ARM64 JIT agrees on estimate cases; artificial subnormal differences documented in `harness/gekko_probe/fmuls/README.md`.

## Milestone 2: HSD engine (`hsd-archive`, `hsd-gobj`, `hsd-anim`)

Gate: load one character and one stage archive, evaluate the wait
animation, match bone matrices from the oracle. **Gate passed 2026-09-08**
(Fox Wait1 frame 6 and 7 on the `idle_ys_fox` savestate, bit-exact on all
recomputed bones). Remaining items below are breadth, not gate blockers.

- [x] `hsd-archive`: header, relocs, publics, externs, strings; synthetic tests
- [x] `hsd-archive`: typed readers for JObjDesc/DObj/MObj/AObj/FObj/AnimJoint/MatAnimJoint/ShapeAnimJoint and Melee FigaTree (20 tests)
- [x] `hsd-archive`: test against a real `.dat` (7 tests; Fox 73 joints, GrNLa coll_data 16 verts/16 lines; `docs/DISC.md`)
- [x] `hsd-gobj`: `gobj.c`, `gobjproc.c`, `gobjplink.c`, `gobjgxlink.c` (link/priority only, no GX), `gobjobject.c`, `gobjuserdata.c` (~800 lines)
- [x] `hsd-anim`: `mtx.c`, `quatlib.c`, and the SDK `PSMTX*`/`PSVEC*` paired-single kernels; all 31 fusion sites audited (Codex), ~22% of mtx/quat sweep inputs differ from unfused
- [ ] `hsd-anim`: `PSMTXRotAxisRad` (used by jobj.c, psdisp.c, cobj.c), `C_MTXLookAt`
- [ ] Wire `hsd_anim::mtx::InverseTrig` to `melee_lb::trigf` in `melee-sim` (hsd-anim must not depend on melee-lb)
- [x] `hsd-anim`: `aobj.c`, `fobj.c` keyframe evaluation (retail-faithful native-C oracle, 0 mismatches; Hermite and linear tracks are fused in retail)
- [ ] `hsd-anim`: confirm via Dolphin whether retail data ever hits the uninitialised single-key FObj path (see fobj.rs `FOBJ_UNINITIALISED_VALUE`)
- [x] `hsd-anim`: `jobj.c` hierarchy, matrix setup, dirty flags, SRT setters, anim application, ftparts bone lookup (52 tests)
- [x] `hsd-anim`: `load.rs` converts `JObjDesc`/`AnimJoint` trees into `JObjTree` (Codex; 12 synthetic tests + real Fox skeleton test)
- [x] `hsd-anim`: `dobj.c`, `mobj.c` data and anim plumbing, no render
- [ ] `hsd-anim`: `robj.c` (942) constraints. jobj.rs lists exactly which RObj entry points it calls (all currently no-ops, matching `robj == NULL`); check whether fighter skeletons carry RObjs before porting
- [ ] `hsd-anim`: `cobj.c` (1406) camera object, needed by `cm` later
- [ ] HSD `class.c`/`object.c` object model: decide Rust representation (traits vs enums)
- [-] Rendering: `tobj`, `pobj`, `lobj`, `tev`, `texp*`, `psdisp*`, `displayfunc`, `video`, `shadow`, `fog` (Milestone 8)
- [-] Audio: `axdriver`, `synth`, `hsd_3A94` and other sound files (Milestone 8)
- [x] (2026-09-08) `hsd-particle`: **matches the real game**: restored from the live dump at the `idle_fd_fox` savestate, 600/600 ticks reproduce the RNG draw counts, draw-site order and per-tick seed; 8/8 dumped frames match all 10,120 particle/generator fields bit-for-bit (`tests/live_fd.rs`). Remaining breadth (nested spawning, tornado/rectangle emitters, more opcodes) tracked in `docs/PARTICLES.md`. History: first pass committed (Codex): bank parsing, generator emission shapes, particle bytecode interpreter subset, list ordering, six RNG sites with gating conditions (`docs/PARTICLES.md`); 35 tests incl. real FD bank. Cold-start draw counts 43,19,1,1,61,... vs live ledger 61,7,7,19,25,...: the live generator/particle state at the savestate must be captured (`harness/dolphin_particle_snippet.py`, `docs/PARTICLES_DUMP.md`) and restored before the sequences can be compared. Remaining: nested spawning, AppSRT callbacks, tornado/rectangle emitters, more opcodes. Original note: `generator.c` (1,244) and `particle.c` (3,095) lifetime/emission logic. **Not visual-only**: the RNG ledger shows every idle FD tick draws once in `hsd_8039EE24` (generator update) plus six per live particle (2 sites in `hsd_8039DAD4`, 4 in `hsd_8039930C`); this is the 6k+1 pattern. Required for `rng.seed` parity on every stage. `psdisp.c` rendering stays out of scope.
- [-] `spline.c` (visual only)

## Milestone 3: One fighter idle (`melee-lb`, `melee-mp`, `melee-ft`, `ft-fox`)

Gate: `harness/scenarios/idle_fd_fox.toml`, 600 frames bit-exact.
**Gate passed 2026-09-09**: `melee-sim gate harness/scenarios/idle_fd_fox.toml` reproduces all 600 ticks x 49 keys (both Foxes' 24 fields and `rng.seed`) from the imported savestate boundary, with the real scheduler (`hsd-gobj::World`), stage, particle system and fighters; no per-tick oracle inputs. Remaining items below are breadth toward Milestone 4.

- [x] `melee-lb`: `lbtrigf.c` (atan2f, atanf, asinf, acosf, lb_sqrtf) and `lb_00CE.c` expf/powf
- [x] `melee-lb`: `lbanim.c` FigaTree -> JObj attachment (Codex; real Fox Wait1: 120 frames, 73 nodes, 49 animated joints); translation-filtered path `fn_8001E60C`/`lbAnim_8001E7E8` deliberately skipped, see anim.rs header
- [x] `melee-ft`: `desc.rs` reads `ftData.xC` animation table from PlFx.dat and slices sub-archives out of PlFxAJ.dat (Codex)
- [ ] `melee-lb`: `lbvector.c`, `lbcollision.c`, `lbarchive.c`, `lbfile.c` (headless file access)
- [ ] `melee-lb`: remaining `lb_*` files as needed by callers (17k lines total)
- [x] `melee-mp`: `mplib.c` (103 fns) and `mpcoll.c` (complete) as `CollMap`; 42 tests on synthetic FD; all 39 fusion sites audited against retail (33 fused), geometry native-C oracle 200k inputs 0 mismatches (`tests/ref/FUSION_AUDIT.md`)
- [ ] `melee-mp`: `mpisland.c` (626 lines; feeds CPU AI and Link hookshot) — `CollMap::island_update` is the hook
- [ ] `melee-mp`: terrain sound-id tables (`mpLib_803BD3D8..`) once an sfx layer exists
- [x] `melee-mp`: `desc.rs` reads `coll_data` from an archive (Codex); real GrNLa.dat loads into `CollMap`, floor/ledge queries verified (3 real-stage tests)
- [ ] `melee-gr`: `ground.c`, `grlib.c`, `grdatfiles.c` plumbing for one stage (Final Destination)
- [~] (2026-09-08) `melee-gr`: `grlast.c` Final Destination decorations, first pass committed (procs, direct RNG sites, desc readers). **Finding:** the direct grlast.c logic draws on only 2 of 600 idle ticks; the 6k+1 draws per tick come from elsewhere (likely the HSD particle generator via a particle-spawn animation track on map 4). RNG ledger (`harness/dolphin/rng_ledger.py`, needs the fork patch `docs/patches/0002-scripting-read-pc-lr.patch`) attributes every draw: 120 idle ticks = 3,716 draws: 120 from `hsd_8039EE24+0xDC`, 599 each from six particle sites, 1 from `grLast_8021ADD0+0x270`, 1 from `ftCo_8008A7A8+0x114` (Wait anim choice). So the stage's own logic is nearly RNG-free; the particle system is the dependency.
- [ ] `melee-gr`: Dolphin capture of the FD `Ground` struct at the `idle_fd_fox` savestate (timers/phases) to seed the decoration state; then a 600-tick RNG-draw-count test against `idle_fd_fox.tick.expected.jsonl`
- [x] `melee-ft`: `ftanim.c` playback/attach/blend + `ftwaitanim.c` Wait1/Wait2 choice (Codex T7): both Foxes' animation state matches the FD trace 600/600 ticks; all 9 idle-choice RNG draws match the ledger (`src/anim/README.md`)
- [x] `melee-ft`: human input path `Fighter_Spaghetti_8006AD10` + 21 `ftCo_Wait_IASA` predicates (Codex T9): 600 ticks x 2 fighters, 108 input bytes + 7 CPU fields match (`src/input/README.md`)
- [ ] `melee-ft`: `fighter.c` init and per-frame update order, `ftcommon.c`, `ftcoll.c`, `ftlib.c`
- [x] `melee-ft`: `fighter/` composes data+physics+collision+animation+input into `Fighter` with spawn/reset, the Wait callback table, all 15 scheduler proc methods and the 24-key Snapshot (Codex T10): `tests/idle_fox_600.rs` replays both Foxes 600/600 ticks, all 24 fields each, bit-exact. Cold spawn at FD's y=10 markers enters Fall (retail-derived: one jump used, 10-frame ECB lock, one CPU-init draw). Non-idle branches are explicit `unimplemented` with C lines.
- [ ] `melee-ft`: `ftCo_*` action states for squat and turn (Wait done)
- [ ] `melee-ft`: physics (`ft_08A1.c` etc): gravity, friction, ground snap
- [x] `ft-fox`: attributes (Fox special block, 0xD4 bytes) read from PlFx.dat (Codex T4); init and Wait wiring still to do
- [x] `melee-ft`: `desc/` reads ftData attributes (0x184 bytes, grouped by concept), part table (54 entries, TopN/TransN/XRotN/YRotN=0/1/2/3), ECB joints [41,55,25,13,7,4], bone lists, PlCo common subset (`docs/FOX_DATA.md`)
- [x] `melee-sim`: scenario/asset loading, `InitialState::from_savestate_traces`, `World`-driven frame loop, 49-key trace, `run`/`gate` CLI (Codex T13; `crates/melee-sim/src/M3.md` lists imported vs computed state)
- [x] Record savestate and `expected.jsonl`: **`idle_ys_fox`** (Yoshi's Story) instead of FD, two idle Foxes, 600 frames, byte-identical on rerun (`docs/DOLPHIN_RUN.md`)
- [x] `idle_fd_fox` recorded (2026-09-08): FD unlocked via `poke-or 0x8045BF2A u16 0xC0` (save data `gmMainLib_804D3EE0->thing.x186A`), Stock 1 via `GameRules` bytes, items were already NONE. Savestate `harness/roms/idle_fd_fox.sav` at frame 50841, both Foxes in Wait at (+-60, 0.0001, 0). Tick trace `harness/traces/idle_fd_fox.tick.expected.jsonl`, 600 ticks, deterministic.
- [~] `pl/player.c` and `gm` match setup: the slot->kind/costume subset spawning needs is in `fighter/spawn.rs`; stocks/percent bookkeeping still to do

## Milestone 4: Movement (`melee-ft`)

Gate: scripted input scenarios for walk, dash, run, jump, double jump,
fast fall, ledge grab, ledge options, platform drop, wavedash.

- [x] (2026-09-09) **Match start gate passed** `melee-sim gate harness/scenarios/start_fd_fox.toml`: 600 ticks x 49 keys, 0 divergences, all RNG produced by the port (effect id -> particle spawn mapping from eflib.c/efasync.c lives in `melee-sim/src/effects/` pending a `melee-ef` crate; FD's tick-1 spawn comes from melee-gr's animation track). `harness/scenarios/start_fd_fox.toml`: savestate at the first initialised frame (both Foxes in Entry 322 at y=10); tick trace, RNG ledger and 600-frame particle dump recorded. Sequence: 322 -> 323 (t6) -> 324 (t35, anim frame holds at 10 until GO) -> Fall 29 (t65) -> Landing 42 (t75) -> Wait 14 (t105); P1 +5 ticks. Entry warp effects spawn new particle generators (new RNG sites). Done: hsd-particle matches 600/600 (1,051,650 fields, 23,092 draws; 9 external spawn requests as a fixture); melee-ft Entry/Fall/Landing + airborne physics match the 24 fields 600/600 in the state-callback replay. Dynamic-bone solver `lb_8001044C` + `ftdynamics.c` ported (Fox tail, bones 17-20); `start_fox_600` passes 600/600 x 24 fields; **bone oracle passes**: all 73 bones' SRT for both fighters over 130 start ticks + 8 idle ticks (tick-aligned dump), and 202,020 matrix words vs post-render dumps (matrices are render-time caches, so they are compared against VI-sampled captures, see `melee-ft/src/dynamics/README.md`). Fixed en route: part-animation ownership must be cleared before the pose reset at motion entry. Next: melee-sim `gate start_fd_fox` (wire fighter/stage effect requests into hsd-particle spawns; import the entry-state boundary).

- [x] (2026-09-09) Scripted-input recording: `tick_trace.py` issues per-port holds from the scenario `inputs` (a top-level TOML key; the tracer and `Scenario` reject it under a `[[fighters]]` table) and records `HSD_PadGameStatus[4]` per tick; `decode.py` emits `record["inputs"]`; `validate_ticks --scripted` (run_scenario adds it automatically). Gotcha: Dolphin's `StickX 0.5` is raw 64, which HSD scales to 0.8 (dash threshold); `0.35` gives WalkMiddle.
- [x] (2026-09-09) Scenarios recorded from `idle_fd_fox.sav` (300 ticks, P1 scripted, P2 idle) with tick traces and RNG ledgers (`<name>.ledger.raw.jsonl`): `squat_fd_fox` (39 Squat t31, 40 SquatWait t38, 41 SquatRv t91, Wait t101), `turn_fd_fox` (18 Turn t31, 15 WalkSlow t42, 16 WalkMiddle t55, Wait t61), `walk_fd_fox` (15 t31, 16 t41, Wait t71), `dash_fd_fox` (20 Dash t31, 21 Run t42, 23 RunBrake t56, Wait t74), `jump_fd_fox` (24 KneeBend t31, 25 JumpF t34, 42 Landing t55, Wait t85; then 24/25 t101-104, 27 JumpAerialF t113 with fast fall from t135, 42 t152, 40/41 SquatWait/Rv t156-177, Wait t187). RNG: squat/turn/walk draw nothing new; dash and jump add particle sites (`hsd_8039DAD4+0x710/+0x900`, `hsd_8039930C+0x22D4/+0x1504/+0x31EC/+0x3280/+0x3564`, `hsd_8039F05C+0x1F4`) and `ftCo_8009F834+0x4A8/+0x4CC/+0x4F0` (3 draws per dust effect).
- [~] (2026-09-09) Codex M4-T1 implemented and verified, left uncommitted as requested: Squat/SquatWait/SquatRv, Turn, WalkSlow/Middle/Fast; all three movement gates 300x49, zero divergences; raw-field state replays pass; existing gates pass; `cargo gate` 550 passed / 0 failed / 1 pre-existing ignored; clippy clean. See `crates/melee-ft/src/fighter/M4_FOX.md`.
- [x] (2026-09-09) Codex M4-T2: Dash/Run/RunBrake (`ftCo_Dash.c`, `ftCo_Run.c`, `ftCo_RunBrake.c`) + the subaction GFX command and dust spawner `ftCo_8009F834` (fmadds at 8009FCF8/FD1C/FD44 audited): Dash GFX 0x3FF -> async kind 6 -> effect 5 -> particles 9 -> 7/8; Run 0x3FE -> kind 5 -> generator 263 -> 264/265; RunBrake 0x401 -> kind 5 -> generator 90. The missing particle behaviour was shared AppSRT inheritance (child generators keep the parent's transform until the last child dies). Gate `dash_fd_fox` 300x49; ordered particle RNG sites match all 300 ticks; report `melee-ft/src/fighter/M4_DASH.md`. Follow-up M4-T2b: `hsd-particle/tests/live_fd_dash.rs` replays the 300-tick dash particle dump with the port's own spawn requests as a fixture: 441,857 generator/particle/AppSRT fields match, 0 mismatches. Excluded, documented in the test: 532 AppSRT *display-cache* fields (`psDispSubAppSRT`, psdisp.c:1400-1439: cached model matrix, camera-view product, column lengths, status bits, `psFrameNum`), which need the camera and render schedule (Milestone 8); `hsd-particle/src/appsrt.rs` ports that display update with explicit view/frame inputs for later.
- [!] (2026-09-09) M4-T2 particle-dump follow-up: strict 300-tick replay compares 442389 fields; generator/particle and AppSRT SRT/ownership fields match, but 425 display-cache values differ from tick 50. Await display-time camera view matrices and psFrameNum/pass order, absent from the particle capture. New test and cargo gate remain red; existing M4/idle/start/live gates and clippy pass. No commit. See `crates/melee-ft/src/fighter/M4_DASH.md`.
- [x] (2026-09-09) Codex M4-T3: KneeBend/JumpF/JumpAerialF (Fox's basic double jump via the new `CharacterCallbacks::aerial_jump_style` hook; Ness/Yoshi/Peach/Mewtwo styles named, unported), fast fall (`ftCommon_CheckFallFast` 8007D528), Landing into SquatWait; dust: jump GFX 0x402 -> particle 89 and 0x403 -> 94 (async kind 0, live root joint, no offset RNG), landing 0x404 -> effect 0x18 -> particle 10. Gate `jump_fd_fox` 300x49; `hsd-particle/tests/live_fd_jump.rs` matches 489,588 dump fields with no exclusions (shared `tests/support/dust_replay.rs`); report `melee-ft/src/fighter/M4_JUMP.md`. Backward jumps, jump cancels and relaxed/C-stick jump entry stay explicit `unimplemented`.
- [x] (2026-09-09) Second scenario batch recorded from `idle_fd_fox.sav` (tick traces + ledgers): `shield_fd_fox` (182 GuardOn t31, 179 Guard t39, 180 GuardOff t91, Wait t106; 1 extra particle draw pair), `spotdodge_fd_fox` (182 t31, 235 EscapeN t35, 178 GuardSetOff t57, 179, 180, Wait t96; one dust spawn), `roll_fd_fox` (182 t31, 233 EscapeF t35 to x=-26.4, 178 t66, 179, 180, Wait t96; two dust spawns), `airdodge_fd_fox` (full hop, 236 EscapeAir at the apex t141, 43 LandingFallSpecial t186, Wait t196; the first hop's L during jump squat is too early to wavedash), `ledge_fd_fox` (420 ticks: backward full hop 26 JumpB t34 drifting past the left edge, 252 CliffCatch t70, 253 CliffWait t77, 262/263 CliffJumpSlow t232/t246 from stick up, 42 Landing t283, Wait t313). Lesson: a double jump does not turn the fighter around, so grabbing a ledge after running off it needs a backward hop (facing stays toward the ledge). A stock-1 death ends the scene and resets the tick counter (the tracer reports a discontinuity).
- [x] (2026-09-09) Codex M4-T4: GuardOn/Guard/GuardOff/GuardSetOff (`ftCo_Guard.c`: shield size/health/tilt as typed state, collision volumes, shield pose, attached shield particle) and EscapeN/EscapeF (`ftCo_Escape.c`: invincibility as typed state, roll velocity, ground collision); Yoshi/Marth/Samus branches are `CharacterCallbacks::guard_variant`/`escape_variant` hooks. Gates `shield_fd_fox`, `spotdodge_fd_fox`, `roll_fd_fox` 300x49; report `melee-ft/src/fighter/M4_SHIELD.md`. Shield stun/damage/powershield/break and grab out of shield stay explicit `unimplemented` (M5). Follow-up M4-T6: particle dumps of shield/spotdodge/roll/airdodge/wavedash/ledge replayed field-by-field (`hsd-particle/tests/live_fd_*.rs`, 3,014,914 fields, 0 mismatches; only the documented AppSRT display-cache exclusion); one fix in the snapshot adapter's AppSRT generator reference.
- [x] (2026-09-09) `wavedash_fd_fox` recorded: L + diagonal-down on the first airborne frame (VI 34, Fox's 3-frame jump squat): 25 JumpF t34 (y=2.1), 236 EscapeAir t35, 43 LandingFallSpecial t36, Wait t46 having slid from x=-60 to -44.4; repeated at t121-138.
- [x] (2026-09-09) Codex M4-T5: air dodge (`ftCo_EscapeAir.c`) and LandingFallSpecial (43); gates `airdodge_fd_fox`, `wavedash_fd_fox` 300x49. FallSpecial 35 is never reached in these scenes.
- [x] (2026-09-09) Codex M4-T5: JumpB, ledge grab during airborne collision (`ftcliffcommon.c`), CliffCatch/CliffWait, CliffJumpQuick1/2 (262/263 are the *Quick* pair; the dust at t246 is the launch); gate `ledge_fd_fox` 420x49; report `melee-ft/src/fighter/M4_LEDGE.md`. CliffClimb/CliffAttack/CliffEscape stay explicit `unimplemented`.
- [ ] Input system: `fighter.c` input parsing, buffers, deadzones (`ftCo_800A2040` CPU vs pad path)
- [x] (2026-09-09) Third scenario batch recorded (tick traces + ledgers): `turnrun_fd_fox` (20 Dash t31, 21 Run t42, 19 TurnRun t61 at x=5.6, 21 Run left t91, 23 RunBrake t101, Wait t119; six dust spawns), `walkfast_fd_fox` (gradual tilt 0.35/0.6/0.95: 15 t31, 16 t41, 17 WalkFast t54, Wait t76; RNG-quiet), `ledgeclimb_fd_fox` (420 ticks: ledge grab as in ledge_fd_fox, 255 CliffClimbQuick t232, 15 WalkSlow t266, Wait t272), `ledgeescape_fd_fox` (420: 259 CliffEscapeQuick t232, Wait t281 at x=-50.7).
- [x] (2026-09-09) Codex M4-T7: TurnRun (`ftCo_TurnRun.c`), WalkFast verified against its first retail trace, CliffClimbQuick/CliffEscapeQuick (`ftCo_CliffClimb.c`, `ftCo_CliffEscape.c`); gates `turnrun_fd_fox`, `walkfast_fd_fox` 300x49, `ledgeclimb_fd_fox`, `ledgeescape_fd_fox` 420x49 (17 scenario gates green); report `melee-ft/src/fighter/M4_TURNRUN.md`.
- [x] (2026-09-09) `airjumpb_fd_fox` recorded: full hop, then stick back + X in the air: 28 JumpAerialB t51 (y=35.5, facing kept), drifted past the left edge into 32 FallAerial t101 and a ledge grab 252/253 at t104/t111; RNG only particle updates.
- [x] (2026-09-09) JumpAerialB and FallAerial; gate `airjumpb_fd_fox` (M4-T9)
- [ ] RunDirect, skid, CliffClimbSlow/CliffEscapeSlow (the slow variants are chosen at 100%+ damage, so they wait for Milestone 5 combat)
- [ ] Jumps backward, air movement, L-cancel
- [ ] Ledge states (`ftcliffcommon.c` and `ftCo_Cliff*`)
- [ ] Shield, roll, spot dodge, air dodge (no hit interaction yet)
- [ ] Scenarios for each, recorded from Dolphin
- [x] (2026-09-09) Battlefield (Codex lane B1): `melee-gr/src/battle/`, `melee-sim/src/scene_stage.rs` stage dispatch, HSD light reader, particle opcode 0xB3; gates `idle_bf_fox`, `start_bf_fox` 600x49; particle replays 6,409,194 fields, 0 mismatches; `docs/BATTLEFIELD.md`.
- [x] (2026-09-09) Codex lane B2: platform landing, `ftCo_Pass` drop-through and one-way lines (`platform_bf_fox` 300x49); Yoshi's Story idle (`melee-gr/src/story/`, Randall and the Shy Guy schedule for the idle interval, spline support; `idle_ys_fox` 600x49, particle replay 51,303 fields). `docs/YOSHIS_STORY.md`. Full Shy Guy motion/hit/escape and a YS match start remain.
- [ ] Platform drop-through with an opponent on the platform; Yoshi's Story match start (needs a start_ys_fox recording)

- [x] (2026-09-09) **Second character: Marth (Codex M4-T8).** Characters and FD unlocked in RAM (`poke-or 0x8045BF28 u16 0xFFFF`; rules re-poked to stock 1 each session). Savestates `harness/roms/{idle,start}_fd_marth.sav` (P1 Marth kind 18 at -60, P2 Fox idle at +60) with tick traces, ledgers, particle and bone dumps. New `ft-mars` crate; `melee-ft` loaders take a `CharacterDescriptor` (files, symbols, part/animation counts); `melee-sim` scene holds a closed `SceneFighter` enum of boxed `Fighter<C>`; `melee-gr/src/music.rs` models the match-start alternate-music `HSD_Randi(100)` that only fires when all characters are unlocked (found via the ledger: `grLast`/`Ground_801C24F8+0x1B4`). Gates `idle_fd_marth`, `start_fd_marth` 600x49; bone oracle 90 Marth + 73 Fox bones, 202,672 SRT words. Report `melee-ft/src/fighter/M4_MARTH.md`, data `docs/MARTH_DATA.md`. Next: Marth variants of the 16 movement scenarios (recording).

- [x] (2026-09-09) **Marth variants of all 16 movement scenarios** recorded from `idle_fd_marth.sav` with the Fox input scripts (`<name>_fd_marth.toml`; airjumpb needed an earlier drift back, Marth's longer air drift carried the Fox timing to a death). **Without any code change 8 of 15 pass**: squat, turn, walk, dash, jump, wavedash, turnrun, walkfast (300x49 each). The rest stop at explicit boundaries: shield/spotdodge/roll at the Marth shield-offset hook (`ftCo_Guard.c:342-346`), airdodge at FallSpecial 35 (Marth's full-hop air dodge lands in special fall), ledge/ledgeclimb/ledgeescape at directional Fall (FallF/FallB from a residual x velocity after JumpB), and airjumpb (both characters) at JumpAerial animation end -> FallAerial 32.
- [x] (2026-09-09) Codex M4-T9: FallF/FallB (directional falls blend a secondary animation while keeping the Fall state), FallAerial, FallSpecial + LandingFallSpecial from an airborne air dodge, Marth's `guard_variant` override (ft-mars: shield model + sound selection, no custom offset). All 16 Marth movement scenes and both airjumpb scenes pass; `m4_gate` 65 tests; report `melee-ft/src/fighter/M4_FALLS.md`.

- [x] (2026-09-09) **Parallel lanes prepared.** `harness/record.py` records tick trace, ledger, particles and bones in one command; melee-sim characters register in one line (`scene_characters!`); three git worktrees under `../melee-lanes/{combat,battlefield,chars}` (CLAUDE.md "Parallel lanes"). Recorded: `jab_fd_marth` (first hit: Marth dash into Fox, push apart, jab at t121, Fox damage state 79 at t124 for 4%, Wait t150; ledger adds `ifStatus_802F4B84+0x1C0/+0x1D0`, the HUD percent shake), Battlefield `idle_bf_fox`/`start_bf_fox` (Fox vs Fox spawn at (0,0) and (0,54.4); idle ledger adds `hsd_8039DAD4+0x900` x232 from the stage's own generators), and Falco (kind 22), Captain Falcon (kind 2) and Peach (kind 9) idle/start plus all sixteen movement scenes each (airjumpb needs the Marth drift timing for all three; Falcon's three ledge scenes need the drift back at VI 60). Lanes launched: A1 combat (jab), B1 Battlefield, C1 Falco; Falcon and Peach queued in the character lane.

- [x] (2026-09-09) **Cold start (Codex lane B3).** `melee-sim/src/initial_state/cold.rs` builds the match-start state from `(stage, characters/costumes/ports, seed)` with no savestate: `start_fd_fox_cold`, `start_fd_marth_cold`, `start_bf_fox_cold`, `start_fd_falco_cold` all reach the recorded traces 600x49. Found and fixed en route: retail CPU initialisation draws twice (`ftCo_800A101C` + `ftCo_800B9704`); the first-session one-draw spawn expectation was self-authored and is corrected. `docs/COLD_START.md`. Next: Slippi replays as tests (lane B4).

- [x] (2026-09-09) Yoshi's Story match start and cold start (Codex lane B5): `start_ys_fox`, `start_ys_fox_cold` 600x49, particle replay 215,202 fields. The Fox-vs-Falco Slippi fixture is an online match (per-frame netplay RNG resets), so only offline replays can be compared.
- [x] (2026-09-09) Dream Land N64 (Codex lane B6): `melee-gr/src/pupupu/`, Whispy's wind schedule and gust on the fighters (the idle P2 is pushed from t519 and blown into Fall at t593), point lights, particle opcode A9, cold setup; gates `idle_dl_fox`, `start_dl_fox`, `start_dl_fox_cold` 600x49; particle replays 231,747 fields. `docs/DREAM_LAND.md`. Multi-minute flybys and auxiliary gust/camera effects remain.
- [~] (2026-09-09) Fountain of Dreams recorded (`idle_fod_fox`, `start_fod_fox`: fighters spawn on the side platforms at (+-41.25, 16.1)); its idle ledger is particle-heavy (~7,800 draws each at `hsd_8039930C+0x31EC/+0x3280` over 600 ticks: the fountain water). Lane B7 prompt drafted; paused for discussion.

## Steel thread: Fox vs Marth on Final Destination

Definition: a recorded human-vs-human match (four stocks, eight minutes,
FD) replays tick for tick on the 49 gated keys + RNG ledger + particle
replay, from match start through the GAME banner. Acceptance: (a) our own
Dolphin recording `match_fd_foxmarth`; (b) one offline Slippi replay of
the same matchup (user records with Slippi Dolphin, `docs/SLIPPI.md`).
Camera is out of gate scope (touches neither fighter state nor RNG). Fox
vs Fox is the fallback if Marth's moveset stalls. Design discussion and
code sketches: `docs/STEEL_THREAD.md`.

### Consolidation round (architecture first; existing gates prove behaviour unchanged)

- [x] (2026-09-09, merged) C1 **Motion state tables.** 261 M4 gates unchanged on the re-recorded data after the refactor; callbacks take explicit per-phase contexts (no thread-locals, no unsafe). Report `docs/PORT_NOTES/C1_STATE_TABLES.md`. `MotionRow<C>` = anim id, flags, move id, five `fn(&mut Fighter<C>)` callbacks (retail `MotionState`, `ft/types.h:853`). Common table (`ftData_MotionStateList`, 341 rows) built by `const fn common_table::<C>()` behind an associated const; `CharacterCallbacks::special_rows()` for rows >= 341 (`ftFx_Init_MotionStateTable`); `enter_special(slot, airborne)` hook called from the shared attack-input check (the `ftData_SpecialN`.. per-kind tables). Replaces the `AnimationCallback`/`InputCallback` enums and the `MotionState::CATCH`-style consts. Zero gate changes.
- [x] (2026-09-09, merged) C2 **Concrete core.** Hook-free fighter and fighter-pair helpers compiled once (pair helpers were 49 copies); melee-ft IR 361,940 -> 218,803 lines; clean release build 10.97 -> 8.16 s; stripped melee-sim 3,796,320 bytes on main (was 3,866,720 before the round). Report `docs/PORT_NOTES/C2_CONCRETE_CORE.md`. (Originally:) Split `Fighter<C>` into a non-generic `FighterCore` (physics, environment collision, animation player, subaction interpreter, hitboxes) plus the thin generic shell that calls hooks. Budget: `cargo llvm-lines` instantiation count of melee-ft code per character crate, recorded in `docs/PERF.md`.
- [x] (2026-09-10, merged) P1 **Revival without reconstruction; Yoshi duplicate codegen; fresh baseline.** Fighter owners (revival platform tree, tracks, part state) live from spawn and reset in place on death/revival: `ko_fd_marth` 690 -> 22 allocations (revival zero). ft-yoshi's 228 melee-ft copies were 189 common-graph specializations duplicated with melee-sim (its egg-shield overrides reached `change_motion_state`, materializing `C::COMMON` in the crate); tables are now bound once in `Fighter::prepare` (`SPECIAL_ROWS` const): 228 -> 22, total 2,796 -> 2,591. COMPLETE baseline: stripped 3,747,632, text 3,342,336, ~25,400 ticks/s; thresholds ratcheted. Report `docs/PORT_NOTES/P1_REVIVAL_YOSHI_BASELINE.md`.
- [x] (2026-09-10, merged) C15 **One concrete fighter shell with static character tables.** `Fighter` concrete; one `state::COMMON`; per-crate `static TABLE: CharacterTable` + `SPECIAL_ROWS`; checked aligned inline `CharacterState` (typed accessors; review note: an enum roster crate would remove the small unsafe core at the cost of a reverse dependency). melee-sim copies 2,112 -> 128, all crates 2,754 -> 1,440, stripped 3,916,784 -> 3,483,104, ~25,500 ticks/s, clean release build 10.4 -> 9.7 s; zero behaviour change (M4 261, M5 28, 1,017/0 both profiles); perf-gate PASS, baseline promoted. Report `docs/PORT_NOTES/C15_CONCRETE_SHELL.md` (migration notes for S2..S10). (Originally:) (design: `docs/PORT_NOTES/C15_DESIGN_CONCRETE_SHELL.md`). melee-sim still emits ~2,015 per-character copies of the generic shell (104 `Fighter<C>` methods x 7, 101 motion callbacks x 7, 4 pair helpers x 49); one concrete body plus a per-character static fn-pointer table removes ~1,400 definitions and the character-product for pair helpers (26 characters would otherwise mean 676 copies each). Budget: <= 900 census copies in melee-sim, one definition per common label. Schedule right after S1 and S4 merge (touches every callback signature), before S2/S3 launch; re-measure on the merged tree first.
- [x] (2026-09-10, merged with S4) C3 **Family crates.** `ft-fox-family` (retail `ftFx_`, shared by Fox and Falco) with a `FoxFamily` trait (laser/ghost item kinds, sound ids, attributes accessor, typed per-move scratch). `ft-fox` and `ft-falco` depend on it, never on each other. Same pattern later for Marth/Roy, Mario/Dr. Mario, Pikachu/Pichu, Link/Young Link. Lands with S4 (core lane, launched 2026-09-10 with Fox's SpecialN rows).
- [x] (2026-09-09, merged) C4 **`melee-ef` crate.** Effect layer with fixed request/instance/model pools; melee-ft fills a request queue; melee-sim only wires the update; allocations 22,588 -> 21,672 on its base. Report `docs/PORT_NOTES/C4_MELEE_EF.md`. (Originally:) Effect layer out of melee-sim/melee-ft (`effects.rs`, `effects/dust.rs`, death effects) into its own crate with fixed pools. After C1 merges (both touch the motion-change path).
- [x] (2026-09-09, merged) C5 **No per-tick allocation.** C5-b: reusable buffers and fixed pools in melee-ft anim/ECB/root motion, hsd-particle storage and banks, hsd-anim event buffers, melee-gr stage matrices, melee-lb; `start_fd_fox` 22,589 -> 1,694 allocations per 599 ticks (2.8/tick), five scenes with ratcheting ceilings (`alloc_gate`); the remainder is in the command/hitbox path (C8) and the diagnostic snapshot (125/tick, reported not asserted). First COMPLETE `docs/PERF.md` block. Report `docs/PORT_NOTES/C5B_ZERO_ALLOC.md`.
- [x] (2026-09-09, merged; first `docs/PERF.md` block still INCOMPLETE: rerun `tools/perf-gate.sh` after C14 makes release exact) C6 **Performance gates.** Criterion bench (ticks/s headless on `start_fd_fox`, 600 ticks), stripped binary size (`cargo bloat` per crate), `cargo llvm-lines` budget; all regression-only, none bit-exact; `tools/perf-gate.sh`, results in `docs/PERF.md`. Baseline 2026-09-09: stripped `melee-sim` 3.9 MB, text 3.6 MB, `gate start_fd_fox` 0.25 s CPU incl. savestate load and compare.
- [x] (2026-09-09, merged with C2) C7 **Kind checks out of trait default bodies.** Every kind branch in the trait defaults is descriptor data or a hook; kind checks remain only in asset metadata. (Originally:) The interim `if kind == ..` checks inside `CharacterCallbacks` defaults become hooks or attribute data. After C1.
- [x] (2026-09-10, merged `b33c49b`) C8 **Shared subaction interpreter and collision crates.** `melee-cmd` (decode + `ScriptState`, depends only on melee-types/gekko-math) and `melee-coll` (capsules, `PairCursor` detection, audited geometry from melee-lb, knockback/hitlag, reflect/absorb, overlap); shared payloads in `melee-types::combat`, inline storage in `melee-types::fixed`. Both scheduler `Rc<RefCell>` owners removed (procs carry a dispatch tag; `World::run_procs_with`). Allocation ceilings 1,694/2,116/639/1,517/1,431 -> 17/1/12/690/17 (start_fd_fox/idle_fd_fox/jab_fd_marth/ko_fd_marth/start_bf_fox; the interpreter allocates zero, the rest is particle storage and revival). Every cmd/coll function one LLVM copy; stripped 3,864,176 after fixing a part-joint size regression. Gates 977/0 in both profiles. Report `docs/PORT_NOTES/C8_CMD_COLL.md`. (Originally:) extracted from melee-ft so items can use them. Prerequisite for S4; after C2.
- [x] (2026-09-09) C9 **Item oracle.** Merged (`5a9f7ac`); `laser_fd_fox` recording pending data. Tracer records the item GObj list each tick (Item struct bytes, owner, kind, position/velocity, state) beside fighters; decoder and validator keys for items; a laser scene (`laser_fd_fox`) recorded once the tracer lands. Needed before S4.
- [x] (2026-09-09, harness lane, merging) C12 **Import from any scheduler boundary.** 26 idle/start gates and 261 M4 gates pass on the new data; report `docs/PORT_NOTES/C12_RESUME_ANY_BOUNDARY.md`. The savestate importer accepts only link 24, (4, 8006B82C) with Wait, and (14, 8006D1EC); the re-created savestates land on links 1, 6, a stage proc at 4, and mid fighter creation. Generalise to (s_link, proc, position); tests = the ten idle/start savestates that fail today.
- [x] (2026-09-09, merged) C13 **Fixtures regenerate by command; no recording constants in tests.** `melee-sim fixture-spawns` with a zero-cost event sink; 23 fixtures regenerated; boundary-aware melee-ft tests. The ef-library hit-spark draw (site 0x80063770) is now consumed once from the fixture; all 75 hsd-particle tests pass. The Falcon partial-emission replay no longer assumes the old savestate's interrupted emission. `melee-sim fixture-spawns` replaces the README's "temporarily instrument and re-run" recipe; hardcoded seeds/frames/draw counts copied from recordings become sidecar/trace reads or invariants.
- [x] (2026-09-09, merged) C14 **Release profile bit-exact.** Fused results are negated via `copysign` of the complemented bits so LLVM cannot fold the sign into the FMA; signed-zero unit tests; `tools/check-release-math.sh`; `cargo gate --release` in the gate list; 261 M4 gates in both profiles. Report `docs/PORT_NOTES/C14_RELEASE_EXACTNESS.md`. (Originally:) Found by C12 (2026-09-09): LLVM folds `-fma(a,c,-b)` into `fma(-a,c,b)`, flipping the sign of an exact zero; `gekko-math` `fnmsubs` (fma.rs:38) therefore differs between debug and release (first divergence `utilt_fd_marth` frame 147, `p1.kb_vel.y` -0 vs +0). Every gate so far ran in debug; the release build (bench, perf-gate) is unverified: 21 failing release targets incl. the hsd-anim native-C oracle. Fix the fused helpers with an unfoldable construction, add signed-zero unit tests, run the workspace in release, add `cargo gate --release` to the gates and merge-check.
- [x] (2026-09-09, with C13's kind-6 dispatch) S1-pre **`jab_fd_fox` needs ef particle 0/6.** M5 8/8 in both profiles. (Originally:) On the re-recorded scene fighter 1's HitDetection at frame 108 requests an effect kind the allowlist (`melee-sim/src/effects.rs:57`) does not implement (the one-tick boundary shift changed which hit spark fires). Small combat item; first S1 task.
- [x] (2026-09-09, merged) C11 **Gates must not pass on missing data.** `melee-test-support`: oracle tests fail naming the producing command unless `MELEE_ALLOW_MISSING_DATA=1`; merge-check refuses permissive mode; strict workspace 953 passed / 1 failed (the legacy M2 bone oracle: the new VI capture landed before matrix setup, 60/73 dirty bones; being moved to the aligned post-render model on the C11 thread). Report `docs/PORT_NOTES/C11_NO_SILENT_PASS.md`. (Originally:) The m4/m5 gate tests return early and report `ok` when `harness/traces`/`harness/roms` are absent (found 2026-09-09 when the C1 lane ran green against empty directories). Make a missing trace a hard failure, or an explicit `ignored` with the path in the message, so a green chain always means the oracle ran. Add the same check to `tools/merge-check.sh`.
- [x] (2026-09-09, merged) C10 **Reports and CI.** `tools/merge-check.sh` refuses tracked/touched roms, traces, decomp paths and an empty traces dir; reports live in `docs/PORT_NOTES/`. `melee-ft/src/fighter/M4_*.md`, `M5_*.md` move to `docs/PORT_NOTES/`; new lane reports go there directly; `tools/merge-check.sh` runs the strict merge chain (build errors, gate/test failures, clippy, fmt all block) so it is not something only Claude runs by hand.

### Combat table (each row: Dolphin scenes recorded by Claude, ported by Codex, gated)

Recorded 2026-09-10 (all from `idle_fd_fox.sav` / `idle_fd_marth.sav`, P2 Fox idle at +60; tick trace with items, ledger, particles):
- S3 specials (11): `illusion`, `firefox`, `reflector`, `reflectorjc`, `airillusion`, `airfirefox`, `airreflector` (`_fd_fox`); `shieldbreaker`, `dancingblade` (four hits: 349/351/352/357), `dolphinslash`, `counter` (`_fd_marth`; Counter catches Fox's scripted dash attack). Illusion, Shield Breaker and Dolphin Slash connect.
- S1 ground attacks (19): `dashattack`, `ftilt`, `ftiltup`, `ftiltdown`, `dtilt`, `fsmashcharge`, `usmash`, `dsmash`, `jabcombo` for both, `utilt_fd_fox`. Fox's rapid-jab kick (Attack100), Marth's two-hit jab; Marth's angled tilts resolve to neutral (Fox covers AttackS3Hi/Lw). Most connect; Marth's usmash, fsmash and Dolphin Slash KO the target.
- S2 aerials (17): `nair`..`dair` for both (Marth's nair/fair/bair/uair autocancel into Landing 42), `nairlc`..`dairlc` (`_fd_fox`), `dairlc_fd_marth`, `fairlc_fd_marth` (halved landing lag observed).
- S5 hit reactions (8, recorded 2026-09-10): `di_upaway_fsmash`, `di_downin_fsmash` (DI + ASDI; down-in turns the launch into DownBoundU 183 -> DownFowardD 196 and Fox survives), `sdi_fsmash` (three taps, +18 units during hitlag), `cc_ftilt` (SquatWait 40 -> DamageHi2 76 -> Landing 42 -> 40), `tumbledi_dolphinslash` (`_fd_marth`); `getupattack`/`getupstand`/`getuproll_fd_fox` (DownWaitU 184 -> 187/186/188; the getup attack knocks Marth down).
- S6 shield (5): `shieldstun_ftilt`, `shieldtilt_ftilt` (same states, tilted shield position), `powershield_ftilt` (hit lands while in GuardReflect 182: the powershield window), `lightshield_ftilt` (TriggerLeft 0.5: GuardOn 178), `shieldbreak` (`_fd_marth`, 520 frames: full Shield Breaker into a held shield -> ShieldBreakFly 205 -> 207 -> 209 -> Furafura 211). Retail: pressing L from Wait enters GuardReflect 182 first, then Guard 179.
- S7 grabs (5): `fthrow` (ThrowF 219 / ThrownF 239 -> DamageAir3 86 -> off stage), `uthrow` (221/241 -> DamageFlyTop 90 -> knockdown), `dthrow` (222/242), `pummel` (CatchAttack 217, CaptureDamageLw 228, then CatchCut 218 on grab timeout), `grabmash` (CatchCut/CaptureCut at vi138) (`_fd_marth`).
- S8 ledge (3): `ledgeattack` (CliffAttackQuick 257), `ledgejump` (CliffJumpQuick1/2 262/263), `ledgeroll` (CliffEscapeQuick 259) (`_fd_fox`). Also: the 252/253 seen on the target in `utilt`/`dsmash_fd_marth` are CliffCatch/CliffWait (Fox is pushed off the edge and grabs the ledge), already ported.
- S10 match flow (1, recorded 2026-09-10): `match_fd_marth_scripted` (1,600 ticks from the new stock-4 `idle_fd_marth4.sav`): Marth KOs Fox four times (vi 229, 654, 1076, 1528) with charged fsmashes; after each respawn Fox drops from the platform on a stick tap (RebirthWait 13 -> Fall 29), walks slowly to the left edge and teeters (Ottotto 245 -> OttottoWait 246), and is smashed off once his respawn invincibility (~120 frames after the drop; a smash 69 frames after landing whiffed) expires. Marth is pushed to the edge and teeters too in stock 4. The GAME banner is on screen for the last 72 ticks; the scene reset (tick counter -> 0) comes ~113 frames after the last KO, so recordings must end before it. Lessons: a stock-1 savestate ends the match on the first death; a tipper fsmash at 0% flies ~25 units before touching down, so the launch point must be within 25 units of an edge.
- S5/S9 high percent (5, recorded 2026-09-10 from `idle_fd_marth4_fox200.sav`: Fox's fighter percent x1830 and player-block damage RAM-poked to 200; the port imports both from the savestate): `hi200_jab` (DamageN3 80, Fall, off the edge), `hi200_uthrow`/`hi200_uthrow2` (ThrowHi at 200%: DamageFlyTop 90 -> tumble at y=115 -> DownBoundD 191 -> DownWaitD 192; no top KO), `hi200_utilt` and `hi200_dolphinslash` (DamageFlyHi 87 / DamageFlyN 88 at 200% -> DeadRight 2 at x=250; side blast zone, 420 frames incl. respawn). Lesson: knockback follows x1830 (the jab launch doubled), but Marth's up smash sourspot is set-knockback (identical launch at 8% and 208%). A top KO needs a steeper launch: Fox's up smash next.
- S9 top blast zone (2, recorded 2026-09-10 from `idle_fd_fox_p2_200.sav`: P2 Fox at 200%, stock 1, 230 frames so the recording ends before the match-end scene reset): `topko_usmash_fd_fox` (Fox up smash: DamageFlyTop 90 -> DeadUpStar 4 at y=191, star KO) and `topko2_usmash_fd_fox` (up smash four frames later: DeadUpFall 6 -> DeadUpFallHitCamera 7, screen KO). The star/screen choice is the RNG draw at the top-blast death.
- Not recordable on FD: wall tech, wall jump (no walls); meteor cancel needs an airborne target (later).
- Scripting facts learned: Dolphin stick 0.3 is about 0.48 game units (walk/tilt), 0.5 about 0.8 (dash/smash), 0.55 vertical is a tap jump; A on the same frame the stick moves is a smash input, so tilts hold the stick six frames first; side-B during run-brake is ignored (press while running); Dancing Blade's fourth window opens ~27 frames into the third hit; Marth's run is slower than Fox's, so his approach runs to VI 86.


| # | Area | Have | Missing | Est. tasks |
|---|---|---|---|---|
| S1 | Ground attacks | **done 2026-09-10** (merged): dash attack, tilts with angles, up/down smash, smash charge, jab combos and rapid jab; 19 scenes gated both profiles; face-up knockdown rows; report `docs/PORT_NOTES/S1_GROUND_ATTACKS.md`. Perf debt: melee-sim melee-ft copies 2,015 -> 2,114 (new `fn(&mut Fighter<C>)` callbacks x 7 characters), stripped 3,747,632 -> 3,797,952; assigned to C15 | done |
| S2 | Aerials | **done 2026-09-10** (merged `86f0dfb`): five aerials each, LandingAir lag, L-cancel (x67F < PlCo E4, lag/E8 truncated, min 1), autocancel, running KneeBend entry, plus the ground-pose leg IK (`fn_8008998C` endpoint fallback, `lbBgFlash_80021410` two-joint solver in `melee-lb/src/ik.rs` with a native-C reference) that Marth's landings needed; 17 scenes exact both profiles. Also: perf census now gates duplicate definitions (labels emitted more than once), totals informational. Follow-up: record a rendered-matrix bone oracle for `fairlc_fd_marth` (endpoint-IK ticks). Report `docs/PORT_NOTES/S2_AERIALS_LCANCEL.md` | done |
| S3 | Specials | **done 2026-09-10** (merged): Fox Illusion (ground/air, `it-foxillusion` ghost), Fire Fox (ground/air), Reflector (loop, release, jump cancel, air; reflect descriptor data; particle opcode 0xB8), Marth Shield Breaker, Dancing Blade, Dolphin Slash, Counter (graphics dispatch at link >= 9 now immediate, as retail); `melee-lb/radial_force.rs` for the wind/radial dynamics commands. All 11 scenes + `tumbledi_dolphinslash` exact both profiles; M5 65; workspace 1,076/0; perf-gate PASS (cross-crate duplicate baseline ratcheted 99 -> 100 for an auto-inlined leaf, reviewed JSON is now the documented ratchet). Report `docs/PORT_NOTES/S3_SPECIALS.md` | done |
| S4 | Projectiles/items | **done 2026-09-10** (merged): `melee-it` (SpawnItem, fixed storage, static ItemStateRow/ItemLogic fn-pointer rows, retail s-links, Item_802697D4 physics, melee-mp spawn collision), `it-foxlaser` (laser + blaster tables shared by Fox/Falco), `ft-fox-family` (FoxFamily trait, SpecialN rows 341..346; C3 done), item comparator (items.count + 12 keys/entry; `laser_fd_fox` 300 ticks, 62 keys, 0 divergences both profiles); report `docs/PORT_NOTES/S4_ITEMS_LASER.md`. Merge unified the two lanes' extra hit-spark request on S1's `NormalSparkExtra` (also listed in `direct_draws`). Perf debt to C15: stripped 3,916,784 (+4.5% vs P1 baseline) | done |
| S5 | Hit reactions | **done 2026-09-10** (merged): DI at launch, SDI taps, ASDI, crouch cancel, damage levels by knockback, DownBound/DownWait/DownStand/DownAttack/DownFoward rows (face-up and face-down); 7 scenes exact, `tumbledi_dolphinslash` explicitly ignored until S3 part 2's Dolphin Slash. Not on FD: wall tech, wall jump; meteor cancel deferred (needs an airborne high-percent target). Report `docs/PORT_NOTES/S5_HIT_REACTIONS.md` | done |
| S6 | Shield | **done 2026-09-10** (merged; Codex after the credit reset): physical powershield (`ftColl_80076CBC`: attacker hitlag/pushback, defender impact without accumulated damage, GuardOff interrupt window, effect 27, sound 104), exhaustion break, ShieldBreakFly/DownU/StandU and Furafura rows (`shield_break.rs`), ShieldBreakFly's x2222_b3 top exit (replaced the S9 stop). All five scenes exact incl. the 520-tick shield break; M5 75; alloc zero; perf-gate PASS (3,620,912 stripped, duplicates unchanged). Unrecorded, explicit: projectile reflection, ShieldBreakFall 206. Report `docs/PORT_NOTES/S6_SHIELD.md` | done |
| S7 | Grabs | **done 2026-09-10** (merged; Codex): four throws (thrown releases into the airborne damage path; ftCo_800DE7C0's argument is a motion override), pummel, grab escape (timer, mash, CatchCut/CaptureCut), captured damage (CaptureDamageLw 228); forward throw's last bit was the thrown animation's declared source skeleton 33 (remaps now use the animation's own source table). fthrow/uthrow/dthrow/pummel/grabmash and both 200% up throws exact | done |
| S8 | Ledge | **done 2026-09-10** (merged with S7): quick ledge attack row; ledge jump and roll exact | done |
| S9 | KO/respawn | **done 2026-09-10** (merged, ported directly): heavy/medium damage sounds + voice draw, DamageFlyRoll draw, blast-zone dispatch in retail order, DeadLeft/DeadRight (clamped, rotated explosion), top-exit knockback test, the star/screen `HSD_Randi(100)+1` vs PlCo+520 roll, DeadUpStar entry + flight (fmsubs aim, 130 frames), DeadUpFall entry; `hi200_jab`/`hi200_utilt`/`hi200_dolphinslash`/`topko_usmash` exact. Explicit stops: star vanish (needs a 400-frame `topko_usmash_fd_fox` recording), screen-KO approach (camera port: retail places the fighter through the camera inverse view matrix), ShieldBreakFly x2222_b3 (S6). `hi200_uthrow*` wait on S7. Report `docs/PORT_NOTES/S9_KO_VARIANTS.md` | done |
| S10 | Match flow | **done 2026-09-10** (merged): `match_fd_marth_scripted` 1,600 ticks exact (four stocks: KO, death, respawn platform, RebirthWait drop on input with revival invincibility, walk, teeter Ottotto/OttottoWait incl. the fighter-overlap nudge that snaps the teetering fighter to the edge vertex, charged smash KOs, GAME window). Timer/sudden death/stale queue as exercised by the scene; report in the commit and `docs/PORT_NOTES/S10` to follow | done |
| S11 | Acceptance | **steel thread reached 2026-09-10** (user decision): both recorded human Fox-vs-Marth matches bit-exact end to end, `match_fd_foxmarth` 6,083 ticks and `match2_fd_foxmarth` 10,059 ticks (seven KOs), 62 keys, 0 divergences, ordered particle draws, zero simulate-only allocations; P2 merged (FD background matrices published only where read; P2 faster than main in every loaded A/B round; quiet-host re-benchmark waived by the user). Reports `docs/PORT_NOTES/S11_HUMAN_MATCH.md`, `P2_FD_MATRICES.md` | open, off the critical path: the user's offline Slippi replay (`docs/SLIPPI.md`); S9's camera-space screen-KO approach (needs the camera port) | done |

Effects and sound RNG for every new state are folded into each row (about
a third of the cost so far). Budget one unknown-unknown per row.

## Milestone 5: Combat (`melee-ft`, `melee-lb`)

Gate: two-fighter scenarios with hits, shields, grabs, KOs.

- [x] (2026-09-09) **First hit (Codex lane A1):** `jab_fd_marth` gate 300x49: Marth Attack11 into idle Fox with fighter overlap push before it; typed hitboxes from the subaction commands, hurtboxes, hit detection in retail pair order, hitlag, damage, knockback, hitstun, Fox DamageN2 -> Wait; slash effects; new `melee-if` crate for the HUD percent-shake RNG (`ifstatus.c`, s_link 17). Particle replay 467,132 fields. Report `melee-ft/src/fighter/M5_HIT.md`. Next (A2, scenes scripted, recording pending): attacker swap `jab_fd_fox`, `fsmash_fd_marth` launch/tumble, `shieldhit_fd_marth`, `grab_fd_marth`.
- [x] (2026-09-09) Combat batch 2 (Codex lane A2): `jab_fd_fox` (Fox as attacker), `utilt_fd_marth` (AttackHi3 launch, DamageHi, landing, slide into the ledge grab), `shieldhit_fd_marth` (GuardDamage, shield damage/stun, pushback) 300x49 each with particle replays; grab startup (ticks 0-126) with the pair states left for A3. Report `M5_COMBAT2.md`.
- [x] (2026-09-09) Codex lane A3: grab/throw (pair selection, CatchPull/CatchWait/ThrowB, Fox constrained to the throw bone, DownBoundD/DownWaitD), tech roll (PassiveStandB), KO/death/respawn/stocks (`ko_fd_marth` on the stock-2 boundary: smash charge frame, integer-damage knockback term, blast zone, DeadDown, 60-tick death delay, revival platform, RebirthWait; death effect tornado/rectangle emitters and child generators; death HUD digit draws and the stock-icon generator in `melee-if`). Gates `grab_fd_marth` 300x49 (502,342 particle fields), `tech_fd_marth` 300x49 (496,564), `ko_fd_marth` 480x49 (872,466). Comparator fix: throw sound severity/kind at HitCapsule +0x38/+0x3C. Report `M5_COMBAT3.md`.
- [x] Throw raw-scratch blocker resolved in the C-stick throw packet: restore owner before initial damage-animation commands and disable collateral hitboxes below retail knockback threshold. Both old tests are re-enabled in `melee-lib/src/frame/combat.rs` and pass in debug/release. Expected bytes unchanged.
- [ ] Hitbox/hurtbox system breadth (`ftcoll.c`, `lbcollision.c`, `ftcolanim.c`): item and projectile collision, multi-hitbox priority
- [ ] Damage, knockback, hitlag, hitstun, DI, SDI, ASDI
- [ ] Shield damage and stun, powershield, shield break
- [ ] Grabs, throws, grab escape
- [ ] Tech, missed tech, getups
- [ ] Death and respawn, stock counting
- [ ] Attack action states for the common set (jab, tilts, smashes, aerials)
- [ ] Fox full moveset (`ft-fox`, 3829 lines) including specials
- [ ] Second character for matchup testing: Marth (`ft-mars`, 2184 lines)
- [ ] Item-free CPU AI (`melee-cpu`): `ftCo_0A01.c` (8.5k), `ftcpuattack.c` (2.7k), `ftcmdscript.c` (394)

## Milestone 6: Breadth

Gate: per-character and per-stage scenarios; every playable character idle,
moving, and fighting bit-exact.

Characters (decomp lines, `ft/kinds/`). Check off when the character's own
crate passes its scenarios.

| Character | Crate | Lines | Status |
|---|---|---|---|
| Fox | `ft-fox` | 3829 | `[x]` idle/start/17 movement scenes on FD and Battlefield; jab defender (A1) |
| Marth | `ft-mars` | 2184 | `[x]` idle/start/16 movement scenes; first hit as attacker (M4-T8/T9, A1) |
| Mario | `ft-mario` | 1257 | `[ ]` |
| Dr. Mario | `ft-drmario` | 399 | `[ ]` |
| Luigi | `ft-luigi` | 1922 | `[ ]` |
| Captain Falcon | `ft-captain` | 1705 | `[x]` idle/start/16 movement scenes (2026-09-09, lane C2) |
| Falco | `ft-falco` | 503 | `[x]` idle/start/16 movement scenes (2026-09-09, lane C1) |
| Sheik | `ft-seak` | 2858 | `[ ]` |
| Zelda | `ft-zelda` | 2218 | `[ ]` (transform pair with Sheik) |
| Peach | `ft-peach` | 2195 | `[x]` idle/start/16 movement scenes incl. her double jump hook (2026-09-09, lane C3) |
| Jigglypuff | `ft-purin` | 2533 | `[x]` idle/start/16 movement scenes through the shared F1-F5 multi-jump path (2026-09-09, lane C5) |
| Pikachu | `ft-pikachu` | 2377 | `[ ]` |
| Pichu | `ft-pichu` | 421 | `[ ]` |
| Samus | `ft-samus` | 1846 | `[ ]` |
| Link | `ft-link` | 1865 | `[ ]` |
| Young Link | `ft-clink` | 553 | `[ ]` |
| Donkey Kong | `ft-donkey` | 2156 | `[ ]` |
| Bowser | `ft-koopa` | 1988 | `[ ]` |
| Ness | `ft-ness` | 5677 | `[ ]` |
| Yoshi | `ft-yoshi` | 3364 | `[x]` idle/start/16 movement scenes through the egg-shield, escape and armoured-jump hooks (2026-09-09, lane C4) |
| Mewtwo | `ft-mewtwo` | 2703 | `[ ]` |
| Mr. Game & Watch | `ft-gamewatch` | 3738 | `[ ]` |
| Roy | `ft-emblem` | 454 | `[ ]` |
| Ganondorf | `ft-ganon` | 354 | `[ ]` |
| Ice Climbers | `ft-popo`, `ft-nana` | 2415 + 1078 | `[ ]` (Nana AI in `melee-cpu`) |
| Kirby | `ft-kirby` | 18190 | `[ ]` (copy abilities; largest by far, split by hat) |
| Master Hand | `ft-masterhand` | 4067 | `[ ]` |
| Crazy Hand | `ft-crazyhand` | 3879 | `[ ]` |
| Giga Bowser | `ft-gigakoopa` | 391 | `[ ]` |
| Sandbag, Wireframes | `ft-sandbag`, `ft-zako` | 132 + 135 | `[ ]` |

Common fighter code shared by all: `ft/kinds/ftCommon` (30.4k lines) and
`ft/*.c` (35.3k lines). Split `melee-ft` when it passes 30k.

Stages (`gr/`, 56k lines across 77 files). Every stage's decorations must
be ported for `rng.seed` parity, not just its collision; known per-tick RNG
consumers are listed so they are not forgotten:

| Stage | File | Status | Known RNG / dynamic elements |
|---|---|---|---|
| Final Destination | `grlast.c` | `[~]` (M3) | background lights/flicker, transitions (1-61 draws/tick) |
| Battlefield | `grbattle.c` | `[ ]` | background; unlocked alongside FD |
| Yoshi's Story | `grstory.c` | `[ ]` | Randall (moving platform + puff particle generators, `ef`), Shy Guys (`itheiho`, `it/itzako`), see `docs/M3_PLAN.md` §3; `idle_ys_fox` savestate + traces exist |
| Dream Land | `grpura.c` | `[ ]` | Whispy wind timing, Bronto Burts |
| Fountain of Dreams | `grizumi.c` | `[ ]` | platform height schedule, background |
| Pokemon Stadium | `grpstadium.c` | `[ ]` | transformations, background screen |
| Others (Kongo Jungle, Corneria, Brinstar, ...) | | `[ ]` | per stage |
| Target stages `grt*`, Adventure routes `gr*route` | | `[ ]` | last |

Items (`it/`, 17k core + 57.5k in `it/kinds` across 166 files):
`[ ]` core item system (`item.c`, `itcoll.c`, `ithitbox.c`, `itdrop.c`),
`[ ]` character projectiles needed by M5 movesets (Fox laser, etc), then
`[ ]` common items by family. Split `melee-it` into `it-<family>` crates.

CPU AI (`melee-cpu`): `[ ]` full port including item logic and all
character-specific branches.

## Milestone 7: Replay corpus

Gate: zero divergence over thousands of Slippi replays.

- [x] `slp` crate: parse replays, emit scenarios and expected traces
- [x] (2026-09-09) Codex lane B4: `melee-sim replay <file.slp>` cold-starts the replay's match, replays its pads, compares post-frame fields per tick; seed alignment (Slippi frame-start seed vs tick-end snapshot) and port-gap mapping implemented; verified on recorded scenes (599/599 frames, 1,800 pad samples). All eight peppi fixtures run but match 0 frames: each needs an unported character (Ice Climbers, Pichu, Ganondorf, Samus), stage (Fountain, Stadium, Dream Land), a CPU opponent, or Slippi Online setup. `docs/SLIPPI.md` gives the recipe for a qualifying offline Fox-vs-Fox FD replay (needs Slippi Dolphin to record).
- [x] (2026-09-09) `rng.seed` timing resolved (lane B4, `docs/SLIPPI.md`)
- [x] (2026-09-09) Slippi ports mapped to fighter-list order incl. gaps (lane B4)
- [ ] Verify `self_vel`/`kb_vel` field mapping against Slippi's recording code
- [ ] Batch runner: N replays in parallel, aggregate first divergences by function
- [ ] Triage tooling: given a divergence, print the phase, the fighter's action state, and the likely decomp file

## Milestone 8: Platform (out of scope until M7 is green)

- [-] Rendering via wgpu, emulating enough GX for HSD display lists
- [-] Audio
- [-] Menus (`mn/`, 32.8k), HUD (`if/`, 10k), trophies (`ty/`, 12k), cutscenes/video (`vi/`), camera (`cm/`, 4.7k), effects (`ef/`, 4.2k), debug (`db/`)
- [-] Game modes beyond Versus (`gm/`, 54k): Classic, Adventure, All-Star, Event, Target Test, Home Run Contest

## Session log

- 2026-09-11 (post-hitstun input): five attack panics and two priority/movement divergences resolved. Seven new 300-tick oracles and all zero-allocation checks pass; full debug/release each 1,213/0 with 3 existing ignores, clippy and 220 harness tests pass. Rebuilt native smoke and 233-tick exported replay pass. Size audit recorded separately (`e00b442`); next: Reflector turn and aerial jump cancel, with six captures and root-Y bone evidence prepared.

- 2026-09-11 (diagonal smash): authored-angle fallback and main-stick/C-stick priority, including dash entry, now exact in four directed Fox/Marth scenarios. Full debug/release gates each 1,205/0 with 3 existing ignores, all-target clippy and 220 harness tests pass. Committing at the user-requested boundary; next: post-hitstun attacks, tumble input priority and air-dodge knockback decay.

- 2026-09-11 (matchup packet 2): aerial Counter, aerial hit response, preserved landing and counterpart support-loss transitions implemented. Four 300-tick Dolphin scenarios and raw window/volume/damage/jump/ECB scratch checks pass; all three simulate-only allocation scenarios remain zero. Release workspace 1,200/0, clippy clean; original UI fault reproducer now succeeds. Data mirrored. Natural-boundary commits now authorized; packet 1 committed `4e08fb7`. Continue diagonal smash/input precedence next while keeping performance debt visible; user explicitly prioritized ongoing gameplay coverage.

- 2026-09-11 (matchup packet 1): added bounded exact-input native recordings, automatic first-fault export, Cmd-S export and a renderer-free `melee-replay` consumer; audited reachable Fox/Marth gaps. Ported ordinary laser/shield damage and deflection from retail ASM, with four new 300-tick Dolphin scenarios (including actual upward deflection), shield scratch comparisons and zero-allocation checks. Debug workspace gate 1,192/0 plus final focused checks; final release 1,194/0, three pre-existing ignores; clippy, 220 harness tests, six math opt levels, native smoke and headless replay pass. Serial perf remains REGRESSION (3,926,592 bytes, 184.015 ms load, 26.036 ms/600 ticks); unchanged HEAD measured 3,910,040 bytes/178.86 ms/26.11 ms. Buffered hashing reduced initial 195.61 ms load. No thresholds/expected values changed, no decomp changes; four new trace sets mirrored. Committed later as `4e08fb7`. Full matchup coverage is still open.

- 2026-09-11 (matchup coverage planning): inspected UI stepping/fault handling and selected combat/special stubs; documented a scoped interaction-coverage milestone in `docs/MATCHUP_COMPLETENESS.md`. No gameplay changes or commit. Baseline verification results are reported in the session response; next implementation packet is deterministic UI replay capture and reachable-gap inventory.

- 2026-09-10 (native rendering phases): committed native/shared presentation (`a4487fe`), custom texture combiners (`e78cd33`), particles/shields (`5f709a1`) and animated directional lighting/normals (`d58f669`). Focused tests, both complete-match allocation/nonmutation captures, particle replays, 13 Metal fixtures, clippy and native smoke pass. Full workspace reruns omitted per user scope. Next material fades require shared stage animation state; laser shield response remains an existing gameplay gap.

Newest first. One line per session: date, what landed, what is next.

- 2026-09-10 (rendering pass): implemented shared material/pixel state, camera, live stage presentation and laser instancing; expanded allocation/nonmutation checks through article spawn/reset, retained archives shared with fighters. Focused checks and native Metal smoke pass; no gameplay execution changes, data/decomp edits or commit. Next: custom TEV/material animation and held-weapon/effect poses.

- 2026-09-10 (native prototype): committed melee-lib as `e995c04`, then built shared Rust presentation/wgpu/session/C ABI with a thin Swift shell. Focused tests, clippy and native smoke pass; offscreen Metal frame inspected. App-only changes use scoped verification per user instruction. New app work remains uncommitted; no game data or decomp changes. Next: user playtesting and visual fidelity.

- 2026-09-09 (A3 rebase conflict resolution): preserved A3 grab/tech/KO and tornado/rectangle particles alongside main generic inverse trig, stages, character hooks and action IDs; scoped A3 motion resources to Fox/Marth. All 11 requested scenario gates exact (49 keys), M4 261/261, M5 8/8, particle/fighter/interface 166 tests pass; workspace all-target build and clippy clean, fmt applied. No git commands run. Next: user continues the rebase.

- 2026-09-09 (dash particle follow-up): added strict full-field replay, production spawn fixture, AppSRT ownership adapter and display-cache API; 425 display-cache mismatches remain because camera/frame inputs were not captured. Need those external display inputs to complete the red gate; no comparisons weakened or commits made.

- 2026-09-09 (M4-T1): ported Squat/Turn/Walk families, shared grounded physics/collision, walk command-loop and footstep/rumble requests; three movement gates 300x49 exact, state scratch/rate/command replay exact, all regressions and clippy pass. No commit requested. Next: review, then dash/run and jump/effect ports.

- 2026-09-09 (night): match-start scenario `start_fd_fox` recorded (tick trace, ledger, particles, tick-aligned + post-render bone dumps); Entry/Fall/Landing states, airborne physics, dynamic-bone solver (Fox tail), effect-id -> spawn mapping, FD animation spawn; bone oracle 73/73 bones; **start_fd_fox gate passed** 600x49 with produced RNG. Lesson: cached JObj matrices are render-time caches, compare them post-render. Next: M4 movement with scripted inputs.
- 2026-09-09 (later): T5 physics/collision, T10 Fighter composition, T13 melee-sim loop. **Milestone 3 gate passed**: 600 ticks x 49 keys, 0 divergences, seed included. Next: match-start scenario (spawn/Fall/landing), then M4 movement.
- 2026-09-09: T7 animation playback and T9 human input both match the FD trace for 600 ticks; hsd-particle matches all 600 dumped frames field-for-field; Docker/IDE memory pressure killed two concurrent Codex runs (resumed; rule: one at a time unless memory is free). Next: T5 grounded physics + collision for a standing fighter, T10 spawn/state machine, T13 melee-sim frame loop, then the M3 gate `idle_fd_fox_600`.
- 2026-09-08 (night): FD unlocked (save unlock mask poke), `idle_fd_fox` savestate + tick-boundary 600-tick trace (deterministic), Dolphin fork patched for PC/LR, RNG ledger attributes all 18,318 idle draws (particle system: 1 + 6 per emission per tick), M3 T4 Fox data readers, melee-gr FD first pass, hsd-particle first pass + live state dump. In flight: Codex replaying live particle state against the ledger. Next: fighter-side M3 tasks (T5 physics, T6/T7 animation playback, T9 input, T10 spawn) once particles match.
- 2026-09-08 (evening): Disc extracted (1,209 files), real .dat tests, retail asm lookup (`harness/asm.py`), fusion audit complete across gekko-math/melee-lb/hsd-anim/melee-mp (MWCC fused most sites; sinf ~18% of inputs differ), JObjDesc->JObjTree glue, coll_data reader + real FD collision test, lbanim FigaTree attach + ftData reader (Fox Wait1 plays), Dolphin oracle booted: `idle_ys_fox` savestate + 600-frame trace at 131 fps. Delegation switched to Codex (`tools/codex-task.sh`). M2 gate tooling built (Codex) and **M2 gate passed**: 0 mismatches on 3,080 bone words vs Dolphin. FD unlocked, tick-boundary sampling fixed, `idle_fd_fox` tick trace recorded (deterministic). Next: M3 tasks from `docs/M3_PLAN.md` on FD (fighter fields first, `rng.seed` after grlast.c).
- 2026-09-08 (late): Disc arrived and verified; main.dol placed for decomp tooling. No disc-dependent work run yet. Session paused by user. Next: the four items under "Milestone 0: disc arrived".

- 2026-09-08: Third batch: frsqrte/fres hardware-exact (M1 complete), JObj scene graph + DObj/MObj, typed descriptor readers + FigaTree, Snapshot/RecordSink/SchemaCoverage, melee-mp mplib+mpcoll. Gate: 55 suites green, clippy clean. Next: JObjDesc→JObjTree glue, melee-gr Final Destination, melee-ft skeleton (fighter.c init/update order), melee-sim frame loop, Dolphin savestate workflow doc.

- 2026-09-08: Second batch: hsd-gobj scheduler (35 tests), hsd-anim mtx/quat + SDK PS kernels and aobj/fobj (all native-C oracle clean), melee-lb lbtrigf + expf/powf, Dolphin scripting fork built on arm64, gekko_probe captured frsqrte/fres behaviour. Gate: 46 suites green. Next: hsd-anim jobj/dobj/mobj, hsd-archive typed node readers, Snapshot trait, melee-mp, user decision on estimate tables.

- 2026-09-08: Parallel batch: hsd-archive parser, type enums, MSL math with native oracle, harness walk + schema generator, slp parser, Dolphin research. Fixed melee-diff float equality. Next: hsd-gobj, hsd-anim start, lbtrigf into melee-lb, Dolphin build.
- 2026-09-08: Scaffolded workspace, gekko-math RNG/FMA, melee-diff, harness skeleton, docs. Decomp pinned as submodule.
- 2026-09-09: scripted-input tracer + per-tick pad replay in melee-sim; five movement scenarios and ledgers recorded; Codex M4-T1 launched (squat/turn/walk).
- 2026-09-09 (late): stock-take with the user; reoriented on the steel thread (Fox vs Marth, FD, full match) with a consolidation round first; design in `docs/STEEL_THREAD.md`; lanes launched: core (C1), perf (C6/C5/C10), harness (C9).
- 2026-09-09 (night): C9 merged; C1 and the perf lane (C5-sim/C6/C10) implemented and committed on their lane branches, unmerged: the game data loss (see Blockers) means no oracle can run. Both lanes and the laser recording wait for the disc image.
- 2026-09-09 (late night): disc image restored, 25 savestates and 162 scenes re-recorded, cold seeds refreshed, `~/melee-data` mirror; C12 (any-boundary import) and C13 (fixture command, constant audit) launched.
- 2026-09-09 (night): C12 merged (import from any scheduler boundary; 26 idle/start + 261 M4 gates on the new data); C1 merged on top of it; C13 (fixtures) and C14 (release exactness) running; C2 next on the core lane.
- 2026-09-09 (night, cont.): perf lane merged (C13, C5 sim-side, C6, C10, 23 regenerated fixtures); main M4 261/261, M5 8/8; remaining red: two Marth particle replays (ef draw classification), C14 and C2 in flight.
- 2026-09-09 (night, cont.): C14 merged (release exact in both profiles); C13 follow-up and Falcon test fix merged; hsd-particle 75/75, M4 261/261, M5 8/8 on main. C2 (concrete core, 49 pair-helper copies -> 1, -12% binary) committed on the core lane, rebasing.
- 2026-09-09 (night, cont.): C2/C7 merged; main fully green in both profiles (M4 261, M5 8, hsd-particle 75, melee-ft all). C11 running (harness lane), C4 launched (core lane).
- 2026-09-09 (night, cont.): C11 merged; JIT probes, fox_ys bones and start_fd_fox VI bones re-captured; C4 chain running; C5-b running; M2 legacy oracle -> aligned model (follow-up).
- 2026-09-09 (night, cont.): C4 merged (melee-ef). Awaiting C5-b follow-up and the M2 aligned-oracle follow-up; then a stock-take (user request), no new lanes.
- 2026-09-09 (late night): C5-b and the M2 aligned oracle merged. Consolidation round complete except C8 (`melee-cmd`/`melee-coll`) and C3 (family crate, lands with the first Fox special). Main: strict workspace 965 passed / 0 failed, both profiles exact. Paused for a stock-take at the user's request; no lanes running.
- 2026-09-10 (19:10Z): **Steel thread reached** (user decision). P2 merged: FD background joint matrices are computed only for live attachments and direct reads; both profiles 1,127 passed, M4 261, M5 89, alloc 33, census 19/7/100; P2 beat main in all four loaded A/B rounds (25.28-25.70 vs 26.06-28.05 ms); the perf-gate timing ceiling failed only under host load and the quiet re-benchmark was waived by the user. Open follow-ups: the user's offline Slippi replay; the screen-KO camera path; CI (plan discussed: committed pad schedules + per-tick digests, hosted tier without game data, self-hosted runner with the user's own ISO for scene gates).
- 2026-09-10 (18:20Z): S11-a merged: both human matches exact end to end (6,083 and 10,059 ticks). Chain green; perf-gate PASS (3,672,736 stripped, 25.3 ms). Interleaved A/B vs main, four rounds: ticks_600 23.51-23.82 -> 24.78-24.99 ms (+5%, ~2 us/tick), load unchanged; profile: all six FD background models now animate every tick (retail-required) and every joint's matrix is rebuilt and published each tick even without an attachment. Follow-up P2 scheduled (publish only attached joints; exactness on every scene; ticks_600 back to ~23.6 ms). Steel thread: all combat rows done; S11 waits on the user's Slippi replay.
- 2026-09-10 (15:05Z): combat lane merged: star-KO vanish, the human-play harness, and the match-start countdown fix (savestate imports never released the input freeze; retail releases at tick 85). Perf-gate flagged ticks_600 25.99 ms (CI 24.3..29.1) under load average 14-30; an interleaved A/B vs main on the same host puts the countdown's real cost at ~1% (23.4 -> 23.5-23.9 ms; load unchanged 167-170 ms), inside tolerance and retail-required. Recorded match2_fd_foxmarth live (10,059 ticks, seven KOs incl. three Marth deaths, 87/78 distinct states). S11-a (Codex, perf lane): match-start stock HUD ported, match_fd_foxmarth exact past the first stock loss, next TurnRun jump cancel at 352; FD background transitions queued. S7+S8 forward throw: retail's release collision subdivides the motion and probes the floor at y=-2.47 (Codex tracing).
- 2026-09-10 (midday): S6 merged (Codex). Star-KO vanish ported from a 340-frame re-recording (exact). Human-play pipeline for S11: the rebuilt Dolphin sees no keyboard because `loginwindow` holds Secure Keyboard Entry (lock/unlock did not release it), so keys come from a Ghostty terminal gamepad over the kitty keyboard protocol; the user's 600-tick smoke play is exact in the port and the acceptance match `match_fd_foxmarth` was played live. S7+S8 (Codex): 9 of 10 scenes exact incl. both 200% up-throws; forward throw one ULP off in Fox's Y at the release (ECB bottom through `mpLib_8004DD90_Floor`), being chased directly.
- 2026-09-10 (08:45): S9 merged (ported directly): damage sounds/voice draw, fly roll, side deaths, star KO flight, screen-KO entry; four high-percent scenes exact, M5 69, perf-gate PASS (3,620,544 stripped, duplicates at baseline). Codex credits restored by the user: `lane/battlefield` (S6) and `lane/chars` (S7+S8) rebased onto S9 (S3/S7 both ported effect 0x3FA: unified on S3's ModelSpawn) and resumed with ownership of their damage.rs paths (S6: shield-hit/powershield/break; S7: thrown airborne-hit and captured-victim). S10 report written (`docs/PORT_NOTES/S10_MATCH_FLOW.md`). Remaining: S6, S7+S8, S11.
- 2026-09-10 (07:40): S10 merged, ported directly (Codex credits exhausted): RebirthWait input exit, Ottotto rows, stop-at-edge ground adapter, victim-state whitelist removed; the four-stock scripted match is exact for 1,600 ticks with zero simulate-only allocations; perf-gate PASS (3,587,312 stripped). S9 in progress on the lane: heavy-hit sound/voice queue and DamageFlyRoll random draw done (200% scenes exact through the launch); side/top death entries next.
- 2026-09-10 (05:10): S3 merged (all specials exact; the lane's WIP commit turned out complete). Fixes on the way in: the new per-fighter wind/radial queues bounded to 8 (Fighter 44,768 -> ~40 KB; debug stack overflow in start_puff_bones_130), perf census ratchet via the reviewed baseline JSON. Remaining for the steel thread: S6 powershield branch + Fox shield-break rows (2 scenes), S7+S8 airborne-hit and captured-victim damage paths (4 scenes; ledge and mash scenes already exact), S9 KO variants (7 scenes), S10 match flow (scripted four-stock match), S11 acceptance. Codex blocked (credits).
- 2026-09-10 (04:00): Codex credits exhausted (see Blockers). WIP committed on all three lanes. S6 had been resumed with ownership of the powershield branch and Fox's shield-break rows; S3 part 3 was mid-way (0xB8 opcode, Reflector replays, Counter graphics timing); S7+S8 had M4 green and M5 finishing. Next: measure each lane's scene gates, merge what is fully green, and decide who finishes the rest.
- 2026-09-10 (cont. 11): PERF.md markers repaired (two merges had dropped a marker's closing tag, breaking perf-gate's evidence parse everywhere). Serial perf-gate on main `77c28e8`: duplicates 20/7/99 = C15 baseline, stripped 3,518,384 (ceiling 3,747,632); timing REGRESSION only (ticks_600 76 ms) caused by three concurrent Codex builds; re-measure when lanes idle before promoting. S3 part 2 committed and rebased onto S2+S5; part 3 resumed (opcode 0xB8 ruling: a port-limitation test may change; full scenes now reachable).
- 2026-09-10 (cont. 10): S5 merged (M5 55 + 1 ignored, workspace 1,056/0 both profiles). Launched S6 shield (perf lane) and S7+S8 grabs/ledge (harness lane); S3 part 2 continues on the core lane. perf-gate on the verify worktree fails with 'invalid or missing evidence' (corrupt evidence file after two concurrent runs); rerunning serially on main.
- 2026-09-10 (cont. 9): S2 merged (17 aerial scenes, leg IK, duplicate-definition census; M5 48, workspace 1,047/0). S5 rebased onto it (append-only conflicts in common_table/m5_gate/alloc_gate/assets resolved as main + S5's insertions; S2's aerial motions 68..78 folded into S5's `motion_indices`), verifying. perf-gate cannot run in two worktrees at once (shared evidence file: 'invalid or missing evidence'); run it serially on main after each merge.
- 2026-09-10 (cont. 8): S3 part 1 merged (Illusion ground/air rows, typed SpecialSide scratch, `it-foxillusion`; airillusion exact; grounded Illusion stops at the LegCorrection/IK boundary at tick 125). S3 part 2 (Fire Fox, Reflector, Shield Breaker, Dancing Blade, Dolphin Slash, Counter) running on the core lane (thread resumed). S5 done on the perf lane: 7/8 scenes exact, `tumbledi_dolphinslash` explicitly ignored until S3's Dolphin Slash; merging next. S2: 13/17 exact, blocked on the ground-pose leg IK (`fn_8008998C`, `lbBgFlash_80021410`) which no lane owned; S2's thread resumed with that ownership plus a perf-census fix (gate duplicate definitions, not total labels: five new concrete functions had tripped the +0 budget). New `../melee-lanes/verify` worktree runs merge chains on committed lane heads so chains never race Codex's edits.
- 2026-09-10 (cont. 7): C15 merged: the S1/S4 instantiation and size debt is repaid (copies 2,112 -> 128, stripped 3,483,104 < P1 baseline), perf-gate PASS. Launched S2 (harness lane), S3 (core lane), S5 (perf lane) with lane-ownership rules and append-only shared files. Next: merge in the order they finish (S5 first if possible: S3's Illusion hit and S2's landing hits need its damage levels), then S6, S7+S8, S9, S10, then S11.
- 2026-09-10 (cont. 6): S4 merged (item engine, laser, Fox family crate = C3). Rebase conflicts: both lanes had implemented the ftColl_80078538+0x94 extra hit spark (S1 `NormalSparkExtra`, S4 `HitSparkExtra`); unified on S1's request, item hits included, laser fixture regenerated. perf-gate REGRESSION (size +4.5%, copies up: S4 rows x 7) recorded; C15 launched alone on the core lane. Main: M5 28, workspace 1,013/0 both profiles.
- 2026-09-10 (cont. 5): S1 merged (19 ground-attack scenes, M5 27, workspace 1,004/0 both profiles). perf-gate REGRESSION on the copies metric only (+99 in melee-sim: S1 callbacks x 7 characters) and +1.3% size, within the size tolerance; recorded in `docs/PERF.md` as the C15 debt, baseline not promoted. S4 (items/laser) done on its lane, rebasing next; then C15 alone.
- 2026-09-10 (cont. 4): P1 merged (revival zero-alloc, Yoshi fix, baseline). C15 concrete shell scheduled after S1/S4. S9: a 200% Fox savestate (`idle_fd_marth4_fox200.sav`, fighter and player-block percent RAM-poked) for top blast-zone KOs; recording uthrow/utilt/Dolphin Slash/jab at 200%.
- 2026-09-10 (cont. 3): recorded the scripted four-stock match `match_fd_marth_scripted` (S10 gate; needs S1 fsmash, respawn drop, teeter, GAME). Stock-4 savestates `start_fd_marth4.sav`/`idle_fd_marth4.sav` created.
- 2026-09-10 (cont. 2): recorded 21 S5..S8 scenes (DI/SDI/CC/getups, shield variants incl. powershield and shield break, throws/pummel/mash, ledge options); mirrored. Launched P1 on the harness lane (revival without reconstruction: ko_fd_marth 690 allocs; ft-yoshi 228-copy anomaly + C15 concrete-shell design note; fresh perf baseline). S2/S3 prompts drafted; S3 waits for S4 (Illusion spawns item kind 56, the afterimage).
- 2026-09-10 (cont.): C8 merged (`b33c49b`, both profiles 977/0, alloc ceilings 17/1/12/690/17). Consolidation round done. Launched S4 (melee-it, it-foxlaser, ft-fox-family + Fox SpecialN; core lane) and S1 (19 ground-attack scenes; perf lane). Next: S2/S3 prompts when a lane frees up; merge S4 first (S1 touches m5_gate/alloc_gate too).
- 2026-09-10: C8 running; 47 combat scenes recorded for S1/S2/S3 (specials, ground attacks, aerials, L-cancels); scenario files committed, traces mirrored.

- 2026-09-10 (melee-lib extraction): reusable public library and oracle adapters verified in debug/release (1,144/0 each), three allocation tests and both examples pass; lifecycle and many-match memory measured. Timing and duplicate gates pass; binary size deferred by explicit user instruction. No commit or game-data/decomp changes. Next: melee-platform session/presentation/C ABI and Swift macOS window using wgpu.

- 2026-09-11 (rendering integration): completed and committed shared material/background animation, perspective/MSAA, held weapons, model effects, mip filtering, two Illusion afterimages, specialized particles and planar shadows. Final native and Metal checks pass; focused tests and allocation gates pass; workspace clippy clean. No game data or decomp modifications.

- 2026-09-11 (screenshot comparison): fixed transformed light paths, the camera-facing central glow and material lighting order. Verified same-frame Metal previews, debug/release capture checks, zero allocations, numeric shaders, workspace clippy and native smoke; committed without game-data or decomp changes.

- 2026-09-11 (Reflector input): seven new exact scenarios cover turn, release, priority, aerial button/tap jump cancel and turn landing; raw scratch and root-Y rotation checks pass. Both workspace profiles 1,223/0, clippy and 220 harness tests green; native smoke/exported replay passed at 232 ticks. Stripped simulator changed 3,926,672 -> 3,926,784 bytes (+112); no new full performance claim. Next: captured jump-squat up-special gap.

- 2026-09-11 (jump-squat up-special): six reproduced Fox/Marth panics replaced by grounded Up-slot dispatch, preserving up-special priority over grab/up-smash and diagonal Side input. Full debug/release each 1,230/0 with 3 existing ignores; clippy, formatting and 220 harness tests pass. Native smoke/exported replay passed at 232 ticks. Stripped simulator remains 3,926,784 bytes. Next priorities remain C-stick throws, recovery collision transitions and projectile reflection; the broader matchup milestone is not complete.

- 2026-09-11 (C-stick throws): fourteen new exact scenarios plus raw item/throw/hitlag checks pass; fixed Fox throw articles and re-enabled both old throw regressions. Debug/release each 1,249/0 with one existing ignore, clippy and 220 harness tests pass. Native smoke/headless replay:232 ticks; stripped simulator:+56 bytes. Recovery, common-input and revival oracle preparation continues; broader matchup completion remains in progress.

- 2026-09-11 (recovery verification): five 300-tick counterpart/rebound trajectories
  and both-fighter 150-tick local-SRT replays pass. Fixed animation-owned dynamic
  joint cache publication and retained command words across revival reset; all
  oracle expectations remain unchanged. Full debug/release each 1,257 passed,
  zero failed, one existing ignore; clippy, formatting, 220 harness tests and
  schema checks pass. Native smoke and exported replay pass at 234 ticks.
  Stripped simulator 3,943,568 bytes (+16,728); full performance census not rerun.
  Remaining recovery edges and common/defense/capture/ledge/timer gaps continue
  within Fox/Marth/FD; no breadth work started. Captures are ignored and backed up.

- 2026-09-12 (common inputs): 24 new exact scenarios, raw guard windows and
  local bone transforms; both workspace profiles 1,307/0 with one existing
  ignore, clippy, formatting, allocation and harness checks pass. Native build
  passes; graphics smoke and further Dolphin launches abort at the host IOSurface
  client limit. Stripped size +280 bytes. Continuing already-recorded revival,
  capture and ledge cases; no breadth work started.

- 2026-09-12 (required test-policy stop): common-input packet committed as
  e5db5fd. Uncommitted revival/capture and ledge implementations pass seven
  and ten fighter/particle/RNG scenarios respectively; capture raw and both
  airborne-grab 600-tick SRT checks pass. Revival SRT still needs its audited
  no-blend entry override; ledge root publication also remains to fix. The new
  raw ledge test incorrectly asserts the inactive wait word during CliffCatch:
  retail retains prior scratch 0x00000001 at Fox tick70, while the typed inactive
  placeholder is zero. CliffWait initializes the timer before use. No assertion
  or expected data changed. AGENTS.md requires stopping when a bit-exact test is
  believed wrong; awaiting authorization to correct that assertion's ownership
  boundary. No failing implementation committed. New graphics captures remain
  blocked by the host IOSurface limit; the Safari restart question is pending.

- 2026-09-12 (resume and repair): user explicitly replaced the test-policy stop
  with immediate source-backed diagnosis and repair. AGENTS.md and CLAUDE.md
  now require fixing implementation or test defects and rerunning gates without
  requesting confirmation. Corrected the raw ledge assertion to respect the
  CliffWait initialization boundary; captured expectations are unchanged. Added
  Fighter_procMap's final root publication and revival's explicit no-blend
  animation entry. Focused revival checks now pass 7/8; Marth revival retains
  a bone45 rotation delta under investigation. Ledge checks are running.

- 2026-09-12 (delta repaired): no-blend Rebirth entry, final map-root publication,
  retained dynamic locks/secondary pose and removal of creation-only respawn
  setup fix all new SRT deltas. Retained ledge-timeout provenance fixes the raw
  reset delta. Full debug gate: 1,345 passed, zero failed, one existing ignore;
  220 harness tests, schema and format checks pass. Release gate running.
  No captured expected values changed.

- 2026-09-12 (revival/capture/ledge verification): debug and release each
  1,345 passed, zero failed, one existing ignore; all-target clippy clean. All
  directed SRT/raw/allocation checks pass. Remaining uncaptured ledge/capture
  combinations stay open in MATCHUP_COMPLETENESS.md. Native build and size
  measurement running; graphics smoke remains host-limited. Next recorded
  packet: Fire Fox charge/travel contact, then projectile reflection.

- 2026-09-12 (verified packet boundary): native macOS build passes. Stripped
  simulator is 3,960,704 bytes (+16,856 from e5db5fd). Full performance census
  not rerun; graphics smoke cannot start while the host IOSurface limit persists.
  Committing the directed revival/capture/ledge packet and instruction repair.

- 2026-09-12 (Fire contact): charge and travel hits now match 300-tick fighter,
  RNG/ordered-particle, full SRT and particle simulation fields. Fixed authored
  fire overlays, hitlag dynamics/effect pause and locked-chain display caches.
  Debug/release each 1,352/0 with one existing ignore; clippy, formatting,
  220 harness tests, schema and native build pass. Stripped size 3,960,712
  (+8 bytes). Full perf census not rerun. Null-backend pilot matches an existing
  300-tick powershield trace; full capture equivalence next, then reflection.

- [x] (2026-09-12) Fresh/delayed/return/stale laser reflection: four 300-tick
  state/item/RNG/ordered-particle, raw contact/history, SRT and zero-allocation
  regressions pass. Both full profiles: 1,371 passed, 0 failed, 1 existing ignore;
  final 18 focused checks per profile after Clippy cleanup. All-target Clippy,
  225 harness tests, schema, formatting and native rebuild pass. Stripped
  simulator: 3,977,544 bytes (+16,832). Corrected child AppSRT ownership and the
  documented estimate-FMULS omission using 264 standalone guest cases and an
  independent C matrix reference. Headless scripting frontend validated against
  300 state/particle/RNG and 150 bone records, unblocking captures despite GUI
  IOSurface startup failures. Report: `docs/PORT_NOTES/LASER_REFLECTION.md`.
- [x] (2026-09-12) Directed clank and reflector overflow: six300-tick contacts
  and one600-tick shield-break recovery pass raw, SRT, particle and allocation
  checks. Implemented ReboundStop/Rebound, shield-break fall/down/stand, exact
  hit-phase effect dispatch and Fox knockback part/texture ownership. Both full
  profiles:1,404 passed,0 failed,1 existing ignore. Final shared-callback cleanup
  passes56 affected tests per profile and14 release throw checks after no-inline;
  Clippy,225 harness tests,schema,formatting,native build pass. Perf census restored
  cross-crate duplicates104→100; stripped3,978,032 (+488 versus f0dd3b8), load183.259ms,
  ticks26.137ms. Existing size/time ceilings remain red and unchanged. Report:
  `docs/PORT_NOTES/CONTACT_CLOSURE.md`; full measurements in `docs/PERF.md`.
- [x] (2026-09-12) Wall/ceiling recovery: four450-tick recordings cover Fox
  WallJump, Fox/Marth StopCeil and a Marth wall-contact control. Corrected common
  nonzero motion-entry blending, revival collision reset and tornado import.
  Both full profiles:1,418 passed,0 failed,1 existing ignore. Final callback
  sharing passes14 affected checks per profile; Clippy,225 harness tests,
  formatting and native build pass. Census20 local/100 cross-crate duplicates;
  stripped3,995,440 bytes. Existing size/tick-time ceilings remain red. Report:
  `docs/PORT_NOTES/WALL_CEILING.md`.
- [x] (2026-09-12) CaptureJump and airborne release: six450-tick recordings
  cover both captors, held-Up/XY jump release, prolonged airborne hold, air Cut
  and air Jump. Fixed capture x4 inheritance through landing and stop-at-edge
  grab startup.20 directed checks pass; both full profiles1,438/0/1; Clippy,
  225 harness tests,formatting,native build pass. Stripped3,995,872 (+432),
  census20 local/100 cross-crate; existing size/time ceilings remain red. Report:
  `docs/PORT_NOTES/CAPTURE_RELEASE.md`.
- [x] (2026-09-12) Revival-laser contact: a 600-tick retail recording verifies
  contact response without damage, hitlag or attacker staling during revival
  invincibility. Five directed checks and both full profiles pass: 1,443 tests,
  zero failures, one existing ignore. All-target Clippy, formatting, 225 harness
  tests and native macOS build pass. Stripped size remains 3,995,872 bytes;
  census remains 20 local/100 cross-crate; load 185.969 ms, ticks 26.359 ms.
  Existing size/time ceilings remain red and unchanged. Report:
  `docs/PORT_NOTES/REVIVAL_LASER.md`.
- [ ] Paused at the user's requested revival-laser commit boundary. Next:
  DamageFlyRoll (three recordings captured; source draft unapplied), remaining
  contact/Reflector gaps, camera, timer/scene flow and Sudden Death Bob-ombs.
  The fixed 48-case robustness corpus now has 20 match finishes, 17 full
  6,000-tick runs and 11 faults; it is not an exactness oracle. Natural timed
  fixtures and clock/scene diagnostics are captured. No breadth work started.
- 2026-09-26: Headless recording is the standard. Rebuilt the lost no-GUI
  scripting build (patch `docs/patches/0003-nogui-scripting-backend.patch`,
  `tools/build-headless-dolphin.sh`); the earlier build's silent savestate
  failure was a missing `Sys` directory outside an app bundle. Harness launch
  selection lives in `harness/dolphin_config.py`; host audio muted by default.
  Two scenes re-recorded byte-identical; 227 harness tests pass. Next: resume
  DamageFlyRoll when the matchup work restarts.
- 2026-09-26: DamageFlyRoll (91) for every fighter: common row (animation
  181), roll rotation on entry and twice per physics tick (retail 0x800903C4 /
  0x8009045C straddle ftColl_8007AFF8), DamageFall at hitstun expiry. Three
  retail witnesses (t125 blast-zone KO, dtilt_t132 hitstun expiry, crouch floor
  bounce) pass fighter/RNG/particle, 450 bone ticks incl. XRotN, raw scratch
  and zero allocations; debug/release 1,458 passed. DamageFly wall/ceiling
  tech chain remains unported. Notes: `docs/PORT_NOTES/DAMAGE_FLY_ROLL.md`.
- 2026-09-26: Corpus-to-retail bridge. Four-stock Fox/Marth FD boundaries
  (`start_fd_fox4` recorded headless, `start_fd_marth4`) match cold
  construction exactly; `harness/boundaries.toml` registers them.
  `replay_to_scenario.py` turns a melee-replay recording into a tick-clock
  scenario: TickTracer rewrites HSD's raw pad queue at every tick end so each
  tick consumes exactly the recorded pad (VI scheduling drifted a tick);
  `calibrate_pads.py` measures Dolphin floats for every stick/trigger value.
  Corpus v2 (explore.rs) starts from those boundaries with a neutral first
  tick; 48 cases: 9 faults (5 fighter phantom, 2 item phantom, 1 captured
  damage, 1 Reflector ground-to-air). Bridged phantom case: pads exact for 894
  ticks, port matches retail through the fault. Next: phantom hits.
- 2026-09-26: Generated matches as retail oracles. Seven corpus v2 cases
  bridged to Dolphin at full length now match retail start to GAME (25,115
  ticks, items and ordered particle sites; `m5_gate::corpus_v2_matches_through_game`,
  two zero-allocation). Fixes, all shared: per-victim damage logs and phantom
  hits (melee-coll `damage_log`, `hit_log.rs`), hitlag callback ownership
  (SDI after specials), charged-hitbox staling, unified color overlays
  (priorities, invincibility flash, charge sparkle suppression), grounded
  mid-animation root-motion velocity, ledge/getup/landing stale ids, TurnRun
  edge flags, one-jump ground departures for Marth specials, airborne
  knockback decay tail, down-tilt squat hold, shield SDI/ASDI. Debug and
  release gates 1,465 passed, 0 failed; clippy, fmt, 243 harness tests pass.
  Notes: `docs/PORT_NOTES/CORPUS_BRIDGE_FIXES.md`. Next: remaining corpus
  faults (captured damage, Reflector ground-to-air), then bridge all 48 cases.
- 2026-09-26: Two more corpus faults matched in retail. The "captured
  damage" fault came from an earlier divergence: ftCo_Damage_CalcKnockback's
  full modifier chain (smash charge PlCo +7C4, DamageIce +718, Y scale,
  armor, +104 floor) replaces the crouch-only multiply. The Reflector gets
  ground-to-air for all five rows, shared ftCommon_8007CF58 (over-drift air
  friction PlCo +1FC, also Fire Fox rebound), and a
  `CharacterCallbacks::retained_scratch_word` hook: Reflector Start never
  writes turnFrames (+2344), so it carries the previous state's mv word into
  JumpAerial/Landing. `corpus_v2_s1_edeadbeef_p0` (665) and
  `corpus_v2_s0_e12345678_p2` (1342) match; the port-side corpus now faults
  only on the screen-KO camera. Testing cadence: focused checks per change,
  full gate at work boundaries (CLAUDE.md). Next: bridge all 48 cases to
  retail, then the gameplay camera (`cm/camera.c`) for screen KOs.
- 2026-09-26: All 48 corpus v2 cases bridged to retail (batch: bridge,
  record, verify, gate). 26 now pass fighter keys, items and ordered particle
  draws (`corpus_v2_matches_through_game`, was 9). Shared fixes: Guard states
  slide into MissFoot (ft_800845B4); grab release only from CaptureWait's own
  callback; DownDamage for light hits on prone fighters (ftCo_8009F0F0);
  boost grab window in the dash attack; sealed effects dispatch before a new
  script's graphics; item hitlag (xCBC/it_8026B424, link-0 countdown) with the
  Illusion's DmgDealt override; ftCommon_8007DB58 on capture and a character
  death hook (Blaster removal); the Blaster accessory disarms outside its loop.
  New diagnostics: `melee-sim bones-diff`, `particle-sites`, `particles-diff`,
  `record.py --bones-from` and `--camera` (camera oracle snippet). Remaining
  groups in `docs/PORT_NOTES/CORPUS_BRIDGE_FIXES.md`: gameplay camera
  (magnifier damage and screen KO, 7 cases), Fox tail dynamics in
  GuardSetOff (4), particle list ordering (4), small offsets/RNG (7).
  Particle order then matched after three fixes: replays re-sort particle
  lists only on retail display passes (the tracer now records psFrameNum),
  generator deletions park the insertion cursor at the tail, and landing dust
  resolves with script graphics in script order. 30 corpus matches gated.
  Then ft_80084DB0's fast-fall check for the air Blaster and platform drops,
  and no extra color step after a shield hit: 34 corpus matches gated.
  Then: forward-smash IASA without spot dodge, first-capsule per-bone hurt
  states, joint caches refreshed only on display passes, Jump's first-frame
  skip limited to Jump rows. 37 corpus matches gated; 8 of the remaining 11
  need the gameplay camera. Next: the camera port (`melee-cm`).
- 2026-09-26: Gameplay camera port, new crate `melee-cm` (cm/camera.c): CmSubject
  framing and extent smoothing, standard mode (Camera_8002B3D4 and callees,
  every fmadds cited), quakes with the stage's `quake_model_set` animations,
  the rendered CObj (Camera_8002AF68) and world-to-screen (C_MTXLookAt,
  MTXPerspective, GXProject, lbVector_WorldToScreen in `hsd_anim::cobj` and
  `melee_lb::vector`). The stage camera comes from grGroundParam and markers;
  its +8 "tilt" is the field of view (Camera_80030730). The savestate import
  reads game_camera, the subject list and ifMagnify from saved MEM1. Fighters
  own their subject with retail's dead/rebirth camera callbacks
  (ftCamera_80076064/80076320, Rebirth_Cam) and the magnifier damage; protection
  timers now tick during hitlag as in Fighter_8006A360. `melee-sim camera-diff`
  checks a camera dump in isolation (bit-exact outside quakes and revivals).
  Item hitboxes now clank with fighter hitboxes (ftColl_80077970) and hit
  Marth's Counter as a shield volume (ftColl_80077688, xCC0 item hitlag),
  which Fox's Illusion ghost needs. 44 corpus matches gated (was 37). Next:
  the screen-KO approach, the GuardOn shield case (s0_e2a_p2), then the
  remaining singles.
- 2026-09-26: Three more corpus singles fixed (47/48 gated): air decay of the
  attacker's shield recoil (Fighter_procUpdate 8006BA5C, including the
  kb_vel.y store bug), DownDamage's facing argument after the knockback
  (ftCo_8008DCE0 block_42) without ftCommon_8007DB58, and the Illusion trail
  accessory (ftFx_SpecialS_CreateGFX) only in the dash states; CatchWait's
  flash queues behind the proc's color-program effects. `particles-diff`
  lists both generator orders with MELEE_PARTICLE_LISTS=1. Remaining:
  s0_e2a_p2 (Fox's GuardOn shield vs Dancing Blade, tail/cape bones) and the
  screen-KO approach.
- 2026-09-26: Shield Breaker's radial gusts (ftMs_SpecialNLoop/End_Anim ->
  lb_800119DC) ported; s0_e2a_p2 passes, so all 48 corpus v2 cases gate
  (the committed s0_e80000000_p1 stops before its screen KO). Next: the
  screen-KO approach (DeadUpFall camera-space flight), then timer/tie/sudden
  death and the coverage audit.
- 2026-09-26: Screen KO ported (ftCo_DeadUpFall_Anim 800D4A08, _Phys 800D4CE8,
  ftCo_800D481C, fn_800D4DD4, ftDrawCommon_80080E18_inline2): camera-space
  approach/impact/fall, placed each display pass through the inverse view of
  cm_804D6464 (transform_copy). ftCommon_8007EBAC is rumble, not a colour
  program. The HUD percent explodes on the screen KO's vanish too
  (`LifeState::stock_lost`). s0_e80000000_p1 now gates through its screen KO
  (5208 ticks). Next: Sudden Death (recorded: Bob-omb rain from
  Ground_801C0C2C after frame 1200 -> it_8026BE84/itbombhei.c), then the
  one-minute timeout/tie flow and the coverage audit.
- 2026-09-26: Sudden Death ported and gated (`sudden_death_bombs_fd_marth`
  1300 ticks, `sudden_death_idle_fd_marth` 1576 ticks through the Bob-omb KO
  and GAME; particles match too). HUD banners (countdown, GO) and the match
  clock (lbl_8046B6A0), the rain (Ground_801C0C2C/801C0A70, Stage_80224FDC),
  the Bob-omb (`it-bombhei`) and shared item engine pieces: common Articles,
  script subroutines, map collision, spin, lifetime, hitbox radius at 1/scl.
  `docs/PORT_NOTES/SUDDEN_DEATH.md`. Next: the timeout (timer, TIME!,
  standings) and the live transition into Sudden Death.
- 2026-09-26: Timeout and the live Sudden Death transition. Timer rule and
  countdown (fn_8016CD98, gm_GetMatchOutcome), stock standings, the frozen
  hold to the scene exit (fn_8016D634), `Match::sudden_death` (exit seed plus
  the setup draws), cold Sudden Death setup, GO stepping in its creation
  pass, FD's start-created procs registered at stage start. New witnesses:
  `timeout_tie_fd_marth` (3839), `sudden_death_start_fd_marth` (1697), both
  also rebuilt cold; `melee-lib/tests/match_endings.rs` runs the whole path
  through the public API bit-exact. Next: the final coverage audit.
- 2026-09-26: Corpus v3 (`explore <dir> 40`, 240 matches from xorshift seeds
  of 0x00C0FFEE): 9 port faults, all fixed and gated
  (`m5_gate::corpus_v3_matches_retail`). A motion change ends the attack
  interaction; grabs and throws use the stop-at-edge map pass (ft_800841B8 ->
  ft_800827A0), not the fall-off one; a dying grabber releases its victim
  (ftCo_800D331C/ftCo_800DD100); thrown positioning (accessory1) waits out
  hitlag (Fighter_CallAcessoryCallbacks_8006C624); Marth's Counter volume dies
  with the motion (fighter.c:1049 `x221B_b0`). `sudden_death_jab_bomb_fd_marth`
  gated; `sudden_death_pickup_bomb_fd_marth` recorded as the retail witness
  for the item pickup packet. Next: Sudden Death item interactions
  (COVERAGE_AUDIT.md silent gaps).
