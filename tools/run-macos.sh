#!/usr/bin/env bash
# Build and launch the macOS app. It asks for the disc image itself (drop it
# on the window or use File > Open Disc); no paths are passed.
# Development smoke test (docs/APP.md):
#   MELEE_APP_AUTOSTART=/path/to/GALE01.iso:Fox:Marth:FinalDestination MELEE_APP_SMOKE_SECONDS=5 tools/run-macos.sh
set -euo pipefail
cd "$(dirname "$0")/.."
tools/build-macos.sh
exec target/macos/Melee.app/Contents/MacOS/Melee
