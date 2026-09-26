# Building Felk's Dolphin scripting fork on macOS arm64

Status: **built and smoke-tested on 2026-09-08.** The embedded Python starts,
`--script` runs a script at app launch, all `dolphin.*` modules import, and
`event.on_codebreakpoint` exists. Not yet verified: `frameadvance` firing with
a game booted (no GALE01 image was on the machine; see "Next step" at the end).

Everything lives under `/Users/nikhilunni/Projects/dolphin-scripting/`:

| Path | What |
|---|---|
| `src/` | Felk/dolphin `master` @ `a22afcb` (== tag `scripting-preview4`), shallow clone, 24 submodules |
| `build/` | Ninja build dir (~900 MB) |
| `build/Binaries/Dolphin.app/Contents/MacOS/Dolphin` | **the binary** (arm64 Mach-O, `--version` prints `Dolphin a22afcb`) |
| `build/Binaries/dolphin-emu-nogui`, `dolphin-tool` | also built; nogui has no scripting hooks |
| `src/python-stubs/dolphin/*.pyi` | API docs / type stubs (point your editor at this dir) |
| `patches/0001-dolphinqt-link-qt6-guiprivate.patch` | the one source patch needed (Qt 6.11) |
| `scripts/smoke_test.py` | smoke test used below |
| `user/` | isolated Dolphin user dir used for testing (`-u`) |
| `clone.log`, `configure.log`, `build.log` | full logs of the run |

## Machine / toolchain that this was verified with

- macOS 26.2 (arm64, 12 cores, 64 GB). Xcode 26.6, Apple clang 21.0.0.
- Homebrew: cmake 4.2.1, ninja 1.13.2, **qt 6.11.2** (installed for this build),
  python@3.12 3.12.9, ffmpeg 7.1.1, sdl2 2.32.6.
- Also present and relevant: Homebrew LLVM 22 is first on `PATH` as `clang`
  (so the compiler must be pinned to `/usr/bin/clang`), and Homebrew
  python@3.14 is the default `python3` (so Python must be pinned to 3.12).

## Exact commands that worked

```sh
# 0. prerequisites (only qt was missing here; ~1 min from bottles)
brew install cmake ninja qt python@3.12

# 1. source: shallow clone with shallow submodules works fine with this repo
mkdir -p ~/Projects/dolphin-scripting && cd ~/Projects/dolphin-scripting
git clone --depth 1 --branch master --recursive --shallow-submodules -j 8 \
    https://github.com/Felk/dolphin.git src            # 1 min 53 s
git -C src describe --tags                              # -> scripting-preview4

# 2. one-line-ish patch for Homebrew Qt 6.11 (see "Deviations" #3)
git -C src apply ../patches/0001-dolphinqt-link-qt6-guiprivate.patch

# 3. configure (19-25 s)
mkdir -p build && cd build
P="$(brew --prefix python@3.12)/Frameworks/Python.framework/Versions/3.12"
cmake ../src -G Ninja \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_POLICY_VERSION_MINIMUM=3.5 \
  -DENABLE_AUTOUPDATE=OFF \
  -DENABLE_LLVM=OFF \
  -DSKIP_POSTPROCESS_BUNDLE=ON \
  -DCMAKE_C_COMPILER=/usr/bin/clang \
  -DCMAKE_CXX_COMPILER=/usr/bin/clang++ \
  -DCMAKE_OSX_SYSROOT=/Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk \
  -DCMAKE_CXX_FLAGS="-DFMT_CONSTEVAL=" \
  -DCMAKE_PREFIX_PATH="$(brew --prefix qt)" \
  -DPython3_FIND_STRATEGY=LOCATION \
  -DPython3_ROOT_DIR="$P" \
  -DPython3_EXECUTABLE="$P/bin/python3.12" \
  -DPython3_INCLUDE_DIR="$P/include/python3.12" \
  -DPython3_LIBRARY="$P/lib/libpython3.12.dylib"

# 4. build
ninja -j 12          # 3779 steps; ~5 min wall on this machine

# 5. result
ls Binaries/Dolphin.app/Contents/MacOS/Dolphin
Binaries/Dolphin.app/Contents/MacOS/Dolphin --version     # Dolphin a22afcb
```

