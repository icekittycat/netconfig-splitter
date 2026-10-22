from datetime import date
from pathlib import Path

from netconfig_splitter.parser import parse

FIXTURES = Path(__file__).parent / "fixtures"


class TestIosXr:
    def test_splits_by_command_with_banner_date(self):
        text = (FIXTURES / "sample_iosxr.txt").read_text()
        captures = parse(text, default_year=2026)

        assert [(c.hostname, c.command, c.date) for c in captures] == [
            ("router-xr", "show running-config", date(2026, 10, 19)),
            ("router-xr", "show version", date(2026, 10, 19)),
        ]
        assert "hostname router-xr" in captures[0].text
        assert "interface MgmtEth0/RSP1/CPU0/0" in captures[0].text
        # The original prompt line is kept as a clean header on the capture.
        assert captures[0].text.startswith("RP/0/RSP1/CPU0:router-xr#show running-config")

    def test_commands_with_no_real_output_are_dropped(self):
        text = (FIXTURES / "sample_iosxr.txt").read_text()
        captures = parse(text, default_year=2026)

        # "terminal length 0" has no output of its own (just a blank line before
        # the auto-printed XR banner), so it shouldn't produce a capture.
        assert "terminal length 0" not in {c.command for c in captures}


class TestIos:
    def test_uses_show_clock_date(self):
        text = (FIXTURES / "sample_ios.txt").read_text()
        captures = parse(text)

        by_command = {c.command: c for c in captures}
        assert by_command["show running-config"].date == date(2026, 10, 19)
        assert "hostname router-ios" in by_command["show running-config"].text
        assert by_command["show version"].hostname == "router-ios"


class TestNxos:
    def test_splits_correctly(self):
        text = (FIXTURES / "sample_nxos.txt").read_text()
        # Pin the fallback date explicitly: the "show clock" capture's own
        # `.date` reflects whatever was current *before* its own banner was
        # parsed (there's no earlier banner in this fixture), so leaving this
        # to `date.today()` would make the test flaky across real days.
        captures = parse(text, fallback_date=date(2026, 10, 19))

        by_command = {c.command: c for c in captures}
        assert set(by_command) == {"show clock", "show running-config"}

        clock = by_command["show clock"]
        assert clock.hostname == "switch-nxos"
        assert clock.date == date(2026, 10, 19)
        assert "Mon Oct 19 2026" in clock.text

        running_config = by_command["show running-config"]
        assert running_config.hostname == "switch-nxos"
        assert running_config.date == date(2026, 10, 19)
        assert "interface Ethernet1/1" in running_config.text


class TestGenericPromptEdgeCases:
    def test_ignores_banner_like_lines_in_output(self):
        text = "\n".join(
            [
                "r1#show running-config",
                "banner motd ^C Welcome to r1#Restricted access^C",
                "end",
                "r1#",
            ]
        )
        captures = parse(text, fallback_date=date(2026, 1, 1))

        assert len(captures) == 1
        assert "Welcome to r1#Restricted access" in captures[0].text
