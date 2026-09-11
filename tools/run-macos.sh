#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
tools/build-macos.sh
exec target/macos/Melee.app/Contents/MacOS/Melee "$PWD/harness/roms/files" "$@"
