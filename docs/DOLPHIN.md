# Driving Dolphin programmatically on macOS arm64 (oracle harness)

Research snapshot: 2026-09-08. Everything below was checked against the linked
sources on that date. Items marked **UNVERIFIED** could not be confirmed and
should not be relied on without testing.

Requirements for the oracle harness:

- (a) load a savestate
- (b) advance frame by frame
- (c) read arbitrary guest memory each frame
- (d) inject GameCube controller input
- (e) ideally: code breakpoints at guest addresses with a callback

## Comparison

| Option | Frame-advance hook | Memory read | Input inject | Code breakpoints | Savestates | Headless | macOS arm64 | Maintenance |
|---|---|---|---|---|---|---|---|---|
| **Felk Python-scripting fork** | Yes, `event.on_frameadvance` / `await event.frameadvance()` (fires from CPU thread at frame begin) | Yes, `memory.read_u8..u64/s8..s64/f32/f64` (scalars only, no bulk read) | Yes, `controller.set_gc_buttons(port, {...})`, per-frame override | Callback yes (`event.on_codebreakpoint`); **adding** the breakpoint has no Python API, must use Dolphin debugger UI | Yes, `savestate.load_from_file/slot/bytes`, `save_*` | No headless (Qt frontend only carries `--script`) | No prebuilt binary (Windows x64 only). Source builds on arm64 mac in CI (Dec 2025); `find_package(Python3)` works with Homebrew Python | Low-activity, single maintainer ("half-heartedly at best"). Last commit 2025-07-02; preview4 release 2025-12-06 |
| **Mainline Dolphin GDB stub** | No (single-instruction `s` only; no frame step) | Yes (`m` packet) | No | Yes (Z0/Z1 + watchpoints Z2-Z4), via GDB/RSP client | Not via stub (use `-s` CLI flag or Qt hotkeys) | Yes (`dolphin-emu-nogui --platform headless`), builds on macOS | Yes, native arm64 mainline builds | Actively maintained (upstream) |
| **libmelee + Slippi Dolphin** | Yes, `Console.step()` returns one `GameState` per frame (Slippi spectator stream, UDP 51441) | Only Slippi-serialized fields (player pos, action state, %, etc.) -- not arbitrary addresses | Yes, Dolphin named-pipe controller (`Pipes/slippibot1`) | No | Not exposed (Slippi netplay build; savestates not part of libmelee API) | Headless only for **mainline** Slippi Dolphin (`--platform headless`); mainline Slippi Dolphin **segfaults on macOS** per libmelee | Ishiiruka runs under Rosetta 2 only; mainline Slippi ships a `-Mac.dmg` but libmelee says it crashes on macOS | altf4/libmelee archived 2026-01-25; vladfi1 fork active (pushed 2026-08-17) |
| **Mainline MemoryWatcher (unix socket)** | Implicit: one datagram per frame (`OnFrameEnd`) | Only pre-listed addresses in `Locations.txt` (pointer chains OK); push-only, changed values only | No | No | No (combine with `-s`) | Yes with nogui | Yes (Unix only; macOS OK) | Upstream, but a legacy feature |
| **Dolphin-Lua-Core (SwareJonge / dragonbane0) + Dolphin-Lua-Mac** | Yes (Lua) | Yes (Lua) | Yes (Lua) | UNVERIFIED | Yes | No | Mac port is x86_64, macOS 10.12-11 only; based on Dolphin 5.0 | SwareJonge repo "will not be updated anymore"; TASLabz says "do not use - obsolete" |
| **dolphin-tool** | n/a | n/a | n/a | n/a | n/a | n/a | n/a | Disc-image utility only (`convert`, `verify`, `header`, `extract`). Not an emulation driver |
| **Embedding as a library / libretro core** | libretro API has frame-step + memory map | libretro `retro_get_memory_data` | libretro input callbacks | No | libretro serialize | Yes | libretro Dolphin core has **no arm64 macOS build** and no working macOS build at all | libretro Dolphin core outdated; Dolphin's `core` static lib has no public API (would mean writing your own frontend, like `DolphinNoGUI`) |

## Recommendation

Use **Felk's Python-scripting fork, built from source on macOS arm64**, with
the harness as an in-process Python script. It is the only option that gives
(a)-(d) in one process with a per-frame hook, and it gives (e) as a callback
if breakpoints are set through Dolphin's debugger UI.

