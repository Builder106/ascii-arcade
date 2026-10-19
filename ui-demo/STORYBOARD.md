# Tier 2 social cut storyboard

Built in HyperFrames. `AsciiArcadeUiDemo` is 29.5 seconds at 1080×1920,
30 fps. The real site runs on a phone, captions go in the band above it, and
the scenes play dimmed behind it like a wallpaper. Cuts are on the beats of
"Pixel Peeker Polka - faster".

| Time | Beat | Source |
| --- | --- | --- |
| 0.00-8.75 | Power-on, boot log, the cabinet pulls out; the demo loop starts | `phone-attract` take, 0.00 s |
| 8.75-14.12 | A tap takes over (PLAYER 1), swipes step to DOOM and Scenes | `phone-player` take, 2.90 s |
| 14.12-20.97 | Scene and theme buttons repaint the cabinet | `phone-player` take, 9.30 s |
| 20.97-25.94 | The DOOM station, close on the screen | `phone-attract` take, 12.30 s |
| 25.94-29.50 | End card, INSERT COIN | `compositions/end.html` |

Behind the phone is the `wall` take from 0 s, cropped to the slice that
shows. Captions are in `compositions/captions.html`.

Sound: `audio.mp3` from the start, fading out over the last 1.5 s, plus the
power-on, a whoosh on each of the two swipes, and a coin under INSERT COIN.
Credit the music the way `public/CREDITS.md` says when you publish.
