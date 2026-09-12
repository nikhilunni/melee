#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd -- "$(dirname -- "$0")" && pwd)
mkdir -p /tmp/melee-fmuls-probe
CPU=${1:-0}
export FMULS_PROBE_OUT=${FMULS_PROBE_OUT:-/tmp/melee-fmuls-probe/results-cpu$CPU}
export FMULS_SQSUM_BITS=${FMULS_SQSUM_BITS:-0x426e26ee}
mkdir -p "$FMULS_PROBE_OUT/user/Config"
cat > "$FMULS_PROBE_OUT/user/Config/Dolphin.ini" <<INI
[Interface]
UsePanicHandlers = False
[Analytics]
PermissionAsked = True
Enabled = False
[Core]
CPUCore = $CPU
CPUThread = False
GFXBackend = Null
INI
python3 "$ROOT/mkdol.py"
exec "${DOLPHIN_BIN:-$HOME/Projects/dolphin-scripting/build/Binaries/Dolphin.app/Contents/MacOS/Dolphin}" -u "$FMULS_PROBE_OUT/user" -v Null --script "$ROOT/capture.py" -C Dolphin.Core.CPUCore="$CPU" -C Dolphin.Core.CPUThread=False -e /tmp/melee-fmuls-probe/probe.dol
