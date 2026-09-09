# Dream Land N64: idle, match start and cold start

Lane B6, 2026-09-09. `idle_dl_fox`, `start_dl_fox`, and
`start_dl_fox_cold` each reproduce **600 ticks × 49 keys, zero divergences**.
The two independent particle replays also match every captured simulation
field and ordered particle RNG site, with no exclusions.

## Resources and collision

`grOp_StageData` in `groldpupupu.c` selects `Gr_Kind_OldPupupu` (28),
`/GrOp.dat`, and `grOldPupupu_802107E0`. `grdatfiles` resolves that archive's
publics. The tracker previously named `grpura.c`; that is not Dream Land N64.
The scene descriptor registers `DreamLand`, `GrOp.dat`, and music row 28.
The music row uses rule 0, primary track 58, so neither recording consumes a
music draw even though all characters and stages were unlocked.

The archive has eight model descriptors and map-head section counts
`[1,8,0,38,10,8]`. Map scale is exactly 1. All model collision-binding lists
are empty; collision stays in `melee-mp` and uses its existing static and
one-way floor resolver. Floor queries give:

| Location | Floor line | Floor y | One-way |
|---|---:|---:|---|
| Main platform, x=0 | 4 | 0.0088 | No |
| Left platform, x=-46.6 | 0 | 30.1421 | Yes |
| Right platform, x=46.6 | 1 | 30.2425 | Yes |

The supplied positions were rounded. At Wait, the collision separation
places P1 at y=0.0088999998 and P2 at y=30.1421985626. P1's entry marker is
(0,7.0953040123); P2's is (-46.5999984741,37.2215003967). The cold path reads
these markers from map 0, without stage-specific fighter spawn constants.
P1's action transitions agree with the notes: 322 at tick 0, 323 at 6,
324 at 35, Fall 29 at 65, Landing 42 at 74, Wait 14 at 104. P2 reaches
those later states at 11,40,70,79,109.

Map 5 selects an ambient light and a point light, both white. The point
position is (-0.0,5,17); its brightness/distance/function descriptor is
loaded through `hsd-archive`. Both light lists are static. The existing
shared light loader now represents point attenuation as owned typed data;
no archive layout enters the scene or controller. GX shading is outside
this headless simulator.

## Controller, animations and scheduler

`melee-gr/src/pupupu/` owns the Waiting/Turning/Blowing controller, its
blink and wind timers, the background timer, wind rectangles, and animation
selection. `melee-sim/src/scene_stage/pupupu.rs` supplies current animation
completion, fighter bone queries, animation attachment and particle allocation.
Shared fighter code receives an ordinary wind vector through `proc_update`.

`grOldPupupu_802107E0` creates maps **0,3,7,5,4,6,1,8**. Map 8 is a
controller without a model descriptor. Ground animations run at s_link 1;
Ground wrappers and stage callbacks run at s_link 4 in that creation order,
before fighter physics. Lights run at s_link 0; common particle main/aux
passes run at s_link 15. The map-7 Whispy callback therefore precedes the
cloud callback (map 6) and secondary tree callback (map 1).

`grAnime_801C8138` replaces the selected animation and evaluates frame zero
immediately. The animation helper now supports that replacement without
replacing the model, and exposes the first joint AObj's completion flag
(`grAnime_801C83D0`). Saved frames rebuild the archive animation cursors;
historical particle events are discarded. The idle boundary has Whispy's
blink at frame 35, completed; map 4 at frame 197 of its 216-frame loop;
and the secondary tree at frame zero. Map 6 has no animation until the
blow transition requests it. Cold setup evaluates maps 0,3,7,4,1 during
creation; it starts with empty generator/particle lists and family counter 256.

The archive supplies wind delay `[600,1200)`, blink delay `[180,360)`,
background flyby delay `[3000,4000)`, and wind speed 0.2. The range helper
also preserves reversed endpoints and consumes no draw for equal endpoints.
Whispy decrements the wait timer every callback; the blink timer decrements
only after the current animation has ended. It resets the wait timer again
on entering Waiting, even though initialization already chose one.

On Turning entry, `ftLib_800864A8` votes using each enabled fighter's
camera-target bone and camera offset (`ftLib_800866DC`), not its floor
position. Negative X votes left; zero votes right. A tied vote draws
`HSD_Randi(2)`. No tie occurs in these recordings. Turning/blowing animations
and the secondary tree/cloud requests retain the retail callback ordering.