Check the configure log for these two lines before building; if either shows a
different version the pin did not take:

```
-- Found Python3: .../python@3.12/.../bin/python3.12 (found version "3.12.9") found components: Interpreter
-- Found Python3: .../python@3.12/.../include/python3.12 (found version "3.12.9") found components: Development Development.Module Development.Embed
-- Found Qt version 6.11.2
```

### Build time

| Phase | Wall time |
|---|---|
| `brew install qt` | ~1 min (parallel with clone) |
| clone + submodules | 1 min 53 s |
| configure | 19-25 s |
| ninja, clean, up to the first failure at step 3721/3779 | 4 min 57 s |
| fix + finish (recompile MainWindow, link, bundle) | ~2 min |
| **Total including the debugging detours** | **~17 min** (07:23 to 07:40) |

A clean rebuild with the commands above should be ~6 min.

## Deviations from the DOLPHIN.md recipe

The recipe's flags (`-DCMAKE_POLICY_VERSION_MINIMUM=3.5 -DENABLE_AUTOUPDATE=OFF`,
Homebrew Qt, `find_package(Python3)` with Homebrew python@3.12) were all
necessary and correct. Five extra things were needed on this machine:

1. **Pin Apple clang.** `clang` on `PATH` is Homebrew LLVM 22. Without
   `-DCMAKE_C_COMPILER=/usr/bin/clang -DCMAKE_CXX_COMPILER=/usr/bin/clang++`
   CMake would pick the wrong toolchain. Related: `-DENABLE_LLVM=OFF`, because
   the first configure found Homebrew LLVM for the debugger's host
   disassembler and leaked `-I/opt/homebrew/Cellar/llvm/22.1.0/include` into
   every compile. It is only used for disassembling host (arm64) JIT code;
   not needed for the oracle.

2. **Really pin Python 3.12.** `-DPython3_ROOT_DIR` alone was *not* enough:
   the first configure found the 3.12 *interpreter* but 3.14 *Development*
   headers/libs (`/opt/homebrew/opt/python@3.14/...`), because the Scripting
   `find_package(Python3 REQUIRED COMPONENTS Development)` is a second call
   with different components. Fix: `-DPython3_FIND_STRATEGY=LOCATION` plus
   explicit `Python3_EXECUTABLE` / `Python3_INCLUDE_DIR` / `Python3_LIBRARY`
   pointing into the python@3.12 framework. Verified in `CMakeCache.txt`
   (`_Python3_LIBRARY_RELEASE` = `.../3.12/lib/libpython3.12.dylib`) and with
   `otool -L` on the binary.

