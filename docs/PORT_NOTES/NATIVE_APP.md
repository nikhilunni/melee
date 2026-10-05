# Native application boundaries

> Superseded for the app's current shape (menus, disc image input, the C
> API, the web host) by `docs/APP.md`; this note records the first build.

The library extraction is committed as e995c04. This next milestone follows
Ghostty's separation of shared application behavior and native runtime adapters:
https://github.com/ghostty-org/ghostty/blob/main/src/apprt.zig

- `hsd-archive`: the sole owner of original on-disc layouts; decode visual data
  here alongside existing archive and animation descriptors.
- `melee-lib`: deterministic match state and renderer-independent presentation
  access. Visual resources are opt-in; ordinary headless use has no GPU dependency.
- `melee-platform`: shared session timing, logical input, camera, presentation
  buffers, and wgpu rendering. A narrow C ABI wraps this same Rust implementation.
- `apps/macos`: Swift/AppKit window lifecycle, physical key translation, menus,
  focus and display scheduling. No asset parsing, match rules, or duplicated
  renderer in Swift. Other native shells can reuse the Rust session/renderer.

Decode/upload immutable geometry and textures once. Reuse presentation and GPU
buffers; never clone a whole match to render a frame. Simulation advances in
fixed ticks independent of display refresh. Presentation reads must not change
simulation state or RNG. Keep platform conditionals at surface construction and
native event handling; do not add an abstract backend framework above wgpu.

First visual scope: original Fox/Marth models and Final Destination with basic
materials. Full GameCube material combiners, shadows, effects, and pixel-exact
rendering are subsequent work, distinct from bit-exact gameplay.

## First playable build

Build and run on macOS 14 or newer:

```
tools/run-macos.sh
```

The bundle is `target/macos/Melee.app`. It contains no game data. The run script
passes the existing extracted disc directory; launching the bundle directly
asks for that directory. `tools/build-macos.sh` only builds the bundle.

| Action | Fox (P1) | Marth (P2) |
|---|---|---|
| Move / aim | WASD | Arrow keys |
| Attack | K | N |
| Special | J | M |
| Jump | Space | Comma |
| Shield | L | Period |
| Grab | I | Slash |

Command-P pauses/resumes, Command-R restarts, Command-Q quits. Losing focus pauses
and clears held input without clearing an explicit user pause. Rust publishes
whether another frame is needed; the shell stops its display link while paused
or finished. The session runs fixed 60 Hz ticks, latches quick button
taps until a tick consumes them, and normalizes keyboard sticks through the
existing public controller adapter. Resizing and display refresh do not set the
simulation rate. A host stall longer than 250 ms is treated as a pause; ordinary
catch-up is bounded to eight ticks per display callback with retained remainder.

The first shell advances and draws on the main thread, using AppKit's display
link and a FIFO surface with two frames of desired latency. All session behavior
and drawing are Rust code. A dedicated worker can be introduced behind this
boundary if measured drawable waits make input latency a problem; there is no
new generic scheduler or renderer abstraction above wgpu.

`Presentation` is opt-in by construction and owns reusable matrix scratch.
`MatrixPose` reuses the existing HSD matrix evaluator, copying only pose state;
it does not copy animation tracks or the match. Geometry and textures are parsed
by hsd-archive, then uploaded once. Archive-local image caches share immutable
pixels without a global cache. Each frame uploads the matrix palettes and draws
visible geometry with GPU skinning. Presentation capture allocates nothing and
is tested not to mutate simulation snapshots.

## Current rendering scope (2026-09-11)

The Fox/Marth/Final Destination application now draws original skinned fighters,
held weapons with opening/recoil animation, projectiles, both Illusion afterimages,
live model effects, particle sprites/trails/points, shield volumes and planar
fighter shadows. All 17 FD background phases run through shared stage state.
Authored joint/material/texture animation, prepared image variants and mip chains,
texture combiners, alpha/depth/blend state, directional lighting, perspective and
4x MSAA are implemented in the shared Rust path. Swift remains the native window,
input, scheduling and text HUD adapter. No renderer or animation clock is duplicated
in the shell, and headless consumers do not initialize GPU resources.

This completes the planned first-app rendering features, not retail pixel equivalence. Remaining fidelity limits are explicit:

- Camera fitting is application policy, without the retail tracking/screen-KO
  camera behavior. Planar silhouette shadows are application presentation rather
  than a port of the retail shadow-map filter.