Fallback / complement: **mainline Dolphin with the GDB stub** for (e) when you
need many breakpoints scripted without a GUI, or the **MemoryWatcher** socket if
you only need a fixed list of addresses per frame from unmodified mainline
Dolphin. Neither can advance one *frame* on command, and neither injects input.

Do not build the oracle on libmelee: it only sees Slippi's serialized subset,
requires a Slippi (Gecko-patched) build, and its macOS story is Rosetta-only
(Ishiiruka) or crashing (mainline Slippi).

### Why not each alternative

- **GDB stub**: `Step()` sets `CPU::SetStepping(true)` -- single instruction,
  not a frame ([GDBStub.cpp](https://github.com/dolphin-emu/dolphin/blob/master/Source/Core/Core/PowerPC/GDBStub.cpp)).
  No `vCont`. You could emulate a frame step by breaking on a known per-frame
  address (e.g. the VI interrupt handler or Melee's main loop), which is
  workable but slow and clunky compared to an in-process hook. No input path.
- **libmelee**: state comes from Slippi's spectator protocol, not memory;
  `Console.step()` cannot read arbitrary addresses
  ([console.py](https://raw.githubusercontent.com/vladfi1/libmelee/master/melee/console.py)).
  The README says "On MacOS, mainline slippi dolphin crashes (segfaults) for
  unknown reasons. You should use Ishiiruka instead", and Ishiiruka on Apple
  Silicon "runs under the Rosetta 2 translation layer"
  ([vladfi1/libmelee README](https://raw.githubusercontent.com/vladfi1/libmelee/master/README.md),
  [Slippi on macOS wiki](https://github.com/project-slippi/Ishiiruka/wiki/Slippi-on-macOS)).
  Retail Melee: libmelee's setup requires Slippi and its Gecko codes
  (`GALE01r2.ini`); the game-state stream itself is produced by Slippi's EXI
  device + codes, so unmodified retail Melee does not produce it
  ([altf4/libmelee](https://github.com/altf4/libmelee)).
- **MemoryWatcher**: push-only datagrams of *changed* values for addresses in
  `User/MemoryWatcher/Locations.txt`, over `AF_UNIX`/`SOCK_DGRAM` at
  `User/MemoryWatcher/MemoryWatcher`; stepped in `Core::OnFrameEnd` on the CPU
  thread ([MemoryWatcher.cpp](https://raw.githubusercontent.com/dolphin-emu/dolphin/master/Source/Core/Core/MemoryWatcher.cpp),
  [Core.cpp](https://raw.githubusercontent.com/dolphin-emu/dolphin/master/Source/Core/Core/Core.cpp),
  [CommonPaths.h](https://raw.githubusercontent.com/dolphin-emu/dolphin/master/Source/Core/Common/CommonPaths.h)).
  Good for a small fixed address set, useless for "arbitrary memory".
- **Lua forks**: all Dolphin-5.0-era, the maintained one says "will not be
  updated anymore", the Mac port targets macOS 10.12-11 on x86_64
  ([SwareJonge/Dolphin-Lua-Core](https://github.com/SwareJonge/Dolphin-Lua-Core),
  [Hibyehello/Dolphin-Lua-Mac](https://github.com/Hibyehello/Dolphin-Lua-Mac),
  [TASLabz/dolphin-lua-core](https://github.com/TASLabz/dolphin-lua-core)).
- **TAS Input**: mainline's TAS Input is a GUI window, not a scripting API.
- **dolphin-tool**: disc-image CLI only
  ([DolphinTool](https://github.com/dolphin-emu/dolphin/tree/master/Source/Core/DolphinTool)).
- **libretro core**: missing from arm64 macOS RetroArch and reported as having
  no working macOS build ([RetroArch #16625](https://github.com/libretro/RetroArch/issues/16625),
  [RetroArch #6841](https://github.com/libretro/RetroArch/issues/6841)).

## Felk fork: verified facts

Repo: <https://github.com/Felk/dolphin> (default branch `master`, not archived).
Stubs (the API documentation): <https://github.com/Felk/dolphin/tree/master/python-stubs/dolphin>.

- Last commit on `master`: 2025-07-02 (`a22afcb`, merge of PR #57 "CMake: Ignore
  python as dependency in bundle to fix verify_bundle failure in Mac OS").
  Tag `scripting-preview4` (2025-12-06) points at that same commit.
- Upstream base: `master` merged `upstream/master` on 2024-12-19 (Dolphin
  ~2412). A user reports the build as "Dolphin 2412-155-dirty"
  ([issue #58](https://github.com/Felk/dolphin/issues/58)).
- Releases (all Windows x64 only): `dolphin-scripting-preview{1..4}-x64.7z`
  ([releases](https://github.com/Felk/dolphin/releases)). No macOS or Linux
  binaries. Felk in issue #58: "Given how half-heartedly at best I work on this
  project ... I switched to Linux."
- Readme FAQ: "Why does it only exist for the x86-64 architecture? There is
  nothing fundamentally stopping this to work on ARM as well, but currently
  only the Python externals for Windows x86-64 are bundled"
  ([Readme](https://raw.githubusercontent.com/Felk/dolphin/master/Readme.md)).
  On non-Windows the build uses `find_package(Python3 REQUIRED COMPONENTS
  Development)` ([Scripting/CMakeLists.txt](https://raw.githubusercontent.com/Felk/dolphin/master/Source/Core/Scripting/CMakeLists.txt)),
  so a Homebrew Python is enough; the `Externals/python` submodule is only the
  Windows bundle ([Felk/ext-python](https://github.com/Felk/ext-python)).
- macOS arm64 build evidence: open [PR #59](https://github.com/Felk/dolphin/pull/59)
  adds a `Build Mac OS Dolphin` workflow on the `macos-15` (arm64) runner using
  `brew install qt` + `cmake .. -DCMAKE_POLICY_VERSION_MINIMUM=3.5
  -DENABLE_AUTOUPDATE=OFF` + `make`. On the author's fork that workflow
  **succeeded** on 2025-12-24 (`ci` branch) and **failed** on later `ci-test`
  runs in Feb 2026 (cause not investigated). So: compiles on arm64 mac at
  least once; **UNVERIFIED that the resulting app runs scripts correctly on
  arm64** -- nobody has reported doing so.
- Old branch `scripting` (2021) is stale; use `master`.

### Threading / timing semantics (from source)

- `FrameAdvance` is emitted in `Core::OnFrameBegin` on the CPU thread
  ([Core.cpp](https://raw.githubusercontent.com/Felk/dolphin/master/Source/Core/Core/Core.cpp)).
  All events assert `Core::IsCPUThread()` ([API/Events.h](https://raw.githubusercontent.com/Felk/dolphin/master/Source/Core/Core/API/Events.h)),
  so memory reads inside the callback see a consistent frame boundary and the
  emulator is effectively paused while your callback runs.
- Controller overrides set via `set_gc_buttons` hold "for the current frame"
  and are cleared on the next `FrameAdvance` once used
  ([API/Controller.cpp](https://raw.githubusercontent.com/Felk/dolphin/master/Source/Core/Core/API/Controller.cpp),
  [controller.pyi](https://raw.githubusercontent.com/Felk/dolphin/master/python-stubs/dolphin/controller.pyi)).
  Re-issue inputs every frame from the frameadvance callback.
- `CodeBreakpoint{addr}` is emitted from `BreakPoints::GetRegularBreakpoint`
  ([BreakPoints.cpp](https://raw.githubusercontent.com/Felk/dolphin/master/Source/Core/Core/PowerPC/BreakPoints.cpp)),
  which is called by `PowerPCManager::CheckBreakPoints()` (runtime, PC of a
  breakpointed instruction) **and** by `JitBase::CanMergeNextInstructions`
  when debugging is enabled (compile-time query per instruction)
  ([JitBase.cpp](https://raw.githubusercontent.com/Felk/dolphin/master/Source/Core/Core/PowerPC/JitCommon/JitBase.cpp)).
  Consequences: the callback fires *before* the enabled/condition check, and
  may fire spuriously when the JIT compiles a block. Filter by `addr` in your
  callback and, if it proves noisy, verify against `registers.read_gpr` / an
  expected PC. **UNVERIFIED** how noisy this is in practice.
- If the breakpoint has `break_on_hit`, `CheckAndHandleBreakPoints` calls
  `CPU::Break()` and emulation pauses; for a non-pausing tracepoint add the
  breakpoint with "break on hit" off (log-only) in the debugger UI.
- There is **no Python API to add a code or memory breakpoint**. Neither
  `memory.pyi` nor any module exposes `BreakPoints::Add`/`MemChecks::Add`
  (grep of `Source/Core/Scripting`). Add them via the Qt debugger
  (`--debugger`, Breakpoints panel). Dolphin's Breakpoints panel can load
  `[BreakPoints]` / `[MemoryChecks]` sections from
  `User/GameSettings/GALE01.ini` via its Load button (format:
  `$80xxxxxx flags`, see `BreakPoints::AddFromStrings`); this is a manual GUI
  action, not automatic at boot ([BreakpointWidget.cpp](https://raw.githubusercontent.com/Felk/dolphin/master/Source/Core/DolphinQt/Debugger/BreakpointWidget.cpp)).
  Adding a `dolphin.debug.add_breakpoint` function to the fork is a small C++
  change if you need it.
- `--script` and `--no-python-subinterpreters` are parsed in
  `UICommon/CommandLineParse.cpp` but only consumed by the **Qt** frontend
  (`DolphinQt/Main.cpp`); `DolphinNoGUI/MainNoGUI.cpp` has no scripting hooks.
  So the harness must run the Qt app (a window will be open; use
  `-v Null` for the video backend if rendering is unneeded -- **UNVERIFIED**
  that `frameadvance` still fires with the Null backend; `framedrawn` clearly
  won't).
- Only one listener per event; re-registering replaces it.
- Numpy/scipy in scripts require `--no-python-subinterpreters`.

## Exact API names (Felk fork, `master` / `scripting-preview4`)

Corrections to the names in the task prompt: all five guessed names are
correct on `master`. Note `on_codebreakpoint` did **not** exist in
`scripting-preview3` (only `on_frameadvance`, `on_memorybreakpoint`,
`system_reset`); it was added between preview3 and preview4. `emulation.reset`
replaced `event.system_reset` (2025-03-20).

```python
from dolphin import event, memory, controller, savestate, registers, gui, emulation
# NOTE: `emulation` is not re-exported in python-stubs/dolphin/__init__.pyi but
# the module exists on master (issue #58 was a stale preview3 binary).

# --- events (python-stubs/dolphin/event.pyi) ---
event.on_frameadvance(callback: Callable[[], None] | None) -> None
await event.frameadvance() -> None
event.on_codebreakpoint(callback: Callable[[addr: int], None] | None) -> None
await event.codebreakpoint() -> int                     # addr
event.on_memorybreakpoint(callback: Callable[[is_write: bool, addr: int, value: int], None] | None) -> None
await event.memorybreakpoint() -> tuple[bool, int, int]
event.on_framedrawn(callback: Callable[[width: int, height: int, data: bytes], None] | None) -> None
await event.framedrawn() -> tuple[int, int, bytes]       # perf cost; not needed for oracle

# --- memory (memory.pyi) --- addresses are guest virtual (0x80xxxxxx)
memory.read_u8/read_u16/read_u32/read_u64(addr) -> int
memory.read_s8/read_s16/read_s32/read_s64(addr) -> int
memory.read_f32/read_f64(addr) -> float
memory.write_u8/u16/u32/u64/s8/s16/s32/s64(addr, value) -> None
memory.write_f32/write_f64(addr, value) -> None
# No read_bytes / bulk read. Loop over read_u32 for struct dumps (or add a C++ function).

# --- controller (controller.pyi) ---
controller.get_gc_buttons(controller_id) -> GCInputs
controller.set_gc_buttons(controller_id, inputs: GCInputs) -> None
# GCInputs keys: A B X Y Z Start Up Down Left Right L R (bool);
#   StickX StickY CStickX CStickY (float -1..1); TriggerLeft TriggerRight (float 0..1)
# Omitted keys keep their state; override holds for the current frame only.

# --- savestate (savestate.pyi) ---
savestate.save_to_slot(slot) / savestate.load_from_slot(slot)      # 0..99
savestate.save_to_file(filename) / savestate.load_from_file(filename)
savestate.save_to_bytes() -> bytes / savestate.load_from_bytes(state_bytes)
# Implemented via State::Save/Load/SaveAs/LoadAs/SaveToBuffer/LoadFromBuffer.
# UNVERIFIED whether load completes synchronously before the next frameadvance;
# Dolphin's State::LoadAs is normally deferred to the CPU thread, so await one
# frameadvance after loading before trusting memory reads.

# --- registers (registers.pyi) --- useful inside on_codebreakpoint
registers.read_gpr(index) -> int ; registers.read_fpr(index) -> float
registers.write_gpr(index, value) ; registers.write_fpr(index, value)

# --- emulation (emulation.pyi) ---
emulation.pause() ; emulation.resume() ; emulation.reset()

# --- gui (gui.pyi) --- optional overlay
gui.add_osd_message(msg, duration_ms=2000, color=0xFFFFFF30) ; gui.draw_text(pos, color, text) ; ...
```

Minimal oracle skeleton:

```python
from dolphin import event, memory, controller, savestate
import json, sys

savestate.load_from_file("/path/to/melee_frame0.sav")
await event.frameadvance()          # let the load settle (see UNVERIFIED note)

INPUTS = json.load(open("/path/to/inputs.json"))   # per-frame GCInputs dicts
out = open("/path/to/dump.jsonl", "w")

def on_bp(addr: int):
    # fires for breakpoints added in the Dolphin debugger UI; filter by addr
    pass
event.on_codebreakpoint(on_bp)

frame = 0
while frame < len(INPUTS):
    controller.set_gc_buttons(0, INPUTS[frame])      # must be re-issued every frame
    await event.frameadvance()
    rec = {"frame": frame, "p1_x": memory.read_f32(0x80453090)}  # example addresses
    out.write(json.dumps(rec) + "\n")
    frame += 1
out.close()
```

Launch: `Dolphin.app/Contents/MacOS/Dolphin --script harness.py -e GALE01.iso
[--debugger] [-v Null]` (Felk Readme; mainline CLI reference:
[Readme.md](https://raw.githubusercontent.com/dolphin-emu/dolphin/master/Readme.md)).
Script `print` output goes to the Dolphin log under the `Scripting` type
(View -> Show Log Configuration, verbosity Error or lower); write to files for
the actual dump.

## Build steps: Felk fork on macOS arm64

Derived from PR #59's `build-mac.yml` (which passed on `macos-15` arm64 in
Dec 2025) plus mainline's macOS build notes. **UNVERIFIED end-to-end on a
local machine**; expect to iterate.

```sh
# prerequisites
xcode-select --install            # or full Xcode; macOS >= 10.15 required by mainline docs
brew install cmake ninja qt python@3.12   # PR #59 references python312 libs; any 3.x with Development headers should satisfy find_package

# source
git clone https://github.com/Felk/dolphin.git ~/src/felk-dolphin
cd ~/src/felk-dolphin
git checkout master               # == scripting-preview4 (a22afcb)
git submodule update --init --recursive

# configure (flags copied from PR #59; ENABLE_AUTOUPDATE=OFF avoids the updater;
# CMAKE_POLICY_VERSION_MINIMUM=3.5 is needed with recent CMake for old submodules)
mkdir -p build && cd build
cmake .. -G Ninja \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_POLICY_VERSION_MINIMUM=3.5 \
  -DENABLE_AUTOUPDATE=OFF \
  -DPython3_ROOT_DIR="$(brew --prefix python@3.12)"   # pin which Python find_package picks
ninja                              # PR #59 runs `make` twice with continue-on-error; if the
                                   # first pass fails on a bundling step, just re-run.

# result
ls Binaries/Dolphin.app
```

Notes:

- PR #57 (merged) fixed macOS bundling: `fixup_bundle` used to fail trying to
  copy the Python interpreter into the .app. The fix marks python as
  `IGNORE_ITEM`, so the app links the Homebrew `libpython` at runtime -- keep
  that Python installed, and the stdlib it ships with is what scripts import.
- If Python symbols fail to link, try PR #59's tree
  (`unexploredtest/dolphin` branch `ci`, sha `c583ca1`) which also changes
  `Source/Core/Scripting/CMakeLists.txt`; its non-Windows path is unchanged
  (`find_package`), so this is unlikely to matter on mac.
- If Homebrew Qt is Qt 6 and the fork's DolphinQt expects Qt 6, fine (the
  fork is ~Dolphin 2412, which is Qt 6). `brew install qt` in the workflow is
  Qt 6.
- Checking that scripting works: `Binaries/Dolphin.app/Contents/MacOS/Dolphin
  --script /path/to/framecounter.py`, then View -> Scripting should list it.
- Alternative if the local build is painful: fork the repo, push PR #59's
  `.github/workflows/build-mac.yml` to your fork, and download the
  `DolphinMac-arm64` artifact from Actions.

## Mainline Dolphin GDB stub (fallback for breakpoints)

Source: [GDBStub.cpp](https://github.com/dolphin-emu/dolphin/blob/master/Source/Core/Core/PowerPC/GDBStub.cpp),
[MainSettings.cpp](https://raw.githubusercontent.com/dolphin-emu/dolphin/master/Source/Core/Core/Config/MainSettings.cpp),
[Core.cpp](https://raw.githubusercontent.com/dolphin-emu/dolphin/master/Source/Core/Core/Core.cpp).

- Enable via `Dolphin.ini` `[General]`: `GDBPort = 9090` (TCP) or
  `GDBSocket = /path/to/sock` (Unix socket, non-Windows). On the CLI:
  `-C Dolphin.General.GDBPort=9090`. Disabled if RetroAchievements hardcore
  mode is on.
- Behaviour: at CPU-thread start the stub `listen()`s and **blocks in
  `accept()`** until a debugger connects, then starts paused
  (`CPUSetInitialExecutionState(system, true)`).
- Supported packets: `?`, `g/G`, `p/P`, `m/M`, `s`, `c`, `k`, `H`, `T`,
  `q*` (including `qSupported` -> `swbreak+;hwbreak+`), `Z0/z0` (sw bp),
  `Z1/z1` (hw bp), `Z2/Z3/Z4` (write/read/access watchpoints). **No `vCont`**.
- Limitations for frame stepping: `s` steps one instruction. No frame-advance
  packet. To step a frame, set a breakpoint on a once-per-frame address and
  `c`. No input injection. Memory reads via `m` are fine (any address).
- Clients: `gdb-multiarch` / `powerpc-eabi-gdb`, or a pure-Python RSP client.
  Forum thread on pitfalls: <https://forums.dolphin-emu.org/Thread-gdbstub-not-working>.
- Combine with `-s state.sav` on the CLI to load a savestate at boot, and
  `-e GALE01.iso -u <userdir>`. NoGUI: `dolphin-emu-nogui --platform headless`
  builds on macOS (`#ifdef __APPLE__` platform path in
  [MainNoGUI.cpp](https://raw.githubusercontent.com/dolphin-emu/dolphin/master/Source/Core/DolphinNoGUI/MainNoGUI.cpp);
  `ENABLE_NOGUI` defaults ON in the top-level CMakeLists).

## libmelee + Slippi (not recommended for this oracle)

Sources: [altf4/libmelee](https://github.com/altf4/libmelee) (archived
2026-01-25, points to fork), [vladfi1/libmelee](https://github.com/vladfi1/libmelee)
(pushed 2026-08-17), [docs](https://libmelee.readthedocs.io/en/latest/console.html),
[console.py](https://raw.githubusercontent.com/vladfi1/libmelee/master/melee/console.py).

- State: `Console.step()` -> `GameState` (frame, stage, menu state, per-player
  position, percent, character, action state, action frame, shield, velocity,
  on-ground, ECB...). Comes from Slippi's spectator/enet stream on UDP 51441
  (`SlippstreamClient`), i.e. only what Slippi serializes.
- Input: Dolphin's named-pipe input backend (`User/Pipes/slippibot<N>`,
  `mkfifo`; text protocol `PRESS A`, `SET MAIN 0.5 0.5`, ...). Pipes compile
  on any `UNIX` platform in mainline ([InputCommon/CMakeLists.txt](https://raw.githubusercontent.com/dolphin-emu/dolphin/master/Source/Core/InputCommon/CMakeLists.txt)),
  so the pipe mechanism itself is reusable with mainline Dolphin on macOS
  even without libmelee.
- Headless: `Console.run(platform="headless")` -> `--platform headless`,
  "Only applies to mainline dolphin"; Ishiiruka bakes the platform in at
  build time (`ENABLE_HEADLESS`, with a known build failure
  [Ishiiruka #209](https://github.com/project-slippi/Ishiiruka/issues/209)).
- macOS arm64: Ishiiruka `FM-Slippi-3.6.4-Mac.dmg` (2026-06-15) is x86_64 under
  Rosetta; `Mainline-Slippi-4.0.0-mainline-beta.19-Mac.dmg` (2026-06-15) exists
  but libmelee says it segfaults on macOS. Fast-forward needs vladfi1's
  `slippi-Ishiiruka` fork (Linux AppImage only).
- Retail Melee: not supported as-is; setup requires Slippi Dolphin plus its
  Gecko code set (`GALE01r2.ini`), and the state stream is generated by
  Slippi's recording codes/EXI device.

## Open items / could not verify

1. Felk fork actually running (not just compiling) on macOS arm64.
2. Whether `frameadvance` fires with `-v Null`; whether savestate load is
   synchronous with respect to the next `frameadvance`.
3. Noise level of `on_codebreakpoint` given it is emitted from
   `GetRegularBreakpoint` (also hit by the JIT at compile time).
4. Whether Homebrew Python 3.13 works with the fork's embedding code (PR #59
   references 3.12 on Windows).
5. Cause of the Feb 2026 failures of the mac CI workflow on the PR author's
   fork.
