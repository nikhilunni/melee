"""Exercise the merge guard with real, disposable Git repositories, no game data.

Write minimal loose objects and a version-2 index directly: fixture setup never
runs git init/add/commit or any other Git write command. Only cargo is stubbed.
"""
import hashlib
import os
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile
import unittest
import zlib

ROOT = Path(__file__).resolve().parents[2]


class Repository:
    def __init__(self, root, main_entries=None, lane_entries=None, ancestor=True):
        self.root = root
        self.git = root / ".git"
        (self.git / "objects").mkdir(parents=True)
        (self.git / "refs/heads/lane").mkdir(parents=True)
        (self.git / "HEAD").write_text("ref: refs/heads/lane/test\n")
        (self.git / "config").write_text("[core]\nrepositoryformatversion = 0\nbare = false\n")
        main_entries = main_entries or {}
        lane_entries = main_entries if lane_entries is None else lane_entries
        base = self.commit({}, message="common base fixture")
        main = self.commit(main_entries, parent=base, message="main fixture")
        lane = self.commit(lane_entries, parent=main if ancestor else base, message="lane fixture")
        (self.git / "refs/heads/main").write_text(main + "\n")
        (self.git / "refs/heads/lane/test").write_text(lane + "\n")
        self.index(lane_entries)
        (root / "tools").mkdir()
        shutil.copy2(ROOT / "tools/merge-check.sh", root / "tools/merge-check.sh")
        (root / "bin").mkdir()
        cargo = root / "bin/cargo"
        cargo.write_text('''#!/bin/sh
echo "$*" >> "$CALLS"
if [ "$1" = "$FAIL" ]; then
    if [ -n "$MARKER" ]; then echo "$MARKER"; exit 0; fi
    exit 1
fi
''')
        cargo.chmod(0o755)

    def object(self, kind, payload):
        contents = f"{kind} {len(payload)}\0".encode() + payload
        digest = hashlib.sha1(contents).hexdigest()
        path = self.git / "objects" / digest[:2] / digest[2:]
        path.parent.mkdir(exist_ok=True)
        path.write_bytes(zlib.compress(contents))
        return digest

    def tree(self, entries):
        directories = {}
        files = {}
        for path, (mode, value) in entries.items():
            name, separator, rest = path.partition("/")
            if separator:
                directories.setdefault(name, {})[rest] = (mode, value)
            else:
                # Gitlinks refer to commit IDs rather than blob objects.
                oid = value if mode == 0o160000 else self.object("blob", value.encode())
                files[name] = (mode, oid)
        for name, children in directories.items():
            files[name] = (0o40000, self.tree(children))
        ordered = sorted(files, key=lambda name: name + ("/" if files[name][0] == 0o40000 else ""))
        payload = b"".join(f"{files[name][0]:o} {name}\0".encode() + bytes.fromhex(files[name][1])
                           for name in ordered)
        return self.object("tree", payload)

    def commit(self, entries, parent=None, message="fixture"):
        lines = [f"tree {self.tree(entries)}"]
        if parent:
            lines.append(f"parent {parent}")
        lines += ["author Fixture <fixture@example.invalid> 1 +0000",
                  "committer Fixture <fixture@example.invalid> 1 +0000", "", message, ""]
        return self.object("commit", "\n".join(lines).encode())

    def index(self, entries):
        payload = b"DIRC" + struct.pack("!II", 2, len(entries))
        for path, (mode, value) in sorted(entries.items()):
            encoded = path.encode()
            oid = value if mode == 0o160000 else self.object("blob", value.encode())
            entry = struct.pack("!10I", 0, 0, 0, 0, 0, 0, mode, 0, 0, 0)
            entry += bytes.fromhex(oid) + struct.pack("!H", len(encoded)) + encoded + b"\0"
            payload += entry + bytes((-len(entry)) % 8)
        (self.git / "index").write_bytes(payload + hashlib.sha1(payload).digest())

    def oracle(self, kind):
        harness = self.root / "harness"
        harness.mkdir(exist_ok=True)
        traces = harness / "traces"
        if kind == "missing":
            return
        if kind == "broken":
            traces.symlink_to("absent-directory", target_is_directory=True)
            return
        if kind == "loop":
            traces.symlink_to("traces", target_is_directory=True)
            return
        if kind == "symlink":
            target = self.root / "fixture-traces/nested"
            target.mkdir(parents=True)
            (target / "fixture.expected.jsonl").write_text("{}\n")
            traces.symlink_to("../fixture-traces", target_is_directory=True)
            return
        traces.mkdir()
        if kind == "present":
            (traces / "fixture.expected.jsonl").write_text("{}\n")
        elif kind == "compressed":
            (traces / "fixture.expected.jsonl.zst").write_bytes(b"fixture")
        elif kind == "wrong-suffix":
            (traces / "fixture.raw.jsonl").write_text("{}\n")
        elif kind == "directory-only":
            (traces / "fixture.expected.jsonl").mkdir()

    def run(self, *, override=False, fail="", marker="", data_env=None):
        env = dict(os.environ)
        for name in list(env):
            if name.startswith("GIT_") or name in ("MELEE_ALLOW_MISSING_DATA", "MELEE_TEST_DATA_ROOT"):
                del env[name]
        env.update(PATH=f"{self.root / 'bin'}:{os.environ['PATH']}",
                   CALLS=str(self.root / "calls"), FAIL=fail, MARKER=marker,
                   GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull, GIT_OPTIONAL_LOCKS="0")
        env.update(data_env or {})
        command = [str(self.root / "tools/merge-check.sh"), "lane/test"]
        if override:
            command.append("--allow-missing-data")
        result = subprocess.run(command, cwd=self.root, env=env, capture_output=True, text=True)
        calls = (self.root / "calls").read_text().splitlines() if (self.root / "calls").exists() else []
        return result, calls


