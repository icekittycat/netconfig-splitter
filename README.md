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
under [src/](src/) (`date.rs`, `clean.rs`, `prompt.rs`, `paths.rs`,
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

## Development

```sh
make develop   # build the Rust extension and install it into .venv
make test      # build, then run both `cargo test` and `pytest`
make fmt       # cargo fmt
make lint      # cargo clippy
```

