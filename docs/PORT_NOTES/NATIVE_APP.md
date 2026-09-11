# Native application boundaries

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

## Current visual limits

This is a playable visual prototype, not a pixel-exact renderer. It now draws
original fighter and laser meshes, plus live stage joint animation and visibility.
Common HSD color/alpha texture operations support up to eight layers, authored
wrapping/filtering and texture SRT. Materials choose depth writes, comparisons,
blend factors, color masks and alpha tests. Backgrounds composite before world
geometry; translucent world meshes sort by palette origin with authored index
as a deterministic tie-breaker. This is coarse mesh sorting, not correct ordering
for every intersecting transparent triangle or instance.

Camera framing fits living fighters with margins and adapts to aspect ratio. It
is application policy, not the retail camera: it currently has no smoothing,
perspective or stage-specific limits. Camera queries do not advance any clock.

Authored lighting, material/texture animation,
mipmaps, destination-alpha behavior, and precise GX blend/color rounding remain
unfinished. Reflection/highlight coordinates use an approximate normal mapping;
other generated coordinate modes still fall back to UVs. In particular, laser
glow and several FD surfaces do not yet look like retail. Logic blend modes and
materials exceeding eight textures fail explicitly.

Held blasters need hand attachments and opening/recoil animation, so they remain
hidden. Afterimages, shields, hit sparks, dust, flames, other particle/effect
visuals, shadows, and audio remain future work. All existing simulation work and
RNG still run. Laser geometry uploads once per kind; live instances use prepared
capacity and ItemCore's position/rotation/scale. Article archives share existing
immutable character data instead of rereading or duplicating it.

The generic decoder rejects unsupported shape-animation polygon modes and GX
primitives explicitly. It supports triangle lists, strips, fans and quads;
indexed/direct positions, normals, colors and UVs; rigid/shared/envelope matrix
bindings; and base-level I4/I8/IA4/IA8/RGB565/RGB5A3/RGBA8/CI/CMPR textures.
Texture decoding has synthetic layout/color tests; it is not yet covered by a
retail framebuffer pixel oracle.

The C ABI is the initial application adapter, not yet a versioned public SDK.
Calls for a handle must be serialized; the CAMetalLayer outlives the handle.
Swift owns the window/layer, and releases Rust rendering resources before the
layer is destroyed. Only macOS has a native shell and has been built/tested;
other platforms can use the shared Rust session/renderer with their own surface.

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
