#!/usr/bin/env bash
# Build the probe .dol, boot it in the Dolphin scripting fork with probe.py,
# wait for the results, print their paths.
#
#   harness/gekko_probe/run.sh [cpu_core]     cpu_core: 0=interpreter (default), 4=JITARM64, 5=cached interp
#
# Env: DOLPHIN (binary), GEKKO_PROBE_OUT (results dir), TIMEOUT (seconds).
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$HERE/../.." && pwd)"
CPU_CORE="${1:-0}"
DOLPHIN="${DOLPHIN:-/Users/nikhilunni/Projects/dolphin-scripting/build/Binaries/Dolphin.app/Contents/MacOS/Dolphin}"
OUT="${GEKKO_PROBE_OUT:-$REPO/harness/traces}"
TIMEOUT="${TIMEOUT:-180}"
USERDIR="$OUT/gekko_probe_user"
DOL="$OUT/gekko_probe.dol"

mkdir -p "$OUT" "$USERDIR/Config"
# Minimal isolated user dir: no analytics prompt, no panic dialogs, file logging.
cat > "$USERDIR/Config/Dolphin.ini" <<INI
[Interface]
UsePanicHandlers = False
[Analytics]
PermissionAsked = True
Enabled = False
[Core]
CPUCore = $CPU_CORE
CPUThread = False
GFXBackend = Null
INI
cat > "$USERDIR/Config/Logger.ini" <<INI
[Options]
WriteToFile = True
WriteToConsole = False
WriteToWindow = True
Verbosity = 4
[Logs]
Scripting = True
BOOT = True
CORE = True
MASTER = True
INI
rm -f "$USERDIR/Logs/dolphin.log" "$OUT/gekko_probe.status" \
      "$OUT/frsqrte_probe.jsonl" "$OUT/fres_probe.jsonl" "$OUT/fres64_probe.jsonl"

python3 "$HERE/mkdol.py" "$DOL"

export GEKKO_PROBE_DIR="$HERE" GEKKO_PROBE_OUT="$OUT"
echo "launching Dolphin (cpu core $CPU_CORE), timeout ${TIMEOUT}s"
"$DOLPHIN" -u "$USERDIR" -v Null \
    --script "$HERE/probe.py" \
    -C Logger.Logs.Scripting=True -C Logger.Options.WriteToFile=True \
    -C Dolphin.Interface.UsePanicHandlers=False \
    -C Dolphin.Core.CPUCore="$CPU_CORE" -C Dolphin.Core.CPUThread=False \
    -e "$DOL" >"$OUT/gekko_probe.console.log" 2>&1 &
PID=$!

deadline=$((SECONDS + TIMEOUT))
rc=""
while kill -0 "$PID" 2>/dev/null; do
    if (( SECONDS >= deadline )); then
        echo "timeout; killing Dolphin ($PID)"
        kill "$PID" 2>/dev/null || true
        sleep 2
        kill -9 "$PID" 2>/dev/null || true
        rc=124
        break
    fi
    sleep 1
done
if [[ -z "$rc" ]]; then wait "$PID" || rc=$?; rc="${rc:-0}"; fi

echo "--- status ($OUT/gekko_probe.status):"
cat "$OUT/gekko_probe.status" 2>/dev/null || echo "(none)"
echo "--- log: $USERDIR/Logs/dolphin.log"
grep -E "Script|BOOT|Boot|Panic|error|Error" "$USERDIR/Logs/dolphin.log" 2>/dev/null | tail -n 40 || true
echo "--- outputs:"
for f in frsqrte_probe fres_probe fres64_probe; do
    p="$OUT/$f.jsonl"
    if [[ -f "$p" ]]; then echo "$p  ($(wc -l <"$p") lines)"; else echo "$p  MISSING"; fi
done
grep -q '^DONE$' "$OUT/gekko_probe.status" 2>/dev/null
