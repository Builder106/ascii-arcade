# E2E demo suite

Narrative [playwright-bdd](https://github.com/vitalets/playwright-bdd) recordings of
the **browser-DOOM bonus surface** (the Vapor server + xterm.js page). The wallpaper
app itself is a desktop overlay — record those scenes manually with a screen capture.

## Run

```bash
cd e2e
npm install            # installs Playwright + downloads Chromium
npm run demo           # builds + boots the Swift Server, records, transcodes
```

> **Environment notes**
> - The `webServer` command exports a `GIT_CONFIG_*` override for `safe.bareRepository`
>   so SwiftPM's package cache works even if you've set that git option globally.
> - SwiftPM's build database (`.build/build.db`) is a sqlite file and can throw
>   `disk I/O error` on network/synced filesystems (e.g. Google Drive / iCloud).
>   If you hit that, clone the repo to a local-disk path before recording.

`npm run demo` will:
1. start the server via the `webServer` block in `playwright.demo.config.ts`
   (it runs `scripts/setup.sh` to ensure `bin/doom_ascii` exists, then `swift run Server`),
2. run the demo scenarios with `DEMO=1` (slow-mo + dwell beats + a visible cursor),
3. transcode each scenario's `webm` → `mp4` into `e2e/recordings/` via the custom
   reporter (warmup + 0-byte clips are discarded).

## Tuning knobs

| Var | Default | Purpose |
|-----|---------|---------|
| `DEMO` | `0` | Master switch — dwell/cursor only fire when `1` |
| `DEMO_SLOWMO` | `1000` | Per-action pause (ms) |
| `DEMO_DWELL_MS` | `1500` | Default dwell after "thing appeared" beats |
| `DOOM_PORT` | `8787` | Server port |

## Make a GIF for the README

```bash
ffmpeg -i recordings/doom-in-the-browser-doom-boots-and-responds-to-the-keyboard.mp4 \
  -vf "fps=10,scale=960:-1" recordings/doom.gif
```

Keep fps low (8–12) and width ≤ 960px to stay under GitHub's attachment limit.
