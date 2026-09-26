#!/usr/bin/env bash
# Build the headless (no-GUI) Dolphin used for every oracle recording.
#
#   tools/build-headless-dolphin.sh [dolphin-scripting-root]
#
# 1. Applies docs/patches/0003-nogui-scripting-backend.patch to the scripting
#    fork if it is not applied yet (the stock no-GUI frontend parses --script
#    but never starts a scripting backend).
# 2. Builds the dolphin-nogui target.
# 3. Wraps the binary in DolphinHeadless.app. On macOS Dolphin finds its Sys
#    directory through the app bundle; a bare executable finds none, so it
#    loses Sys/GameSettings/GALE01.ini and the DSP coefficients (savestate
#    loads then fail silently in the AX ucode). The bundle links Sys from the
#    GUI build, so both frontends run with identical data.
#
# Prints the executable path to use as DOLPHIN_BIN (docs/DOLPHIN_RUN.md).
set -euo pipefail

root="${1:-$HOME/Projects/dolphin-scripting}"
repo="$(cd "$(dirname "$0")/.." && pwd)"
patch="$repo/docs/patches/0003-nogui-scripting-backend.patch"
src="$root/src"
build="$root/build"
gui_sys="$build/Binaries/Dolphin.app/Contents/Resources/Sys"
bundle="$build/Binaries/DolphinHeadless.app"

[[ -d "$gui_sys" ]] || { echo "missing $gui_sys: build the GUI app first (docs/DOLPHIN_BUILD.md)" >&2; exit 1; }

if git -C "$src" apply --check "$patch" 2>/dev/null; then
  git -C "$src" apply "$patch"
  echo "applied $(basename "$patch")"
elif git -C "$src" apply --reverse --check "$patch" 2>/dev/null; then
  echo "$(basename "$patch") already applied"
else
  echo "$(basename "$patch") neither applies nor is applied; reconcile $src by hand" >&2
  exit 1
fi

cmake --build "$build" --target dolphin-nogui -j"$(sysctl -n hw.ncpu)" 2>&1 | grep -v 'ld: warning' | tail -3

mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources"
cp -f "$build/Binaries/dolphin-emu-nogui" "$bundle/Contents/MacOS/dolphin-emu-nogui"
ln -sfn "$gui_sys" "$bundle/Contents/Resources/Sys"
cat > "$bundle/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key><string>dolphin-emu-nogui</string>
  <key>CFBundleIdentifier</key><string>org.dolphin-emu.dolphin.headless</string>
  <key>CFBundleName</key><string>DolphinHeadless</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>LSUIElement</key><true/>
</dict>
</plist>
PLIST

echo "$bundle/Contents/MacOS/dolphin-emu-nogui"