3. **fmt 10.2.1 vs Apple clang 21 (build break #1).** Bundled
   `Externals/fmt/fmt/include/fmt/chrono.h:1159` fails with
   `call to consteval function 'fmt::basic_format_string<...>' is not a constant expression`
   in every TU that includes `<fmt/chrono.h>` (first seen in
   `Common/Logging/LogManager.cpp`). Dolphin already applies the workaround
   `-DFMT_CONSTEVAL=` for old clang on FreeBSD (`CMakeLists.txt:410`); passing
   it via `-DCMAKE_CXX_FLAGS="-DFMT_CONSTEVAL="` fixes it without touching
   source. (Alternative not tried: Homebrew fmt 12 via `pkg-config`; the
   Dolphin-2412-era code likely does not compile against fmt 12.)

4. **Homebrew Qt 6.11 private headers (build break #2, the only source patch).**
   `DolphinQt/MainWindow.cpp:34` includes `<qpa/qplatformnativeinterface.h>`
   (a Qt private header). Dolphin adds `${Qt6Gui_PRIVATE_INCLUDE_DIRS}` to the
   include path, but with Homebrew's framework build of Qt 6.11 that variable
   is empty, and `Qt6::GuiPrivate` is not defined by `Qt6GuiConfig.cmake`
   anymore; it is its own component (`lib/cmake/Qt6GuiPrivate`). Patch
   (`patches/0001-dolphinqt-link-qt6-guiprivate.patch`), 2 lines in
   `Source/Core/DolphinQt/CMakeLists.txt`:

   ```diff
   -find_package(Qt6 REQUIRED COMPONENTS Core Gui Widgets Svg)
   +find_package(Qt6 REQUIRED COMPONENTS Core Gui Widgets Svg GuiPrivate)
   ...
      Qt6::Widgets
   +  Qt6::GuiPrivate
   ```

   Without the `find_package` half you get
   `Target "dolphin-emu" links to: Qt6::GuiPrivate but the target was not found`.
   (Many `QCheckBox::stateChanged is deprecated` warnings from Qt 6.9+ are
   harmless.)

5. **Skip bundle fixup (build break #3).** The post-link step
   `CMake/DolphinPostprocessBundle.cmake` (`fixup_bundle`, copies Homebrew
   dylibs into the .app) failed:
   `otool -l failed ... can't open file: @rpath/libjxl_cms.0.11.dylib` (an
   ffmpeg -> libjxl dependency; BundleUtilities cannot resolve `@rpath` and
   `extra_dirs` lacks `/opt/homebrew/lib`). Fix: `-DSKIP_POSTPROCESS_BUNDLE=ON`
   (existing upstream option). Consequence: **the .app is not relocatable**;
   it dynamically links these Homebrew packages at runtime:
   `ffmpeg libusb lz4 lzo python@3.12 qtbase sdl2 zstd`. A `brew upgrade` that
   bumps a major soname (ffmpeg 61 -> 62, python 3.12 removal, Qt 7) will
   break the binary until you re-run `ninja`. PR #57 already made this true
   for libpython (`IGNORE_ITEM Python`), so nothing is lost for our purpose.
   The alternative fix, untried, is adding `"/opt/homebrew/lib"` to
   `extra_dirs` in `DolphinPostprocessBundle.cmake` to get a standalone app.

Non-fatal noise you will see and can ignore:

- `xcodebuild: error: SDK "/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk" cannot be located`
  during configure. CMake's Darwin init for the ASM language asks `xcrun` for
  the default SDK, which on this machine is the CommandLineTools one, while
  `xcode-select` points at Xcode. Pinning `CMAKE_OSX_SYSROOT` to the Xcode SDK
  (done above) makes the actual compiles consistent; the message itself is
  harmless.
- `ld: warning: building for macOS-11.0, but linking with dylib ... built for newer version 14.0`
  for every Homebrew dylib (Dolphin sets `CMAKE_OSX_DEPLOYMENT_TARGET` low).
- Many `CMake Deprecation Warning ... Compatibility with CMake < 3.10` from
  submodules (this is what `CMAKE_POLICY_VERSION_MINIMUM=3.5` is for).

Not needed: PR #59's tree (`unexploredtest/dolphin` branch `ci`), running
`make` twice, or anything from `Externals/python` (Windows-only bundle).

## Running with `--script`

```sh
D=/Users/nikhilunni/Projects/dolphin-scripting
$D/build/Binaries/Dolphin.app/Contents/MacOS/Dolphin \
    -u $D/user \
    --script /abs/path/to/harness.py \
    -C Logger.Logs.Scripting=True -C Logger.Options.WriteToFile=True \
    -C Dolphin.Interface.UsePanicHandlers=False \
    -e /path/to/GALE01.iso            # optional; [-v Null] [--debugger] [-b]
```

Verified facts about the runtime (from `scripts/smoke_test.py`):

- The script is executed **immediately at app launch**, from the
  `ScriptingWidget` (`MainWindow` -> `ScriptsListModel::Add` constructs the
  backend synchronously). No game needs to be running for top-level code to
  execute; `frameadvance` obviously only fires once a game is booted.
- Embedded interpreter: `3.12.9 ... [Clang 15.0.0]`, `sys.prefix` =
  `/opt/homebrew/opt/python@3.12/Frameworks/Python.framework/Versions/3.12`,
  `sys.path` = the Homebrew 3.12 stdlib (`python312.zip`, `lib/python3.12`,
  `lib-dynload`). **No `site-packages`** is on the path (isolated config), so
  third-party modules need `sys.path.append(...)` in the script (and numpy
  additionally needs `--no-python-subinterpreters`).
- **`__file__` is not defined** in the script (it is run with `PyRun_File`
  into `__main__`). Use absolute paths. `os.getcwd()` is wherever you launched
  Dolphin from. `sys.executable` is the Dolphin binary.
- `import dolphin` works; `sys.modules` gets `dolphin, dolphin_controller,
  dolphin_emulation, dolphin_event, dolphin_gui, dolphin_memory,
  dolphin_registers, dolphin_savestate`.
- `event`: `on_frameadvance on_codebreakpoint on_memorybreakpoint on_framedrawn`
  and the awaitable `frameadvance codebreakpoint memorybreakpoint framedrawn`.
  So this build has `on_codebreakpoint` (preview4 API), as DOLPHIN.md expects.
- `memory`: `read_/write_{u8,u16,u32,u64,s8,s16,s32,s64,f32,f64}` **plus
  `add_memcheck` and `remove_memcheck`** -- a Python API for *memory*
  breakpoints does exist (DOLPHIN.md says none does; that is only true for
  *code* breakpoints, which still have no add API). Check
  `src/Source/Core/Scripting/Python/Modules/memorymodule.cpp` for the signature.
- `savestate`: `load_from_bytes load_from_file load_from_slot save_to_bytes save_to_file save_to_slot`.
  `registers`: `read_gpr read_fpr write_gpr write_fpr`. `emulation`: `pause resume reset`.
- `print()` goes to the Dolphin log, type `Scripting`, level NOTICE
  (`Script stdout: ...`); Python tracebacks go there at ERROR
  (`Script stderr: ...`). Nothing is written to the terminal. To get them into
  `<userdir>/Logs/dolphin.log`, either pass the two `-C Logger.*` flags above
  (verified) or write `<userdir>/Config/Logger.ini`:

  ```ini
  [Options]
  WriteToFile = True
  Verbosity = 4        ; 4 = INFO, shows "Initializing embedded python..." too
  [Logs]
  Scripting = True
  ```

  (One early run with only `-C` flags produced an empty `dolphin.log`; every
  later run, including one with the identical flags, logged fine. If the log
  is empty, use the ini.) For the actual trace dump, write files from the
  script as DOLPHIN.md recommends.
- `-C Dolphin.Interface.UsePanicHandlers=False` turns modal PanicAlert dialogs
  into log lines, which matters for an unattended harness (a Python init
  failure would otherwise block in a dialog).
- The Qt window opens. The stock no-GUI frontend cannot script; the patched
  headless build (`tools/build-headless-dolphin.sh`, `docs/DOLPHIN_RUN.md`)
  can and is now the default for recordings. Both handle SIGTERM gracefully
  (`kill <pid>`).

`--help` confirms the flags: `--script=<file>`, `--no-python-subinterpreters`,
`-u`, `-e`, `-s <savestate>`, `-C`, `-d/--debugger`, `-b/--batch`, `-v`.

## Next step (not done): verify `frameadvance` with a game

No `GALE01` image exists on this machine (searched `~/Projects`, `~/Downloads`,
`~/Documents`, Dolphin/Slippi app-support dirs). Once one is in
`harness/roms/` per `docs/ISO.md`:

```sh
D=/Users/nikhilunni/Projects/dolphin-scripting
rm -f $D/scripts/smoke_test.out
$D/build/Binaries/Dolphin.app/Contents/MacOS/Dolphin -u $D/user \
    --script $D/scripts/smoke_test.py \
    -C Logger.Logs.Scripting=True -C Logger.Options.WriteToFile=True \
    -e harness/roms/GALE01.iso
# expect lines "frameadvance=1..3" in $D/scripts/smoke_test.out and
# "Script stdout: smoke_test: frameadvance #N" in $D/user/Logs/dolphin.log
```

Then repeat with `-v Null` to settle DOLPHIN.md open item 2, and try a
`savestate.load_from_file` + `await event.frameadvance()` round trip.

## Rebuilding after changes

```sh
cd /Users/nikhilunni/Projects/dolphin-scripting/build && ninja -j 12
```

The cache already holds every flag above; re-running plain `ninja` (or
`cmake .` then `ninja`) is enough. If you `git pull` in `src/`, re-apply
`patches/0001-...` if upstream has not fixed the Qt private-header include.
Adding a `dolphin.debug.add_breakpoint`-style function (DOLPHIN.md suggests
this for code breakpoints) is a change to
`src/Source/Core/Scripting/Python/Modules/` plus a stub in `python-stubs/`,
followed by the same `ninja`.
