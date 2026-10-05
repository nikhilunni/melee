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

Everything above is unit-tested without a GPU (`cargo test -p
melee-platform`: flow, costume rules, config, caching) and on the real disc
(`tests/disc_flow.rs`: open the image, load, play, rematch, reuse cache).

## The C API

`crates/melee-platform/include/melee_platform.h`, version
`MELEE_API_VERSION` (1; bump on any incompatible change, hosts check it at
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
in UserDefaults. The menus are SwiftUI views in an `NSHostingView` above the
`CAMetalLayer` view; errors and faults are sheets.

Development smoke test (walks the menus, prints the HUD, exits 0 when the
match is running):

```sh
MELEE_APP_AUTOSTART=$PWD/harness/roms/GALE01.iso:Fox:Marth:FinalDestination \
MELEE_APP_SMOKE_SECONDS=5 target/macos/Melee.app/Contents/MacOS/Melee
```

Fewer fields stop earlier (`iso` at character select, `iso:P1:P2` with the
picks made). Keys are the catalog's (`CaptainFalcon`, `FinalDestination`).
`MELEE_APP_PRINT_WINDOW=1` prints the window id for `screencapture -l`.

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
- **Art.** Menus show names, not portraits or stage previews (the next
  step: decode CSS icons and stock icons from `MnSlChr.usd`/`IfAll.usd` in
  the core and hand RGBA to the hosts).
- The fetched-file cache keeps every file read for the session (about
  15 MB per new character/stage pair).
