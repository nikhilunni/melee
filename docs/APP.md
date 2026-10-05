# The playable app: one core, native hosts

The game ships as a macOS app and a web page with the same flow: give it a
Super Smash Bros. Melee NTSC-U 1.02 disc image, pick two characters and a
stage, play, see the results. Nothing is hard-coded: no ISO path, no
extracted files directory.

The structure follows Ghostty (libghostty plus a native "apprt" per
platform). A shared Rust core owns all application logic and the
renderer; each platform host is a thin native layer that draws menus with
its own toolkit, forwards user intent and gives the core a surface to draw
the game into. Hosts hold no game or menu rules.

```
            +-------------------- libmelee (crates/melee-platform) ---------------------+
            | gc-disc       disc header + FST, validation (sans IO)                     |
            | disc.rs       DiscFiles: which bytes a match needs; fetched-file cache    |
            | catalog.rs    character / stage ids, names, costume counts, CSS grid      |
            | app.rs        App: screen flow, picks, costume rules, config, loading,    |
            |               match, results, rematch, notices                            |
            | session.rs    fixed 60 Hz ticks, input latching, pause, recording         |
            | surface.rs    WindowRenderer: one wgpu device per surface, scene follows  |
            | renderer.rs   the match renderer (shared with render_frame)               |
            | preview.rs    stage previews rendered from the disc at runtime            |
            +-------------------+--------------------------------+----------------------+
                                |                                |
               C API (ffi.rs, include/melee_platform.h)   wasm-bindgen (crates/melee-web)
                                |                                |
            apps/macos: Swift + AppKit (SwiftUI views)   crates/melee-web/www: HTML/CSS/JS
            NSWindow, menu bar, open panel, Dock drop,   drop zone, file picker, DOM menus,
            CAMetalLayer, CADisplayLink, keyboard        WebGPU canvas, rAF loop, keyboard
```

## The flow (`app::App`)

```
Disc -> Characters -> Stages -> Loading -> Match -> Results
```

- **Disc.** The host hands the core a disc (natively a path; on the web the
  header and file-table bytes it read). `gc-disc` accepts only `GALE01`
  revision 2 and names what it found otherwise: another revision (1.00,
  1.01), region (PAL, Japanese), game, a Wii disc, or a container format
  (.zip, .7z, .rvz, .wia, .gcz, .ciso, .wbfs, NKit). A short image fails
  before any file is read.
