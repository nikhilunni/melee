"""Parallel recording keeps every Dolphin in its own user folder."""
import os

import dolphin_config


def test_isolated_user_dirs_are_private_and_leave_the_environment_alone(monkeypatch, tmp_path):
    default = tmp_path / "Dolphin"
    (default / "Config").mkdir(parents=True)
    (default / "Config" / "Dolphin.ini").write_text("[Core]\n")
    (default / "Logs").mkdir()
    monkeypatch.setattr(dolphin_config, "DEFAULT_USER_DIR", default)
    monkeypatch.delenv("DOLPHIN_USER_DIR", raising=False)
    with dolphin_config.isolated_user_dir() as a, dolphin_config.isolated_user_dir() as b:
        assert a != b
        assert (a / "Config" / "Dolphin.ini").read_text() == "[Core]\n"
        assert not (a / "Logs").exists()
        assert "DOLPHIN_USER_DIR" not in os.environ
    assert not a.exists() and not b.exists()


def test_launch_flags_pass_the_user_dir(monkeypatch):
    monkeypatch.setenv("DOLPHIN_USER_DIR", "/tmp/private-dolphin")
    flags = dolphin_config.launch_flags(dolphin_config.HEADLESS_BIN, None)
    assert flags[-2:] == ["-u", "/tmp/private-dolphin"]