- Transparent meshes use coarse origin sorting; intersecting triangles and model
  effect priorities are not covered by a GX framebuffer oracle.
- Exact GX color/alpha rounding, destination-alpha behavior, bias-clamp/edge-LOD,
  complete generated texture coordinates and toon/lightmap ordering remain
  unverified or approximate. AppSRT camera handling is not a full retail port.
- The current assets have no active shape-deformation tracks. Unsupported shape
  animation, unsupported light types and logic blending fail explicitly.
- A laser hitting a shield still reaches the separately tracked unimplemented
  gameplay response. This is not hidden or fixed by presentation.
- Audio and retail menu/HUD artwork are outside this rendering milestone.

The C ABI remains an initial application adapter, not a versioned public SDK.
Calls for a handle must be serialized; its CAMetalLayer must outlive it. Only the
macOS shell has been built and tested. Other native shells can reuse the same
session and renderer with their own surface.

## Verification

- Original Fox: 6,658 triangles / 73 joints; Marth: 7,031 triangles / 90 joints.
- Final Destination: 13,597 triangles and 68 texture-layer references decoded.
- Synthetic winding, fixed-point, tiled texture, packed color, transparency,
  truncated-data and cache-identity tests pass.
- Presentation capture: zero allocations and unchanged simulation snapshots over
  600 ticks, including laser spawning. Instance population matches moving items,
  matrices stay finite, and reset clears all projectile instances.
- Resource ownership test passes after sharing retained character archives.
- Camera tests cover landscape/portrait framing and missing/nonfinite targets.
- Final envelope-transform regression and presentation allocation test pass in
  release; session tests pass after the focus/pause ownership change.
- Session tests: 30/60/120/144 Hz display schedules produce 60 ticks/second;
  a press/release between frames survives exactly until the next simulation tick.
- `tools/build-macos.sh`: native app bundle built successfully.
- Offscreen wgpu Metal laser render at tick 256 inspected locally.
- Native `--smoke`: resize, focus, pause/resume and keyboard path passed; tick
  233, Fox x moved from -60 to -0.2800008. Focus changes preserve user pause.
- Full release workspace gate passed before the final presentation/focus changes
  (1,157 passed, zero failures, three existing ignored tests). The debug workspace
  run was stopped at the user's request. App changes use focused decoder,
  skinning, presentation-allocation, session and native-window checks; changes
  affecting simulation behavior still warrant the full exactness regressions.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.

GPU/window smoke checks require a macOS graphical session. The normal workspace
test gate remains headless and does not require a GPU. For a local offscreen
preview through the production renderer:

```
cargo run --release -p melee-platform --example render_frame -- \
    harness/roms/files /tmp/melee-frame.rgba --laser
```

Output is a 1280x720 tightly packed RGBA8 image. Game-derived previews stay outside
source control, just like the original assets. No game data or decomp reference
was modified.

## Changed files

- `crates/hsd-archive/src/visual/{mod,polygon,texture,color,material}.rs`, its module
  export, and `tests/visual.rs`: shared decoding and fixtures.
- `crates/hsd-anim/src/jobj/pose.rs` and its module export: reusable read-only
  matrix scratch and envelope transforms using the existing matrix kernels.
- `crates/melee-gr/src/last/animation.rs`: read-only live stage pose access.
- `crates/melee-lib/src/presentation.rs`, `assets.rs`, `scene_items.rs`, `game.rs`, `lib.rs`, and
  `tests/allocation.rs`: curated presentation access and its checks.
- `crates/melee-platform/{Cargo.toml,src/*,include/melee_platform.h,
  examples/render_frame.rs}`: shared session, renderer, surface and C adapter.
- `apps/macos/{main.swift,Info.plist}` and `tools/{build,run}-macos.sh`: native
  shell and bundle scripts.
- `Cargo.lock`, `TRACKER.md`, and this document: dependencies and delivery notes.

## Rendering pass verification (2026-09-10)

Focused release checks: two presentation/ownership tests, four platform
camera/session tests, nine visual decoder/default-material unit tests, and the
original Fox/Marth mesh integration test all passed. Workspace clippy and native
bundle build passed. Native resize/focus/pause/keyboard smoke passed at tick 233;
the offscreen laser preview at tick 256 used the production Metal renderer.
No full workspace simulation suite was rerun for this rendering pass, per the
user's scoped-verification preference. No game data or decomp changes, no commit.

## Custom texture combiners (2026-09-10)

