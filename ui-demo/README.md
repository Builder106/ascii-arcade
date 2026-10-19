# ASCII Arcade UI demo

<p align="center">
  <a href="ascii-arcade-ui-demo.mp4"><img src="preview.gif" alt="A Mac desktop running ASCII Arcade: Helix and then Matrix picked from the menu bar's ◎ menu, with the Dock below" width="800"></a>
</p>

<p align="center"><a href="ascii-arcade-ui-demo.mp4"><b>Watch the full video (41 s, with sound)</b></a></p>

The real macOS app as a wallpaper: the five scenes, the four themes, and DOOM
running its own demo, each picked from the ◎ menu. Nothing here is mocked up.
The wallpaper is the app's window recorded on its own, and the menu bar, Dock
and ◎ menu are stills captured from the same Mac.

## How it's made

`scripts/record-mac-wallpaper.sh` records the take on a Mac with the app
running. It presses the ◎ menu's items through Accessibility, so the menu
never opens and no keys get sent, and `screencapture -l` grabs only the
wallpaper window, so nothing else on screen ends up in the video. Your scene,
theme and auto-cycle setting go back the way they were afterwards. It needs
Accessibility and Screen Recording permission for whatever runs it.

`scripts/capture-mac-desktop.sh` grabs the menu bar, the Dock and the ◎ menu
open in each state, with the next pick highlighted. It slides your windows
aside with Show Desktop while it works and puts them back after.

```console
bash scripts/record-mac-wallpaper.sh
bash scripts/capture-mac-desktop.sh
bash scripts/cut-video-clips.sh
```

The clips are cropped from the Mac's 16:10 screen to 16:9. The shot list is in
[STORYBOARD.md](STORYBOARD.md). Then render with HyperFrames:

```console
npx --yes hyperframes@0.8.116 check
npx --yes hyperframes@0.8.116 render --quality high --strict -o renders/ascii-arcade-ui-demo.mp4
```

The full-quality render stays out of git (`renders/` is ignored). The MP4
above is a 720p copy for GitHub.

## Music

"Pixel Peeker Polka - faster" by Kevin MacLeod (CC BY 4.0), in
`public/audio.mp3`. Keep the credit from `public/CREDITS.md` with anything you
publish. The menu blip and coin come from `scripts/video-sfx.py`.
