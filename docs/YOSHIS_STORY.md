# Yoshi's Story: idle, Lane B2

2026-09-09. `idle_ys_fox` reproduces **600 ticks × 49 keys, zero
divergences**, including RNG. Its independent particle replay compares
**51,303 fields and 476 ordered particle RNG draws**, with zero mismatches
and **no excluded fields**. Both Foxes remain on the side platforms at
x=±42, y=23.45. No recording, scenario, ROM, or decomp data was changed;
Dolphin was not run, and no commit was made.

## Resources, animation, collision

`melee-gr::story` owns the stage timers, spawn decisions and scheduler table.
`melee-sim::scene_stage` registers `YoshisStory` with `GrSt.dat`. Archive
layouts remain in descriptor readers; HSD spline arithmetic lives in
`hsd-anim`; fighter collision remains stage-independent.

The archive has four map models, map scale 0.7, 34 collision vertices,
29 lines and two collision joints. Side-platform floors 1 and 5 use the
existing one-way floor resolver. Map 2 binds collision joint 0 to archive
descendant 1 (Randall). Ground's wrapper is excluded when counting bones.
`BackgroundAnimation::update_collision` supplies each bound joint's current
matrix and hidden flag to `CollMap::update_joint_transform`; the map preserves
previous/current vertices. Initial transforms and previous-position snapshots
are installed before the first simulated tick.

Randall's path is **animation-driven**, with its phase restored from the
savestate, rather than inferred from the trace tick number. Its spline at
archive offset 0x18518 has type 0 and 13 control points. The HSD port resolves
the animated joint's AObj reference to the spline joint, evaluates the linear
arc-length parameter, and applies the spline reference's world transform to
the translation column (`HSD_JObjMakeMatrix`, jobj.c:187-195). This last step
matters: the saved animated local position is not Randall's world position.
The spline math preserves both divisions, the multiply back into segment
space, and the fused interpolation at retail 0x80378B58/6C/80.

Background map 1 uses animation 0. Map 3 uses animation 0 plus animation 1
on descendant 5, matching `grStory_801E3234` and `grAnime_801C7FF8`. Saved
frames are 108 for maps 1 and 2, and 107 for map 3. The existing interpreter
rebuilds keyframe cursors through those frames; historical animation events
are discarded. Static lights come from map 3 through the shared light
loader. Material/texture display remains outside the headless gate.

## Saved boundary and scheduling

The LZ4 savestate already contains the necessary stage state; no new capture
was needed. The particle metadata alone has no attached stage joints, because
its initial common-bank puff is detached. The boundary adapter reads Ground
and JObj state directly from saved MEM1, never from a later trace row.

- Saved RNG seed: 249636915.
- Scheduler: between ticks (`current_proc = 0`, saved link 24), so tick zero
  executes a complete scheduler pass.
- Map creation order: **0, 1, 3, 2**.
- Randall's signed 16-bit puff timer: **15**.
- Shy Guy timer: **12**, previous pattern 0, count 0; the saved item list is
  empty and the adapter checks this boundary.
- Existing detached common-bank particle 44 and its generator are restored
  from the particle capture. The scene now selects the correct bank for
  each initial generator and accepts a null JObj attachment.

Ground animations run at s_link 1. The map 3 stage callback runs before map 2
at s_link 4; collision transforms update before fighter collision. Common
particle emission and interpretation run at s_link 15. The old scenario omits
a cold-start seed, so `Scenario.seed` now defaults when absent; restored RNG
still comes exclusively from the saved boundary and is cross-checked against
both metadata and the savestate sidecar.

## All RNG draws over 600 ticks

The ledger has **521 draws**: 33 stage, 12 fighter Wait choices, and 476
particle draws. The gate produces the same seed after every tick, and
`m4_gate::story_idle_particle_rng_order` compares particle sites in order.

| Callsite | Draws | Meaning |
|---|---:|---|
| `grStory_801E3418+0x74` | 1 | Random delay, subsequently overwritten by 120 |
| `grStory_801E3418+0xC4` | 1 | Pattern choice, retrying the previous pattern |
| `grStory_801E3418+0x11C` | 1 | Speed variant |
| `grStory_801E3418+0x148` | 1 | First group-count choice |
| `grStory_801E3418+0x160` | 1 | First multi-count, subsequently overwritten |
| `grStory_801E3418+0x17C` | 1 | Second group-count choice |
| `grStory_801E3418+0x1F8` | 1 | Vertical jitter, including after the last spawn |
| `grStory_801E366C+0x44` | 26 | Next puff delay |
| `ftCo_8008A7A8+0x114` | 12 | Wait animation choice |
| `hsd_8039EE24+0xDC` | 398 | Live generator update |
| `hsd_8039DAD4+0x710` | 26 | Puff emission draw |
| `hsd_8039DAD4+0x900` | 26 | Puff emission draw |
| `hsd_8039930C+0x35C8` | 26 | Puff rotation |

