# Tier 2 UI demo storyboard

Built in HyperFrames. `AsciiArcadeUiDemo` is 41 seconds at 1920×1080, 30 fps.
Every shot is the macOS app's wallpaper window from the `mac-wallpaper` take,
cropped to 16:9, under the real menu bar and Dock. Each shot opens 1.3 s
before a pick: the real ◎ menu drops open, the pointer moves to the
highlighted item, and the menu closes as the app's own cell dissolve starts.

| Time | Beat | Source |
| --- | --- | --- |
| 0.00-7.50 | Donut, then ◎ Helix | take, 2.00 s |
| 7.50-11.24 | ◎ Matrix | take, 11.10 s |
| 11.24-14.91 | ◎ Life | take, 17.10 s |
| 14.91-19.32 | ◎ Pipes | take, 23.10 s |
| 19.32-22.62 | ◎ Amber, on Matrix | take, 29.10 s |
| 22.62-25.94 | ◎ Ice, on Donut | take, 35.10 s |
| 25.94-29.23 | ◎ Ghost, on Helix | take, 41.10 s |
| 29.23-31.72 | ◎ DOOM: the dissolve onto the title | take, 54.10 s |
| 31.72-37.08 | DOOM melts into its own demo | take, 60.30 s |
| 37.08-41.00 | End card, INSERT COIN | `compositions/end.html` |

The menu bar, Dock, menus and pointer are in `compositions/desktop.html`,
the captions in `compositions/captions.html`.

Sound: `audio.mp3` from the start, fading out over the last 1.5 s, a menu
blip on each pick, and a coin under INSERT COIN. Credit the music the way
`public/CREDITS.md` says when you publish.
