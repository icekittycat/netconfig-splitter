"""Command-line interface for netconfig-splitter."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from .parser import parse
from .writer import write_captures


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="netconfig-splitter",
        description=(
            "Split combined IOS/IOS-XE/IOS-XR/NX-OS CLI capture files into "
            "output/<hostname>/<command>/<yyyy-mm-dd>.txt"
        ),
    )
    parser.add_argument("inputs", nargs="+", type=Path, help="Capture file(s) to split")
    parser.add_argument(
        "-o", "--output", type=Path, default=Path("output"),
        help="Output directory (default: ./output)",
    )
    parser.add_argument(
        "--flat", action="store_true",
        help=(
            "Write output/<hostname>/<command>.txt instead of the default "
            "output/<hostname>/<command>/<yyyy-mm-dd>.txt"
        ),
    )
    vendor_group = parser.add_mutually_exclusive_group()
    vendor_group.add_argument(
        "--vendor", choices=["cisco", "juniper"],
        help="Restrict prompt recognition to one vendor (default: try all known vendors)",
    )
    vendor_group.add_argument(
        "--prompt-regex",
        help=(
            "Match prompts with a custom regex instead of any built-in vendor "
            "pattern, for syntaxes not yet supported. Must have a named group "
            "'host'; a 'cmd' group is optional."
        ),
    )
    args = parser.parse_args(argv)

    total_written = 0
    for input_path in args.inputs:
        if not input_path.is_file():
            print(f"error: {input_path}: no such file", file=sys.stderr)
            return 1

        text = input_path.read_text(encoding="utf-8", errors="replace")
        try:
            captures = parse(text, vendor=args.vendor, prompt_regex=args.prompt_regex)
        except ValueError as e:
            print(f"error: {e}", file=sys.stderr)
            return 1

        if not captures:
            print(f"{input_path}: no device prompts/commands recognized", file=sys.stderr)
            continue

        written = write_captures(captures, args.output, flat=args.flat)
        for path in written:
            print(path)
        total_written += len(written)

    print(f"wrote {total_written} file(s) to {args.output}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