The initial native/rendering work is committed as `a4487fe`. This phase decodes
HSD_TObjTevDesc in `hsd-archive/src/visual/tev.rs`, including independent color and
alpha expressions, Konst/register inputs, bias, scale, clamp, and comparisons.
`melee-platform/src/tev.wgsl` evaluates the expression before the existing texture
mapping stage. Shader generation remains shared with the native renderer.
`examples/material_probe.rs` checks ten numeric GPU fixtures explicitly on a
GPU host; it adds no GPU requirement to ordinary workspace tests. Run it with
`cargo run --release -p melee-platform --example material_probe`.

This is display-float evaluation, not a claim of bit-exact GX fixed-point
rounding. Material animation, lighting and precise framebuffer behavior remain.

Combiner phase validation: descriptor and real-asset tests, allocation/nonmutation
regression, ten Metal numeric fixtures, workspace clippy and native build pass.

## Particle and shield presentation (2026-09-10)

`hsd-archive/src/visual/particle.rs` reads bank-relative particle image and palette
tables using the same tiled GX pixel decoder as model textures. Immutable effect
archives remain shared with assets. `melee-lib/src/presentation/sprites.rs` captures
live particle position, size, rotation, image, interpolated colors and AppSRT into
a prepared buffer without changing simulation state. Color interpolation reuses
the existing integer countdown calculation in `hsd-particle/src/color.rs`.

`melee-platform/src/sprites.rs` uploads an atlas once and batches instanced quads
through shared wgpu pipelines, retaining alpha/additive blending and depth flags.
Shields use the live bone transform, health-dependent size, tilt, and opacity;
the application chooses player colors and a procedural hemisphere appearance.
Swift contains none of this rendering behavior. Capture remains allocation-free.

These are initial billboard effects, not complete retail particle rendering:
trail/direction/point geometry, particle lighting/fog, custom forms, model effects,
and exact framebuffer comparisons remain. Shield shading is procedural. Atlas
capacity is explicitly bounded to 2048 square with an error on overflow.

The offscreen example accepts `--shield` or `--laser`. Combining these exposes an
existing simulation limitation: `item shield response` is unimplemented when a
laser reaches a shield. The rendering regression exercises the two at separate
times; this phase does not change combat behavior.

Changed files: the archive visual decoder module and texture helper; particle
color helper; library assets, presentation and allocation test; platform sprite
renderer/shader, renderer integration, module declaration and offscreen example;
this report and `TRACKER.md`.

Validation: all release `hsd-particle` tests and recorded particle replays pass;
11 archive visual unit tests, five platform tests, both presentation allocation
tests (including both complete matches), workspace clippy, native bundle build,
resize/focus/pause/keyboard smoke and Metal shield/laser previews pass.
Full workspace simulation gates were not rerun, per the requested focused scope.

## Authored directional lighting (2026-09-10)

Particle/shield work is committed as `5f709a1`. The next phase reads the selected
Final Destination light table, including its position animations. The new archive
reader is shared with the existing static stage-light reader. The animation
descriptor adapter in `hsd-anim/src/load.rs` is reused, as are AObj/FObj and the
audited linear-spline evaluator. No new simulation clock or RNG is introduced.

`melee-lib/src/presentation/lighting.rs` samples prepared light tracks at the
absolute match tick. A fresh presentation, sparse capture, repeated capture and
reset therefore give the same light state. Sampling seeks encoded tracks each
capture; this is bounded by asset track size rather than elapsed match duration.
Light animation phase relative to retail rendering has not been measured with a
Dolphin lighting oracle. The current graphical projection remains approximate.

`melee-platform/src/lighting.rs` packs the small light set into a fixed GPU
uniform. Materials retain authored ambient, diffuse, specular and shininess.
Lighting runs per vertex, with GX-style rational specular attenuation; normals
use inverse transpose including reflected-transform signs. This remains display
floating-point shading, not exact GX quantization, complete toon/lightmap stage
ordering, or a pixel oracle. Point/spot lights, light color/interest animations,
constraints and nonlinear light paths fail explicitly in this initial FD view.

Changed files: archive visual light reader/export; animation loader visibility;
stage static light adapter; library presentation/material/light capture and
allocation tests; platform light uniform, material shader/resources, renderer,
module declaration and GPU probe; tracker and this report.

