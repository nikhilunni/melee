#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ $(uname -s) != Darwin ]]; then echo 'The Swift shell requires macOS.' >&2; exit 1; fi
cargo build --release -p melee-platform
bundle="$PWD/target/macos/Melee.app"
mkdir -p "$bundle/Contents/MacOS"
cp apps/macos/Info.plist "$bundle/Contents/Info.plist"
mkdir -p target/swift-module-cache target/clang-module-cache
xcrun swiftc -O -module-cache-path target/swift-module-cache -Xcc -fmodules-cache-path=target/clang-module-cache -target "$(uname -m)-apple-macosx14.0" \
    -import-objc-header crates/melee-platform/include/melee_platform.h \
    apps/macos/main.swift -L target/release -lmelee_platform \
    -framework AppKit -framework QuartzCore -framework Metal -framework IOKit \
    -framework Security -o "$bundle/Contents/MacOS/Melee"
printf '%s\n' "$bundle"
