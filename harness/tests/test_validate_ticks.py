import json
import struct

import pytest

from validate_ticks import draw_count, lcg, main, validate


def record(p0, p1, seed=123):
    def field(frame):
        return {"t": "f32", "v": {"bits": struct.unpack(">I", struct.pack(">f", frame))[0],
                                  "approx": -999}}  # deliberately wrong approximation
    return {"frame": 99, "phase": "frame_end", "state": {
        "p0.cur_anim_frame": field(p0), "p1.cur_anim_frame": field(p1),
        "rng.seed": {"t": "u", "v": seed}}}


def test_unit_steps_restarts_and_zero_through_k_draws():
    rows = [record(118, 0), record(119, 1), record(0, 2, lcg(123)),
            record(1, 0, lcg(lcg(lcg(123))))]
    result = validate(rows, max_draws=2)
    assert result == {"records": 4, "histogram": {0: 1, 1: 1, 2: 1}, "violations": []}
    assert lcg(0xFFFFFFFF) == 2316998  # overflow is modulo 2^32


def test_duplicate_skipped_ticks_and_rng_bound_report_ordinals():
    rows = [record(11, 15), record(13, 16), record(13, 18), record(15, 18, lcg(lcg(123)))]
    result = validate(rows, max_draws=1)
    assert len(result["violations"]) == 6
    assert sum("ordinal 2:" in line for line in result["violations"]) == 2
    assert "not reachable in 0..1" in result["violations"][-1]
    assert draw_count(123, lcg(lcg(123)), 2) == 2
    assert draw_count(123, lcg(123), 0) is None


@pytest.mark.parametrize("bad", [float("nan"), float("inf"), float("-inf")])
def test_nonfinite_animation_is_invalid(bad):
    assert validate([record(0, 0), record(bad, 1)])["violations"]


def test_missing_fighter_or_empty_trace_is_invalid():
    row = record(1, 1)
    del row["state"]["p1.cur_anim_frame"]
    assert "field set changed" in validate([record(0, 0), row])["violations"][0]
    assert validate([])["violations"]
    assert validate([record(0, 0)])["violations"]


def test_large_unchanged_f32_is_not_mistaken_for_unit_advance():
    assert validate([record(2**60, 0), record(2**60, 1)])["violations"]


def test_tick_metadata_allows_two_ticks_per_vi_but_rejects_counter_gap():
    rows = [record(0, 0), record(1, 1)]
    for i, row in enumerate(rows):
        row.update(tick=100+i, vi_frame=5, watch_value=99+i, watch_address=0x80479D58)
    assert not validate(rows)["violations"]
    rows[1].update(tick=102, watch_value=101)
    assert validate(rows)["violations"] == ["ordinal 1: nonconsecutive game tick"]


def test_cli_exit_status_and_histogram(tmp_path, capsys):
    trace = tmp_path / "trace.jsonl"
    trace.write_text("\n" + json.dumps(record(0, 0)) + "\n" + json.dumps(record(1, 1)))
    assert main([str(trace)]) == 0
    assert "{0: 1}" in capsys.readouterr().out
    trace.write_text(json.dumps(record(0, 0)) + "\n" + json.dumps(record(0, 1)))
    assert main([str(trace)]) == 1
    assert "ordinal 1: p0.cur_anim_frame" in capsys.readouterr().out
    trace.write_text("not json")
    assert main([str(trace)]) == 1
    assert "invalid trace" in capsys.readouterr().out