During Blowing, wind is enabled strictly after counter 45 and before 320.
`fn_802112F4` applies it only strictly inside the selected world rectangle:
left x=(-74,-18), right x=(-17,76), y=(-10,40). In the idle recording it
first moves P2 at tick **519**, by -0.2 per update. P2 leaves Wait at
**593**, x=-61.6000556946, and falls off the left platform. P1 remains at
x=0. No wind interval occurs during the 600 match-start ticks.

## Saved scheduler boundary

The idle savestate is inside P1 `Fighter_procUpdate` (0x8006B82C), at
**PC 0x8006BF28**, s_link 4. Its physics, wind integration, and crowd check
have already run; the remaining Wait-path work invalidates collision caches.
The adapter checks the exact PC, r31 fighter address, callback and owner.
On the first resumed tick it skips earlier phases, all stage callbacks at
link 4, and P1's completed physics; P2 physics and later phases execute.
The imported collision caches are already invalid. In particular, Whispy's
timers must not advance again on tick zero. The saved controller has
elapsed=629, wait timer=439, blink timer=119, facing right, no active wind,
and background timer=3230. Initial particles and the saved gust list are empty.

The start boundary is between scheduler passes (current proc null, saved
link 24). Observation zero finishes setup without executing a scheduler pass;
tick 1 is the first full pass. Whispy starts with timer 630 and its entering
flag set; the background timer is 3859. No later trace row, ledger draw,
or expected particle state is a runtime input.

## RNG audit

| Consumer | Idle draws | Start draws |
|---|---:|---:|
| Whispy controller | 2 | 3 |
| Fighter Wait choice | 10 | 11 |
| Landing-effect placement | 0 | 6 |
| Particle system | 240 | 4481 |
| **Total** | **252** | **4501** |

Whispy's complete captured direct draw sequence is:

| Scene / tick | Callsite | Meaning |
|---|---|---|
| idle 119,399 | `grOldPupupu_802113E0+0x264`, 0x80211644 | Blink-delay reset |
| start 1 | `grOldPupupu_802113E0+0x98`, 0x80211478 | Waiting-entry wind delay |
| start 1 | `grOldPupupu_802113E0+0x170`, 0x80211550 | Initial blink delay |
| start 386 | `grOldPupupu_802113E0+0x264`, 0x80211644 | Blink-delay reset |

The idle particle sites are `hsd_8039930C+0x21F0` (52),
`hsd_8039EE24+0xDC` (47), `hsd_8039DAD4+0x710/+0x900` (39 each),
`hsd_8039930C+0x1184/+0x11C8/+0x1210` (19 each),
`hsd_80398F8C+0x188` (5), and `hsd_8039F05C+0x1F4` (1).
The stage requests bank-30 kinds 30000,30001,30002 through DPtcl animation
commands. Match start requests the existing bank-0 entry/landing particles;
its complete particle sequence is compared in `m4_gate` and `live_dl_start`.

Opcode **A9**, `hsd_80398F8C` (0x80398F8C), randomizes velocity azimuth
on a fixed-aperture cone while preserving its magnitude. The new HSD helper
uses caller-provided inverse trig, preserving workspace layering. It keeps
the fused projection at 0x80399048, squared-length fmadds at B8/BC, three
double-precision sqrt Newton steps, double PI × random × 2 with one rounding,
and final fmadds/fmsubs at 0x80399178/80/8C/90/94. Its draw is at 0x80399114.
This is first reached at idle tick 560. The existing A8 position offsets and
other required opcodes needed no arithmetic changes. The new opcode is
included in the capability inventory and truncated-program checks.

Assembly was read for `grOldPupupu_802113E0`, `8021119C`, `80210C7C`,
`80210D10`, `fn_802112F4`, `ftLib_800864A8`, `Fighter_procUpdate`, and
`hsd_80398F8C`. The captured controller branches have integer arithmetic;
light position scaling reuses the audited Ground fmuls path. Existing HSD
animation/matrix routines and Melee collision retain their audited math.

## Cold construction

The new scenario uses sidecar boundary seed **3759750154 / 0xE0193C0A**.
The fixed setup interval is six draws: Whispy's initial wind delay
(`grOldPupupu_8021119C`, 0x80211200), background delay
(`grOldPupupu_80210C7C`, 0x80210CB4), then two existing CPU initialization
draws per human fighter. Reversing six LCG steps yields **0x7F00788C**;
executing setup forward reaches the boundary seed and imported timers.
The cold/imported comparison matches **88 initialized scene fields**, the
initial particle snapshot, and attachment metadata. The DAT-only scratch-root
test includes Dream Land and runs all 600 ticks without saved files or traces.

