"""ASCII Arcade trailer cue, composed from ORIGINAL-CUE.md.

Render with `lmms-cue render music/cue.py` (see SankofaForge/lmms-cue).
115 BPM in 4/4, so one beat is 0.5217 s. Every cut in the montage is two beats.
The cue starts at 2.84 s of picture (T0), so the cut to the desktop at 7.01 s
falls on bar 3 and the montage cuts fall on beats 2 and 4. Mux it with a
2.84 s delay.
"""

from lmms_cue import Song, effect, reverb, sf2

BPM = 115
T0 = 2.84
END = 39.5
song = Song(bpm=BPM, length=END - T0, fade_out=(38.1 - T0, END - T0),
            master_fx=[
                reverb(size=0.55, color=6500, wet=0.12),
                # A limiter keeps peaks out of LMMS's 16-bit clip and lets the
                # render's loudness normalisation stay linear.
                effect("compressor", "CompressorControls", threshold=-16, ratio=20, attack=0.5,
                       release=120, knee=2, lookahead=1, lookaheadLength=5, peakmode=1, limiter=1),
            ])


def at(seconds: float, grid: float = 1.0) -> float:
    """Beat nearest a picture time, on a grid of `grid` beats."""
    return round(song.beat(seconds - T0) / grid) * grid


# GM drum keys on FluidR3's TR-808 kit.
KICK, SNARE, CLAP, HAT, OPEN_HAT, CRASH, TOM_LO, TOM_HI = 36, 38, 39, 42, 46, 49, 45, 50

drums = song.track("Drums", sf2(25, bank=128, gain=3), volume=100)
bass = song.track("Synth bass", sf2(38, gain=3), volume=85)
riff = song.track("Riff", sf2(39, gain=3), volume=85)
arp = song.track("Arp", sf2(80, gain=2), volume=42, pan=-25)
lead = song.track("Lead", sf2(81, gain=2), volume=48, pan=25)
pad = song.track("Pad", sf2(89, gain=3), volume=55)

PROG = ["Em", "C", "G", "D"]
ROOT = {"Em": "E2", "C": "C2", "G": "G2", "D": "D2"}
TRIAD = {"Em": ["E3", "G3", "B3"], "C": ["C3", "E3", "G3"],
         "G": ["G3", "B3", "D4"], "D": ["D3", "F#3", "A3"]}
ARP = {"Em": ["E4", "G4", "B4", "E5"], "C": ["C4", "E4", "G4", "C5"],
       "G": ["G3", "B3", "D4", "G4"], "D": ["D4", "F#4", "A4", "D5"]}

# Picture times in cue beats: 7.01 s = 8, 13.79 s = 21, 20.04 s = 33,
# 21.62 s = 36, 27.36 s = 47, 31.53 s = 55, 33.62 s = 59, 35.20 s = 62.
WALL, MONTAGE, GLASS, DOOM, IDLE, SNAP, END_CARD, FINAL = 8, 21, 33, 36, 47, 55, 59, 64

# Two-bar phrase for the DOOM section: E pedal with the flat second as menace.
CHUG_A = [0, 0, None, 0, 0, None, 0, 0, None, 0, 0, None, 1, None, 0, None]
CHUG_B = [0, 0, None, 0, 0, None, 0, 0, None, 0, 0, None, 3, None, 5, 7]
KICK_16 = [1, 0, 0, 1, 0, 0, 1, 0, 1, 0, 0, 0, 1, 0, 0, 1]

MOTIF = [("B4", 1), ("E5", 1), ("G5", 1.5), ("F#5", 0.5), ("E5", 2)]


def motif(start: float, velocity: int = 70, octave_up: bool = False):
    beat = start
    for note, length in MOTIF:
        name = note[:-1] + str(int(note[-1]) + 1) if octave_up else note
        lead.note(beat, length, name, velocity)
        beat += length


def semitone(root: str, up: int) -> int:
    from lmms_cue.song import key
    return key(root) + up


def section(beat: float) -> str:
    if beat < WALL:
        return "intro"
    if beat < MONTAGE:
        return "wall"
    if beat < GLASS:
        return "montage"
    if beat < DOOM:
        return "glass"
    if beat < IDLE:
        return "doom"
    if beat < SNAP:
        return "idle"
    if beat < SNAP + 1:
        return "snap"
    if beat < FINAL:
        return "end"
    return "final"


TOTAL_STEPS = int(song.beat(END - T0) * 4)

