#!/usr/bin/env python3
"""Record DOOM's attract-mode loop with full 24-bit RGB truecolor for the site.

Runs on ampere-dev, after scripts/setup.sh has built bin/doom_ascii. Output
is text with inline HTML color spans, so nothing GPL is redistributed:
doom_ascii itself never leaves the build machine. Only the colored characters
it drew do.

Colors are captured as-is. #doomFrame is aria-hidden and excluded from the
axe suite (it is decorative, and every fact it illustrates is also in page
copy), so there is no per-glyph contrast floor to hit here — an earlier draft
lifted every color's luminance into a narrow band to chase one anyway, which
just flattened DOOM's actual palette into a wash of pastel midtones.

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

SECONDS = float(sys.argv[1]) if len(sys.argv) > 1 else 15.0
OUT = sys.argv[2] if len(sys.argv) > 2 else os.path.join(ROOT, "site/assets/doom-attract.json")
# -scaling 2 makes doom_ascii emit DOOMGENERIC_RESX=160, RESY=100 cells (see
# dg_Create() in doomgeneric.c: RESX = 320/scaling). "-chars block" prints two
# block characters per cell, so the terminal stream is 320 columns by 100
# rows. MAX_COLS/MAX_ROWS must cover that or capture crops the frame instead
# of showing all of it, which looks identical to "too zoomed in" but for a
# different reason: not enough native detail vs. throwing detail away.
FPS = 8
LOOP_SECONDS = 3
MAX_COLS = 320
MAX_ROWS = 100

BOOT_MARKERS = ("Init", "W_Init", "Z_Init", "adding ", "saving config")


def ansi_to_row_runs(line: str) -> list[tuple[str | None, int]]:
    runs: list[tuple[str | None, int]] = []
    current_color = None
    count_blocks = 0
    col_count = 0

    pos = 0
    while pos < len(line) and col_count < MAX_COLS:
        match = SGR_RE.search(line, pos)
        if not match:
            text_len = len(line) - pos
            take = min(text_len, MAX_COLS - col_count)
            count_blocks += take
            col_count += take
            break

        text_len = match.start() - pos
        if text_len > 0:
            take = min(text_len, MAX_COLS - col_count)
            count_blocks += take
            col_count += take
            if col_count >= MAX_COLS:
                break

        codes = match.group(1).split(";")
        if codes in ([""], ["0"]):
            if count_blocks > 0:
                runs.append((current_color, count_blocks))
                count_blocks = 0
            current_color = None
        elif len(codes) >= 5 and codes[0] == "38" and codes[1] == "2":
            try:
                r, g, b = int(codes[2]), int(codes[3]), int(codes[4])
                new_color = f"#{r:02x}{g:02x}{b:02x}"
                if new_color != current_color:
                    if count_blocks > 0:
                        runs.append((current_color, count_blocks))
                        count_blocks = 0
                    current_color = new_color
            except ValueError:
                pass
        pos = match.end()

    if count_blocks > 0:
        runs.append((current_color, count_blocks))

    return runs


def clean(chunk: str) -> tuple[list[str], list[list[tuple[str | None, int]]]]:
    raw_lines = chunk.split("\n")
    plain_rows = []
    frame_runs = []
    for line in raw_lines:
        plain = ANSI_STRIP.sub("", line).rstrip()
        if plain.strip():
            plain_rows.append(plain[:MAX_COLS])
            frame_runs.append(ansi_to_row_runs(line))
            if len(plain_rows) >= MAX_ROWS:
                break
    return plain_rows, frame_runs


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
        os.execv(BIN, [BIN, "-scaling", "2", "-chars", "block", "-fixgamma", "2", "-iwad", WAD])
        os._exit(1)

    buf = ""
    frames: list[tuple[list[str], list[list[tuple[str | None, int]]]]] = []
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
                plain_rows, frame_runs = clean(part)
                if is_gameplay(plain_rows):
                    frames.append((plain_rows, frame_runs))
    finally:
        os.kill(pid, signal.SIGKILL)
        os.waitpid(pid, 0)
        os.close(fd)

    if not frames:
        print("captured no frames", file=sys.stderr)
        return 1

    moving = [f for i, f in enumerate(frames) if i == 0 or f[0] != frames[i - 1][0]]
    if len(moving) >= FPS:
        frames = moving

    want = int(LOOP_SECONDS * FPS)
    if len(frames) > want:
        frames = frames[-want:]

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
    for _, frame_runs in frames:
        encoded_frame = []
        for r_idx, row in enumerate(frame_runs):
            if r_idx > 0:
                encoded_frame.append([-1, 1])
            for color, count in row:
                c_idx = get_color_index(color)
                encoded_frame.append([c_idx, count])
        encoded_frames.append(encoded_frame)

    # cols/rows travel with the data rather than being assumed client-side: a
    # hardcoded column count the client trusts is exactly the kind of
    # magic-number coupling that breaks silently the next time -scaling
    # changes. Computed from the first frame's first row and total row count,
    # which is representative since doom_ascii's grid is fixed for the run.
    first_frame = encoded_frames[0]
    first_row_end = next(
        (i for i, (c, _) in enumerate(first_frame) if c == -1), len(first_frame)
    )
    cols = sum(n for c, n in first_frame[:first_row_end] if c != -1)
    rows = sum(1 for c, _ in first_frame if c == -1) + 1

    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    payload = {
        "fps": FPS,
        "cols": cols,
        "rows": rows,
        "palette": palette,
        "frames": encoded_frames,
    }
    with open(OUT, "w") as fh:
        json.dump(payload, fh, separators=(",", ":"))

    size = os.path.getsize(OUT)
    print(f"wrote {len(frames)} frames, {size // 1024} kB -> {OUT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
