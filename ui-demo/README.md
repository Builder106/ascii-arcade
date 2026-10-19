# ASCII Arcade Tier 2 social cut

The Tier 2 video: a 29.5-second, 1080×1920 vertical cut of the site on a
phone, built in HyperFrames. It's cut from the same takes as the trailer in
`../trailer/`, and that README covers recording them again.

## Render

```console
npx --yes hyperframes@0.8.116 check
npx --yes hyperframes@0.8.116 render --quality high --strict -o renders/ascii-arcade-ui-demo.mp4
```

Don't commit the render.

## Music

`public/audio.mp3` is "Pixel Peeker Polka - faster" by Kevin MacLeod
(CC BY 4.0). Keep the credit from `public/CREDITS.md` with anything you
publish. The sound effects come from `scripts/video-sfx.py`.
