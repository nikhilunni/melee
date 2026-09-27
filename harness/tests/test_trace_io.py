import json
import os

import pytest
import zstandard

import compress_traces
import trace_io

ROWS = "".join(json.dumps({"frame": i, "state": {"x": i * 0.5}}) + "\n" for i in range(2000))


def write_compressed(path, text):
    trace_io.compressed_path(path).write_bytes(zstandard.ZstdCompressor(level=3).compress(text.encode()))


def test_plain_and_compressed_read_identically(tmp_path):
    plain, packed = tmp_path / "plain.jsonl", tmp_path / "packed.jsonl"
    plain.write_text(ROWS)
    write_compressed(packed, ROWS)
    assert not packed.exists() and trace_io.exists(packed)
    assert trace_io.read_text(plain) == trace_io.read_text(packed) == ROWS
    with trace_io.open_text(packed) as lines:
        assert list(lines) == ROWS.splitlines(keepends=True)
    assert trace_io.read_text(trace_io.compressed_path(packed)) == ROWS


def test_plain_file_is_preferred_over_compressed_sibling(tmp_path):
    path = tmp_path / "both.jsonl"
    path.write_text("plain\n")
    write_compressed(path, "compressed\n")
    assert trace_io.resolve(path) == path
    assert trace_io.read_text(path) == "plain\n"
    path.unlink()
    assert trace_io.resolve(path) == trace_io.compressed_path(path)
    assert trace_io.read_text(path) == "compressed\n"


def test_missing_trace_names_both_forms(tmp_path):
    path = tmp_path / "absent.jsonl"
    assert not trace_io.exists(path)
    with pytest.raises(FileNotFoundError, match="absent.jsonl.zst"):
        trace_io.open_text(path)


def test_verified_compression_replaces_plain_and_keeps_mtime(tmp_path):
    path = tmp_path / "t.tick.expected.jsonl"
    path.write_text(ROWS)
    os.utime(path, ns=(1_000_000_000, 2_000_000_000))
    result = trace_io.compress_verified(path, level=5, threads=2)
    assert not path.exists()
    assert result.compressed == trace_io.compressed_path(path)
    assert result.plain_bytes == len(ROWS) and 0 < result.compressed_bytes < len(ROWS)
    assert result.compressed.stat().st_mtime_ns == 2_000_000_000
    assert trace_io.read_text(path) == ROWS
    assert not list(tmp_path.glob("*.tmp"))


def test_failed_verification_leaves_plain_untouched(tmp_path, monkeypatch):
    path = tmp_path / "t.particles.jsonl"
    path.write_text(ROWS)
    write_compressed(path, "stale\n")
    monkeypatch.setattr(trace_io, "_same_bytes", lambda plain, compressed: False)
    with pytest.raises(RuntimeError, match="does not decompress"):
        trace_io.compress_verified(path)
    assert path.read_text() == ROWS
    assert trace_io.read_text(trace_io.compressed_path(path)) == "stale\n"
    assert not list(tmp_path.glob("*.tmp"))


def test_verifier_detects_differing_and_truncated_output(tmp_path):
    plain, other = tmp_path / "p.jsonl", tmp_path / "o.jsonl"
    plain.write_text(ROWS)
    other.write_text(ROWS[:-2] + "X\n")
    write_compressed(plain, ROWS)
    packed = trace_io.compressed_path(plain)
    assert trace_io._same_bytes(plain, packed)
    assert not trace_io._same_bytes(other, packed)
    write_compressed(other, ROWS[:100])
    assert not trace_io._same_bytes(plain, trace_io.compressed_path(other))


def test_recompression_replaces_a_stale_compressed_sibling(tmp_path):
    path = tmp_path / "t.ledger.raw.jsonl"
    write_compressed(path, "old recording\n")
    path.write_text(ROWS)
    trace_io.compress_verified(path)
    assert not path.exists() and trace_io.read_text(path) == ROWS


def test_compress_outputs_skips_small_missing_and_non_jsonl(tmp_path):
    big, small, marker = tmp_path / "a.jsonl", tmp_path / "b.jsonl", tmp_path / "a.jsonl.done"
    big.write_text(ROWS)
    small.write_text("{}\n")
    marker.write_text(ROWS)
    done = trace_io.compress_outputs([big, big, small, marker, tmp_path / "gone.jsonl"],
                                     min_size=1024, log=None)
    assert [r.plain for r in done] == [big]
    assert small.exists() and marker.exists() and not big.exists()


def test_migration_is_a_dry_run_unless_applied(tmp_path, capsys):
    nested = tmp_path / "jit"
    nested.mkdir()
    paths = [tmp_path / "x.tick.raw.jsonl", nested / "fres_probe.jsonl"]
    for path in paths:
        path.write_text(ROWS)
    old = 1_000_000_000
    for path in paths:
        os.utime(path, (old, old))
    (tmp_path / "x.tick.raw.jsonl.done").write_text("{}")
    args = ["--dir", str(tmp_path), "--min-size", "1024"]
    assert compress_traces.main(args + ["--estimate"]) == 0
    assert all(p.exists() for p in paths)
    assert "dry run" in capsys.readouterr().out
    assert compress_traces.main(args + ["--apply"]) == 0
    assert not any(p.exists() for p in paths)
    assert all(trace_io.read_text(p) == ROWS for p in paths)
    assert (tmp_path / "x.tick.raw.jsonl.done").read_text() == "{}"


def test_migration_skips_recently_modified_files(tmp_path):
    path = tmp_path / "live.particles.jsonl"
    path.write_text(ROWS)
    assert compress_traces.main(["--dir", str(tmp_path), "--min-size", "1024", "--apply"]) == 0
    assert path.exists()