Lighting validation: both complete-match capture/allocation checks, capture-frequency
and reset checks, real Battlefield light assets, real Fox/Marth visual assets, five
platform tests, 13 Metal numeric fixtures, workspace clippy, native bundle build
and resize/focus/pause/keyboard smoke pass. Final Metal preview inspected at tick
260. Full workspace simulation gates were not rerun for this display-only phase.

## Resolved dependency boundary

The original stage fade/material-animation and model-effect dependencies were
subsequently implemented in shared subsystem state, with exact match oracles and
allocation checks. The phases below record that work chronologically; their
"remaining" lists describe the state at that phase, not the current milestone.
The laser-on-shield gameplay response remains a separate dependency.

## Material animation foundation (2026-09-10)

Typed MatAnim/TexAnim readers now preserve texture-map IDs and nullable image/
palette tables. TObj descriptors are read once through shared archive logic:
headless animation retains metadata, while presentation separately decodes pixels.
`hsd-anim/src/tobj.rs` runs UV, image, palette, blend, LOD-bias and TEV constant
tracks through the existing AObj/FObj interpreter; MObj/DObj owns their timing.
The archive-to-runtime adapter converts material-animation trees before attachment.
TObjUpdateFunc's retail assembly at 0x8035E860 has no fused instructions; color
conversion reuses MObj's double-precision multiply and byte-store helper.

Validation: hsd-anim and hsd-archive release suites; texture track/clone regression
in debug and release; nullable-table/cycle reader regression; all six library
allocation tests; both 6083/10059-tick full-match oracles; workspace clippy pass.
This foundation does not yet attach stage material tracks or upload live GPU
material values; that integration is the next phase of the same rendering task.
Changed files are the new archive material-animation reader, shared texture
metadata decoder, animation TObj/MObj and loader glue, focused tests and docs.

## Live material playback (2026-09-10)

Stage models now attach prepared material and texture clocks, including the
previously static platform models. Animation switches retain FObj capacity;
clones preserve it. Presentation reads current colors, alpha references, UV
transforms, TEV constants and selected images without changing the match.
Possible images and palettes are decoded at construction and uploaded to stable
portable 2D texture arrays. The shader addresses each filtering tap within the
selected image's dimensions, so differently sized frames retain wrapping without
sampling array padding. Material uniform uploads are skipped when unchanged.

Validation: seven library allocation/continuation/capture tests, including both
complete matches, pass. Both full-match fighter/item/ordered-particle oracles
passed after stage clock integration. Workspace clippy passes. The 15 Metal shader fixtures and a real
1280x720 Metal frame pass. Authored mip levels/LOD, stage overlay fade scripts,
held weapons, model effects, specialized particles, shadows and precise camera/
framebuffer behavior remain separate work.

## Complete FD background cycle (2026-09-10)

Typed color-overlay scripts now drive shared `melee-lb` playback. The stage
controller reads completion after Ground's animation callback; creation and
removal use the real scheduler, with preloaded trees reset in reserved storage.
The overlay is applied after texture/lighting composition. No second stage timer
lives in the application. Assembly audits of lb_800140F8 / lb_80014258 found no
fused instructions; the half-unit bias and byte conversion are preserved.

The 17-phase cycle returns to phase 1 at tick 13073 in the cold seed-42 run.
Debug/release cycle tests and a 27,000-tick allocation/capture/clone check pass.
The latter exposed free-list growth on map destruction; scheduler initialization
now reserves removal capacity for every prepared slot. Both exact full-match
oracles, scheduler tests, workspace clippy, native build/smoke and late-phase
Metal frame generation pass. Late backgrounds still need camera/framing fidelity;
these checks do not establish pixel equivalence with Dolphin.

Stage fade timing follow-up: grLast_8021B920 explicitly calls the interpreter
once more after grMaterial_801C9604 already interpreted the request. The first
fade color is now asserted as [3, 3, 3, 4]; the cycle and 27,000-tick allocation
checks pass. Both full-match oracles also passed in debug (as well as release).

## Perspective and framebuffer phase (2026-09-11)

