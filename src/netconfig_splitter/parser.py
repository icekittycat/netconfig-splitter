"""Parsing combined Cisco CLI capture files into per-command `Capture`s.

The heavy lifting (prompt/timestamp parsing) is implemented in Rust (see
``src/parser.rs``, compiled as the ``netconfig_splitter._core`` extension
module via pyo3/maturin). This module adapts it to a small, stable Python API.
"""

from __future__ import annotations

from datetime import date

from . import _core
from .capture import Capture

normalize_command = _core.normalize_command


def parse(
    text: str,
    default_year: int | None = None,
    fallback_date: date | None = None,
    normalize_commands: bool = True,
    vendor: str | None = None,
    prompt_regex: str | None = None,
) -> list[Capture]:
    """Parse a raw capture file into a list of per-command `Capture`s.

    By default, prompt recognition tries every known vendor (Cisco,
    Juniper) in turn. Pass `vendor` ("cisco" or "juniper") to restrict
    recognition to just one, or `prompt_regex` to match a fully custom
    pattern instead (must have a named group `host`; `cmd` is optional).
    At most one of `vendor`/`prompt_regex` may be given.
    """
    year_hint = default_year or date.today().year
    fallback_iso = (fallback_date or date.today()).isoformat()

    raw_captures = _core.parse(text, year_hint, fallback_iso, normalize_commands, vendor, prompt_regex)
    return [
        Capture(c.hostname, c.command, date.fromisoformat(c.date), c.text)
        for c in raw_captures
    ]
