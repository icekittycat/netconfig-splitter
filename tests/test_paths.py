import pytest

from netconfig_splitter.paths import sanitize_path_component


class TestSanitizePathComponent:
    @pytest.mark.parametrize(
        "raw,expected",
        [
            ("show running-config", "show running-config"),
            ("a/b:c*d", "a_b_c_d"),
            ("  trailing.  ", "trailing"),
            ("", "_"),
        ],
    )
    def test_sanitize(self, raw, expected):
        assert sanitize_path_component(raw) == expected
