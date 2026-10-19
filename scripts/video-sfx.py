#!/usr/bin/env python3
"""Synthesizes the sound effects for the HyperFrames videos in trailer/ and
ui-demo/: the cabinet powering on, a menu blip, a coin, and a camera whoosh.

They are square waves and noise, the way an arcade board makes them, so
they are generated here rather than sourced. The noise is seeded, so the
output is the same on every run.

    python3 scripts/video-sfx.py trailer/public ui-demo/public
"""

import math
import random
import struct
import sys
import wave
from pathlib import Path

RATE = 44100


def square(freq, t):
    return 1.0 if math.sin(2 * math.pi * freq * t) >= 0 else -1.0


def envelope(t, length, attack=0.004, release=0.05):
    if t < attack:
        return t / attack
    if t > length - release:
        return max(0.0, (length - t) / release)
    return 1.0


def power():
    """A CRT switching on: a low thump, a burst of static, the flyback settling."""
    rng = random.Random(1)
    length = 0.9
    out = []
    for i in range(int(RATE * length)):
        t = i / RATE
        thump = math.sin(2 * math.pi * (55 + 40 * math.exp(-t * 18)) * t) * math.exp(-t * 7)
        static = (rng.random() * 2 - 1) * math.exp(-t * 9) * 0.35
        whine = math.sin(2 * math.pi * 7800 * t) * 0.025 * min(1.0, t * 6) * math.exp(-t * 2.5)
        out.append((thump * 0.8 + static + whine) * envelope(t, length, 0.002, 0.2))
    return out


def blip():
    """The cabinet's button: two quick square tones."""
    length = 0.12
    return [square(880 if t < 0.05 else 1320, t) * 0.32 * envelope(t, length, 0.002, 0.03)
            for t in (i / RATE for i in range(int(RATE * length)))]


def coin():
    """A coin in the slot: B5, then a longer E6 that rings out."""
    length = 0.55
    out = []
    for i in range(int(RATE * length)):
        t = i / RATE
        if t < 0.08:
            out.append(square(987.77, t) * 0.3)
        else:
            out.append(square(1318.51, t) * 0.3 * math.exp(-(t - 0.08) * 6) * envelope(t, length, 0.002, 0.08))
    return out


def whoosh():
    """A camera move: noise through a low-pass that opens and closes."""
    rng = random.Random(2)
    length = 0.6
    out = []
    y = 0.0
    for i in range(int(RATE * length)):
        t = i / RATE
        shape = math.sin(math.pi * t / length)
        cutoff = 0.02 + 0.25 * shape
        y += cutoff * ((rng.random() * 2 - 1) - y)
        out.append(y * 1.6 * shape)
    return out


def write(path, samples):
    with wave.open(str(path), "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(RATE)
        w.writeframes(b"".join(struct.pack("<h", int(max(-1.0, min(1.0, s)) * 32767)) for s in samples))


def main(dirs):
    sounds = {"sfx_power.wav": power(), "sfx_blip.wav": blip(), "sfx_coin.wav": coin(), "sfx_whoosh.wav": whoosh()}
    for d in dirs:
        Path(d).mkdir(parents=True, exist_ok=True)
        for name, samples in sounds.items():
            write(Path(d) / name, samples)
            print(Path(d) / name)


if __name__ == "__main__":
    main(sys.argv[1:] or ["."])
