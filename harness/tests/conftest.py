import sys
from pathlib import Path

HARNESS = Path(__file__).resolve().parents[1]
for p in (HARNESS, HARNESS / "dolphin"):
    if str(p) not in sys.path:
        sys.path.insert(0, str(p))
