from datetime import date
from pathlib import Path

from netconfig_splitter.parser import parse
from netconfig_splitter.writer import write_captures

FIXTURES = Path(__file__).parent / "fixtures"


class TestWriteCaptures:
    def test_default_layout_uses_command_name_as_filename(self, tmp_path):
        text = (FIXTURES / "sample_iosxr.txt").read_text()
        captures = parse(text, default_year=2026)

        written = write_captures(captures, tmp_path)

        expected = tmp_path / "router-xr" / "show running-config.txt"
        assert expected in written
        text = expected.read_text()
        assert text.startswith("RP/0/RSP1/CPU0:router-xr#show running-config")
        assert "Building configuration..." in text

    def test_default_layout_dedupes_same_host_command(self, tmp_path):
        text = "\n".join(
            [
                "r1#show version",
                "output one",
                "r1#show version",
                "output two",
                "r1#",
            ]
        )
        captures = parse(text, fallback_date=date(2026, 1, 1))
        written = write_captures(captures, tmp_path)

        assert (tmp_path / "r1" / "show version.txt") in written
        assert (tmp_path / "r1" / "show version-2.txt") in written

    def test_dated_layout_uses_date_subfolder(self, tmp_path):
        text = (FIXTURES / "sample_iosxr.txt").read_text()
        captures = parse(text, default_year=2026)

        written = write_captures(captures, tmp_path, flat=False)

        expected = tmp_path / "router-xr" / "show running-config" / "2026-10-19.txt"
        assert expected in written
        assert "Building configuration..." in expected.read_text()

    def test_dated_layout_dedupes_same_host_command_date(self, tmp_path):
        text = "\n".join(
            [
                "r1#show version",
                "output one",
                "r1#show version",
                "output two",
                "r1#",
            ]
        )
        captures = parse(text, fallback_date=date(2026, 1, 1))
        written = write_captures(captures, tmp_path, flat=False)

        assert (tmp_path / "r1" / "show version" / "2026-01-01.txt") in written
        assert (tmp_path / "r1" / "show version" / "2026-01-01-2.txt") in written

