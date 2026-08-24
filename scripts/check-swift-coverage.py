#!/usr/bin/env python3
"""Enforce coverage for the deterministic targets runnable on every platform."""

import json
import pathlib
import sys


TARGETS = {
    "Sources/AsciiArcadeCore/DonutFrameGenerator.swift",
}


def main() -> int:
    roots = [pathlib.Path(".build"), pathlib.Path("/home/ubuntu/work/verify/ascii-arcade/.build")]
    paths = []
    for root in roots:
        paths.extend(root.glob("**/debug/codecov/*.json"))
    paths = sorted(set(paths))
    if not paths:
        print("No Swift coverage JSON found", file=sys.stderr)
        return 1

    report = json.loads(paths[0].read_text())
    files = {}
    for entry in report["data"][0]["files"]:
        filename = entry["filename"]
        for target in TARGETS:
            if filename.endswith(target):
                files[target] = entry["summary"]

    missing = TARGETS - files.keys()
    if missing:
        print(f"Missing coverage entries: {', '.join(sorted(missing))}", file=sys.stderr)
        return 1

    failed = False
    for target in sorted(TARGETS):
        summary = files[target]
        lines = summary["lines"]["percent"]
        functions = summary["functions"]["percent"]
        print(f"{target}: lines {lines:.2f}%, functions {functions:.2f}%")
        failed |= lines < 100.0 or functions < 100.0
    return int(failed)


if __name__ == "__main__":
    sys.exit(main())
