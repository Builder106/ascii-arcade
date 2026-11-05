# Original trailer music cue

Status: rendered from `music/cue.py` and in use. The trailer plays this cue.

## Composition

- Length: 36.66 s. It starts at 2.84 s of picture, where the boot log gives way
  to the cabinet. Tempo: 115 BPM. Meter: 4/4. Tonal center: E minor.
- Why 115 BPM: every montage cut is two beats long, and the cuts at 7.01, 13.79,
  20.04, 21.62, 27.36 and 33.62 s all fall on a beat once the cue starts at
  2.84 s. The montage cuts land on beats 2 and 4, where the snare is.
- Harmony: Em, C, G, D, a bar each. The glass moment and the final bar sit on
  low E.
- Motif: B4, E5, G5, F#5, E5 on a square-wave synth, stated once in the montage
  and again over the end card.
- Sound: a TR-808 drum kit, a synth bass, a square arpeggio, a saw lead and a
  warm pad. The DOOM section uses a palm-muted E riff with a flat second as
  menace. Everything comes from built-in LMMS sounds.

## Picture map

| Time | Picture beat | Music change |
| --- | --- | --- |
| 0.00-2.84 | Power-on and boot log | No music. The power-on effect plays alone. |
| 2.84-7.01 | The cabinet pulls out of the glass | Soft arpeggio and a low E pedal, then a bass pulse and a snare ghost-roll into the cut. |
| 7.01-13.79 | The wall behind two windows | Four-on-the-floor kick, offbeat hat, eighth-note bass. The menu blips at 8.90 and 10.00 s sit in the gaps. |
| 13.79-20.04 | Five scenes in four themes | Snare on every cut, sixteenth arpeggio, the motif, and a snare roll through the last bar. |
| 20.04-21.62 | The camera goes through the glass | Crash and a low E as the cut lands at 20.04 s, ahead of the whoosh at 20.42 s, then a tom fill into DOOM. |
| 21.62-27.36 | DOOM | The strongest part: E riff in sixteenths, kick and snare, saw stabs and a held power chord. |
| 27.36-31.53 | Idle at the coin door | Drums and bass drop to a breather: soft kick, hats, a quiet arpeggio and pad. |
| 31.53-33.62 | The loop snaps back | A clap on the snap sound at 31.53 s and a beat of space. The groove returns at 32.06 s. |
| 33.62-39.50 | End card | Groove and the motif return. Nothing high near the coin at 35.20 s. Resolve to an open E power chord on 36.23 s, hold, fade from 38.10 s. |

## Mix and export

`lmms-cue render` writes `cue.wav` (48 kHz), `cue.mp3` and `cue.report.json` at
-17 LUFS and -2 dBTP. A master limiter in `cue.py` keeps LMMS from clipping its
16-bit output and lets the loudness pass stay linear. `preview-new-cue.mp4`
puts this cue and the trailer's sound effects on the current video.

In use: `music/cue.mp3` is copied to `public/audio.mp3`, and the `music` clip
in `index.html` starts at 2.84 s for 36.66 s.
