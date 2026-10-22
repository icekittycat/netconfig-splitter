"""Writing parsed captures out to disk.

The heavy lifting (path layout and de-duplication) is implemented in Rust
(see ``src/writer.rs``, compiled as the ``netconfig_splitter._core``
extension module via pyo3/maturin). This module adapts it to a small, stable
Python API.
"""

from __future__ import annotations

from pathlib import Path

from . import _core
from .capture import Capture


def write_captures(
    captures: list[Capture], output_dir: Path, flat: bool = False
) -> list[Path]:
    """Write captures to disk.

    By default, writes `output_dir/<hostname>/<command>/<yyyy-mm-dd>.txt`. If
    `flat` is True, writes `output_dir/<hostname>/<command>.txt` instead, with
    no per-date subfolder.

    Either way, if the same host/command (and, in dated mode, date) occurs
    more than once, later occurrences get a numeric suffix (`-2`, `-3`, ...)
    so no data is lost.
    """
    rows = [(c.hostname, c.command, c.date.isoformat(), c.text) for c in captures]
    paths = _core.write_captures(rows, str(output_dir), flat)
    return [Path(p) for p in paths]