Meshes and particles now share one camera shader and uniform. Perspective uses
Melee's default 30-degree vertical FOV, near/far planes, and initial viewing
direction; fighter fitting remains application policy. Eye-relative projection
avoids near-plane cancellation in tall windows. Shield hemisphere depth and
specular view vectors use this same camera. Authored polygon face culling uses
GX clockwise winding (also reflected in Dolphin's Vulkan front-face setting).

The renderer uses 4x multisampling when the adapter supports it for both color
and depth, and recreates attachments on resize. The stage's live background color
is converted through an immutable sRGB transfer table for the clear operation.
No clock, GPU state, or application policy was added to match simulation.

Six platform tests, including actual perspective clip/depth checks, all eight
library allocation tests (including 27,000 stage ticks), workspace clippy, 15
Metal arithmetic fixtures and native build/resize/focus/pause/keyboard smoke pass.
Opening stars, the shield and late vortex were inspected in Metal frames. Full
workspace simulation tests were not rerun for this display-only phase. Exact
retail camera tracking, GX pixel quantization, authored mip/LOD filtering and
remaining item/effect/shadow rendering are still outstanding.

## Final particle and shadow phases (2026-09-11)

Particles use authored orientation, ribbon trails, point/line framebuffer sizing,
alpha-test references and depth-write modes. Tornado trail reconstruction follows
the audited retail fused operations. Color and alpha-reference interpolation share
one fixed-point helper; presentation does not advance either track.

Planar shadows reuse fighter and held-article vertex/index buffers and current
skinning matrices. Read-only capture selects live floor segments beneath fighters,
including disabled/offstage rejection. A shared depth/stencil attachment restricts
shadow coverage to visible foreground stage pixels and blends each covered pixel
once, even where caster triangles overlap. The shadow and material passes share
one vertex layout; there is no shadow mesh copy or shadow animation clock.

Particle suite and workspace clippy pass. All eight allocation checks pass,
including both complete-match captures and the 27,000-tick stage cycle. Library
API and platform tests pass; the floor regression covers running off the ledge.
A Metal shadow-on/off comparison changes 230 pixels beneath the two fighters at
tick 240, bounded by x338..924/y410..417. This is a visibility check, not a retail
shadow oracle. Native and final GPU checks are recorded in TRACKER.md.

## Screenshot lighting correction (2026-09-11)

The screenshot comparison exposed two specific omissions. WObj spline positions
were consumed in local coordinates instead of being transformed by the referenced
JObj (`HSD_WObjGetPosition`, 0x8037D720; PSMTXMultVec call at 0x8037D7C4).
At tick 327 both lights incorrectly became [-17.44388, 0, -4.9638906]. The existing
joint loader and audited matrix kernel now preserve the first path's y=7.5 and
the second path's y=-2 plus its half-turn rotation. No light intensity or global
brightness was changed, and no simulation state is mutated.

The central white region is a vertex-alpha glow mesh, not a specular highlight.
Its JObj has BILLBOARD set (flags 0x80200). Presentation now exposes authored
view-plane/view-point billboarding, and the shared vertex shader orients the mesh
while retaining scale, projected up axis and translation (`HSD_JObjMakePositionMtx`,
0x803740E8). Normal meshes retain their complete original transform. This restores
the white glow at the same tick and camera used for the before capture.

Material composition now follows `MObjMakeTExp` (0x80363284): diffuse/ambient maps,
raster lighting, separate specular maps and illumination, then EXT maps. Textures
within one category all contribute alpha; a texture reused in a later category
does not apply alpha twice. Reflection coordinates use view-space normals, and
highlight coordinates use the first directional light's half-vector. GPU display
arithmetic remains distinct from a bit-exact GX framebuffer implementation.

Changed files: `melee-lib/src/presentation.rs`, `presentation/lighting.rs`,
`tests/allocation.rs`; `melee-platform/src/material.rs`, `render.wgsl`,
`renderer.rs`, `examples/material_probe.rs`, `examples/render_frame.rs`; tracker
and this report. The optional `--inspect-materials` preview argument reports
actual visible material flags and light vectors for future comparisons.

Validation: all eight release allocation tests, including both complete-match
captures; light separation/capture-frequency/reset checks; 24 Metal numeric
fixtures (six new category/alpha/billboard cases, existing expectations unchanged);
workspace clippy. Same-frame Metal before/after previews confirm the brighter
fighters and restored central glow. Camera framing was held fixed, and the user
screenshots themselves are not an aligned retail pixel oracle.

Final checks also pass in debug for the light-path regression and in release for
all six platform tests. The rebuilt native smoke passes at tick 226, with resize,
focus, pause/resume and Fox movement from -60 to -0.2800008 verified. An initial
shader version passed the full material structure through composition stages and
failed native timing (ticks 19 and 9). Compact composition state removes unused
texture/TEV metadata from that path; all 24 numeric outputs remain unchanged.
The smoke threshold was not modified. No full workspace simulation rerun was
needed for these display-only changes; workspace clippy passes.
