"""The parsed result of one command's output, captured from a single device."""

from __future__ import annotations

from dataclasses import dataclass
from datetime import date


@dataclass
class Capture:
    """One command's output, captured from a single device."""

    hostname: str
    command: str
    date: date
    text: str
