#!/usr/bin/env bash
# Build the coverage Dolphin used by harness/retail_coverage.py, beside the
# recording build and without touching it.
#
#   tools/build-coverage-dolphin.sh [dolphin-scripting-root]
#
# 1. Clones (APFS copy-on-write) the fork's src/ into src-coverage/ once, with
#    its working-tree patches (0002, 0003), and applies
#    docs/patches/0004-scripting-branch-watch.patch there: Python access to
#    Dolphin's Branch Watch (emulation.branch_watch_start/stop/clear/save).
# 2. Configures build-coverage/ with the recording build's compiler settings
#    (no Qt, no tests) and builds dolphin-nogui (~100 MB).
# 3. Wraps it in DolphinCoverage.app with Sys linked from the GUI build, as
#    build-headless-dolphin.sh does.
#
# Prints the executable (retail_coverage.py's default; MELEE_COVERAGE_DOLPHIN
# overrides it). Runs need debug mode, which retail_coverage.py passes.
set -euo pipefail

root="${1:-$HOME/Projects/dolphin-scripting}"
repo="$(cd "$(dirname "$0")/.." && pwd)"
patch="$repo/docs/patches/0004-scripting-branch-watch.patch"
src="$root/src-coverage"
build="$root/build-coverage"
gui_sys="$root/build/Binaries/Dolphin.app/Contents/Resources/Sys"
bundle="$build/Binaries/DolphinCoverage.app"

[[ -d "$gui_sys" ]] || { echo "missing $gui_sys: build the GUI app first (docs/DOLPHIN_BUILD.md)" >&2; exit 1; }
if [[ ! -d "$src" ]]; then
  cp -Rc "$root/src" "$src" 2>/dev/null || true   # the fsmonitor sockets do not copy
fi

if git -C "$src" apply --check "$patch" 2>/dev/null; then
  git -C "$src" apply "$patch"
  echo "applied $(basename "$patch")"
elif git -C "$src" apply --reverse --check "$patch" 2>/dev/null; then
  echo "$(basename "$patch") already applied"
else
  echo "$(basename "$patch") neither applies nor is applied; reconcile $src by hand" >&2
  exit 1
fi

if [[ ! -f "$build/build.ninja" ]]; then
  sdk="/Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk"
  cmake -G Ninja -S "$src" -B "$build" -DCMAKE_POLICY_VERSION_MINIMUM=3.5 -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_C_COMPILER=/usr/bin/clang -DCMAKE_CXX_COMPILER=/usr/bin/clang++ "-DCMAKE_CXX_FLAGS=-DFMT_CONSTEVAL=" \
    -DCMAKE_OSX_SYSROOT="$sdk" -DCMAKE_OSX_DEPLOYMENT_TARGET=10.15.0 \
    -DENABLE_QT=OFF -DENABLE_NOGUI=ON -DENABLE_TESTS=OFF -DSKIP_POSTPROCESS_BUNDLE=ON >/dev/null
fi
cmake --build "$build" --target dolphin-nogui -j"$(sysctl -n hw.ncpu)" 2>&1 | grep -v 'ld: warning' | tail -3

mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources"
# Replace by rename: running coverage Dolphins keep their own copy.
cp -f "$build/Binaries/dolphin-emu-nogui" "$bundle/Contents/MacOS/dolphin-emu-nogui.new"
mv -f "$bundle/Contents/MacOS/dolphin-emu-nogui.new" "$bundle/Contents/MacOS/dolphin-emu-nogui"
ln -sfn "$gui_sys" "$bundle/Contents/Resources/Sys"
cat > "$bundle/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key><string>dolphin-emu-nogui</string>
  <key>CFBundleIdentifier</key><string>org.dolphin-emu.dolphin.coverage</string>
  <key>CFBundleName</key><string>DolphinCoverage</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>LSUIElement</key><true/>
</dict>
</plist>
PLIST

echo "$bundle/Contents/MacOS/dolphin-emu-nogui"
