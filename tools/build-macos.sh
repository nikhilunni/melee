#!/usr/bin/env bash
# Build target/macos/Melee.app: libmelee (crates/melee-platform, a static
# library) linked into the Swift/AppKit host in apps/macos.
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ $(uname -s) != Darwin ]]; then echo 'The macOS app requires macOS.' >&2; exit 1; fi
cargo build --release -p melee-platform
bundle="$PWD/target/macos/Melee.app"
mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources"
cp apps/macos/Info.plist "$bundle/Contents/Info.plist"
# The menu fonts (SIL OFL 1.1) and their licence; registered at launch.
rm -rf "$bundle/Contents/Resources/Fonts"
mkdir -p "$bundle/Contents/Resources/Fonts"
cp assets/fonts/*.ttf assets/fonts/OFL.txt "$bundle/Contents/Resources/Fonts/"
mkdir -p target/swift-module-cache target/clang-module-cache
xcrun swiftc -O -module-name Melee \
    -module-cache-path target/swift-module-cache -Xcc -fmodules-cache-path=target/clang-module-cache \
    -target "$(uname -m)-apple-macosx14.0" \
    -import-objc-header crates/melee-platform/include/melee_platform.h \
    apps/macos/*.swift -L target/release -lmelee_platform \
    -framework AppKit -framework SwiftUI -framework QuartzCore -framework Metal -framework IOKit \
    -framework Security -o "$bundle/Contents/MacOS/Melee"
printf '%s\n' "$bundle"
