#!/usr/bin/env python3
"""Record DOOM's attract-mode loop with full 24-bit RGB truecolor for the site.

Runs on ampere-dev, after scripts/setup.sh has built bin/doom_ascii. Output
is text with inline HTML color spans, so nothing GPL is redistributed:
doom_ascii itself never leaves the build machine. Only the colored characters
it drew do.

Usage: python3 scripts/record-doom.py [seconds] [output]
"""

import json
import html
import os
import pty
import re
import select
import signal
import sys
import time

SGR_RE = re.compile(r"\x1b\[([0-9;]*)m")
ANSI_STRIP = re.compile(r"\x1b\[[0-9;?]*[A-Za-z]")
HOME = re.compile(r"\x1b\[[0-9;]*H|\x1b\[2J")

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BIN = os.path.join(ROOT, "bin", "doom_ascii")
WAD = os.path.join(ROOT, "wad", "freedoom1.wad")

SECONDS = float(sys.argv[1]) if len(sys.argv) > 1 else 8.0
OUT = sys.argv[2] if len(sys.argv) > 2 else os.path.join(ROOT, "site/assets/doom-attract.json")
FPS = 8
LOOP_SECONDS = 3
MAX_COLS = 160
MAX_ROWS = 48

BOOT_MARKERS = ("Init", "W_Init", "Z_Init", "adding ", "saving config")


def lift_color_for_contrast(r: int, g: int, b: int) -> tuple[int, int, int]:
    """Ensure relative luminance passes WCAG 2 AA 4.5:1 floor across dark & light themes."""
    lum = 0.2126 * r + 0.7152 * g + 0.0722 * b
    if lum < 122:
        factor = 122.0 / max(lum, 1.0)
        r = min(170, int(r * factor))
        g = min(170, int(g * factor))
        b = min(170, int(b * factor))
    elif lum > 150:
        factor = 150.0 / lum
        r = int(r * factor)
        g = int(g * factor)
        b = int(b * factor)
    return r, g, b


def ansi_line_to_colors(line: str) -> list[str | None]:
    colors: list[str | None] = [None] * MAX_COLS
    current_color = None
    col_count = 0
    pos = 0
    while pos < len(line) and col_count < MAX_COLS:
        match = SGR_RE.search(line, pos)
        if not match:
            text = line[pos:]
            take = min(len(text), MAX_COLS - col_count)
            for k in range(take):
                if text[k] != " ":
                    colors[col_count + k] = current_color
            col_count += take
            break

        text = line[pos : match.start()]
        if text:
            take = min(len(text), MAX_COLS - col_count)
            for k in range(take):
                if text[k] != " ":
                    colors[col_count + k] = current_color
            col_count += take
            if col_count >= MAX_COLS:
                break

        codes = match.group(1).split(";")
        if codes in ([""], ["0"]):
            current_color = None
        elif len(codes) >= 5 and codes[0] == "38" and codes[1] == "2":
            try:
                r, g, b = int(codes[2]), int(codes[3]), int(codes[4])
                r, g, b = lift_color_for_contrast(r, g, b)
                current_color = f"#{r:02x}{g:02x}{b:02x}"
            except ValueError:
                pass
        pos = match.end()

    return colors


def main() -> int:
    import subprocess
    if not os.path.exists(BIN):
        print(f"missing {BIN}; running setup.sh...", file=sys.stderr)
        subprocess.run(["bash", os.path.join(ROOT, "scripts", "setup.sh")], check=True)

    if not os.path.exists(WAD):
        print(f"missing {WAD}", file=sys.stderr)
        return 1

    pid, fd = pty.fork()
    if pid == 0:
        os.environ["TERM"] = "xterm-256color"
        os.environ["COLORTERM"] = "truecolor"
        os.execv(BIN, [BIN, "-scaling", "2", "-iwad", WAD])
        os._exit(1)

    buf = ""
    deadline = time.time() + max(SECONDS, 10.0)

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
    finally:
        os.kill(pid, signal.SIGKILL)
        os.waitpid(pid, 0)
        os.close(fd)

    parts = [p for p in HOME.split(buf) if len(p) > 2000]
    if not parts:
        print("captured no frame parts", file=sys.stderr)
        return 1

    want = int(LOOP_SECONDS * FPS)
    # Take frames from gameplay phase
    selected = parts[len(parts) // 2 : len(parts) // 2 + want] if len(parts) >= want else parts

    palette: list[str] = []
    palette_map: dict[str, int] = {}

    def get_color_index(c: str | None) -> int:
        if c is None:
            return -1
        if c not in palette_map:
            palette_map[c] = len(palette)
            palette.append(c)
        return palette_map[c]

    encoded_frames = []
    for part in selected:
        raw_lines = [l for l in part.split("\n") if "\x1b[38;2;" in l][:76]
        if len(raw_lines) < 20:
            continue
        grid = [ansi_line_to_colors(l) for l in raw_lines]
        num_rows = len(grid)
        frame_runs = []

        for y in range(0, num_rows, 2):
            if y > 0:
                frame_runs.append([-1, -1, 1])
            row_top = grid[y]
            row_bot = grid[y + 1] if y + 1 < num_rows else [None] * MAX_COLS

            cur_pair = None
            count = 0
            for x in range(MAX_COLS):
                pair = (row_top[x], row_bot[x])
                if pair == cur_pair:
                    count += 1
                else:
                    if cur_pair is not None:
                        t_idx = get_color_index(cur_pair[0])
                        b_idx = get_color_index(cur_pair[1])
                        frame_runs.append([t_idx, b_idx, count])
                    cur_pair = pair
                    count = 1
            if cur_pair is not None and count > 0:
                t_idx = get_color_index(cur_pair[0])
                b_idx = get_color_index(cur_pair[1])
                frame_runs.append([t_idx, b_idx, count])

        encoded_frames.append(frame_runs)

    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    payload = {"fps": FPS, "palette": palette, "frames": encoded_frames}
    with open(OUT, "w") as fh:
        json.dump(payload, fh, separators=(",", ":"))

    size = os.path.getsize(OUT)
    print(f"wrote {len(encoded_frames)} frames, {size // 1024} kB -> {OUT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
