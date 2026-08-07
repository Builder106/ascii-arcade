#!/usr/bin/env python3
"""Record DOOM's attract-mode loop as plain character frames for the site.

Runs on ampere-dev, after scripts/setup.sh has built bin/doom_ascii. The output
is text: nothing GPL is redistributed, because doom_ascii itself never leaves
the build machine. Only the characters it drew do.

doom_ascii repaints by homing the cursor, so the stream splits into frames on
the home sequence. Colour is dropped: the cold open renders into a <pre> in the
theme's own colour, so per-cell colour would be thrown away anyway.

Usage: python3 scripts/record-doom.py [seconds] [output]
"""

import json
import os
import pty
import re
import select
import signal
import sys
import time

ANSI = re.compile(r"\x1b\[[0-9;?]*[A-Za-z]")
# doom_ascii homes the cursor with an empty first parameter, "\x1b[;H", rather
# than the more usual "\x1b[H" or "\x1b[1;1H". Match the general form or the
# stream never splits into frames at all.
HOME = re.compile(r"\x1b\[[0-9;]*H|\x1b\[2J")

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BIN = os.path.join(ROOT, "bin", "doom_ascii")
WAD = os.path.join(ROOT, "wad", "freedoom1.wad")

SECONDS = float(sys.argv[1]) if len(sys.argv) > 1 else 8.0
OUT = sys.argv[2] if len(sys.argv) > 2 else os.path.join(ROOT, "site/assets/doom-attract.json")
FPS = 12
# The cold open is a short loop behind a headline, not a video. Four seconds
# reads as "this is running" and keeps the autoplaying payload small; the grid
# is cropped for the same reason.
LOOP_SECONDS = 4
MAX_COLS = 80
MAX_ROWS = 24


BOOT_MARKERS = ("Init", "W_Init", "Z_Init", "adding ", "saving config")


def clean(chunk: str) -> list[str]:
    """One frame as a list of rows, ANSI stripped and clipped to the grid."""
    rows = [ANSI.sub("", line).rstrip() for line in chunk.split("\n")]
    rows = [r[:MAX_COLS] for r in rows if r.strip()]
    return rows[:MAX_ROWS]


def is_gameplay(rows: list[str]) -> bool:
    """True once DOOM is painting rather than logging its startup."""
    if len(rows) < 15:
        return False
    head = "\n".join(rows[:6])
    return not any(m in head for m in BOOT_MARKERS)


def main() -> int:
    if not os.path.exists(BIN):
        print(f"missing {BIN}; run scripts/setup.sh first", file=sys.stderr)
        return 1
    if not os.path.exists(WAD):
        print(f"missing {WAD}", file=sys.stderr)
        return 1

    pid, fd = pty.fork()
    if pid == 0:
        os.environ["TERM"] = "xterm-256color"
        os.execv(BIN, [BIN, "-iwad", WAD])
        os._exit(1)

    buf = ""
    frames: list[list[str]] = []
    deadline = time.time() + SECONDS

    try:
        while time.time() < deadline:
            r, _, _ = select.select([fd], [], [], 0.2)
            if not r:
                continue
            try:
                data = os.read(fd, 65536)
            except OSError:
                break
            if not data:
                break
            buf += data.decode("utf-8", "replace")

            parts = HOME.split(buf)
            buf = parts.pop()
            for part in parts:
                rows = clean(part)
                # DOOM spends the first several seconds logging its startup.
                # Nothing is kept until it is actually painting the screen.
                if is_gameplay(rows):
                    frames.append(rows)
    finally:
        os.kill(pid, signal.SIGKILL)
        os.waitpid(pid, 0)
        os.close(fd)

    if not frames:
        print("captured no frames", file=sys.stderr)
        return 1

    # Drop consecutive repeats first. DOOM sits on its title screen for a
    # while after boot, and a loop whose first quarter is a frozen image reads
    # as a broken hero rather than a running one. Also compresses better.
    moving = [f for i, f in enumerate(frames) if i == 0 or f != frames[i - 1]]
    if len(moving) >= FPS:
        frames = moving

    # Keep a bounded, evenly sampled window rather than everything captured.
    want = int(LOOP_SECONDS * FPS)
    if len(frames) > want:
        step = len(frames) / want
        frames = [frames[int(i * step)] for i in range(want)]

    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    payload = {"fps": FPS, "frames": ["\n".join(f) for f in frames]}
    with open(OUT, "w") as fh:
        json.dump(payload, fh, separators=(",", ":"))

    size = os.path.getsize(OUT)
    print(f"wrote {len(frames)} frames, {size // 1024} kB -> {OUT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
