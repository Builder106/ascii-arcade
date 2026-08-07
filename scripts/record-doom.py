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
FPS = 12
LOOP_SECONDS = 4
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


def ansi_to_html_row(line: str) -> str:
    row_html = ""
    current_color = None
    current_text = ""
    col_count = 0

    pos = 0
    while pos < len(line) and col_count < MAX_COLS:
        match = SGR_RE.search(line, pos)
        if not match:
            text = line[pos:]
            take = min(len(text), MAX_COLS - col_count)
            current_text += text[:take]
            col_count += take
            break

        text = line[pos : match.start()]
        if text:
            take = min(len(text), MAX_COLS - col_count)
            current_text += text[:take]
            col_count += take
            if col_count >= MAX_COLS:
                break

        codes = match.group(1).split(";")
        if codes in ([""], ["0"]):
            if current_text:
                esc = html.escape(current_text)
                row_html += f'<span style="color:{current_color}">{esc}</span>' if current_color else esc
                current_text = ""
            current_color = None
        elif len(codes) >= 5 and codes[0] == "38" and codes[1] == "2":
            try:
                r, g, b = int(codes[2]), int(codes[3]), int(codes[4])
                r, g, b = lift_color_for_contrast(r, g, b)
                # Convert to compact hex color (#rrggbb)
                new_color = f"#{r:02x}{g:02x}{b:02x}"
                if new_color != current_color:
                    if current_text:
                        esc = html.escape(current_text)
                        row_html += f'<span style="color:{current_color}">{esc}</span>' if current_color else esc
                        current_text = ""
                    current_color = new_color
            except ValueError:
                pass
        pos = match.end()

    if current_text:
        esc = html.escape(current_text)
        row_html += f'<span style="color:{current_color}">{esc}</span>' if current_color else esc

    return row_html


def clean(chunk: str) -> tuple[list[str], list[str]]:
    """Returns (plain_rows, html_rows) for a frame chunk."""
    raw_lines = chunk.split("\n")
    plain_rows = []
    html_rows = []
    for line in raw_lines:
        plain = ANSI_STRIP.sub("", line).rstrip()
        if plain.strip():
            plain_rows.append(plain[:MAX_COLS])
            html_rows.append(ansi_to_html_row(line))
            if len(plain_rows) >= MAX_ROWS:
                break
    return plain_rows, html_rows


def is_gameplay(rows: list[str]) -> bool:
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
        os.environ["COLORTERM"] = "truecolor"
        os.execv(BIN, [BIN, "-scaling", "4", "-chars", "block", "-iwad", WAD])
        os._exit(1)

    buf = ""
    frames: list[tuple[list[str], list[str]]] = []
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
                plain_rows, html_rows = clean(part)
                if is_gameplay(plain_rows):
                    frames.append((plain_rows, html_rows))
    finally:
        os.kill(pid, signal.SIGKILL)
        os.waitpid(pid, 0)
        os.close(fd)

    if not frames:
        print("captured no frames", file=sys.stderr)
        return 1

    # Drop consecutive repeats
    moving = [f for i, f in enumerate(frames) if i == 0 or f[0] != frames[i - 1][0]]
    if len(moving) >= FPS:
        frames = moving

    want = int(LOOP_SECONDS * FPS)
    if len(frames) > want:
        step = len(frames) / want
        frames = [frames[int(i * step)] for i in range(want)]

    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    payload = {"fps": FPS, "frames": ["\n".join(f[1]) for f in frames]}
    with open(OUT, "w") as fh:
        json.dump(payload, fh, separators=(",", ":"))

    size = os.path.getsize(OUT)
    print(f"wrote {len(frames)} frames, {size // 1024} kB -> {OUT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
