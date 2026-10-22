"""Making arbitrary strings safe to use as filesystem path segments.

Implemented in Rust (see ``src/paths.rs``), re-exported here for a stable
Python API.
"""

from __future__ import annotations

from . import _core

sanitize_path_component = _core.sanitize_path_component
