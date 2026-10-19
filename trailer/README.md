# ASCII Arcade Tier 3 trailer

The Tier 3 video: a 39.5-second, 1920×1080 trailer built in HyperFrames.
It borrows the site's idea. The cabinet plays its own demo like an arcade
machine nobody's using: power-on, the cabinet, the wallpaper under your
windows, the scenes, DOOM, and the snap-back when you come back.

Anything on the cabinet or the wall is real footage of the site. The Mac
desktop and the end card are HTML and GSAP.

## Footage

The clips in `public/clips/` are cut from takes recorded by
`e2e/capture/footage.mjs`. It steps the page on a fake clock and screenshots
every frame, so you get a clean 30 fps take on any machine. To record again
after a site change, build `site/pkg/` and `site/doom-wasm/` first, then:

```console
cd e2e && npm run footage
cd .. && bash scripts/cut-video-clips.sh
```

The cut list for both videos lives in `scripts/cut-video-clips.sh`.

## Render

```console
npx --yes hyperframes@0.8.116 check
npx --yes hyperframes@0.8.116 render --quality high --strict -o renders/ascii-arcade-trailer.mp4
```

Don't commit the render.

## Music

`public/audio.mp3` is "Pixelland" by Kevin MacLeod (CC BY 4.0). Keep the
credit from `public/CREDITS.md` with anything you publish. The sound effects
come from `scripts/video-sfx.py`.