class MergeCheck(unittest.TestCase):
    def run_chain(self, *, main_entries=None, lane_entries=None, ancestor=True,
                  oracle="present", override=False, fail="", marker="", data_env=None):
        with tempfile.TemporaryDirectory() as temporary:
            repo = Repository(Path(temporary), main_entries, lane_entries, ancestor)
            repo.oracle(oracle)
            return repo.run(override=override, fail=fail, marker=marker, data_env=data_env)

    def test_full_chain_order_with_real_git(self):
        result, calls = self.run_chain()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(calls, ["build --workspace --all-targets --locked", "gate --locked",
                                "test -p melee-sim --test m4_gate --locked", "test -p melee-sim --test m5_gate --locked",
                                "test -p hsd-particle --locked", "clippy --workspace --all-targets --locked -- -D warnings",
                                "fmt --all -- --check"])
        self.assertEqual(result.stdout.count("[PASS]"), 10)

    def test_ancestry_failure_never_builds(self):
        result, calls = self.run_chain(ancestor=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("[FAIL] rebase:", result.stdout)
        self.assertEqual(calls, [])

    def test_first_failed_step_stops_the_chain(self):
        for fail, marker, count in [("build", "error[E0001]", 1),
                                    ("build", "could not compile", 1),
                                    ("gate", "FAILED", 2), ("test", "panicked", 3),
                                    ("clippy", "", 6), ("fmt", "Diff in file.rs", 7)]:
            with self.subTest(fail=fail, marker=marker):
                result, calls = self.run_chain(fail=fail, marker=marker)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(len(calls), count)
                self.assertEqual(result.stdout.count("[FAIL]"), 1)

    def test_tracked_game_data_and_symlinks_fail_even_with_override(self):
        for path, mode in [("harness/roms", 0o120000), ("harness/traces", 0o120000),
                           ("harness/roms/nested/file with spaces.dat", 0o100644),
                           ("harness/traces/nested/fixture.expected.jsonl", 0o100644)]:
            for override in (False, True):
                with self.subTest(path=path, override=override):
                    # Inherited entries have no lane diff: the index check must catch them.
                    result, calls = self.run_chain(main_entries={path: (mode, "fixture")},
                                                   oracle="missing", override=override)
                    self.assertEqual(result.returncode, 1)
                    self.assertIn(f"[FAIL] data: {path} is tracked", result.stdout)
                    self.assertNotIn("[PASS] rebase", result.stdout)
                    self.assertEqual(calls, [])

    def test_staged_game_data_is_rejected_before_it_reaches_a_lane_commit(self):
        with tempfile.TemporaryDirectory() as temporary:
            repo = Repository(Path(temporary))
            repo.oracle("present")
            repo.index({"harness/roms": (0o120000, "fixture-target")})
            result, calls = repo.run(override=True)
            self.assertEqual(result.returncode, 1)
            self.assertIn("[FAIL] data: harness/roms is tracked", result.stdout)
            self.assertEqual(calls, [])

    def test_lane_removal_of_protected_data_fails_even_when_index_is_clean(self):
        for path in ("harness/roms", "harness/traces", "harness/traces/nested/old.expected.jsonl"):
            with self.subTest(path=path):
                result, calls = self.run_chain(main_entries={path: (0o120000, "fixture")},
                                               lane_entries={}, override=True)
                self.assertEqual(result.returncode, 1)
                self.assertIn(f"[FAIL] data: lane commits touch {path}", result.stdout)
                self.assertEqual(calls, [])

    def test_renaming_protected_data_out_of_the_directory_is_rejected(self):
        path = "harness/traces/old.expected.jsonl"
        result, calls = self.run_chain(main_entries={path: (0o100644, "same fixture")},
                                       lane_entries={"docs/moved-fixture": (0o100644, "same fixture")})
        self.assertEqual(result.returncode, 1)
        self.assertIn(f"[FAIL] data: lane commits touch {path}", result.stdout)
        self.assertEqual(calls, [])

    def test_lane_decomp_gitlink_change_fails_even_with_override(self):
        path = "third_party/melee-decomp"
        result, calls = self.run_chain(main_entries={path: (0o160000, "1" * 40)},
                                       lane_entries={path: (0o160000, "2" * 40)}, override=True)
        self.assertEqual(result.returncode, 1)
        self.assertIn(f"[FAIL] data: lane commits touch {path}", result.stdout)
        self.assertEqual(calls, [])

    def test_unrelated_paths_and_unchanged_decomp_are_allowed(self):
        main = {"third_party/melee-decomp": (0o160000, "1" * 40)}
        lane = dict(main, **{"harness/traces_notes.md": (0o100644, "fixture"),
                             "third_party/melee-decomp-notes.md": (0o100644, "fixture")})
        result, calls = self.run_chain(main_entries=main, lane_entries=lane)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(len(calls), 7)

    def test_absent_oracle_fails_before_build_and_advertises_recording(self):
        for kind in ("missing", "empty", "broken", "loop", "wrong-suffix", "directory-only"):
            with self.subTest(kind=kind):
                result, calls = self.run_chain(oracle=kind)
                self.assertEqual(result.returncode, 1)
                self.assertIn("[FAIL] data: harness/traces is empty;", result.stdout)
                self.assertIn("harness/record.py <scenario>", result.stdout)
                self.assertEqual(calls, [])

    def test_legacy_override_cannot_bypass_missing_oracle(self):
        result, calls = self.run_chain(oracle="missing", override=True)
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn("--allow-missing-data no longer permits", result.stdout)
        self.assertEqual(calls, [])

    def test_missing_data_environment_never_runs_the_chain(self):
        for variable in ("MELEE_ALLOW_MISSING_DATA", "MELEE_TEST_DATA_ROOT"):
            for value in ("1", "0", ""):
                with self.subTest(variable=variable, value=value):
                    result, calls = self.run_chain(data_env={variable: value})
                    self.assertEqual(result.returncode, 1)
                    self.assertEqual(result.stdout.strip(),
                                     f"[FAIL] data: {variable} is set; unset it before running the merge chain")
                    self.assertEqual(calls, [])

    def test_compressed_oracle_counts_as_present(self):
        result, calls = self.run_chain(oracle="compressed")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("[PASS] data: oracle traces present", result.stdout)
        self.assertEqual(len(calls), 7)

    def test_oracle_directory_symlink_is_followed(self):
        result, calls = self.run_chain(oracle="symlink")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("[PASS] data: oracle traces present", result.stdout)
        self.assertEqual(len(calls), 7)


if __name__ == "__main__":
    unittest.main()