- **Characters.** Two keyboard players. Character ids are indices into
  `Character::ALL` (the retail CSS reading order; `catalog::grid_cell`
  gives the retail 9/9/7 grid, Sheik in Kirby's cell). Picking the other
  player's character moves to the first costume they are not wearing;
  costume changes skip theirs. Stocks 1 to 99.
- **Stages.** Choosing one builds the `MatchConfig` with a host-random seed
  (recorded in the replay) and lists the files to read
  (`GameAssets::files`, minus those already fetched).
- **Loading.** The host reads each request's byte range and hands it over
  (`provide_file`, length-checked against the FST); natively the core
  reads the image itself, one file per `load_step` so the host can show
  progress. `finish_loading` builds the session; a failure (a character or
  stage that cannot be presented yet, a renderer that rejects the scene)
  returns to stage select with the error as the notice. Fetched files stay
  cached for the next match (a rematch loads nothing).
- **Match.** `advance(host time)` runs fixed ticks; the HUD is read each
  frame. A fault stops the match: natively the replay is saved to the temp
  directory and the notice names it; the host offers Save Replay. A
  finished match moves to Results.
- **Results.** Winner (or draw), final percents and stocks; Rematch (same
  picks, new seed) or back to character select.

**Menu art** (`art.rs`) comes from the user's disc at runtime, never from
the repository. After `open_disc` the core asks for `MnSlChr.usd`,
`MnSlMap.usd` and `IfAll.usd` (about 5 MB; `art_requests`, handed over
through `provide_file` on any screen; natively `load_art`, which the C
API's open-disc call runs) and decodes each image on first use, found the
way the retail menus find it (model, joint, texture animation frame;
`art/retail.rs` cites the code). Images are RGBA8, straight alpha, native
size:

| Piece | Size | Notes |
|---|---|---|
| Portrait (character, costume) | 136x188 | the select-screen portrait; the best art for a modern menu |
| Face (character) | 64x56 | grid face with its name plate |
| Stock (character, costume) | 24x24 | in-match stock icon |
| Character emblem | 80x64 | series emblem, intensity (tint its alpha) |
| Stage icon | 64x56 | 48x48 for Past Stages (Dream Land); retail has no 2D preview (the stage select shows 3D miniatures) |
| Stage name | 224x56 | series above the stage name, intensity |
| Stage emblem | 64x64 | retail's faint watermark (alpha peaks at 119) |

| Stage preview | 1920x1080 | rendered, not decoded (below); opaque, smooth: scale with filtering |

Sheik has no portrait or face (retail picks her through Zelda's); her
emblem is Zelda's, her stock icons her own. `examples/art_survey.rs` dumps
every texture of an archive to `target/` for exploring.

**Stage previews** (`preview.rs`, `MELEE_ART_STAGE_PREVIEW`): the disc
has no 2D stage picture, so the core renders one per stage. The art
requests also list the files the previews need (about 16 MB: the six
stages, Jigglypuff twice, the common fighter and effect files; kept in the
fetched-file cache, so a match on that stage reuses them). Once they are in
and a surface exists, the core runs a short match per stage (Jigglypuff
twice, 300 neutral ticks so platforms, water and backgrounds settle;
Pokemon Stadium stays in its neutral form), builds a stage-only renderer
(`Renderer::stage_only`: no fighters, items, effects, shadows or sprites;
the match is only read) and draws it through a per-stage hero framing
(`preview::framing`: visible width, interest height and a slightly
elevated pitch, chosen by eye) at 3840x2160, then averages 2x2 blocks in
linear light to 1920x1080 RGBA8 sRGB. The match and renderer are dropped
after each image; the six images (8.3 MB each, about 50 MB) stay. It
runs on the surface's own device: natively on a background thread (0.15 to
0.3 s per stage on an M-series Mac, about 1.25 s for all six), on the web
as steps between page tasks (a `MessageChannel` message, which hidden tabs
do not throttle; 0.4 to 0.7 s per stage in Chrome, 3.4 s for all six, the
page blocked at most about 0.35 s at a time by one step: posing the scene or
building the renderer). Until a preview is rendered the art call fails (C)
or returns null (wasm); a preview that fails stays so and is logged.
`melee_app_stage_previews_ready` / `stage_previews_ready()` turn
true when all six are settled. `examples/stage_previews.rs` writes them to
`target/` for inspection (never commit them).

Everything above is unit-tested without a GPU (`cargo test -p
melee-platform`: flow, costume rules, config, caching, art requests) and
on the real disc (`tests/disc_flow.rs`: open the image, load, play,
rematch, reuse cache; `tests/disc_art.rs`: every piece's size, distinct
picks decode to distinct images, determinism; `tests/stage_previews.rs`,
skipped without a GPU adapter: every preview's size, variety, distinctness
and determinism, compared in memory).

## The C API

`crates/melee-platform/include/melee_platform.h`, version
`MELEE_API_VERSION` (3; bump on any incompatible change, hosts check it at
start). One opaque `melee_app_t` on one thread; plain structs out; static
strings in catalog structs; calls that can fail return `bool` and leave
the message in `melee_app_last_error`; user-facing notices come from
`melee_app_take_notice`. The surface is a `CAMetalLayer` the host owns
(`melee_app_attach_metal_layer`); `melee_app_frame(width, height)`
advances by wall time and draws.

The wasm-bindgen API in `crates/melee-web` mirrors it method for method
(`WebApp`), returning plain JS objects instead of structs, plus
`fst_range(header)` and `create_surface(canvas)` for the async parts.

## macOS (`apps/macos`)

```sh
tools/run-macos.sh          # builds target/macos/Melee.app and launches it, no arguments
```

`tools/build-macos.sh` builds the static library and compiles the Swift
files with the C header as the bridging header. The app: an NSWindow with
the standard menu bar (File: Open Disc, Open Recent, Save Replay; Match:
Pause, Restart, Quit to Character Select; Window; Help), drag and drop of
the image onto the window or the Dock icon, recent discs kept as bookmarks
in UserDefaults. The menus (docs/DESIGN.md) are SwiftUI views in an
`NSHostingView` above a Core Animation backdrop and the `CAMetalLayer` view,
drawing the disc's art; the Barlow fonts are registered from the bundle.
Notices and load errors are glass cards; a match fault is a sheet that
offers Save Replay. One file per screen: `DiscScreen.swift`,
`CharacterSelect.swift`, `StageSelect.swift`, `MatchScreens.swift`
(loading, HUD, pause, results); `Theme.swift` holds the tokens and
components, `Navigation.swift` the keyboard paths.

Development smoke test (walks the menus, prints the HUD, exits 0 when the
match is running):

```sh
MELEE_APP_AUTOSTART=$PWD/harness/roms/GALE01.iso:Fox:Marth:FinalDestination \
MELEE_APP_SMOKE_SECONDS=5 target/macos/Melee.app/Contents/MacOS/Melee
```

Fewer fields stop earlier (`iso` at character select, `iso:P1:P2` with the
picks made). Keys are the catalog's (`CaptainFalcon`, `FinalDestination`).
With `MELEE_APP_SMOKE_SECONDS` set it also prints the art sizes and, once
the stage previews are rendered, `previews: ready after N s` with their
sizes (the core logs each preview's time on stderr).
`MELEE_APP_PRINT_WINDOW=1` prints the window id for `screencapture -l`.

For screenshots: `MELEE_APP_WINDOW_SIZE=1280x800` sizes the window;
`MELEE_APP_SCREEN` stops on a screen (`disc-empty`, `disc` with the disc
open, `stages` with `iso:P1:P2`, `loading` held, `pause` or `results` over
the running match after `MELEE_APP_SCREEN_DELAY` seconds; results are a
preview with P1 winning); `MELEE_APP_PREVIEW_PERCENTS=57,142` shows those
HUD percents. `MELEE_APP_KEYS=right,down,enter,tab,e,...` feeds menu keys
to the model 0.25 s apart and prints the cursor after each.
`MELEE_APP_CLICK=x,y` (fractions of the window content, top-left origin)
delivers one real left click to the window after 1 s and prints the screen
and whether a sheet opened: it exercises hit-testing, which `MELEE_APP_KEYS`
bypasses (e.g. `0.438,0.758` is Open Disc… at 1280x800 with a recent disc).
Never test with global synthetic events (`osascript` key codes, `CGEvent`):
they reach whatever app is frontmost.

## Web (`crates/melee-web`)

```sh
tools/run-web.sh            # builds target/web and serves http://localhost:8080
tools/run-web.sh --build    # build only
```

WebGPU only: the renderer reads storage buffers in the vertex stage, which
WebGL lacks. Without `navigator.gpu` the page says so. The disc never
leaves the machine: the page reads the header, the file table and then only
the files a match needs (about 15 MB) with `Blob.slice()`. In Chromium the
last disc is remembered as a File System Access handle in IndexedDB (the
handle, not the file); other browsers ask again after a reload.

The build drops `split-debuginfo` for wasm32 (target rustflags in
`.cargo/config.toml`) and installs the wasm-bindgen CLI whose version equals
the `wasm-bindgen` crate in `Cargo.lock` under `target/tools` (never
`~/.cargo/bin`). `wasm-opt` runs if installed.

Development: `tools/run-web.sh --disc harness/roms/GALE01.iso` also serves
the image read-only at `/disc.iso` (with HTTP Range), and
`http://localhost:8080/?disc=disc.iso&autostart=Fox:Marth:FinalDestination`
walks the menus (`autostart=Fox:Marth` stops at character select).

### Same bits as native

The simulation on wasm32 must produce the bits native aarch64 does. Three
std float operations are target-dependent, and the game never uses them:

- **Single-precision fma.** wasm has no fma instruction, so `f32::mul_add`
  calls musl's `fmaf`, which rounds twice for results in the single
  subnormal range. `gekko_math::fma` keeps the instruction on aarch64 (and
  x86_64 with `fma`) and elsewhere computes the exact double product, a
  TwoSum and a round-to-odd before one conversion to single.
- **Double fma on WASI.** wasi-libc's `fma` returns `x*y + z` when `z` is
  zero, losing an underflowed product's sign. `gekko_math::fma` returns the
  product itself there (`cfg(target_os = "wasi")`); the browser target's
  compiler-builtins `fma` is already correct.
- **`max`/`min` of mixed zeros.** Rust leaves `f32::max(+0, -0)`
  unspecified (aarch64 gives `+0`; wasm follows argument order).
  `gekko_math::cmp::{max, min}` are retail's `fcmpo`-and-branch clamps,
  keeping the first argument on a tie; each call site cites the retail
  comparison it transcribes.

The workspace `clippy.toml` forbids `f32`/`f64` `max`, `min`,
`minimum`, `maximum`, `mul_add` and the libm functions (`sqrt`, `sin`,
`powf` and the rest) outside gekko-math; tests that compare against std
allow them locally.

```sh
tools/wasm-check.sh               # pure crates' tests under wasmtime, then
                                  # 3 matches x 2000 ticks hashed native vs wasm
tools/wasm-check.sh --tests-only
```

It needs `rustup target add wasm32-wasip1` and a wasmtime binary on `PATH`
or in `MELEE_WASMTIME` (a release archive from
github.com/bytecodealliance/wasmtime, unpacked anywhere; the script installs
nothing). The tests run debug and release (LLVM's folds only appear
optimized). From a worktree, set `MELEE_DATA_ROOT` to the main checkout for
the match hash and `MELEE_ALLOW_MISSING_DATA=1` for the crate tests. The
match hash is `crates/melee-lib/examples/cross_target_hash.rs`: per tick, an
FNV-1a of `diagnostics::inspect` and `observe`.

## Known limits

- **Panics on the web.** wasm32-unknown-unknown aborts on panic, so the
  `catch_unwind` in `Match::step` cannot turn an `unimplemented!` into a
  fault there. The panic hook shows a crash overlay with the message and,
  when the panic happened inside a tick, a download of that match's
  replay (`session::panic_replay`), which reproduces the crash natively
  (`melee-sim`/`Recording::replay`). The page must be reloaded.
- **Loading blocks briefly.** Building the match (parsing, scene upload)
  runs on the main thread after the files are read; a few hundred ms.
- **Input.** Two players on one keyboard; gamepads are not supported yet.
- **Rendering.** Some characters and stages cannot be presented yet;
  the app reports the error and returns to stage select.
- **Art.** The core decodes the menu art and renders the stage previews
  (above); both bindings hand them out (`melee_app_art`, `WebApp.art`).
  On the web the preview steps run on the page's thread, so the menus
  may skip frames for up to about 0.35 s at a time while the previews
  render (3.4 s after the files are read).
- The fetched-file cache keeps every file read for the session (about
  15 MB per new character/stage pair).