The one-off Shy Guy sequence occurs at **tick 12**, not tick zero. It selects
one Shy Guy. Both overwritten choices still consume their draws. The jitter
uses fsubs, fmuls, then `fmadds` at 0x801E362C. The puff timer tests the old
signed value before decrementing, so the first new puff is at tick 16.

`grLib_801C97DC` queues common-bank particle 44 against Randall's joint.
`hsd_8039EE24` bakes and releases queued attachments before iterating
generators. The new `hsd_8039D71C` path snapshots position, normalizes matrix
columns, transforms velocity (and line endpoints when applicable), and clears
the attachment identity while retaining the generator's flags. The existing
particle interpreter already supported the required puff opcodes.

## Verification scope

The 600-tick gate proves fighter state and shared RNG; the particle replay
also proves puff positions, lifetimes, generator flags, and all captured
particle fields. The replay's spawn fixture was logged from production calls,
not reconstructed from retail particle outputs. See
`crates/hsd-particle/tests/data/README.md` for regeneration.

The stage's Shy Guy schedule and spawn requests are ported. The idle scene
remains in the no-hit occupied interval before Heiho's 960-frame upward
escape, so the stage suppresses subsequent groups while retaining its spawn
requests. This is **not a complete Heiho item simulation**: motion, food
ownership, hits, and escape/despawn need `melee-it` work. Initial live items
are rejected, and the occupied interval beyond 960 frames is rejected. No
claim is made for combat with Shy Guys or a complete long-running stage.
Cubic splines, external AObj references and detached AppSRT generators also
remain explicit unsupported branches. Static lights and collision are
covered; rendering, animated materials and camera behavior are not.

## Final checks

- Both requested CLI gates: `platform_bf_fox` 300 ticks × 49 keys and
  `idle_ys_fox` 600 ticks × 49 keys, zero divergences.
- `cargo gate`: **662 passed, zero failed, one pre-existing ignored doctest**.
  This includes all 73 `melee-sim --test m4_gate` tests, all `hsd-particle`,
  `melee-ft` and `melee-gr` suites, and existing bone/native-reference gates.
- Focused runs: `cargo test -p melee-gr`, `cargo test -p hsd-anim --test load`,
  `cargo test -p hsd-particle --test live_ys_idle -- --nocapture` passed.
- `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all`
  and `git diff --check` passed.

## Changed files

Paths below are relative to the repository root; the list includes Part 1.

- `crates/hsd-archive/src/desc.rs`, `src/desc/spline.rs`: bounded linear spline reader.
- `crates/hsd-anim/src/{lib.rs,spline.rs,load.rs,jobj.rs}`: spline evaluation,
  reference resolution and matrix translation; linear path behavior test.
- `crates/hsd-particle/src/{generator.rs,system.rs}`: queued attachment baking.
- `crates/hsd-particle/tests/{live_ys_idle.rs,support/dust_replay.rs,support/fixture_spawns.rs}`,
  `tests/data/{idle_ys_spawns.json,README.md}`: particle replay and input provenance.
- `crates/melee-ft/src/collision/air.rs`, `src/desc/common.rs`,
  `src/fighter/{assets.rs,landing.rs,mod.rs,pass.rs,procs.rs,spawn.rs,squat.rs,state.rs,walk.rs}`:
  Pass entry, callbacks, drop velocity and motion scratch inheritance.
- `crates/melee-gr/src/{lib.rs,desc.rs,battle/lights.rs,last/animation.rs}`,
  `src/story/{mod.rs,procs.rs}`, `tests/real_story.rs`: stage controller,
  resources, animation/collision binding and archive checks.
- `crates/melee-sim/src/{frame.rs,scene_stage.rs,scenario.rs,initial_state/mod.rs,initial_state/stage.rs}`,
  `tests/m4_gate.rs`: stage registration/restoration, common-bank initialization,
  detached puff requests and both new gates with particle ledger tests.
- `docs/{BATTLEFIELD.md,YOSHIS_STORY.md}`, `TRACKER.md`: findings and status.