for i in range(TOTAL_STEPS):
    b = i / 4
    sec = section(b)
    bar = int(b // 4)
    chord = PROG[bar % 4]
    step = i % 16
    on_beat = i % 4 == 0
    off_eighth = i % 4 == 2
    eighth = i % 2 == 0
    arp_note = ARP[chord][i % 4]
    root = ROOT[chord]

    if sec == "intro":
        if b == 0:
            drums.note(b, 0.25, KICK, 100)
        if bar == 0 and eighth:
            arp.note(b, 0.4, arp_note, 48)
        if bar == 1:
            arp.note(b, 0.2, arp_note, 52)
            if eighth:
                bass.note(b, 0.4, root, 72)
            if off_eighth:
                drums.note(b, 0.2, HAT, 35)
        if b >= 7:
            drums.note(b, 0.2, SNARE, 45 + (i - 28) * 12)

    elif sec == "wall":
        if on_beat:
            drums.note(b, 0.25, KICK, 85)
        if off_eighth:
            drums.note(b, 0.2, HAT, 45)
        if eighth:
            bass.note(b, 0.4, root, 66)
        arp.note(b, 0.2, arp_note, 42)

    elif sec == "montage":
        if on_beat:
            drums.note(b, 0.25, KICK, 95)
            if int(b) % 4 in (1, 3):
                drums.note(b, 0.3, SNARE, 90)
        if eighth:
            drums.note(b, 0.15, HAT, 52)
            bass.note(b, 0.4, root if i % 8 != 6 else root[:-1] + str(int(root[-1]) + 1), 70)
        elif b >= 28:
            drums.note(b, 0.1, HAT, 42)
        arp.note(b, 0.2, arp_note, 50)
        if 30 <= b < 33:
            drums.note(b, 0.2, SNARE, min(120, int(50 + (b - 30) * 24)))

    elif sec == "glass":
        pass

    elif sec == "doom":
        pattern = CHUG_B if bar % 2 == 1 else CHUG_A
        up = pattern[step]
        if up is not None:
            riff.note(b, 0.28, semitone("E2", up), 125)
        if KICK_16[step]:
            drums.note(b, 0.15, KICK, 105)
        if step in (4, 12):
            drums.note(b, 0.3, SNARE, 100)
        if eighth:
            drums.note(b, 0.15, HAT, 55)
        if step == 14:
            drums.note(b, 0.3, OPEN_HAT, 60)
        if step in (0, 10) and b < 46:
            lead.chord(b, 2.5 if step == 0 else 0.75, ["E4", "B4", "E5"], 92)
        if b >= 46:
            drums.note(b, 0.15, SNARE, 80 + int((b - 46) * 40))

    elif sec == "idle":
        if b % 4 == 0:
            drums.note(b, 0.3, KICK, 70)
        elif b % 4 == 2:
            drums.note(b, 0.3, KICK, 50)
        if eighth:
            drums.note(b, 0.15, HAT, 38 if on_beat else 26)
        if b < 54.5:
            arp.note(b, 0.2, arp_note, 46)
        if eighth and b >= 48 and b < 54.5:
            bass.note(b, 0.4, root, 54)

    elif sec == "snap":
        pass

    elif sec in ("end", "final"):
        if sec == "end":
            if on_beat:
                drums.note(b, 0.25, KICK, 92)
                if int(b) % 4 in (1, 3):
                    drums.note(b, 0.3, SNARE, 88)
            if eighth:
                drums.note(b, 0.15, HAT, 56)
                bass.note(b, 0.4, root, 80)
            # The coin lands on beat 62 (35.2 s); keep that stretch clear of high notes.
            if not 61.75 <= b <= 63.0:
                arp.note(b, 0.2, arp_note, 56)

# Pad chords, one a bar: soft under the intro, wall, idle and end card.
for bar in range(0, 17):
    start = bar * 4
    chord = PROG[bar % 4]
    if start < MONTAGE or IDLE <= start < SNAP or END_CARD - 4 <= start < FINAL:
        length = 4
        if start == 44:
            length = 3
        pad.chord(start, length, TRIAD[chord], 56 if start < IDLE else (62 if start < SNAP else 56))

# Snap-back: a clap on beat 55 and nothing around it for the whoosh.
drums.note(SNAP, 0.3, CLAP, 90)

# Intro bass pedal, one note a bar, so the cabinet has a floor.
bass.note(0, 4, "E2", 62)

# Montage melody once the scenes are cycling, and again over the end card.
motif(MONTAGE + 3, 62)
motif(MONTAGE + 11, 66, octave_up=False)
motif(END_CARD + 0.5, 78)

# The glass: crash, a big low note, a tom fill into DOOM.
drums.note(GLASS, 1.5, CRASH, 105)
drums.note(GLASS, 0.5, KICK, 110)
bass.note(GLASS, 3, "E1", 110)
for step, tom in enumerate([TOM_LO, TOM_LO, TOM_HI, TOM_HI]):
    drums.note(35 + step * 0.25, 0.2, tom, 90 + step * 8)

# DOOM: strongest energy. Crash on the downbeat.
drums.note(DOOM, 1.5, CRASH, 118)

# DOOM pad: a sustained power chord under the riff.
for start in (36, 40, 44):
    pad.chord(start, 4 if start < 44 else 3, ["E3", "B3", "E4"], 70)

# Idle at the coin door: a slow pad line, and the bass holds each chord.
for bar, chord in [(12, "Em"), (13, "C")]:
    start = max(bar * 4, IDLE)
    bass.note(start, bar * 4 + 4 - start - (0.5 if bar == 13 else 0), ROOT[chord], 52)

# Final bar: power chord, crash, one sparkle at the top.
drums.note(FINAL, 3, CRASH, 105)
drums.note(FINAL, 0.5, KICK, 110)
bass.note(FINAL, 8, "E1", 100)
pad.chord(FINAL, 8, ["E3", "B3", "E4"], 55)
lead.chord(FINAL, 8, ["E4", "B4"], 58)
arp.note(FINAL + 1, 6, "E5", 40)
