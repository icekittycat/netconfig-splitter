# netconfig-splitter

Split combined IOS / IOS-XE / IOS-XR / NX-OS / ASA CLI capture files into
per-host, per-command output files.

The parser looks for device prompts (including IOS-XR's `RP/0/RSP1/CPU0:host#`
style and ASA multi-context `host/context#` prompts) to figure out which
device/command each block of output belongs to, and for timestamp banners
(`show clock` output, or IOS-XR's auto-printed `Mon Oct 19 02:45:08.742 UTC`)
to figure out which date to stamp it with. It also cleans up common terminal
capture artifacts: ANSI escapes, backspace erasures, `--More--` pagination
prompts, and stray queued-command echoes/overwrites left behind when a
scripted capture tool sends a batch of commands before reading any output
back. Each output file starts with a small header (the original prompt line,
and the device's timestamp banner if one was printed) followed by the
command's actual output.

The parsing/writing core is implemented in Rust, split into focused modules
under [src/](src/) (`date/`, `clean.rs`, `prompt/`, `paths.rs`,
`capture.rs`, `parser.rs`, `writer.rs`, wired together in `lib.rs`), and
exposed to Python via [pyo3](https://pyo3.rs)/[maturin](https://www.maturin.rs)
as the `netconfig_splitter._core` extension module. The Python package
under [src/netconfig_splitter/](src/netconfig_splitter/) mirrors that
module layout and is a thin, stable wrapper around it for the CLI and tests.

## Install

```sh
uv venv
uv pip install maturin pytest
maturin develop  # builds the Rust extension and installs the package in-place
```

Or simply `make develop` (uses `uv` under the hood — see [Makefile](Makefile)).

## Usage

```sh
netconfig-splitter capture1.txt capture2.txt -o output
```

Writes `output/<hostname>/<command>/<yyyy-mm-dd>.txt` by default. If the same
host/command/date combination occurs more than once, later occurrences get a
numeric suffix (`-2`, `-3`, ...) so no data is lost.

Options:

- `-o, --output`: output directory (default: `./output`)
- `--flat`: write `output/<hostname>/<command>.txt` instead, with no per-date
  subfolder (collisions still get a numeric suffix)
- `--vendor {cisco,juniper}`: restrict prompt recognition to one vendor
  instead of trying all known vendors (useful if a capture happens to
  contain a prompt-like line that's ambiguous across vendors)
- `--prompt-regex REGEX`: match prompts with a fully custom regex instead
  of any built-in vendor pattern, for syntaxes not yet supported. Must have
  a named capture group `host`; a `cmd` group is optional. Mutually
  exclusive with `--vendor`.

## Timestamp formats

Date banners are recognized on a whole-line basis (plus any stray leading
garbage from queued-command echoes or syslog's node-id prefix, see below) -
a timestamp followed by other content on the same line is treated as
ordinary command output, not a banner, so real output is never swallowed.

Cisco's "show clock" output always has the year and is recognized as-is
(e.g. `*02:45:08.742 UTC Mon Oct 19 2026`). IOS-XR's year-less banner/syslog
timestamp is more variable - its shape depends on the device's
`service timestamps` configuration:

| Format example | `service timestamps` config |
| --- | --- |
| `Mon Oct 19 02:45:08.742 UTC` | auto-printed command banner (weekday + msec + timezone) |
| `Oct 27 20:33:15` | `log datetime` (default: no msec, no timezone) |
| `Oct 27 20:33:15.826 UTC` | `log datetime msec show-timezone` |
| `RP/0/RSP0/CPU0:Oct 27 20:33:15.826 :` | raw syslog line, prefixed with the active RP's node id |

The weekday, milliseconds, and timezone are all optional in the regex that
matches these; the node-id prefix (`RP/0/RSP0/CPU0:`, `RP/0/RP0/CPU0:`,
etc. - the path segment names vary by chassis/role) isn't pattern-matched
at all, since it's already tolerated as the same kind of leading garbage as
a queued-command echo.

**Known limitation**: `service timestamps log uptime` replaces the date
entirely with elapsed router uptime (e.g. `00:05:23:`) - there is no real
calendar date in that format, so such lines are intentionally left
unrecognized and the parser falls back to `fallback_date` instead.

## Development

```sh
make develop   # build the Rust extension and install it into .venv
make test      # build, then run both `cargo test` and `pytest`
make fmt       # cargo fmt
make lint      # cargo clippy
```

