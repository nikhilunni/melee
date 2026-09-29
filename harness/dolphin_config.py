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
  DOLPHIN_USER_DIR  a private Dolphin user folder (-u); parallel recordings
                    each get one from isolated_user_dir()
  DOLPHIN_CHEATS=1  enable cheats, so the user folder's Gecko codes run
                    (gecko.py; set by record.py for a scenario's `gecko`)
"""
from __future__ import annotations

import contextlib
import os
import shutil
import tempfile
from collections.abc import Iterator
from pathlib import Path

BINARIES = Path.home() / "Projects/dolphin-scripting/build/Binaries"
HEADLESS_BIN = BINARIES / "DolphinHeadless.app/Contents/MacOS/dolphin-emu-nogui"
GUI_BIN = BINARIES / "Dolphin.app/Contents/MacOS/Dolphin"
DEFAULT_USER_DIR = Path.home() / "Library/Application Support/Dolphin"


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
    if user_dir := os.environ.get("DOLPHIN_USER_DIR"):
        flags += ["-u", user_dir]
    if os.environ.get("DOLPHIN_CHEATS"):
        flags += ["-C", "Dolphin.Core.EnableCheats=True"]
    return flags


@contextlib.contextmanager
def isolated_user_dir() -> Iterator[Path]:
    """A private copy of the default user folder (a few MB: config, SRAM,
    memory cards), removed afterwards. Callers pass it to their Dolphin as
    DOLPHIN_USER_DIR in the child's environment, so concurrent emulators never
    share config, logs or card writes. It never touches os.environ, which
    record_many's threads share."""
    with tempfile.TemporaryDirectory(prefix="dolphin-user-") as tmp:
        root = Path(tmp) / "Dolphin"
        shutil.copytree(DEFAULT_USER_DIR, root, ignore=shutil.ignore_patterns("Cache", "Logs", "Dump", "ScreenShots"))
        yield root
