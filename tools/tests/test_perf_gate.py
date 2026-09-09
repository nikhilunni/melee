"""Regression/failure-path tests for the release and merge guards (no game data)."""
import importlib.util
import contextlib
import io
import json
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("perf_report", ROOT / "tools/perf_report.py")
perf = importlib.util.module_from_spec(spec)
spec.loader.exec_module(perf)


class PerformanceBudget(unittest.TestCase):
    def test_each_metric_rejects_only_increases_past_its_budget(self):
        old = dict(stripped_bytes=100, text_bytes=100, load_ns=100, ticks_600_ns=100,
                   copies={"melee-sim": 8, "ft-fox": 2})
        current = dict(old, stripped_bytes=105, text_bytes=105, load_ns=110,
                       ticks_600_ns=110, copies={"melee-sim": 8, "ft-fox": 2})
        self.assertEqual(perf.compare(current, old, 10, 5, 0), [])
        for key in ("stripped_bytes", "text_bytes", "load_ns", "ticks_600_ns"):
            with self.subTest(key=key):
                bad = dict(current)
                bad[key] += 1
                self.assertEqual(len(perf.compare(bad, old, 10, 5, 0)), 1)
        current["copies"] = {"melee-sim": 9, "ft-fox": 1, "ft-new": 1}
        self.assertEqual(len(perf.compare(current, old, 10, 5, 0)), 2)
        self.assertEqual(perf.compare(current, old, 10, 5, 1), [])

    def test_platform_size_formats_and_invalid_evidence(self):
        self.assertEqual(perf.read_text_size("__TEXT __DATA __OBJC others dec hex\n3604480 1 0 0 0 0\n"), 3604480)
        self.assertEqual(perf.read_text_size("text data bss dec hex filename\n42 0 0 42 2a a.out\n"), 42)
        with self.assertRaises(ValueError):
            perf.read_text_size("size: cannot read binary")

    def test_llvm_lines_preserves_real_copy_counts(self):
        rows = perf.llvm_rows(""" Lines Copies Function name
 100 9 (TOTAL)
 90 (90.0%, 90.0%) 7 (77.8%, 77.8%) melee_ft::fighter::Fighter<C>::proc_anim
 10 (10.0%, 100.0%) 2 (22.2%, 100.0%) alloc::vec::Vec<T>::new
""")
        self.assertEqual(rows, [dict(lines=90, copies=7, function="melee_ft::fighter::Fighter<C>::proc_anim")])
        with self.assertRaises(ValueError):
            perf.llvm_rows("command failed")

    def test_missing_tools_and_benchmark_cannot_establish_a_baseline(self):
        with tempfile.TemporaryDirectory() as temporary:
            run = Path(temporary)
            (run / "melee-sim.stripped").write_bytes(bytes(100))
            (run / "size.txt").write_text("text data bss dec hex filename\n42 0 0 42 2a binary\n")
            (run / "rustc.txt").write_text("synthetic test compiler")
            (run / "revision.txt").write_text("synthetic test revision")
            (run / "bench.failed").touch()
            report = run / "PERF.md"
            with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
                result = perf.main(run, report)
            self.assertEqual(result, 1)
            self.assertIn("INCOMPLETE", report.read_text())
            self.assertNotIn('"status": "PASS"', report.read_text())

    def test_regression_does_not_raise_the_next_baseline(self):
        with tempfile.TemporaryDirectory() as temporary:
            run = Path(temporary)
            (run / "melee-sim.stripped").write_bytes(bytes(100))
            (run / "size.txt").write_text("text data bss dec hex filename\n42 0 0 42 2a binary\n")
            (run / "rustc.txt").write_text("synthetic test compiler")
            (run / "revision.txt").write_text("synthetic test revision")
            (run / "bloat.txt").write_text("10B melee_sim")
            (run / "llvm-version.txt").write_text("synthetic test llvm-lines")
            (run / "characters.txt").write_text("ft-fox\n")
            (run / "llvm-melee-sim.txt").write_text("100 7 (TOTAL)\n100 (100%, 100%) 7 (100%, 100%) melee_ft::function<C>\n")
            (run / "llvm-ft-fox.txt").write_text("100 1 (TOTAL)\n100 (100%, 100%) 1 (100%, 100%) ft_fox::function\n")
            for name in ("load", "ticks_600"):
                path = run / f"criterion/start_fd_fox/{name}/new/estimates.json"
                path.parent.mkdir(parents=True)
                path.write_text(json.dumps({"mean": {"point_estimate": 100, "confidence_interval": {"lower_bound": 90, "upper_bound": 110}}}))
            report = run / "PERF.md"
            with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
                self.assertEqual(perf.main(run, report), 0)
                (run / "melee-sim.stripped").write_bytes(bytes(106))
                self.assertEqual(perf.main(run, report), 1)
                self.assertEqual(perf.main(run, report), 1)
            self.assertEqual(report.read_text().count('"status": "PASS"'), 1)
            self.assertEqual(report.read_text().count('"status": "REGRESSION"'), 2)


if __name__ == "__main__":
    unittest.main()
