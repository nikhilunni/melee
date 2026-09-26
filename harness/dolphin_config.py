"""Which Dolphin the harness launches, and the flags every launch shares.

Scripted recordings run the headless build (tools/build-headless-dolphin.sh):
no window, no audio, Null video. It records byte-identical traces to the
windowed app (docs/DOLPHIN_RUN.md) and does not use macOS graphics clients.
Live human play and menu driving (drive.py screenshots) need the windowed app.

Environment overrides:
  DOLPHIN_BIN       explicit executable (either kind)
  DOLPHIN_GUI=1     use the windowed app everywhere
  DOLPHIN_PLATFORM  no-GUI window platform (default "headless")
  DOLPHIN_AUDIO=1   keep host audio output
"""
from __future__ import annotations

import os
from pathlib import Path

BINARIES = Path.home() / "Projects/dolphin-scripting/build/Binaries"
HEADLESS_BIN = BINARIES / "DolphinHeadless.app/Contents/MacOS/dolphin-emu-nogui"
GUI_BIN = BINARIES / "Dolphin.app/Contents/MacOS/Dolphin"


def binary(gui: bool = False) -> Path:
    """The executable to launch; `gui` requests the windowed app."""
    if explicit := os.environ.get("DOLPHIN_BIN"):
        return Path(explicit)
    if gui or os.environ.get("DOLPHIN_GUI"):
        return GUI_BIN
    if not HEADLESS_BIN.exists():
        raise SystemExit(f"{HEADLESS_BIN} is missing: run tools/build-headless-dolphin.sh "
                         "(or set DOLPHIN_GUI=1 for the windowed app)")
    return HEADLESS_BIN


def is_headless(executable: Path) -> bool:
    return executable.name == "dolphin-emu-nogui"


def launch_flags(executable: Path, video: str | None) -> list[str]:
    """Video, platform and audio flags. The headless build needs Null video;
    the windowed app defaults to OGL, as recordings always have."""
    headless = is_headless(executable)
    flags = ["-v", video or ("Null" if headless else "OGL")]
    if headless:
        flags += ["--platform", os.environ.get("DOLPHIN_PLATFORM", "headless")]
    if not os.environ.get("DOLPHIN_AUDIO"):
        # The backend only plays samples the emulated DSP already produced, so
        # muting it cannot change game state.
        flags += ["-C", "Dolphin.DSP.Backend=No Audio Output"]
    return flags