As in COLD_START.md, inactive heap words are not parameter-derived state.
Whispy's wind word (+0xDC) is not initialized until the first callback; the
saved start value is 0x09FFFFFF. The cold owner starts calm, and the first
callback writes calm before any physics. This inactive word is not part of
the canonical setup comparison. The seed is the post-creation/pre-music
boundary, not an independently measured earlier stage-select seed.

## Verification and scope

| Particle replay | Ticks | Compared fields | Ordered particle draws | Exclusions |
|---|---:|---:|---:|---:|
| `live_dl_idle` | 600 | 53715 | 240 | 0 |
| `live_dl_start` | 600 | 178032 | 4481 | 0 |

Fixtures contain production external spawn/attachment inputs only, never
retail particle outputs or child-spawn results. See the particle fixture
README for regeneration. No expected values, tolerances or existing scenario
files were changed.

The tested scope is these three 600-tick scenes. Camera-relative background
flyby creation at the expiration of the 3000–4000-frame timer remains an
explicit unsupported branch; neither ledger reaches it. The controller
maintains that timer but this is not a complete multi-minute flyby simulation.
Whispy's auxiliary gusts for dynamic-bone/camera effects, GX material/texture
rendering and camera quake output are not implemented by this stage port.
The 49-key fighter gates and full particle replays do not prove those display
or dynamic-bone outputs. Restoration rejects active-wind saves, nonempty
initial gust/particle populations, and a demo freeze rather than guessing.

No Dolphin run, commit, or write to traces, ROMs, existing scenarios, or the
decomp submodule was made. The lane's pre-existing ROM/trace symlinks and
decomp typechange remain unchanged.

### Final checks

- Three requested CLI gates: each 600 ticks, 49 keys, zero divergences.
- `cargo gate`: **846 passed, zero failed, three pre-existing ignored**
  (baseline 834 passed, same three ignored). `m4_gate`: 189 passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo fmt --all -- --check` and `git diff --check`: passed.
- Harness pytest: **186 passed**, using the main checkout's existing venv
  with bytecode/cache writes disabled. The lane venv lacks pytest.

The ignored tests are the two existing full-grab tests and the existing
schema doctest; this task adds no ignored test and changes no skip policy.

### Changed files

```text
TRACKER.md
crates/hsd-archive/src/desc/light.rs
crates/hsd-particle/src/direction.rs
crates/hsd-particle/src/lib.rs
crates/hsd-particle/src/particle.rs
crates/hsd-particle/src/system.rs
crates/hsd-particle/tests/data/README.md
crates/hsd-particle/tests/data/idle_dl_spawns.json
crates/hsd-particle/tests/data/start_dl_spawns.json
crates/hsd-particle/tests/lifecycle.rs
crates/hsd-particle/tests/live_dl_idle.rs
crates/hsd-particle/tests/live_dl_start.rs
crates/hsd-particle/tests/opcodes.rs
crates/hsd-particle/tests/start_paths.rs
crates/hsd-particle/tests/support/dust_replay.rs
crates/melee-gr/src/battle/lights.rs
crates/melee-gr/src/desc.rs
crates/melee-gr/src/last/animation.rs
crates/melee-gr/src/lib.rs
crates/melee-gr/src/pupupu/mod.rs
crates/melee-gr/src/pupupu/procs.rs
crates/melee-gr/tests/real_pupupu.rs
crates/melee-sim/src/frame.rs
crates/melee-sim/src/initial_state/cold.rs
crates/melee-sim/src/initial_state/cold_tests.rs
crates/melee-sim/src/initial_state/mod.rs
crates/melee-sim/src/initial_state/particle_resume.rs
crates/melee-sim/src/initial_state/stage.rs
crates/melee-sim/src/scenario.rs
crates/melee-sim/src/scene_stage.rs
crates/melee-sim/src/scene_stage/pupupu.rs
crates/melee-sim/tests/m4_gate.rs
docs/COLD_START.md
docs/DREAM_LAND.md
docs/PARTICLES.md
harness/scenarios/start_dl_fox_cold.toml
```
