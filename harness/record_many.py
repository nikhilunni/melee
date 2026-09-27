"""Record several scripted scenarios at once, each in its own headless Dolphin.

    cd harness && uv run python record_many.py scenarios/a.toml scenarios/b.toml ... [--jobs 4] [-- record.py args]

Every scenario runs the full `record.py` capture chain (tick trace, RNG ledger,
particle dump, compression) in a child process with a private Dolphin user
folder (dolphin_config.isolated_user_dir), so concurrent emulators share no
config, SRAM, card or log files. Outputs are per-scenario files in
harness/traces, as with record.py; each child's console output goes to
traces/<name>.record.log. Human-port scenarios need the windowed app and are
refused: record those one at a time with record.py.

Exits non-zero if any recording failed; the summary names each failure.
"""
from __future__ import annotations

import argparse
import os
import subprocess
import sys
import time
import tomllib
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import dolphin_config  # noqa: E402

# Measured 2026-09-27 on 12 cores: eight 1,300-tick Sudden Death scenes took
# 28 s wall at 8 jobs against ~21 s each serially, with byte-identical traces.
DEFAULT_JOBS = 8


def record_one(scenario: Path, extra: list[str]) -> tuple[Path, int, float]:
    name = tomllib.loads(scenario.read_text())["name"]
    log = HERE / "traces" / f"{name}.record.log"
    t0 = time.monotonic()
    with dolphin_config.isolated_user_dir() as user_dir, log.open("wb") as out:
        env = {**os.environ, "DOLPHIN_USER_DIR": str(user_dir)}
        code = subprocess.run([sys.executable, str(HERE / "record.py"), str(scenario), *extra],
                              env=env, stdout=out, stderr=subprocess.STDOUT).returncode
    return scenario, code, time.monotonic() - t0


def main(argv: list[str] | None = None) -> None:
    argv = list(sys.argv[1:] if argv is None else argv)
    extra: list[str] = []
    if "--" in argv:
        at = argv.index("--")
        argv, extra = argv[:at], argv[at + 1:]
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("scenarios", type=Path, nargs="+")
    ap.add_argument("--jobs", type=int, default=DEFAULT_JOBS)
    a = ap.parse_args(argv)

    if not dolphin_config.is_headless(dolphin_config.binary()):
        sys.exit("record_many needs the headless Dolphin (unset DOLPHIN_GUI / DOLPHIN_BIN)")
    scenarios = [p.resolve() for p in a.scenarios]
    for p in scenarios:
        fighters = tomllib.loads(p.read_text()).get("fighters", [])
        if any(f.get("controller") == "human" for f in fighters):
            sys.exit(f"{p.name}: human ports need the windowed app; use record.py")
    (HERE / "traces").mkdir(exist_ok=True)

    t0 = time.monotonic()
    failures = []
    with ThreadPoolExecutor(max_workers=max(1, a.jobs)) as pool:
        for scenario, code, wall in pool.map(lambda p: record_one(p, extra), scenarios):
            status = "ok" if code == 0 else f"FAILED ({code})"
            print(f"{scenario.stem}: {status} in {wall:.0f}s", flush=True)
            if code:
                failures.append(scenario.stem)
    print(f"{len(scenarios)} scenarios, {len(failures)} failed, {time.monotonic() - t0:.0f}s wall "
          f"with {a.jobs} jobs")
    if failures:
        sys.exit("failed: " + " ".join(failures))


if __name__ == "__main__":
    main()
