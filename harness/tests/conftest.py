import sys
from pathlib import Path

import pytest

HARNESS = Path(__file__).resolve().parents[1]
for p in (HARNESS, HARNESS / "dolphin"):
    if str(p) not in sys.path:
        sys.path.insert(0, str(p))


@pytest.fixture(autouse=True)
def data_root_writes_stay_in_tmp(monkeypatch, tmp_path_factory):
    """No test writes into the data root: the bridge tools write scenarios and
    recordings under `data_root.SCENARIOS` and `data_root.TRACES`, which with
    `MELEE_DATA_ROOT` set is another checkout's `harness/`. Every test gets
    empty directories of its own for both; a test that needs files there
    creates them (or points the names at its own `tmp_path`)."""
    import data_root

    scratch = tmp_path_factory.mktemp("data_root")
    for name in ("SCENARIOS", "TRACES"):
        directory = scratch / name.lower()
        directory.mkdir()
        monkeypatch.setattr(data_root, name, directory)
