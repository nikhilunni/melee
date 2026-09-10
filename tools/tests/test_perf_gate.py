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
                   duplicate_labels={"melee-sim": 8, "ft-fox": 2}, cross_crate_duplicate_labels=3)
        current = dict(old, stripped_bytes=105, text_bytes=105, load_ns=110,
                       ticks_600_ns=110, duplicate_labels={"melee-sim": 8, "ft-fox": 2}, cross_crate_duplicate_labels=3)
        self.assertEqual(perf.compare(current, old, 10, 5, reviewed={}), [])
        for key in ("stripped_bytes", "text_bytes", "load_ns", "ticks_600_ns"):
            with self.subTest(key=key):
                bad = dict(current)
                bad[key] += 1
                self.assertEqual(len(perf.compare(bad, old, 10, 5, reviewed={})), 1)
        current["duplicate_labels"] = {"melee-sim": 9, "ft-fox": 1, "ft-new": 1}
        self.assertEqual(len(perf.compare(current, old, 10, 5, reviewed={})), 2)
        current["cross_crate_duplicate_labels"] = 4
        self.assertEqual(len(perf.compare(current, old, 10, 5, reviewed={})), 3)

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
            (run / "llvm-melee-ft.txt").write_text("100 4 (TOTAL)\n" + "".join(
                f"25 (25%, 100%) 1 (25%, 100%) {name}\n" for name in sorted(perf.PAIR_HELPERS)))
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
            self.assertIn("COMPLETE — PASS", report.read_text())
            self.assertIn("### melee-ft: 4 labels, 4 emitted definitions (informational); 0 duplicate labels", report.read_text())


class ConcreteShellBudget(unittest.TestCase):
    def pairs(self):
        return [dict(function=name, copies=1, lines=10) for name in perf.PAIR_HELPERS]

    def test_a_common_body_moved_to_another_crate_still_counts_twice(self):
        name = "melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter>::enter_shield"
        row = dict(function=name, copies=1, lines=20)
        labels, failures = perf.concrete_shell_census({"melee-ft": self.pairs() + [row]})
        self.assertEqual(failures, [])
        self.assertEqual(labels[name]["copies"], 1)
        labels, failures = perf.concrete_shell_census({"melee-ft": self.pairs() + [row], "ft-yoshi": [row]})
        self.assertEqual(labels[name]["copies"], 2)
        self.assertEqual(len(failures), 1)
        self.assertIn("ft-yoshi", failures[0])

    def test_generic_shell_is_rejected_even_with_one_instantiation(self):
        row = dict(function="melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::enter_shield", copies=1, lines=20)
        _, failures = perf.concrete_shell_census({"melee-ft": self.pairs() + [row]})
        self.assertEqual(len(failures), 1)
        self.assertIn("Fighter<C>", failures[0])

    def test_missing_pair_helper_cannot_pass_as_a_smaller_census(self):
        rows = self.pairs()
        missing = rows.pop()["function"]
        _, failures = perf.concrete_shell_census({"melee-ft": rows})
        self.assertEqual(failures, [f"missing concrete pair helper: {missing}"])

    def test_fixed_caps_reject_size_time_and_duplicate_growth(self):
        current = dict(stripped_bytes=perf.C15_STRIPPED_LIMIT, **perf.c15_census(), **perf.C15_P1_TIME_LIMITS)
        self.assertEqual(perf.concrete_limits(current), [])
        bad = dict(current, stripped_bytes=current["stripped_bytes"] + 1)
        self.assertTrue(perf.concrete_limits(bad))
        bad = dict(current, ticks_600_ns=current["ticks_600_ns"] + 1)
        self.assertTrue(perf.concrete_limits(bad))
        counts = dict(current["duplicate_labels"])
        counts["melee-ft"] += 1
        self.assertTrue(perf.concrete_limits(dict(current, duplicate_labels=counts)))
        bad = dict(current, cross_crate_duplicate_labels=current["cross_crate_duplicate_labels"] + 1)
        self.assertTrue(perf.concrete_limits(bad))
        # Ordinary additions do not spend the duplication budget.
        self.assertEqual(perf.concrete_limits(dict(current, labels={"melee-ft": 99999}, definitions={"melee-ft": 99999})), [])

    def test_new_concrete_definitions_pass_but_both_duplication_forms_fail(self):
        def row(name, copies=1):
            return dict(function="melee_ft::" + name, copies=copies, lines=10)
        old, _ = perf.duplicate_census({"melee-ft": [row("old")]})
        added, _ = perf.duplicate_census({"melee-ft": [row("old"), row("new")]})
        self.assertEqual(perf.compare_duplicates(added, old), [])
        self.assertEqual(added["labels"]["melee-ft"], 2)
        repeated, _ = perf.duplicate_census({"melee-ft": [row("old", 3)]})
        self.assertEqual(repeated["duplicate_labels"]["melee-ft"], 1)
        self.assertEqual(len(perf.compare_duplicates(repeated, old)), 1)
        across, labels = perf.duplicate_census({"melee-ft": [row("old")], "ft-new": [row("old")]})
        self.assertEqual(across["cross_crate_duplicate_labels"], 1)
        self.assertEqual(across["duplicate_labels"], {"melee-ft": 0, "ft-new": 0})
        self.assertEqual(len(perf.compare_duplicates(across, old)), 1)
        self.assertEqual(labels["melee_ft::old"], {"melee-ft": 1, "ft-new": 1})

    def test_repeated_rows_are_aggregated_before_counting_labels(self):
        row = dict(function="melee_ft::one", copies=1, lines=10)
        census, _ = perf.duplicate_census({"melee-ft": [row, row, row]})
        self.assertEqual(census["labels"], {"melee-ft": 1})
        self.assertEqual(census["definitions"], {"melee-ft": 3})
        self.assertEqual(census["duplicate_labels"], {"melee-ft": 1})


if __name__ == "__main__":
    unittest.main()
